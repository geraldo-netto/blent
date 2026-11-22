//! T1701: each environment variant owns a subprocess and private working directory.
use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

fn check_location(root: &std::path::Path) {
    let generated = ensure_edid_sized(640, 480, 60, 310, 194, 0);
    if std::env::var_os("BLENT_T1701_VALID").is_some() {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let path = generated.unwrap();
        assert_eq!(path.parent().unwrap(), home.join(".local/share/blent/edid"));
        assert_eq!(std::fs::read(path).unwrap().len(), 128);
    } else {
        assert!(
            generated.is_err(),
            "T1701 accepted an unavailable data location"
        );
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }
}

fn location_case(home: Option<OsString>, valid: bool) {
    let root = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "edid::location_tests::t1701_data_location_requires_absolute_home",
            "--nocapture",
        ])
        .env("BLENT_T1701_ROOT", root.path())
        .env_remove("BLENT_T1701_VALID")
        .env_remove("HOME")
        .current_dir(root.path());
    if let Some(home) = home {
        command.env("HOME", home);
    }
    if valid {
        command.env("BLENT_T1701_VALID", "1");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn t1701_data_location_requires_absolute_home() {
    if let Some(root) = std::env::var_os("BLENT_T1701_ROOT") {
        check_location(&PathBuf::from(root));
        return;
    }
    for home in [None, Some("".into()), Some("relative-home".into())] {
        location_case(home, false);
    }
    let home = tempfile::tempdir().unwrap();
    location_case(Some(home.path().as_os_str().to_owned()), true);
    let non_utf8 = home.path().join(OsString::from_vec(vec![b'h', 0xff]));
    location_case(Some(non_utf8.into_os_string()), true);
}
