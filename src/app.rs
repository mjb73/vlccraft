//! Application state, actions and the update loop.

use crate::model::*;
use crate::skin::{self, Palette};
use crate::vlc::Engine;
use egui::{Context, Key, Modifiers, Pos2, TextureHandle};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Library,
    Player,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Home,
    Videos,
    Music,
    Streams,
    Playlists,
    Folder(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sort {
    Name,
    Added,
    Duration,
    UnwatchedFirst,
    WatchedFirst,
}

impl Sort {
    pub fn label(&self) -> &'static str {
        match self {
            Sort::Name => "Name",
            Sort::Added => "Recently added",
            Sort::Duration => "Duration",
            Sort::UnwatchedFirst => "Unwatched first",
            Sort::WatchedFirst => "Watched first",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Grid,
    List,
    Details,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    Subtitles,
    Audio,
    VideoTracks,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsCategory {
    General,
    Interface,
    Privacy,
    Controls,
    Playback,
    Video,
    Audio,
    Subtitles,
    Library,
    Network,
    Performance,
    Recording,
    Extensions,
}

impl SettingsCategory {
    pub const ALL: [SettingsCategory; 13] = [
        SettingsCategory::General,
        SettingsCategory::Interface,
        SettingsCategory::Privacy,
        SettingsCategory::Controls,
        SettingsCategory::Playback,
        SettingsCategory::Video,
        SettingsCategory::Audio,
        SettingsCategory::Subtitles,
        SettingsCategory::Library,
        SettingsCategory::Network,
        SettingsCategory::Performance,
        SettingsCategory::Recording,
        SettingsCategory::Extensions,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            SettingsCategory::General => "General",
            SettingsCategory::Interface => "Interface",
            SettingsCategory::Privacy => "Privacy",
            SettingsCategory::Controls => "Controls",
            SettingsCategory::Playback => "Playback",
            SettingsCategory::Video => "Video",
            SettingsCategory::Audio => "Audio",
            SettingsCategory::Subtitles => "Subtitles",
            SettingsCategory::Library => "Library",
            SettingsCategory::Network => "Network",
            SettingsCategory::Performance => "Performance",
            SettingsCategory::Recording => "Recording",
            SettingsCategory::Extensions => "Extensions",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Dialog {
    OpenFile,
    OpenFolder,
    OpenUrl,
    OpenDisc,
    Capture,
    NewPlaylist,
    AddToPlaylist(String),
    EditPlaylist(String),
    SubtitleBatch,
    SubtitleSearch,
    Convert,
    StreamOut,
    Presets,
    Hotkeys,
    Customise,
    Relink(String),
    About,
    Onboarding,
}

pub struct Now {
    pub path: String,
    pub title: String,
    pub show: String,
    pub duration_ms: i64,
    pub cues: Vec<Cue>,
    pub subtitle_path: Option<PathBuf>,
    pub subtitle_files: Vec<PathBuf>,
    pub chapters: Vec<Chapter>,
    pub style: SubtitleStyle,
    pub audio_tracks: Vec<(i32, String)>,
    pub video_tracks: Vec<(i32, String)>,
    pub audio_track_index: usize,
    pub video_track_index: usize,
    pub opened_at: f64,
    pub last_track_poll: f64,
    pub preset_name: Option<String>,
    pub is_audio: bool,
    pub error: Option<String>,
    pub rotate: i32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub zoom: f32,
    pub aspect: String,
    pub filter: String,
    pub equalizer: Option<String>,
}

impl Now {
    pub fn active_cue(&self, time_ms: i64) -> Option<&Cue> {
        let scaled = (time_ms as f32 / self.style.scale.max(0.25)) as i64 - self.style.delay_ms;
        self.cues
            .iter()
            .find(|c| scaled >= c.start_ms && scaled <= c.end_ms)
    }
}

#[derive(Clone, Debug)]
pub struct ResumePrompt {
    pub position_ms: i64,
    pub dismissed: bool,
}

pub struct Toast {
    pub text: String,
    pub until: f64,
}

pub struct ShotJob {
    pub path: PathBuf,
    pub requested: bool,
    pub frames: u64,
    pub need_frames: u64,
    pub saved: bool,
}

pub struct App {
    pub settings: Settings,
    pub store: Store,
    pub library: Library,
    pub playlists: Vec<Playlist>,
    pub presets: Vec<Preset>,
    pub profiles: Vec<Profile>,
    pub queue: Queue,

    pub engine: Option<Engine>,
    pub engine_error: Option<String>,
    pub libvlc_version: Option<String>,

    pub now: Option<Now>,
    pub view: View,
    pub section: Section,
    pub sort: Sort,
    pub view_mode: ViewMode,
    pub search: String,
    pub music_group: String,
    pub folder_open: Option<usize>,
    pub playlist_open: Option<String>,
    pub library_filter: String,

    pub sidebar_open: bool,
    pub sidebar_tab: usize,
    pub queue_open: bool,
    pub compact_library: bool,

    pub dialog: Option<Dialog>,
    pub dlg_text: String,
    pub dlg_text2: String,
    pub dlg_sel: Vec<String>,
    pub dlg_msg: Option<String>,
    pub browse_dir: String,

    pub popup: Option<Popup>,
    pub popup_anchor: Pos2,
    pub popup_rect: egui::Rect,
    pub popup_opened: f64,
    pub audio_popup_tab: usize,
    pub preset_save_name: String,
    pub subtitle_scope: String,
    pub video_rect: egui::Rect,
    pub drop_hover: bool,
    pub sleep_until: Option<f64>,
    pub sleep_minutes: u32,
    pub waveform_ui: bool,
    pub cue_edit: Option<(usize, String)>,
    pub toast: Option<Toast>,
    pub resume: Option<ResumePrompt>,

    pub fullscreen: bool,
    pub interface_hidden: bool,
    pub last_pointer_move: f64,
    pub last_interaction: f64,

    /// A convert or stream job, driven through the installed VLC executable.
    pub job: Option<crate::tools::SideJob>,
    pub convert_in: String,
    pub convert_out: String,
    pub convert_preset: usize,
    pub stream_in: String,
    pub stream_port: String,
    pub stream_transcode: bool,

    pub thumbs: Vec<(i64, TextureHandle)>,
    pub last_thumb_bucket: i64,
    pub scrub_to: Option<i64>,

    pub shot: Option<ShotJob>,
    pub frame_no: u64,
    pub time: f64,
    pub dirty: bool,
    pub last_save: f64,
    pub last_tick: f64,

    pub settings_cat: SettingsCategory,
    pub settings_search: String,
    pub settings_highlight: Option<String>,
    pub hotkey_search: String,
    pub rebinding: Option<String>,
    pub rename_bookmark: Option<(usize, String)>,
    pub drag_bookmark: Option<usize>,
    pub profile_name: String,
    pub export_path: String,
    pub import_path: String,
    pub pending_open: bool,
    pub start: Instant,

    pub mock: MockPlayer,
    pub last_end_check: f64,
    /// When the window geometry was last written down, so an ordinary frame
    /// does not keep touching the settings file.
    pub last_geom: f64,
    /// Watchdog for a player that reports "playing" while the picture sits
    /// frozen on its last frame: the last position read and when it last moved.
    pub last_time_seen: i64,
    pub last_time_change: f64,
    /// Playback has reached its end and has been dealt with once; the card or
    /// notice must never be re-created while the item sits at the end.
    pub ended_handled: bool,
    /// The item ran to its end and is waiting for the person to replay it.
    pub finished: bool,
    /// A seek asked for while the media was still being restarted after its
    /// end; applied as soon as the new playback reports time.
    pub pending_seek: Option<(i64, f64)>,
    pub window_title: String,
    pub toast_seq: u64,

    /// A link is being read in the background to see whether it is a playlist.
    /// Playback never waits for it: the check either fills the queue or the
    /// link opens as one stream.
    pub url_probe: Option<std::sync::mpsc::Receiver<UrlProbe>>,
    pub url_probe_url: String,
    pub url_probe_started: Instant,

    /// Which app opens video and music right now, and the handlers that were
    /// in place before VLCCraft asked for them.
    pub defaults: crate::defaults::Status,
    pub defaults_rec: crate::defaults::Record,
    pub defaults_checked: f64,
    pub defaults_msg: Option<String>,

    /// Files handed over by a second launch while this window is open.
    pub open_inbox: Option<std::sync::mpsc::Receiver<Vec<String>>>,
}

pub enum UrlProbe {
    Playlist(Vec<String>),
    Single,
    Failed(String),
}

/// Reads a link in a worker thread and decides whether it is a playlist.
/// Bounded in time and size; any failure simply means "open it as one stream".
fn fetch_playlist(url: &str) -> UrlProbe {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(6))
        .build();
    let response = match agent
        .get(url)
        .set("User-Agent", "VLCCraft/0.2 (skin over libvlc)")
        .set("Accept", "audio/x-mpegurl, application/xspf+xml, text/plain, */*")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, _)) => {
            return UrlProbe::Failed(format!("the link answered {code}"))
        }
        Err(_) => return UrlProbe::Failed("the link did not answer".into()),
    };
    let mut body = String::new();
    use std::io::Read;
    match response
        .into_reader()
        .take(512 * 1024)
        .read_to_string(&mut body)
    {
        Ok(_) => {}
        Err(_) => return UrlProbe::Single,
    }
    let head = body.trim_start_matches('\u{feff}').trim_start();
    let looks_like_playlist = head.starts_with("#EXTM3U")
        || head.to_lowercase().contains("<playlist")
        || head.to_lowercase().contains("<asx");
    if !looks_like_playlist || is_hls_body(&body) {
        return UrlProbe::Single;
    }
    let items = parse_playlist_body(&body, url);
    if items.len() > 1 {
        UrlProbe::Playlist(items)
    } else {
        UrlProbe::Single
    }
}

pub struct MockPlayer {
    pub base_ms: i64,
    pub playing_from: Option<Instant>,
    pub duration_ms: i64,
    pub playing: bool,
}

impl Default for MockPlayer {
    fn default() -> Self {
        Self {
            base_ms: 0,
            playing_from: None,
            duration_ms: 0,
            playing: false,
        }
    }
}

impl MockPlayer {
    fn time(&self) -> i64 {
        match self.playing_from {
            Some(t) if self.playing => {
                (self.base_ms + t.elapsed().as_millis() as i64).min(self.duration_ms.max(0))
            }
            _ => self.base_ms,
        }
    }
    fn pause(&mut self) {
        if self.playing {
            self.base_ms = self.time();
            self.playing = false;
            self.playing_from = None;
        }
    }
    fn play(&mut self) {
        if !self.playing {
            self.playing = true;
            self.playing_from = Some(Instant::now());
        }
    }
    fn seek(&mut self, ms: i64) {
        self.base_ms = ms.clamp(0, self.duration_ms.max(0));
        if self.playing {
            self.playing_from = Some(Instant::now());
        }
    }
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        screenshot: Option<PathBuf>,
        open_paths: Vec<String>,
        open_inbox: Option<std::sync::mpsc::Receiver<Vec<String>>>,
    ) -> Self {
        let shot = screenshot.map(|p| ShotJob {
            path: if p.is_absolute() {
                p
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p)
            },
            requested: false,
            frames: 0,
            need_frames: std::env::var("VLCCRAFT_SHOT_FRAMES")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(120)
                .max(10),
            saved: false,
        });
        let shot_mode = shot.is_some();
        let store = Store::new(data_dir());
        let mut settings = store.settings();
        let mut library = if shot_mode {
            // The screenshot run builds a self-contained demo library so it
            // never reads or writes the person's own collection.
            let mut lib = Library::default();
            let media = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/media");
            if media.exists() {
                lib.ensure_folder(&media);
            }
            lib
        } else {
            let mut lib = store.library();
            if lib.items.is_empty() {
                let media = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/media");
                if media.exists() {
                    lib.ensure_folder(&media);
                }
            }
            lib
        };

        let mut playlists = store.playlists();
        let presets = store.presets();
        let profiles = store.profiles();
        let queue = if shot_mode {
            Queue::default()
        } else {
            store.queue()
        };

        if settings.theme.is_empty() {
            settings.theme = "Midnight".into();
        }
        let p = skin::palette(&settings.theme, settings.accent);
        skin::install(&cc.egui_ctx, &p, settings.interface_scale);

        cc.egui_ctx.set_visuals(cc.egui_ctx.style().visuals.clone());

        let (engine, engine_error, version) = match Engine::new(shot_mode) {
            Ok(engine) => {
                let v = engine.version().to_string();
                (Some(engine), None, Some(v))
            }
            Err(e) => (None, Some(e), None),
        };

        let mut app = App {
            settings,
            store,
            library: std::mem::take(&mut library),
            playlists: std::mem::take(&mut playlists),
            presets,
            profiles,
            queue,
            engine,
            engine_error,
            libvlc_version: version,
            now: None,
            view: View::Library,
            section: Section::Home,
            sort: Sort::Added,
            view_mode: ViewMode::Grid,
            search: String::new(),
            music_group: "Albums".into(),
            folder_open: None,
            playlist_open: None,
            library_filter: "all".into(),
            sidebar_open: false,
            sidebar_tab: 0,
            queue_open: false,
            compact_library: false,
            dialog: None,
            dlg_text: String::new(),
            dlg_text2: String::new(),
            dlg_sel: Vec::new(),
            dlg_msg: None,
            browse_dir: crate::platform::browse_start().to_string_lossy().to_string(),
            popup: None,
            popup_anchor: Pos2::ZERO,
            popup_rect: egui::Rect::NOTHING,
            popup_opened: 0.0,
            audio_popup_tab: 0,
            preset_save_name: String::new(),
            subtitle_scope: "file".into(),
            video_rect: egui::Rect::NOTHING,
            drop_hover: false,
            sleep_until: None,
            sleep_minutes: 30,
            waveform_ui: false,
            cue_edit: None,
            toast: None,
            resume: None,
            fullscreen: false,
            interface_hidden: false,
            last_pointer_move: 0.0,
            last_interaction: 0.0,
            job: None,
            convert_in: String::new(),
            convert_out: String::new(),
            convert_preset: 0,
            stream_in: String::new(),
            stream_port: "8080".into(),
            stream_transcode: true,
            thumbs: Vec::new(),
            last_thumb_bucket: -1,
            scrub_to: None,
            shot,
            frame_no: 0,
            time: 0.0,
            dirty: false,
            last_save: 0.0,
            last_tick: 0.0,
            settings_cat: SettingsCategory::General,
            settings_search: String::new(),
            settings_highlight: None,
            hotkey_search: String::new(),
            rebinding: None,
            rename_bookmark: None,
            drag_bookmark: None,
            profile_name: String::new(),
            export_path: String::new(),
            import_path: String::new(),
            pending_open: false,
            start: Instant::now(),
            mock: MockPlayer::default(),
            last_end_check: 0.0,
            last_geom: 0.0,
            last_time_seen: -1,
            last_time_change: 0.0,
            ended_handled: false,
            finished: false,
            pending_seek: None,
            window_title: "VLCCraft".into(),
            toast_seq: 0,
            url_probe: None,
            url_probe_url: String::new(),
            url_probe_started: Instant::now(),
            defaults: crate::defaults::Status::default(),
            defaults_rec: crate::defaults::Record::default(),
            defaults_checked: -99.0,
            defaults_msg: None,
            open_inbox,
        };

        // A quiet, useful first run: onboarding once, library ready.
        app.sidebar_open = app.settings.sidebar_open;

        if let Some(shot) = &app.shot {
            let _ = shot;
            if let Some(first) = open_paths.first().cloned() {
                // A screenshot run can be pointed at a real file: useful when
                // checking how the player looks during and after playback.
                app.open_path(&first, true);
            } else {
                app.demo_setup();
            }
            // A believable state for the default-player card, without reading
            // or changing the real desktop the picture is taken on.
            app.defaults = crate::defaults::Status {
                available: true,
                video_name: "VLC".into(),
                audio_name: "VLC".into(),
                video_ours: false,
                audio_ours: false,
                video_any: false,
                audio_any: false,
            };
            if std::env::var("VLCCRAFT_SHOT_FULLSCREEN").is_ok() {
                app.fullscreen = true;
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
        } else {
            if !app.settings.onboarding_done {
                app.dialog = Some(Dialog::Onboarding);
            }
            app.save_library();
            app.refresh_defaults();
            // Files handed over by the desktop when this program is the
            // standard player: play the first, queue the rest.
            if !open_paths.is_empty() {
                app.open_from_shell(open_paths);
            }
        }
        app
    }

    fn demo_setup(&mut self) {
        let first = self
            .library
            .items
            .iter()
            .find(|i| i.title.contains("First Light"))
            .map(|i| i.path.clone());
        let second = self
            .library
            .items
            .iter()
            .find(|i| i.title.contains("Cold Front"))
            .map(|i| i.path.clone());
        let film = self
            .library
            .items
            .iter()
            .find(|i| i.title.contains("Desert Road"))
            .map(|i| i.path.clone());
        if let Some(first) = first.clone() {
            if let Some(item) = self.library.get_mut(&first) {
                item.position_ms = 3200;
                item.duration_ms = item.duration_ms.max(30_000);
                item.bookmarks.push(Bookmark {
                    at_ms: 12_000,
                    label: "The good shot".into(),
                    created: now_ms(),
                });
            }
        }
        if !self.playlists.iter().any(|p| p.name == "Weekend films") {
            if let Some(film) = &film {
                self.playlists.push(Playlist {
                    id: new_id("pl"),
                    name: "Weekend films".into(),
                    items: vec![film.clone()],
                    created: now_ms(),
                    modified: now_ms(),
                });
            }
        }
        self.queue.items = [first.clone(), second.clone(), film.clone()]
            .into_iter()
            .flatten()
            .collect();
        if std::env::var("VLCCRAFT_SHOT_QUEUE").as_deref() == Ok("one") {
            self.queue.items.truncate(1);
        }
        self.queue.index = 0;
        self.sidebar_open = true;
        self.sidebar_tab = 0;
        self.view = View::Player;
        if let Some(path) = first {
            self.open_path(&path, true);
            let at_end = std::env::var("VLCCRAFT_SHOT_END").is_ok();
            let seek = std::env::var("VLCCRAFT_SHOT_SEEK")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(if at_end { 28_800 } else { 3600 });
            if let Some(engine) = &self.engine {
                engine.set_time(seek);
            }
            // libvlc only takes a seek once the item is under way, so ask again
            // from the frame loop; a picture of the end of a video depends on it.
            self.pending_seek = Some((seek, self.time));
            if at_end {
                // The picture is taken after two seconds, long before a demo
                // clip would really reach its end: show the finished state.
                self.finished = true;
                self.ended_handled = true;
            }
            if let Some(now) = &mut self.now {
                now.style.delay_ms = 220;
                now.style.size_pct = 4.6;
                now.style.position_pct = 7.0;
            }
        }
        if let Ok(target) = std::env::var("VLCCRAFT_SHOT_VIEW") {
            match target.as_str() {
                "library" | "home" => {
                    self.view = View::Library;
                    self.section = Section::Home;
                }
                "videos" => {
                    self.view = View::Library;
                    self.section = Section::Videos;
                }
                "playlists" => {
                    self.view = View::Library;
                    self.section = Section::Playlists;
                }
                "settings" => {
                    self.view = View::Settings;
                }
                "queue" => {
                    self.queue_open = true;
                    self.sidebar_open = false;
                }
                "audiopopup" => {
                    self.popup = Some(Popup::Audio);
                    self.popup_anchor = Pos2::new(900.0, 520.0);
                }
                "subtitlepopup" => {
                    self.popup = Some(Popup::Subtitles);
                    self.popup_anchor = Pos2::new(900.0, 430.0);
                }
                "onboarding" => {
                    self.dialog = Some(Dialog::Onboarding);
                }
                "customise" => {
                    self.dialog = Some(Dialog::Customise);
                }
                "openfolder" => {
                    self.dlg_text.clear();
                    self.dialog = Some(Dialog::OpenFolder);
                }
                "subtitlebatch" => {
                    self.dialog = Some(Dialog::SubtitleBatch);
                }
                "convert" => {
                    self.open_convert_dialog();
                }
                "stream" => {
                    self.open_stream_dialog();
                }
                "hotkeys" => {
                    self.dialog = Some(Dialog::Hotkeys);
                }
                "presets" => {
                    self.dialog = Some(Dialog::Presets);
                }
                "music" => {
                    self.view = View::Library;
                    self.section = Section::Music;
                }
                _ => {}
            }
        }
        if let Ok(tab) = std::env::var("VLCCRAFT_SHOT_TAB") {
            if let Ok(index) = tab.parse::<usize>() {
                self.sidebar_tab = index.min(3);
                self.sidebar_open = true;
                self.view = View::Player;
            }
        }
        if let Ok(cat) = std::env::var("VLCCRAFT_SHOT_CAT") {
            self.view = View::Settings;
            self.settings_cat = match cat.as_str() {
                "interface" => SettingsCategory::Interface,
                "privacy" => SettingsCategory::Privacy,
                "controls" => SettingsCategory::Controls,
                "playback" => SettingsCategory::Playback,
                "video" => SettingsCategory::Video,
                "audio" => SettingsCategory::Audio,
                "subtitles" => SettingsCategory::Subtitles,
                "library" => SettingsCategory::Library,
                "network" => SettingsCategory::Network,
                "performance" => SettingsCategory::Performance,
                "recording" => SettingsCategory::Recording,
                "extensions" => SettingsCategory::Extensions,
                _ => SettingsCategory::General,
            };
            self.settings_search = std::env::var("VLCCRAFT_SHOT_SEARCH").unwrap_or_default();
            // For a picture of the surface the search would land on, with the
            // found row marked, without a pointer to click with.
            if let Ok(hit) = std::env::var("VLCCRAFT_SHOT_HIGHLIGHT") {
                self.settings_highlight = Some(hit);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save_library(&mut self) {
        if self.shot.is_some() {
            return;
        }
        self.store.save_library(&self.library);
    }

    pub fn save_all(&mut self) {
        if self.shot.is_some() {
            return;
        }
        self.store.save_settings(&self.settings);
        self.store.save_library(&self.library);
        self.store.save_playlists(&self.playlists);
        self.store.save_presets(&self.presets);
        self.store.save_profiles(&self.profiles);
        self.store.save_queue(&self.queue);
        self.dirty = false;
        self.last_save = self.time;
    }

    pub fn touch(&mut self) {
        self.dirty = true;
    }

    pub fn toast_now(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast {
            text: msg.into(),
            until: self.time + 3.6,
        });
        self.toast_seq += 1;
    }

    // -----------------------------------------------------------------------
    // Playback
    // -----------------------------------------------------------------------

    /// The player's own idea of the position, before the end is accounted for.
    fn raw_time_ms(&self) -> i64 {
        if let Some(engine) = &self.engine {
            if self.now.is_some() {
                engine.time_ms()
            } else {
                self.mock.time()
            }
        } else {
            self.mock.time()
        }
    }

    pub fn time_ms(&self) -> i64 {
        let raw = self.raw_time_ms();
        // At the end the player reports 0. Show the end of the item instead, so
        // a finished video reads as finished rather than as a restart.
        if self.at_end() {
            let d = self.duration_ms();
            if d > 0 {
                return d;
            }
        }
        raw
    }

    pub fn duration_ms(&self) -> i64 {
        if let Some(now) = &self.now {
            if let Some(engine) = &self.engine {
                let l = engine.length_ms();
                if l > 0 {
                    return l;
                }
            }
            if now.duration_ms > 0 {
                return now.duration_ms;
            }
        }
        0
    }

    pub fn is_playing(&self) -> bool {
        // A finished item is never playing. libvlc can sit in its last frame
        // for a moment after the end, and the round button must not keep
        // offering to pause something that has stopped.
        if self.finished {
            return false;
        }
        if let Some(engine) = &self.engine {
            if self.now.is_some() {
                return engine.is_playing();
            }
        }
        self.mock.playing
    }

    pub fn open_path(&mut self, path: &str, autoplay: bool) {
        if !path.contains("://") && !Path::new(path).exists() {
            self.dialog = Some(Dialog::Relink(path.to_string()));
            return;
        }
        let item = self.library.get(path).cloned();
        let title = item
            .as_ref()
            .map(|i| i.title.clone())
            .unwrap_or_else(|| title_from_path(Path::new(path)));
        let show = item
            .as_ref()
            .map(|i| i.show.clone())
            .unwrap_or_else(|| show_from_path(Path::new(path)));
        let duration = item.as_ref().map(|i| i.duration_ms).unwrap_or(0);
        let is_audio = item
            .as_ref()
            .map(|i| i.kind == "audio")
            .unwrap_or_else(|| {
                path.ends_with(".mp3") || path.ends_with(".flac") || path.ends_with(".ogg")
            });

        let mut style = self.settings.subtitle_style.clone();
        let mut preset_name = None;
        if let Some(item) = &item {
            if let Some(p) = self.preset_for(path, &item.show) {
                style.delay_ms = p.delay_ms;
                style.scale = p.scale;
                style.size_pct = p.size_pct;
                style.outline = p.outline;
                style.position_pct = p.position_pct;
                if !p.font.is_empty() {
                    style.font = p.font.clone();
                }
                preset_name = Some(p.name.clone());
            }
        }

        let mut subtitle_path = item.as_ref().and_then(|i| i.subtitle_path.clone()).map(PathBuf::from);
        let sub_match = find_subtitle(Path::new(path), &self.settings.subtitle_precision);
        if subtitle_path.as_ref().map(|p| !p.exists()).unwrap_or(false) {
            subtitle_path = None;
        }
        if subtitle_path.is_none() {
            subtitle_path = sub_match.chosen.clone();
        }
        let cues = subtitle_path.as_ref().map(|p| load_cues(p)).unwrap_or_default();
        let chapters = load_chapters(Path::new(path));

        if let Some(engine) = &mut self.engine {
            if let Err(e) = engine.open(path) {
                self.engine_error = Some(e);
            } else {
                engine.set_volume(self.settings.volume, self.settings.muted);
                if !self.settings.audio_output.is_empty() && self.settings.audio_output != "Automatic" {
                    let choice = self.settings.audio_output.clone();
                    engine.set_audio_choice(&choice);
                }
            }
        }
        self.mock.duration_ms = if duration > 0 { duration } else { 30_000 };
        self.mock.base_ms = 0;
        self.mock.playing_from = Some(Instant::now());
        self.mock.playing = autoplay && self.engine.is_none();

        let position = item.as_ref().map(|i| i.position_ms).unwrap_or(0);
        self.resume = if self.settings.resume_enabled
            && position > 5_000
            && duration > 0
            && position < duration - 5_000
        {
            Some(ResumePrompt {
                position_ms: position,
                dismissed: false,
            })
        } else {
            None
        };

        self.now = Some(Now {
            path: path.to_string(),
            title,
            show,
            duration_ms: if duration > 0 { duration } else { 30_000 },
            cues,
            subtitle_path,
            subtitle_files: sub_match.candidates,
            chapters,
            style,
            audio_tracks: vec![(1, "Audio 1 — stereo".into())],
            video_tracks: vec![(1, "Video 1".into())],
            audio_track_index: 0,
            video_track_index: 0,
            opened_at: self.time,
            last_track_poll: 0.0,
            preset_name,
            is_audio,
            error: None,
            rotate: 0,
            flip_h: false,
            flip_v: false,
            zoom: 1.0,
            aspect: "Auto".into(),
            filter: "None".into(),
            equalizer: None,
        });
        self.view = View::Player;
        self.ended_handled = false;
        self.finished = false;
        self.pending_seek = None;
        self.thumbs.clear();
        self.last_thumb_bucket = -1;
        self.last_end_check = 0.0;
        if let Some(now) = &self.now {
            self.window_title = format!("{} — VLCCraft", now.title);
        }
        if self.shot.is_none() {
            self.save_library();
        }
    }

    /// True when the current item has run to its end. libvlc then sits in its
    /// "Ended" state, where play and seek requests are ignored, so the only
    /// way back is to open the media again.
    pub fn at_end(&self) -> bool {
        if self.now.is_none() {
            return false;
        }
        if self.finished {
            return true;
        }
        if self.engine.as_ref().map(|e| e.ended()).unwrap_or(false) {
            return true;
        }
        // Position is the fallback for an engine that reports the last frame
        // without announcing the end: sitting on the last of the duration
        // while not playing is an ended item for every purpose here.
        let dur = self.duration_ms();
        if dur > 0 && self.raw_time_ms() >= dur - 120 && !self.is_playing() {
            return true;
        }
        // …and the same when the engine still claims to play but the picture
        // has stopped moving on its last frame.
        self.stalled_at_end()
    }

    /// The position is at the end and has not advanced for a moment, whatever
    /// the engine says about its state.
    fn stalled_at_end(&self) -> bool {
        let dur = self.duration_ms();
        dur > 0
            && self.raw_time_ms() >= dur - 1200
            && self.is_playing()
            && self.time - self.last_time_change > 0.7
    }

    pub fn toggle_play(&mut self) {
        if self.now.is_none() {
            if let Some(item) = self.library.items.first().cloned() {
                self.open_path(&item.path, true);
            }
            return;
        }
        if self.at_end() {
            self.replay();
            return;
        }
        if let Some(engine) = &self.engine {
            engine.toggle_pause();
            let label = if engine.is_playing() {
                "Playing"
            } else {
                "Paused"
            };
            self.toast_now(format!("{label} · {}", self.settings.hotkey("Play / pause")));
        } else {
            if self.mock.playing {
                self.mock.pause();
            } else {
                self.mock.play();
            }
            self.toast_now(if self.mock.playing { "Playing" } else { "Paused" });
        }
        self.last_interaction = self.time;
    }

    /// Plays the current item again from the start.
    pub fn replay(&mut self) {
        self.replay_at(None);
    }

    /// Restarts the item, optionally moving to a point once the new playback
    /// reports time. The media is reopened because libvlc will not leave the
    /// ended state on its own.
    pub fn replay_at(&mut self, at_ms: Option<i64>) {
        let Some(path) = self.now.as_ref().map(|n| n.path.clone()) else {
            return;
        };
        self.resume = None;
        self.finished = false;
        self.ended_handled = false;
        if let Some(engine) = &mut self.engine {
            engine.stop();
            if let Err(e) = engine.open(&path) {
                self.engine_error = Some(e);
            } else {
                engine.set_volume(self.settings.volume, self.settings.muted);
            }
        }
        self.mock.seek(0);
        self.mock.play();
        self.pending_seek = at_ms.map(|ms| (ms, self.time));
        if let Some(now) = &mut self.now {
            if let Some(item) = self.library.get_mut(&now.path) {
                item.position_ms = at_ms.unwrap_or(0);
            }
        }
        self.last_end_check = 0.0;
        self.last_interaction = self.time;
    }

    pub fn seek_to(&mut self, ms: i64) {
        let dur = self.duration_ms().max(1);
        let ms = ms.clamp(0, dur - 1);
        if self.at_end() {
            // Scrubbing a finished item restarts it at the chosen point.
            self.replay_at(Some(ms));
            return;
        }
        if let Some(engine) = &self.engine {
            engine.set_time(ms);
        }
        self.mock.seek(ms);
        if let Some(now) = &mut self.now {
            if let Some(item) = self.library.get_mut(&now.path) {
                item.position_ms = ms;
            }
        }
    }

    pub fn seek_by(&mut self, delta_ms: i64) {
        let t = self.time_ms() + delta_ms;
        self.seek_to(t);
        self.toast_now(format!(
            "{} · {}",
            format_time(self.time_ms()),
            if delta_ms > 0 { "later" } else { "earlier" }
        ));
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.settings.volume = volume.min(125);
        self.settings.muted = false;
        if let Some(engine) = &self.engine {
            engine.set_volume(self.settings.volume, false);
        }
        self.touch();
    }

    pub fn toggle_mute(&mut self) {
        self.settings.muted = !self.settings.muted;
        if let Some(engine) = &self.engine {
            engine.set_volume(self.settings.volume, self.settings.muted);
        }
        self.toast_now(if self.settings.muted {
            format!("Muted · {}", self.settings.hotkey("Mute"))
        } else {
            format!("Sound on · {}", self.settings.hotkey("Mute"))
        });
        self.touch();
    }

    pub fn enqueue(&mut self, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        let was_empty = self.queue.items.is_empty();
        let count = paths.len();
        let first = paths[0].clone();
        self.queue.items.extend(paths);
        if was_empty {
            self.queue.index = 0;
            self.open_path(&first, true);
        }
        self.toast_now(format!(
            "Added {count} item{} to the queue",
            if count == 1 { "" } else { "s" }
        ));
        self.touch();
    }

    pub fn play_next(&mut self) {
        if self.queue.items.is_empty() {
            return;
        }
        if self.queue.index + 1 < self.queue.items.len() {
            self.queue.index += 1;
            let path = self.queue.items[self.queue.index].clone();
            self.open_path(&path, true);
        }
        // Nothing after the last item: stay quiet. A finished queue is not an
        // event worth a message.
    }

    pub fn play_prev(&mut self) {
        if self.queue.items.is_empty() {
            return;
        }
        if self.queue.index > 0 {
            self.queue.index -= 1;
            let path = self.queue.items[self.queue.index].clone();
            self.open_path(&path, true);
        } else {
            self.seek_to(0);
        }
    }

    pub fn play_queue_index(&mut self, index: usize) {
        if let Some(path) = self.queue.items.get(index).cloned() {
            self.queue.index = index;
            self.open_path(&path, true);
        }
    }

    pub fn play_playlist(&mut self, id: &str) {
        if let Some(pl) = self.playlists.iter().find(|p| p.id == id).cloned() {
            if pl.items.is_empty() {
                self.toast_now("That playlist is empty");
                return;
            }
            self.queue.items = pl.items.clone();
            self.queue.index = 0;
            let first = pl.items[0].clone();
            self.toast_now(format!("Playing “{}”", pl.name));
            self.open_path(&first, true);
        }
    }

    pub fn mark_watched(&mut self, path: &str, watched: bool) {
        if let Some(item) = self.library.get_mut(path) {
            item.watched = watched;
            if watched {
                item.position_ms = item.duration_ms;
            }
        }
        self.touch();
        self.save_library();
        self.toast_now(if watched { "Marked watched" } else { "Marked unwatched" });
    }

    pub fn clear_resume(&mut self, path: &str) {
        if let Some(item) = self.library.get_mut(path) {
            item.position_ms = 0;
        }
        self.touch();
        self.save_library();
        self.toast_now("Resume point cleared");
    }

    // -----------------------------------------------------------------------
    // Subtitles / presets / bookmarks
    // -----------------------------------------------------------------------

    pub fn preset_for(&self, path: &str, show: &str) -> Option<Preset> {
        let folder = Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.presets
            .iter()
            .find(|p| {
                (p.match_kind == "folder" && folder.starts_with(&p.match_value))
                    || (p.match_kind == "pattern" && show.to_lowercase().contains(&p.match_value.to_lowercase()))
            })
            .cloned()
    }

    pub fn save_preset_for_current(&mut self, name: String) {
        let Some(now) = &self.now else { return };
        let folder = Path::new(&now.path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let preset = Preset {
            id: new_id("preset"),
            name: name.clone(),
            match_kind: "folder".into(),
            match_value: folder,
            delay_ms: now.style.delay_ms,
            scale: now.style.scale,
            size_pct: now.style.size_pct,
            outline: now.style.outline,
            position_pct: now.style.position_pct,
            font: now.style.font.clone(),
            track_language: self.settings.subtitle_default_language.clone(),
            audio_language: "Any".into(),
            filter: "None".into(),
            created: now_ms(),
        };
        self.presets.retain(|p| p.match_value != preset.match_value || p.match_kind != "folder");
        self.presets.push(preset);
        self.toast_now(format!("Preset “{name}” saved for this show"));
        self.touch();
        self.save_all();
    }

    pub fn select_subtitle(&mut self, path: Option<PathBuf>) {
        let Some(now) = &mut self.now else { return };
        now.subtitle_path = path.clone();
        now.cues = path.as_ref().map(|p| load_cues(p)).unwrap_or_default();
        if let Some(item) = self.library.get_mut(&now.path) {
            item.subtitle_path = path.map(|p| p.to_string_lossy().to_string());
        }
        self.touch();
        self.save_library();
    }

    pub fn add_bookmark(&mut self) {
        let at = self.time_ms();
        let Some(now) = &self.now else { return };
        let path = now.path.clone();
        let label = format!("Bookmark {}", format_time(at));
        let Some(item) = self.library.get_mut(&path) else { return };
        if item.bookmarks.iter().any(|b| (b.at_ms - at).abs() < 1200) {
            self.toast_now("A bookmark is already here");
            return;
        }
        item.bookmarks.push(Bookmark {
            at_ms: at,
            label,
            created: now_ms(),
        });
        item.bookmarks.sort_by_key(|b| b.at_ms);
        let index = item
            .bookmarks
            .iter()
            .position(|b| (b.at_ms - at).abs() < 1)
            .unwrap_or(0);
        self.rename_bookmark = Some((index, item.bookmarks[index].label.clone()));
        self.sidebar_open = true;
        self.sidebar_tab = 3;
        self.toast_now(format!("Bookmark added · {}", self.settings.hotkey("Add bookmark")));
        self.touch();
        self.save_library();
    }

    pub fn delete_bookmark(&mut self, index: usize) {
        let Some(now) = &self.now else { return };
        let path = now.path.clone();
        if let Some(item) = self.library.get_mut(&path) {
            if index < item.bookmarks.len() {
                item.bookmarks.remove(index);
            }
        }
        self.rename_bookmark = None;
        self.touch();
        self.save_library();
        self.toast_now("Bookmark removed");
    }

    pub fn nudge_bookmark(&mut self, index: usize, delta_ms: i64) {
        let Some(now) = &self.now else { return };
        let path = now.path.clone();
        if let Some(item) = self.library.get_mut(&path) {
            if let Some(b) = item.bookmarks.get_mut(index) {
                b.at_ms = (b.at_ms + delta_ms).max(0);
            }
            item.bookmarks.sort_by_key(|b| b.at_ms);
        }
        self.touch();
    }

    pub fn export_bookmarks(&mut self) {
        let Some(now) = &self.now else { return };
        let path = Path::new(&now.path);
        let Some(item) = self.library.get(&now.path) else { return };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let target = path.with_file_name(format!("{stem}.chapters.txt"));
        let rows: Vec<(String, i64)> = item
            .bookmarks
            .iter()
            .map(|b| (b.label.clone(), b.at_ms))
            .collect();
        let text = export_chapters(&rows);
        if std::fs::write(&target, text).is_ok() {
            self.toast_now(format!("Chapters written to {}", target.file_name().unwrap_or_default().to_string_lossy()));
        } else {
            self.toast_now("Could not write the chapter file");
        }
    }

    // -----------------------------------------------------------------------
    // Files and folders
    // -----------------------------------------------------------------------

    pub fn open_file_dialog(&mut self) {
        self.dlg_text = String::new();
        self.dlg_msg = None;
        self.dialog = Some(Dialog::OpenFile);
    }

    /// Convert and Stream-out are their own commands with one job each, not
    /// variants hidden inside "Open". Both are prefilled with what is playing.
    pub fn open_convert_dialog(&mut self) {
        let source = self
            .now
            .as_ref()
            .map(|n| n.path.clone())
            .or_else(|| self.queue.items.get(self.queue.index).cloned())
            .unwrap_or_default();
        self.convert_in = source.clone();
        self.convert_out = crate::model::suggested_convert_path(
            &source,
            crate::tools::conversion_presets()
                .get(self.convert_preset)
                .map(|p| p.ext)
                .unwrap_or("mp4"),
        );
        self.dlg_msg = None;
        self.dialog = Some(Dialog::Convert);
    }

    pub fn open_stream_dialog(&mut self) {
        let source = self
            .now
            .as_ref()
            .map(|n| n.path.clone())
            .or_else(|| self.queue.items.get(self.queue.index).cloned())
            .unwrap_or_default();
        self.stream_in = source;
        self.dlg_msg = None;
        self.dialog = Some(Dialog::StreamOut);
    }

    /// Watch the child we started. Finishing or failing is reported in the
    /// same place the job was started, never as a modal over the picture.
    pub fn poll_job(&mut self) {
        let Some(job) = &mut self.job else { return };
        let outcome = job.child.try_wait();
        match outcome {
            Ok(Some(status)) => {
                let job = self.job.take().unwrap();
                let ok = status.success();
                if job.kind == crate::tools::JobKind::Convert {
                    if ok {
                        self.toast_now(format!("Converted · {}", job.label.trim_start_matches("Converting to ")));
                    } else {
                        self.toast_now("VLC could not finish the conversion — the original is untouched");
                    }
                } else {
                    self.toast_now("Stream stopped");
                }
            }
            Ok(None) => {}
            Err(e) => {
                self.job.take();
                self.toast_now(format!("The job ended unexpectedly: {e}"));
            }
        }
    }

    pub fn stop_job(&mut self) {
        if let Some(job) = self.job.take() {
            job.stop();
            self.toast_now("Stopped — the original files are untouched");
        }
    }

    pub fn add_folder(&mut self, folder: &str) {
        let path = PathBuf::from(folder);
        if !path.is_dir() {
            self.dlg_msg = Some("That folder does not exist.".into());
            return;
        }
        let added = self.library.ensure_folder(&path);
        self.toast_now(format!(
            "Indexed {added} new item{}",
            if added == 1 { "" } else { "s" }
        ));
        self.section = Section::Videos;
        self.dialog = None;
        self.touch();
        self.save_library();
    }

    pub fn open_media_file(&mut self, path: &str) {
        let p = PathBuf::from(path);
        if !p.exists() {
            self.dlg_msg = Some("That file does not exist.".into());
            return;
        }
        if self.library.get(path).is_none() {
            let kind = kind_for(&p);
            self.library.items.push(item_from_path(&p, kind));
            self.library
                .items
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        self.dialog = None;
        self.queue.items.push(path.to_string());
        if self.queue.items.len() == 1 {
            self.queue.index = 0;
        } else {
            self.queue.index = self.queue.items.len() - 1;
        }
        self.open_path(path, true);
        self.touch();
        self.save_library();
    }

    /// A file, folder or link handed over by the desktop or by a second
    /// launch: play the first thing, keep the rest as the queue. Nothing is
    /// interrupted — the queue simply grows.
    pub fn open_from_shell(&mut self, paths: Vec<String>) {
        let mut rest = paths.into_iter();
        let Some(first) = rest.next() else { return };
        let first = canonical_media(&first);
        if first.is_empty() {
            return;
        }
        if first.starts_with("http://") || first.starts_with("https://") || first.starts_with("rtsp://") {
            self.open_url(&first);
            return;
        }
        if Path::new(&first).is_dir() {
            self.add_folder(&first);
            self.view = View::Library;
            return;
        }
        self.open_media_file(&first);
        let mut extra = 0usize;
        for raw in rest {
            let path = canonical_media(&raw);
            if path.is_empty() || !Path::new(&path).exists() {
                continue;
            }
            if self.library.get(&path).is_none() {
                let kind = kind_for(Path::new(&path));
                self.library.items.push(item_from_path(Path::new(&path), kind));
                self.library
                    .items
                    .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            if !self.queue.items.contains(&path) {
                self.queue.items.push(path);
                extra += 1;
            }
        }
        if extra > 0 {
            self.queue_open = true;
            self.toast_now(format!(
                "{extra} more file{} waiting in the queue",
                if extra == 1 { "" } else { "s" }
            ));
            // Let the first picture start before the queue is written out.
            let _ = self.save_library();
            self.touch();
        }
    }

    /// Reads the desktop's current file handlers. Cheap enough to repeat when
    /// the setting is on screen, never run in a screenshot session.
    pub fn refresh_defaults(&mut self) {
        if self.shot.is_some() {
            return;
        }
        self.defaults = crate::defaults::status();
        self.defaults_rec = crate::defaults::load_record();
        self.defaults_checked = self.time;
    }

    pub fn open_url(&mut self, url: &str) {
        let url = url.trim().replace(' ', "%20");
        if url.is_empty() {
            self.dlg_msg = Some("Paste a link first.".into());
            return;
        }
        // Local playlist files expand immediately; a remote link is read in
        // the background so the window never waits on the network.
        let mut items = Vec::new();
        if let Some(rest) = url.strip_prefix("file://") {
            if let Ok(text) = std::fs::read_to_string(rest) {
                items = parse_playlist_body(&text, rest);
            }
        } else if !url.contains("://") && Path::new(&url).exists() {
            if let Ok(text) = std::fs::read_to_string(&url) {
                items = parse_playlist_body(&text, &url);
            }
        }
        if !items.is_empty() {
            self.library.streams.push(url.clone());
            self.queue.items.extend(items.clone());
            if self.queue.items.len() == items.len() {
                self.queue.index = 0;
            }
            self.open_path(&items[0], true);
            self.dialog = None;
            self.toast_now(format!("Playlist added as {} queue items", items.len()));
            self.touch();
            self.save_library();
            return;
        }
        let remote = url.starts_with("http://") || url.starts_with("https://");
        if remote && !is_hls_url(&url) {
            let (tx, rx) = std::sync::mpsc::channel();
            let target = url.clone();
            std::thread::spawn(move || {
                let result = fetch_playlist(&target);
                let _ = tx.send(result);
            });
            self.url_probe = Some(rx);
            self.url_probe_url = url.clone();
            self.url_probe_started = Instant::now();
            self.library.streams.push(url.clone());
            self.dialog = None;
            self.toast_now("Checking the link for a playlist…");
            self.touch();
            self.save_library();
            return;
        }
        self.open_single_stream(&url);
    }

    fn open_single_stream(&mut self, url: &str) {
        self.library.streams.push(url.to_string());
        self.queue.items.push(url.to_string());
        if self.queue.items.len() == 1 {
            self.queue.index = 0;
        }
        self.open_path(url, true);
        self.dialog = None;
        self.toast_now("Stream added to the queue");
        self.touch();
        self.save_library();
    }

    /// Choices for the output picker: real devices when the current output
    /// reports them, output modules before playback starts. Values carry a
    /// `device:` / `module:` prefix so a saved choice is unambiguous.
    pub fn audio_choices(&self) -> Vec<(String, String)> {
        let mut out = vec![(
            "Automatic (system default)".to_string(),
            "Automatic".to_string(),
        )];
        if let Some(engine) = &self.engine {
            let devices = engine.audio_devices();
            if !devices.is_empty() {
                for (id, desc) in devices {
                    let label = if desc.trim().is_empty() {
                        id.clone()
                    } else {
                        desc
                    };
                    out.push((label, format!("device:{id}")));
                }
            } else {
                for module in engine.audio_outputs() {
                    out.push((
                        crate::sidebar::pretty_aout(module),
                        format!("module:{module}"),
                    ));
                }
            }
        }
        out
    }

    pub fn set_audio_choice(&mut self, value: &str) {
        let label = self
            .audio_choices()
            .into_iter()
            .find(|(_, v)| v == value)
            .map(|(l, _)| l)
            .unwrap_or_else(|| "system default".into());
        self.settings.audio_output = value.to_string();
        if let Some(engine) = &self.engine {
            engine.set_audio_choice(value);
        }
        self.toast_now(format!("Audio output: {label}"));
        self.touch();
    }

    fn poll_url_probe(&mut self) {
        let Some(rx) = &self.url_probe else { return };
        let outcome = match rx.try_recv() {
            Ok(v) => Some(v),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(UrlProbe::Failed("the link could not be read".into()))
            }
        };
        let Some(outcome) = outcome else {
            if self.url_probe_started.elapsed().as_secs() > 12 {
                let url = self.url_probe_url.clone();
                self.url_probe = None;
                self.open_single_stream(&url);
                self.toast_now("The link took too long to answer — opened as one stream");
            }
            return;
        };
        let url = self.url_probe_url.clone();
        self.url_probe = None;
        match outcome {
            UrlProbe::Playlist(items) if items.len() > 1 => {
                self.queue.items.extend(items.iter().cloned());
                if self.queue.items.len() == items.len() {
                    self.queue.index = 0;
                }
                self.open_path(&items[0], true);
                self.toast_now(format!("Playlist link added as {} queue items", items.len()));
            }
            UrlProbe::Playlist(items) if items.len() == 1 => {
                self.open_single_stream(&items[0]);
                return;
            }
            UrlProbe::Failed(reason) => {
                self.open_single_stream(&url);
                self.toast_now(format!("No playlist found ({reason}) — opened as one stream"));
                return;
            }
            _ => {
                self.open_single_stream(&url);
                return;
            }
        }
        self.touch();
        self.save_library();
    }

    pub fn relink(&mut self, old_path: &str, new_path: &str) {
        let new = PathBuf::from(new_path);
        if !new.exists() {
            self.dlg_msg = Some("That file is not there.".into());
            return;
        }
        let old_item = self.library.get(old_path).cloned();
        if let Some(mut item) = old_item {
            item.path = new.to_string_lossy().to_string();
            item.title = title_from_path(&new);
            item.show = show_from_path(&new);
            item.size = std::fs::metadata(&new).map(|m| m.len()).unwrap_or(item.size);
            self.library.items.retain(|i| i.path != old_path);
            self.library.items.push(item);
        }
        for pl in &mut self.playlists {
            for p in &mut pl.items {
                if p == old_path {
                    *p = new_path.to_string();
                }
            }
        }
        for q in &mut self.queue.items {
            if q == old_path {
                *q = new_path.to_string();
            }
        }
        self.dialog = None;
        self.toast_now("Relinked — position and subtitles kept");
        self.touch();
        self.save_all();
    }

    // -----------------------------------------------------------------------
    // Per-frame work
    // -----------------------------------------------------------------------

    pub fn tick(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        self.time = now;
        self.frame_no += 1;
        self.poll_job();

        // A picture run must not inherit a window the person left in full
        // screen: eframe restores that state before the first frame, and the
        // check would photograph a bare picture with no chrome at all.
        if self.shot.is_some()
            && self.frame_no <= 3
            && std::env::var("VLCCRAFT_SHOT_FULLSCREEN").is_err()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                1440.0, 900.0,
            )));
        }

        // Progress watchdog. Some files leave libvlc frozen on the last frame
        // while it still answers "playing", so trust the position instead: when
        // the reported time stops moving near the end, the item is over and the
        // round button must offer play again.
        let reported = self.raw_time_ms();
        if reported != self.last_time_seen {
            self.last_time_seen = reported;
            self.last_time_change = now;
        }

        let pointer_moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        if pointer_moved {
            self.last_pointer_move = now;
            self.last_interaction = now;
        }
        if ctx.input(|i| i.pointer.any_click()) {
            self.last_interaction = now;
        }

        if let Some(toast) = &self.toast {
            if now > toast.until {
                self.toast = None;
            }
        }

        if let Some(now_playing) = &mut self.now {
            if now - now_playing.last_track_poll > 1.2 {
                now_playing.last_track_poll = now;
                if let Some(engine) = &self.engine {
                    let audio = engine.audio_tracks();
                    if !audio.is_empty() {
                        now_playing.audio_tracks = audio.iter().map(|t| (t.id, t.name.clone())).collect();
                    }
                    let video = engine.video_tracks();
                    if !video.is_empty() {
                        now_playing.video_tracks = video.iter().map(|t| (t.id, t.name.clone())).collect();
                    }
                }
            }
        }

        // Watch state + resume position, twice a second.
        if now - self.last_tick > 0.5 {
            self.last_tick = now;
            if let Some(now_playing) = &self.now {
                let t = self.time_ms();
                let dur = self.duration_ms();
                if let Some(item) = self.library.get_mut(&now_playing.path) {
                    if t > 0 && dur > 0 {
                        item.position_ms = t;
                        if dur > 0 && t as f32 / dur as f32 > self.settings.watched_threshold as f32 / 100.0
                        {
                            item.watched = true;
                        }
                    }
                }
                self.dirty = true;
            }
            if self.dirty && now - self.last_save > 4.0 {
                self.save_all();
            }
        }

        // Thumbnails for the seek bar: one small frame every two seconds,
        // bounded to 60 textures. A slow decoder simply skips them.
        if self.settings.show_thumbnails && self.now.is_some() {
            let bucket = self.time_ms() / 2000;
            if bucket != self.last_thumb_bucket && self.is_playing() {
                self.last_thumb_bucket = bucket;
                self.capture_thumbnail(ctx, bucket);
            }
        }

        // End of item. Deliberately silent: nothing is drawn, no toast, no
        // countdown, no "up next" card. The queue simply moves on when there
        // is a next item and automatic playback is on; otherwise the item
        // rests at its end and the round button shows the play glyph again.
        // Handled once per playback: libvlc keeps reporting "ended" after the
        // item has finished, so without this flag the work below would repeat.
        if now - self.last_end_check > 0.25 {
            self.last_end_check = now;
            let ended = self.stalled_at_end()
                || self
                    .engine
                    .as_ref()
                    .map(|e| e.ended())
                    .unwrap_or_else(|| {
                    let t = self.time_ms();
                    let d = self.duration_ms();
                    d > 0 && t >= d - 300
                });
            if ended && !self.ended_handled && self.now.is_some() {
                self.ended_handled = true;
                let current = self.now.as_ref().map(|n| n.path.clone()).unwrap_or_default();
                let next = self
                    .queue
                    .items
                    .get(self.queue.index + 1)
                    .cloned()
                    .unwrap_or_default();
                let has_next = !next.is_empty() && next != current;
                if has_next && self.settings.auto_advance {
                    self.play_next();
                } else {
                    self.finished = true;
                }
            }
        }

        // A seek asked for while the item was being restarted after its end
        // waits for the new playback, then moves to the requested point.
        if let Some((ms, at)) = self.pending_seek {
            let ready = self
                .engine
                .as_ref()
                .map(|e| e.is_playing() || e.time_ms() > 0)
                .unwrap_or(true);
            if ready {
                if let Some(engine) = &self.engine {
                    engine.set_time(ms);
                }
                self.mock.seek(ms);
                self.pending_seek = None;
            } else if self.time - at > 4.0 {
                self.pending_seek = None;
            }
        }

        if self.resume.as_ref().map(|r| r.dismissed).unwrap_or(false) {
            self.resume = None;
        }

        self.handle_shortcuts(ctx);
    }

    fn capture_thumbnail(&mut self, ctx: &Context, bucket: i64) {
        // A small copy of the live frame keeps seek previews bounded and never
        // blocks seeking; no frame means no preview, which is fine.
        let image = match &self.engine {
            Some(engine) => engine.snapshot(168),
            None => None,
        };
        if let Some(image) = image {
            let tex = ctx.load_texture(
                format!("thumb-{bucket}-{}", self.frame_no),
                image,
                egui::TextureOptions::LINEAR,
            );
            self.thumbs.push((bucket * 2000, tex));
            if self.thumbs.len() > 60 {
                self.thumbs.remove(0);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Keyboard
    // -----------------------------------------------------------------------

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let wants_typing = ctx.wants_keyboard_input();

        // Esc closes the innermost surface, always.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.popup.is_some() {
                self.popup = None;
            } else if self.dialog.is_some() {
                self.dialog = None;
            } else if self.rename_bookmark.is_some() {
                self.rename_bookmark = None;
            } else if self.interface_hidden {
                self.interface_hidden = false;
            } else if self.fullscreen {
                self.set_fullscreen(ctx, false);
            } else if self.view == View::Player {
                self.view = View::Library;
            }
            return;
        }

        if self.rebinding.is_some() {
            // Next chord is captured by the Controls surface itself.
            return;
        }
        if wants_typing {
            return;
        }
        if self.dialog.is_some() {
            return;
        }

        let hotkeys = self.settings.hotkeys.clone();
        let actions: Vec<(String, String)> = vec![
            ("Play / pause".into(), hotkeys.get("Play / pause").cloned().unwrap_or_default()),
            ("Seek back 5 s".into(), hotkeys.get("Seek back 5 s").cloned().unwrap_or_default()),
            ("Seek forward 5 s".into(), hotkeys.get("Seek forward 5 s").cloned().unwrap_or_default()),
            ("Seek back 30 s".into(), hotkeys.get("Seek back 30 s").cloned().unwrap_or_default()),
            ("Seek forward 30 s".into(), hotkeys.get("Seek forward 30 s").cloned().unwrap_or_default()),
            ("Volume up".into(), hotkeys.get("Volume up").cloned().unwrap_or_default()),
            ("Volume down".into(), hotkeys.get("Volume down").cloned().unwrap_or_default()),
            ("Mute".into(), hotkeys.get("Mute").cloned().unwrap_or_default()),
            ("Full screen".into(), hotkeys.get("Full screen").cloned().unwrap_or_default()),
            ("Subtitles sidebar".into(), hotkeys.get("Subtitles sidebar").cloned().unwrap_or_default()),
            ("Queue".into(), hotkeys.get("Queue").cloned().unwrap_or_default()),
            ("Media library".into(), hotkeys.get("Media library").cloned().unwrap_or_default()),
            ("Add bookmark".into(), hotkeys.get("Add bookmark").cloned().unwrap_or_default()),
            ("Open file".into(), hotkeys.get("Open file").cloned().unwrap_or_default()),
            ("Open network stream".into(), hotkeys.get("Open network stream").cloned().unwrap_or_default()),
            ("Subtitle delay −50 ms".into(), hotkeys.get("Subtitle delay −50 ms").cloned().unwrap_or_default()),
            ("Subtitle delay +50 ms".into(), hotkeys.get("Subtitle delay +50 ms").cloned().unwrap_or_default()),
            ("Next item".into(), hotkeys.get("Next item").cloned().unwrap_or_default()),
            ("Previous item".into(), hotkeys.get("Previous item").cloned().unwrap_or_default()),
            ("Settings".into(), hotkeys.get("Settings").cloned().unwrap_or_default()),
            ("Hide the interface".into(), hotkeys.get("Hide the interface").cloned().unwrap_or_default()),
        ];

        for (action, combo) in actions {
            if !combo_pressed(ctx, &combo) {
                continue;
            }
            match action.as_str() {
                "Play / pause" => self.toggle_play(),
                "Seek back 5 s" => self.seek_by(-5_000),
                "Seek forward 5 s" => self.seek_by(5_000),
                "Seek back 30 s" => self.seek_by(-30_000),
                "Seek forward 30 s" => self.seek_by(30_000),
                "Volume up" => {
                    let v = (self.settings.volume + 5).min(125);
                    self.set_volume(v);
                }
                "Volume down" => {
                    let v = self.settings.volume.saturating_sub(5);
                    self.set_volume(v);
                }
                "Mute" => self.toggle_mute(),
                "Full screen" => self.set_fullscreen(ctx, !self.fullscreen),
                "Subtitles sidebar" => {
                    self.sidebar_open = !self.sidebar_open;
                    self.sidebar_tab = 0;
                    self.settings.sidebar_open = self.sidebar_open;
                    self.settings.sidebar_tab = self.sidebar_tab;
                    self.touch();
                }
                "Queue" => {
                    self.queue_open = !self.queue_open;
                    self.toast_now(if self.queue_open { "Queue open" } else { "Queue hidden" });
                }
                "Media library" => {
                    self.view = if self.view == View::Library {
                        View::Player
                    } else {
                        View::Library
                    };
                }
                "Add bookmark" => self.add_bookmark(),
                "Open file" => self.open_file_dialog(),
                "Open network stream" => {
                    self.dlg_text.clear();
                    self.dialog = Some(Dialog::OpenUrl);
                }
                "Subtitle delay −50 ms" => {
                    if let Some(now) = &mut self.now {
                        now.style.delay_ms -= 50;
                    }
                    self.touch();
                    self.toast_now("Subtitle delay −50 ms");
                }
                "Subtitle delay +50 ms" => {
                    if let Some(now) = &mut self.now {
                        now.style.delay_ms += 50;
                    }
                    self.touch();
                    self.toast_now("Subtitle delay +50 ms");
                }
                "Next item" => self.play_next(),
                "Previous item" => self.play_prev(),
                "Settings" => self.view = View::Settings,
                "Hide the interface" => {
                    self.interface_hidden = !self.interface_hidden;
                    self.toast_now(if self.interface_hidden {
                        "Interface hidden — move the pointer to bring it back"
                    } else {
                        "Interface shown"
                    });
                }
                _ => {}
            }
            return;
        }
    }

    pub fn set_fullscreen(&mut self, ctx: &Context, on: bool) {
        self.fullscreen = on;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
        self.toast_now(format!(
            "{} · {}",
            if on { "Full screen" } else { "Windowed" },
            self.settings.hotkey("Full screen")
        ));
    }

    // -----------------------------------------------------------------------
    // Screenshot
    // -----------------------------------------------------------------------

    fn drive_screenshot(&mut self, ctx: &Context) {
        let Some(job) = &mut self.shot else { return };
        job.frames += 1;
        if std::env::var("VLCCRAFT_SHOT_TRACE").is_ok() && job.frames % 100 == 0 {
            eprintln!("shot frame {} saved={} requested={}", job.frames, job.saved, job.requested);
        }
        if !job.requested && job.frames >= job.need_frames {
            job.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(0u8)));
        }
        if job.requested && !job.saved {
            let images: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Screenshot { image, .. } => Some(image.clone()),
                        _ => None,
                    })
                    .collect()
            });
            if let Some(image) = images.last() {
                let [w, h] = image.size;
                let mut rgba = Vec::with_capacity(w * h * 4);
                for px in &image.pixels {
                    rgba.extend_from_slice(&[px.r(), px.g(), px.b(), 255]);
                }
                if let Some(parent) = job.path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let result = image::save_buffer(
                    &job.path,
                    &rgba,
                    w as u32,
                    h as u32,
                    image::ColorType::Rgba8,
                );
                match result {
                    Ok(()) => {
                        println!("screenshot saved: {}", job.path.display());
                        job.saved = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Err(e) => {
                        eprintln!("screenshot failed: {e}");
                        job.saved = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
        ctx.request_repaint();
    }

    pub fn palette(&self) -> Palette {
        skin::palette(&self.settings.theme, self.settings.accent)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let p = self.palette();
        skin::install(ctx, &p, self.settings.interface_scale);
        self.tick(ctx);
        self.poll_url_probe();

        // Remember the window the person leaves, so the next start opens the
        // size it was left at. A resize arrives in dozens of small steps, so
        // the value is written once it has settled; a pending one asks for one
        // more frame, which is enough to finish a drag with no other input.
        if self.settings.remember_window && self.shot.is_none() && !self.fullscreen {
            // The viewport's own inner_rect is not filled on every backend, so
            // the content rectangle egui already tracks is the source of truth.
            let rect = ctx.screen_rect();
            let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
            if rect.width() > 1.0 {
                let size = [rect.width().round(), rect.height().round()];
                let size_differs = !maximized
                    && size[0] >= 400.0
                    && size[1] >= 300.0
                    && self
                        .settings
                        .window_size
                        .map(|s| (s[0] - size[0]).abs() > 1.0 || (s[1] - size[1]).abs() > 1.0)
                        .unwrap_or(true);
                // While maximised the inner size is the whole screen, so only
                // the flag is stored; the size to come back to is the last
                // windowed one.
                let max_differs = maximized != self.settings.window_maximized;
                if size_differs || max_differs {
                    if self.time - self.last_geom > 0.6 {
                        if max_differs {
                            self.settings.window_maximized = maximized;
                        }
                        if size_differs {
                            self.settings.window_size = Some(size);
                        }
                        self.last_geom = self.time;
                        self.touch();
                    } else {
                        ctx.request_repaint_after(std::time::Duration::from_millis(300));
                    }
                }
            }
        }

        // A second launch hands its files here instead of opening a window.
        let mut handed_over: Vec<Vec<String>> = Vec::new();
        if let Some(rx) = &self.open_inbox {
            while let Ok(paths) = rx.try_recv() {
                handed_over.push(paths);
            }
        }
        for paths in handed_over {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.open_from_shell(paths);
        }

        // Keep the default-player card honest while the person is looking at it.
        if self.view == View::Settings
            && self.settings_cat == SettingsCategory::General
            && self.time - self.defaults_checked > 2.5
        {
            self.refresh_defaults();
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title.clone()));

        if self.interface_hidden && self.view == View::Player {
            // Only the video remains; any pointer movement reveals the chrome.
            self.player_video_only(ctx, &p);
        } else {
            self.top_chrome(ctx, &p);
            match self.view {
                View::Library => self.library_view(ctx, &p),
                View::Player => self.player_view(ctx, &p),
                View::Settings => self.settings_view(ctx, &p),
            }
        }

        self.status_overlays(ctx, &p);
        self.dialogs(ctx, &p);
        self.drive_screenshot(ctx);

        if self.dirty {
            self.save_all();
        }
        if self.shot.is_some() || self.url_probe.is_some() {
            ctx.request_repaint();
        }
    }
}

/// `file:///home/me/Film.mkv` and `/home/me/Film.mkv` are the same file.
fn file_uri_to_path(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let hex = |b: u8| -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    };
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    Some(String::from_utf8_lossy(&out).to_string())
}

/// One identity per file: the desktop hands over absolute paths (or file://
/// links), but a command line can be relative, and the library must not grow
/// a twin.
fn canonical_media(raw: &str) -> String {
    let text = file_uri_to_path(raw.trim()).unwrap_or_else(|| raw.trim().to_string());
    if text.is_empty() || text.contains("://") {
        return text;
    }
    std::fs::canonicalize(&text)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(text)
}

fn combo_pressed(ctx: &Context, combo: &str) -> bool {
    let Some((mods, key)) = parse_combo(combo) else { return false };
    ctx.input_mut(|i| i.consume_key(mods, key))
}

fn parse_combo(combo: &str) -> Option<(Modifiers, Key)> {
    let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut mods = Modifiers::NONE;
    for part in &parts[..parts.len().saturating_sub(1)] {
        match part.to_lowercase().as_str() {
            "ctrl" | "cmd" | "command" | "control" => mods |= Modifiers::COMMAND,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            _ => {}
        }
    }
    let key_name = parts[parts.len() - 1];
    let key = match key_name {
        "Space" | "space" => Key::Space,
        "Left" | "ArrowLeft" => Key::ArrowLeft,
        "Right" | "ArrowRight" => Key::ArrowRight,
        "Up" | "ArrowUp" => Key::ArrowUp,
        "Down" | "ArrowDown" => Key::ArrowDown,
        "Esc" | "Escape" => Key::Escape,
        "Enter" | "Return" => Key::Enter,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "[" => Key::OpenBracket,
        "]" => Key::CloseBracket,
        "," => Key::Comma,
        "." => Key::Period,
        "-" => Key::Minus,
        "=" | "+" => Key::Equals,
        s if s.chars().count() == 1 => {
            let c = s.chars().next()?.to_ascii_lowercase();
            match c {
                'a' => Key::A,
                'b' => Key::B,
                'c' => Key::C,
                'd' => Key::D,
                'e' => Key::E,
                'f' => Key::F,
                'g' => Key::G,
                'h' => Key::H,
                'i' => Key::I,
                'j' => Key::J,
                'k' => Key::K,
                'l' => Key::L,
                'm' => Key::M,
                'n' => Key::N,
                'o' => Key::O,
                'p' => Key::P,
                'q' => Key::Q,
                'r' => Key::R,
                's' => Key::S,
                't' => Key::T,
                'u' => Key::U,
                'v' => Key::V,
                'w' => Key::W,
                'x' => Key::X,
                'y' => Key::Y,
                'z' => Key::Z,
                '0' => Key::Num0,
                '1' => Key::Num1,
                '2' => Key::Num2,
                '3' => Key::Num3,
                '4' => Key::Num4,
                '5' => Key::Num5,
                '6' => Key::Num6,
                '7' => Key::Num7,
                '8' => Key::Num8,
                '9' => Key::Num9,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((mods, key))
}

// Small helper shared by the library and player surfaces.
pub fn sorted_items(items: &[LibraryItem], sort: Sort) -> Vec<LibraryItem> {
    use std::cmp::Ordering;
    let mut list = items.to_vec();
    match sort {
        Sort::Name => list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        Sort::Added => list.sort_by(|a, b| b.added_at.cmp(&a.added_at)),
        Sort::Duration => list.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms)),
        Sort::UnwatchedFirst => list.sort_by(|a, b| match (a.watched, b.watched) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        }),
        Sort::WatchedFirst => list.sort_by(|a, b| match (a.watched, b.watched) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        }),
    }
    list
}

#[cfg(test)]
mod url_tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn file_links_become_plain_paths() {
        assert_eq!(
            file_uri_to_path("file:///home/me/Film%20Night.mkv").as_deref(),
            Some("/home/me/Film Night.mkv")
        );
        assert_eq!(file_uri_to_path("/home/me/a.mkv"), None);
    }

    #[test]
    fn parses_m3u_body_and_resolves_relative_entries() {
        let body = "#EXTM3U\r\n#EXTINF:-1,One\r\nfirst.mp4\r\n#EXTINF:-1,Two\r\nhttps://cdn.example/two.mp4\r\n";
        let items = parse_playlist_body(body, "https://example.org/lists/tv.m3u");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "https://example.org/lists/first.mp4");
        assert_eq!(items[1], "https://cdn.example/two.mp4");
    }

    #[test]
    fn parses_xspf_locations() {
        let body = r#"<?xml version="1.0"?><playlist version="1"><trackList><track><location>a.mp4</location></track></trackList></playlist>"#;
        let items = parse_playlist_body(body, "http://host/dir/list.xspf");
        assert_eq!(items, vec!["http://host/dir/a.mp4".to_string()]);
    }

    #[test]
    fn hls_body_is_not_expanded() {
        let body = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:6\nseg0.ts\nseg1.ts\n";
        assert!(is_hls_body(body));
        assert!(is_hls_url("https://host/live/index.m3u8?token=1"));
    }

    #[test]
    fn fetches_playlist_over_http() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = "#EXTM3U\n#EXTINF:-1,A\na.mp4\n#EXTINF:-1,B\nb.mp4\n";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: audio/x-mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        match fetch_playlist(&format!("http://127.0.0.1:{port}/list.m3u")) {
            UrlProbe::Playlist(items) => {
                assert_eq!(items.len(), 2, "two entries expected, got {items:?}");
                assert!(items[0].ends_with("/a.mp4"));
            }
            UrlProbe::Single | UrlProbe::Failed(_) => panic!("playlist not recognised"),
        }
    }
}
