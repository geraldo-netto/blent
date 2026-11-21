//! One poll-owner cache: bounded current identities and labels, never device handles.
use super::{device_label, pci_database, Adapter, Backend, Device, Path, PathBuf};
use std::{ffi::CString, io::Read, os::unix::ffi::OsStrExt};

#[derive(PartialEq, Eq)]
struct Source {
    identity: String,
    node: PathBuf,
    event: String,
}
struct Label {
    source: Source,
    text: String,
}

#[derive(Default)]
pub struct Discovery {
    labels: Vec<Label>,
}

pub fn discover() -> Vec<Device> {
    Discovery::default().refresh()
}

// Permission evidence only; opening/using the device still performs actual admission.
pub(super) fn permissions(node: &Path) -> bool {
    let Ok(path) = CString::new(node.as_os_str().as_bytes()) else {
        return false;
    };
    // Effective credentials and ACLs, without opening the render node.
    unsafe {
        libc::faccessat(
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::R_OK | libc::W_OK,
            libc::AT_EACCESS,
        ) == 0
    }
}

impl Discovery {
    pub fn refresh(&mut self) -> Vec<Device> {
        self.refresh_in(
            Path::new("/dev/dri"),
            Path::new("/sys/class/drm"),
            permissions,
            || {
                pci_database(&[
                    Path::new("/usr/share/misc/pci.ids"),
                    Path::new("/usr/share/hwdata/pci.ids"),
                ])
            },
        )
    }

    pub(super) fn refresh_in(
        &mut self,
        dri: &Path,
        sys: &Path,
        accessible: impl Fn(&Path) -> bool,
        database: impl FnOnce() -> String,
    ) -> Vec<Device> {
        let current = sources(dri, sys);
        if !self
            .labels
            .iter()
            .map(|label| &label.source)
            .eq(current.iter())
        {
            self.rebuild(current, database);
        }
        self.labels
            .iter()
            .map(|label| label.device(&accessible))
            .collect()
    }

    fn rebuild(&mut self, sources: Vec<Source>, database: impl FnOnce() -> String) {
        if sources.is_empty() {
            self.labels.clear();
            return;
        }
        let database = database();
        self.labels = sources
            .into_iter()
            .map(|source| {
                let text = device_label(&source.identity, &source.event, &database);
                Label { source, text }
            })
            .collect();
    }
}

impl Label {
    fn device(&self, accessible: &impl Fn(&Path) -> bool) -> Device {
        Device {
            adapter: Adapter {
                id: format!("vaapi:{}", self.source.identity),
                label: self.text.clone(),
                backend: Backend::Vaapi,
                accessible: accessible(&self.source.node),
            },
            node: self.source.node.clone(),
        }
    }
}

fn sources(dri: &Path, sys: &Path) -> Vec<Source> {
    let Ok(entries) = std::fs::read_dir(dri.join("by-path")) else {
        return Vec::new();
    };
    let mut sources: Vec<_> = entries
        .take(256)
        .flatten()
        .filter_map(|entry| source(&entry.path(), dri, sys))
        .collect();
    sources.sort_by(|a, b| a.identity.cmp(&b.identity));
    sources.dedup_by(|a, b| a.identity == b.identity);
    sources
}

fn source(alias: &Path, dri: &Path, sys: &Path) -> Option<Source> {
    let identity = alias.file_name()?.to_str()?;
    if !identity.ends_with("-render") || identity.len() > 200 {
        return None;
    }
    let node = std::fs::canonicalize(alias).ok()?;
    if node.parent()? != std::fs::canonicalize(dri).ok()? {
        return None;
    }
    let name = node.file_name()?.to_str()?;
    name.strip_prefix("renderD")?.parse::<u32>().ok()?;
    let event = event(&sys.join(name).join("device/uevent"));
    Some(Source {
        identity: identity.into(),
        node,
        event,
    })
}

fn event(path: &Path) -> String {
    let mut text = String::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(4096).read_to_string(&mut text);
    }
    text
}
