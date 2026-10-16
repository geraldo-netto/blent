use super::{Key, Selected};
use crate::{
    capture::{
        probe::{self, Measurement},
        CaptureConfig,
    },
    latency::LatencyTracker,
    media::EncoderSettings,
};
use std::{cmp::Ordering, future::Future, sync::Arc, time::Duration};
use tokio::sync::watch;

// Calibrate one encoder at a time across all tablet sessions. Current streams
// continue normally; concurrent probes would distort comparative measurements.
static ADMISSION: std::sync::LazyLock<Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(1)));
mod activity;
mod admission;
use activity::TrialAuthority;

pub(crate) async fn run(
    config: CaptureConfig,
    settings: watch::Sender<EncoderSettings>,
    mut display: watch::Receiver<bool>,
    mut stop: watch::Receiver<bool>,
    latency: LatencyTracker,
    attachment: Arc<dyn blent::capture_contract::ProfileSource>,
    scope: blent::native_scope::Scope,
    activity: Arc<blent::display_work::View>,
) {
    let mut updates = settings.subscribe();
    let mut completed: Option<Completed<'_>> = None;
    loop {
        let activity_change = activity.changed();
        let profile_change = attachment.changed();
        let profile = attachment.profile();
        if stopped(&stop) || display.has_changed().is_err() {
            return;
        }
        let snapshot = updates.borrow_and_update().clone();
        let key = snapshot.selection_key();
        let work = pending_work(
            &snapshot,
            *display.borrow(),
            &key,
            completed.as_ref(),
            &activity,
        )
        .filter(|_| profile.active());
        if let Some(work) = work {
            let authority = TrialAuthority {
                activity: &activity,
                work,
                profile,
            };
            // Reserve observation before cache/environment preparation can create
            // native work, including preparation before calibration admission.
            let (trial, original) = scope.group();
            let finished = until_changed(
                &key,
                &mut updates,
                &mut display,
                &mut stop,
                &authority,
                optimize(
                    &config,
                    &settings,
                    &snapshot,
                    &latency,
                    attachment.as_ref(),
                    &trial,
                ),
            )
            .await
            .is_some();
            drop(trial);
            if !activity::retired(&original, &mut stop).await {
                return;
            }
            completed = finish_trial(&settings, &snapshot, key, authority, finished);
            continue;
        }
        if !wait_inputs(
            &mut updates,
            &mut display,
            &mut stop,
            activity_change,
            profile_change,
        )
        .await
        {
            return;
        }
    }
}

struct Completed<'a> {
    key: Key,
    authority: TrialAuthority<'a>,
}
fn pending_work(
    settings: &EncoderSettings,
    visible: bool,
    key: &Key,
    completed: Option<&Completed<'_>>,
    activity: &blent::display_work::View,
) -> Option<blent::display_work::Observation> {
    if !eligible(settings, visible)
        || completed.is_some_and(|completed| completed.key == *key && completed.authority.current())
    {
        return None;
    }
    activity.begin()
}

fn finish_trial<'a>(
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    key: Key,
    authority: TrialAuthority<'a>,
    finished: bool,
) -> Option<Completed<'a>> {
    if finished && authority.current() {
        return Some(Completed { key, authority });
    }
    if authority.profile.active() {
        publish(
            settings,
            &key,
            fallback_encoder(snapshot),
            "Cancelled trial; restored fallback",
        );
    }
    None
}

async fn wait_inputs(
    updates: &mut watch::Receiver<EncoderSettings>,
    display: &mut watch::Receiver<bool>,
    stop: &mut watch::Receiver<bool>,
    activity: blent::activity_view::Change,
    profile: blent::capture_contract::Signal<'static>,
) -> bool {
    tokio::select! {
        result = updates.changed() => result.is_ok(),
        result = display.changed() => result.is_ok(),
        _ = stop.changed() => false,
        _ = activity => true,
        _ = profile => true,
    }
}

fn stopped(stop: &watch::Receiver<bool>) -> bool {
    *stop.borrow() || stop.has_changed().is_err()
}
fn eligible(settings: &EncoderSettings, visible: bool) -> bool {
    visible
        && settings.encoder == "auto"
        && settings.geometry_ready
        && settings.decoder_supports(crate::media::Codec::H264)
}

async fn until_changed<T>(
    key: &Key,
    updates: &mut watch::Receiver<EncoderSettings>,
    display: &mut watch::Receiver<bool>,
    stop: &mut watch::Receiver<bool>,
    authority: &TrialAuthority<'_>,
    work: impl Future<Output = T>,
) -> Option<T> {
    let inputs = activity::Inputs::new(key, updates, display, stop);
    let guarded = activity::guarded(inputs, authority, work);
    tokio::pin!(guarded);
    loop {
        tokio::select! {
            biased;
            result = &mut guarded => return result,
            _ = updates.changed() => {},
            _ = display.changed() => {},
            _ = stop.changed() => {},
            _ = authority.activity.changed() => {},
            _ = authority.profile.retired() => {},
        }
    }
}

async fn supervise(
    settings: &watch::Sender<EncoderSettings>,
    snapshot: &EncoderSettings,
    latency: &LatencyTracker,
    candidates: Vec<Candidate>,
    cache: Option<Arc<cache::Cache>>,
) {
    let key = snapshot.selection_key();
    let mut pending = candidates;
    let mut fallback = fallback_encoder(snapshot).to_string();
    while let Some((candidate, remaining)) = choose(settings, &key, &fallback, pending, |name| {
        let key = &key;
        let decoder = settings.borrow().decoder_choice().cloned();
        let workers = settings.borrow().selected_workers();
        async move { rendered(latency, key, &name, decoder.as_ref(), workers).await }
    })
    .await
    {
        let name = candidate.measurement.encoder.clone();
        if let Some(cache) = cache
            .as_ref()
            .filter(|_| settings.borrow().selection_matches(&key))
        {
            cache.remember(&candidate).await;
        }
        let decoder = settings.borrow().decoder_choice().cloned();
        super::health::failed(latency, &key, &name, decoder.as_ref()).await;
        tracing::warn!(encoder = %name, "Verified encoder lost render progress; trying remaining compatible candidates");
        if let Some(cache) = &cache {
            cache.invalidate();
        }
        // Never return to a candidate that failed in this settings/peer epoch.
        pending = remaining;
        fallback = "libx264".into();
        publish(
            settings,
            &key,
            &fallback,
            "Render progress lost; using H.264 during recovery backoff",
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn choose<F: Future<Output = bool>>(
    settings: &watch::Sender<EncoderSettings>,
    key: &Key,
    fallback: &str,
    candidates: Vec<Candidate>,
    mut verify: impl FnMut(String) -> F,
) -> Option<(Candidate, Vec<Candidate>)> {
    let mut candidates = candidates.into_iter();
    while let Some(candidate) = candidates.next() {
        let reason = measured::reason(&candidate).unwrap_or_else(|| format!("Host probe: {:.1} FPS, packet-interval p95 {:.2} ms; {} decoder advertised; awaiting render ACKs",
            candidate.measurement.fps, candidate.measurement.p95_us as f64 / 1000.0,
            if candidate.hardware { "hardware" } else { "software/unknown" }));
        if !publish_choice(
            settings,
            key,
            &candidate.measurement.encoder,
            &reason,
            candidate.decoder.clone(),
            candidate.measurement.workers_requested,
        ) {
            return None;
        }
        if verify(candidate.measurement.encoder.clone()).await {
            let verified = reason.replace(
                "awaiting render ACKs",
                if candidate.cached {
                    "render ACKs reverified; historical profile, not ranked this session"
                } else if candidate.observation.is_some() {
                    "render ACKs reverified; best tested in this session"
                } else {
                    "render ACKs verified; decoder speed not benchmarked"
                },
            );
            settings.send_if_modified(|current| {
                if !current.selection_matches(key) {
                    return false;
                }
                current.selection = Some(Selected {
                    workers: candidate.measurement.workers_requested,
                    key: key.clone(),
                    encoder: candidate.measurement.encoder.clone(),
                    reason: verified.clone(),
                    verified: true,
                    calibrating: false,
                    decoder: candidate.decoder.clone(),
                });
                true
            });
            return Some((candidate, candidates.collect()));
        }
        tracing::warn!(encoder = %candidate.measurement.encoder, "Automatic codec trial produced no verified render ACKs; trying next compatible candidate");
    }
    publish(
        settings,
        key,
        fallback,
        "Preserved fallback: compatible encoder trials failed or did not render",
    );
    None
}

fn publish(
    settings: &watch::Sender<EncoderSettings>,
    key: &Key,
    encoder: &str,
    reason: &str,
) -> bool {
    let published = publish_choice(settings, key, encoder, reason, None, 0);
    if published {
        settings.send_modify(|current| {
            if let Some(selected) = current.selection.as_mut().filter(|s| &s.key == key) {
                selected.calibrating = false;
            }
        });
    }
    published
}

fn publish_choice(
    settings: &watch::Sender<EncoderSettings>,
    key: &Key,
    encoder: &str,
    reason: &str,
    decoder: Option<blent_config::negotiation::DecoderChoice>,
    workers: u32,
) -> bool {
    settings.send_if_modified(|current| {
        if !current.selection_matches(key) {
            return false;
        }
        current.selection = Some(Selected {
            workers,
            key: key.clone(),
            encoder: encoder.into(),
            reason: reason.into(),
            verified: false,
            calibrating: true,
            decoder: decoder.clone(),
        });
        tracing::info!(
            encoder,
            workers,
            ?decoder,
            reason,
            "Automatic encoder selection"
        );
        true
    })
}

async fn rendered(
    latency: &LatencyTracker,
    key: &Key,
    name: &str,
    decoder: Option<&blent_config::negotiation::DecoderChoice>,
    workers: u32,
) -> bool {
    let mut updates = latency.activity_updates();
    let previous = latency.encoder_evidence().map(|e| (e.epoch, e.rendered()));
    // T715: native capture can start after the control session (EVDI device scan).
    // Bound startup separately; only a matching encoder starts the ACK deadline.
    let startup_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut render_deadline = None;
    loop {
        let evidence = latency
            .encoder_evidence()
            .filter(|e| e.workers == workers && matches_profile(e, key, name, decoder));
        if evidence.is_some() {
            render_deadline
                .get_or_insert_with(|| tokio::time::Instant::now() + Duration::from_secs(6));
        }
        if matches_evidence(evidence, key, name, previous, decoder) {
            return true;
        }
        if !matches!(
            tokio::time::timeout_at(
                render_deadline.unwrap_or(startup_deadline),
                updates.changed(),
            )
            .await,
            Ok(Ok(()))
        ) {
            return false;
        }
    }
}
fn matches_evidence(
    evidence: Option<Arc<crate::latency::EncoderEvidence>>,
    key: &Key,
    name: &str,
    previous: Option<(u64, u64)>,
    decoder: Option<&blent_config::negotiation::DecoderChoice>,
) -> bool {
    evidence.is_some_and(|e| {
        let before = previous
            .filter(|&(epoch, _)| epoch == e.epoch)
            .map(|(_, count)| count)
            .unwrap_or(0);
        matches_profile(&e, key, name, decoder) && e.rendered().saturating_sub(before) >= 3
    })
}

fn matches_profile(
    evidence: &crate::latency::EncoderEvidence,
    key: &Key,
    name: &str,
    decoder: Option<&blent_config::negotiation::DecoderChoice>,
) -> bool {
    evidence.active()
        && evidence.name == name
        && evidence.format == key.format
        && evidence.decoder.as_ref() == decoder
}

#[derive(Clone, Debug)]
struct Candidate {
    cached: bool,
    measurement: Measurement,
    hardware: bool,
    decoder: Option<blent_config::negotiation::DecoderChoice>,
    observation: Option<super::trial::Observation>,
}

async fn calibrate(
    base: &CaptureConfig,
    snapshot: &EncoderSettings,
    scope: &blent::native_scope::Scope,
) -> Vec<Candidate> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut candidates = Vec::new();
    'probes: for encoder in allowed_encoders(base) {
        let codec = crate::media::Codec::from_encoder(encoder.name);
        if !snapshot.decoder_supports(codec) {
            continue;
        }
        for workers in probe_budgets(base, snapshot, encoder.name) {
            let mut config = probe_config(base, snapshot, encoder.name);
            config.selected_workers = workers;
            let Ok(result) =
                tokio::time::timeout_at(deadline, probe::measure(&config, scope)).await
            else {
                break 'probes;
            };
            match result {
                Ok(measurement) => {
                    if let Some(candidate) =
                        compatible_candidate(snapshot, measurement, base.ten_bit)
                    {
                        candidates.push(candidate);
                    }
                }
                Err(error) => {
                    tracing::info!(encoder = encoder.name, workers, %error, "Automatic encoder probe rejected candidate")
                }
            }
        }
    }
    candidates.sort_by(|a, b| rank(a, b, snapshot.fps));
    candidates
}

fn allowed_encoders(base: &CaptureConfig) -> Vec<blent_config::encoding::Encoder> {
    let mut encoders = blent_config::encoding::ENCODERS;
    encoders.sort_by_key(|encoder| probe_order(encoder.name));
    encoders
        .into_iter()
        .filter(|encoder| base.gpu_policy.allows(encoder.name))
        .collect()
}

fn probe_budgets(base: &CaptureConfig, settings: &EncoderSettings, name: &str) -> Vec<u32> {
    let requested = if settings.decoders.as_ref().is_some_and(|d| d.protocol == 2) {
        base.encoder_workers
    } else {
        base.encoder_workers.max(1)
    };
    blent_config::encoder_workers::candidates(name, requested)
}

fn probe_order(encoder: &str) -> u8 {
    match encoder {
        "libx264" => 0,
        "h264_vaapi_baseline" => 1,
        "h264_vaapi" => 2,
        _ => 3,
    }
}

fn compatible_candidate(
    settings: &EncoderSettings,
    measurement: Measurement,
    ten_bit: bool,
) -> Option<Candidate> {
    let caps = settings.decoders.as_ref()?;
    let codec = crate::media::Codec::from_encoder(&measurement.encoder);
    let decoder = if caps.protocol == 2 {
        if !supported_output(&measurement, codec, ten_bit) {
            return None;
        }
        Some(caps.choose(measurement.stream.as_ref()?, true)?)
    } else {
        None
    };
    let hardware = match &decoder {
        Some(choice) => caps.details.iter().any(|d| {
            d.name == choice.name && d.codec == codec.wire_name() && d.hardware == Some(true)
        }),
        None => caps.hardware.iter().any(|name| name == codec.wire_name()),
    };
    Some(Candidate {
        cached: false,
        measurement,
        hardware,
        decoder,
        observation: None,
    })
}

fn supported_output(measurement: &Measurement, codec: crate::media::Codec, ten_bit: bool) -> bool {
    let depth = if ten_bit && codec == crate::media::Codec::Hevc {
        10
    } else {
        8
    };
    measurement
        .stream
        .as_ref()
        .is_some_and(|stream| stream.codec == codec.wire_name() && stream.format.depth == depth)
}

fn probe_config(base: &CaptureConfig, settings: &EncoderSettings, encoder: &str) -> CaptureConfig {
    let (width, height) = settings.video_dimensions();
    CaptureConfig {
        encoder: encoder.into(),
        width,
        height,
        stream_scale: 1,
        fps: settings.fps,
        bitrate: settings.bitrate,
        quality: settings.quality,
        ..base.clone()
    }
}

fn rank(a: &Candidate, b: &Candidate, fps: u32) -> Ordering {
    let score = |candidate: &Candidate| {
        blent_config::encoding::rank(
            candidate.measurement.fps,
            candidate.hardware,
            candidate.measurement.p95_us,
            candidate.measurement.first_us,
            fps,
        )
    };
    score(a)
        .cmp(&score(b))
        .then_with(|| a.measurement.encoder.cmp(&b.measurement.encoder))
}

mod cache;
mod measured;
mod preparation;
use preparation::optimize;
#[cfg(test)]
mod tests;

fn fallback_encoder(settings: &EncoderSettings) -> &str {
    settings
        .selection
        .as_ref()
        .filter(|s| s.verified && settings.selection_matches(&s.key))
        .map(|s| s.encoder.as_str())
        .unwrap_or("libx264")
}

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
fn spawn(
    config: CaptureConfig,
    settings: watch::Sender<EncoderSettings>,
    display: watch::Receiver<bool>,
    stop: watch::Receiver<bool>,
    latency: LatencyTracker,
    attachment: crate::attachment::Attachment,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run(
        config,
        settings,
        display,
        stop,
        latency,
        Arc::new(attachment),
        Default::default(),
        Arc::new(blent::display_work::View::new(None, None, true)),
    ))
}
