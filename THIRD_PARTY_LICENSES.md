# Third-party components
- The optional experimental host `accessory-transport` feature uses stock Rustls,
  tokio-rustls, rcgen and their crypto/parsing dependencies. New locked dependency
  notices, exact crate checksums and license declarations are retained in
  [the accessory inventory](licenses/accessory-transport/inventory.json).
  These files come from the local Cargo cache; no upstream source was modified.
  The Linux USB helper links the system's unmodified libusb-1.0 shared library;
  it does not bundle that library. Enabling this feature in a release also requires
  its native-helper packaging and runtime acceptance tracked under T846.
- Android APKs carry project notices, the Apache-2.0 runtime dependency license,
  the MPL-2.0 public-suffix data license and its upstream source notice in
  `assets/licenses/`. The reviewed dependency coordinates and artifact hashes
  are in `runtime-dependencies.json` in that APK directory.
  APKs are ZIP archives; these files are readable without installing the app.
  Review the inventory and applicable notices whenever Android dependencies change.
- libevdi (host/evdi/evdi_lib.h, and libevdi.so.1 shipped next to the helper
  in release packages) is copyright DisplayLink (UK) Ltd. and licensed under
  the GNU Lesser General Public License v2.1 or later. It is used unmodified as
  a separate shared library. Source for the bundled v1.15.0 library:
  https://github.com/DisplayLink/evdi/tree/v1.15.0/library
  License text: [LGPL-2.1](licenses/libevdi-LGPL-2.1.txt).
- The evdi kernel module is not part of this project; it is installed from
  your distribution or built from the same upstream repository.
- The AppImage builds unmodified FFmpeg 6.1.6 from upstream source with GPL
  components enabled, including libx264. It also bundles libvpx/libaom and uses
  MIT-licensed NV codec headers. The installed AppImage's
  `usr/share/doc/blent/bundled` directory contains the exact dependency
  notices and manifests. Its required matching `AppImage-sources.tar.gz`
  asset contains the FFmpeg/header archives, build configuration and exact
  Debian sources for bundled external codec libraries. See
  [AppImage dependencies and source distribution](docs/appimage-plan.md).

- The isolated audio helper uses `pipewire`, `pipewire-sys`, `libspa` and
  `libspa-sys` from pipewire-rs 0.8, copyright The pipewire-rs Contributors,
  under the [MIT license](licenses/pipewire-rs-MIT.txt). The PipeWire shared
  library and SPA modules are supplied by the host system, including when
  running the AppImage.
