use super::*;

#[test]
fn t1650_editor_labels_follow_resource_identity_without_rewriting_saved_request() {
    for direction in [Direction::Microphone, Direction::Speakers] {
        let mut panel = Panel::default();
        let opposite = if direction == Direction::Microphone {
            Direction::Speakers
        } else {
            Direction::Microphone
        };
        let mut options = AudioOptions::new(opposite);
        let original = options.clone();
        let context = egui::Context::default();
        let output = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default()
                .show(context, |ui| panel.show(ui, &mut options, direction));
        });
        let mut labels = Vec::new();
        for shape in &output.shapes {
            crate::test_shapes::collect_text_rects(&shape.shape, &mut labels);
        }
        let heading = if direction == Direction::Microphone {
            "Microphone sharing"
        } else {
            "Speakers sharing"
        };
        assert!(
            labels.iter().any(|(label, _)| label == heading),
            "T1650: resource label changed"
        );
        assert_eq!(options, original);
        assert!(panel.backend.is_none());
        assert!(panel.start(options, direction).is_err());
        assert!(panel.backend.is_none());
    }
}
