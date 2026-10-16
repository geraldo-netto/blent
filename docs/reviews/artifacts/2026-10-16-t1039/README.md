# T1039: original-connection liveness negotiation and reception

Accepted session/media policy requires explicit acknowledgement before new packet
types. This prerequisite supplies a strict optional `!BLENT1\n` acknowledgement on
the original video TCP socket after authentication/legacy token prelude. The first
acknowledgement byte starts a three-second completion deadline. Only the exact
bounded record publishes its connection-owned grant. Read-only views reject a
closed publisher; replacements start with a separate unacknowledged channel.
Extra input/EOF ends that viewer. Authentication failures admit no subscription.
Legacy silence adds no periodic input polling or grants.

The Android protected connected greeting accepts only numeric protocol 1;
absence/unsupported/malformed offers retain legacy behavior. That configuration
travels separately from decoder format in immutable display-worker requests.
Old display work retires before replacement. The video adapter writes credentials
before feature ACK and checks original generation/socket ownership around writes.
Native blocking ACK writes hold no receiver monitor, allowing stop to close only
that original socket. Stale socket/UI callbacks cannot retarget replacement work.

The strict type-2 receiver contract accepts only an enabled connection, body length
2 and version byte 1. It produces no decoder input, first-frame connection callback
or render ACK. Framing/storage reuse and current media authority remain intact.
Host `video_liveness` advertisement remains absent until T1038 emission integration;
this commit claims negotiation/receiver readiness, not active host heartbeat output.

Permanent red/green cases cover exact/fragmented/truncated/malformed/unsupported
records, bounded byte mutations, legacy traffic, authentication, publisher closure,
replacement, private worker schema, stale control/UI callbacks, canceled native
ACK writes and decoder/frame isolation. The unnegotiated-reception assertion fails
before the permission guard. Two host grant assertions fail before publication.
A further injected optional-observer fault reproduces interrupted control cleanup;
protected notification now passes the same revocation/socket-close assertions.

Fresh locked offline suites pass 776 host-library and 342 shared-contract tests;
three existing benchmark ignores remain. All 1,996 Android tests pass, with no skips.
Seven architecture/portability checks pass. Native Rust coverage covers all nine
scoped production functions individually at >=94.74%. JaCoCo method/line counters
cover 313 production functions/methods in changed Kotlin files at >=80%, retaining
supported inline-body attribution and separate callback requirements. Source/class
hashes, raw XML/exec, native Rust object attestations and raw LCOV are retained.

Source audits cover 16 scoped Rust production/test functions (max 8) and 614 Kotlin
production/test functions (max 9), using unchanged repository Sonar-rule counting,
cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2 and pinned Kotlin PSI 1.9.20.
Native SonarQube remains unavailable under T1019; source scores are not certification.
Native Android physical/device qualification remains separate from Robolectric/JVM
coverage. No download, installation, upload, push or live device/display change
occurred. T1038 emission, T1041 capture capability projection, T1037 selection and
T1015 activity integration remain open; early invalid-payload efficiency is T1040.
