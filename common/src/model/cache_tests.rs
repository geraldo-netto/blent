use super::*;

#[test]
fn t1490_cache_capacity_is_persisted_bounded_and_does_not_restart_media() {
    let defaults = FileConfig::default();
    assert_eq!(
        toml::Value::try_from(&defaults).unwrap()["tuning_cache_entries"].as_integer(),
        Some(64)
    );
    for requested in [0, 1, 16, 64, 256, 257, u32::MAX] {
        let mut config: FileConfig =
            toml::from_str(&format!("tuning_cache_entries = {requested}")).unwrap();
        assert_eq!(config.validate().is_ok(), (1..=256).contains(&requested));
        config.sanitize_requested();
        let expected = if (1..=256).contains(&requested) {
            requested
        } else {
            64
        };
        assert_eq!(
            toml::Value::try_from(&config).unwrap()["tuning_cache_entries"].as_integer(),
            Some(i64::from(expected))
        );
        assert!(!config.requires_restart_from(&defaults));
    }
}

#[test]
fn t1490_cache_capacity_rejects_wrong_types_and_preserves_unedited_changes() {
    for value in ["-1", "4294967296", "true", "'64'", "[64]"] {
        assert!(toml::from_str::<FileConfig>(&format!("tuning_cache_entries = {value}")).is_err());
    }
    let baseline = FileConfig::default();
    let latest: FileConfig = toml::from_str("tuning_cache_entries = 2").unwrap();
    let merged = baseline.merge_edits(&baseline, latest).unwrap();
    assert_eq!(
        toml::Value::try_from(merged).unwrap()["tuning_cache_entries"].as_integer(),
        Some(2)
    );
}
