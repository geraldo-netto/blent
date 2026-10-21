# T1243 joined truncation and isolated native media acceptance

The normal host suite now includes
`audio::session_tests::t1243_truncated_microphone_frames_retire_only_the_original_native_helper`.
It tests28 cases: fourteen selected header/payload/last-byte truncation offsets
for each negotiated clock mode. Each case first authenticates and delivers a
complete known PCM packet through the real local transport and actual helper
process. The next packet is truncated and its original socket write side closes.
The session must fail, reap that original helper and obtain its native completion
proof, while the independent remote registration still remains unretired.
Only explicitly supplied original client/source completion settles that route.

The focused new test passes all28 cases. The combined host camera/audio/media
registration and native IPC cohort passes171 tests, including the new test,
real H.264 encode/decode fixtures, native output-boundary faults, stale replies,
lost IPC, blocked cleanup and malformed/partial frame handling. There are no
failures or ignored tests in this selected cohort.

Two normal native audio tests pass against a private PipeWire daemon: real mono
microphone PCM, stereo speaker PCM, underflow silence, graph loss and exact
endpoint retirement. Synthetic inputs produce all media; no physical camera,
microphone, speaker or acoustic validation occurs.

The33 retained accessory/reconnect/probe/discovery/selection tests pass with
AddressSanitizer/UndefinedBehaviorSanitizer. Their fake libusb boundaries exercise
partial I/O, timeouts, stale selection, zero-length transfers, reconnect identities,
invalid ranges and helper/process retirement. Initial user/network namespaces
lacked `/run/user/0`, causing two ownership-lock setup failures. The successful
run adds a private mount namespace and1MiB tmpfs over `/run/user`, with its own
0700 `/run/user/0`. It changes no host runtime directory or real USB ownership.
XDG_RUNTIME_DIR alone cannot replace the helper's native `/run/user/<uid>` contract.

No additional production defect was confirmed in this bounded audit. Production
code and existing tests remain unchanged. The new Rust source functions have
cyclomatic estimates4 and1 using the project counter, tree-sitter0.25.2 and the
cached tree-sitter-languages1.10.2 Rust ABI14 grammar, which parses this file.
`rustfmt --edition 2021 --check host/src/audio/session_tests.rs` passes. Owner clarification2026-10-21 accepts local cyclomatic tools; T1244 verifies
these archived results against unchanged source and closes that tool prerequisite.
The separate T1221 grammar limitation elsewhere remains unresolved. No native Rust coverage was
recollected or whole-project coverage claimed; production functions are unchanged.

## Ownership and limits

Each sequential truncation case owns one temporary fixture directory, listener,
peer socket, helper process and registration bridge. Payloads are bounded by the
negotiated packet size; only14 selected prefixes per mode are examined. Original
transport failure cuts off delivery; the production process owner supplies native
retirement. Native exit never substitutes for remote/client/source proof. The
fixture supplies that separate proof before closing its bridge; scoped temporary
resources settle at case end. Existing deliberate faulty controls and uncertain
native-retirement cases remain in the passing host cohort.

The private PipeWire fixture owns its runtime directory, graph process, helper
children and synthetic feeders. Its normal assertions verify PCM and endpoint
absence, and its cleanup settles original children. ASan/UBSan cover the exercised
C fixture domains; they are not proofs of unrestricted leak freedom, kernel USB
behavior or native Android ownership. Eventual cleanup depends on actual original
native return/death, explicit original remote proof and fair task scheduling.

All commands use cached tools. Cargo runs `--offline --locked` through
`scripts/ci/test-host-offline.py`, selecting the existing Freedesktop SDK25.08,
matching codecs-extra and libclang18; exact paths/commands are preserved in logs.
User/network namespaces expose loopback only. The sanitizer run additionally owns
the private runtime mount. Local raw logs remain under
`/backups/disk2/blent-work/2026-10-21-offline-integration/T1243/`.

No device, EVDI attachment, installed service, package transfer, repository push
or physical acceptance was performed. T835/T1211 exclusions and unavailable
platform/device validation remain explicit. T1238's physical Stop discrepancy
remains unresolved and live checks stay paused.
