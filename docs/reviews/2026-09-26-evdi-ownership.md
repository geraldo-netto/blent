# T599: display ownership is not independent of grabbing

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

EVDI grabbing completes compositor flips, including while a same-GPU consumer
runs. The reviewed ABI couples pixel readback to that completion handshake.

## Evidence

`host/evdi/capture.c::on_update_ready` calls `grab_now` before requesting another
update. Its existing comment explains that grabbing completes compositor flips.
`publish_frame` already checks active frame-exchange generation before converting
BGRA to NV12. Thus the no-reader path already avoids conversion, while continuing
the EVDI update/grab lifecycle. The earlier FIFO CPU profile is not a measurement
of redundant conversion during GPU capture.

In pinned EVDI revision `2713cd41932f2bd8697953a205862a68a966b5ba`,
`evdi_painter_grabpix_ioctl` takes the pending CRTC/vblank, copies pixels, then
sends the vblank event. The public grab mode is DIRTY; zero rectangle capacity
is rejected. Request-update alone checks/arms damage and does not perform this
completion. Neither replacing `poll` nor choosing a newer syscall interface
changes that driver contract. See the pinned
[painter implementation](https://github.com/DisplayLink/evdi/blob/2713cd41932f2bd8697953a205862a68a966b5ba/module/evdi_painter.c#L1000-L1158)
and [ABI](https://github.com/DisplayLink/evdi/blob/2713cd41932f2bd8697953a205862a68a966b5ba/module/evdi_drm.h).

The installed Ubuntu `7.0.0-31-generic` module was also inspected directly:
`evdi_painter_grabpix_ioctl` has relocations to `_copy_to_user` and
`drm_crtc_send_vblank_event`. Its exact module hash and disassembly are retained
in [the evidence directory](artifacts/2026-09-26-followup/t599/). The distro module
is not asserted to be byte/source-identical to the pinned upstream revision.
This supports retaining the existing handshake; no intentional compositor stall
or invalid-ioctl experiment was necessary.

## Implemented boundary

Same-GPU import avoids its own userspace raw-frame transfer. EVDI drain,
GPU color conversion, encoder access and compressed transport remain, so this
is not end-to-end zero-copy. Existing ownership, restart and resize tests remain
applicable. This ABI review changed no production code and claims no readback
savings or cross-GPU support.
