//! T1559: Video consumes the frame snapshot, including poison recovery.
use super::*;

#[test]
fn t1559_poisoned_status_does_not_panic_video_or_advanced_controls() {
    let (_directory, mut app) = settings_test_app(Tab::Video);
    let status = app.status.clone();
    assert!(std::thread::spawn(move || {
        let _guard = status.lock().unwrap();
        panic!("T1559 injected status producer panic");
    })
    .join()
    .is_err());
    assert!(app.status.is_poisoned());
    let context = egui::Context::default();
    context.style_mut(|style| style.animation_time = 0.0);
    let labels = video_test_frame(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(label, _)| label == "Encoding GPU"));
    click_settings_text(
        &mut app,
        &context,
        "Advanced video settings",
        video_test_frame,
    );
    let labels = video_test_frame(&mut app, &context, vec![]);
    assert!(labels
        .iter()
        .any(|(label, _)| label.contains("Effective capacity:")));
    assert!(
        app.status.is_poisoned(),
        "rendering must not hide producer failure"
    );
}
