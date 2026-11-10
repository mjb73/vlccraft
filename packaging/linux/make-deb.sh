#!/bin/sh
# Builds a .deb from an already-built binary.
#   make-deb.sh <binary> <version> <arch> <outdir>
# <arch> is the Debian name: amd64, arm64.
set -eu

bin=$1
version=$2
arch=$3
out=$4

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/usr/bin" \
         "$work/usr/share/applications" \
         "$work/usr/share/icons/hicolor/256x256/apps" \
         "$work/usr/share/doc/vlccraft" \
         "$work/DEBIAN"

install -m 755 "$bin" "$work/usr/bin/vlccraft"
install -m 644 "$here/vlccraft.desktop" "$work/usr/share/applications/vlccraft.desktop"
install -m 644 "$root/assets/vlccraft-256.png" "$work/usr/share/icons/hicolor/256x256/apps/vlccraft.png"
install -m 644 "$root/README.md" "$work/usr/share/doc/vlccraft/README.md"
sed 's/^# .*$//' "$root/LICENSE" > "$work/usr/share/doc/vlccraft/copyright"

size=$(du -sk "$work" | cut -f1)
cat > "$work/DEBIAN/control" <<EOF
Package: vlccraft
Version: $version
Section: video
Priority: optional
Architecture: $arch
Installed-Size: $size
Depends: libc6 (>= 2.36), libxkbcommon0, libwayland-client0, libgl1, libegl1
Recommends: vlc
Maintainer: mjb73 <mjb73@users.noreply.github.com>
Description: A calm skin over the VLC you already have
 VLCCraft draws its own interface and loads the libvlc that the installed
 player provides, so VLC keeps updating normally and nothing is bundled.
EOF

mkdir -p "$out"
pkg="$out/vlccraft_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$work" "$pkg" >/dev/null
echo "$pkg"
