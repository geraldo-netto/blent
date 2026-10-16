//! Encoder acquisition scope is independent of retained helper-monitor ownership.
use blent::capture_contract::NativeRetirement;
use blent::native_scope::Scope;
use std::sync::Arc;

type Report = Arc<dyn NativeRetirement>;
#[derive(Default)]
pub(super) struct Generation {
    active: Option<Report>,
    original: Option<blent::native_scope::Retirement>,
}
impl Generation {
    pub(super) fn begin(&mut self, parent: &Option<Report>) -> anyhow::Result<Scope> {
        self.ensure_retired()?;
        let (scope, original) = Scope::new(parent.clone()).group();
        self.original = Some(original);
        Ok(scope)
    }
    pub(super) fn activate(&mut self, scope: Scope) {
        self.active = Some(Arc::new(scope));
    }
    pub(super) fn report(&self) -> Option<Report> {
        self.active.clone()
    }
    pub(super) fn close(&mut self) {
        self.active = None;
    }
    pub(super) fn ensure_retired(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.active.is_none(), "encoder generation still owned");
        anyhow::ensure!(
            self.original
                .as_ref()
                .is_none_or(blent::native_scope::Retirement::is_complete),
            "previous encoder generation native retirement incomplete"
        );
        Ok(())
    }
}
