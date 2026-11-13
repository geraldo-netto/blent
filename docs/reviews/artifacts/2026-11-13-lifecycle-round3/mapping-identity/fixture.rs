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

pub fn run(test: &str, status: &str) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    compile(root, &["../../docs/reviews/artifacts/2026-11-13-lifecycle-round3/mapping-identity/command.c"], "xinput", false);
    for tool in ["xrandr", "busctl"] {
        std::fs::hard_link(root.join("xinput"), root.join(tool)).unwrap();
    }
    compile(
        root,
        &[
            "uinput_mock.c",
            "input_commands/deny_kill.c",
            "input_mapping/sysfs.c",
        ],
        "shim.so",
        true,
    );
    std::fs::create_dir(root.join("sysfs")).unwrap();
    std::fs::write(root.join("device"), "").unwrap();
    std::fs::write(root.join("calls"), "").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("LD_PRELOAD", root.join("shim.so"))
        .env("BLENT_T497_UINPUT_FILE", root.join("device"))
        .env("BLENT_T497_UINPUT_LOG", root.join("ioctl"))
        .env("BLENT_T1433_CHILD", root.join("child"))
        .env("BLENT_T1439_CALLS", root.join("calls"))
        .env("BLENT_T1439_RELEASE", root.join("release"))
        .env("BLENT_T1439_MAP_STATUS", status)
        .env("BLENT_T1439_SYSFS", root.join("sysfs"))
        .env("PATH", root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root)
        .env("XDG_RUNTIME_DIR", root)
        .env("XDG_SESSION_TYPE", "x11")
        .env("XDG_CURRENT_DESKTOP", "test")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .output()
        .unwrap();
    print!("{}", String::from_utf8_lossy(&result.stdout));
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
pub fn intercepted() {
    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "T1439 refuses real device access");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
    assert_eq!(
        std::fs::read_dir("/sys/devices/platform").unwrap().count(),
        0
    );
}
pub fn calls() -> String {
    std::fs::read_to_string(std::env::var_os("BLENT_T1439_CALLS").unwrap()).unwrap()
}
pub fn counts() -> (usize, usize) {
    let text = std::fs::read_to_string(std::env::var_os("BLENT_T497_UINPUT_LOG").unwrap())
        .unwrap_or_default();
    (
        text.matches("request 21761\n").count(),
        text.matches("request 21762\n").count(),
    )
}
pub fn child() -> PathBuf {
    std::env::var_os("BLENT_T1433_CHILD").unwrap().into()
}
pub struct Release(PathBuf);
impl Release {
    pub fn new() -> Self {
        Self(std::env::var_os("BLENT_T1439_RELEASE").unwrap().into())
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        std::fs::write(&self.0, "release").unwrap();
    }
}
