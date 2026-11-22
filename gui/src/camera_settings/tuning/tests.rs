use super::*;
#[test]
fn t1468_camera_capability_controls_preserve_settings_and_render_all_labels() {
    let context = egui::Context::default();
    let caps = Capabilities {
        minimum: -6,
        maximum: 6,
        numerator: 1,
        denominator: 3,
        center: true,
        noise_mask: 7,
        flicker_mask: 15,
    };
    for capability in [
        None,
        Some(caps),
        Some(Capabilities {
            numerator: 0,
            center: false,
            ..caps
        }),
    ] {
        let mut sensor = Sensor::default();
        let output = context.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::Grid::new("t1468").show(ui, |ui| show(ui, &mut sensor, capability));
            });
        });
        assert!(!output.shapes.is_empty());
        assert_eq!(sensor, Sensor::default());
    }
    for value in [Flicker::Device, Flicker::Auto, Flicker::Hz50, Flicker::Hz60] {
        assert!(!flicker_label(value).is_empty());
    }
}

fn frame(
    context: &egui::Context,
    sensor: &mut Sensor,
    events: Vec<egui::Event>,
    center: bool,
) -> Vec<(String, egui::Rect)> {
    let caps = Capabilities {
        minimum: -6,
        maximum: 6,
        numerator: 1,
        denominator: 3,
        center,
        noise_mask: 7,
        flicker_mask: 15,
    };
    let output = context.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::Grid::new("controls").show(ui, |ui| show(ui, sensor, Some(caps)));
            });
        },
    );
    let mut text = Vec::new();
    for shape in output.shapes {
        crate::test_shapes::collect_text_rects(&shape.shape, &mut text);
    }
    text
}
fn click(context: &egui::Context, sensor: &mut Sensor, pos: egui::Pos2, center: bool) {
    for pressed in [true, false] {
        frame(
            context,
            sensor,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
            center,
        );
    }
}
#[test]
fn t1468_supported_camera_menus_apply_and_reset_without_starting() {
    let context = egui::Context::default();
    let mut sensor = Sensor::default();
    let text = frame(&context, &mut sensor, vec![], true);
    let pos = text
        .iter()
        .find(|(label, _)| label == "Device default")
        .unwrap()
        .1
        .center();
    click(&context, &mut sensor, pos, true);
    let text = frame(&context, &mut sensor, vec![], true);
    let pos = text
        .iter()
        .find(|(label, _)| label == "Center weighted")
        .unwrap()
        .1
        .center();
    click(&context, &mut sensor, pos, true);
    assert_eq!(sensor.metering, Metering::Center);
    let text = frame(&context, &mut sensor, vec![], true);
    let pos = text
        .iter()
        .find(|(label, _)| label == "Device default")
        .unwrap()
        .1
        .center();
    click(&context, &mut sensor, pos, true);
    let text = frame(&context, &mut sensor, vec![], true);
    let pos = text
        .iter()
        .find(|(label, _)| label == "50 Hz")
        .unwrap()
        .1
        .center();
    click(&context, &mut sensor, pos, true);
    assert_eq!(sensor.flicker, Flicker::Hz50);
    let text = frame(&context, &mut sensor, vec![], true);
    let pos = text
        .iter()
        .find(|(label, _)| label == "Reset tablet adjustments")
        .unwrap()
        .1
        .center();
    click(&context, &mut sensor, pos, true);
    assert_eq!(sensor, Sensor::default());
}

#[test]
fn t1468_unavailable_exposure_does_not_silently_rewrite_saved_preference() {
    let context = egui::Context::default();
    let mut sensor = Sensor {
        exposure: 100,
        ..Default::default()
    };
    let caps = Capabilities {
        minimum: 0,
        maximum: 0,
        numerator: 0,
        denominator: 1,
        center: false,
        noise_mask: 1,
        flicker_mask: 1,
    };
    let _ = context.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| exposure(ui, &mut sensor, caps));
    });
    assert_eq!(
        sensor.exposure, 100,
        "Rendering unavailable capabilities mutated saved exposure"
    );
}

#[test]
fn t1472_exposure_text_uses_displayed_ev_units() {
    for (input, expected) in [("1.00", 100), ("100", 200), ("-100", -200)] {
        let context = egui::Context::default();
        let mut sensor = Sensor::default();
        let text = frame(&context, &mut sensor, vec![], true);
        let pos = text
            .iter()
            .find(|(label, _)| label == "+0.00 EV")
            .unwrap()
            .1
            .center();
        click(&context, &mut sensor, pos, true);
        frame(&context, &mut sensor, vec![], true);
        frame(
            &context,
            &mut sensor,
            vec![egui::Event::Text(input.into())],
            true,
        );
        assert_eq!(
            sensor.exposure, expected,
            "T1472 incorrect EV units or advertised bounds: {input}"
        );
    }
}

#[test]
fn t1472_exposure_parser_rejects_nonfinite_and_malformed_text() {
    for text in ["NaN", "inf", "-inf", "1e999", "", "EV", "1x", "1.0 EV EV"] {
        assert_eq!(parse_exposure(text), None);
    }
    for (text, expected) in [
        ("1", 100.0),
        ("+1.00 EV", 100.0),
        (" -0.33 EV ", -33.0),
        ("0", 0.0),
    ] {
        assert_eq!(parse_exposure(text), Some(expected));
    }
}

#[test]
fn t1654_portable_camera_controls_expose_their_rendered_labels() {
    let context = egui::Context::default();
    let mut sensor = Sensor::default();
    let labels = frame(&context, &mut sensor, Vec::new(), true);
    assert!(labels.iter().any(|(name, _)| name == "Noise reduction"));
    assert!(labels.iter().all(|(_, rect)| rect.is_finite()));
    assert_eq!(sensor, Sensor::default());
}

#[test]
fn t1787_sensor_controls_expose_associated_names_and_values() {
    let context = egui::Context::default();
    context.enable_accesskit();
    let caps = Capabilities {
        minimum: -2,
        maximum: 2,
        numerator: 1,
        denominator: 1,
        center: true,
        noise_mask: 7,
        flicker_mask: 15,
    };
    let mut sensor = Sensor::default();
    let output = context.run(Default::default(), |context| {
        egui::CentralPanel::default().show(context, |ui| {
            egui::Grid::new("sensor-names").show(ui, |ui| show(ui, &mut sensor, Some(caps)));
        });
    });
    let nodes = output.platform_output.accesskit_update.unwrap().nodes;
    for name in [
        "Exposure compensation",
        "Exposure metering",
        "Anti-flicker",
        "Noise reduction",
    ] {
        let label = nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::Label && node.value() == Some(name)
            })
            .unwrap()
            .0;
        let control = nodes
            .iter()
            .find(|(_, node)| node.labelled_by().contains(&label))
            .unwrap()
            .1
            .clone();
        assert!(control.numeric_value().is_some() || control.value().is_some());
    }
}
