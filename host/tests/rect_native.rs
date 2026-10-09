#![cfg(target_os = "linux")]
#[test]
fn t767_project_jni_wrappers_reject_invalid_buffer_ranges() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let java = std::env::var_os("JAVA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/usr/lib/jvm/default-java".into());
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("rect-native-test");
    let output = std::process::Command::new("cc")
        .args([
            "-std=c11",
            "-O1",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-fno-pie",
            "-no-pie",
            "-I",
        ])
        .arg(java.join("include"))
        .arg("-I")
        .arg(java.join("include/linux"))
        .arg("-I")
        .arg(root.join("tests/native_contracts"))
        .arg(root.join("tests/rect_native.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "JNI/EGL development headers required: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
