//! Linux dynamic status and assigned-tablet probes.
use super::{Capabilities, Source};
use crate::{apply_tablet_sessions, autostart_enabled, command_exists, pid_path, Status};
use blent_config::{commands::SyncCommandExt, linux::daemon, runtime};
use std::process::Command;
#[derive(Default)]
pub(super) struct Platform {
    daemon: daemon::StatusProbe,
    gpus: Vec<blent_config::linux::gpu::Device>,
    inventory: Vec<blent_config::workspace::Device>,
    inventory_error: Option<String>,
    display: Option<blent_config::linux::display_control::Client>,
}

impl Source for Platform {
    fn refresh_devices(&mut self) {
        match adb_devices()
            .map_err(|error| error.to_string())
            .and_then(|text| inventory(&text))
        {
            Ok(devices) => {
                self.inventory = devices;
                self.inventory_error = None;
            }
            Err(error) => {
                self.inventory.clear();
                self.inventory_error = Some(error);
            }
        }
    }
    fn capabilities(&mut self) -> Capabilities {
        self.gpus = blent_config::linux::gpu::discover();
        Capabilities {
            daemon_binary: crate::find_blent_bin().is_some(),
            ffmpeg: command_exists("ffmpeg"),
            adb: command_exists("adb"),
            autostart: autostart_enabled(),
            diagnostics: None,
        }
    }

    fn dynamic(&mut self, capabilities: &Capabilities) -> Status {
        let pid = self.daemon.poll(Some(&pid_path()));
        let mut status = Status {
            gpus: blent_config::linux::gpu::catalog(&self.gpus),
            daemon_binary: capabilities.daemon_binary,
            ffmpeg_ok: capabilities.ffmpeg,
            adb_ok: capabilities.adb,
            autostart: capabilities.autostart,
            diagnostics: capabilities.diagnostics.clone(),
            daemon_running: pid.is_some(),
            daemon_pid: pid.unwrap_or_default(),
            evdi_count: std::fs::read_to_string("/sys/devices/evdi/count")
                .ok()
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or(-1),
            uinput_ok: std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/uinput")
                .is_ok(),
            ..Status::default()
        };
        let sessions = runtime::runtime_dir()
            .ok()
            .and_then(|dir| runtime::load_sessions(&dir.join("sessions.json")))
            .unwrap_or_default();
        status.pipe_ceiling = blent_config::linux::pipe::ceiling_bytes();
        status.pipe_capacities = sessions
            .iter()
            .map(|session| {
                let capacity = runtime::fifo_path_for(session.instance)
                    .ok()
                    .and_then(|path| blent_config::linux::pipe::effective_bytes(&path));
                (session.instance, capacity)
            })
            .collect();
        let fifos: Vec<_> = sessions
            .iter()
            .filter_map(|session| runtime::fifo_path_for(session.instance).ok())
            .collect();
        status.active_gpus =
            blent_config::linux::gpu::active(status.daemon_pid, &fifos, &self.gpus);
        query_tablets(&mut status, sessions, capabilities.adb, adb_devices);
        status.inventory_error = self.inventory_error.clone();
        merge_inventory(&mut status, &self.inventory);
        if status.daemon_running {
            self.query_display(&mut status);
        }
        status
    }
}

impl Platform {
    fn query_display(&mut self, status: &mut Status) {
        let result = self.display_query(status.daemon_pid);
        match result {
            Ok(reply) => {
                status.display = Some(reply.snapshot);
                status.display_error = reply.error;
            }
            Err(error) => {
                status.display = None;
                status.display_error = Some(error);
            }
        }
    }
    fn display_query(&mut self, pid: u32) -> Result<blent_config::display_control::Reply, String> {
        if self.display.is_none() {
            self.display = Some(
                blent_config::linux::display_control::Client::new()
                    .map_err(|error| error.to_string())?,
            );
        }
        self.display
            .as_ref()
            .unwrap()
            .request(pid, &blent_config::display_control::Command::Inspect {})
            .map_err(|error| format!("Display controls unavailable. In Settings, restart the display service after updating Blent. Details: {error:#}"))
    }
}

fn inventory(text: &str) -> Result<Vec<blent_config::workspace::Device>, String> {
    use blent_config::{
        tablets::Identity,
        workspace::{Catalog, Device, Presence},
    };
    let mut catalog = Catalog::new(1024).unwrap();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(serial) = fields.next() else {
            continue;
        };
        if fields.next() != Some("device") {
            continue;
        }
        let name = fields
            .find_map(|field| field.strip_prefix("model:"))
            .unwrap_or(serial)
            .replace('_', " ");
        let device = Device::new(
            Identity::Adb(serial.to_owned()),
            name,
            Presence::Unknown,
            false,
        )
        .map_err(|error| error.to_string())?;
        catalog.insert(device).map_err(|error| error.to_string())?;
    }
    Ok(catalog.into_devices())
}

fn merge_inventory(status: &mut Status, inventory: &[blent_config::workspace::Device]) {
    let mut catalog = blent_config::workspace::Catalog::new(1024).unwrap();
    for device in status.tablets.drain(..).chain(inventory.iter().cloned()) {
        if let Err(error) = catalog.insert(device) {
            status.inventory_error = Some(match status.inventory_error.take() {
                Some(discovery) => format!("{discovery}; {error}"),
                None => error.to_string(),
            });
            break;
        }
    }
    status.tablets = catalog.into_devices();
}

#[cfg(test)]
mod tests;

fn adb_devices() -> std::io::Result<String> {
    let out = Command::new("adb")
        .args(["devices", "-l"])
        .output_bounded()?;
    if !out.status.success() {
        return Err(std::io::Error::other("ADB inventory command failed"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub(super) fn query_tablets(
    status: &mut Status,
    sessions: Vec<runtime::TabletSession>,
    adb_available: bool,
    devices: impl FnOnce() -> std::io::Result<String>,
) {
    let needs_adb = adb_available
        && sessions
            .iter()
            .any(|session| session.identity.adb_serial().is_some());
    let text = if needs_adb {
        devices().unwrap_or_default()
    } else {
        String::new()
    };
    apply_tablet_sessions(status, &text, sessions);
}
