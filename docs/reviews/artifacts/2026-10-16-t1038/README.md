# T1038: negotiated frame-independent video liveness

The original authenticated video socket must acknowledge protocol 1 before the
host emits a six-byte type-2/version-1 record. The protected media greeting now
offers `video_liveness: 1`; input-only compositions omit it. No capability travels
between socket generations. Legacy/unacknowledged/closed views deny writes and
legacy playback waits without a periodic heartbeat timer. Late acknowledgement
wakes the existing idle playback. Successful frame/config/liveness writes renew
the deadline; ready obsolete producer data does not. Configuration and a fresh
IDR remain necessary after retirement. Native/registration/activity authority is
not derived from a heartbeat or its deadline.

`BLENT_VIDEO_LIVENESS_MS` accepts 100–5000 ms, default 2000. Shorter intervals add
traffic and wakeups; longer intervals remain below Android's 10-second read
timeout. Invalid input rejects before subscription/IDR demand. Initial CSD wait
retains its five-second bound; whole writes and batches retain one-second bounds.

Permanent red/green cases cover >10-second no-frame intervals on the same viewer,
fresh resume bytes, obsolete ready producer traffic, authenticated admission,
legacy silence/late ACK, closed grants, actual frame writes, failed writes,
stalled whole-record writes, lag recovery and bounded invalid interval settings.
Three original no-heartbeat assertions failed before emission. Three additional
permission assertions fail with unconditional emission and pass unchanged with
the original-socket guard. The offer assertion fails before advertisement. The
shared host/Android greeting oracle is updated for the implemented optional offer.
Android malformed/bounded type-2 and no-frame/frame isolation regressions and
per-method native counters were committed under T1039 and remain unchanged.

Fresh offline native suites: 787 host-library and 343 shared-contract tests pass;
three existing benchmark ignores remain. Twelve scoped production functions each
have >=95.65% executable-line coverage, using LLVM counters and retained source,
fixture and native-object attestations. Seven architecture/portability checks
pass. All 1,996 Android tests pass again against the updated shared greeting,
without skips. Source audit: 36 changed production/test functions, max complexity 9, using
unchanged repository Sonar-rule counting with cached tree-sitter-rust 0.24.2 and
tree-sitter 0.25.2. Native SonarQube is unavailable (T1019); this is no substitute.
No physical device, lock, suspend, installation, download or push occurred.
T1041 capture capability projection and T1015/T1016/T1011 runtime activity policy
remain separate work.
