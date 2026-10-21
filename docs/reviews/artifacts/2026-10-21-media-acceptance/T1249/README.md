# T1249 cached-SDK physical audio launch environment

The initial physical acceptance invocation omitted `SPA_PLUGIN_DIR` and
`PIPEWIRE_MODULE_DIR`. The existing offline SDK runner sets both explicitly in
`scripts/ci/test-host-offline.py`; the source-built audio executable already has
the matching SDK interpreter and RPATH. Direct execution without those plugin
paths selected incompatible host modules. Both host media sessions failed
before native `READY` with `early eof`; a separate direct diagnostic exited139.
This is an acceptance invocation error, not a demonstrated production defect.

The corrected invocation retains the private broker's `XDG_RUNTIME_DIR` and
sets `PIPEWIRE_RUNTIME_DIR=/run/user/1000` for the existing user audio server.
It selects `spa-0.2` and `pipewire-0.3` beneath the cached SDK's
`files/lib/x86_64-linux-gnu`, matching the existing runner. No global loader
override, native library replacement, package installation or network transfer
is used. System `pw-play`, `pw-record`, `pw-dump` and FFmpeg run in their ordinary
host environment, outside the SDK plugin environment.

The same direct native diagnostic then exits0 and produces exactly `READY\n`.
Fresh Android consent admits both production audio routes on connection248:
microphone `blent_microphone_723c0476e318` / PipeWire object.serial539 and speakers
`blent_speakers_45e1f85c5ee0` / object.serial542. The owner hears the bounded tone;
the microphone supplies144000 real mono48kHz samples. Raw PCM is discarded;
statistics are retained in the parent T1238 acceptance work.

Recorded exact local Stops retire Android speaker2401 and microphone2279,
their original host sessions196709/196988, native helpers197906/197847 and their
separate endpoints. Android exit-info identifies each original process's exit;
host children are reaped and the final PipeWire snapshot has neither endpoint.
Camera delivery and the original authenticated link survive audio Stop. The
first failed sessions are also reaped; the diagnostic creates no retained node.
All raw observations remain under
`/backups/disk2/blent-work/2026-10-21-physical-resumption/`.

The unchanged runner's permanent environment tests and native audio tests remain
in the normal suite. No project behavior is modified by this invocation repair;
no artificial failing source regression is claimed. T1238/T858/T859/T860 remain
unfinished pending their debugging-off hot-reconnect acceptance, which is
independent of this corrected Linux launch setup.
