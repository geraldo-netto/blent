use super::*;
use blent::capture_contract::ProfileSource;

enum Prepared {
    Cached(Box<Candidate>),
    Measured(Vec<Candidate>),
}

pub(super) async fn optimize(
    base: &CaptureConfig,
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    latency: &LatencyTracker,
    attachment: &dyn ProfileSource,
    scope: &blent::native_scope::Scope,
) {
    let cache = cache::Cache::open(base, snapshot, attachment, scope).await;
    optimize_with(&HostProbes(base, scope), settings, snapshot, latency, cache).await;
}

trait Probes {
    fn scope(&self) -> blent::native_scope::Scope;
    async fn candidates(
        &self,
        snapshot: &EncoderSettings,
        scope: &blent::native_scope::Scope,
    ) -> Vec<Candidate>;
}
struct HostProbes<'a>(&'a CaptureConfig, &'a blent::native_scope::Scope);
impl Probes for HostProbes<'_> {
    fn scope(&self) -> blent::native_scope::Scope {
        self.1.clone()
    }
    async fn candidates(
        &self,
        snapshot: &EncoderSettings,
        scope: &blent::native_scope::Scope,
    ) -> Vec<Candidate> {
        calibrate(self.0, snapshot, scope).await
    }
}

async fn optimize_with(
    probes: &impl Probes,
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    latency: &LatencyTracker,
    cache: Option<Arc<cache::Cache>>,
) {
    let mut reuse = cache.as_deref();
    loop {
        let Some(prepared) = prepare(probes, settings, snapshot, latency, reuse).await else {
            publish(
                settings,
                &snapshot.selection_key(),
                fallback_encoder(snapshot),
                "Calibration interrupted; restored fallback",
            );
            return;
        };
        match prepared {
            Prepared::Measured(candidates) => {
                supervise(settings, snapshot, latency, candidates, cache.clone()).await;
                return;
            }
            Prepared::Cached(candidate) => {
                watch_cached(
                    reuse.unwrap(),
                    latency,
                    &snapshot.selection_key(),
                    &candidate,
                )
                .await;
                publish(
                    settings,
                    &snapshot.selection_key(),
                    fallback_encoder(snapshot),
                    "Historical profile failed; measuring current candidates",
                );
                reuse = None;
            }
        }
    }
}

async fn prepare(
    probes: &impl Probes,
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    latency: &LatencyTracker,
    cache: Option<&cache::Cache>,
) -> Option<Prepared> {
    if !snapshot.decoders.as_ref()?.valid() {
        return None;
    }
    // T714: verify the saved winner before idle admission or expensive probes.
    if let Some(candidate) = cached(settings, snapshot, latency, cache).await {
        return Some(Prepared::Cached(Box::new(candidate)));
    }
    let mut input = latency.interaction_updates();
    if !super::super::trial::quiet(&mut input).await {
        return None;
    }
    super::super::trial::uninterrupted(&mut input, async {
        let admission = admission::begin(&probes.scope()).await?;
        let candidates = probes.candidates(snapshot, admission.scope()).await;
        let candidates = measured::benchmark(settings, snapshot, latency, candidates).await;
        Some(Prepared::Measured(candidates))
    })
    .await
    .flatten()
}

async fn cached(
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    latency: &LatencyTracker,
    cache: Option<&cache::Cache>,
) -> Option<Candidate> {
    let cache = cache?;
    let candidate = cache.load(cache::now(), snapshot)?;
    let key = snapshot.selection_key();
    let choice = candidate.decoder.clone();
    let workers = candidate.measurement.workers_requested;
    let mut retirement = Box::pin(cache.retired());
    let result = tokio::select! {
        _ = &mut retirement => return None,
        result = choose(settings, &key, fallback_encoder(snapshot), vec![candidate], |name| {
            let key = &key; let choice = choice.as_ref();
            async move { rendered(latency, key, &name, choice, workers).await }
        }) => result,
    };
    if let Some((candidate, _)) = result {
        return Some(candidate);
    }
    cache.invalidate();
    None
}

async fn watch_cached(
    cache: &cache::Cache,
    latency: &LatencyTracker,
    key: &Key,
    candidate: &Candidate,
) {
    tokio::select! {
        _ = cache.retired() => {},
        _ = super::super::health::failed(latency, key, &candidate.measurement.encoder, candidate.decoder.as_ref()) => cache.invalidate(),
    }
}

#[cfg(test)]
mod tests;
