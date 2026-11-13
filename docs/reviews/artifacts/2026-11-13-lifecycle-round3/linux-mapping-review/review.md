# T1444 — native command completion is not delegated mutation completion

The private probe runs the real installed `/usr/bin/busctl` against a private
KWin-shaped D-Bus property setter. After the setter enters, the command reaches
its explicit one-second deadline. Both original command receipts complete while
the remote setter is still held. Releasing that setter then applies its mutation
after both receipts. Exact output: `delegated-set.log`; independently authored
source: `delegated_set.rs`. This establishes the acknowledgment gap in
ObservedCommands.complete and its keyboard-gate/mapping-lifetime consumers.
It is a native D-Bus protocol counterexample, not a physical KWin device-reuse
trial or proof about every compositor scheduling order.

The mapping draft otherwise serializes commands behind original child/collector
proofs, moves a retiring payload out of the live sink, and preserves the original
constructor independently. Payload/command/OSK references have no back-reference
to the parent Lifetime. Its separate task-admission saturation concern was sent
to the input agent as a T1439 draft correction, not counted as another T1444 bug.

Cursor preparation produced no finding: the JavaScript hook selects an exact
name/type, does not adopt existing devices on creation, expires after ten seconds
without a device, and cleans up when its exact adopted object is removed. A delayed
Eval reply alone did not establish a wrong effect or persistent leak.

## Native acknowledgment references

The installed xinput package is 1.6.4-1build1. Its
[official 1.6.4 source archive](https://www.x.org/releases/individual/app/xinput-1.6.4.tar.xz)
shows main calling XSync and XCloseDisplay after the selected command returns.
Normal successful map-to-output exit therefore includes a server round trip.
A killed process or an early Xlib failure need not cross that boundary.
The archive SHA-256 is
`ad04d00d656884d133110eeddc34e9c69e626ebebbbab04dc95791c2907057c8`.

The installed busctl is systemd 255.4-1ubuntu8.18. The
[systemd v255 source](https://raw.githubusercontent.com/systemd/systemd/v255/src/busctl/busctl.c)
passes arg_timeout into the Set method call; `--timeout=infinity` is accepted by
the installed parser. Local source SHA-256 is
`555d7dc408febed4d4ccecf2f8e3614d463665509a6a99450b5e7153ea8d3cd0`.
The [busctl manual](https://www.freedesktop.org/software/systemd/man/latest/busctl.html)
describes the ordinary 25-second default, which is distinct from Blent's caller
deadline. [Qt's qdbus source](https://raw.githubusercontent.com/qt/qttools/dev/src/qdbus/qdbus/qdbus.cpp)
waits for a method reply and maps error messages to nonzero exits; it does not
expose an equivalent native-timeout option. Its own timeout or ambiguous CLI
failure must remain unknown rather than being inferred from child exit.
[KWin's setter](https://raw.githubusercontent.com/KDE/kwin/master/src/backends/libinput/device.cpp)
updates output association synchronously before returning. No upstream
implementation has been incorporated into project code.

## Scope and reproduction

Compile the probe with rustc edition2021/instrument-coverage, linking the existing
matching blent_config e8fb2a5f0cf99980, zbus240d4e1d9a0efdaf and
tokio9a9d4a755c6dd115 rlibs under target/debug/build, with dependency directories
for their matching out directories. Execute under a new dbus-run-session with
LLVM_PROFILE_FILE pointing outside the repository. The executable is retained at
`/tmp/blent-lifecycle-round3-linux/delegated-set`; no Cargo lane or live service
was used. Its command receipts settled and private server closed before exit.
`complexity.json` records exact source/binary/library hashes and all six authored
probe functions; maximum complexity is three. No owned probe remains running.
