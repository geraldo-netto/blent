async fn probe_device_identity(serial: &str, adb: &str) -> Option<(String, String)> {
    let output = tokio::process::Command::new(adb)
        .args(["-s", serial, "shell", "getprop", "ro.serialno"])
        .output_bounded()
        .await
        .ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !id.is_empty() && !id.eq_ignore_ascii_case("unknown"))
        .then(|| (serial.to_string(), id))
}
