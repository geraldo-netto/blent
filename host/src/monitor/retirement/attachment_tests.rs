//! T1543: rejected preparations release metadata without retiring native owners early.
use super::*;

fn pending(state: &mut Monitor, serial: &str) -> watch::Receiver<bool> {
    let attachment = attachment::Attachment::new(watch::channel(settings()).0);
    let receiver = attachment.subscribe();
    state.attachments.insert(serial.into(), attachment);
    state
        .slots
        .reserve(DisplayOwner::Adb(serial.into()), 1)
        .unwrap();
    state.pending.insert(
        serial.into(),
        Pending {
            instance: 1,
            session: None,
        },
    );
    receiver
}

fn settings() -> blent::media::EncoderSettings {
    blent::media::EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 3000,
        width: 1280,
        height: 720,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}

async fn settled(state: &mut Monitor) {
    assert!(state.available_slot().is_none());
    let retired = state.retiring.join_next().await.unwrap().unwrap();
    state.retired(retired);
    assert_eq!(state.available_slot(), Some(1));
}

#[tokio::test]
async fn t1543_rejected_preparations_release_each_distinct_attachment() {
    let mut state = super::super::tests::fixture();
    for ordinal in 0..32 {
        let serial = format!("rejected-{ordinal}");
        let receiver = pending(&mut state, &serial);
        state.prepared(serial, false, RelaunchBackoff::default());
        let released = receiver.has_changed().is_err();
        settled(&mut state).await;
        assert!(
            released,
            "T1543 rejected metadata still owns its watch sender"
        );
        assert!(state.attachments.is_empty());
    }
    state.stop().await;
}

#[tokio::test]
async fn t1543_panicked_and_cancelled_preparations_release_metadata() {
    let mut state = super::super::tests::fixture();
    for cancel in [false, true] {
        let receiver = pending(&mut state, "failed");
        let task = tokio::spawn(async move {
            assert!(cancel, "T1543 injected preparation panic");
            std::future::pending::<Mutation>().await
        });
        if cancel {
            task.abort();
        }
        let error = task.await.err().expect("injected failure");
        state.mutation_ready("failed".into(), Err(error));
        let released = receiver.has_changed().is_err();
        settled(&mut state).await;
        assert!(
            released,
            "T1543 failed mutation retained attachment metadata"
        );
        assert!(state.attachments.is_empty());
    }
    state.stop().await;
}
