# T1189: current probe decoder and original retirement ownership

The camera probe now uses current production decoder/controller retirement APIs. Offline compile and late-creator tests passed; this does not qualify physical camera use or unscoped remote cleanup.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The permanent normal-suite test first failed the actual offline example check:
its root lacked camera_control and its caller omitted the current Report argument.
The same test now passes. The current production decoder is included directly;
there is no old signature, compatibility decoder or dummy resource report.

One linear creator transfers to one probe owner. The owner uses the current
Controller with one worker slot and one consuming session factory. The report
reserves original producer/decoder resources through the production API. On
owner drop, shutdown publishes the original controller Completion. The receiver
waits until its unique creator closes, then until that original completion proves
actual controller-thread retirement. The production controller also retains its
runtime until all originally reserved native obligations settle. Funded late
creation after cancellation remains covered; sealing is not proof of native death.
Bridge cleanup is ordered after original retirement, including errors.

Permanent tests cover a held unique creator, invalid profile rejection before
factory effects, and cancellation while a real blocking creator is held. That
creator subsequently starts an actual /bin/cat child after cutoff. Retirement
must remain pending until both gates release and the original child exits. Gate
sponsors release on failed assertions; command tasks retain native reapers. Test
channels have capacity one, one controller/worker is admitted, and the bounded
sample store remains capped by its independent contract. Cleanup assumes actual
child exit and scheduler/reaper progress; an unknown obligation is never released
just because a timer, future or logical session ended.

Verification: cargo test --offline --locked -p blent --lib camera (124 cases),
and --test camera_probe --test camera_probe_options --test camera_probe_policy
--test camera_probe_samples (16 cases). The latter includes cargo check --offline
--locked --example camera-feedback-probe. Formatting and whitespace checks pass.
Raw red/green logs and exact source hashes are retained. Rustc 1.98.1; benchmark
code retains the established per-function coverage exemption. No production
library function changed. Conservative source decision estimates for changed
functions are <=9: run<=3, sample<=5, decode_session<=3, connections<=2,
observe<=3, print_samples=1, channel=1, Creation.start<=4, Owner.drop<=2,
Retirement.wait<=4; test functions/helpers<=7. These are manual estimates, not
SonarQube measurements. Native Sonar and installed Rust parser gaps remain T1019.

No physical ADB/camera, Windows/EVDI or service actions occurred. This restores
software probe integration; it is not physical acceptance. Existing unscoped
remote stop is separately open as T1204; conditional ADB mapping removal remains
blocked as T1090. No new signal-triggered remote cleanup path is introduced.

</details>
