use std::path::{Path, PathBuf};

fn compile(root: &Path, sources: &[&str], target: &str, shared: bool) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut compiler = std::process::Command::new("cc");
    compiler.args(["-Wall", "-Wextra", "-Werror"]);
    if shared {
        compiler.args(["-shared", "-fPIC"]);
    }
    for file in sources {
        compiler.arg(source.join(file));
    }
    let result = compiler.arg("-o").arg(root.join(target)).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

pub fn run(test: &str, task_limit: &str) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    compile(root, &["input_commands/command.c"], "busctl", false);
    compile(
        root,
        &["uinput_mock.c", "input_commands/deny_kill.c"],
        "shim.so",
        true,
    );
    std::fs::write(root.join("device"), "").unwrap();
    std::fs::write(root.join("calls"), "").unwrap();
    let backup = root.join(".local/share/blent/osk-restore");
    std::fs::create_dir_all(backup.parent().unwrap()).unwrap();
    std::fs::write(backup, "1").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("LD_PRELOAD", root.join("shim.so"))
        .env("BLENT_TASK_LIMIT", task_limit)
        .env("BLENT_T497_UINPUT_FILE", root.join("device"))
        .env("BLENT_T497_UINPUT_LOG", root.join("ioctl"))
        .env("BLENT_T1433_CHILD", root.join("child"))
        .env("BLENT_T1433_CALLS", root.join("calls"))
        .env("BLENT_T1433_RELEASE", root.join("release"))
        .env("PATH", root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root)
        .env("XDG_RUNTIME_DIR", root)
        .env("XDG_CURRENT_DESKTOP", "test")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

pub fn intercepted() {
    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "T1433 refuses real device access");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
}
pub fn calls() -> usize {
    std::fs::read_to_string(std::env::var_os("BLENT_T1433_CALLS").unwrap())
        .unwrap()
        .lines()
        .count()
}
pub struct Release(PathBuf);
impl Release {
    pub fn new() -> Self {
        Self(std::env::var_os("BLENT_T1433_RELEASE").unwrap().into())
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        std::fs::write(&self.0, "release").unwrap();
    }
}
