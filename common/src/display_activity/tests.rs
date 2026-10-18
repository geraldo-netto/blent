use super::*;

#[test]
fn t1055_shared_wire_vectors_require_exact_types_and_ranges() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../testdata/display-activity-v1.json")).unwrap();
    for case in vectors["valid"].as_array().unwrap() {
        let signal: Signal = serde_json::from_str(case["json"].as_str().unwrap()).unwrap();
        let encoded = serde_json::to_string(&signal).unwrap();
        assert_eq!(serde_json::from_str::<Signal>(&encoded).unwrap(), signal);
        assert!(signal.revision() > 0);
        assert!(signal.stopped() <= signal.revision());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["active"],
            signal.active()
        );
    }
    for case in vectors["invalid"].as_array().unwrap() {
        assert!(
            serde_json::from_str::<Signal>(case["json"].as_str().unwrap()).is_err(),
            "{}",
            case["name"]
        );
    }
}
#[test]
fn t1055_bounded_counter_boundaries_and_invalid_values_never_construct_a_signal() {
    for revision in [
        0,
        1,
        2,
        MAX_REVISION - 1,
        MAX_REVISION,
        MAX_REVISION + 1,
        u64::MAX,
    ] {
        for stopped in [0, 1, 2, MAX_REVISION, MAX_REVISION + 1, u64::MAX] {
            for active in [false, true] {
                let valid = revision > 0 && revision <= MAX_REVISION && stopped <= revision;
                assert_eq!(Signal::new(revision, stopped, active).is_ok(), valid);
            }
        }
    }
}
