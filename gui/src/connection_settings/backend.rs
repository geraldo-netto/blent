//! Native composition stays outside widgets and portable control contracts.
use blent_config::connection::control_client::Client;

pub(super) trait Source {
    fn start(&mut self) -> Result<Box<dyn Client>, String>;
}

pub(super) fn create() -> Result<Box<dyn Source>, String> {
    #[cfg(target_os = "linux")]
    return linux::create();
    #[cfg(not(target_os = "linux"))]
    Err("ADB-free USB connections are not supported on this operating system.".into())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use blent::broker::linux::{
        client::Launch,
        presentation::{Factory, Settings},
    };
    use std::{ffi::OsString, path::PathBuf};

    pub(super) fn create() -> Result<Box<dyn Source>, String> {
        let launch = paths(
            std::env::current_exe().map_err(|error| error.to_string())?,
            std::env::var_os("XDG_RUNTIME_DIR"),
            std::env::var_os("XDG_STATE_HOME"),
            std::env::var_os("HOME"),
        )?;
        Ok(Box::new(
            Factory::new(Settings::new(launch)).map_err(|error| error.to_string())?,
        ))
    }

    // Local acceptance uses sibling binaries. Packaging discovery is T861.
    // No directory scan, external command or device I/O occurs on the GUI thread.
    fn paths(
        executable: PathBuf,
        runtime: Option<OsString>,
        state: Option<OsString>,
        home: Option<OsString>,
    ) -> Result<Launch, String> {
        let executable = absolute(Some(executable.into_os_string()), "GUI executable")?;
        let directory = executable
            .parent()
            .ok_or("GUI executable has no directory")?;
        let state = match state {
            Some(state) => absolute(Some(state), "XDG_STATE_HOME")?,
            None => absolute(home, "HOME")?.join(".local/state"),
        };
        Ok(Launch {
            executable: directory.join("blent-usb-broker"),
            helper: directory.join("blent-usb-io"),
            runtime: absolute(runtime, "XDG_RUNTIME_DIR")?.join("blent-usb"),
            state: state.join("blent/connection"),
        })
    }
    fn absolute(value: Option<OsString>, name: &str) -> Result<PathBuf, String> {
        value
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| format!("{name} must be an absolute path"))
    }
    impl Source for Factory {
        fn start(&mut self) -> Result<Box<dyn Client>, String> {
            Factory::start(self)
                .map(|handle| Box::new(handle) as Box<dyn Client>)
                .map_err(|error| error.to_string())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn t894_paths_are_absolute_and_do_not_probe_devices_or_tokens() {
            let launch = paths(
                "/opt/blent/gui".into(),
                Some("/run/user/42".into()),
                None,
                Some("/home/test".into()),
            )
            .unwrap();
            assert_eq!(
                launch.executable,
                PathBuf::from("/opt/blent/blent-usb-broker")
            );
            assert_eq!(launch.helper, PathBuf::from("/opt/blent/blent-usb-io"));
            assert_eq!(launch.runtime, PathBuf::from("/run/user/42/blent-usb"));
            assert_eq!(
                launch.state,
                PathBuf::from("/home/test/.local/state/blent/connection")
            );
            let launch = paths(
                "/bin/gui".into(),
                Some("/tmp/private".into()),
                Some("/state".into()),
                None,
            )
            .unwrap();
            assert_eq!(launch.state, PathBuf::from("/state/blent/connection"));
            for value in [
                None,
                Some("".into()),
                Some("relative".into()),
                Some("../escape".into()),
            ] {
                assert!(absolute(value.clone(), "test").is_err());
                assert!(paths("/bin/gui".into(), value, None, Some("/home/test".into())).is_err());
            }
            assert!(paths("gui".into(), Some("/run".into()), None, None).is_err());
            assert!(paths("/".into(), Some("/run".into()), None, None).is_err());
            assert!(paths(
                "/bin/gui".into(),
                Some("/run".into()),
                Some("relative".into()),
                None
            )
            .is_err());
            assert!(paths("/bin/gui".into(), Some("/run".into()), None, None).is_err());
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests {
    use super::*;
    use blent_config::commands::SyncCommandExt;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    #[test]
    fn t894_environment_composition_is_lazy_in_isolated_process() {
        if std::env::var_os("BLENT_T894_COMPOSITION").is_some() {
            // Construction owns only a factory; no broker or accessory is opened.
            drop(create().unwrap());
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "connection_settings::backend::native_tests::t894_environment_composition_is_lazy_in_isolated_process"])
            .env("BLENT_T894_COMPOSITION", "1")
            .env("XDG_RUNTIME_DIR", root.path())
            .env("XDG_STATE_HOME", root.path())
            .output_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn t894_native_start_failure_remains_bounded_and_retires() {
        use blent::broker::linux::{
            client::Launch,
            presentation::{Factory, Settings},
        };
        let root = tempfile::tempdir().unwrap();
        let settings = Settings::new(Launch {
            executable: root.path().join("missing-broker"),
            helper: root.path().join("missing-helper"),
            state: root.path().join("state"),
            runtime: root.path().join("runtime"),
        });
        let mut source = Factory::new(settings).unwrap();
        let client = Source::start(&mut source).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !client.retired() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            client.status().phase,
            blent_config::connection::control_client::Phase::Failed
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
