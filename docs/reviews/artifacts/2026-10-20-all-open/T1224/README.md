# T1224: canonical local candidate metadata

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

The existing read-only generator check exposed noncanonical candidate prose/links
at the recorded prior page date. Prepare current local source version1.2.3 with
the authoritative generator and actual2026-10-20 metadata preparation date. The
same --check1.2.3/2026-10-20 fails before generation and passes afterward. All17
normal release/metadata validation regressions pass after the documentation edit.

Candidate wording remains explicit: datePublished and downloadUrl are null, the
citation release date remains unpublished, and the status tells readers to check
actual publication/available files or build from source. No publication or native
release qualification is implied. Only local metadata/prose/date fields change;
no generator or runtime function changes, no artificial behavioral test or new
resource owner. Existing input validation and atomic validation-before-output
workflow stay intact. No remote request, upload, package installation or device
operation.
