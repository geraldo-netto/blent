# T1422 — Linux input construction outlives its observer

Reviewed `9f8660aa`. This is a source/isolated-adapter review, not a physical
input-device test or an implementation change.

`host/src/input/linux.rs:598` spawns blocking `InjectDevices::create` and awaits
its Tokio join handle directly. Each successful touch constructor performs
UI_DEV_CREATE and then sleeps 200 ms for udev (`:266`). Canceling the async
follower during that wait drops the join handle; it does not stop or retain an
observable completion for the original blocking operation. `DeviceOwner::drop`
only clears devices already published into its shared mutex, which is still
empty during creation. The blocking result is eventually destroyed when its
unobserved output is discarded.

`LinuxBackend` implements `follow` but inherits `InputBackend::native_completion`
as `None` (`input/backend.rs`). `session_core.rs:159` therefore has no input
native proof to attach to its retained session lifetime. Joining the canceled
input task cannot prove original device destruction. The same-instance names
can be reused by another follower, and session/process retirement can finish
without that constructor's proof. This is separate from the Windows direct-input
worker fixed in T1402/T1403/T1410 and does not reopen declined stylus work.

## Executed evidence

`creation_probe.rs` uses the actual public LinuxBackend and InputBackend APIs.
The existing repository syscall fixture `host/tests/uinput_mock.c` redirects
every `/dev/uinput` open to a private regular file and rejects ioctl calls on
unowned descriptors. The probe verifies its identifying symbol before any
backend use. Touch is enabled; pen and pointer are disabled. PATH, XDG config
and runtime directories point to the private fixture, desktop is `test`, and
the session-bus address is removed. No physical input API receives an ioctl.

Observed one probe execution:

1. First follower canceled and joined: 1 create, 0 destroys, native completion
   absent.
2. Replacement follower using the same backend/configuration canceled and joined:
   2 creates, 0 destroys.
3. Both original blocking constructors settle: 2 creates, 2 destroys, exit 0.

The program waits for both destruction records before runtime teardown. It does
not abandon fixture native obligations. `probe.log` contains the exact output;
the reproduction executes the input adapter boundary, while the consequent
session/process omission is source-traced rather than a full-daemon trial.

Build/reproduction: Rust 1.100.0; first run
`cargo build --offline --locked -p blent --lib --features accessory-transport --message-format=json`.
Compile `host/tests/uinput_mock.c` with
`cc -shared -fPIC -Wall -Wextra -Werror ... -o shim.so` in a private directory.
Compile the probe with rustc edition 2021, the Cargo-reported blent/tokio/libc
rlibs as `--extern`, every reported artifact directory as a dependency search
path and build-script `linked_paths` as native search paths. Run with
`LD_PRELOAD=shim.so`, `BLENT_T497_UINPUT_FILE` naming the private regular file,
`BLENT_T497_UINPUT_LOG` naming its new log, and `BLENT_T497_UINPUT_FAIL=0`, plus
the environment isolation above. Compiled artifacts stay outside the repository.

Five authored probe functions were measured by the project syn 2.0.119
cyclomatic checker: maximum 2; exact scores/hash in `complexity.json`.
There is no newly claimed native platform acceptance or unrestricted leak proof.

The TODO row requires a permanent blocked-construction regression before the
eventual fix, an exact owner/proof propagated into session retirement, stale
result cutoff, same-identity exclusion, partial-failure cleanup and graceful
exit acceptance. Production/test sources and installed applications are unchanged.
