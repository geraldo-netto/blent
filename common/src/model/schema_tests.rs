use super::*;

#[test]
fn t1679_obsolete_and_unknown_root_fields_are_rejected() {
    for name in ["profile_cache", "encoder_worker", "unsupported_option"] {
        for value in ["false", "true", "0", "-1", "'old'", "[]", "{}"] {
            let input = format!("{name} = {value}\ncalibration_generation = 42\n");
            assert!(
                toml::from_str::<FileConfig>(&input).is_err(),
                "T1679 accepted {input}"
            );
        }
    }
    for index in 0..64 {
        let key = format!("unsupported_{index:x}");
        assert!(toml::from_str::<FileConfig>(&format!("[{key}]\nvalue = 1")).is_err());
        let value = serde_json::json!({key: index});
        assert!(serde_json::from_value::<FileConfig>(value).is_err());
    }
}

#[test]
fn t1679_current_known_settings_and_defaults_remain_valid() {
    let defaults = FileConfig::default();
    assert_eq!(toml::from_str::<FileConfig>("").unwrap(), defaults);
    let text = toml::to_string(&defaults).unwrap();
    assert_eq!(toml::from_str::<FileConfig>(&text).unwrap(), defaults);
    assert_eq!(
        serde_json::from_value::<FileConfig>(serde_json::to_value(&defaults).unwrap()).unwrap(),
        defaults
    );
    let settings: FileConfig = toml::from_str(
        "calibration_generation = 42\nadaptive_idle = true\ntuning_cache_entries = 2",
    )
    .unwrap();
    assert_eq!(settings.calibration_generation, 42);
    assert!(settings.adaptive_idle);
    assert_eq!(settings.tuning_cache_entries, 2);
}
