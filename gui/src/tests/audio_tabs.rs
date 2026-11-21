//! T1664: independent resource tabs retain one authoritative owner per direction.
use super::*;

#[test]
fn t1664_microphone_and_speakers_have_separate_navigation_and_controls() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    let draft = app.cfg.clone();
    for (title, heading, other) in [
        ("Microphone", "Microphone sharing", "Speakers sharing"),
        ("Speakers", "Speakers sharing", "Microphone sharing"),
    ] {
        let tab = Tab::ALL
            .into_iter()
            .find(|tab| tab.title() == title)
            .expect("T1664 missing resource tab");
        click_settings_text(&mut app, &context, title, window_test_frame);
        assert!(app.tab == tab);
        let labels = window_test_frame(&mut app, &context, vec![]);
        assert!(labels.iter().any(|(text, _)| text == heading));
        assert!(!labels.iter().any(|(text, _)| text == other));
        assert_eq!(app.cfg, draft);
        assert!(app.microphone.backend.is_none());
        assert!(app.speakers.backend.is_none());
    }
}
