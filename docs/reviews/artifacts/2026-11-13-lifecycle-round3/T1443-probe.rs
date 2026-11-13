#[path = "../../../../host/src/daemon_pid.rs"]
mod daemon_pid;
fn remove_pid_file_if_ours(_: &std::path::Path, _: u32) { panic!("publication must fail before a guard exists"); }
fn main() {
 let path = std::env::args_os().nth(1).unwrap();
 let path = std::path::PathBuf::from(path);
 let error = daemon_pid::Publication::new(path.clone()).err().expect("expected constrained write failure");
 let partial = std::fs::read_to_string(&path).unwrap();
 println!("error={error}; partial={partial:?}; original_pid={}", std::process::id());
 assert!(!partial.is_empty());
 assert_ne!(partial, std::process::id().to_string());
 std::fs::remove_file(path).unwrap();
}
