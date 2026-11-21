# T1464: X11 input retirement, 2026-11-21

The mapper admitted a pointer-only xinput command for Blent Pen even though X11
classified it as a slave keyboard. Native Xvfb reproduction returns BadMatch.
The generic mutation owner correctly treats an unconfirmed external mutation as
unknown; the input payload consequently remained quarantined and process drain
never completed. The fix rejects non-pointer classes before mutation admission.
It ends discovery when all owned kinds are present and an unsupported class is
reported, without claiming that those devices were mapped.

Both new admission/retirement regressions failed before the fix. All 141 input
and 12 native-command tests passed afterward. Generic unknown-mutation fences
remain intact. The five changed mapping functions have 96–100% individual
executable-line coverage. Project complexity checker (Rust syn 2.0.119) reports
all measured mapping production/test functions <=9; rustc 1.100.0 and
cargo-llvm-cov 0.8.7 supplied counters.

The installed candidate daemon 489627 mapped the two pointer devices, rejected
the keyboard class, destroyed all three uinput devices and exited normally in
0.326 seconds after stop. Original PID exit was observed through pidfd. This
proves the reported native shutdown trigger is fixed, not general leak freedom.
The independently shared USB broker remained alive and exposed a separate
service-restart placement defect, recorded as T1473; it is not daemon completion.

Ownership: mappings borrow a bounded command-output buffer; extracted names and
kinds never escape that call. New inventory storage holds at most the three
owned device kinds. No native mutation/receipt is admitted for rejected classes.
For accepted commands, the original child, collector, mutation worker, uinput
payload and OSK restoration retain the existing exact settlement order. Rejection
creates no new cleanup debt; no deadlines or reference counts were substituted
for original completion. The unchanged generic quarantine tests still pass.
