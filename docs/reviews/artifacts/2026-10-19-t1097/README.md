# T1097: reuse immutable deadline snapshots

Parent revision `3f76e44c`. `ConnectionDeadline.at(null)` returns `IDLE`.
Composition returns the first matching immutable input, then the second, and
allocates only when neither input represents the earliest deadline and combined
knowledge. Unknown providers still preserve other known budgets. Partial
knowledge is not promoted, and `Long.MAX_VALUE` remains a valid absolute budget.

Three permanent normal-suite tests fail before optimization (`red.log.gz`): idle
reuse, a 144-pair independent value/reuse oracle, and new partial knowledge with
unchanged original MAX/unknown inputs. They pass afterward, together with the
existing 1,728-triple oracle, negative/boundary tests and native-owner/actor wait
regressions. Fresh clean coverage run: **53 tests, zero failures/errors/skips**.
All five authored production initializer/method entries in the complete deadline
file have **100% executable-line coverage**, measured individually using the
strict repository mapper. Original XML/exec, class/source hashes, inventory,
per-function results and test XML retain the scope. Twelve architecture checks
pass. Kotlin PSI 1.9.20 source estimates cover 21 methods/callbacks, maximum 9;
native SonarQube evidence is still missing under T1019.

## Matched allocation measurement

Run the unchanged T918 tool with `--allocations` before and after. Same installed
JDK `21.0.12.1+1-1-24.04.4-Ubuntu`, offline tools and probe source. Each case warms
up one million iterations, then measures two million using original-thread
native allocation counters. Raw source manifests identify the changed deadline
implementation; the benchmark entry points and measurement settings match.

| Probe | Before bytes | After bytes |
| --- | --- | --- |
| Direct idle projection, nonescaping consumption | 4,529,448 | 7,488 |
| Direct idle projection, explicit escape | 48,000,168 | 168 |
| Idle composition, nonescaping consumption | 96,000,168 | 168 |
| Idle composition, explicit escape | 96,000,168 | 168 |

The 168-byte sample overhead remains; measurements do not assert zero total
allocation or unrestricted JIT behavior. Direct nonescaping results show warmup
effects. This finite hosted pattern establishes neither ART allocation counts
nor physical Android battery savings. Actor timing/defaults are unchanged.

## Lifetime and bounds

Snapshots are immutable values, admitted only with null/nonnegative absolute
times and a knowledge flag. Constants live with their class; returned snapshots
remain owned by their callers' managed references. Reuse retains no native
owner, borrowed buffer, callback, queue or task and introduces no mutable pool,
release or identity-reuse state. When neither input has the required complete
value, one new managed snapshot is necessary; previous inputs remain unchanged.
The operation handles exactly two snapshots per call and does not infer native
retirement, admission or stop/fence evidence. Production callers use snapshot
values rather than snapshot reference identities. Resource ownership and actual
native joins remain with the existing owners.
