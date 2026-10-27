//! GUI admission metadata belongs to one panel. Native lifecycle remains in the
//! controller; its consumed-owner fence survives selection and draft changes.
use blent_config::{
    connection::media::Target, retirement::Fence, tablets::Identity, workspace::Activity,
};

#[derive(Default)]
pub(super) struct Owner {
    admitted: bool,
    target: Option<Identity>,
    queued: bool,
    fence: Option<Fence>,
}

pub(super) fn identity(target: Target<'_>) -> Option<Identity> {
    match target {
        Target::Adb(serial) => serial.map(|serial| Identity::Adb(serial.to_owned())),
        Target::Usb(peer) => Some(Identity::UsbPeer(peer)),
    }
}

impl Owner {
    pub(super) fn needs_release(&mut self, target: &Option<Identity>, observed: Activity) -> bool {
        !self.activity(observed).is_active() && self.admitted && self.target != *target
    }
    pub(super) fn target(&self) -> Option<&Identity> {
        self.target.as_ref()
    }

    pub(super) fn prepare(&mut self, target: &Option<Identity>) -> Result<(), String> {
        self.settle();
        if self.fence.is_some() {
            return Err("Original resource is still stopping; wait for cleanup.".into());
        }
        if self.admitted && self.target != *target {
            return Err("Stop the original tablet resource before choosing another tablet.".into());
        }
        Ok(())
    }

    pub(super) fn admit(&mut self, target: Option<Identity>) {
        self.admitted = true;
        self.target = target;
        self.queued = true;
    }

    pub(super) fn release(&mut self, fence: Fence) {
        self.fence = Some(fence);
        self.queued = false;
        self.settle();
    }

    pub(super) fn settle(&mut self) {
        if self.fence.as_ref().is_some_and(Fence::is_complete) {
            self.fence = None;
            self.target = None;
            self.admitted = false;
        }
    }

    pub(super) fn activity(&mut self, observed: Activity) -> Activity {
        self.settle();
        if self.fence.is_some() {
            return Activity::Stopping;
        }
        if observed != Activity::Off {
            self.queued = false;
        }
        if self.queued {
            Activity::Starting
        } else {
            observed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1273_original_media_owner_survives_queued_start_and_retirement() {
        let target = Some(Identity::Adb("first".into()));
        let mut owner = Owner::default();
        owner.prepare(&target).unwrap();
        owner.admit(target.clone());
        assert!(!owner.needs_release(&None, Activity::Off));
        assert_eq!(owner.activity(Activity::Off), Activity::Starting);
        assert!(owner
            .prepare(&Some(Identity::Adb("second".into())))
            .is_err());
        assert_eq!(owner.activity(Activity::Waiting), Activity::Waiting);
        assert_eq!(owner.activity(Activity::Off), Activity::Off);
        assert!(owner.needs_release(&None, Activity::Off));
        assert!(!owner.needs_release(&target, Activity::Off));
        let pool = blent_config::retirement::Pool::new(1).unwrap();
        let admission = pool.reserve().unwrap();
        owner.release(Fence::Pending(admission.completion()));
        assert_eq!(owner.activity(Activity::Off), Activity::Stopping);
        assert!(owner.prepare(&target).is_err());
        assert_eq!(owner.target(), target.as_ref());
        drop(admission);
        owner.prepare(&None).unwrap();
        assert!(owner.target().is_none());
        owner.admit(None);
        owner.release(Fence::Complete);
        assert_eq!(owner.activity(Activity::Off), Activity::Off);
        assert_eq!(identity(Target::Adb(Some("first"))), target);
        assert_eq!(identity(Target::Adb(None)), None);
        let peer = blent_config::connection::pairing::PeerId::parse("a".repeat(64)).unwrap();
        assert_eq!(
            identity(Target::Usb(peer.clone())),
            Some(Identity::UsbPeer(peer))
        );
    }
}
