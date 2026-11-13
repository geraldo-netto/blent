# Native ownership review, 2026-11-13

Source review began at `31912bc7`; probes used the frozen instrumented library
`libblent-c16265474e6be6f1.rlib` from that review baseline, before the changes
listed below. These are historical counterexamples, not claims about fixed HEAD.

## Confirmed outcomes

| ID | Trigger and observed result | Required permanent regression / completion |
| --- | --- | --- |
| T1430 | Linux input instances 4096 and `u32::MAX` panic during unchecked identity arithmetic; 65536 aliases a truncated product range. Public backend probe reproduced the panic without polling a constructor. | Checked independent identity boundary before lifetime admission; all 4096 allowed identities unique, invalid boundaries rejected without panic/native creation, valid follow still admitted afterward. Fixed separately in `ae9bd7ae`. |
| T1432 | A public control worker retains its original thread and pool slot, but `runtime_owner::run` returns before that thread is released. Actual return was observed while `native_complete=false` and `pool_active=1`; releasing the original produces completion and pool count zero. | Admit control workers into the process fence before spawn; close shared configuration executor admission and retire its existing workers at shutdown. Permanent process-boundary regression must hold the original worker until release. |
| T1433 | Cancel a Linux input follower while its OSK `busctl` command is alive and targeted SIGKILL fails. Input native completion becomes true before the exact original command exits. Releasing the controlled child subsequently proves it was reaped. | Retain bounded original child/output observations through OSK cancellation/timeout and preserve the shared keyboard-operation gate until original settlement, including direct daemon restore serialization. Task completion alone is insufficient. Mapping is separately owned by dependent T1439. |
| T1439 | Mapping calls share the unobserved command path; source tracing additionally shows `DeviceOwner::drop` destroys uinput while an identifier-addressed mapping command may remain pending. This mapping-specific destruction race has source evidence, not a new executed probe. | Depends on T1433. Prove a held mapping command prevents original uinput destruction/reuse, retain both original command/collector proofs and device payload, then verify combined input/process-exit acceptance. Permanent held-mapping regression must fail before the fix. |
| T1434 | Successful touch release writes 144 bytes (six native events). After a failed touch UP, recovered `release_all` writes zero bytes: original contact debt was erased before native acknowledgement. | Failed UP/release/partial write/SYN retains original per-device contact debt; fresh controller input cannot overtake it. Successful release or device destruction settles debt. Permanent fault and controller-replacement regressions required. |

The parent owns the authoritative TODO rows and separate fix commits.

## Programs executed and containment

Four Rust programs were executed: `input_identity.rs` (T1430, archived with its
fix artifacts), `control_exit.rs`, `input_command.rs`, and `input_release.rs`.
Their adjacent logs contain the observations, not elapsed-time conclusions.
Three C fixtures provide a held command, rejection of only its exact owned
SIGKILL, and rejection of writes only to the private fake-uinput inode.

Input programs refuse to construct the backend unless the repository's
`host/tests/uinput_mock.c` exports marker 497. The shim redirects uinput to a
private regular file and intercepts ioctls. Private PATH/HOME/XDG directories
prevent desktop commands and configuration effects; the command probe exposes
only its controlled `busctl`. No real input device, active desktop, tablet,
settings, service or installed application was changed. Owned threads and
children were released and awaited after observations. Native construction and
destruction use the fake device; this is not physical hardware evidence.

Reproduction from the historical baseline requires an instrumented library
artifact selected from Cargo JSON (`target.kind == ["lib"]`, not the same-name
binary). Compile each Rust probe with Rust 2021, `-C instrument-coverage`, the
exact `--extern blent=<library>` and `--extern tokio=<matching library>`, and
its build's dependency search directories. Keep executables, dependency paths
and LLVM profiles in an external temporary workspace. Compile C fixtures with
`cc -Wall -Wextra -Werror`; for each input probe link its preload shim with
`host/tests/uinput_mock.c` using `-shared -fPIC -ldl`. Set:

- `LD_PRELOAD` to that external shim and `BLENT_T497_UINPUT_FILE` to an existing private regular file;
- `BLENT_T497_UINPUT_LOG` to a private ioctl log and `LLVM_PROFILE_FILE` to an external profile path;
- `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `PATH` to private fixture directories, remove `DBUS_SESSION_BUS_ADDRESS`, and set `XDG_CURRENT_DESKTOP=review`;
- for T1433, expose only the compiled `held_command.c` as `busctl` in PATH and provide private `BLENT_REVIEW_CHILD` / `BLENT_REVIEW_RELEASE` paths;
- for T1434, use an empty PATH and provide a private `BLENT_REVIEW_WRITE_FAIL` path.

The T1432 program needs neither preload nor native devices. Each input program
checks its containment marker before constructors. Do not run them without this
fail-closed fixture. Logs and source are retained; binaries, raw device event
files, dependency manifests and coverage profiles remain outside the repository.

## Scope and limitations

Source paths examined include Linux input construction, injection, follow/remap
and teardown; input lifetime/task retention; native retirement pool, original
thread/TLS cleanup and process admission; control workers, runtime shutdown and
persistence callers; session worker startup and capture retirement handoff.
Candidate constructor cancellation was traced through the original oneshot send
failure and device destruction; no additional defect was established there.
Existing quarantine on unknown/panicking original work remains intentional.
Session/capture and pool paths received source review, not new native campaigns.

This review did not validate real uinput, EVDI, compositor behavior, Windows,
macOS, hardware cancellation, or every possible native failure. It did not
reopen declined pen/macOS work, blocked native evidence, or performance campaigns.
Finite probes demonstrate the listed counterexamples only. Exact original
settlement, admission before effects, no access after destruction, bounded
retention and conditional cleanup were reviewed separately; elapsed time or
logical task completion was never substituted for native evidence.

`complexity.json` records the repository's syn 2.0.119 and tree-sitter C local
measurements for all 16 retained probe functions: maximum 7, limit 9. Review
source is supporting evidence; permanent regressions belong to the separate fixes.
