//! T898: independent common settings contract exercised through the real CLI.
use super::*;
use crate::broker::linux::entry::Options;
use clap::Parser;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
};

struct Range {
    option: &'static str,
    default: i64,
    minimum: i64,
    maximum: i64,
}
fn range(line: &'static str) -> Range {
    let fields: Vec<_> = line.split('\t').collect();
    assert_eq!(fields.len(), 5);
    Range {
        option: fields[1],
        default: fields[2].parse().unwrap(),
        minimum: fields[3].parse().unwrap(),
        maximum: fields[4].parse().unwrap(),
    }
}
fn contract() -> Vec<Range> {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../testdata/connection-settings-v1.tsv"
    ));
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap(),
        "android_field\thost_option\tdefault\tminimum\tmaximum"
    );
    let result: Vec<_> = lines.map(range).collect();
    assert_eq!(result.len(), 9);
    result
}
fn values(settings: &Settings) -> BTreeMap<&'static str, i64> {
    [
        ("--channel-bytes", i64::from(settings.session.channel_bytes)),
        (
            "--queued-messages",
            i64::try_from(settings.session.queued_messages).unwrap(),
        ),
        ("--quantum", i64::from(settings.session.quantum)),
        ("--resource-timeout-ms", settings.resource_timeout_ms),
        (
            "--io-timeout-ms",
            i64::try_from(settings.session.io_timeout.as_millis()).unwrap(),
        ),
        (
            "--retirement-timeout-ms",
            i64::try_from(settings.retirement_timeout.as_millis()).unwrap(),
        ),
        ("--approval-ms", settings.control.approval_ms),
        (
            "--trust-peers",
            i64::try_from(settings.trust_peers).unwrap(),
        ),
        (
            "--tls-timeout-ms",
            i64::try_from(settings.tls_timeout.as_millis()).unwrap(),
        ),
    ]
    .into_iter()
    .collect()
}
fn arguments() -> Vec<OsString> {
    [
        "blent-usb-broker",
        "--runtime",
        "/runtime",
        "--state",
        "/state",
        "--helper",
        "/helper",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn selected(option: &str, value: i64) -> Settings {
    let mut args = arguments();
    args.extend([option.into(), value.to_string().into()]);
    Options::try_parse_from(args).unwrap().settings()
}
#[tokio::test]
async fn t898_shared_defaults_and_boundaries_reach_validated_cli_snapshot() {
    let defaults = values(&Options::try_parse_from(arguments()).unwrap().settings());
    let ranges = contract();
    assert_eq!(
        defaults.keys().copied().collect::<BTreeSet<_>>(),
        ranges.iter().map(|range| range.option).collect()
    );
    for range in ranges {
        assert_eq!(defaults[range.option], range.default);
        for value in [range.minimum, range.maximum] {
            let selected = selected(range.option, value);
            selected.validate().unwrap();
            let mut expected = defaults.clone();
            expected.insert(range.option, value);
            assert_eq!(values(&selected), expected);
        }
    }
}
#[tokio::test]
async fn t898_shared_out_of_bounds_values_reject_without_altering_current_settings() {
    let current = Options::try_parse_from(arguments()).unwrap().settings();
    let snapshot = values(&current);
    for range in contract() {
        for value in [range.minimum - 1, range.maximum + 1] {
            assert!(
                selected(range.option, value).validate().is_err(),
                "{}={value}",
                range.option
            );
            assert_eq!(values(&current), snapshot);
        }
    }
}
