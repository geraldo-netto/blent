
# Blent settings

Use **Settings** on the desktop for shared defaults and the tablet gear menu for
Android preferences. Select a tablet in the workspace before starting a resource.
Saving preferences and starting sharing are separate actions.

## Where settings are saved

Host settings live in `~/.config/blent/config.toml`; edit them with
`blent-gui` or override supported settings with CLI flags. An absolute
`XDG_CONFIG_HOME` changes the path to `$XDG_CONFIG_HOME/blent/config.toml`;
empty or relative values use the default. The tablet’s ⚙
sheet stores app preferences locally. Current host bitrate and FPS seed the stream
controls; **Apply video changes**, directly below those controls, sends only edited
fields and is disabled when nothing changed. **Done** closes the sheet; closing or
dismissing with pending video edits asks whether to discard them or keep editing.
Immediate display/app changes are retained when video edits are discarded.
Explicit field overrides are saved and resent on reconnect. **Use host settings**
clears those overrides and prevents future resend; it does not undo or cancel changes already
sent to the host. Brightness/refresh preferences take effect immediately.

At startup, a missing host configuration uses defaults silently. Unreadable or
malformed configuration also uses defaults, with a warning identifying the file
and error. Loading never overwrites that file; partial updates require a readable,
valid configuration. On Unix, saves report success only after syncing both the
file and its parent directory.
Save also validates camera, microphone and speaker settings. Incomplete USB
fingerprints, invalid ADB serials or incompatible target selectors disable Save
and show the resource and validation reason beside it. Audio profiles must match
their microphone or speaker resource; Start also checks that identity before
admitting a backend or queuing a replacement.

## Desktop workspace and resource controls

The desktop workspace adapts between a tablet sidebar and a compact selector.
On X11, initial window centering waits at most 25 geometry checks for the window
to become visible, then leaves placement to the desktop. Wayland controls its
own positioning.
Connection status stays explicit: a remembered or last-seen tablet is not proof
of a current connection, and a display assignment is not proof of video delivery.

**Configure** opens the relevant resource preferences; **Settings → Video**
keeps encoder, frame rate and resolution visible;
**Video → Encoding GPU** selects a stable Linux VAAPI adapter or CPU encoders.
Automatic preserves existing device choices; requested and active devices appear
separately. **Save & restart all displays** is required. Missing devices and unsupported codecs
use software fallback; this does not move desktop composition. Discovery checks
current read/write permissions without opening render nodes. It reuses GPU labels
until device identities or driver metadata change. Permission evidence does not
prove that opening the device or using a codec will succeed.


**Advanced video settings** contains quality, bitrate, calibration and worker/pipe
capacity controls. The tablet groups display/input, video, camera and app controls;
bitrate is under **Advanced video settings**, with whole-Mbps stops from 1 to 60
and a 20 Mbps default. Its label preserves the exact kilobit value; an
existing fractional setting remains exact until changed. Update/statistics
switches are under **App & diagnostics**. The decoder panel shows the active codec, advertised
capabilities, requested performance hints and watchdog fallback; effective hints
remain unknown. Camera Start/Stop is explicit and independent of
display sharing; opening desktop settings or pressing Save does not start capture.
When the tablet display rotates during camera sharing, Blent briefly restarts the
selected camera with the new orientation. Replacement waits for the old worker's
actual retirement; rapid turns keep only the latest requested orientation.
Inactive cameras stay off. This also follows display changes during explicitly
enabled background camera sharing.
Save/Discard controls appear in preference editors. The existing two-second background poll also observes saved configuration changes, including tablet settings persisted by the daemon. It rebases unsaved local edits onto the latest valid snapshot; Discard uses that new baseline. Missing, malformed, non-regular or over-1-MiB snapshots do not replace the draft. A read started before a completed GUI save cannot overwrite that save. Workspace resource Start uses
the exact selected tablet without rewriting saved target preferences. Explicit
Start in a camera/audio editor uses its target fields. Stop preserves the original
resource owner until native cleanup completes; selection changes cannot retarget
queued or retiring work. Global display-service controls remain in **Settings**.
If a camera/audio run has already ended and you start it on another tablet,
Blent retains that request until the original worker finishes cleanup. This also
progresses while its editor is hidden. Stop cancels the queued start; a failed
successor reports its error without an automatic retry.

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
### Windows touch/mouse preview

The opt-in **Touch/mouse input preview** chooses a host monitor and either touch or
finger-driven mouse input. Its saved `[direct_input]` table contains `monitor`
(the selected monitor identity) and `mode` (`touch` or `direct_mouse`). The table
is absent by default. Root settings `input_touch` and `input_mouse` both default
to `true`; the selected mode must be enabled. Save and restart display sharing
to apply changes. This Windows preview still requires physical tablet acceptance;
stylus input is unavailable. It does not select ADB versus direct USB transport.

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

In **Connections → USB devices**, each row shows the USB product/model name
and manufacturer reported by the device. Cards reflow into columns as the window
widens and return to one column in narrow windows. For example, a tablet can appear as
**RugKing Pad 2 Pro**, with **Manufacturer: Unisoc** below it. USB identifiers
remain secondary details. Missing names are marked unavailable; the USB
manufacturer may differ from the retail brand. **Android accessory mode** helps
identify an Android candidate, but names and USB IDs do not authenticate it.
Compare the pairing fingerprints before approving a tablet.

A USB row shows its broker connection state, including **Connected**,
**Disconnecting**, and **Cleanup pending**. **Connect** stays disabled while
that connection owns the physical device, including during discovery refreshes.
It becomes available only after the connection and its native worker retire.
Use the connection’s **Disconnect** button to stop it.

The broker defaults to 128 local clients, 4 tablet links and 64 registrations.
Each registration uses one client; media bridges use separate loopback streams.
Clients awaiting original task retirement still consume capacity. At capacity,
one bounded exchange returns **Busy** for the request before any route is admitted;
other excess connections wait in the socket backlog while existing clients remain
served. Missing replies never prove remote resource cleanup.

For a newly launched on-demand broker, set `BLENT_USB_CLIENTS` (1–1024),
`BLENT_USB_LINKS` (1–64), or `BLENT_USB_REGISTRATIONS` (1–1024). A manually
started broker offers `--clients`, `--links` and `--registrations` with the same
defaults and ranges. These settings apply at broker startup. Higher limits permit
more retained client tasks, bounded message queues, links and route owners; lower
limits reduce that capacity. Leave client headroom above expected registrations
for connection controls. Changing limits does not resize a running broker.

A failed USB scan is shown as **USB inventory unavailable**. Existing links,
pairing choices and saved tablets remain visible; Disconnect, pairing decisions
and Forget remain available. New connections require successful discovery.
The first inventory view waits for the initial scan to finish, including failure.

Android reports worker binding/startup expiry as a connection failure, rather
than a normal end. Saved-computer list/forget operations have a separate
five-second observation budget after startup. Expiry does not prove native
cleanup: the original binding and IPC calls remain owned until retirement.

Once a computer has remembered peers, its running USB broker automatically tries
devices already in Android accessory mode on startup and reconnect. Normal USB
keyboards, cameras, phones and tablets are never switched automatically. Use
**Connect** to switch a tablet that has returned to its normal USB mode. An unchanged
saved fingerprint skips repeat approval; every connection still requires fresh TLS proof. A changed
fingerprint requires approval again. USB names and locations are scheduling hints,
not trust. Devices reserved for ADB or with unknown availability are not attempted.
Startup failures before native admission retry after 2 seconds, doubling up to
60 seconds; successful authentication resets the delay. Each retry waits for the
original native owner to retire. Native discovery that ends without a Blent Hello,
Disconnect, Deny and Revoke suppress automatic retries for that physical attachment
until confirmed unplug/replug or an explicit Connect request. An
unavailable inventory cannot clear that suppression. Pairing grants no media consent.

On Android, an accessory attachment received during an old connection's cleanup
is retained until that owner retires. After a successfully authenticated,
remembered connection ends unexpectedly, the current Activity retries without
requiring another cable event while a supported accessory is still present.
A missing or unavailable accessory snapshot stops that retry chain; a fresh
attachment event or explicit Connect can start another connection. Presence
checks share the original bounded snapshot owner and do not consume attachment
events or grant media consent. Retries wait 2 seconds initially, double after
failed attempts up to 30 seconds, and return to 2 seconds after authentication.
Changed or forgotten peers still require approval; an initial unknown connection
or an unremembered manual failure does not start a retry loop. Disconnect, Deny
and Revoke stop automatic recovery, including while cleanup or a retry is pending.
Activity restoration does not replay the original USB attachment; a fresh
attachment Intent or explicit Connect is required after an explicit cutoff.
Destroying the Activity cancels its retry observer; an established application-owned
link remains owned. This does not promise unattended recovery after application
process death or certify physical tablet/display behavior.

Android’s **Native setup timeout (ms)** also bounds TLS handshake and authenticated
protocol setup reads. Its range is 1–60000 ms, with a 10000 ms default. TLS uses
the remaining setup budget and cannot extend the remaining approval window.
Preface fragments share that deadline; successful setup restores the caller’s
stream read timeout. These setup limits do not set the lifetime of an established
connection.

While a device advertises USB debugging, its row shows **Reserved for USB
debugging** and disables **Connect**. Use **Tablets** for ADB display sharing.
To use debugging-free USB instead, turn USB debugging off on the tablet and
refresh discovery. An unreadable USB configuration shows **USB availability
unknown** and also disables Connect. The native helper rechecks the selected
device before any mode switch, reset, or interface claim.

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
  cable is out. `blent wifi --off` forgets the address and attempts to disconnect.
  An explicit socket refusal releases the attempt for a later retry. A timeout
  or unknown creation outcome retains the endpoint until its original outcome
  can be settled; automatic retry is suspended during that interval. See
  [network ADB ownership](adb-network-lifecycle.md).
  It reports a confirmed disconnect separately from forgetting an address when
  ADB is unavailable, and returns an error if cleanup cannot be admitted or
  confirmed. It does not disable the tablet's network adb listener. See [SECURITY.md](../SECURITY.md).
  USB preference also recognizes network ADB endpoints and mDNS wireless
  identifiers. See [benchmarks](benchmarks.md)
  for historical Wi-Fi results.
## Power, queues and updates

### Host activity pauses

The desktop configuration accepts the following `[host_activity]` settings.
They have no GUI controls; restart the daemon after editing them.

| Key | Default | Display behavior |
| --- | --- | --- |
| `pause_on_lock` | `true` | Pause display work while the host is locked, where lock observation is supported. |
| `pause_on_display_off` | `true` | Pause display work while the host display is off, where display-power observation is supported. |
| `pause_without_viewer` | `true` | Pause display work when there is no active tablet viewer. |

When a supported lock or display-power observer has no confirmed state, its
enabled pause also applies. Disabling one setting disables that pause reason;
other reasons still apply. Host suspend, unavailable power state and unknown
state from a supported power observer independently block display and auxiliary
media work. These three preferences do not bypass native cleanup or Android
camera/microphone consent. Lock, display-off and no-viewer reasons themselves
affect display work, not auxiliary camera/audio work. Backend observation support
is separate from physical-device acceptance.

Activity-source creation failures, including the first attempt, remain supervised.
Blent withholds display demand while evidence is unavailable, waits for the
original creator's native completion, then retries after the source deadline.
Stopping cancels retry admission; it does not discard unsettled native ownership.

### Other controls

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

## Update checks

When `check_updates` is enabled, the daemon first checks after 30 seconds. A valid
response schedules the next check in 24 hours. Failed or invalid responses retry
after one minute, doubling up to one hour; a valid response resets the retry delay.
Failures retain the last valid update notice. Checks do not download or install updates.

Root configuration keys must belong to the current schema; unknown and obsolete
keys are parse errors. Existing read-error handling warns and uses defaults in
memory, but locked edits preserve the invalid file and return an error. Correct
the reported key before saving. No automatic obsolete-field migration is used.

Camera options, camera sensor controls and host activity tables also reject unknown
keys. Camera options stay directly under `[camera]`; sensor controls belong under
`[camera.sensor]`. There is no `[camera.options]` configuration wrapper. Current
known fields and omitted-field defaults retain their existing meaning.

### Desktop appearance

On Linux with GTK3/PyGObject and fontconfig available, Blent reads the desktop's
light palette, font, text scale and animation preference at GUI startup. It keeps
readable text contrast, including selected controls. This themes egui controls;
it does not replace them with native GTK widgets. General → Advanced app settings
shows the appearance source or why standard light defaults were used. Theme
changes apply at the next GUI start, not when a tray click reveals the same window.
On Debian/Ubuntu the optional packages are `python3-gi gir1.2-gtk-3.0 fontconfig`.
