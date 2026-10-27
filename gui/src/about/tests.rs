use super::*;
use crate::tests::collect_text_rects;
use std::path::{Path, PathBuf};

fn files(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn t1277_embedded_notices_match_all_authoritative_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut expected = vec![
        root.join("LICENSE"),
        root.join("COPYRIGHT.md"),
        root.join("THIRD_PARTY_LICENSES.md"),
    ];
    files(&root.join("licenses"), &mut expected);
    files(
        &root.join("android/app/src/main/assets/licenses"),
        &mut expected,
    );
    assert_eq!(expected.len(), DOCUMENTS.len());
    for path in expected {
        let name = path.strip_prefix(root).unwrap().to_str().unwrap();
        let matches: Vec<_> = DOCUMENTS
            .iter()
            .filter(|(candidate, _)| *candidate == name)
            .collect();
        assert_eq!(matches.len(), 1, "T1277: {name}");
        assert_eq!(matches[0].1, std::fs::read_to_string(path).unwrap());
    }
    assert!(DOCUMENTS.iter().all(|(_, text)| text.len() <= 256 * 1024));
    assert!(DOCUMENTS.iter().map(|(_, text)| text.len()).sum::<usize>() <= 2 * 1024 * 1024);
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    mut render: impl FnMut(&mut egui::Ui),
) -> Vec<(String, egui::Rect)> {
    let output = ctx.run(
        egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(380.0, 760.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| render(ui));
        },
    );
    for command in output.platform_output.commands {
        match command {
            egui::OutputCommand::CopyText(text) => {
                ctx.data_mut(|data| data.insert_temp(egui::Id::new("copied"), text))
            }
            egui::OutputCommand::OpenUrl(url) => {
                ctx.data_mut(|data| data.insert_temp(egui::Id::new("opened"), url.url))
            }
            _ => {}
        }
    }
    let mut labels = Vec::new();
    for shape in output.shapes {
        collect_text_rects(&shape.shape, &mut labels);
    }
    labels
}

fn click(ctx: &egui::Context, label: &str, render: fn(&mut egui::Ui)) {
    frame(ctx, vec![], render);
    let labels = frame(ctx, vec![], render);
    let pos = labels
        .iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("{label}: {labels:?}"))
        .1
        .center();
    for pressed in [true, false] {
        frame(
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            render,
        );
    }
}

#[test]
fn t1277_version_and_full_notice_can_be_copied_without_network() {
    let ctx = egui::Context::default();
    click(&ctx, "Copy version details", identity);
    assert_eq!(
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("copied")))
            .unwrap(),
        build_identity()
    );
    click(&ctx, "Copy full notice", document_viewer);
    assert_eq!(
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("copied")))
            .unwrap(),
        include_str!("../../../LICENSE")
    );
    assert!(ctx
        .data(|data| data.get_temp::<String>(egui::Id::new("opened")))
        .is_none());
}

#[test]
fn t1277_every_document_and_invalid_selection_are_bounded() {
    let ctx = egui::Context::default();
    for index in (0..DOCUMENTS.len()).chain([DOCUMENTS.len(), usize::MAX]) {
        let labels = frame(&ctx, vec![], |ui| {
            let id = ui.make_persistent_id("about-document");
            ui.data_mut(|data| data.insert_temp(id, index));
            document_viewer(ui);
            assert!(ui.data(|data| data.get_temp::<usize>(id)).unwrap() < DOCUMENTS.len());
        });
        for (text, rect) in labels {
            assert!(rect.right() <= 380.0, "T1277 clipped {text}: {rect:?}");
        }
    }
}

#[test]
fn t1277_document_picker_reaches_full_copyright_notice() {
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    click(&ctx, "LICENSE", document_viewer);
    for (text, rect) in frame(&ctx, vec![], document_viewer) {
        assert!(
            rect.right() <= 380.0,
            "T1277 picker clipped {text}: {rect:?}"
        );
    }
    click(&ctx, "COPYRIGHT.md", document_viewer);
    click(&ctx, "Copy full notice", document_viewer);
    assert_eq!(
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("copied")))
            .unwrap(),
        include_str!("../../../COPYRIGHT.md")
    );
}

fn bundle_fixture(ui: &mut egui::Ui) {
    let available = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(egui::Id::new("available")))
        .unwrap_or(false);
    bundle_link(ui, || {
        if available {
            Ok("file:///owned/notices".into())
        } else {
            Err("Bundle notices unavailable".into())
        }
    });
}

#[test]
fn t1277_missing_bundle_notices_stay_explicit_and_retry_clears_error() {
    let ctx = egui::Context::default();
    click(&ctx, "Open installed bundle notices", bundle_fixture);
    let labels = frame(&ctx, vec![], bundle_fixture);
    assert!(labels
        .iter()
        .any(|(text, _)| text == "Bundle notices unavailable"));
    assert!(ctx
        .data(|data| data.get_temp::<String>(egui::Id::new("opened")))
        .is_none());
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("available"), true));
    click(&ctx, "Open installed bundle notices", bundle_fixture);
    let labels = frame(&ctx, vec![], bundle_fixture);
    assert!(!labels
        .iter()
        .any(|(text, _)| text == "Bundle notices unavailable"));
    assert_eq!(
        ctx.data(|data| data.get_temp::<String>(egui::Id::new("opened")))
            .unwrap(),
        "file:///owned/notices"
    );
}

fn key(key: egui::Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

#[test]
fn t1277_notice_copy_is_keyboard_accessible() {
    let ctx = egui::Context::default();
    frame(&ctx, vec![], document_viewer);
    let labels = frame(&ctx, vec![], document_viewer);
    let target = labels
        .iter()
        .find(|(text, _)| text == "Copy full notice")
        .unwrap()
        .1;
    for _ in 0..12 {
        frame(&ctx, key(egui::Key::Tab), document_viewer);
        let focused = ctx
            .memory(|memory| memory.focused())
            .and_then(|id| ctx.read_response(id));
        if focused.is_some_and(|response| response.rect.contains(target.center())) {
            frame(&ctx, key(egui::Key::Enter), document_viewer);
            assert_eq!(
                ctx.data(|data| data.get_temp::<String>(egui::Id::new("copied")))
                    .unwrap(),
                DOCUMENTS[0].1
            );
            return;
        }
    }
    panic!("T1277 Copy full notice is not keyboard reachable");
}

fn t1291_notice_clip(app: &mut crate::App, ctx: &egui::Context, size: egui::Vec2) -> egui::Rect {
    crate::workspace_style::configure(ctx);
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        },
        |ctx| app.show_window(ctx),
    );
    for shape in output.shapes {
        let mut texts = Vec::new();
        collect_text_rects(&shape.shape, &mut texts);
        if texts.iter().any(|(text, _)| {
            text.split_whitespace()
                .eq(DOCUMENTS[0].1.split_whitespace())
        }) {
            return shape.clip_rect;
        }
    }
    panic!("T1291 notice text missing");
}

#[test]
fn t1291_notice_reader_adapts_to_tall_and_compact_windows() {
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::About);
    let context = egui::Context::default();
    for height in [2088.0, 900.0, 740.0, 1100.0, 2088.0] {
        let size = egui::vec2(1040.0, height);
        for _ in 0..3 {
            t1291_notice_clip(&mut app, &context, size);
        }
        let clip = t1291_notice_clip(&mut app, &context, size);
        assert!(
            clip.bottom() <= height,
            "T1291 reader exceeds viewport: {clip:?}"
        );
        // T1293 supersedes the separate reader height with one page viewport.
        assert!(
            clip.top() < 150.0,
            "T1293 notice still owns an inner viewport: {clip:?}"
        );
        assert!(
            clip.height() > height / 2.0,
            "T1293 notice clipped within page: {clip:?}"
        );
        if height >= 2088.0 {
            let labels = crate::tests::sized_window_frame(&mut app, &context, vec![], size);
            let notice = labels
                .iter()
                .find(|(text, _)| {
                    text.split_whitespace()
                        .eq(DOCUMENTS[0].1.split_whitespace())
                })
                .unwrap()
                .1;
            assert!(
                clip.contains_rect(notice),
                "T1293 full MIT notice should fit: {notice:?}"
            );
            let bundle = labels
                .iter()
                .find(|(text, _)| text == "Open installed bundle notices")
                .unwrap()
                .1;
            assert!(
                bundle.bottom() < height,
                "T1293 bundle action exceeds tall page: {bundle:?}"
            );
        }
    }
}

fn t1293_about_output(
    app: &mut crate::App,
    ctx: &egui::Context,
    size: egui::Vec2,
) -> egui::FullOutput {
    crate::workspace_style::configure(ctx);
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        },
        |ctx| app.show_window(ctx),
    )
}

#[test]
fn t1293_about_selector_uses_available_page_width_after_resizes() {
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::About);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    for width in [1040.0, 3840.0, 380.0, 1920.0, 1040.0] {
        let size = egui::vec2(width, 2088.0);
        for _ in 0..3 {
            t1293_about_output(&mut app, &ctx, size);
        }
        let output = t1293_about_output(&mut app, &ctx, size);
        let tree = output.platform_output.accesskit_update.unwrap();
        let picker = tree
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == egui::accesskit::Role::ComboBox)
            .unwrap()
            .bounds()
            .unwrap();
        assert!(
            picker.width() >= f64::from(width - 32.0),
            "T1293 selector remains capped: {picker:?}"
        );
        assert!(
            picker.x0 >= 8.0 && picker.x1 <= f64::from(width),
            "T1293 selector exceeds page: {picker:?}"
        );
    }
}

#[test]
fn t1294_mit_text_stays_left_and_reflows_without_losing_content() {
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::About);
    let ctx = egui::Context::default();
    for width in [3840.0, 1040.0, 380.0, 1920.0] {
        let size = egui::vec2(width, 2088.0);
        for _ in 0..3 {
            t1293_about_output(&mut app, &ctx, size);
        }
        let labels = crate::tests::sized_window_frame(&mut app, &ctx, vec![], size);
        let (text, rect) = labels
            .iter()
            .find(|(text, _)| text.starts_with("MIT License\n"))
            .unwrap();
        assert!(
            (rect.left() - 8.0).abs() < 1.0,
            "T1294 notice centered: {rect:?}"
        );
        assert!(
            rect.right() <= width,
            "T1294 notice exceeds viewport: {rect:?}"
        );
        assert!(
            text.split_whitespace()
                .eq(DOCUMENTS[0].1.split_whitespace()),
            "T1294 license content changed"
        );
        assert_eq!(
            text.split("\n\n").count(),
            DOCUMENTS[0].1.split("\n\n").count()
        );
        if width == 3840.0 {
            assert!(
                rect.width() > 1000.0,
                "T1294 MIT prose remains hard-wrapped: {rect:?}"
            );
        }
    }
}

#[test]
fn t1294_other_notices_preserve_original_formatting() {
    for (index, document) in DOCUMENTS.iter().enumerate().skip(1) {
        let shown = notice_display_text(index);
        assert!(matches!(shown, std::borrow::Cow::Borrowed(_)));
        assert_eq!(shown.as_ref(), document.1);
    }
}
