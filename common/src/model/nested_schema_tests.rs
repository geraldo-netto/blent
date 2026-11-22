use super::*;

fn rejects<T: serde::de::DeserializeOwned>(key: &str) {
    assert!(
        toml::from_str::<T>(&format!("{key} = true")).is_err(),
        "T1680 accepted {key}"
    );
    assert!(
        serde_json::from_value::<T>(serde_json::json!({key:true})).is_err(),
        "T1680 accepted JSON {key}"
    );
}

#[test]
fn t1680_nested_and_direct_settings_reject_unknown_keys() {
    for index in 0..64 {
        let key = format!("unsupported_{index:x}");
        for section in ["camera", "camera.sensor", "host_activity"] {
            let input = format!("[{section}]\n{key} = true");
            assert!(
                toml::from_str::<FileConfig>(&input).is_err(),
                "T1680 accepted {input}"
            );
        }
        rejects::<crate::camera::CameraSettings>(&key);
        rejects::<crate::camera::CameraProfile>(&key);
        rejects::<crate::camera::tuning::Sensor>(&key);
        rejects::<crate::host_activity::Settings>(&key);
    }
}

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    let text = toml::to_string(value).unwrap();
    assert_eq!(&toml::from_str::<T>(&text).unwrap(), value);
    let json = serde_json::to_value(value).unwrap();
    assert_eq!(&serde_json::from_value::<T>(json).unwrap(), value);
}

#[test]
fn t1680_known_flat_camera_fields_sensor_and_activity_round_trip() {
    let current: FileConfig = toml::from_str("[camera]\nmirror = true\n[camera.sensor]\nexposure = 100\n[host_activity]\npause_on_lock = false").unwrap();
    assert!(current.camera.options.mirror);
    assert_eq!(current.camera.options.sensor.exposure, 100);
    assert!(!current.host_activity.pause_on_lock);
    round_trip(&current);
    round_trip(&current.camera);
    round_trip(&current.camera.options);
    round_trip(&current.camera.options.sensor);
    round_trip(&current.host_activity);
    assert_eq!(
        toml::from_str::<FileConfig>("[camera]\n[camera.sensor]\n[host_activity]").unwrap(),
        FileConfig::default()
    );
    let camera = toml::Value::try_from(&current.camera).unwrap();
    assert!(camera.get("options").is_none());
    assert_eq!(camera["mirror"].as_bool(), Some(true));
}
