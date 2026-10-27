//! Installed packaging paths stay behind the native adapter. No network lookup.
#[cfg(target_os = "linux")]
pub(crate) fn bundled_notices_url() -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    bundled_at(&executable)
}

#[cfg(windows)]
pub(crate) fn bundled_notices_url() -> Result<String, String> {
    Err("Installed bundle notice lookup is unavailable in this Windows preview. Embedded project notices remain available.".into())
}

#[cfg(target_os = "linux")]
fn bundled_at(executable: &std::path::Path) -> Result<String, String> {
    let prefix = executable
        .parent()
        .and_then(|bin| bin.parent())
        .ok_or("Cannot locate installed bundle notices from this executable path.")?;
    let notices = prefix.join("share/doc/blent/bundled");
    if !notices.is_dir() || !notices.join("manifest.json").is_file() {
        return Err("Installed bundle notices or their manifest are missing. Embedded project notices remain available; they are not a complete inventory of this installation.".into());
    }
    Ok(file_url(&notices))
}

#[cfg(target_os = "linux")]
fn file_url(path: &std::path::Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut url = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                url.push(byte as char)
            }
            _ => url.push_str(&format!("%{byte:02X}")),
        }
    }
    url
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn t1277_bundle_lookup_requires_installed_notices_and_manifest() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("usr/bin/blent-gui");
        assert!(bundled_at(&executable).unwrap_err().contains("missing"));
        let notices = root.path().join("usr/share/doc/blent/bundled");
        std::fs::create_dir_all(&notices).unwrap();
        assert!(bundled_at(&executable).is_err());
        std::fs::write(notices.join("manifest.json"), "{}").unwrap();
        assert_eq!(bundled_at(&executable).unwrap(), file_url(&notices));
        assert!(bundled_at(std::path::Path::new("/")).is_err());
        assert!(bundled_notices_url().is_err());
    }

    #[test]
    fn t1277_local_notice_urls_preserve_paths_without_url_injection() {
        use std::os::unix::ffi::OsStrExt;
        let path = std::path::Path::new(std::ffi::OsStr::from_bytes(b"/a b/#?%\xff-_.~"));
        assert_eq!(file_url(path), "file:///a%20b/%23%3F%25%FF-_.~");
        for byte in 0..=255_u8 {
            let bytes = [b'/', byte];
            let encoded = file_url(std::path::Path::new(std::ffi::OsStr::from_bytes(&bytes)));
            assert!(!encoded.contains('?'));
            assert!(!encoded.contains('#'));
            assert!(encoded.is_ascii());
        }
    }
}
