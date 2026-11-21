# Troubleshooting

Start with `blent doctor`. It checks configuration, tools, devices, adb and
selected desktop/decoder state. ADB is Android's debugging connection tool;
EVDI creates the extra Linux display. Doctor offers diagnostic suggestions;
it does not certify every desktop or encoding combination. Include the exact
fork commit as well as host/app version numbers in a report.

Doctor checks the daemon's reported tablet sessions when available. Without a
live session report, it selects an authorized ADB candidate, preferring USB,
even if the tablet app is absent; the package check then reports installation
instructions. This diagnostic selection does not start sharing or install anything.

Autostart diagnostics recognize both the systemd user service and an enabled
desktop login entry. States such as `static`, `masked` and `enabled-runtime` are
reported by name; a failed or unreadable observation is reported as unavailable.
Doctor does not enable, disable or rewrite either autostart route.

## "Failed to start helper" / "evdi-helper exited prematurely"

Read the preceding error. Possible causes include a missing/incompatible
EVDI module or userspace library, permissions, a busy card, invalid EDID or a
missing helper executable. Check `modinfo evdi`, `cat /sys/devices/evdi/count`
and `ldd` on the installed `evdi_helper`. A missing sysfs path differs from
a loaded module reporting zero devices.

If the module is loaded but has too few devices, use GUI system setup and
follow its reboot guidance. Setup reuses existing devices and refuses live growth
under Xorg (or when it cannot determine whether Xorg is running). Review
`initial_device_count` in `/etc/modprobe.d/blent-evdi.conf` before rebooting;
preserve other options and administrator-managed settings.

**Do not unload EVDI or write directly to `/sys/devices/evdi/add` in a running
Xorg session.** Driver changes can disrupt the desktop. The installer preserves
existing devices and boot configuration; see [safe capacity changes](installation.md#evdi-setup-and-safe-capacity-changes).

## "Failed to open /dev/uinput"

Check whether the module is loaded and the current desktop seat has access.
From a source checkout, when the Blent udev rule is missing:

```bash
sudo modprobe uinput
sudo install -Dm644 packaging/60-blent-uinput.rules /etc/udev/rules.d/60-blent-uinput.rules
sudo udevadm control --reload
sudo udevadm trigger --name-match=uinput
```

Run Blent as the logged-in desktop user; do not use a root daemon as a
permission workaround.

## The app keeps opening and closing / "did not authenticate"

Check that the app and host came from the same checkout. For the ADB route,
check that ADB still shows the tablet as authorized. A stale token, failed token delivery or
mismatched builds can cause this; an older app is not the only explanation.
Authentication is always required; there is no disable switch. ADB delivers
attachment credentials automatically. Direct USB remembers approved fingerprints
by default and asks again when the peer fingerprint changes. Delivery retries back off, potentially to ten minutes; reconnecting the
cable triggers a fresh attempt. Do not include session tokens in public logs.

## Interrupted USB trust or identity

The Linux USB broker resumes a complete interrupted trust update or identity
publication using the original record. An incomplete or conflicting record fails
closed. Its launcher now reports an observed startup exit promptly; another elected
broker still starting keeps the normal readiness wait.

For damaged saved trust, run the matching installation's `blent-usb-broker`
executable (source installations normally put it in `~/.local/bin`). The state
path is `$XDG_STATE_HOME/blent/connection`, or `~/.local/state/blent/connection`
when `XDG_STATE_HOME` is unset. A custom broker uses its explicit `--state` path.

Run repair after the broker for that state directory has exited. A live owner
causes an in-use error. Request normal termination with SIGTERM to that identified
broker process and wait for its cleanup before retrying; retain the stable lock
files. The reset command requires explicit confirmation:

```sh
blent-usb-broker --state "${XDG_STATE_HOME:-$HOME/.local/state}/blent/connection" \
  --reset trust --confirm-reset
```

This removes remembered tablet approvals and interrupted trust files while
preserving the host certificate/key. Reconnect and approve the tablets again.
For incomplete or invalid host identity storage, explicitly select the broader
reset instead:

```sh
blent-usb-broker --state "${XDG_STATE_HOME:-$HOME/.local/state}/blent/connection" \
  --reset identity --confirm-reset
```

Identity reset also removes every approval bound to the former identity. The next
broker start creates a new identity, which requires renewed approval on the tablets.
Both resets retain durable intent through interruption and finish it before admitting
trust. Unsafe file permissions, links or foreign ownership require correcting the
filesystem problem; a reset reports that failure and preserves its recovery marker.

On Android, use **Settings → Saved computers → Reset saved computers**, then
confirm. This preserves the tablet identity. See [saved computers](saved-computers.md)
for cancellation and cleanup behavior. The host repair adapter is Linux-only.

## Black screen on the tablet

1. Check `blent status` and the connection route shown in the GUI.
2. For ADB only, check `adb devices` and `adb -s TABLET_ID reverse --list`
   for the assigned ports (8890/8891 by default for the first slot). For paired
   direct USB, check the desktop connection/trust status instead; ADB listings
   do not diagnose that route.
3. For a service launch, inspect `journalctl --user -u blent -n 200`.
   Direct GUI launches log to `~/.local/share/blent/daemon.log`.
4. Check the selected encoder and desktop Display settings. The virtual
   output may be disabled, or FFmpeg may lack the selected encoder.

For a foreground debug run, first stop the service with
`systemctl --user stop blent` (or stop an unmanaged daemon with
`blent stop`), then run `RUST_LOG=blent=debug blent start`. This can
attach an EVDI display; it is not a read-only diagnostic. Do not deliberately
repeat a session-crashing attachment on your working desktop; see below.

## Touch or pen land on the wrong screen

KDE Wayland mapping uses KWin D-Bus. X11 writes use native XInput/RandR;
install `xinput` and `xrandr` for discovery and readback. Mapping runs on attachment and mode changes, and graphics-tablet
mode targets a physical screen. KDE selection honors the enabled physical
output's current KScreen priority; the former priority bug (T299) has regression
coverage. Check mapping after mode changes. Other Wayland desktops need manual
mapping where supported. `blent doctor` gives manual-mapping guidance there;
the former unconditional KWin check (T234) is fixed.

For an X11 session, inspect `xrandr --listproviders` and `xrandr --query`.
If the EVDI provider is not linked, the general command is
`xrandr --setprovideroutputsource EVDI_PROVIDER SOURCE_PROVIDER`, replacing
both names/IDs with the appropriate providers from that output. Do not assume
the source is provider 0. Display-server/driver support is required; this
command changes the live display configuration.

Automatic KScreen placement runs only when `XDG_SESSION_TYPE=wayland` and
`XDG_CURRENT_DESKTOP` identifies KDE (including colon-separated desktop names).
X11, other Wayland desktops and unidentified sessions skip those commands and
their retry delay; configure placement through the desktop's display settings.

## Cinnamon or Xorg restarts when connecting

The [2026-09-17 incident report](reviews/2026-09-17-cinnamon-restart.md)
records an Xorg crash during an EVDI attachment. The root cause and mitigation
remain unverified (T222). A later startup-geometry fix does not establish that
the crash is fixed. Preserve the journal and Xorg/coredump evidence; further
reproduction needs an isolated session.

## The on-screen keyboard pops up or remains disabled

Blent saves KDE's keyboard setting before suppressing it while touch devices
exist, then attempts restoration. Invalid/unreadable saved state prevents a new
suppression; failed persistence also leaves the keyboard unchanged (T334).
Restoration failures are logged and the saved state is retained for retry.

If the keyboard remains disabled, inspect the log and restore your desired mode
in **System Settings → Virtual Keyboard**. Repair a damaged saved state before
retrying; simply restarting the daemon is not a guaranteed recovery.

## Wi-Fi is stuttery

The historical locked-radio test had a median close to USB, but much longer
tail delays. Your network may differ. See [benchmarks.md](benchmarks.md).
Use USB when those delays are disruptive. `blent wifi --off` does not close
the tablet's adb network listener; see [SECURITY.md](../SECURITY.md).

## A required GLIBC version is not found

The binary or one of its libraries was built against a newer glibc than the
runtime provides. The portable workflow targets glibc 2.36; local builds may
need newer versions regardless of the displayed Blent version. Use a build
matching your distribution, or rebuild with the documented portable toolchain.
Do not replace the system glibc to satisfy an application binary.

## A busy EVDI card is selected despite another free card

Automatic sessions lease free cards rather than assigning them by enumeration
order. Restarting capture prefers its previous card if still free, otherwise
it searches the current card set. Explicit helper `--card` pins remain strict.
If every card is connected or leased, automatic growth is refused under Xorg.
Use [system setup and reboot guidance](installation.md#evdi-setup-and-safe-capacity-changes)
instead of repeatedly starting capture. Outside Xorg the helper may request more
capacity; check its error and EVDI permissions if that fails. Include
card/connector state in reports without removing another application's display.

## Getting more help

Open a [fork issue](https://github.com/geraldo-netto/blent/issues) with the
build commit, `blent doctor` output and relevant log excerpt. Remove tokens,
device serials and other personal information before posting.
