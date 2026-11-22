//! T1787: names/values, disabled reasons and keyboard focus in the emitted tree.
use super::*;
use egui::accesskit::{Node, NodeId, Role};

fn field_frame(app: &mut App, context: &egui::Context, events: Vec<egui::Event>) -> egui::accesskit::TreeUpdate {
    context.enable_accesskit();
    context.run(egui::RawInput { events, ..Default::default() }, |context| {
        egui::CentralPanel::default().show(context, |ui| {
            egui::Grid::new("accessible-fields").show(ui, |ui| {
                app.setting_encoder(ui);
                app.setting_frame_rate(ui);
            });
        });
    }).platform_output.accesskit_update.unwrap()
}

fn named<'a>(nodes: &'a [(NodeId, Node)], role: Role, name: &str) -> (NodeId, &'a Node) {
    let label = nodes.iter().find(|(_, node)| node.role() == Role::Label && (node.label() == Some(name) || node.value() == Some(name)))
        .unwrap_or_else(|| panic!("T1787 missing label {name}: {nodes:?}")).0;
    let (id, node) = nodes.iter().find(|(_, node)| node.role() == role && node.labelled_by().contains(&label))
        .unwrap_or_else(|| panic!("T1787 missing associated {name}: {nodes:?}"));
    (*id, node)
}

#[test]
fn t1787_encoder_and_frame_rate_have_names_values_and_keyboard_focus() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    let context = egui::Context::default();
    let tree = field_frame(&mut app, &context, vec![]);
    let (encoder, node) = named(&tree.nodes, Role::ComboBox, "Encoder");
    assert!(node.value().is_some_and(|value| !value.is_empty()));
    assert!(node.supports_action(egui::accesskit::Action::Focus));
    let (fps, node) = named(&tree.nodes, Role::ComboBox, "Frame rate");
    assert_eq!(node.value(), Some("60 fps"));
    field_frame(&mut app, &context, vec![egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Focus, target: encoder, data: None,
    })]);
    let tree = field_frame(&mut app, &context, overview::key(egui::Key::Tab));
    assert_eq!(tree.focus, fps, "T1787 Tab follows visible field order");
}

fn tall_camera(app: &mut App, context: &egui::Context, events: Vec<egui::Event>) -> Vec<(String, egui::Rect)> {
    sized_window_frame(app, context, events, egui::vec2(1040.0, 2000.0))
}

#[test]
fn t1787_camera_fields_retain_values_and_explain_disabled_controls() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    app.cfg.camera.options.shadow_mode = blent_config::camera::ShadowMode::Off;
    app.cfg.camera.options.adaptive_bitrate = false;
    let context = egui::Context::default();
    context.enable_accesskit();
    click_settings_text(&mut app, &context, "Advanced camera settings", tall_camera);
    let output = context.run(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1040.0, 2000.0))),
        ..Default::default()
    }, |context| app.show_window(context));
    let nodes = output.platform_output.accesskit_update.unwrap().nodes;
    for name in ["Shadow strength", "Image processing threads", "Frame rate", "Camera bitrate ceiling", "Camera bitrate floor", "Freshness budget"] {
        let (_, node) = named(&nodes, Role::Slider, name);
        assert!(node.numeric_value().is_some(), "T1787 missing {name} value");
        assert!(node.description().is_some_and(|text| !text.is_empty()));
    }
    for name in ["Shadow strength", "Camera bitrate floor"] { assert!(named(&nodes, Role::Slider, name).1.is_disabled()); }
    for name in ["Camera resolution", "Rotate clockwise"] { assert!(named(&nodes, Role::ComboBox, name).1.value().is_some()); }
    assert!(app.camera.backend.is_none());
}

#[test]
fn t1787_navigation_announces_parent_and_current_child_selection() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    let context = egui::Context::default();
    context.enable_accesskit();
    let output = context.run(Default::default(), |context| app.show_window(context));
    let nodes = output.platform_output.accesskit_update.unwrap().nodes;
    for name in ["Settings", "Cameras"] {
        assert!(nodes.iter().any(|(_, node)| node.label() == Some(name) && node.is_selected() == Some(true)), "T1787 {name} selection");
    }
    assert!(nodes.iter().any(|(_, node)| node.label() == Some("Tablets") && node.is_selected() == Some(false)));
}
