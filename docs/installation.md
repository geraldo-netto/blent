# Installing Blent for Linux

Build the Linux daemon/GUI/helper and Android app from the same checkout.
As of 2026-09-17 this fork has no published releases; start with
[building from source](development.md#building-from-source). Upstream releases
do not contain the unreleased fork changes. The instructions below also cover
artifacts produced locally or available in a future [fork release](https://github.com/geraldo-netto/blent/releases).

Before attaching a display, check [current limitations](compatibility.md#current-fork-limitations).
A Cinnamon/Xorg session crash during EVDI attachment remains unresolved (T222).
The full installer and native package hooks preserve loaded EVDI devices.
They reuse sufficient capacity immediately. When Xorg is absent, they load the
module if needed and add missing capacity up to two devices. Under Xorg, or when
the process inventory fails, loading/growth is deferred to reboot (T751).
Failed provisioning also reports that setup must be checked and deferred to reboot.
Boot configuration changes take effect on the next module load.
Source/tarball reinstallation and `make setup-system` preserve an existing
`blent-evdi.conf` verbatim, including capacity provisioned through GUI setup and other module options.
Both create the two-device boot default only when that file is absent; existing
empty, invalid or externally managed configurations require manual review.
`make setup-system` and GUI setup use the same provisioning script. GUI setup
persists its configured tablet count (one to four) for the next boot; it reports
failure/reboot when live capacity cannot safely be increased, while still
configuring input permissions. The capture helper reuses free pre-created devices
under Xorg and refuses automatic hot-add when none are available. Review
`initial_device_count` in `/etc/modprobe.d/blent-evdi.conf` and reboot to provision
more devices before starting Xorg. Existing leases and displays are preserved.
This avoids Blent-triggered hot-add under a detected Xorg session; it does not
establish safety for an external display server starting concurrently or cure
unrelated driver/compositor faults.

## GPU ordering before EVDI (T700)

The full source/tarball installer, `make setup-system`, and Arch installation
hooks run `scripts/gpu-boot-order.sh` before EVDI provisioning. It creates
`/etc/modprobe.d/blent-gpu-order.conf` with a `softdep evdi pre: ...` rule for the
currently bound, loadable DRM drivers. Multiple modules are deduplicated;
NVIDIA's module is mapped to `nvidia_drm`, and `virtio_pci` to `virtio_gpu`.
EVDI, simpledrm, built-in drivers and vanished devices contribute no dependency.
The helper warns when no loadable physical GPU is found; it cannot discover an
unbound GPU or make unavailable drivers ready.

The rule is published atomically. Existing files and managed symlinks are
preserved, including empty configurations. Review the file after a GPU or driver
change. The helper does not reload modules or restart the desktop; its rule
applies to the next EVDI load. If EVDI is loaded in the initramfs, follow your
distribution's procedure to include the rule and GPU modules there and regenerate
the image. A root-filesystem rule alone does not change an existing initramfs.

To configure ordering separately from a checkout, with the intended GPU drivers
already bound:

```sh
sudo bash scripts/gpu-boot-order.sh
```

The AppImage includes this helper under `usr/share/blent/` in its extracted
AppDir. Its user installer does not run privileged system setup; an administrator
can run that helper explicitly. A `softdep` requests module ordering; it does not
guarantee GPU readiness before Xorg on every boot arrangement or when a driver
fails to load. See the [reproduction, tests and remaining native acceptance](reviews/2026-09-28-gpu-boot-order.md).

## Linux artifacts and prerequisites

The portable packaging workflow targets **Linux x86-64, glibc 2.36 or newer**.
This is an ABI baseline, not a guarantee that every distribution, GPU or
compositor works. Local builds can require a newer glibc. Runtime needs adb,
FFmpeg with the chosen encoder, compatible EVDI kernel support, libdrm, GUI
libraries and access to `/dev/uinput`. Container tests cannot validate kernel
attachment or tablet operation.

| Artifact | Installation |
| --- | --- |
| `blent-<ver>-x86_64.AppImage` | Linux x86-64/glibc 2.36+: make executable and launch; see [AppImage setup, extraction and migration](appimage-plan.md) |
| `blent-<ver>-PKGBUILD.tar.gz` | Arch family: install AUR `evdi-dkms` first, extract the recipe and run `makepkg -si` |
| `blent-<ver>-linux-x86_64.tar.gz` | Other compatible Linux setups: extract and inspect/run `./scripts/install.sh`; unsupported package managers need manual dependency installation |

Review the package transaction. AppImage replaces the Debian release asset and
bundles userspace dependencies; it does not install EVDI/DKMS, GPU drivers or
device permissions. A working EVDI kernel module is still required for an
extended display. Install the appropriate module package for your kernel.

For fast-reconnect failures, use a distribution-compatible **stock kernel module**
containing [upstream PR590](https://github.com/DisplayLink/evdi/pull/590);
[v1.15.1](https://github.com/DisplayLink/evdi/releases/tag/v1.15.1) includes it and
the import-page sizing fix from v1.14.16. Updating bundled libevdi alone does not
update the kernel driver. Distribution backports must be checked separately.
The [isolated stock comparison](reviews/artifacts/2026-10-09-evdi-stock/reproduction.json)
reproduced a missing scanout after reconnect without a modeset on Ubuntu's 1.14.15;
the same operation restored the frame on unmodified 1.15.1. This is a driver-level
result, not validation of every desktop or import boundary. T749 retains those
checks; no host module was installed or reloaded during this review.

`makepkg -s` installs repository dependencies through pacman; it does not build
AUR packages. Install [evdi-dkms](https://aur.archlinux.org/packages/evdi-dkms)
separately first; see [makepkg(8)](https://man.archlinux.org/man/makepkg.8.en).
The recipe downloads its declared source tag, so it needs that tag to exist;
use the source workflow for an unreleased checkout.

The native USB transport requires the libusb-1.0 shared runtime in addition to
FFmpeg, adb, libevdi and the GUI libraries.

Module and package availability depend on the distribution release and kernel.
Automatic dependency installation supports Debian/Ubuntu and Arch-family systems.
Other families require manual installation with their own package manager.

| Distribution family | FFmpeg/adb names used by the project | EVDI source to check |
| --- | --- | --- |
| Debian/Ubuntu | `ffmpeg`, `adb` | `evdi-dkms` for the running kernel |
| Arch | `ffmpeg`, `android-tools` | AUR `evdi-dkms` |

The installer matches complete supported `ID`/`ID_LIKE` words. Unknown families
receive manual guidance for the runtime prerequisites listed above; the installer
does not configure their repositories or install their dependencies.

Use `modinfo evdi` and, for DKMS installations, `dkms status` to inspect the
installed module. If a DKMS build fails, inspect its build log and check the
module's compatibility with the running kernel and available headers. Follow
your distribution or [EVDI upstream](https://github.com/DisplayLink/evdi)
instructions for a compatible module; one pinned version is not a universal
kernel fix. A packaged userspace library is not the kernel module.

Portable tarballs/native packages bundle libevdi v1.15.0 next to the helper
and use an `$ORIGIN` lookup. A normal source installation may instead use the
system library installed during the build; see [development.md](development.md).
The Debian-family installer installs FFmpeg/ADB and GUI libraries independently
from libevdi and DKMS. It skips a libevdi package when the required library is
already present: a bundle/system `libevdi.so.1` for prebuilt programs, or the
system `libevdi.so` linker file for a source build. Standard system library
locations and `/usr/local/lib`, `/usr/local/lib64` and their architecture
subdirectories are checked. Otherwise it requests `libevdi1` for prebuilt
programs or `libevdi-dev` for source builds. The kernel module remains a separate
requirement even when the bundle supplies its userspace library.

## Start the host

Enable **Start Blent with the desktop** in the GUI for automatic login startup,
including on Cinnamon. It enables/disables the available route and starts/stops
the current daemon.

On a systemd desktop that activates `graphical-session.target`, the unit alone
can also be enabled and started from a terminal:

```bash
systemctl --user enable --now blent
```

The full tarball/source installer enables the user service and creates a desktop
login entry when it can reach the user manager. Otherwise it creates an entry
that starts the daemon directly. It
reports the selected route and does not start the daemon during installation.
`make install` installs user files while preserving the autostart preference;
it also repairs the login entry for an already enabled service.

Both routes use `XDG_CONFIG_HOME/autostart/blent.desktop` (default
`~/.config/autostart/blent.desktop`) on desktops implementing the
[XDG autostart specification](https://specifications.freedesktop.org/autostart/latest/).
With systemd, that entry starts `blent.service`, even when the desktop does not
activate `graphical-session.target`. If the target already started the service,
systemd keeps the same daemon. Enabling the systemd route replaces any direct
fallback entry with this service entry. Use the GUI setting to disable both the
unit and its login entry; `systemctl disable` alone does not remove an XDG entry.
Without systemd, the entry runs a direct daemon without service restart
supervision. `blent start` also runs a foreground session from a terminal.
If systemctl still reports an enabled unit while its manager is unreachable,
autostart changes report an error; restore the user manager before switching
routes or disabling that unit.

Autostart does not provide kernel modules or input permissions. On another
init system, configure `evdi` and `uinput` to load at boot; do not assume it
reads `/etc/modules-load.d`. For example, OpenRC supplies a `modules` setting
in [`/etc/conf.d/modules`](https://github.com/OpenRC/openrc/blob/master/conf.d/modules);
add these modules to the existing list using your distribution's service setup.
The installer writes modprobe options and a udev rule. A device manager without
udev-compatible rules needs its own `/dev/uinput` access configuration; verify
module availability and permissions with `blent doctor` before streaming.

Both user installers share the same paths: launchers and icons go under
`XDG_DATA_HOME` (default `~/.local/share`), and the user unit goes under
`XDG_CONFIG_HOME/systemd/user` (default `~/.config/systemd/user`). Empty or
relative XDG values use those defaults, matching Blent's configuration-path
policy. Program binaries remain in `~/.local/bin`; these XDG overrides do not
change that location. Native packages use their system-wide package paths.
For a custom program directory, use `make install BIN_DIR=/absolute/path`.
The generated launcher and user service both refer to that selected directory,
including the service's helper and stop commands.

If `~/.local/bin` is not on PATH yet, use `~/.local/bin/blent` or add the
directory to your shell's PATH. Run `blent doctor` to inspect the setup.
New configurations use [automatic encoder selection](video-codecs.md#automatic-selection)
after tablet negotiation, with `libx264` fallback. Existing explicit preferences
remain unchanged; choose a supported encoder or `auto` in Linux settings.

For published artifacts, run `sha256sum -c SHA256SUMS` alongside all files
listed in the manifest. An absent file is reported as a verification failure.
See [release integrity](../SECURITY.md#release-integrity) for signing limitations.

## Android and first connection

1. Install the APK built from the same checkout (debug builds produce
   `android/app/build/outputs/apk/debug/app-debug.apk`). Android **8.1/API 27**
   is the minimum; decoding capabilities must also support the chosen stream.
2. Enable USB debugging in Developer options. On many tablets, tap *Build
   number* seven times under About to reveal that menu. Accept the computer's
   authorization prompt when connecting a data-capable USB cable.
3. With the host running, the daemon sets up adb forwarding and, by default,
   launches the app. Check desktop Display settings to enable/place the EVDI
   output if the desktop does not do so automatically. KDE Wayland has the
   most automation; see [compatibility](compatibility.md) for mapping limits.

Blent defaults to 50% brightness and a 60 Hz display-mode preference only
while its window is in use. Change these in the app's gear menu; they do not
change other apps' system display settings. Stream FPS is a separate control.

System changes and removal instructions are in [SECURITY.md](../SECURITY.md).
For failures, use [troubleshooting.md](troubleshooting.md) before repeating setup.

## Capture pipe capacity

Linux **Video → Capture pipe buffer** offers 1, 2, 4 and 8 MiB, with 1 MiB as
the default. Pipe-only edits apply without restarting the display. The GUI shows
the actual capacity and includes copyable instructions for raising the kernel's
limit. See [capture pipe configuration](pipe-buffer.md) for temporary/persistent
commands, rollback and the latency tradeoff.
