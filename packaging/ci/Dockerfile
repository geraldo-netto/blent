# Pinned Rust on Debian 12 keeps the release ABI at glibc 2.36.
FROM rust:1.100-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea
RUN sed -i "s/^Types: deb$/Types: deb deb-src/" /etc/apt/sources.list.d/debian.sources \
    && apt-get update && apt-get install -y --no-install-recommends \
    build-essential autoconf automake libtool meson ninja-build patch pkg-config git ca-certificates python3 python3-xlib python3-tk \
    libpipewire-0.3-dev libusb-1.0-0-dev libclang-dev pipewire-bin libdrm-dev libxkbcommon-dev libxkbcommon-x11-0 libegl1 libwayland-dev libxcb-render0-dev \
    libxcb-shape0-dev libxcb-xfixes0-dev libssl-dev \
    ffmpeg adb binutils patchelf squashfs-tools file \
    nasm libx264-dev libx265-dev libvpx-dev libaom-dev libva-dev zlib1g-dev \
    xvfb xauth x11-utils xinput dbus-x11 at-spi2-core libglib2.0-bin libgl1-mesa-dri
# Source indexes are required for exact corresponding-source downloads.

RUN git clone --depth 1 --branch v1.15.0 https://github.com/DisplayLink/evdi /opt/evdi \
    && make -C /opt/evdi/library \
    && make -C /opt/evdi/library install PREFIX=/usr/local \
    && ldconfig
WORKDIR /work
