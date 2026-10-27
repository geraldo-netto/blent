# Pinned crate workspace notices (T1280)

These verbatim texts supply workspace notices omitted by the exact published
crate versions in `manifest.json`. They are consumed offline by
`crate_supplements.py`; the build does not download anything here or modify
Cargo's registry cache. Do not update a version, revision or hash independently
of its source evidence.

The AccessKit, egui, profiling, cookie-factory and gl-rs texts were recovered
under T1278 from the immutable URLs recorded per file. Their bytes and SHA-256
values are unchanged. AccessKit and egui workspace texts apply only to listed
crates at the exact same upstream commit. epaint_default_fonts also retains its
own nested font notices; workspace texts supplement those notices.

The atspi texts were read from the checksum-verified atspi-common 0.9.0 registry
archive at the same upstream revision as atspi-proxies 0.9.0. Each donor record
identifies its registry, version, archive checksum and member path. No new
transfers were used for T1280.

Pins include Cargo metadata identity, normalized manifest SHA-256, the published
VCS record, upstream revision and original registry archive SHA-256. The latter
is provenance for the published input; the generated augmented source archive
has its own checksum. Permanent tests compare registry checksums against
`Cargo.lock` and verify every text checksum.

The collector checks metadata identity, manifest bytes and VCS identity before
and after making a temporary source copy. It adds verified texts under
`.blent-upstream-notices/`, preserving that namespace in notice outputs and source
archives. The crate output manifest records each supplement's upstream provenance.
Missing inputs, changed pins and a pre-existing reserved namespace fail the build.

The local Cargo source tree and repository catalog are trusted build inputs;
these checks are not a sandbox against hostile concurrent filesystem mutation.
The catalog is an explicit reviewed inventory, not a general license inference
engine or a claim of legal completeness. New crates/versions need review. The implemented AppImage graph selection is documented in
[build inputs and sources](../../../docs/appimage-plan.md#build-inputs-licenses-and-sources);
it does not change these pinned notice bytes.
