# T1035: retain original helper activity control

Optional Linux helper control attaches its reserved inherited endpoint before
spawn, then retains the original controller socket lease with that same child
before further fallible startup work. Canceled/failed handshakes, bounded process
retirement and dependent destruction retain original ownership. Stop/terminate
revoke issuing access independently of resource retention. Existing strict control
transport preserves pending acknowledgements across cancellation and rejects
foreign echoes terminally. Fresh helpers acquire a new channel/sequence only after
original child/stdout retirement. Default uncontrolled composition remains intact;
T1015 must enable control with actual pause/resume policy because controlled native
helpers begin with pixel capture inactive.

Seven permanent CLI regressions exercise real harmless helper subprocesses:
inherited descriptors, transitions with stable child/card identity, held-ack
cancellation/resume, foreign echoes, legacy/unavailable control, canceled/failed
startup with held native destruction, and spawn failure/fresh retry. Initial
missing implementation failed six tests. Removing only original socket retention
then reproduces both held-startup regressions; restoring it passes unchanged
assertions. The held-control fixture explicitly selects FIFO so unrelated optional
raw sockets cannot enter its socket lifetime oracle. An eighth SDK regression
separately verifies independent shared-raw/control endpoint coexistence. The
first SDK fixture ambiguity remains diagnostic only, outside final evidence.

Fresh locked offline host suite: 490 passed, four existing benchmark ignores.
All eight SDK helper regressions and seven contract/portability checks pass.
Matching authored Rust source inventories and native object attestations accompany
raw LCOV. Joined CLI/SDK counters cover all 11 changed production functions
individually at >=98.30%. SDK-only counters omit a live-child stop branch covered
by the full CLI suite; joined matching-source coverage is authoritative.

Source audit: 27 changed production/test functions, maximum complexity 5, using
unchanged repository Sonar-rule counting, cached tree-sitter-rust 0.24.2 and
tree-sitter 0.25.2. Native SonarQube remains unavailable under T1019; no native
certification is claimed. Collector scripts are retained; native objects remain
under /tmp/blent-t1035-evidence. SDK tests use installed libavcodec 61.19.101,
libavutil 59.39.100 and the installed SDK loader in private host namespaces with
one fresh runtime/config directory per regression.

No downloads, installations, uploads, pushes, live monitor changes or physical
power transitions occurred. These process/socket fixtures do not establish native
EVDI monitor/power acceptance. Policy integration remains T1015, encoder generation
retirement T1036, and activity-aware selection T1037.
