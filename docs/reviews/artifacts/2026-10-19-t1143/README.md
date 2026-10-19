# T1143 owned Linux output configuration

Spec copies a validated portable camera profile into an immutable bounded image/
rate contract. Configuration owns one already admitted original File, applies
single-planar progressive YUV420 and verifies dimensions, exact stride/frame size
and rational rate. A matching current rate needs no setting; a different rate
requires the driver's explicit time-per-frame capability and accepted readback.
ConfiguredOutput hands that same file description and contract to a writer.
No path is reopened and no OS handle enters UI, configuration or wire schemas.

Input ownership moves once; failure closes it after the synchronous ioctls finish.
The native payloads remain exclusively borrowed and initialized through return;
no pointer/view or callback is retained. This blocking backend must run inside
an admitted owner before writer creation (T1137 composition). Its result is not
a native-worker join or permission to replace pending work. Kernel ABI calls
are assumed to honor synchronous borrowed-memory semantics; external mutation
that violates the caller's exclusive ownership is outside that precondition.

Eleven normal host tests include631 bounded profile/readback/result cases, each
native stage failure, matching/unsupported static rates, independent installed
C-header size/alignment/offset/constant checks, and kernel flock checks for owned
handoff and every partial failure. The virtual-driver entry point is a helper,
not an ordinary-host claim of driver qualification.

An isolated offline guest boots cached Ubuntu7.0.0-34-generic with its matching
stock v4l2loopback0.15.3 module. Real configuration/write at320x240 and30/5/7/30FPS
matches an independent C query through the same inherited original capability.
The module unloads after all original files/helper processes settle. No host
module, device or service is changed; physical camera/tablet acceptance remains
explicitly deferred T858. Only current Linux x86_64 native evidence is claimed.

Native coverage and reproducibility evidence follows below. The pinned source
audit stays <=9 and is not actual SonarQube measurement (T1019). New C follows
Allman formatting using installed clang-format18.1.3. The only lockfile change
adds an existing cached linux-raw-sys0.12.1 direct dependency; no version changes,
package installation, download, upload, Windows/EVDI implementation or push.
T1137's unchanged parent-loss/path-swap regressions remain red pending integration.

The initial draft's GNU-only ioctl integer cast is also rejected by a permanent
compile contract using the actual native.rs against unsigned-long, signed-int
and unsigned32 request prototypes. The unchanged oracle passes after inference
at the FFI boundary. This checks signatures; it is not native musl qualification.

Final offline source/compiled-object-attested LLVM evidence combines the host
suite and the identical instrumented ELF in the real-driver guest:17/17 production
functions pass individually >=80% executable lines, none unmeasured or newly
excluded. Raw manifests, original host LCOV, guest profiles and combined export:
`/backups/disk2/blent-work/2026-10-19-open-items/t1143/native-final` and
`final-native-guest-reviewed`. Earlier `native`/guest runs predate the ioctl correction
and are historical. The guest ELF hash matches the attested compiled object;
export metadata preserves original report hashes and guest profile hashes.

Pinned source audit:61 authored Rust/C/Python functions, maximum8.
The embedded compile-contract exercise has complexity1. Scoped host library
Clippy and Cargo formatting pass. The permanent offline guest runner replaces
itself with the foreground QEMU process, so the caller waits for that same
identity's actual exit before checking evidence; it leaves no second controller
owning a detached VM. Its explicit domain is2 virtual CPUs/2048MiB, installed
zstd-compressed matching modules, supplied readable kernel/static busybox/ELF,
and a fresh comma-free output path. Read-only programs and a private evidence
share are the only host filesystem exports. Interrupted/failed output is retained.

Reproduce after the focused native collection, substituting its emitted ELF path:

```sh
python3 host/src/camera/output_device/tests/loopback_guest.py run \
  --kernel /path/to/readable/matching/vmlinuz \
  --release 7.0.0-34-generic --busybox /path/to/static/busybox \
  --binary /path/to/instrumented/blent-tests --output /path/to/new/fixture
python3 host/src/camera/output_device/tests/loopback_guest.py check /path/to/new/fixture
```

Run `check` after the foreground VM exits; a QEMU exit alone is not a test pass.
No elapsed execution cap is used. Preparation owns only finite local file/tool
operations; after exec the VM itself owns its2GiB context until actual exit.
