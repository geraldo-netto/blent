#!/usr/bin/env bash
# Build the AppImage (with sources) from an already-built portable dist tree
# (scripts/build-release.sh). Runs inside the Debian 12 build container,
# with AppImage tooling, Debian source repositories.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="$(sed -n 's/^VERSION = //p' Makefile)"
D="dist/blent-$VERSION"
[ -x "$D/bin/blent" ] || { echo "run scripts/build-release.sh first"; exit 1; }

# Remove prior outputs and success marker before entering the container.
rm -f "dist/blent-$VERSION-x86_64.AppImage" "dist/blent-$VERSION-AppImage-sources.tar.gz" \
      "dist/blent-$VERSION-PKGBUILD.tar.gz" dist/.packages-ok

distrobox enter "${BLENT_BUILD_CONTAINER:-blent-build}" -- bash -lc '
  set -euo pipefail
  cd "$1"
  V="$2"; D="$3"

  python3 packaging/appimage/build.py --bundle "$D" --output dist --version "$V" \
    --evdi-source "${BLENT_EVDI_SOURCE:-target-deb12/evdi-src}"

  touch dist/.packages-ok
' blent-packages "$PWD" "$VERSION" "$D"
[ -f dist/.packages-ok ] || { echo "!! package build inside container failed"; exit 1; }
rm -f dist/.packages-ok
[ -s "dist/blent-$VERSION-x86_64.AppImage" ] && [ -s "dist/blent-$VERSION-AppImage-sources.tar.gz" ] \
  || { echo "!! package outputs missing"; exit 1; }

# Arch users get the PKGBUILD as a release file too; makepkg needs the
# install script next to it, hence a small archive rather than a bare file.
# pkgver is rewritten like the other package versions so the archive can never
# point at an older tag than the release it ships with.
ARCH_TMP=$(mktemp -d); trap 'rm -rf "$ARCH_TMP"' EXIT
# The published tag must exist first. Bound temporary source storage to 128 MiB;
# the EXIT trap owns cleanup after both successful and failed downloads.
( ulimit -f 131072
  exec curl --fail --location --proto '=https' --proto-redir '=https' \
    --max-filesize 134217728 --output "$ARCH_TMP/source.tar.gz" \
    "https://github.com/geraldo-netto/blent/archive/refs/tags/v$VERSION.tar.gz"
)
SOURCE_SHA256=$(sha256sum "$ARCH_TMP/source.tar.gz" | cut -d ' ' -f 1)
sed -e "s/^pkgver=.*/pkgver=$VERSION/" -e "s/@BLENT_SOURCE_SHA256@/$SOURCE_SHA256/" \
  packaging/arch/PKGBUILD > "$ARCH_TMP/PKGBUILD"
cp packaging/arch/blent.install "$ARCH_TMP/"
tar -C "$ARCH_TMP" -czf "dist/blent-$VERSION-PKGBUILD.tar.gz" PKGBUILD blent.install

ls -la dist/*.AppImage dist/*AppImage-sources.tar.gz dist/*PKGBUILD*
