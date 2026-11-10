#!/bin/sh
# Installs VLCCraft for the current user. No root, no system folders.
#
#   ./install.sh              install into ~/.local/bin and add a desktop entry
#   PREFIX=~/bin ./install.sh install the program somewhere else
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${PREFIX:-$HOME/.local/bin}
data=${XDG_DATA_HOME:-$HOME/.local/share}

if [ ! -f "$here/vlccraft" ]; then
    echo "install.sh must sit next to the vlccraft program" >&2
    exit 1
fi

mkdir -p "$prefix" "$data/applications" "$data/icons/hicolor/256x256/apps"
install -m 755 "$here/vlccraft" "$prefix/vlccraft"
echo "program:  $prefix/vlccraft"

if [ -f "$here/vlccraft.png" ]; then
    install -m 644 "$here/vlccraft.png" "$data/icons/hicolor/256x256/apps/vlccraft.png"
    echo "icon:     $data/icons/hicolor/256x256/apps/vlccraft.png"
fi

# The entry is written here rather than copied so a PREFIX outside the home
# folder still produces a working launcher.
cat > "$data/applications/vlccraft.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.5
Name=VLCCraft
GenericName=Media Player
Comment=A calm skin over the player already installed on this machine
Exec="$prefix/vlccraft" %F
TryExec=$prefix/vlccraft
Icon=vlccraft
Terminal=false
Categories=AudioVideo;Player;Video;Audio;
MimeType=video/mp4;video/x-matroska;video/webm;video/x-msvideo;video/quicktime;video/mpeg;video/ogg;video/x-flv;video/x-ms-wmv;video/mp2t;video/3gpp;video/x-m4v;application/vnd.apple.mpegurl;application/x-mpegurl;audio/mpeg;audio/flac;audio/x-flac;audio/ogg;audio/opus;audio/x-wav;audio/mp4;audio/x-m4a;audio/aac;audio/x-ms-wma;audio/x-aiff;
StartupNotify=true
StartupWMClass=vlccraft
Keywords=video;audio;movie;music;player;media;subtitle;
EOF

command -v update-desktop-database >/dev/null 2>&1 &&
    update-desktop-database "$data/applications" >/dev/null 2>&1 || true

echo "entry:    $data/applications/vlccraft.desktop"
echo
echo "VLCCraft needs VLC installed — it uses VLC's engine and bundles none of it."
case ":$PATH:" in
    *":$prefix:"*) echo "Start it from your applications menu, or type: vlccraft" ;;
    *) echo "Add $prefix to PATH, then start VLCCraft from your applications menu." ;;
esac
