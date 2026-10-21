# T1251 offline pairing acceptance prerequisite

The existing normal suites pass236 host broker cases and129 Android cases in27
suites, with zero failures, errors or skips. All execution uses cached tools in
private user/network namespaces with loopback only, Cargo offline/locked and
Gradle/Robolectric offline. No native device is acquired or excluded EVDI fixture
resumed. Logs, Android XML, raw JaCoCo XML and456 source hashes are retained.

The host cohort includes the existing permanent
`t855_presentation_commands_pair_real_tls_then_durably_revoke` regression in
`host/src/broker/linux/server/tests/tls.rs`: actual native presentation commands
travel through the Unix broker, approval precedes real mutual TLS, trust is not
saved before peer proof, authenticated trust becomes visible, and completed
durable revocation removes it. Independent native/store admission, stale tickets,
denials, original thread/process retirement, delayed proofs and invalid protocol
boundaries remain exercised by the broader broker cohort.

The Android cohort covers pairing controls/state/TLS, saved-computer UI with real
store persistence, native file/lock adapters, Binder/lifecycle ownership, stale
browser/confirmation callbacks and the existing pairing UI-order regression.
Existing tests are retained unchanged. No new production defect or confirmed
coverage gap requires a behavioral fix; no regression is replaced or weakened.
The JaCoCo XML is additional raw observation, not a new whole-project/per-method
coverage pass claim; prior individual native counters remain independent.

Each fixture owns private bounded state, endpoints, workers and temporary store
directories. Current native/store proofs, actual thread joins and OS-backed file
ownership checks govern their release; logical completion alone supplies no new
authority. These tests complement the T1252 physical evidence. Synthetic USB
inventory/media and hosted Android execution cannot establish physical tablet
trust persistence, approval, process death or unsupported provider behavior.
