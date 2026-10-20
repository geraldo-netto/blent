# T1193: remove obsolete encoder alias

Blent accepts current registry profiles directly. The GStreamer name
`vaapih264enc` no longer resolves to an FFmpeg profile. The compatibility-only
`canonical_name` function is removed; current `h264_vaapi` and
`h264_vaapi_baseline` retain their distinct selectable profiles and stock FFmpeg
mapping. Generic unknown saved encoders still follow current validation policy:
warn and select `auto`. No alias-specific migration remains.

Two permanent normal-library T1193 regressions fail before removal: the obsolete
name admitted a Profile and survived saved-configuration validation. Both pass
afterward. All388 shared-library tests pass, including390 bounded obsolete-name
variants (three padding characters,65 lengths, two placements; maximum203 UTF-8
bytes), current profiles, capability matrix and invalid inputs. Host/GUI targeted
caller checks pass5 tests. Feature-gated native in-process tests require the
separately tracked T835 environment; no all-features pass is claimed here.

Native LLVM counters report100% executable-line coverage individually for
`find` and `ffmpeg_name`; raw counters, source hashes, commands and logs retained.
The initial workspace export included stale objects from the reused target;
source-matched export uses only the current blent-config test executable.
Authored-source Rust decision estimates: `ffmpeg_name` <=3, `find` <=2, changed
new tests <=4. These are local estimates, not SonarQube certification; the native
measurement gap remains T1019. `cargo fmt --all -- --check` passes.

Both production functions are stateless bounded string views. `find` borrows its
input during comparison against11 static registry entries and returns only a
static entry. `ffmpeg_name` returns either its original borrowed input or a
static stock name. Neither allocates, retains callbacks, owns native resources,
or adds release/reuse transitions. Profile rejection precedes native effects;
current generic configuration ownership is unchanged. Test strings/configuration
are locally owned and released at scope exit. No Windows/EVDI backend or physical
device/service was changed. T1195 separately tracks an unrelated recalibration
counter wrap discovered while reviewing the next configuration cleanup.
