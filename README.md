# VLCCraft

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

## Install

You need VLC installed — VLCCraft uses its engine.

**Linux** — the tarball, or the `.deb`:

```sh
tar -xzf vlccraft-1.0.0-x86_64-linux.tar.gz
./vlccraft-1.0.0-x86_64-linux/install.sh
```

The installer copies the program into `~/.local/bin`, adds an ordinary desktop
entry and icon, and can then hand double-clicked video and music files over
(Settings → General → Default player).

**macOS** — open the `.dmg` for your Mac (`aarch64` for Apple silicon, `x86_64`
for Intel) and drag VLCCraft to Applications. The app is signed ad-hoc, so the
first launch needs right-click → Open.

**Windows** — unzip and run `vlccraft.exe`.

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
```

A file handed to the program while a window is already open joins that window's
queue instead of starting a second player. `--screenshot` draws the window
120 frames, saves it as a PNG and exits without taking focus — that is how the
interface is checked after every build.

## Licence

MIT. VLCCraft is an independent interface and is not affiliated with the
VideoLAN project; VLC is VideoLAN's work and stays installed under its own
licence.
