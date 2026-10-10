# T867 durable host identity

Independent storage contract separates load/create and strict bounded record parsing
from stock TLS validation and Linux filesystem operations. The Linux adapter pins
an existing private directory, holds a stable exclusive lock and uses fixed names
with no-follow/nonblocking opens, owner/mode/type/link checks and bounded reads.
A synced initialization marker precedes generation. Complete private certificate/key
state is file-synced, atomically renamed without replacement, then directory-synced.
Malformed records and interrupted initialization are preserved and rejected; neither
storage failure nor an invalid key causes silent generation of a replacement.

Fourteen T867 tests and eight existing stock-TLS tests pass offline. Evidence covers
restart reuse, concurrent ownership, malformed/oversized records, 4096 bounded header
mutations, symlinks/hardlinks/FIFOs, unsafe permissions and descriptor-pinned rename.
Test-only thread-local failures and subprocess exit cover eight persistence boundaries;
an actual write to a read-only descriptor additionally exercises native write failure.
These are process/fault tests, not a power-cut/filesystem hardware qualification.

All 12 production functions pass the normal per-function LLVM gate: reserve 94.12%,
all others 100%. All 35 production/test functions have cyclomatic complexity at most 8.
New capability, not a behavioral regression fix; tests do not claim a pre-fix failure.

T868 still supplies peer trust and T852 composes persisted identity with authenticated
pairing. Production broker selection/retirement and directory creation remain T853;
no live private identity was created. No runtime install, restart or download occurred.
This adapter is Linux-only; no Windows native persistence support is claimed.
