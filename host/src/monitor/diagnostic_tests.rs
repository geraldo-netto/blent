use super::*;

#[tokio::test]
async fn t929_missing_adb_reports_only_the_build_supported_requirement() {
    const TEST: &str =
        "monitor::diagnostic_tests::t929_missing_adb_reports_only_the_build_supported_requirement";
    if std::env::var_os("BLENT_T929_DIAGNOSTIC").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env("BLENT_T929_DIAGNOSTIC", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    crate::test_logging::enable();
    let mut state = super::tests::fixture();
    state.queue_inventory(false);
    let result = tokio::time::timeout(Duration::from_secs(2), state.inventory.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(result.devices.is_none());
    state.inventory_ready(result);
    state.stop().await;
    let message = crate::test_logging::text();
    #[cfg(feature = "accessory-transport")]
    {
        assert!(
            !message.contains("install android-tools (or adb) and restart"),
            "T929 native USB was described as requiring ADB: {message}"
        );
        assert!(message.contains("Direct USB does not require ADB"));
    }
    #[cfg(not(feature = "accessory-transport"))]
    assert!(message.contains("install android-tools (or adb) and restart"));
}
