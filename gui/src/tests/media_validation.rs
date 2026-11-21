//! T1553: invalid media targets have a visible error and cannot be saved.
use super::*;

fn invalid_media(resource: u8, app: &mut App) -> &'static str {
    match resource {
        0 => {
            app.cfg.camera.options.usb_peer = Some(String::new());
            "Camera settings"
        }
        1 => {
            app.cfg.audio.microphone.usb_peer = Some(String::new());
            "Microphone settings"
        }
        2 => {
            app.cfg.audio.speakers.usb_peer = Some(String::new());
            "Speaker settings"
        }
        _ => panic!("invalid fixture resource"),
    }
}

fn error_visible(output: &egui::FullOutput, prefix: &str) -> bool {
    let mut labels = Vec::new();
    for shape in &output.shapes {
        collect_text_rects(&shape.shape, &mut labels);
    }
    labels
        .iter()
        .any(|(label, _)| label.starts_with(prefix) && label.contains("invalid peer fingerprint"))
}

#[test]
fn t1553_save_discloses_invalid_media_target_and_remains_disabled() {
    for resource in 0..3 {
        let (_root, mut app) = settings_test_app(Tab::Camera);
        let prefix = invalid_media(resource, &mut app);
        let status = app.status.lock().unwrap().clone();
        let context = egui::Context::default();
        context.enable_accesskit();
        let output = context.run(egui::RawInput::default(), |context| {
            app.show_footer(context, &status)
        });
        assert!(
            error_visible(&output, prefix),
            "T1553 resource={resource}: missing actionable error"
        );
        let nodes = output.platform_output.accesskit_update.unwrap().nodes;
        let label = settings::apply_label(status.daemon_running, &app.cfg, &app.saved_cfg);
        let save = nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .unwrap();
        assert!(save.1.is_disabled(), "T1553: invalid Save enabled");
        assert!(!app.busy());
        assert!(!app.store.path().unwrap().exists());
    }
}
