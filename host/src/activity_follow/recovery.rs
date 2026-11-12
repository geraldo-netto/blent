//! One admitted attempt at a time; retries wait for original native settlement.
use super::*;

struct Recovery {
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    preferences: blent_config::host_activity::Settings,
    shared: Arc<Shared>,
}

pub(super) async fn run(
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    preferences: blent_config::host_activity::Settings,
    lifetime: SourceLifetime,
    shared: Arc<Shared>,
) -> Result<()> {
    Recovery {
        factory,
        settings,
        preferences,
        shared,
    }
    .run(lifetime)
    .await
}
impl Recovery {
    async fn run(self, lifetime: SourceLifetime) -> Result<()> {
        let attempts = Pool::new(1)?;
        let mut epoch = 0;
        loop {
            let attempt = SourceLifetime::new(attempts.reserve()?);
            let proof = attempt.completion();
            lifetime.retain(proof.clone());
            let Some(next) = self.attempt(attempt, epoch).await else {
                return Ok(());
            };
            epoch = next;
            if !self.retry(&proof).await {
                return Ok(());
            }
        }
    }
    async fn attempt(&self, lifetime: SourceLifetime, previous: u64) -> Option<u64> {
        let creation = std::panic::AssertUnwindSafe(self.create(lifetime))
            .catch_unwind()
            .await;
        let mut binding =
            match creation.unwrap_or_else(|_| Err(anyhow::anyhow!("activity creator panicked"))) {
                Ok(binding) => binding,
                Err(error) => {
                    tracing::warn!(%error, "activity source creation failed");
                    self.shared.publish(Publication::Failed);
                    return (previous != 0).then_some(previous);
                }
            };
        let outcome = std::panic::AssertUnwindSafe(self.follow(&mut binding, previous))
            .catch_unwind()
            .await;
        let retry = self.outcome(outcome);
        binding.source.stop();
        while !binding.source.retired() {
            tokio::time::sleep(self.settings.interval).await;
        }
        retry.then_some(binding.epoch)
    }
    async fn create(&self, lifetime: SourceLifetime) -> Result<StartedSource> {
        tokio::select! { biased;
            _ = self.shared.stopped() => anyhow::bail!("activity creation stopped"),
            value = tokio::time::timeout(self.settings.startup_deadline,
                self.factory.create(self.settings.source, lifetime)) => Ok(value??),
        }
    }
    async fn follow(&self, binding: &mut StartedSource, previous: u64) -> Result<bool> {
        ensure!(
            binding.epoch > previous,
            "activity replacement epoch is not fresh"
        );
        observe(binding, self.settings, self.preferences, &self.shared).await
    }
    fn outcome(&self, result: std::thread::Result<Result<bool>>) -> bool {
        match result {
            Ok(Ok(retry)) => retry,
            _ => {
                tracing::warn!(?result, "activity source following failed");
                self.shared.publish(Publication::Failed);
                false
            }
        }
    }
    async fn retry(&self, proof: &Completion) -> bool {
        tokio::select! { biased;
            _ = self.shared.stopped() => false,
            _ = self.settle_and_backoff(proof) => true,
        }
    }
    async fn settle_and_backoff(&self, proof: &Completion) {
        while !proof.is_complete() {
            tokio::time::sleep(self.settings.interval).await;
        }
        // The configured source deadline also bounds retry frequency. It never
        // retires an unconfirmed creator and does not replace its native fence.
        tokio::time::sleep(self.settings.source.deadline).await;
    }
}
