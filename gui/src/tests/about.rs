//! T1276: project navigation must not move when changing pages.
use super::*;

#[test]
fn t1276_navigation_position_does_not_depend_on_selected_page() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let context = egui::Context::default();
    let mut positions = Vec::new();
    for tab in [Tab::Overview, Tab::Connections, Tab::Settings, Tab::About] {
        app.tab = tab;
        window_test_frame(&mut app, &context, vec![]);
        let labels = window_test_frame(&mut app, &context, vec![]);
        positions.push(labels.iter().find(|(text, _)| text == "Tablets").unwrap().1);
    }
    assert!(
        positions.windows(2).all(|pair| pair[0] == pair[1]),
        "T1276: navigation moves: {positions:?}"
    );
}

#[test]
fn t1276_about_is_reachable_and_owns_project_links() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let draft = app.cfg.clone();
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    click_settings_text(&mut app, &context, "About", window_test_frame);
    let labels = window_test_frame(&mut app, &context, vec![]);
    for expected in ["Report compatibility", "Star on GitHub"] {
        assert!(labels.iter().any(|(text, _)| text == expected));
    }
    assert!(!labels.iter().any(|(text, _)| text.starts_with("Save")));
    assert!(!labels.iter().any(|(text, _)| text == "Video"));
    assert_eq!(app.cfg, draft);
    assert!(!app.busy());
    click_settings_text(&mut app, &context, "Settings", window_test_frame);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(!labels
        .iter()
        .any(|(text, _)| text == "Report compatibility"));
}

#[test]
fn t1276_about_links_fit_narrow_windows() {
    for width in [380.0, 440.0, 1040.0] {
        let (_root, mut app) = settings_test_app(Tab::About);
        let context = egui::Context::default();
        let size = egui::vec2(width, 560.0);
        sized_window_frame(&mut app, &context, vec![], size);
        let labels = sized_window_frame(&mut app, &context, vec![], size);
        for (text, rect) in labels {
            assert!(rect.right() <= width, "T1276 clipped {text}: {rect:?}");
        }
        assert!(!app.busy());
    }
}

#[test]
fn t1276_about_navigation_supports_keyboard_activation() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    let target = labels.iter().find(|(text, _)| text == "About").unwrap().1;
    for _ in 0..12 {
        window_test_frame(&mut app, &context, super::overview::key(egui::Key::Tab));
        let response = context
            .memory(|memory| memory.focused())
            .and_then(|id| context.read_response(id));
        if response.is_some_and(|response| response.rect.contains(target.center())) {
            window_test_frame(&mut app, &context, super::overview::key(egui::Key::Enter));
            assert!(app.tab == Tab::About);
            return;
        }
    }
    panic!("T1276 About has no reachable keyboard target");
}

#[test]
fn t1277_about_exposes_version_credits_and_offline_notices() {
    let (_root, mut app) = settings_test_app(Tab::About);
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels
        .iter()
        .any(|(text, _)| text.contains(env!("CARGO_PKG_VERSION"))));
    assert!(labels.iter().any(|(text, _)| text.contains("majmichu1")));
    assert!(labels
        .iter()
        .any(|(text, _)| text.contains("Geraldo Netto")));
    assert!(labels
        .iter()
        .any(|(text, _)| text == "Licenses and notices"));
    assert!(labels.iter().any(|(text, _)| text.contains("MIT")));
}

// T1290: content and save actions share a readable column after live resizes.
#[test]
fn t1290_wide_pages_and_footer_share_bounded_content() {
    for (tab, anchor) in [
        (Tab::Settings, "CPU scheduling"),
        (Tab::Connections, "USB connection"),
        (Tab::About, "Credits"),
    ] {
        let (_root, mut app) = settings_test_app(tab);
        let context = egui::Context::default();
        for width in [1040.0, 3840.0, 380.0, 1920.0, 1040.0] {
            let size = egui::vec2(width, 1100.0);
            for _ in 0..3 {
                sized_window_frame(&mut app, &context, vec![], size);
            }
            let labels = sized_window_frame(&mut app, &context, vec![], size);
            let left = ((width - 960.0) / 2.0).max(0.0);
            let rect = labels.iter().find(|(text, _)| text == anchor).unwrap().1;
            assert!(
                rect.left() >= left,
                "T1290 {anchor} at width {width}: {rect:?}"
            );
            for (text, rect) in &labels {
                assert!(rect.right() <= width, "T1290 clipped {text}: {rect:?}");
            }
            if tab == Tab::Settings {
                let save = labels
                    .iter()
                    .find(|(text, _)| text.starts_with("Save"))
                    .unwrap()
                    .1;
                assert!(
                    save.left() >= left,
                    "T1290 footer detached from form: {save:?}"
                );
            }
        }
    }
}
