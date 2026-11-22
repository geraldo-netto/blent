#!/usr/bin/env bash
# Exercise the real Arch package() recipe using the portable build's ELF files.
set -euo pipefail
[[ -f /.dockerenv ]]
VERSION=$(sed -n 's/^VERSION = //p' /source/Makefile)
pacman -Syu --noconfirm --needed base-devel
useradd --create-home packager
BUILD=$(mktemp -d)
chmod 755 "$BUILD"
tar -xf "/artifacts/blent-$VERSION-PKGBUILD.tar.gz" -C "$BUILD"
EVDI_COMMIT=$(sed -n 's/^_evdi=//p' "$BUILD/PKGBUILD")
mkdir -p "$BUILD/src"
tar -xf "/artifacts/blent-$VERSION-linux-x86_64.tar.gz" -C "$BUILD/src"
D="$BUILD/src/blent-$VERSION"
mkdir -p "$D/target/release" "$D/host/evdi" "$BUILD/src/evdi-$EVDI_COMMIT/library"
cp "$D/bin/blent" "$D/bin/blent-gui" "$D/bin/blent-audio" "$D/bin/blent-usb-broker" "$D/bin/blent-usb-io" "$D/target/release/"
cp "$D/bin/evdi_helper" "$D/host/evdi/"
cp "$D/bin/libevdi.so.1.15.0" "$BUILD/src/evdi-$EVDI_COMMIT/library/"
# Include the original distribution-docs manifest and copier for package().
cp -a /source/scripts /source/packaging "$D/"
chown -R packager:packager "$BUILD"
runuser -u packager -- bash -c '
    set -euo pipefail
    cd "$1"
    source PKGBUILD
    srcdir="$PWD/src"; pkgdir="$PWD/pkg/blent"
    mkdir -p "$pkgdir"
    cd "$srcdir"
    package
    cd "$1"
    makepkg --repackage --nodeps --nosign --force
' blent-arch "$BUILD"
cp "$BUILD"/blent-[0-9]*.pkg.tar.zst /artifacts/
