# T1227: accept native asynchronous Binder transport flags

Binder validation accepts the asynchronous flag together with other native transport flags. The repaired production worker completed USB/Binder/TLS pairing on the selected tablet with debugging enabled; debugging-off media acceptance remained separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Actual API36 production pairing killed the private connection worker before
Hello. A diagnostic release captured code1, flags17 and IllegalArgumentException.
The local Linux Binder UAPI defines TF_ONE_WAY=1 and TF_ACCEPT_FDS=16;
Android supplied both. Exact equality with FLAG_ONEWAY incorrectly rejected it.
The receiver now requires the asynchronous bit. Other bits grant no authority;
same-UID, interface, generation, parcel extent and native ownership checks remain.

Permanent normal-suite API27/34 regressions were added before each change.
The bounded rejection logger failed both API executions before implementation;
native flag validation, bounded flag-property and original-worker service tests
failed all six executions before the bit test. Raw red logs are retained.
The final offline cohort passes301 tests without failures, errors or skips.
JaCoCo0.8.15 counters independently measure validate at5/5 and onTransact at7/7.
Raw XML/exec, test reports, exact source and compiled class hashes are retained.
Kotlin PSI1.9.20 estimates209 production/test functions, maximum8; this is not
native SonarQube evidence. SonarQube is unavailable; no download was performed.

The current signed release was installed with preserved app data on the physical
tablet. The production source-built broker, libusb helper and Android worker
exchanged Hello and displayed matching Computer/Tablet fingerprints. After both
approvals, native-authenticated.json records the original link13764 at epoch1,
state3 (Authenticated). This is actual production USB/Binder/TLS evidence with
debugging enabled, not a fixture. Debugging-off camera/audio acceptance and full
pairing denial/revocation qualification remain T858/T860/T859/T855 respectively.
An earlier approval attempt expired while the tablet locked; it is not counted
as successful. The fresh attempt used a new link and matching current labels.

Validation borrows its caller-owned bounded Parcel only for the transaction.
The new logger retains no payload or Throwable: only numeric code/flags and an
exception class name bounded to80 characters. Rejected calls still terminate;
existing workers retain their exact generation, endpoint and retirement duties.
No allocation owner, release/reuse boundary or cleanup sponsor changes. Existing
fault, stale-generation, Stop, death and actual original-worker retirement tests
remain. Native flags establish the bug and fix; hosted counters do not establish
unrestricted native lifetime safety or physical media delivery.

</details>
