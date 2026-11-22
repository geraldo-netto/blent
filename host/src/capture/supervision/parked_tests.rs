//! T1687: parked supervision observes native failure without renewed demand.
use super::*;
use crate::capture::{activity_tests, ownership_tests};
use blent_config::host_activity::Publication;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

struct Parked {
    manager: CaptureManager,
    run: CaptureRun,
    sources: Sources,
    _settings: watch::Sender<crate::media::EncoderSettings>,
    _display: watch::Sender<bool>,
    _stop: watch::Sender<bool>,
    _source: watch::Sender<Publication>,
}
impl Parked {
    async fn new() -> Self {
        let mut manager = activity_tests::manager();
        std::fs::write(
            &manager.config.helper_path,
            r#"#!/usr/bin/python3
import os, pathlib, select, socket, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
control = socket.socket(fileno=int(args[args.index('--capture-activity-fd') + 1]))
print('EVDI_CONNECTED card4294967295', flush=True)
while True:
    if (root / 'exit-parked').exists(): sys.exit(7)
    if (root / 'fail-parked-stdout').exists():
        (root / 'fail-parked-stdout').unlink()
        os.write(1, b'\xff\n')
    ready, _, _ = select.select([control], [], [], 0.01)
    if ready:
        request = control.recv(32)
        if not request: break
        control.send(request)
"#,
        )
        .unwrap();
        let (source, observed) = watch::channel(Publication::Pending);
        manager.bind_display_work(Arc::new(blent::display_work::View::new(
            Some(observed),
            None,
            false,
        )));
        manager.start_helper().await.unwrap();
        manager.helper.set_capture_active(false).await.unwrap();
        let mut run = ownership_tests::restart_run(&manager);
        let (settings, settings_rx) = watch::channel(run.settings_rx.borrow().clone());
        run.settings_rx = settings_rx;
        let (display, display_rx) = watch::channel(true);
        run.display_rx = display_rx;
        let (stop, shutdown_rx) = watch::channel(false);
        run.shutdown_rx = shutdown_rx;
        let sources = Sources::new(manager.display_work.clone(), None);
        Self {
            manager,
            run,
            sources,
            _settings: settings,
            _display: display,
            _stop: stop,
            _source: source,
        }
    }
    async fn step(&mut self) -> anyhow::Result<bool> {
        let (video, _received) = crate::video_queue::channel(8, Default::default());
        self.manager
            .capture_step(&video.into(), &mut self.run, &self.sources, &mut true)
            .await
    }
}

async fn parked_failure(marker: &str) {
    let mut f = Parked::new().await;
    let original = f.manager.helper.child.as_ref().unwrap().completion();
    f.manager.codec_config.publish(Some(vec![1, 2, 3].into()));
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join(marker), b"T1687 private failure while parked").unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), f.step()).await;
    let settled = original.is_complete();
    let retained = f.manager.helper.is_running();
    let headers = f.manager.codec_config.current();
    let mode = f.run.encoder_mode;
    f.manager.shutdown().await;
    assert!(!result
        .expect("T1687 parked supervisor never observed helper failure")
        .unwrap());
    assert!(settled, "T1687 returned before original cleanup");
    assert!(!retained, "T1687 retained failed helper handle");
    assert!(headers.is_none());
    assert!(mode.is_none());
    assert!(
        tokio::time::timeout(Duration::from_millis(30), f.step())
            .await
            .is_err(),
        "T1687 no-demand state spins or restarts capture"
    );
}

#[tokio::test]
async fn t1687_parked_child_exit_is_retired_without_new_demand() {
    if ownership_tests::isolated(
        "capture::supervision::parked_tests::t1687_parked_child_exit_is_retired_without_new_demand",
    ) {
        return;
    }
    parked_failure("exit-parked").await;
}

#[tokio::test]
async fn t1687_parked_stdout_failure_retires_live_original_child() {
    if ownership_tests::isolated("capture::supervision::parked_tests::t1687_parked_stdout_failure_retires_live_original_child") { return; }
    parked_failure("fail-parked-stdout").await;
}
