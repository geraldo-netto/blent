#[test]
fn t1552_external_snapshot_rebases_draft_and_discard_baseline() {
    let (_directory, mut app) = settings_test_app(Tab::Video);
    let edited_width = app.cfg.width;
    let mut external = app.saved_cfg.clone();
    external.height += 8;
    external.fps = 47;
    app.store.save(&external).unwrap();
    app.status.lock().unwrap().config = Some(crate::config_refresh::Snapshot {
        started: std::time::Instant::now(),
        config: Arc::new(app.store.load()),
    });
    let ctx = egui::Context::default();
    window_test_frame(&mut app, &ctx, vec![]);
    assert_eq!(app.cfg.width, edited_width, "T1552 unsaved local edit lost");
    assert_eq!(app.cfg.height, external.height, "T1552 external height remained stale");
    assert_eq!(app.cfg.fps, 47);
    assert_eq!(app.saved_cfg, external, "T1552 Discard must use the new baseline");
}

#[test]
fn t1552_save_completion_rejects_earlier_observations() {
    let (_directory, mut app) = settings_test_app(Tab::Video);
    app.store.save(&app.saved_cfg).unwrap();
    let stale = crate::config_refresh::Snapshot::read(&app.store).unwrap();
    app.apply(false);
    app.refresh_config(Some(&stale));
    assert_ne!(app.cfg, *stale.config);
    while !app.save.as_ref().unwrap().is_finished() {
        std::thread::sleep(Duration::from_millis(1));
    }
    app.poll_save();
    let saved = app.saved_cfg.clone();
    app.refresh_config(Some(&stale));
    assert_eq!(app.saved_cfg, saved, "T1552 old read overwrote a completed save");
    assert_eq!(app.cfg, saved);
    let current = crate::config_refresh::Snapshot::read(&app.store).unwrap();
    app.refresh_config(Some(&current));
    assert_eq!(app.cfg, saved);
    assert_eq!(app.config_observed_after, Some(current.started));
}

#[test]
fn t1552_invalid_snapshot_keeps_draft_and_snapshot_times_do_not_repaint() {
    let (_directory, mut app) = settings_test_app(Tab::Video);
    let original = app.cfg.clone();
    app.store.save(&app.saved_cfg).unwrap();
    let first = crate::config_refresh::Snapshot::read(&app.store).unwrap();
    let second = crate::config_refresh::Snapshot::read(&app.store).unwrap();
    assert_eq!(first, second, "T1552 timestamps cause idle repaint");
    std::fs::write(app.store.path().unwrap(), "invalid = [").unwrap();
    let bad = crate::config_refresh::Snapshot::read(&app.store);
    assert!(bad.is_none());
    app.refresh_config(bad.as_ref());
    assert_eq!(app.cfg, original);
}

#[test]
fn t1552_repeated_observation_preserves_new_edits_and_discard_is_current() {
    let (_directory, mut app) = settings_test_app(Tab::Video);
    let external = FileConfig { fps: 45, ..app.saved_cfg.clone() };
    app.store.save(&external).unwrap();
    let current = crate::config_refresh::Snapshot::read(&app.store).unwrap();
    app.status.lock().unwrap().config = Some(current.clone());
    app.refresh_config(Some(&current));
    app.cfg.fps = 30;
    app.refresh_config(Some(&current));
    assert_eq!(app.cfg.fps, 30);
    let ctx = egui::Context::default();
    window_test_frame(&mut app, &ctx, vec![]);
    click_settings_text(&mut app, &ctx, "Discard", window_test_frame);
    assert_eq!(app.cfg, external);
}
