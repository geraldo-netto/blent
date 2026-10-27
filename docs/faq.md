# FAQ

**Is Blent an open-source SuperDisplay alternative for Linux?**
Yes. It creates an extra Linux display on an Android tablet over USB.
See [compatibility limits](compatibility.md) before choosing hardware.
SuperDisplay supports Windows hosts and Android clients; its
[official FAQ](https://superdisplay.app/help/) says no Linux or macOS port is planned
(checked 2026-09-17).

**Can an Android tablet be used as a real second monitor on Linux?**
Yes. Blent creates a virtual display through the EVDI kernel module, so the
tablet shows up in your display settings like any other monitor — windows can
be moved onto it, it has its own resolution and position, and it is not a
mirror.

**Does Blent work over a normal USB cable?**
Yes, over a normal data-capable USB cable (charge-only cables carry no data).
On Linux, paired direct USB does not require USB debugging: open
**Connections → USB connection** on the desktop, find and connect the tablet,
then compare and approve the matching fingerprints. This route remains a preview;
discovery and pairing do not establish native acceptance or grant camera/audio
consent. See the [setup instructions](../README.md#quick-install).
The separate ADB-over-USB route requires `adb`, USB debugging and authorization
of the computer on the tablet. Neither route needs USB tethering.

**Does Blent require USB tethering or Wi-Fi?**
No. Neither USB route requires tethering or Wi-Fi. USB debugging is required
only for the ADB route. ADB over Wi-Fi is an optional fallback, with more long
delays in the historical test; see [benchmarks](benchmarks.md).

**Can I adjust the tablet's brightness and refresh rate?**
Open Blent's gear menu. Brightness starts at 50% and the display refresh-rate preference at
60 Hz. Both controls apply immediately and remember your choices, including in
graphics-tablet mode. They affect only Blent; other apps keep the tablet's
normal settings. Refresh-rate choices use the current display resolution,
with the closest supported rate as a fallback. Choose **System default** to
let Android select the refresh rate. Android may override a requested mode.
These controls are separate from the stream's frame rate and bitrate.

**Can I use it without the cable?**
Yes, through the ADB route. Install `adb`, enable USB debugging and authorize
the computer first. Run `blent wifi` once with the cable plugged in: it puts
the tablet's adb on the network, remembers the address, and from then on the
daemon reconnects on its own whenever the cable is out. The tablet goes back
to USB-only when it reboots, so that one command has to be repeated after a
tablet restart. `blent wifi --off` forgets the address and disconnects; it
does not close the tablet's adb TCP listener. Host video/input ports stay on
loopback, but `blent wifi` opens tablet port 5555. Use a trusted network;
see [SECURITY.md](../SECURITY.md#connections-and-trust-boundaries) for returning
adbd to USB mode.

**Does Blent support Samsung S Pen pressure and tilt?**
The Linux implementation forwards pressure, tilt, eraser and button events.
Automated tests cover tilt units and hover/button transitions. Historical upstream
reports describe drawing with S Pen, but they do not certify this fork on your
hardware. Further physical stylus work is declined; existing code and tests remain.
See [compatibility](compatibility.md#current-fork-limitations).

**Does Blent extend the desktop or only mirror the screen?**
It extends. There is also a "graphics tablet" mode in which nothing is
streamed and the pen drives your existing screen. Its connection overlay waits
for the host's authenticated control greeting; opening a socket alone is insufficient.

**Does Blent work on Bazzite and KDE Wayland?**
That is the historical upstream reference setup. KDE on Wayland gets automatic output placement,
input mapping and on-screen-keyboard suppression. X11 desktops get automatic
input mapping when `xinput` and `xrandr` are installed; place outputs through
desktop display settings. Other Wayland desktops depend on their compositor
facilities and manual mapping. See [current limitations](compatibility.md#current-fork-limitations).

**Does Blent require a dummy HDMI plug?**
No. The virtual display is created in software by EVDI.

**What Android versions are supported?**
Android 8.1/API 27 and newer. The selected codec, profile, resolution and
frame rate must be supported by the device decoder; the Android version
alone does not guarantee compatibility.

**Is screen or input data sent to the cloud?**
Blent does not upload screen or input data to a cloud service. It sends them
to the connected peer over your selected USB or ADB network route. The only automatic internet request is an optional check of the latest
release tag on GitHub, off with `check_updates = false` on the desktop and a
switch in the app's settings sheet. See
[SECURITY.md](../SECURITY.md).

**How is Blent different from Weylus?**
[Weylus](https://github.com/H-M-H/Weylus#readme) uses the tablet's browser and
captures a host screen or window; extension requires a separately configured
output. Its documented Android USB setup uses `adb reverse`. Blent creates
an EVDI output and uses a native Android app. Both offer Linux pen input.

**How is it different from Sunshine/Moonlight?**
[Sunshine](https://docs.lizardbyte.dev/projects/sunshine/latest/) streams a host
display to a Moonlight client over a network. How an extra output is provided
depends on the host setup. Blent provisions its output through EVDI and
offers paired direct USB or ADB over USB for video and input. This comparison
does not imply that Sunshine/Moonlight lacks stylus support.

**What latency should I expect?**
Historical upstream results on the reference hardware: 18–22 ms median with H.264, 15–18 ms with
HEVC, over USB, measured from encoded packet readiness to receipt of the
render acknowledgement. Capture, encoding and packetizer assembly are excluded;
see [measurement boundaries](benchmarks.md#how-latency-is-measured).

**Can I use two tablets?**
The configuration supports 1–4 slots. Historical validation used one real
tablet plus a simulated client. The current allocator leases a free EVDI card
and skips busy cards; automated tests cover independent slots and fallback.
Reports with multiple physical tablets are welcome.

**How do I uninstall Blent completely?**
Instructions for package and source installs, custom paths and residual
system state are in
[SECURITY.md](../SECURITY.md#how-to-uninstall-completely).
