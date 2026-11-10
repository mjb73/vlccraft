//! Making VLCCraft the standard media player.
//!
//! Nothing here touches the system packages. The app copies its own binary
//! into the person's home folder, adds a normal desktop entry, and asks the
//! desktop's own tool (`xdg-mime`) to hand video and music files to it. The
//! previous handlers are remembered so one button can hand them back.

use crate::model::{now_ms, read_json, write_json};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DESKTOP_ID: &str = "vlccraft.desktop";

/// The everyday video containers. Keeping this list short keeps the desktop
/// entry honest: what is listed is what was checked.
pub const VIDEO_MIMES: &[&str] = &[
    "video/mp4",
    "video/x-matroska",
    "video/webm",
    "video/x-msvideo",
    "video/quicktime",
    "video/mpeg",
    "video/ogg",
    "video/x-flv",
    "video/x-ms-wmv",
    "video/mp2t",
    "video/3gpp",
    "video/x-m4v",
    "application/vnd.apple.mpegurl",
    "application/x-mpegurl",
];

pub const AUDIO_MIMES: &[&str] = &[
    "audio/mpeg",
    "audio/flac",
    "audio/x-flac",
    "audio/ogg",
    "audio/opus",
    "audio/x-wav",
    "audio/mp4",
    "audio/x-m4a",
    "audio/aac",
    "audio/x-ms-wma",
    "audio/x-aiff",
];

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub available: bool,
    pub video_name: String,
    pub audio_name: String,
    pub video_ours: bool,
    pub audio_ours: bool,
    pub video_any: bool,
    pub audio_any: bool,
}

impl Status {
    pub fn all_ours(&self) -> bool {
        self.available && self.video_ours && self.audio_ours
    }
}

/// Which handlers were in place before VLCCraft changed them, plus which
/// scopes the person last asked for.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Record {
    pub previous: BTreeMap<String, String>,
    pub video: bool,
    pub audio: bool,
    pub installed_at: u64,
    pub launcher: String,
}

impl Record {
    pub fn can_hand_back(&self) -> bool {
        self.video || self.audio || !self.previous.is_empty()
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

fn data_home() -> PathBuf {
    match std::env::var("XDG_DATA_HOME") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(".local/share"),
    }
}

fn config_home() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(".config"),
    }
}

pub fn launcher_path() -> PathBuf {
    match std::env::var("XDG_BIN_HOME") {
        Ok(v) if !v.is_empty() => PathBuf::from(v).join("vlccraft"),
        _ => home().join(".local/bin").join("vlccraft"),
    }
}

pub fn desktop_path() -> PathBuf {
    data_home().join("applications").join(DESKTOP_ID)
}

fn icon_path() -> PathBuf {
    data_home()
        .join("icons/hicolor/256x256/apps")
        .join("vlccraft.png")
}

fn record_path() -> PathBuf {
    crate::model::data_dir().join("defaults.json")
}

pub fn load_record() -> Record {
    read_json(&record_path())
}

pub fn save_record(rec: &Record) -> bool {
    write_json(&record_path(), rec)
}

// ---------------------------------------------------------------------------
// Asking the desktop
// ---------------------------------------------------------------------------

/// The default recorded in a mimeapps.list, which is what a desktop reads
/// first. `xdg-mime query` also falls back to a scan of every installed app,
/// so a freshly installed entry can look like the default before it is one.
fn mimeapps_files() -> Vec<PathBuf> {
    let mut out = vec![config_home().join("mimeapps.list")];
    let config_dirs = std::env::var("XDG_CONFIG_DIRS").unwrap_or_else(|_| "/etc/xdg".into());
    for dir in config_dirs.split(':').filter(|d| !d.is_empty()) {
        out.push(PathBuf::from(dir).join("mimeapps.list"));
    }
    out.push(data_home().join("applications").join("mimeapps.list"));
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
        out.push(PathBuf::from(dir).join("applications").join("mimeapps.list"));
    }
    out
}

fn configured_default(mime: &str) -> Option<String> {
    let needle = format!("{mime}=");
    for file in mimeapps_files() {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        let mut in_defaults = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_defaults = trimmed == "[Default Applications]";
                continue;
            }
            if !in_defaults {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(&needle) {
                let first = rest.split(';').next().unwrap_or("").trim();
                if !first.is_empty() {
                    return Some(first.to_string());
                }
            }
        }
    }
    None
}

fn effective_default(mime: &str) -> Option<String> {
    configured_default(mime).or_else(|| query_default(mime))
}

fn query_default(mime: &str) -> Option<String> {
    let out = Command::new("xdg-mime")
        .args(["query", "default", mime])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn pretty(id: &str) -> String {
    let base = id.trim().trim_end_matches(".desktop");
    if base.is_empty() {
        return "nothing yet".into();
    }
    let last = base.rsplit('.').next().unwrap_or(base);
    match last.to_ascii_lowercase().as_str() {
        "vlc" => "VLC".into(),
        "vlccraft" => "VLCCraft".into(),
        "mpv" => "mpv".into(),
        "celluloid" => "Celluloid".into(),
        "haruna" => "Haruna".into(),
        "totem" => "Videos".into(),
        "dragon" => "Dragon Player".into(),
        "smplayer" => "SMPlayer".into(),
        "parole" => "Parole".into(),
        "clapper" => "Clapper".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => id.into(),
            }
        }
    }
}

/// Two representative checks per kind: the window never waits on a long list
/// of subprocesses, and the picture the person sees stays truthful.
pub fn status() -> Status {
    if !available() {
        return Status::default();
    }
    let mp4 = effective_default(VIDEO_MIMES[0]);
    let mkv = effective_default("video/x-matroska");
    let mp3 = effective_default(AUDIO_MIMES[0]);
    let flac = effective_default("audio/x-flac");
    // A name for display: what the desktop would actually open. A bare
    // candidate (our entry sitting in the app list) is not a default.
    let configured_mp4 = configured_default(VIDEO_MIMES[0]);
    let configured_music = configured_default(AUDIO_MIMES[0]);
    let name = |effective: Option<&String>, configured: Option<&String>| -> String {
        match effective {
            Some(id) if !(id == DESKTOP_ID && configured.map(|s| s.as_str()) != Some(DESKTOP_ID)) => {
                pretty(id)
            }
            _ => "no player set".into(),
        }
    };
    // "Ours" is read from the recorded defaults, not from the fallback scan:
    // an installed entry is a candidate, not a default.
    let v1 = configured_default(VIDEO_MIMES[0]);
    let v2 = configured_default("video/x-matroska");
    let a1 = configured_default(AUDIO_MIMES[0]);
    let a2 = configured_default("audio/x-flac");
    let is_ours = |id: &Option<String>| id.as_deref() == Some(DESKTOP_ID);
    Status {
        available: true,
        video_name: name(mp4.as_ref(), configured_mp4.as_ref()),
        audio_name: name(mp3.as_ref(), configured_music.as_ref()),
        video_ours: is_ours(&v1) && is_ours(&v2),
        audio_ours: is_ours(&a1) && is_ours(&a2),
        video_any: is_ours(&v1) || is_ours(&v2),
        audio_any: is_ours(&a1) || is_ours(&a2),
    }
}

// ---------------------------------------------------------------------------
// Installing the launcher, entry and icon
// ---------------------------------------------------------------------------

/// A debug run installs the release build beside it when there is one:
/// smaller, faster, and the one the person actually uses.
fn preferred_source(mut src: PathBuf) -> PathBuf {
    let text = src.to_string_lossy().to_string();
    if text.contains("/target/debug/") {
        if let Some(root) = src.parent().and_then(|p| p.parent()) {
            let release = root.join("release").join("vlccraft");
            let newer = match (fs::metadata(&release), fs::metadata(&src)) {
                (Ok(r), Ok(d)) => match (r.modified(), d.modified()) {
                    (Ok(rt), Ok(dt)) => rt >= dt,
                    _ => false,
                },
                _ => false,
            };
            if newer {
                src = release;
            }
        }
    }
    src
}

/// Keeps the copy the desktop runs in step with this build.
///
/// The standard-player entry runs `~/.local/bin/vlccraft`, a copy made at the
/// moment the person said "use this as my player". Without this, a rebuilt
/// program never reaches a double-clicked file: the desktop keeps starting the
/// older copy and its old behaviour. Only an already installed copy is
/// refreshed; a run that is itself that copy does nothing.
pub fn refresh_launcher() {
    let dest = launcher_path();
    if !dest.is_file() {
        return;
    }
    let Ok(current) = std::env::current_exe() else {
        return;
    };
    if let (Ok(a), Ok(b)) = (fs::canonicalize(&current), fs::canonicalize(&dest)) {
        if a == b {
            return;
        }
    }
    let src = preferred_source(current);
    let stale = match (fs::metadata(&src), fs::metadata(&dest)) {
        (Ok(s), Ok(d)) => {
            s.len() != d.len()
                || match (s.modified(), d.modified()) {
                    (Ok(st), Ok(dt)) => st > dt,
                    _ => true,
                }
        }
        _ => false,
    };
    if stale {
        let _ = install_launcher();
    }
}

fn install_launcher() -> Result<PathBuf, String> {
    let src = preferred_source(
        std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?,
    );

    let dest = launcher_path();
    if let (Ok(a), Ok(b)) = (fs::canonicalize(&src), fs::canonicalize(&dest)) {
        if a == b {
            return Ok(dest);
        }
    }
    if let (Ok(s), Ok(d)) = (fs::metadata(&src), fs::metadata(&dest)) {
        if s.len() == d.len() {
            if let (Ok(st), Ok(dt)) = (s.modified(), d.modified()) {
                if dt >= st {
                    return Ok(dest);
                }
            }
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    let tmp = dest.with_extension("new");
    let _ = fs::remove_file(&tmp);
    fs::copy(&src, &tmp).map_err(|e| format!("cannot copy this program: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = fs::metadata(&tmp)
            .map_err(|e| format!("cannot read the copy: {e}"))?
            .permissions();
        perm.set_mode(0o755);
        let _ = fs::set_permissions(&tmp, perm);
    }
    fs::rename(&tmp, &dest).map_err(|e| format!("cannot put the launcher in place: {e}"))?;
    Ok(dest)
}

fn exec_field(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if raw.contains(' ') || raw.contains('"') {
        format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        raw.to_string()
    }
}

fn write_desktop(launcher: &Path, video: bool, audio: bool) -> Result<(), String> {
    let mut mimes: Vec<&str> = Vec::new();
    if video {
        mimes.extend_from_slice(VIDEO_MIMES);
    }
    if audio {
        mimes.extend_from_slice(AUDIO_MIMES);
    }
    if mimes.is_empty() {
        mimes.extend_from_slice(VIDEO_MIMES);
        mimes.extend_from_slice(AUDIO_MIMES);
    }
    let list = mimes
        .iter()
        .map(|m| format!("{m};"))
        .collect::<Vec<_>>()
        .join("");
    let body = format!(
        "[Desktop Entry]\n\
Type=Application\n\
Version=1.5\n\
Name=VLCCraft\n\
GenericName=Media Player\n\
Comment=A calm skin over the player already installed on this machine\n\
Exec={} %F\n\
TryExec={}\n\
Icon=vlccraft\n\
Terminal=false\n\
Categories=AudioVideo;Player;Video;Audio;\n\
MimeType={}\n\
StartupNotify=true\n\
StartupWMClass=vlccraft\n\
Keywords=video;audio;movie;music;player;media;subtitle;\n",
        exec_field(launcher),
        exec_field(launcher),
        list
    );
    let path = desktop_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    fs::write(&path, body).map_err(|e| format!("cannot write the app entry: {e}"))?;
    let _ = Command::new("update-desktop-database")
        .arg(path.parent().unwrap_or(Path::new(".")))
        .output();
    Ok(())
}

/// Draws a neutral play mark — a rounded square in the accent colour with a
/// simple triangle. No third-party artwork is copied.
fn write_icon(accent: [u8; 3]) -> Result<(), String> {
    let path = icon_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
    }
    let size: u32 = 256;
    let mut img = image::RgbaImage::from_pixel(size, size, image::Rgba([0, 0, 0, 0]));
    let top = [
        (accent[0] as f32 * 1.12).min(255.0) as u8,
        (accent[1] as f32 * 1.10).min(255.0) as u8,
        (accent[2] as f32 * 1.06).min(255.0) as u8,
    ];
    let bottom = [
        (accent[0] as f32 * 0.82) as u8,
        (accent[1] as f32 * 0.80) as u8,
        (accent[2] as f32 * 0.80) as u8,
    ];
    let (x0, y0, x1, y1) = (14.0f32, 14.0, 242.0, 242.0);
    let radius = 54.0f32;
    let cx = (x0 + x1) / 2.0;
    let cy = (y0 + y1) / 2.0;
    let hw = (x1 - x0) / 2.0;
    let hh = (y1 - y0) / 2.0;
    let tri = [(104.0f32, 78.0f32), (104.0, 178.0), (186.0, 128.0)];

    let inside_round_rect = |px: f32, py: f32| -> bool {
        let qx = (px - cx).abs() - (hw - radius);
        let qy = (py - cy).abs() - (hh - radius);
        let d = qx.max(0.0).powi(2) + qy.max(0.0).powi(2);
        d.sqrt() + qx.max(qy).min(0.0) - radius <= 0.0
    };
    let inside_tri = |px: f32, py: f32| -> bool {
        let sign = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| {
            (a.0 - c.0) * (b.1 - c.1) - (b.0 - c.0) * (a.1 - c.1)
        };
        let p = (px, py);
        let d1 = sign(p, tri[0], tri[1]);
        let d2 = sign(p, tri[1], tri[2]);
        let d3 = sign(p, tri[2], tri[0]);
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    };

    for y in 0..size {
        for x in 0..size {
            let mut cover = 0u32;
            let mut tri_cover = 0u32;
            for sy in 0..3 {
                for sx in 0..3 {
                    let px = x as f32 + (sx as f32 + 0.5) / 3.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 3.0;
                    if inside_round_rect(px, py) {
                        cover += 1;
                        if inside_tri(px, py) {
                            tri_cover += 1;
                        }
                    }
                }
            }
            if cover == 0 {
                continue;
            }
            let t = ((y as f32 + 0.5) - y0) / (y1 - y0);
            let mut rgb = [
                (top[0] as f32 * (1.0 - t) + bottom[0] as f32 * t) as u8,
                (top[1] as f32 * (1.0 - t) + bottom[1] as f32 * t) as u8,
                (top[2] as f32 * (1.0 - t) + bottom[2] as f32 * t) as u8,
            ];
            if tri_cover > 0 {
                let a = tri_cover as f32 / cover as f32;
                for i in 0..3 {
                    rgb[i] = (rgb[i] as f32 * (1.0 - a) + 250.0 * a) as u8;
                }
            }
            img.put_pixel(
                x,
                y,
                image::Rgba([rgb[0], rgb[1], rgb[2], (255.0 * cover as f32 / 9.0) as u8]),
            );
        }
    }
    image::save_buffer_with_format(
        &path,
        img.as_raw(),
        size,
        size,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("cannot draw the app icon: {e}"))
}

// ---------------------------------------------------------------------------
// Handing files over and back
// ---------------------------------------------------------------------------

fn mimes_for(video: bool, audio: bool) -> Vec<&'static str> {
    let mut out: Vec<&str> = Vec::new();
    if video {
        out.extend_from_slice(VIDEO_MIMES);
    }
    if audio {
        out.extend_from_slice(AUDIO_MIMES);
    }
    out
}

/// Removes one entry from the person's mimeapps.list. Used only when the
/// previous state was "no handler at all", which the desktop tool cannot set.
fn strip_association(mime: &str) {
    let path = config_home().join("mimeapps.list");
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let mut out = String::new();
    let mut section = String::new();
    let mut touched = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed.to_string();
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let matches_section = section == "[Default Applications]" || section == "[Added Associations]";
        if matches_section {
            if let Some(rest) = trimmed.strip_prefix(&format!("{mime}=")) {
                let kept: Vec<&str> = rest
                    .split(';')
                    .filter(|s| !s.is_empty() && *s != DESKTOP_ID)
                    .collect();
                touched = true;
                if !kept.is_empty() {
                    out.push_str(&format!("{mime}={};\n", kept.join(";")));
                }
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if touched {
        let _ = fs::write(&path, out);
    }
}

/// True when the recorded default is ours, or when no default is recorded at
/// all (the desktop tool would decide, so handing back is still the right
/// thing to do). A choice the person made elsewhere is left alone.
fn safe_to_restore(mime: &str) -> bool {
    match configured_default(mime) {
        Some(id) => id == DESKTOP_ID,
        None => true,
    }
}

fn restore_one(mime: &str, previous: &str) {
    if previous.is_empty() || previous == DESKTOP_ID {
        strip_association(mime);
        return;
    }
    let _ = Command::new("xdg-mime")
        .args(["default", previous, mime])
        .output();
}

fn apply_mimes(mimes: &[&str], ours: bool) -> bool {
    if ours {
        let mut cmd = Command::new("xdg-mime");
        cmd.arg("default").arg(DESKTOP_ID);
        for mime in mimes {
            cmd.arg(mime);
        }
        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    } else {
        true
    }
}

/// Installs the launcher and entry, then makes VLCCraft (or not) the handler
/// for the chosen kinds. Returns a sentence for the person.
pub fn apply(video: bool, audio: bool, accent: [u8; 3]) -> Result<String, String> {
    if !available() {
        return Err(
            "This system has no xdg-mime tool, so set the default in your desktop's own settings."
                .into(),
        );
    }
    // Read the existing handlers before our entry exists anywhere: a new
    // desktop file changes the fallback scan the desktop tool uses.
    let mut rec = load_record();
    for mime in mimes_for(true, true) {
        if !rec.previous.contains_key(mime) {
            if let Some(current) = effective_default(mime) {
                if current != DESKTOP_ID {
                    rec.previous.insert(mime.to_string(), current);
                }
            }
        }
    }

    let launcher = install_launcher()?;
    write_desktop(&launcher, video, audio)?;
    let _ = write_icon(accent);

    let wanted = mimes_for(video, audio);

    let ok = apply_mimes(&wanted, true);
    if !ok {
        return Err("The desktop refused the change; nothing else was altered.".into());
    }

    // Turning a kind off hands those files back — unless the person has
    // already chosen a different player for that type in the meantime.
    if !video {
        for mime in mimes_for(true, false) {
            if !safe_to_restore(mime) {
                continue;
            }
            if let Some(prev) = rec.previous.get(mime).cloned() {
                restore_one(mime, &prev);
            } else {
                strip_association(mime);
            }
        }
    }
    if !audio {
        for mime in mimes_for(false, true) {
            if !safe_to_restore(mime) {
                continue;
            }
            if let Some(prev) = rec.previous.get(mime).cloned() {
                restore_one(mime, &prev);
            } else {
                strip_association(mime);
            }
        }
    }

    rec.video = video;
    rec.audio = audio;
    rec.installed_at = now_ms();
    rec.launcher = launcher.to_string_lossy().to_string();
    save_record(&rec);

    Ok(match (video, audio) {
        (true, true) => "Video and music now open in VLCCraft. VLC is still installed, untouched.",
        (true, false) => "Video now opens in VLCCraft. Music is left as it was.",
        (false, true) => "Music now opens in VLCCraft. Video is left as it was.",
        (false, false) => "Files are back with the player that had them before.",
    }
    .to_string())
}

/// Hands every remembered association back to the player that had it.
pub fn restore() -> Result<String, String> {
    let mut rec = load_record();
    if rec.previous.is_empty() {
        return Ok("There was no earlier player to hand files back to.".into());
    }
    let mut count = 0;
    for (mime, prev) in rec.previous.clone() {
        if !safe_to_restore(&mime) {
            continue;
        }
        restore_one(&mime, &prev);
        count += 1;
    }
    rec.previous.clear();
    rec.video = false;
    rec.audio = false;
    save_record(&rec);
    Ok(format!(
        "Handed {count} file type{} back to the earlier player.",
        if count == 1 { "" } else { "s" }
    ))
}

pub fn available() -> bool {
    Command::new("xdg-mime")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
