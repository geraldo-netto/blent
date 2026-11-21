use super::*;
use crate::cli::{Cli, Commands};
use clap::Parser;

#[test]
fn t1671_filter_capacity_default_cli_and_saved_profile_agree() {
    let default = serde_json::to_value(CameraProfile::default()).unwrap();
    assert_eq!(default["filter_threads"], 2);
    let Some(Commands::Cameras(options)) = Cli::parse_from(["blent", "cameras"]).command else {
        panic!()
    };
    let profile: &CameraProfile = &options;
    assert_eq!(serde_json::to_value(profile).unwrap(), default);
    for threads in 1..=4 {
        let cli =
            Cli::try_parse_from(["blent", "cameras", "--filter-threads", &threads.to_string()])
                .unwrap();
        let Some(Commands::Cameras(options)) = cli.command else {
            panic!()
        };
        let profile: &CameraProfile = &options;
        assert_eq!(
            serde_json::to_value(profile).unwrap()["filter_threads"],
            threads
        );
        let encoded = toml::to_string(profile).unwrap();
        assert_eq!(toml::from_str::<CameraProfile>(&encoded).unwrap(), *profile);
    }
}

#[test]
fn t1671_filter_capacity_rejects_outside_the_bounded_grant() {
    for threads in 0..=255 {
        let profile: CameraProfile =
            serde_json::from_value(serde_json::json!({"filter_threads": threads})).unwrap();
        assert_eq!(
            profile.validate().is_ok(),
            (1..=4).contains(&threads),
            "{threads}"
        );
    }
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(256),
        serde_json::json!(1.5),
        serde_json::json!("2"),
        serde_json::Value::Null,
    ] {
        assert!(serde_json::from_value::<CameraProfile>(
            serde_json::json!({"filter_threads": invalid})
        )
        .is_err());
    }
    for invalid in ["0", "5", "255", "256", "-1", "two"] {
        assert!(Cli::try_parse_from(["blent", "cameras", "--filter-threads", invalid]).is_err());
    }
}
