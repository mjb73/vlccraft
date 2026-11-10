#!/bin/sh
# VLCCraft installer for Linux — one file, and it works out where the download is.
#
# Three ways to use it, and none of them needs you to be in a particular folder:
#
#   sh install.sh                  finds the newest download (beside this script,
#                                  in ~/Downloads, ~/Desktop, ~/Documents or the
#                                  current folder); if none is there, fetches the
#                                  newest release itself, checks its fingerprint
#                                  and installs it
#   sh install.sh FILE.tar.gz      install from a download you already have
#   ./install.sh                   inside an unpacked tarball, the classic way
#   curl … | sh                    straight down a pipe: the newest release
#
# One download is used when you ran the installer that came with it; otherwise
# the newest release is fetched. So a tarball left in Downloads from last month
# never wins over the version the one-line command promised.
#
# Other uses:
#   PREFIX=~/bin sh install.sh     put the program somewhere else
#   sh install.sh --uninstall      remove the program, menu entry and icon
#   sh install.sh --purge          the same, and also forget library, presets,
#                                  resume positions and settings
#
# No root, no system folders, and nothing of VLC is touched. VLCCraft needs VLC
# for playback: when it is missing, this script says so in plain words and offers
# to install it with the distribution's own package manager. Set
# VLCCRAFT_SKIP_VLC_PROMPT=1 to install without that question (scripts, CI).
set -eu

REPO=mjb73/vlccraft
APP=vlccraft

prefix=${PREFIX:-$HOME/.local/bin}
data=${XDG_DATA_HOME:-$HOME/.local/share}
appdata=$data/vlccraft

show_help() {
    # The header of this file is the help, so the two cannot disagree. Piped
    # through `curl ... | sh` there is no file to read, so say where to look.
    if [ -r "$0" ] && head -1 "$0" 2>/dev/null | grep -q "^#!"; then
        awk 'NR>1 { if ($0 ~ /^set -eu/) exit; sub(/^# ?/, ""); print }' "$0"
    else
        printf '%s\n' "VLCCraft installer. Usage: sh install.sh [FILE.tar.gz]" \
            "  --uninstall  remove the program, menu entry and icon" \
            "  --purge      the same, and also forget library and settings" \
            "More: https://github.com/$REPO#install"
    fi
}

say() { printf '%s\n' "$*"; }
warn() { printf '%s\n' "$*" >&2; }
die() { printf '\nvlccraft installer: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

[ -n "${HOME:-}" ] || die "HOME is not set, so there is no home folder to install into."

mode=install
want=
for arg in "$@"; do
    case "$arg" in
        -h|--help)
            show_help
            exit 0
            ;;
        --uninstall) mode=uninstall ;;
        --purge) mode=purge ;;
        -*) die "unknown option: $arg (try --help)" ;;
        *) want=$arg ;;
    esac
done

# ---------------------------------------------------------------------------
# Removing
# ---------------------------------------------------------------------------

if [ "$mode" != install ]; then
    removed=0
    for f in "$prefix/$APP" \
             "$data/applications/$APP.desktop" \
             "$data/icons/hicolor/256x256/apps/$APP.png"; do
        if [ -e "$f" ]; then
            rm -f -- "$f" && say "removed  $f" && removed=1
        fi
    done
    have update-desktop-database && update-desktop-database "$data/applications" >/dev/null 2>&1 || true
    if [ "$mode" = purge ]; then
        if [ -d "$appdata" ]; then
            rm -rf -- "$appdata" && say "removed  $appdata (library, presets, settings)"
            removed=1
        fi
    elif [ -d "$appdata" ]; then
        say "kept     $appdata (your library, presets and settings)"
        say "         run with --purge to remove those too"
    fi
    [ "$removed" = 1 ] || say "VLCCraft did not look installed here."
    say ""
    say "Done."
    exit 0
fi

# ---------------------------------------------------------------------------
# Finding the program
# ---------------------------------------------------------------------------

here=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd || pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/vlccraft-install.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

# Newest match first. A tarball, an extracted folder made by a file manager, or
# a bare program: all three are things a person actually ends up with.
newest_in() {
    # $1 = directory, $2 = glob. Prints the newest path, or nothing.
    [ -d "$1" ] || return 0
    ls -1t -- "$1"/$2 2>/dev/null | head -1
}

look_around() {
    for d in "$here" "$PWD" "$HOME/Downloads" "$HOME/Desktop" "$HOME/Documents" "$HOME"; do
        [ -n "$d" ] || continue
        p=$(newest_in "$d" '*vlccraft*.tar.gz')
        [ -n "$p" ] && { printf 'tarball %s\n' "$p"; return 0; }
    done
    for d in "$here" "$PWD" "$HOME/Downloads" "$HOME/Desktop" "$HOME/Documents"; do
        [ -n "$d" ] || continue
        for dir in "$d"/*vlccraft*-linux*; do
            if [ -x "$dir/$APP" ]; then
                printf 'folder %s\n' "$dir"
                return 0
            fi
        done
    done
    return 0
}

release_version() {
    # The tag the release page redirects to. No API key, no rate limit.
    if have curl; then
        page=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
            "https://github.com/$REPO/releases/latest" 2>/dev/null || true)
        case "$page" in
            */tag/v?*) printf '%s\n' "${page##*/tag/v}" ; return 0 ;;
        esac
    fi
    if have wget; then
        api=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null || true)
        v=$(printf '%s\n' "$api" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\([^"]*\)".*/\1/p' | head -1)
        [ -n "$v" ] && { printf '%s\n' "$v" ; return 0 ; }
    fi
    return 1
}

fetch_text() {
    # $1 = url. Prints the body, or nothing when it cannot be had.
    if have curl; then
        curl -fsSL --retry 2 --connect-timeout 20 "$1" 2>/dev/null
    elif have wget; then
        wget -qO- --timeout=20 "$1" 2>/dev/null
    fi
}

release_asset() {
    # The Linux tarball's real name, asked of the release that holds it. The
    # name carries the version, so it is the first thing to be wrong when a
    # release renames or renumbers its files — asking beats guessing.
    body=$(fetch_text "https://api.github.com/repos/$REPO/releases/latest" || true)
    [ -n "$body" ] || return 0
    printf '%s\n' "$body" |
        tr ',' '\n' |
        sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
        grep -E '(x86_64|amd64).*linux.*\.tar\.gz$|linux.*(x86_64|amd64).*\.tar\.gz$' |
        head -1
}

fetch() {
    # $1 = url, $2 = destination
    if have curl; then
        curl -fsSL --retry 2 --connect-timeout 20 -o "$2" "$1"
    elif have wget; then
        wget -q -O "$2" "$1"
    else
        die "this machine has neither curl nor wget, so the download cannot be fetched.
Download the Linux tarball from https://github.com/$REPO/releases and run:
  sh install.sh ~/Downloads/vlccraft-VERSION-x86_64-linux.tar.gz"
    fi
}

arch=$(uname -m 2>/dev/null || echo unknown)
case "$arch" in
    x86_64|amd64) ;;
    *)
        # Say so before downloading something that cannot run.
        die "there is no Linux download for this kind of processor ($arch) yet —
only 64-bit Intel and AMD machines have one. Everything is at
https://github.com/$REPO/releases"
        ;;
esac

found=
version=
srcbin=
srcdir=

# Where did this script come from? Down a pipe (`curl … | sh`), or out of a
# temporary folder (the `curl -o /tmp/…` line in the README), it came from the
# internet: the person asked for the newest release, so a tarball lying in
# Downloads must not win over it.
from_internet=
temp_root=${TMPDIR:-/tmp}
temp_root=${temp_root%/}
case "${0##*/}" in
    sh|dash|bash|zsh|ksh|-sh) from_internet=1 ;;
esac
if [ -z "$from_internet" ]; then
    case "$here" in
        /tmp/*|/var/tmp/*|"$temp_root"|"$temp_root"/*) from_internet=1 ;;
    esac
fi

if [ -n "$want" ]; then
    [ -e "$want" ] || die "there is no file at $want — check the name and try again"
    case "$want" in
        *.tar.gz|*.tgz) found="tarball $want" ;;
        *)
            if [ -x "$want" ]; then
                found="program $want"
            elif [ -d "$want" ] && [ -x "$want/$APP" ]; then
                found="folder $want"
            else
                die "that is not something this installer can use: $want"
            fi
            ;;
    esac
elif [ -x "$here/$APP" ]; then
    found="program $here/$APP"
elif [ -n "$from_internet" ]; then
    # A lone installer that arrived from the internet: nothing beside it, and
    # whatever is lying in Downloads is older than what was just asked for.
    found=
else
    found=$(look_around)
fi

case "$found" in
    program\ *)
        srcbin=${found#program }
        srcdir=$(CDPATH= cd -- "$(dirname -- "$srcbin")" && pwd || dirname -- "$srcbin")
        ;;
    folder\ *)
        srcdir=${found#folder }
        ;;
    tarball\ *)
        tb=${found#tarball }
        say "Unpacking $(basename "$tb")…"
        mkdir -p "$work/x"
        tar -xzf "$tb" -C "$work/x" || die "that download looks incomplete — download it again from
https://github.com/$REPO/releases"
        srcdir=
        for dir in "$work/x"/*/; do
            if [ -x "$dir$APP" ]; then srcdir=$dir; break; fi
        done
        [ -n "$srcdir" ] || die "the tarball unpacked, but there is no program inside it.
Download it again from https://github.com/$REPO/releases"
        ;;
    *)
        # Nothing on this machine: fetch the newest release.
        if [ -n "$from_internet" ]; then
            say "Fetching the newest VLCCraft…"
        else
            say "No download found here, so fetching the newest VLCCraft…"
        fi
        version=$(release_version) || die "could not reach GitHub to find the newest version.
Check the connection, or download the Linux tarball from
https://github.com/$REPO/releases and run:
  sh install.sh ~/Downloads/vlccraft-VERSION-x86_64-linux.tar.gz"
        asset="vlccraft-$version-x86_64-linux.tar.gz"
        base="https://github.com/$REPO/releases/download/v$version"
        say "Downloading VLCCraft $version…"
        if ! fetch "$base/$asset" "$work/$asset" 2>/dev/null; then
            # The file name carries the version, so a renamed or renumbered
            # download is the usual reason an address that looks right holds
            # nothing. Ask the release which file it actually has.
            say "That address held nothing, so asking the release what it carries…"
            asset=$(release_asset || true)
            if [ -z "$asset" ]; then
                die "could not find a Linux download to fetch.
Check the connection first; if the network is fine, the release page is
https://github.com/$REPO/releases"
            fi
            say "Downloading $asset…"
            fetch "$base/$asset" "$work/$asset" || die "the download failed.
Try the release page in a browser: https://github.com/$REPO/releases"
        fi
        if fetch "$base/$asset.sha256" "$work/$asset.sha256" 2>/dev/null; then
            want_sum=$(sed -n '1s/[[:space:]].*//p' "$work/$asset.sha256")
            if have sha256sum; then
                got_sum=$(sha256sum "$work/$asset" | sed -n '1s/[[:space:]].*//p')
            elif have shasum; then
                got_sum=$(shasum -a 256 "$work/$asset" | sed -n '1s/[[:space:]].*//p')
            else
                got_sum=
            fi
            if [ -n "$got_sum" ] && [ "$got_sum" != "$want_sum" ]; then
                die "the download does not match its published fingerprint, so it was not installed.
Try again, or download it in a browser from https://github.com/$REPO/releases"
            fi
            [ -n "$got_sum" ] && say "Fingerprint matches."
        fi
        say "Unpacking…"
        mkdir -p "$work/x"
        tar -xzf "$work/$asset" -C "$work/x" || die "the download looks incomplete — try again"
        srcdir=
        for dir in "$work/x"/*/; do
            if [ -x "$dir$APP" ]; then srcdir=$dir; break; fi
        done
        [ -n "$srcdir" ] || die "the download unpacked, but there is no program inside it"
        ;;
esac

# ---------------------------------------------------------------------------
# Installing
# ---------------------------------------------------------------------------

mkdir -p "$prefix" "$data/applications" "$data/icons/hicolor/256x256/apps"

# Copy beside the target and rename, so replacing a copy that is running right
# now works: the old program keeps its own file until it is closed.
tmp="$prefix/.$APP.new.$$"
cp -- "${srcbin:-$srcdir/$APP}" "$tmp"
chmod 755 "$tmp"
mv -f -- "$tmp" "$prefix/$APP"
chmod 755 "$prefix/$APP"
say "Program   $prefix/$APP"

if [ -f "$srcdir/vlccraft.png" ]; then
    cp -- "$srcdir/vlccraft.png" "$data/icons/hicolor/256x256/apps/$APP.png"
    chmod 644 "$data/icons/hicolor/256x256/apps/$APP.png" "$data/icons/hicolor/256x256/apps/$APP.png"
    say "Icon      $data/icons/hicolor/256x256/apps/$APP.png"
fi

# Written here rather than copied, so a PREFIX outside the home folder still
# produces a working launcher.
cat > "$data/applications/$APP.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.5
Name=VLCCraft
GenericName=Media Player
Comment=A calm skin over the player already installed on this machine
Exec="$prefix/$APP" %F
TryExec=$prefix/$APP
Icon=$APP
Terminal=false
Categories=AudioVideo;Player;Video;Audio;
MimeType=video/mp4;video/x-matroska;video/webm;video/x-msvideo;video/quicktime;video/mpeg;video/ogg;video/x-flv;video/x-ms-wmv;video/mp2t;video/3gpp;video/x-m4v;application/vnd.apple.mpegurl;application/x-mpegurl;audio/mpeg;audio/flac;audio/x-flac;audio/ogg;audio/opus;audio/x-wav;audio/mp4;audio/x-m4a;audio/aac;audio/x-ms-wma;audio/x-aiff;
StartupNotify=true
StartupWMClass=vlccraft
Keywords=video;audio;movie;music;player;media;subtitle;
EOF

have update-desktop-database && update-desktop-database "$data/applications" >/dev/null 2>&1 || true
say "Menu      $data/applications/$APP.desktop"

# Does the program this machine now holds actually start here? A download built
# for a newer system than this one fails exactly here, and that is worth knowing
# now rather than on the first double-click.
installed_version=
if installed_version=$("$prefix/$APP" --version 2>/dev/null); then
    say ""
    say "Installed $installed_version."
else
    die "the program was copied, but it will not start on this system.
This usually means the system is too old for the download. The release page has
a Debian/Ubuntu package and source instructions:
https://github.com/$REPO/releases"
fi

# ---------------------------------------------------------------------------
# VLC
#
# The player is a front end; the engine is VLC, and none of it is bundled. If
# VLC is missing, ask before touching the system, offer the command this
# distribution actually uses, and never fail the install when the answer is no.
# ---------------------------------------------------------------------------

vlc_present() {
    "$prefix/$APP" --vlc-status >/dev/null 2>&1 && return 0
    command -v vlc >/dev/null 2>&1 && return 0
    command -v ldconfig >/dev/null 2>&1 &&
        ldconfig -p 2>/dev/null | grep -q 'libvlc\.so' && return 0
    return 1
}

vlc_command() {
    [ -r /etc/os-release ] || return 0
    # shellcheck disable=SC1091
    . /etc/os-release 2>/dev/null || true
    case " ${ID:-} ${ID_LIKE:-} " in
        *arch*|*manjaro*) echo "sudo pacman -S vlc" ;;
        *fedora*|*rhel*|*centos*) echo "sudo dnf install vlc" ;;
        *suse*) echo "sudo zypper install vlc" ;;
        *alpine*) echo "sudo apk add vlc" ;;
        *debian*|*ubuntu*|*mint*|*pop*|*elementary*) echo "sudo apt install vlc" ;;
    esac
}

if vlc_present; then
    say "VLC       found — playback is ready."
else
    say ""
    say "VLC is not installed yet."
    say "VLCCraft needs VLC: VLC is what decodes and plays; VLCCraft draws the"
    say "interface and bundles none of it. Download: https://www.videolan.org/vlc/"
    cmd=$(vlc_command)
    [ -n "$cmd" ] && say "Or run:   $cmd"
    say ""
    # A piped install has no terminal to answer with — stdin is the script
    # itself — so the question is only asked when there is someone there.
    if [ "${VLCCRAFT_SKIP_VLC_PROMPT:-}" = "1" ] || [ ! -t 0 ]; then
        say "Install VLC when you can, then start VLCCraft again."
    else
        printf "Install VLC now? [Y/n] "
        read -r answer || answer=
        case "$answer" in
            n|N|no|No|NO)
                say "Skipped. Playback begins once VLC is installed."
                ;;
            *)
                if [ -n "$cmd" ]; then
                    say "Running: $cmd"
                    # Word-split on purpose: the command carries its own arguments.
                    # shellcheck disable=SC2086
                    if $cmd; then
                        if vlc_present; then
                            say "VLC       installed — playback is ready."
                        else
                            say "VLC was installed but its engine is not visible yet."
                            say "Start VLCCraft; it will look again."
                        fi
                    else
                        say "That did not finish. Install VLC from https://www.videolan.org/vlc/"
                        say "and start VLCCraft again."
                    fi
                else
                    say "No package manager was recognised."
                    say "Install VLC from https://www.videolan.org/vlc/ and start VLCCraft again."
                fi
                ;;
        esac
    fi
fi

# ---------------------------------------------------------------------------
# How to start it
# ---------------------------------------------------------------------------

say ""
case ":${PATH:-}:" in
    *":$prefix:"*)
        say "Start VLCCraft from your applications menu, or type: vlccraft"
        ;;
    *)
        say "Start VLCCraft from your applications menu."
        say "To also start it by typing vlccraft, add this folder to your PATH:"
        case "${SHELL:-}" in
            *fish*) say "  fish_add_path $prefix" ;;
            *)      say "  for bash or zsh, add to ~/.profile:  export PATH=\"$prefix:\$PATH\"" ;;
        esac
        ;;
esac
