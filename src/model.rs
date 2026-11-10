//! Data model, persistence and media helpers for VLCCraft.
//!
//! Everything here is plain data: the library index, playlists, queue,
//! subtitle presets, layout profiles, settings and the small parsers used
//! by the interface (SRT, chapters, m3u, MP4 duration).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn data_dir() -> PathBuf {
    // Linux, Windows and macOS keep application data in different places; the
    // per-system answer lives in `platform` so the rest of the app has one name.
    crate::platform::data_dir()
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SubtitleStyle {
    pub font: String,
    pub size_pct: f32,
    pub outline: f32,
    pub position_pct: f32,
    pub delay_ms: i64,
    pub scale: f32,
    pub background: bool,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font: "Default".into(),
            size_pct: 4.2,
            outline: 1.0,
            position_pct: 8.0,
            delay_ms: 0,
            scale: 1.0,
            background: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Layout {
    /// Visible controls, in bar order. The default layout is complete.
    pub controls: Vec<String>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            controls: [
                "previous",
                "play",
                "next",
                "seek",
                "time",
                "subtitles",
                "audio",
                "queue",
                "volume",
                "bookmark",
                "pip",
                "fullscreen",
                "settings",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

pub const ALL_CONTROLS: &[(&str, &str)] = &[
    ("previous", "Previous"),
    ("play", "Play / pause"),
    ("next", "Next"),
    ("seek", "Seek bar"),
    ("time", "Timecode"),
    ("subtitles", "Subtitles"),
    ("audio", "Audio"),
    ("queue", "Queue"),
    ("volume", "Volume"),
    ("bookmark", "Bookmark"),
    ("pip", "Picture-in-picture"),
    ("fullscreen", "Full screen"),
    ("settings", "Settings"),
];

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub theme: String,
    pub accent: [u8; 3],
    pub volume: u8,
    pub muted: bool,
    pub resume_enabled: bool,
    /// Start the next queued item when one ends. Deliberately silent: nothing
    /// is drawn or announced at the end of an item.
    #[serde(alias = "end_prompt_enabled")]
    pub auto_advance: bool,
    pub show_thumbnails: bool,
    pub show_buffered: bool,
    pub reveal_distance: f32,
    pub hide_delay_ms: u64,
    pub subtitle_default_language: String,
    pub subtitle_precision: String,
    pub subtitle_style: SubtitleStyle,
    pub sidebar_open: bool,
    pub sidebar_tab: usize,
    pub onboarding_done: bool,
    pub audio_output: String,
    pub hardware_decode: bool,
    pub interface_scale: f32,
    pub reduce_motion: bool,
    pub start_page: String,
    pub remember_window: bool,
    /// The window as the person left it — inner size in points, and whether it
    /// was maximised. Written only while `remember_window` is on, applied at
    /// the next start.
    pub window_size: Option<[f32; 2]>,
    pub window_maximized: bool,
    pub layout: Layout,
    pub active_profile: String,
    pub hotkeys: BTreeMap<String, String>,
    pub addons: Vec<String>,
    pub plugins_readonly: bool,
    pub relink_prompt: bool,
    pub check_updates: bool,
    pub save_metadata_beside_media: bool,
    pub watched_threshold: u8,
    pub recording_dir: String,
    pub recording_format: String,
    pub network_timeout_ms: u32,
    pub subtitle_online_enabled: bool,
    pub thumbnail_cap: u32,
    /// Which kinds of file VLCCraft should open when it is made the default.
    pub default_video: bool,
    pub default_audio: bool,
    /// The small Home hint explaining the default player; dismissible.
    pub default_hint: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut hotkeys = BTreeMap::new();
        for (k, v) in [
            ("Play / pause", "Space"),
            ("Seek back 5 s", "Left"),
            ("Seek forward 5 s", "Right"),
            ("Seek back 30 s", "Shift+Left"),
            ("Seek forward 30 s", "Shift+Right"),
            ("Volume up", "Up"),
            ("Volume down", "Down"),
            ("Mute", "M"),
            ("Full screen", "F"),
            ("Subtitles sidebar", "S"),
            ("Queue", "Q"),
            ("Media library", "L"),
            ("Add bookmark", "Ctrl+B"),
            ("Open file", "Ctrl+O"),
            ("Open network stream", "Ctrl+Shift+O"),
            ("Subtitle delay −50 ms", "["),
            ("Subtitle delay +50 ms", "]"),
            ("Next item", "N"),
            ("Previous item", "P"),
            ("Settings", "Ctrl+,"),
            ("Hide the interface", "H"),
            ("Close surface", "Esc"),
        ] {
            hotkeys.insert(k.into(), v.into());
        }
        Self {
            schema_version: 2,
            theme: "Midnight".into(),
            accent: [242, 168, 76],
            volume: 80,
            muted: false,
            resume_enabled: true,
            auto_advance: true,
            show_thumbnails: true,
            show_buffered: true,
            reveal_distance: 12.0,
            hide_delay_ms: 1500,
            subtitle_default_language: "English".into(),
            subtitle_precision: "contains".into(),
            subtitle_style: SubtitleStyle::default(),
            sidebar_open: true,
            sidebar_tab: 0,
            onboarding_done: false,
            audio_output: "Automatic".into(),
            hardware_decode: true,
            interface_scale: 1.0,
            reduce_motion: false,
            start_page: "Home".into(),
            remember_window: true,
            window_size: None,
            window_maximized: false,
            layout: Layout::default(),
            active_profile: "Default".into(),
            hotkeys,
            addons: vec!["Subtitle finder".into(), "Last.fm scrobbler".into()],
            plugins_readonly: true,
            relink_prompt: true,
            check_updates: true,
            save_metadata_beside_media: false,
            watched_threshold: 92,
            recording_dir: String::new(),
            recording_format: "Matroska (.mkv)".into(),
            network_timeout_ms: 10000,
            subtitle_online_enabled: false,
            thumbnail_cap: 60,
            default_video: true,
            default_audio: true,
            default_hint: true,
        }
    }
}

impl Settings {
    /// SET-10: old keys are accepted through aliases/migrations so a new
    /// version never discards an existing configuration.
    pub fn migrate(&mut self) {
        if self.schema_version < 2 {
            // "subtitle_precision" used numeric enum values 0..4 in older builds.
            self.subtitle_precision = match self.subtitle_precision.as_str() {
                "0" => "none".into(),
                "1" => "any".into(),
                "2" => "contains".into(),
                "3" => "name_plus".into(),
                "4" => "exact".into(),
                other => other.to_string(),
            };
            self.schema_version = 2;
        }
        if self.layout.controls.is_empty() {
            self.layout = Layout::default();
        }
        // Drop ids that no longer exist, keep a complete default instead.
        self.layout
            .controls
            .retain(|id| ALL_CONTROLS.iter().any(|(k, _)| k == id));
        if self.layout.controls.is_empty() {
            self.layout = Layout::default();
        }
    }

    pub fn hotkey(&self, action: &str) -> String {
        self.hotkeys.get(action).cloned().unwrap_or_default()
    }
}

pub const SUBTITLE_PRECISIONS: &[(&str, &str)] = &[
    ("none", "No subtitle file"),
    ("any", "Any subtitle file in the folder"),
    ("contains", "Filename contains the movie name"),
    ("name_plus", "Movie name plus characters"),
    ("exact", "Exactly the movie name"),
];

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Bookmark {
    pub at_ms: i64,
    pub label: String,
    pub created: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LibraryItem {
    pub id: String,
    pub path: String,
    pub kind: String, // video | audio | stream
    pub title: String,
    pub show: String,
    pub size: u64,
    pub mtime: u64,
    pub duration_ms: i64,
    pub watched: bool,
    pub position_ms: i64,
    pub added_at: u64,
    pub subtitle_path: Option<String>,
    pub bookmarks: Vec<Bookmark>,
}

impl Default for LibraryItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            path: String::new(),
            kind: "video".into(),
            title: String::new(),
            show: String::new(),
            size: 0,
            mtime: 0,
            duration_ms: 0,
            watched: false,
            position_ms: 0,
            added_at: now_ms(),
            subtitle_path: None,
            bookmarks: Vec::new(),
        }
    }
}

impl LibraryItem {
    pub fn progress(&self) -> f32 {
        if self.duration_ms <= 0 {
            0.0
        } else {
            (self.position_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub items: Vec<String>, // paths
    pub created: u64,
    pub modified: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Queue {
    pub items: Vec<String>,
    pub index: usize,
    pub pinned: bool,
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
            pinned: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub match_kind: String, // folder | pattern
    pub match_value: String,
    pub delay_ms: i64,
    pub scale: f32,
    pub size_pct: f32,
    pub outline: f32,
    pub position_pct: f32,
    pub font: String,
    pub track_language: String,
    pub audio_language: String,
    pub filter: String,
    pub created: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SettingsPatch {
    pub theme: Option<String>,
    pub accent: Option<[u8; 3]>,
    pub volume: Option<u8>,
    pub subtitle_size: Option<f32>,
    pub subtitle_outline: Option<f32>,
    pub subtitle_position: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub layout: Layout,
    pub overrides: SettingsPatch,
    pub created: u64,
    pub schema_version: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Library {
    pub folders: Vec<String>,
    pub items: Vec<LibraryItem>,
    pub streams: Vec<String>,
}

impl Library {
    pub fn get(&self, path: &str) -> Option<&LibraryItem> {
        self.items.iter().find(|i| i.path == path)
    }
    pub fn get_mut(&mut self, path: &str) -> Option<&mut LibraryItem> {
        self.items.iter_mut().find(|i| i.path == path)
    }
    pub fn title_for(&self, path: &str) -> String {
        self.get(path)
            .map(|i| i.title.clone())
            .unwrap_or_else(|| title_from_path(Path::new(path)))
    }
    pub fn show_for(&self, path: &str) -> String {
        self.get(path).map(|i| i.show.clone()).unwrap_or_default()
    }
    pub fn remove_missing(&mut self) {
        self.items.retain(|i| Path::new(&i.path).exists());
    }
    pub fn ensure_folder(&mut self, folder: &Path) -> usize {
        let canon = folder.canonicalize().unwrap_or_else(|_| folder.to_path_buf());
        let key = canon.to_string_lossy().to_string();
        if !self.folders.iter().any(|f| f == &key) {
            self.folders.push(key.clone());
        }
        let mut added = 0;
        scan_folder(&canon, 0, &mut |path, kind| {
            let p = path.to_string_lossy().to_string();
            if self.items.iter().any(|i| i.path == p) {
                return;
            }
            let mut item = item_from_path(path, kind);
            if let Some(existing) = self.items.iter().find(|e| e.title.eq_ignore_ascii_case(&item.title) && e.path.ends_with(&item.title)) {
                item.position_ms = existing.position_ms;
            }
            self.items.push(item);
            added += 1;
        });
        self.items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        added
    }
}

pub fn title_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".into());
    if let Some((base, _)) = stem.rsplit_once('.') {
        if base.len() > 2 && stem.rsplit('.').next().map(|e| e.len() <= 4).unwrap_or(false) {
            return base.to_string();
        }
    }
    stem
}

pub fn show_from_path(path: &Path) -> String {
    let parent = path.parent().unwrap_or(Path::new(""));
    let name = parent
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    if lower == "season 1" || lower == "season 2" || lower == "season 3" || lower.starts_with("season ") {
        return parent
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(name);
    }
    name
}

pub fn kind_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" | "opus" => "audio",
        _ => "video",
    }
}

pub fn item_from_path(path: &Path, kind: &str) -> LibraryItem {
    let md = fs::metadata(path).ok();
    let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = md
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stem = title_from_path(path);
    let show = show_from_path(path);
    let duration = mp4_duration_ms(path).unwrap_or(0);
    LibraryItem {
        id: format!("item-{}", stable_hash(path.to_string_lossy().as_bytes())),
        path: path.to_string_lossy().to_string(),
        kind: kind.into(),
        title: stem,
        show,
        size,
        mtime,
        duration_ms: duration,
        watched: false,
        position_ms: 0,
        added_at: now_ms(),
        subtitle_path: None,
        bookmarks: Vec::new(),
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

const MEDIA_EXTS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "m4v", "mpg", "mpeg", "ts", "flv", "mp3", "flac", "ogg",
    "wav", "m4a", "aac", "opus",
];

pub fn scan_folder(root: &Path, depth: usize, add: &mut impl FnMut(&Path, &str)) {
    if depth > 8 {
        return;
    }
    let Ok(read) = fs::read_dir(root) else { return };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            scan_folder(&path, depth + 1, add);
        } else {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if MEDIA_EXTS.contains(&ext.as_str()) {
                let kind = kind_for(&path);
                add(&path, kind);
            }
        }
    }
}

/// Read duration from an MP4/M4A `mvhd` box without external tools.
pub fn mp4_duration_ms(path: &Path) -> Option<i64> {
    let data = fs::read(path).ok()?;
    if data.len() < 64 {
        return None;
    }
    let mut i = 0usize;
    while i + 8 < data.len().min(8 * 1024 * 1024) {
        if &data[i..i + 4] == b"mvhd" {
            let version = data[i + 4];
            let (timescale, duration) = if version == 1 {
                let ts = u32::from_be_bytes(data.get(i + 24..i + 28)?.try_into().ok()?) as u64;
                let dur = u64::from_be_bytes(data.get(i + 28..i + 36)?.try_into().ok()?);
                (ts, dur)
            } else {
                let ts = u32::from_be_bytes(data.get(i + 16..i + 20)?.try_into().ok()?) as u64;
                let dur = u32::from_be_bytes(data.get(i + 20..i + 24)?.try_into().ok()?) as u64;
                (ts, dur)
            };
            if timescale > 0 && duration > 0 {
                return Some((duration * 1000 / timescale) as i64);
            }
            return None;
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Subtitles
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Cue {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

pub fn parse_srt(data: &str) -> Vec<Cue> {
    let normalized = data.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();
    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() {
            continue;
        }
        let mut time_line = None;
        let mut text_start = 0;
        for (idx, line) in lines.iter().enumerate() {
            if line.contains("-->") {
                time_line = Some(*line);
                text_start = idx + 1;
                break;
            }
        }
        let Some(tl) = time_line else { continue };
        let mut parts = tl.split("-->");
        let (Some(a), Some(b)) = (parts.next(), parts.next()) else { continue };
        let Some(start) = parse_timestamp(a) else { continue };
        let Some(end) = parse_timestamp(b.split_whitespace().next().unwrap_or(b)) else { continue };
        let text = lines[text_start..].join("\n");
        if !text.trim().is_empty() {
            cues.push(Cue {
                start_ms: start,
                end_ms: end,
                text,
            });
        }
    }
    cues.sort_by_key(|c| c.start_ms);
    cues
}

pub fn parse_timestamp(raw: &str) -> Option<i64> {
    let raw = raw.trim().replace(',', ".");
    let mut it = raw.split(':');
    let h: i64 = it.next()?.trim().parse().ok()?;
    let m: i64 = it.next()?.trim().parse().ok()?;
    let sec: f64 = it.next()?.trim().parse().ok()?;
    Some((h * 3600 + m * 60) as i64 * 1000 + (sec * 1000.0) as i64)
}

/// A safe default output name beside the source. Never the same file.
pub fn suggested_convert_path(source: &str, ext: &str) -> String {
    if source.trim().is_empty() {
        return String::new();
    }
    let path = Path::new(source);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "converted".into());
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("{stem} (converted).{ext}"))
        .to_string_lossy()
        .to_string()
}

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn format_time(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn find_subtitle(media: &Path, precision: &str) -> SubtitleMatch {
    let folder = media.parent().unwrap_or(Path::new("."));
    let stem = media
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut candidates = Vec::new();
    if let Ok(read) = fs::read_dir(folder) {
        for entry in read.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .as_deref()
                != Some("srt")
            {
                continue;
            }
            let s = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let matches = match precision {
                "none" => false,
                "any" => true,
                "exact" => s == stem,
                "name_plus" => s.starts_with(&format!("{stem}.")),
                _ => s == stem || s.starts_with(&format!("{stem}.")) || s.starts_with(&format!("{stem} ")),
            };
            if matches {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    let exact = candidates.iter().find(|p| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .as_deref()
            == Some(stem.as_str())
    });
    SubtitleMatch {
        chosen: exact.cloned().or_else(|| candidates.first().cloned()),
        candidates,
    }
}

#[derive(Clone, Debug, Default)]
pub struct SubtitleMatch {
    pub chosen: Option<PathBuf>,
    pub candidates: Vec<PathBuf>,
}

pub fn load_cues(path: &Path) -> Vec<Cue> {
    fs::read_to_string(path)
        .map(|s| parse_srt(&s))
        .unwrap_or_default()
}

pub struct MatchReport {
    pub matched: Vec<(String, String)>,
    pub unmatched_media: Vec<String>,
    pub unmatched_subs: Vec<String>,
    pub ambiguous: Vec<String>,
}

pub fn match_folder_subtitles(folder: &Path) -> MatchReport {
    let mut media = Vec::new();
    let mut subs = Vec::new();
    if let Ok(read) = fs::read_dir(folder) {
        for entry in read.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if MEDIA_EXTS.contains(&ext.as_str()) && kind_for(&path) == "video" {
                media.push(path);
            } else if ext == "srt" {
                subs.push(path);
            }
        }
    }
    media.sort();
    subs.sort();
    let mut report = MatchReport {
        matched: Vec::new(),
        unmatched_media: Vec::new(),
        unmatched_subs: Vec::new(),
        ambiguous: Vec::new(),
    };
    for m in &media {
        let stem = m.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let mut hits: Vec<&PathBuf> = subs
            .iter()
            .filter(|s| {
                let st = s.file_stem().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                st == stem
            })
            .collect();
        if hits.is_empty() {
            hits = subs
                .iter()
                .filter(|s| {
                    let st = s.file_stem().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                    st.starts_with(&stem)
                })
                .collect();
        }
        if hits.len() == 1 {
            report.matched.push((
                m.to_string_lossy().to_string(),
                hits[0].to_string_lossy().to_string(),
            ));
        } else if hits.len() > 1 {
            report.ambiguous.push(m.to_string_lossy().to_string());
        } else {
            report.unmatched_media.push(m.to_string_lossy().to_string());
        }
    }
    for s in &subs {
        if !report.matched.iter().any(|(_, x)| x == &s.to_string_lossy()) {
            report.unmatched_subs.push(s.to_string_lossy().to_string());
        }
    }
    report
}

// ---------------------------------------------------------------------------
// Chapters / m3u / disk stores
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Chapter {
    pub at_ms: i64,
    pub label: String,
}

pub fn load_chapters(media: &Path) -> Vec<Chapter> {
    let stem = media.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let candidates = [
        media.with_file_name(format!("{stem}.chapters.txt")),
        media.with_file_name(format!("{stem}.txt")),
    ];
    for path in candidates {
        if let Ok(text) = fs::read_to_string(&path) {
            let mut out = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let mut it = line.splitn(2, char::is_whitespace);
                let stamp = it.next().unwrap_or("");
                let label = it.next().unwrap_or("").trim().to_string();
                if let Some(ms) = parse_timestamp(stamp) {
                    out.push(Chapter { at_ms: ms, label });
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    Vec::new()
}

pub fn playlist_to_m3u(paths: &[String]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for p in paths {
        out.push_str("#EXTINF:-1,");
        out.push_str(&title_from_path(Path::new(p)));
        out.push('\n');
        out.push_str(p);
        out.push('\n');
    }
    out
}

pub fn parse_m3u(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        out.push(line.to_string());
    }
    out
}

/// True for HLS playlists, whose entries are segments of one stream rather
/// than separate media items. Expanding those would fill the queue with
/// two-second fragments, so they are opened as a single item instead.
pub fn is_hls_body(text: &str) -> bool {
    text.lines()
        .take(40)
        .any(|line| line.trim_start().starts_with("#EXT-X-"))
}

pub fn is_hls_url(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_lowercase();
    path.ends_with(".m3u8")
}

/// Reads a playlist served as text (`.m3u`, plain text list, or `.xspf`) and
/// returns absolute entries, resolving relative names against the URL that
/// served the file.
pub fn parse_playlist_body(text: &str, base: &str) -> Vec<String> {
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    let mut raw: Vec<String> = Vec::new();
    if trimmed.starts_with('<') || trimmed.to_lowercase().contains("<playlist") {
        // XSPF / ASX: the file references live in <location> or <ref href>.
        let lower = trimmed.to_lowercase();
        let mut rest = lower.as_str();
        while let Some(start) = rest.find("<location>") {
            let after = &lower[start + 10..];
            if let Some(end) = after.find("</location>") {
                let value = trimmed[start + 10..start + 10 + end].trim().to_string();
                if !value.is_empty() {
                    raw.push(value);
                }
                rest = &after[end + 11..];
            } else {
                break;
            }
        }
        if raw.is_empty() {
            let mut rest = lower.as_str();
            while let Some(start) = rest.find("href=\"") {
                let after = &lower[start + 6..];
                if let Some(end) = after.find('"') {
                    let value = trimmed[start + 6..start + 6 + end].trim().to_string();
                    if value.starts_with("http") || value.contains('.') {
                        raw.push(value);
                    }
                    rest = &after[end + 1..];
                } else {
                    break;
                }
            }
        }
    } else {
        raw = parse_m3u(trimmed);
    }
    let mut out = Vec::new();
    let (scheme, host_path) = match base.split_once("://") {
        Some((s, rest)) => (s.to_string(), rest.to_string()),
        None => (String::new(), base.to_string()),
    };
    let origin = if host_path.contains('/') {
        format!("{}://{}", scheme, &host_path[..host_path.rfind('/').unwrap_or(host_path.len())])
    } else {
        format!("{}://{}", scheme, host_path)
    };
    for entry in raw {
        let entry = entry.trim().to_string();
        if entry.is_empty() {
            continue;
        }
        if entry.contains("://") || entry.starts_with("file:") {
            out.push(entry);
        } else if entry.starts_with("//") {
            out.push(format!("{}:{}", scheme, entry));
        } else if entry.starts_with('/') && !scheme.is_empty() {
            out.push(format!("{}://{}/{}", scheme, host_path.split('/').next().unwrap_or(""), entry.trim_start_matches('/')));
        } else if !scheme.is_empty() {
            out.push(format!("{}/{}", origin, entry));
        } else {
            // A local playlist: relative entries sit beside the file.
            let parent = Path::new(base).parent().unwrap_or(Path::new("."));
            out.push(parent.join(entry).to_string_lossy().to_string());
        }
    }
    out
}

pub fn export_chapters(paths: &[(String, i64)]) -> String {
    let mut out = String::new();
    for (label, ms) in paths {
        out.push_str(&format!("{} {}\n", format_time(*ms), label));
    }
    out
}

// ---------------------------------------------------------------------------
// Stores
// ---------------------------------------------------------------------------

pub fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("tmp");
    match serde_json::to_string_pretty(value) {
        Ok(text) => {
            if fs::write(&tmp, text).is_ok() {
                fs::rename(&tmp, path).is_ok()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub struct Store {
    pub dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }
    pub fn settings(&self) -> Settings {
        let mut s: Settings = read_json(&self.dir.join("settings.json"));
        s.migrate();
        s
    }
    pub fn save_settings(&self, s: &Settings) {
        write_json(&self.dir.join("settings.json"), s);
    }
    pub fn library(&self) -> Library {
        read_json(&self.dir.join("library.json"))
    }
    pub fn save_library(&self, l: &Library) {
        write_json(&self.dir.join("library.json"), l);
    }
    pub fn playlists(&self) -> Vec<Playlist> {
        read_json(&self.dir.join("playlists.json"))
    }
    pub fn save_playlists(&self, p: &[Playlist]) {
        write_json(&self.dir.join("playlists.json"), p);
    }
    pub fn presets(&self) -> Vec<Preset> {
        read_json(&self.dir.join("presets.json"))
    }
    pub fn save_presets(&self, p: &[Preset]) {
        write_json(&self.dir.join("presets.json"), p);
    }
    pub fn profiles(&self) -> Vec<Profile> {
        read_json(&self.dir.join("profiles.json"))
    }
    pub fn save_profiles(&self, p: &[Profile]) {
        write_json(&self.dir.join("profiles.json"), p);
    }
    pub fn queue(&self) -> Queue {
        let q: Queue = read_json(&self.dir.join("queue.json"));
        if q.pinned { q } else { Queue::default() }
    }
    pub fn save_queue(&self, q: &Queue) {
        if q.pinned {
            write_json(&self.dir.join("queue.json"), q);
        }
    }
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}-{}", now_ms(), (now_ms() % 9973) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_end_prompt_setting_still_counts() {
        // Settings written before the end-of-item card was removed carry
        // `end_prompt_enabled`; it now means "play the queue automatically".
        let off: Settings = serde_json::from_str(r#"{"end_prompt_enabled": false}"#).unwrap();
        assert!(!off.auto_advance);
        let on: Settings = serde_json::from_str(r#"{"end_prompt_enabled": true}"#).unwrap();
        assert!(on.auto_advance);
        let fresh: Settings = serde_json::from_str("{}").unwrap();
        assert!(fresh.auto_advance, "a new install plays the queue on its own");
    }
}
