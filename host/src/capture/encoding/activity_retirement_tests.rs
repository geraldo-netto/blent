//! T1015: actual native startup remains owned after whole manager shutdown/drop.
use super::*;
use crate::capture::{activity_tests as fixture, ownership_tests::isolated};
use blent_config::host_activity::*;
use std::{
    sync::{mpsc, Mutex},
    time::Duration,
};
use tokio::sync::watch;

#[derive(Default)]
struct Report(Mutex<Vec<blent_config::retirement::Completion>>);
impl blent::capture_contract::NativeRetirement for Report {
    fn retain(&self, original: blent_config::retirement::Completion) {
        self.0.lock().unwrap().push(original);
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[tokio::test]
async fn t1015_whole_manager_shutdown_drop_retains_original_late_native_startup() {
    if isolated("capture::encoding::activity_retirement_tests::t1015_whole_manager_shutdown_drop_retains_original_late_native_startup") { return; }
    let mut manager = fixture::manager();
    let report = Arc::new(Report::default());
    manager.bind_retirement(report.clone());
    let scope = manager.encoder.begin_generation().unwrap();
    let creator = scope.clone();
    manager.encoder.generation.activate(scope);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let startup = tokio::spawn(async move {
        creator
            .blocking(move |scope| {
                drop(scope);
                entered.send(()).unwrap();
                held.recv().unwrap();
            })
            .await
    });
    ready.await.unwrap();
    startup.abort();
    assert!(startup.await.unwrap_err().is_cancelled());
    let original = report.0.lock().unwrap()[0].clone();
    let mut feed = Feed::new(1, fixture::capabilities(), Settings::default()).unwrap();
    fixture::observe(&mut feed, 1, false);
    let (published, source) = watch::channel(fixture::publication(&feed));
    manager.bind_display_work(Arc::new(blent::display_work::View::new(
        Some(source),
        None,
        false,
    )));
    let (_settings, settings) = watch::channel(fixture::settings(&manager.config));
    let (_mode, display) = watch::channel(true);
    let (stop, stopped) = watch::channel(false);
    let (video, _received) = crate::video_queue::channel(8, Default::default());
    let fifo = crate::runtime::fifo_path_for(manager.config.instance).unwrap();
    let task = tokio::spawn(async move {
        let result = manager
            .stream_frames(video.into(), settings, display, stopped)
            .await;
        manager.shutdown().await;
        drop(manager);
        result
    });
    tokio::task::yield_now().await;
    fixture::observe(&mut feed, 2, true);
    published.send_replace(fixture::publication(&feed));
    fixture::observe(&mut feed, 3, false);
    published.send_replace(fixture::publication(&feed));
    tokio::time::sleep(Duration::from_millis(30)).await;
    let denied = !fifo.exists() && !original.is_complete();
    stop.send_replace(true);
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let retained = !original.is_complete();
    drop(release);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        denied,
        "T1015 resumed before original native startup retirement"
    );
    assert!(
        retained,
        "T1015 whole manager shutdown/drop manufactured native completion"
    );
}
