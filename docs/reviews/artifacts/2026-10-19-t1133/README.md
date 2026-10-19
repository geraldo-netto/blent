# T1133 fixed USB inventory records

Use fixed-size array chunks after the existing exact extent validation. Decode
still rejects invalid headers/counts/reserved bytes/records and duplicate physical
targets before returning any inventory. The 16-byte header, 16-byte records and
512-device maximum remain unchanged. Borrowed records stay within the input
slice and this decode call; output owns its candidates. No native owner, handoff
or cleanup path changes, and no physical USB inventory is requested.

All seven existing normal fixtures pass, including the bounded mutation/maximum
corpus and original helper timeout/cancellation/retirement. Fresh offline native
coverage passes all nine production functions individually >=80%, no exclusions.
Raw source/compiled-object-attested evidence:
`/backups/disk2/blent-work/2026-10-19-open-items/t1133/native`.
Pinned source audit is <=9; actual SonarQube remains unavailable under T1019.
Cargo formatting passes.

Package-scoped host Clippy (`--no-deps --features accessory-transport`) drops
from six diagnostics to five; the fixed-record diagnostic is gone. Remaining
host findings are T1132/T1140/T1134/T1141. Without package scoping, dependency
checking still stops at the explicitly excluded EVDI item T1131. No lint allow
attribute, dependency update, Windows/EVDI edit, install or network transfer.
