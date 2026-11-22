//! Frame-local projection of the authoritative draft/baseline. No values are
//! exposed, no mutable state is duplicated, and output is bounded by schema
//! leaf count (configuration values cannot introduce arbitrary table keys).
use crate::{AudioDirection, Tab};
use blent_config::model::FileConfig;
use eframe::egui;
use toml::Value;

#[derive(Debug, PartialEq)]
struct Change {
    path: String,
}

fn changes(edited: &FileConfig, saved: &FileConfig) -> Result<Vec<Change>, toml::ser::Error> {
    let mut result = Vec::new();
    compare(
        "",
        Some(&Value::try_from(edited)?),
        Some(&Value::try_from(saved)?),
        &mut result,
    );
    Ok(result)
}

fn compare(path: &str, edited: Option<&Value>, saved: Option<&Value>, result: &mut Vec<Change>) {
    if edited == saved {
        return;
    }
    if let (Some(Value::Table(edited)), Some(Value::Table(saved))) = (edited, saved) {
        let keys: std::collections::BTreeSet<_> = edited.keys().chain(saved.keys()).collect();
        for key in keys {
            let path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            compare(&path, edited.get(key), saved.get(key), result);
        }
    } else {
        result.push(Change { path: path.into() });
    }
}

fn page(path: &str) -> Tab {
    let key = path.split('.').next().unwrap_or(path);
    match key {
        "camera" => Tab::Camera,
        "audio" => audio_page(path),
        _ if DISPLAY_FIELDS.contains(&key) => Tab::Display,
        _ if GENERAL_FIELDS.contains(&key) => Tab::Settings,
        _ => Tab::Video,
    }
}

const DISPLAY_FIELDS: &[&str] = &[
    "pen_only",
    "position",
    "max_tablets",
    "input_touch",
    "input_pen",
    "input_pointer",
    "input_mouse",
    "direct_input",
];
const GENERAL_FIELDS: &[&str] = &[
    "scheduling_priority",
    "host_activity",
    "check_updates",
    "wifi_address",
    "video_port",
    "input_port",
    "auto_launch_app",
];

fn audio_page(path: &str) -> Tab {
    if path.starts_with("audio.speakers") {
        Tab::Audio(AudioDirection::Speakers)
    } else {
        Tab::Audio(AudioDirection::Microphone)
    }
}

fn label(change: &Change) -> String {
    let field = change
        .path
        .strip_prefix("camera.")
        .or_else(|| change.path.strip_prefix("audio.microphone."))
        .or_else(|| change.path.strip_prefix("audio.speakers."))
        .unwrap_or(&change.path)
        .replace(['_', '.'], " ");
    format!("{} · {field}", page(&change.path).title())
}

pub(super) fn show(ui: &mut egui::Ui, edited: &FileConfig, saved: &FileConfig) -> Option<Tab> {
    let changes = match changes(edited, saved) {
        Ok(changes) => changes,
        Err(error) => {
            ui.label(format!("Unable to list pending fields: {error}"));
            return None;
        }
    };
    if changes.is_empty() {
        return None;
    }
    ui.label(format!("{} unsaved fields · all settings", changes.len()));
    details(ui, &changes)
}

fn details(ui: &mut egui::Ui, changes: &[Change]) -> Option<Tab> {
    let mut target = None;
    ui.menu_button("Review changed fields", |ui| {
        egui::ScrollArea::vertical().id_salt("draft-fields").max_height(100.0).show(ui, |ui| {
            for change in changes {
                if ui.link(label(change)).clicked() {
                    target = Some(page(&change.path));
                    ui.close();
                }
            }
            ui.small("Save and Discard affect all listed fields. Camera image adjustments preview live; discarding restores their saved values when the camera page is shown.");
        });
    });
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1782_summary_covers_nested_edits_and_removed_optional_values_without_exposing_values() {
        let baseline = FileConfig::default();
        assert!(changes(&baseline, &baseline).unwrap().is_empty());
        let mut draft = baseline.clone();
        draft.camera.options.serial = Some("private-tablet-identity".into());
        draft.audio.speakers.profile.background = true;
        draft.audio.microphone.profile.background = true;
        draft.width += 2;
        draft.check_updates = !draft.check_updates;
        draft.position = "left".into();
        let forward = changes(&draft, &baseline).unwrap();
        let reverse = changes(&baseline, &draft).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.len(), 6);
        for change in &forward {
            assert!(!label(change).contains("private-tablet-identity"));
        }
        assert!(page("camera.serial") == Tab::Camera);
        assert!(page("audio.speakers.background") == Tab::Audio(AudioDirection::Speakers));
        assert!(page("audio.microphone.background") == Tab::Audio(AudioDirection::Microphone));
        assert!(page("width") == Tab::Video);
        assert!(page("check_updates") == Tab::Settings);
        assert!(page("position") == Tab::Display);
    }

    #[test]
    fn t1782_leaf_diff_handles_empty_added_removed_and_scalar_shapes() {
        for (edited, saved, expected) in [
            ("a = 1", "a = 1", Vec::<&str>::new()),
            ("a = 1", "", vec!["a"]),
            ("", "a = 1", vec!["a"]),
            ("a = {b = 1}", "a = 2", vec!["a"]),
            ("a = {b = 1}", "a = {b = 2}", vec!["a.b"]),
        ] {
            let mut result = Vec::new();
            compare(
                "",
                Some(&edited.parse().unwrap()),
                Some(&saved.parse().unwrap()),
                &mut result,
            );
            assert_eq!(
                result
                    .iter()
                    .map(|change| change.path.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
