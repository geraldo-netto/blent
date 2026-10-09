# T750 isolated snapshot fixture

Development-only native acceptance fixture. No EVDI changes. Programs refuse
execution unless `/proc/cmdline` contains `blent-evdi-review=1`; use a disposable
VM, never add that marker to a live host. No downloads or package installation
are part of these programs. Do not build with `NDEBUG`.

`produce.c` uses stock libdrm with an already-connected EVDI output. It makes
240 immutable 640×480 XRGB framebuffers, each changing a 16-row frame-ID strip
and one 64×56 tile. Damage metadata contains two rectangles, or 63 overlapping
rectangles when `SNAP_FRAGMENTED` is set. First 120 commits request page-flip
feedback; the rest omit the optional feedback event. DRM can still serialize
those commits; this does not guarantee multiple pending commits.

`wayland.c` presents the same immutable patterns through stock xdg-shell/SHM,
requests presentation feedback and advances on frame callbacks. Generate its
`xdg-shell.h/.c` and `presentation-time.h/.c` with `wayland-scanner` from locally
available protocol XML. Generated files are dependencies, not maintained copies.
Link those generated C files and `-lwayland-client`; add their directory with
`-I`. It must run in the guest compositor's environment.

`observe.c` is a guest-only `LD_PRELOAD` observer around stock libevdi registration
and grab calls. Original functions remain unchanged and are called through
`RTLD_NEXT`. Every nonempty capture is checked before Blent conversion. One
bounded sample prints blue-byte values at fixed coordinates for diagnosis.
Compile with `cc -O2 -shared -fPIC observe.c -ldl -o observe.so`.

`consume.c` reads Blent's packed NV12 FIFO at `/tmp/video` for 16 seconds. It
checks every complete frame, forward frame-ID order and final convergence to
240. The intentional gray startup placeholder is allowed only before a captured
pattern. `SNAP_COMPOSITOR=1` also permits compositor background before the first
recognized pattern; **no recognized frame always fails**, and malformed frames
after recognition are never ignored. The full image and UV plane are checked,
not just the identifier. Compile with `cc -O2 consume.c -o consume`.

Compile the KMS producer with `cc -O2 -I/usr/include/libdrm produce.c -ldrm
-o produce`. Guest setup must start the unmodified Blent helper with the FIFO,
then the consumer and producer/compositor. Source buffers remain immutable and
allocated until the producer exits, so buffer-reuse tearing cannot explain a
failed snapshot. Skipping intermediate frames is permitted; mixed snapshots,
backward IDs and failure to converge are not.

`host/tests/snapshot_order.c` uses this pattern contract in the normal automated
helper suite, with coalesced/reordered/duplicate damage and retained writer
leases. Bounded deliberate pixel corruption checks the oracle's sensitivity.
It does not require a guest, libdrm, libevdi or Wayland.

Native run configurations, observed results and limitations:
[2026-10-09 evidence](../../../docs/reviews/artifacts/2026-10-09-t750-snapshot/README.md).
