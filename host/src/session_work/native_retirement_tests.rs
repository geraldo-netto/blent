//! T1026: production session construction retains backend native evidence.
use super::*;
use blent_config::retirement::{Completion, Pool};

struct NativeCapture {
    original: Completion,
    panic: bool,
}

impl CaptureBackend for NativeCapture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        context.retirement.retain(self.original);
        assert!(
            !self.panic,
            "T1026 native exposure before constructor panic"
        );
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                context.stop.wait_for(|stop| *stop).await.unwrap();
                Ok(())
            }),
        }
    }
}

#[tokio::test]
async fn t1026_constructor_panic_keeps_original_native_admission() {
    let native = Pool::new(1).unwrap();
    let original = native.reserve().unwrap();
    let admission = crate::session::Workers::reserve().unwrap();
    let completion = admission.completion();
    let (_stop, stopped) = watch::channel(false);
    let result = prepared(
        Box::new(NativeCapture {
            original: original.completion(),
            panic: true,
        }),
        (0, 0),
    )
    .start_exported_reserved(stopped, admission)
    .await;
    let retained = !completion.is_complete();
    drop(original);
    completed(&completion).await;
    assert!(result.is_err());
    assert!(
        retained,
        "T1026 startup panic released original native resource"
    );
}

async fn completed(original: &Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

async fn supervisor_finished(work: &RuntimeWork) {
    let Binding::Owned(runtime) = &work.binding else {
        panic!("T1026 requires owned runtime")
    };
    tokio::time::timeout(Duration::from_secs(3), async {
        while !runtime.finished() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1026_route_retirement_waits_for_native_after_supervisor_finished() {
    let native = Pool::new(1).unwrap();
    let original = native.reserve().unwrap();
    let (_daemon, stop) = watch::channel(false);
    let (runtime, listeners) = prepared(
        Box::new(NativeCapture {
            original: original.completion(),
            panic: false,
        }),
        (0, 0),
    )
    .start_exported(stop)
    .await
    .unwrap();
    let completion = runtime.retirement.clone();
    let mut work = RuntimeWork::owned(runtime);
    work.close();
    supervisor_finished(&work).await;
    let held_route = !work.retired();
    let held_native = !completion.is_complete();
    drop(original);
    retired(&work).await;
    assert!(
        held_route && held_native,
        "T1026 parent supervisor replaced actual native retirement"
    );
    assert!(completion.is_complete());
    drop(listeners);
}
