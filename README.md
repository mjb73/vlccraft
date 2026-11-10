# VLCCraft

[![CI](https://github.com/mjb73/vlccraft/actions/workflows/ci.yml/badge.svg)](https://github.com/mjb73/vlccraft/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/mjb73/vlccraft)](https://github.com/mjb73/vlccraft/releases)

A calm skin over the VLC you already have. Written in Rust with egui; runs on
Linux, macOS and Windows.

VLCCraft is not another media engine and not a fork of VLC. It loads the
`libvlc` that the installed player already provides, draws its own interface,
and never touches VLC's files, its plugins folder or its launcher — so VLC keeps
updating normally and nothing is duplicated.

## What it does differently

The interface follows a functional review of VLC, which found that the features
people want exist but are hard to find. The result is a player with fewer
screens and clearer words.

- **A queue and playlists are two things.** Dropping files while something plays
  adds them to the queue without interrupting; a playlist is a named thing you
  keep. The buttons say "Play next", "Add to queue" and "Add to playlist".
- **One thin control bar.** Play sits left of centre with previous and next
  around it, the seek bar runs along the bottom and is thick enough to grab,
  and full screen adds a top band with the title and a bottom band with the
  controls, each revealing near its edge and hiding again after 1.5 s.
- **Tracks are one click away.** A subtitle button lists the tracks for this
  file; the audio button lists audio tracks *and* output devices in the same
  popover; video tracks live in a tab there rather than on the bar.
- **A watching sidebar, not a settings dialog.** Four tabs — Subtitles, Audio,
  Video, Playback — change delay, styling, filters and speed while the picture
  keeps playing, with sliders and exact values side by side.
- **Presets per show.** A folder can remember subtitle delay, styling, the
  preferred subtitle and audio track, so every following episode opens the way
  the last one was set up.
- **A library that is a place.** Videos, Music, Streams and Playlists, with
  sorting, a view mode, search and "unwatched first" on the surface itself, plus
  watched state and last position on every card.
- **Resume you don't have to hunt for.** Position is kept per item and offered
  next to the controls, with a countdown that pauses while you point at it.
- **Settings that read like sentences.** One surface with categories, advanced
  options at the foot of their own category, a search field that finds a setting
  by name or synonym, and an explanation next to every switch.
- **Bookmarks on a timeline.** The bookmark key adds one immediately, and the
  sidebar shows bookmarks and chapters on the same line as the film.
- **A version you can see, and updates that arrive from the release page.**
  The number is on the About card and in Settings; a newer release is noticed
  once a day, downloaded, checked against its published fingerprint and put in
  place, with a restart to finish.

## Install

VLCCraft needs VLC: it draws the interface, and the VLC installed on the
machine does the decoding. Nothing of VLC is bundled, so playback waits until it
is present — the library, playlists and settings work regardless.

If VLC is missing, the app opens a setup card that says so in plain words, shows
the install command for this distribution (or the download page on Windows and
macOS), and has a **Check again** button that finds a freshly installed VLC
without a restart. The same card is reachable from the player, the library
notice and Settings.

`vlccraft --vlc-status` answers the same question the player asks: it prints
`vlc=present` with the engine's version and path, or `vlc=missing` and exits 3,
so a script or an installer can check without opening a window.

**Linux (64-bit Intel or AMD)** — two ways, both ending with VLCCraft in your
applications menu.

*Copy one line into a terminal.* It works in fish, bash and zsh; it fetches the
newest version, checks its fingerprint, and installs it. No root, nothing in
system folders:

```sh
curl -fsSL https://raw.githubusercontent.com/mjb73/vlccraft/main/install.sh -o /tmp/vlccraft-install.sh && sh /tmp/vlccraft-install.sh
```

That address is the installer file itself, kept in this repository, so it is
there at every version and does not depend on a release page being filled in.
The same script is also listed on the
[releases page](https://github.com/mjb73/vlccraft/releases) as `install.sh`; both
copies install the newest release, and running the one from the repository is
the route that cannot go stale.

*Or download it in a browser.* Click the Linux tarball on the
[releases page](https://github.com/mjb73/vlccraft/releases); it lands in your
**Downloads** folder. Then run these three lines in a terminal, one after
another. The `cd` is the part that matters: a fresh terminal starts in your home
folder, and the download is in Downloads. The `*` stands for the version number,
so these lines keep working when a newer release arrives:

```sh
cd ~/Downloads
tar -xzf vlccraft-*-x86_64-linux.tar.gz
sh vlccraft-*-x86_64-linux/install.sh
```

If a command answers `Cannot open: No such file or directory`, the file is not
in the folder the terminal is in. `ls ~/Downloads` shows what is there; the
name must match exactly, including the version number. The same holds for the
`Unknown command` that follows it: it means the unpacking never happened.

If a `curl` line answers `curl: (22) The requested URL returned error: 404`, the
address it names is not on the release page — an older copy of these
instructions asked the release for its installer, which a release published
before that file was added does not carry. Nothing is wrong with the machine or
with VLC. Use the one-line command above (it reads the file from the repository,
not from a release), or click `install.sh` on the releases page and run
`sh ~/Downloads/install.sh`.

The installer copies the program into `~/.local/bin`, adds a menu entry and
icon, and prints where each went. Run on its own like the line above, it
installs the newest release — an older tarball left in Downloads does not win
over it; the installer that came inside a download still installs that
download. It then starts the copy it just wrote, so a
download built for a newer system than yours fails there in plain words instead
of on your first double-click. Before finishing it checks for VLC: if that is
missing, it offers the command your distribution uses and continues cleanly when
the answer is no (set `VLCCRAFT_SKIP_VLC_PROMPT=1` to skip the question in a
scripted install). The `.deb` declares VLC as a dependency, so `apt` offers it
along with the player.

To start it by typing `vlccraft` as well as from the menu, put `~/.local/bin` on
your PATH — fish: `fish_add_path ~/.local/bin`; bash or zsh: add
`export PATH="$HOME/.local/bin:$PATH"` to `~/.profile`.

`sh install.sh --uninstall` removes the program, the menu entry and the icon and
keeps your library, presets and settings; `--purge` removes those too.

**macOS** — open the `.dmg` and drag VLCCraft to Applications. The one download
carries both Apple-silicon and Intel code, so it runs natively on either Mac. The
app is signed ad-hoc, so the first launch needs right-click → Open.

**Windows** — unzip and run `vlccraft.exe`.

### Downloads

Every release carries one download per system, on the
[releases page](https://github.com/mjb73/vlccraft/releases):

| System | File | Install |
|---|---|---|
| Linux (x86_64) | `vlccraft-<version>-x86_64-linux.tar.gz`, `install.sh` (the same installer, on its own) or `vlccraft_<version>_amd64.deb` | the one-line command above, or `sh install.sh`, or `sudo apt install ./vlccraft_<version>_amd64.deb` |
| macOS (universal) | `VLCCraft-<version>.dmg` | drag to Applications |
| Windows (x64) | `vlccraft-<version>-windows-x64.zip` | unzip and run |

Each file has a `.sha256` beside it. The build for all three runs on GitHub's
machines from the tagged source, so what you download is the code in this
repository, not a binary someone carried here.

## Version and updates

The number this copy carries is written in one place, `Cargo.toml`, and shown in
three: the window title, **Tools → About VLCCraft**, and **Settings → General**.
`vlccraft --version` prints it in a terminal.

VLCCraft updates from its own [releases page](https://github.com/mjb73/vlccraft/releases),
not from a package manager:

- **The check.** Once a day, the first time VLCCraft starts, it asks GitHub for
  the newest release. The answer is compared with the number compiled into the
  build. A newer release puts a small notice on screen — once — and the About
  card says what is available, on which date, and what changed. The switch for
  this is in Settings → General, and **Tools → Check for updates…** asks
  immediately.
- **The download.** The file for this system is fetched into
  `~/.local/share/vlccraft/updates` (Windows: `%LOCALAPPDATA%\vlccraft\updates`),
  with a bar and a size that counts up. Linux gets the tarball, Windows the zip,
  macOS the disk image.
- **The check on the file.** The download is hashed and compared with the
  `.sha256` published beside it. A mismatch stops the update and says so; no
  fingerprint published means no fingerprint to compare, which is also said.
- **The install.** Linux and Windows: the new program is unpacked and renamed
  over the old one, so the running copy keeps working until it is restarted.
  macOS: the disk image is opened, because an app inside a bundle cannot replace
  itself while it runs — drag VLCCraft to Applications.
- **What is never touched.** Your library index, resume positions, presets,
  playlists, bookmarks and settings live in `~/.local/share/vlccraft` and stay
  there. VLC's own files are never read or written.

The same three steps are available without the window, for a script or a
machine you reach over a network:

```sh
vlccraft --check-updates        # current number, newest release, which file
vlccraft --update               # download, check and install the newest release
vlccraft --update --force       # reinstall the newest release regardless
```

`--check-updates` exits 1 when GitHub cannot be reached, so a script can tell
"newer" from "unknown" instead of guessing.

## Making a release

1. Bump `version` in `Cargo.toml` (semantic versioning: 1.0.0 → 1.1.0 for a
   feature, → 1.0.1 for a fix).
2. Commit, then tag with the same number: `git tag v1.1.0 && git push origin v1.1.0`.
3. The Release workflow refuses a tag that disagrees with `Cargo.toml`, then
   builds Linux, macOS (universal) and Windows from the tagged source, packs
   them, writes a `.sha256` per file and publishes the release.

That release becomes the answer to every copy's update check within a day.

## Build from source

```sh
cargo build --release
```

Rust 1.80 or newer. On Linux the build needs the usual window-system headers
(`libxkbcommon-dev`, `libwayland-dev`, `libgl1-mesa-dev`).

## Command line

```
vlccraft [FILE|URL ...] [--screenshot FILE.png]
vlccraft --set-default [--video-only|--music-only]
vlccraft --restore-default
vlccraft --default-status
vlccraft --version
vlccraft --check-updates
vlccraft --update [--force]
```

A file handed to the program while a window is already open joins that window's
queue instead of starting a second player. `--screenshot` draws the window
120 frames, saves it as a PNG and exits without taking focus — that is how the
interface is checked after every build. `--version`, `--check-updates` and
`--update` print and exit without opening a window.

## Licence

MIT. VLCCraft is an independent interface and is not affiliated with the
VideoLAN project; VLC is VideoLAN's work and stays installed under its own
licence.
