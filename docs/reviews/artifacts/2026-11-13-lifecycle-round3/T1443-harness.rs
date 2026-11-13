#[path = "../../../../host/src/daemon_pid.rs"]
mod daemon_pid;
fn remove_pid_file_if_ours(pid_path: &std::path::Path, pid: u32) {
    let ours = std::fs::read_to_string(pid_path)
        .ok()
        .and_then(|t| t.trim().parse::<u32>().ok())
        .map(|p| p == pid)
        .unwrap_or(false);
    if ours {
        let _ = std::fs::remove_file(pid_path);
    }
}
