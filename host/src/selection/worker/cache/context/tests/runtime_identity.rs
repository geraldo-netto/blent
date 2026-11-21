use super::*;
use std::os::unix::fs::PermissionsExt;

fn isolated() -> bool {
    if std::env::var_os("BLENT_T1676_CACHE").is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "selection::worker::cache::context::tests::runtime_identity::t1676_native_payload_change_invalidates_an_unchanged_launcher", "--nocapture"])
        .env("BLENT_T1676_CACHE", "1")
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path())
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn t1676_native_payload_change_invalidates_an_unchanged_launcher() {
    if isolated() {
        return;
    }
    // Dedicated process: PATH/config changes cannot reach another test or Blent.
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("usr/bin");
    let libexec = root.path().join("usr/libexec");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&libexec).unwrap();
    let wrapper = bin.join("ffmpeg");
    std::fs::write(&wrapper, "#!/bin/sh\nprintf 'ffmpeg version fixture\\n'\n").unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        bin.join("ffmpeg.blent-runtime.json"),
        r#"{"schema":1,"files":["libexec/ffmpeg"]}"#,
    )
    .unwrap();
    let executable = libexec.join("ffmpeg");
    std::fs::write(&executable, "native encoder revision one").unwrap();
    let helper = root.path().join("helper");
    std::fs::write(&helper, "unchanged capture helper").unwrap();
    std::env::set_var("PATH", &bin);
    let config = CaptureConfig {
        helper_path: helper,
        ..Default::default()
    };
    let scope = blent::native_scope::Scope::default();
    let before = host_stamp(&config, &scope).unwrap();
    std::fs::write(&executable, "native encoder revision two").unwrap();
    let after = host_stamp(&config, &scope).unwrap();
    assert_ne!(
        before, after,
        "T1676 cache ignored changed native FFmpeg behind its launcher"
    );
}
