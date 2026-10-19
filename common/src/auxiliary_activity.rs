//! Portable manual-intent progress; phases grant neither media nor OS authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Intent retained; no attempt admitted until machine availability permits it.
    /// Any previous attempt and its native callbacks have actually settled.
    Paused,
    /// One attempt admitted; its native reports describe setup/media readiness.
    Starting,
    /// Cutoff or logical completion; original work/native cleanup still owns its
    /// resources. This phase never certifies retirement.
    Retiring,
    /// Manual intent ended after its original native obligations settled.
    Finished,
}

#[cfg(test)]
mod tests {
    #[test]
    fn t1085_only_retained_manual_intents_keep_stop_available() {
        use crate::{audio::AudioState as Audio, camera::CameraState as Camera};
        for state in [
            Camera::Starting,
            Camera::Paused,
            Camera::Waiting,
            Camera::Streaming,
            Camera::Stopping,
        ] {
            assert!(state.is_active());
        }
        for state in [Camera::Stopped, Camera::Failed("unavailable".into())] {
            assert!(!state.is_active());
        }
        for state in [
            Audio::Starting,
            Audio::Paused,
            Audio::Streaming,
            Audio::Stopping,
        ] {
            assert!(state.is_active());
        }
        for state in [Audio::Stopped, Audio::Failed] {
            assert!(!state.is_active());
        }
    }
}
