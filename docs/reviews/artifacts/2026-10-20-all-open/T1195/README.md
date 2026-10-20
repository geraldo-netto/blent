# T1195: terminal recalibration requests

The actual GUI regression first fails: after the final valid request the next
click wraps4294967295 to0 and can select an old environment cache identity.
Shared FileConfig request policy now admits only checked u32 increments. At MAX
it rejects without mutation; the GUI disables further requests and shows the
terminal state. Existing profiles and explicit manual encoder preference remain
usable. Current configuration serialization remains unchanged.

Permanent normal-suite tests cover the actual repeated GUI click/persistence
path at MAX−1/MAX, the wider-u64 oracle over66 seeds ×40 attempts (2,640
transitions), six invalid schema values, and real durable cache records. The
latter proves initial records cannot be reused after terminal reset, repeated
denial retains the terminal fingerprint and valid terminal profiles remain
reusable. Original T480/T714 normal request/save and T537 user-byte-isolation
assertions remain. Their old wrap expectation follows the corrected contract.
Targeted current workspace tests pass normally and with native LLVM counters;
source hashes, commands and original logs retained. The later added real-cache
boundary case is included in the final instrumented normal-suite execution.

Both changed production functions have100% individual executable-line coverage.
Authored-source cyclomatic estimates: request policy2, UI3; new/changed tests and
helpers <=4. These are local estimates, not native SonarQube; T1019 retains that
gap. Formatting and whitespace checks pass. No broader all-features claim.

FileConfig remains sole owner of the persisted counter. Requests mutate it
synchronously through an exclusive borrow; the UI borrows settings only during
the frame. Terminal denial allocates or retains nothing and requires no native
release proof. The counter is cache identity, not evidence of encoder/native
quiescence. Existing cache leases, locked settings saves and original native
cleanup sponsors remain responsible for their resources. Test temporary files,
owned strings and completed GUI tasks retire in their original scopes; no new
retaining cycle, alias, buffer, callback or native reuse path is introduced.
No service/device interruption, Windows or EVDI changes.
