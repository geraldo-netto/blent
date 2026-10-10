//! T793: exercise the generated non-test executable against current native code.
#![cfg(all(target_os = "linux", feature = "inproc-encoder"))]

#[test]
fn t793_offline_generated_replay_builds_and_matches_fifo_shared_output() {
    let mut command = std::process::Command::new("python3");
    command.arg(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/shared_encode_replay.py"
    ));
    // This separate, generated benchmark is not a coverage source root. Keep
    // production counters in the outer suite, not deleted temporary copies.
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        for key in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "LLVM_PROFILE_FILE",
            "RUSTC_WRAPPER",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS",
            "__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES",
        ] {
            command.env_remove(key);
        }
    }
    let status = command
        .status()
        .expect("Python is required for the generated replay regression");
    assert!(status.success(), "T793 offline replay build/smoke failed");
}
