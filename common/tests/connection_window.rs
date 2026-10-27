use blent_config::connection::window;
use serde_json::Value;

fn bytes(record: &Value) -> Vec<u8> {
    record["hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn t881_shared_android_window_golden_records_match_rust() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../testdata/connection-window-v1.json")).unwrap();
    for record in fixture["valid"].as_array().unwrap() {
        let wire = bytes(record);
        let host = record["host"].as_bool().unwrap();
        let capacity = u32::try_from(record["capacity"].as_u64().unwrap()).unwrap();
        assert_eq!(window::decode(&wire, host).unwrap(), capacity);
        assert_eq!(window::encode(host, capacity).unwrap().as_slice(), wire);
        assert!(window::decode(&wire, !host).is_err());
    }
    for record in fixture["invalid"].as_array().unwrap() {
        assert!(window::decode(&bytes(record), record["host"].as_bool().unwrap()).is_err());
    }
}
