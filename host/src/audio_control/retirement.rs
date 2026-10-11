//! Observes independent portable completion proofs. The observer owns no device,
//! native implementation, cancellation authority or resource lifetime itself.
use anyhow::{ensure, Result};
use blent_config::retirement::Completion;

#[derive(Default)]
pub(super) struct Tracker {
    // One original connection owner and one isolated native helper per direction.
    completions: [Option<Completion>; 2],
}
impl Tracker {
    pub(super) fn retain(&mut self, completion: Completion) -> Result<()> {
        let slot = self
            .completions
            .iter_mut()
            .find(|slot| slot.as_ref().is_none_or(Completion::is_complete));
        ensure!(
            slot.is_some(),
            "audio retirement observation capacity retained"
        );
        *slot.unwrap() = Some(completion);
        Ok(())
    }
    pub(super) fn complete(&self) -> bool {
        self.completions
            .iter()
            .all(|proof| proof.as_ref().is_none_or(Completion::is_complete))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audio_control::{Controller, Report},
        control_workers::WorkerPool,
    };
    use blent_config::{
        audio::{AudioController, AudioOptions, AudioState, Direction},
        retirement::Pool,
    };
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };
    fn wait(condition: impl Fn() -> bool) {
        let end = Instant::now() + Duration::from_secs(3);
        while !condition() {
            assert!(Instant::now() < end, "T995 bounded controller wait");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn t995_audio_error_keeps_original_admission_until_actual_completion_without_stalling_sibling()
    {
        joined(0);
    }
    #[test]
    fn t995_audio_success_and_panic_keep_original_admission_until_actual_completion() {
        for mode in [1, 2] {
            joined(mode);
        }
    }
    fn speaker(
        workers: WorkerPool,
        proof: Completion,
        started: Arc<AtomicUsize>,
        mode: u8,
    ) -> Controller {
        Controller::with_pool(workers.clone(), move |_, _, report: Report| {
            let proof = proof.clone();
            let started = started.clone();
            async move {
                report.retain(proof)?;
                started.fetch_add(1, Ordering::SeqCst);
                match mode {
                    0 => Err(anyhow::anyhow!("actual native cleanup pending")),
                    1 => Ok(()),
                    _ => panic!("T995 backend failure with original native cleanup pending"),
                }
            }
        })
        .unwrap()
    }
    fn joined(mode: u8) {
        let native = Pool::new(1).unwrap();
        let owner = native.reserve().unwrap();
        let proof = owner.completion();
        let entered = Arc::new(AtomicUsize::new(0));
        let started = entered.clone();
        let workers = WorkerPool::new(2).unwrap();
        let speaker = speaker(workers.clone(), proof, started, mode);
        let other = Arc::new(AtomicUsize::new(0));
        let observed = other.clone();
        let microphone = Controller::with_pool(workers, move |_, _, _| {
            observed.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Err(anyhow::anyhow!("independent microphone finished")))
        })
        .unwrap();
        speaker
            .start(AudioOptions::new(Direction::Speakers))
            .unwrap();
        wait(|| {
            entered.load(Ordering::SeqCst) == 1
                && matches!(
                    speaker.status().state,
                    AudioState::Stopping | AudioState::Failed
                )
        });
        let held = speaker
            .start(AudioOptions::new(Direction::Speakers))
            .is_err();
        microphone
            .start(AudioOptions::new(Direction::Microphone))
            .unwrap();
        wait(|| other.load(Ordering::SeqCst) == 1);
        let original = speaker.shutdown();
        let unaffected = microphone.shutdown();
        let retained = !original.is_complete();
        drop(owner);
        wait(|| original.is_complete() && unaffected.is_complete());
        assert!(
            held,
            "T995 incomplete native retirement released original Start admission"
        );
        assert!(
            retained,
            "T995 logical backend error counted as actual worker cleanup"
        );
    }
    #[test]
    fn t995_audio_observation_capacity_and_replacement_require_real_completion() {
        let pool = Pool::new(3).unwrap();
        let mut tracker = Tracker::default();
        assert!(tracker.complete());
        let first = pool.reserve().unwrap();
        let second = pool.reserve().unwrap();
        let third = pool.reserve().unwrap();
        tracker.retain(first.completion()).unwrap();
        tracker.retain(second.completion()).unwrap();
        assert!(!tracker.complete());
        assert!(tracker.retain(third.completion()).is_err());
        drop(first);
        tracker.retain(third.completion()).unwrap();
        assert!(!tracker.complete());
        drop(second);
        drop(third);
        assert!(tracker.complete());
    }
}

#[cfg(test)]
mod panic_tests {
    use crate::audio_control::{Controller, Report};
    use blent_config::{
        audio::{AudioController, AudioOptions, AudioState, Direction},
        retirement::{Admission, Pool},
    };
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
        time::{Duration, Instant},
    };
    struct OwnedPanic {
        owner: Arc<Mutex<Option<Admission>>>,
    }
    impl Future for OwnedPanic {
        type Output = anyhow::Result<()>;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            panic!("T995 panic while original future owns native proof")
        }
    }
    impl Drop for OwnedPanic {
        fn drop(&mut self) {
            self.owner.lock().unwrap().take();
        }
    }
    #[test]
    fn t995_panicking_backend_future_drops_owned_native_guard_before_retirement_wait() {
        let pool = Pool::new(1).unwrap();
        let admission = pool.reserve().unwrap();
        let proof = admission.completion();
        let original = Arc::new(Mutex::new(Some(admission)));
        let owned = original.clone();
        let worker = crate::control_workers::WorkerPool::new(1).unwrap();
        let controller = Controller::with_pool(worker, move |_, _, report: Report| {
            report.retain(proof.clone()).unwrap();
            OwnedPanic {
                owner: owned.clone(),
            }
        })
        .unwrap();
        controller
            .start(AudioOptions::new(Direction::Speakers))
            .unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        while controller.status().state != AudioState::Failed && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(1));
        }
        let dropped = original.lock().unwrap().is_none();
        original.lock().unwrap().take();
        let complete = controller.shutdown();
        let end = Instant::now() + Duration::from_secs(2);
        while !complete.is_complete() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped,
            "T995 completed panic future still owned guard during retirement wait"
        );
    }
}
