//! T1555: visible action controls agree with the shared busy admission gate.
use super::*;

fn controls(app: &mut App, running: bool) -> Vec<egui::accesskit::Node> {
    let context = egui::Context::default();
    context.enable_accesskit();
    let status = app.status.lock().unwrap().clone();
    let output = context.run(egui::RawInput::default(), |context| {
        egui::CentralPanel::default().show(context, |ui| {
            app.show_daemon_control(ui, running);
            app.show_system_setup(ui, &status);
            egui::Grid::new("t1555-autostart")
                .show(ui, |ui| app.setting_plug_and_play(ui, &status));
        });
    });
    output
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, node)| node)
        .collect()
}

fn daemon_label(running: bool) -> &'static str {
    if !capabilities().daemon {
        return "Start (unavailable)";
    }
    match (capabilities().display, running) {
        (true, true) => "Stop display service (all tablets)",
        (true, false) => "Start display service (all tablets)",
        (false, true) => "Stop daemon",
        (false, false) => "Start daemon",
    }
}

fn disabled(nodes: &[egui::accesskit::Node], label: &str) -> bool {
    nodes
        .iter()
        .find(|node| node.label() == Some(label))
        .unwrap()
        .is_disabled()
}

fn verify_controls(mode: u8, running: bool) {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let (sender, receiver) = std::sync::mpsc::channel();
    match mode {
        1 => app.action = Some(receiver),
        2 => app.apply(false),
        _ => {}
    }
    let nodes = controls(&mut app, running);
    drop(sender);
    wait_for_work(&mut app);
    assert_eq!(
        disabled(&nodes, daemon_label(running)),
        mode != 0 || !capabilities().daemon,
        "T1555 daemon mode={mode}"
    );
    assert_eq!(
        disabled(&nodes, "Set up display and input (asks for password)"),
        mode != 0,
        "T1555 setup mode={mode}"
    );
    assert_eq!(
        disabled(&nodes, "Start Blent with the desktop"),
        mode != 0 || !capabilities().autostart,
        "T1555 autostart mode={mode}"
    );
}

#[test]
fn t1555_daemon_setup_and_autostart_controls_follow_busy_state() {
    for mode in 0..=2 {
        for running in [false, true] {
            verify_controls(mode, running);
        }
    }
}
