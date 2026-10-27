
# Blent settings

Use **Settings** on the desktop for shared defaults and the tablet gear menu for
Android preferences. Select a tablet in the workspace before starting a resource.
Saving preferences and starting sharing are separate actions.

## Where settings are saved

Host settings live in `~/.config/blent/config.toml`; edit them with
`blent-gui` or override supported settings with CLI flags. An absolute
`XDG_CONFIG_HOME` changes the path to `$XDG_CONFIG_HOME/blent/config.toml`;
empty or relative values use the default. The tablet’s ⚙
sheet stores app preferences locally; **Apply** sends its shared streaming
settings to the host. Brightness/refresh preferences take effect immediately.

## Desktop workspace and resource controls

The desktop workspace adapts between a tablet sidebar and a compact selector.
Connection status stays explicit: a remembered or last-seen tablet is not proof
of a current connection, and a display assignment is not proof of video delivery.

**Configure** opens the relevant resource preferences; **Settings → Video**
keeps encoder, frame rate and resolution visible;
**Video → Encoding GPU** selects a stable Linux VAAPI adapter or CPU encoders.
Automatic preserves existing device choices; requested and active devices appear
separately. **Save & restart all displays** is required. Missing devices and unsupported codecs
use software fallback; this does not move desktop composition.


**Advanced video settings** contains quality, bitrate, calibration and worker/pipe
capacity controls. The tablet groups display/input, video, camera and app controls;
bitrate is under **Advanced video settings**, and update/statistics switches are
under **App & diagnostics**. The decoder panel shows the active codec, advertised
capabilities, requested performance hints and watchdog fallback; effective hints
remain unknown. Camera Start/Stop is explicit and independent of
display sharing; opening desktop settings or pressing Save does not start capture.
Save/Discard controls appear in preference editors. Workspace resource Start uses
the exact selected tablet without rewriting saved target preferences. Explicit
Start in a camera/audio editor uses its target fields. Stop preserves the original
resource owner until native cleanup completes; selection changes cannot retarget
queued or retiring work. Global display-service controls remain in **Settings**.

When USB media is authenticated without a streaming desktop, the tablet shows
**USB media connected** and **Open camera and audio controls**. That button opens
the existing local controls directly; it grants no media consent. Pairing alone
does not enable Start; media needs a current host request and local approval. The existing ADB desktop
connection still requires USB debugging, independently of USB media pairing.

While Blent is visible, active camera, microphone and speaker sharing keep the
screen awake through the same window policy as display sharing. Stopping a
resource removes its hold; another active resource may still need the screen.
Home and manual screen lock retain the existing foreground/background consent
rules. Blent does not change Android's system screen timeout or charging settings.
The tray controls the running daemon.

## Input and tablet display

- **Graphics tablet mode** — flip *Graphics tablet* on the tablet: nothing is
  streamed and the pen drives your own screen. Input latency remains. Switch back
  the same way. The connection overlay clears after the authenticated control
  greeting; it returns when the control connection is lost.
- **Tablet display** — brightness defaults to 50%, and the app requests 60 Hz.
  Adjust either in the gear menu; preferences persist and affect Blent only.
  Other apps retain normal system settings. Refresh selection uses the closest
  supported rate at the current display resolution; **System default** clears
  the request, and Android may override it. Stream FPS is separate.
- **Position** — `right` (default), `left`, `above`, `below` your real screens.
- **Orientation** — in the tablet's ⚙ sheet: rotate automatically with the
  tilt sensor, or pin *camera up* / *camera down*.
## Video

- **Codec** — new configurations default to `auto`: check tablet compatibility,
  measure available host encoders and, with current peers, compare bounded live
  render-ACK timing with a quality guard before accepting a selection. Saved
  explicit choices remain unchanged. Choose NVENC,
  VAAPI or a software encoder to override automatic selection. Automatic mode
  [saves tuning per environment](video-codecs.md#durable-environment-tuning)
  and reuses it across reconnects and reboots after a fresh render check. First-run
  tuning keeps one progress screen; **Recalibrate display** requests new tuning. VP9 and AV1
  (software or supported VAAPI/NVENC encoders) use tablet
  capability negotiation; see [codec compatibility](video-codecs.md). HEVC is optional and needs a compatible tablet decoder.
  `h264_vaapi_baseline` is an explicit low-latency VAAPI H.264 profile; it trades
  higher bandwidth for lower decoder delay on the measured tablet. Existing
  selections stay unchanged; see [codec measurements](benchmarks/2026-09-18-codecs.md).
  `ten_bit` requests HEVC Main10 on the FFmpeg path; capture is still 8-bit,
  and this does not enable HDR or guarantee less banding on every scene.
- **Bitrate and quality** — VAAPI uses uncapped constant quality (CQP). Its
  bitrate control is disabled on Linux; adjust **Quality** instead. The saved
  bitrate remains available for other encoders. **Auto** may select VAAPI and
  therefore may not enforce the configured bitrate. Android explains this
  limitation in its settings too; the value is not measured throughput.
- **Stream scale** — `stream_scale = 2` sends a quarter of the pixels for a
  historical ~6 ms lower packet-to-ack median on the reference tablet, at the
  cost of softer text; other devices differ.
  The desktop GUI also offers **180p (320×180)** and **360p (640×360)** stream
  presets. These select a 1280×720 desktop and scale its stream by 4 or 2; restart
  display sharing to apply. FPS, bitrate and quality stay under your control.
  Lower detail is useful for constrained links, but configured bitrate is not
  measured traffic and a calling application's upload settings remain separate.
## Connections and input devices

- **Several tablets** — `max_tablets` supports 1–4 slots. Historical testing
  used one physical tablet plus a simulated client. Automated tests cover
  independent slots and busy-card fallback. Multiple physical tablets have not
  been validated; maintainer testing of that setup is outside the current scope (T382).
- **Input devices** — `input_touch`, `input_pen`, `input_pointer`: which
  virtual devices the desktop sees while a tablet is attached. All on by
  default; turn off what you do not use. On Cinnamon/X11, Blent keeps the
  mouse visible while its touchscreen exists, including during touch input.
  Other desktops retain their own cursor policy; an unavailable Cinnamon
  policy produces a daemon warning. The virtual pointer requires Pen;
  disabling Pen preserves the pointer preference for when Pen is enabled again.
- **Wi-Fi** — `blent wifi` once, with the cable in: it switches the tablet
  over, remembers the address and reconnects to it by itself whenever the
  cable is out. `blent wifi --off` forgets the address and disconnects; it
  does not disable the tablet's network adb listener. See [SECURITY.md](../SECURITY.md).
  USB preference also recognizes network ADB endpoints and mDNS wireless
  identifiers. See [benchmarks](benchmarks.md)
  for historical Wi-Fi results.
## Power, queues and updates

- **Battery saver** — an opt-in Android setting that preserves brightness,
  refresh rate and stream settings. It releases unnecessary CPU/Wi-Fi locks on
  USB or while waiting, while retaining the network lock for active network
  sessions. It applies immediately; turning it off restores the normal policy.
- **Control queue limit** — Android Settings, App & diagnostics: 64 KiB by
  default, with 8 KiB, 256 KiB and 1 MiB options. Smaller limits reconnect
  congested sessions sooner; larger limits tolerate bursts but retain more
  input/ACK backlog. The limit applies to new sends, including authentication,
  settings and decoder reports. An overflowing message retires the entire
  control connection so the host releases uncertain gestures; events are not
  replayed on its replacement. This is a byte budget, not a wall-clock latency
  guarantee. Very small limits may reject large decoder reports.
- **Updates** — the app, the GUI and the tray tell you when a newer release
  exists; nothing installs itself. `check_updates = false` disables host checks;
  the tablet app has its own update-check switch.


For dedicated workflows, see [camera](cameras.md), [audio](audio.md),
[saved computers](saved-computers.md), [video selection](video-codecs.md),
[pipe capacity](pipe-buffer.md) and [CPU scheduling](scheduling.md).
