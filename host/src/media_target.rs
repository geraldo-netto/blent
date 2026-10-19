//! Linux original-target lookup. No invitation, USB grant or media ownership.
use anyhow::{Context, Result};
use blent_config::connection::media::original::{BoundTarget, OriginalTarget};

/// Resolve automatic ADB once for the owning intent. The command adapter admits
/// bounded output/children and retains exact original cleanup on cancellation.
/// Explicit serials and USB identities never invoke ADB discovery.
pub async fn bind<'a>(
    original: &'a OriginalTarget,
    observer: &(dyn Fn(blent_config::retirement::Completion) + Send + Sync),
) -> Result<&'a BoundTarget> {
    original
        .bind(|| async {
            let adb = blent_config::linux::programs::find_in(
                "adb",
                &std::env::var_os("PATH").unwrap_or_default(),
            )
            .context("Media requires adb")?;
            crate::adb_command::output_observed(&adb, &["get-serialno"], observer)
                .await
                .context("select one authorized tablet with --serial")
        })
        .await
}

#[cfg(test)]
mod tests;
