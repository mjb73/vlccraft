//! Thin, dynamically-loaded wrapper around the *system* libvlc.
//!
//! This is the "skin, not a fork" rule: the library is opened at runtime with
//! `dlopen` (`libvlc.so.5`). Nothing under /usr/lib/vlc, /usr/bin/vlc or the
//! system plugin directory is read, replaced or written, so package updates
//! flow through untouched. If libvlc is absent or its ABI moves, the interface
//! still opens and reports why playback is unavailable.

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type VlcNew = unsafe extern "C" fn(c_int, *const *const c_char) -> *mut c_void;
type VlcRelease = unsafe extern "C" fn(*mut c_void);
type MediaNewPath = unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_void;
type MediaNewLocation = unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_void;
type MediaRelease = unsafe extern "C" fn(*mut c_void);
type PlayerNew = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type PlayerRelease = unsafe extern "C" fn(*mut c_void);
type PlayerSetMedia = unsafe extern "C" fn(*mut c_void, *mut c_void);
type PlayerPlay = unsafe extern "C" fn(*mut c_void) -> c_int;
type PlayerStop = unsafe extern "C" fn(*mut c_void);
type PlayerPause = unsafe extern "C" fn(*mut c_void);
type PlayerGetTime = unsafe extern "C" fn(*mut c_void) -> i64;
type PlayerGetLength = unsafe extern "C" fn(*mut c_void) -> i64;
type PlayerIsPlaying = unsafe extern "C" fn(*mut c_void) -> c_int;
type PlayerGetState = unsafe extern "C" fn(*mut c_void) -> c_int;
type PlayerSetTime = unsafe extern "C" fn(*mut c_void, i64);
type PlayerSetRate = unsafe extern "C" fn(*mut c_void, f32) -> c_int;
type PlayerGetRate = unsafe extern "C" fn(*mut c_void) -> f32;
type SetCallbacks = unsafe extern "C" fn(
    *mut c_void,
    Option<unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *const *mut c_void)>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
);
type SetFormat =
    unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*mut *mut c_void, *mut c_char, *mut c_uint, *mut c_uint, *mut c_uint, *mut c_uint) -> c_uint>, Option<unsafe extern "C" fn(*mut c_void)>);
type AudioSetVolume = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type AudioGetVolume = unsafe extern "C" fn(*mut c_void) -> c_int;
type AudioSetMute = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type TrackDescGet = unsafe extern "C" fn(*mut c_void) -> *mut TrackDescription;
type TrackDescRelease = unsafe extern "C" fn(*mut TrackDescription);
type SetTrack = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type AudioOutputList = unsafe extern "C" fn(*const c_void) -> *mut AudioOutput;
type AudioOutputRelease = unsafe extern "C" fn(*mut AudioOutput);
type AudioOutputSet = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type AudioDeviceEnum = unsafe extern "C" fn(*mut c_void) -> *mut AudioDevice;
type AudioDeviceListRelease = unsafe extern "C" fn(*mut AudioDevice);
type AudioDeviceSet = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char);
type GetVersion = unsafe extern "C" fn() -> *const c_char;
type EqPresetCount = unsafe extern "C" fn() -> c_uint;
type EqPresetName = unsafe extern "C" fn(c_uint) -> *const c_char;
type EqNewFromPreset = unsafe extern "C" fn(c_uint) -> *mut c_void;
type EqRelease = unsafe extern "C" fn(*mut c_void);
type SetEqualizer = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;

#[repr(C)]
pub struct TrackDescription {
    pub i_id: c_int,
    pub psz_name: *mut c_char,
    pub p_next: *mut TrackDescription,
}

#[repr(C)]
pub struct AudioOutput {
    pub psz_name: *mut c_char,
    pub psz_description: *mut c_char,
    pub p_next: *mut AudioOutput,
}

/// A concrete sound card or sink reported by the active audio output,
/// as opposed to an output *module* such as `pulse` or `alsa`.
#[repr(C)]
pub struct AudioDevice {
    pub psz_device: *mut c_char,
    pub psz_description: *mut c_char,
    pub p_next: *mut AudioDevice,
}

struct FrameBuffer {
    width: AtomicU32,
    height: AtomicU32,
    version: AtomicU64,
    has_frame: AtomicBool,
    data: Mutex<Vec<u8>>,
}

impl FrameBuffer {
    fn new() -> Self {
        Self {
            width: AtomicU32::new(0),
            height: AtomicU32::new(0),
            version: AtomicU64::new(0),
            has_frame: AtomicBool::new(false),
            data: Mutex::new(Vec::new()),
        }
    }
}

unsafe extern "C" fn cb_setup(
    opaque: *mut *mut c_void,
    chroma: *mut c_char,
    width: *mut c_uint,
    height: *mut c_uint,
    pitches: *mut c_uint,
    lines: *mut c_uint,
) -> c_uint {
    let fb = &*(*opaque as *const FrameBuffer);
    let w = *width;
    let h = *height;
    fb.width.store(w, Ordering::SeqCst);
    fb.height.store(h, Ordering::SeqCst);
    if let Ok(mut buf) = fb.data.lock() {
        buf.resize((w as usize) * (h as usize) * 4, 0);
    }
    for (i, ch) in b"RV32".iter().enumerate() {
        *chroma.add(i) = *ch as c_char;
    }
    *pitches = w * 4;
    *lines = h;
    1
}

unsafe extern "C" fn cb_lock(opaque: *mut c_void, planes: *mut *mut c_void) -> *mut c_void {
    let fb = &*(opaque as *const FrameBuffer);
    let mut buf = match fb.data.lock() {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };
    *planes = buf.as_mut_ptr() as *mut c_void;
    planes as *mut c_void
}

unsafe extern "C" fn cb_unlock(_opaque: *mut c_void, _picture: *mut c_void, _planes: *const *mut c_void) {}

unsafe extern "C" fn eq_count_zero() -> c_uint {
    0
}
unsafe extern "C" fn eq_name_null(_: c_uint) -> *const c_char {
    std::ptr::null()
}
unsafe extern "C" fn eq_new_null(_: c_uint) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn eq_release_nop(_: *mut c_void) {}
unsafe extern "C" fn set_eq_fail(_: *mut c_void, _: *mut c_void) -> c_int {
    -1
}

unsafe extern "C" fn cb_display(opaque: *mut c_void, _picture: *mut c_void) {
    let fb = &*(opaque as *const FrameBuffer);
    fb.has_frame.store(true, Ordering::SeqCst);
    fb.version.fetch_add(1, Ordering::SeqCst);
}

#[derive(Clone)]
pub struct Track {
    pub id: i32,
    pub name: String,
}

pub struct Engine {
    _lib: Library,
    instance: *mut c_void,
    player: *mut c_void,
    media: *mut c_void,
    frame: Arc<FrameBuffer>,
    opaque: *mut c_void,
    texture: Option<TextureHandle>,
    texture_version: u64,
    texture_size: egui::Vec2,
    pub path: String,
    pub silent: bool,
    version: String,
    f_set_media: PlayerSetMedia,
    f_play: PlayerPlay,
    f_stop: PlayerStop,
    f_pause: PlayerPause,
    f_time: PlayerGetTime,
    f_length: PlayerGetLength,
    f_is_playing: PlayerIsPlaying,
    f_state: PlayerGetState,
    f_set_time: PlayerSetTime,
    f_set_rate: PlayerSetRate,
    f_get_rate: PlayerGetRate,
    f_audio_volume: AudioSetVolume,
    f_audio_get_volume: AudioGetVolume,
    f_audio_mute: AudioSetMute,
    f_audio_tracks: TrackDescGet,
    f_video_tracks: TrackDescGet,
    f_audio_track: SetTrack,
    f_video_track: SetTrack,
    f_track_release: TrackDescRelease,
    f_output_list: AudioOutputList,
    f_output_release: AudioOutputRelease,
    f_output_set: AudioOutputSet,
    f_device_enum: Option<AudioDeviceEnum>,
    f_device_release: Option<AudioDeviceListRelease>,
    f_device_set: Option<AudioDeviceSet>,
    f_media_new_path: MediaNewPath,
    f_media_new_location: MediaNewLocation,
    f_media_release: MediaRelease,
    player_release: PlayerRelease,
    audio_outputs: Vec<String>,
    f_eq_count: EqPresetCount,
    f_eq_name: EqPresetName,
    f_eq_new: EqNewFromPreset,
    f_eq_release: EqRelease,
    f_set_eq: SetEqualizer,
}

unsafe impl Send for Engine {}

impl Engine {
    pub fn new(silent: bool) -> Result<Self, String> {
        unsafe {
            // The system VLC is opened by name, per system: libvlc.so.5 on
            // Linux, libvlc.dll on Windows, libvlc.5.dylib inside VLC.app on
            // macOS. Each candidate is tried in turn so an unusual install
            // still works.
            let mut opened: Option<(Library, std::path::PathBuf)> = None;
            let mut last = String::new();
            for candidate in crate::platform::vlc_library_candidates() {
                match Library::new(&candidate) {
                    Ok(lib) => {
                        opened = Some((lib, candidate));
                        break;
                    }
                    Err(e) => last = format!("{}: {e}", candidate.display()),
                }
            }
            let Some((lib, lib_path)) = opened else {
                return Err(format!(
                    "libvlc not available ({last}). {}",
                    crate::platform::vlc_missing_hint()
                ));
            };
            let _ = &lib_path;

            let vlc_new: VlcNew = *lib
                .get(b"libvlc_new\0")
                .map_err(|e| format!("libvlc_new missing: {e}"))?;
            let vlc_release: VlcRelease = *lib
                .get(b"libvlc_release\0")
                .map_err(|e| format!("libvlc_release missing: {e}"))?;
            let get_version: GetVersion = *lib
                .get(b"libvlc_get_version\0")
                .map_err(|e| format!("libvlc_get_version missing: {e}"))?;

            let mut args: Vec<CString> = vec![
                CString::new("vlccraft").unwrap(),
                CString::new("--intf=dummy").unwrap(),
                CString::new("--no-video-title-show").unwrap(),
                CString::new("--no-osd").unwrap(),
                CString::new("--quiet").unwrap(),
                CString::new("--no-snapshot-preview").unwrap(),
                CString::new("--no-sub-autodetect-file").unwrap(),
            ];
            if silent {
                args.push(CString::new("--aout=dummy").unwrap());
            }
            let mut ptrs: Vec<*const c_char> = args.iter().map(|c| c.as_ptr()).collect();
            let instance = vlc_new(ptrs.len() as c_int, ptrs.as_mut_ptr());
            if instance.is_null() {
                return Err("VLC could not start (libvlc_new returned null)".into());
            }

            let guard = LibGuard {
                instance,
                release: vlc_release,
            };

            let player_new: PlayerNew = *lib.get(b"libvlc_media_player_new\0").map_err(|e| e.to_string())?;
            let player_release: PlayerRelease = *lib
                .get(b"libvlc_media_player_release\0")
                .map_err(|e| e.to_string())?;
            let player = player_new(instance);
            if player.is_null() {
                return Err("VLC could not create a media player".into());
            }

            let frame = Arc::new(FrameBuffer::new());
            let opaque = Arc::into_raw(frame.clone()) as *mut c_void;
            let set_callbacks: SetCallbacks = *lib
                .get(b"libvlc_video_set_callbacks\0")
                .map_err(|e| e.to_string())?;
            let set_format: SetFormat = *lib
                .get(b"libvlc_video_set_format_callbacks\0")
                .map_err(|e| e.to_string())?;
            set_callbacks(player, Some(cb_lock), Some(cb_unlock), Some(cb_display), opaque);
            set_format(player, Some(cb_setup), None);

            let version = CStr::from_ptr(get_version()).to_string_lossy().to_string();

            let f_output_list: AudioOutputList = *lib
                .get(b"libvlc_audio_output_list_get\0")
                .map_err(|e| e.to_string())?;
            let f_output_release: AudioOutputRelease = *lib
                .get(b"libvlc_audio_output_list_release\0")
                .map_err(|e| e.to_string())?;
            let mut audio_outputs = Vec::new();
            let list = f_output_list(instance);
            if !list.is_null() {
                let mut cur = list;
                while !cur.is_null() {
                    let name = CStr::from_ptr((*cur).psz_name).to_string_lossy().to_string();
                    audio_outputs.push(name);
                    cur = (*cur).p_next;
                }
                f_output_release(list);
            }

            let engine = Self {
                instance,
                player,
                media: std::ptr::null_mut(),
                frame,
                opaque,
                texture: None,
                texture_version: u64::MAX,
                texture_size: egui::Vec2::ZERO,
                path: String::new(),
                silent,
                version,
                f_set_media: *lib.get(b"libvlc_media_player_set_media\0").map_err(|e| e.to_string())?,
                f_play: *lib.get(b"libvlc_media_player_play\0").map_err(|e| e.to_string())?,
                f_stop: *lib.get(b"libvlc_media_player_stop\0").map_err(|e| e.to_string())?,
                f_pause: *lib.get(b"libvlc_media_player_pause\0").map_err(|e| e.to_string())?,
                f_time: *lib.get(b"libvlc_media_player_get_time\0").map_err(|e| e.to_string())?,
                f_length: *lib.get(b"libvlc_media_player_get_length\0").map_err(|e| e.to_string())?,
                f_is_playing: *lib.get(b"libvlc_media_player_is_playing\0").map_err(|e| e.to_string())?,
                f_state: *lib.get(b"libvlc_media_player_get_state\0").map_err(|e| e.to_string())?,
                f_set_time: *lib.get(b"libvlc_media_player_set_time\0").map_err(|e| e.to_string())?,
                f_set_rate: *lib.get(b"libvlc_media_player_set_rate\0").map_err(|e| e.to_string())?,
                f_get_rate: *lib.get(b"libvlc_media_player_get_rate\0").map_err(|e| e.to_string())?,
                f_audio_volume: *lib.get(b"libvlc_audio_set_volume\0").map_err(|e| e.to_string())?,
                f_audio_get_volume: *lib.get(b"libvlc_audio_get_volume\0").map_err(|e| e.to_string())?,
                f_audio_mute: *lib.get(b"libvlc_audio_set_mute\0").map_err(|e| e.to_string())?,
                f_audio_tracks: *lib.get(b"libvlc_audio_get_track_description\0").map_err(|e| e.to_string())?,
                f_video_tracks: *lib.get(b"libvlc_video_get_track_description\0").map_err(|e| e.to_string())?,
                f_audio_track: *lib.get(b"libvlc_audio_set_track\0").map_err(|e| e.to_string())?,
                f_video_track: *lib.get(b"libvlc_video_set_track\0").map_err(|e| e.to_string())?,
                f_track_release: *lib.get(b"libvlc_track_description_list_release\0").map_err(|e| e.to_string())?,
                f_output_list,
                f_output_release,
                f_output_set: *lib.get(b"libvlc_audio_output_set\0").map_err(|e| e.to_string())?,
                f_device_enum: lib
                    .get::<AudioDeviceEnum>(b"libvlc_audio_output_device_enum\0")
                    .map(|s| *s)
                    .ok(),
                f_device_release: lib
                    .get::<AudioDeviceListRelease>(b"libvlc_audio_output_device_list_release\0")
                    .map(|s| *s)
                    .ok(),
                f_device_set: lib
                    .get::<AudioDeviceSet>(b"libvlc_audio_output_device_set\0")
                    .map(|s| *s)
                    .ok(),
                f_media_new_path: *lib.get(b"libvlc_media_new_path\0").map_err(|e| e.to_string())?,
                f_media_new_location: *lib.get(b"libvlc_media_new_location\0").map_err(|e| e.to_string())?,
                f_media_release: *lib.get(b"libvlc_media_release\0").map_err(|e| e.to_string())?,
                player_release,
                audio_outputs,
                f_eq_count: lib
                    .get::<EqPresetCount>(b"libvlc_audio_equalizer_get_preset_count\0")
                    .map(|s| *s)
                    .unwrap_or(eq_count_zero),
                f_eq_name: lib
                    .get::<EqPresetName>(b"libvlc_audio_equalizer_get_preset_name\0")
                    .map(|s| *s)
                    .unwrap_or(eq_name_null),
                f_eq_new: lib
                    .get::<EqNewFromPreset>(b"libvlc_audio_equalizer_new_from_preset\0")
                    .map(|s| *s)
                    .unwrap_or(eq_new_null),
                f_eq_release: lib
                    .get::<EqRelease>(b"libvlc_audio_equalizer_release\0")
                    .map(|s| *s)
                    .unwrap_or(eq_release_nop),
                f_set_eq: lib
                    .get::<SetEqualizer>(b"libvlc_media_player_set_equalizer\0")
                    .map(|s| *s)
                    .unwrap_or(set_eq_fail),
                _lib: lib,
            };
            std::mem::forget(guard);
            Ok(engine)
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn audio_outputs(&self) -> &[String] {
        &self.audio_outputs
    }

    pub fn open(&mut self, target: &str) -> Result<(), String> {
        unsafe {
            if self.player.is_null() {
                return Err("no player".into());
            }
            let c = CString::new(target).map_err(|_| "invalid media path".to_string())?;
            let media = if target.starts_with("http://")
                || target.starts_with("https://")
                || target.contains("://")
            {
                (self.f_media_new_location)(self.instance, c.as_ptr())
            } else {
                (self.f_media_new_path)(self.instance, c.as_ptr())
            };
            if media.is_null() {
                return Err("VLC could not open that media".into());
            }
            (self.f_set_media)(self.player, media);
            if !self.media.is_null() {
                (self.f_media_release)(self.media);
            }
            self.media = media;
            self.path = target.into();
            self.texture_version = u64::MAX;
            self.frame.has_frame.store(false, Ordering::SeqCst);
            let rc = (self.f_play)(self.player);
            if rc != 0 {
                return Err("VLC refused to start playback".into());
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        unsafe {
            if !self.player.is_null() {
                (self.f_stop)(self.player);
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        unsafe { (self.f_is_playing)(self.player) == 1 }
    }

    pub fn ended(&self) -> bool {
        unsafe { (self.f_state)(self.player) == 6 }
    }

    pub fn time_ms(&self) -> i64 {
        unsafe {
            let t = (self.f_time)(self.player);
            if t < 0 { 0 } else { t }
        }
    }

    pub fn length_ms(&self) -> i64 {
        unsafe {
            let t = (self.f_length)(self.player);
            if t < 0 { 0 } else { t }
        }
    }

    pub fn set_time(&self, ms: i64) {
        unsafe { (self.f_set_time)(self.player, ms.max(0)) }
    }

    pub fn toggle_pause(&self) {
        unsafe { (self.f_pause)(self.player) }
    }

    pub fn rate(&self) -> f32 {
        unsafe {
            let r = (self.f_get_rate)(self.player);
            if r <= 0.01 { 1.0 } else { r }
        }
    }

    pub fn set_rate(&self, rate: f32) {
        unsafe {
            (self.f_set_rate)(self.player, rate);
        }
    }

    pub fn set_volume(&self, volume: u8, muted: bool) {
        unsafe {
            (self.f_audio_volume)(self.player, volume as c_int);
            (self.f_audio_mute)(self.player, muted as c_int);
        }
    }

    pub fn volume(&self) -> i32 {
        unsafe { (self.f_audio_get_volume)(self.player) }
    }

    pub fn audio_tracks(&self) -> Vec<Track> {
        self.tracks(self.f_audio_tracks)
    }

    pub fn video_tracks(&self) -> Vec<Track> {
        self.tracks(self.f_video_tracks)
    }

    fn tracks(&self, get: TrackDescGet) -> Vec<Track> {
        let mut out = Vec::new();
        unsafe {
            let list = get(self.player);
            if list.is_null() {
                return out;
            }
            let mut cur = list;
            while !cur.is_null() {
                let name = if (*cur).psz_name.is_null() {
                    "Track".to_string()
                } else {
                    CStr::from_ptr((*cur).psz_name).to_string_lossy().to_string()
                };
                out.push(Track {
                    id: (*cur).i_id,
                    name,
                });
                cur = (*cur).p_next;
            }
            (self.f_track_release)(list);
        }
        out
    }

    pub fn set_audio_track(&self, id: i32) {
        unsafe {
            (self.f_audio_track)(self.player, id);
        }
    }

    pub fn set_video_track(&self, id: i32) {
        unsafe {
            (self.f_video_track)(self.player, id);
        }
    }

    pub fn set_audio_output(&self, name: &str) {
        if let Ok(c) = CString::new(name) {
            unsafe {
                (self.f_output_set)(self.player, c.as_ptr());
            }
        }
    }

    /// Sound cards / sinks behind the current output. Empty when the output
    /// has not started yet, which is when the UI offers modules instead.
    pub fn audio_devices(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let (Some(enum_devices), Some(release)) = (self.f_device_enum, self.f_device_release) else {
            return out;
        };
        unsafe {
            let list = enum_devices(self.player);
            if list.is_null() {
                return out;
            }
            let mut cur = list;
            while !cur.is_null() {
                let id = if (*cur).psz_device.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr((*cur).psz_device).to_string_lossy().to_string()
                };
                let desc = if (*cur).psz_description.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr((*cur).psz_description)
                        .to_string_lossy()
                        .to_string()
                };
                if !id.is_empty() {
                    out.push((id, desc));
                }
                cur = (*cur).p_next;
            }
            release(list);
        }
        out
    }

    pub fn set_audio_device(&self, id: &str) -> bool {
        let Some(set) = self.f_device_set else { return false };
        let Ok(c) = CString::new(id) else { return false };
        unsafe { set(self.player, std::ptr::null(), c.as_ptr()) };
        true
    }

    /// Applies a saved choice: `device:<id>`, `module:<name>`, or a bare module
    /// name from an older settings file.
    pub fn set_audio_choice(&self, value: &str) {
        if let Some(id) = value.strip_prefix("device:") {
            self.set_audio_device(id);
        } else if let Some(name) = value.strip_prefix("module:") {
            self.set_audio_output(name);
        } else if !value.is_empty() && value != "Automatic" {
            self.set_audio_output(value);
        }
    }

    pub fn equalizer_presets(&self) -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let count = (self.f_eq_count)();
            for i in 0..count {
                let p = (self.f_eq_name)(i);
                if !p.is_null() {
                    out.push(CStr::from_ptr(p).to_string_lossy().to_string());
                }
            }
        }
        out
    }

    pub fn set_equalizer(&self, preset: Option<usize>) {
        unsafe {
            match preset {
                Some(index) => {
                    let eq = (self.f_eq_new)(index as c_uint);
                    if !eq.is_null() {
                        (self.f_set_eq)(self.player, eq);
                        (self.f_eq_release)(eq);
                    }
                }
                None => {
                    (self.f_set_eq)(self.player, std::ptr::null_mut());
                }
            }
        }
    }

    /// A downscaled copy of the current frame, used for seek previews.
    pub fn snapshot(&self, max_w: usize) -> Option<ColorImage> {
        let w = self.frame.width.load(Ordering::SeqCst) as usize;
        let h = self.frame.height.load(Ordering::SeqCst) as usize;
        if w == 0 || h == 0 {
            return None;
        }
        let buf = self.frame.data.lock().ok()?;
        if buf.len() < w * h * 4 {
            return None;
        }
        let tw = max_w.min(w).max(1);
        let th = (h * tw / w).max(1);
        let mut pixels = Vec::with_capacity(tw * th);
        for y in 0..th {
            let sy = y * h / th;
            for x in 0..tw {
                let sx = x * w / tw;
                let i = (sy * w + sx) * 4;
                pixels.push(Color32::from_rgb(buf[i + 2], buf[i + 1], buf[i]));
            }
        }
        Some(ColorImage::new([tw, th], pixels))
    }

    pub fn has_frame(&self) -> bool {
        self.frame.has_frame.load(Ordering::SeqCst)
    }

    /// Upload the newest decoded frame to egui, if there is one.
    pub fn texture(&mut self, ctx: &egui::Context) -> Option<(egui::TextureId, egui::Vec2)> {
        let version = self.frame.version.load(Ordering::SeqCst);
        if self.texture.is_none() || version != self.texture_version {
            let w = self.frame.width.load(Ordering::SeqCst) as usize;
            let h = self.frame.height.load(Ordering::SeqCst) as usize;
            if w == 0 || h == 0 {
                return self.texture.as_ref().map(|t| (t.id(), t.size_vec2()));
            }
            let mut pixels = vec![Color32::BLACK; w * h];
            if let Ok(buf) = self.frame.data.lock() {
                if buf.len() >= w * h * 4 {
                    for (dst, px) in pixels.iter_mut().zip(buf.chunks_exact(4)) {
                        *dst = Color32::from_rgb(px[2], px[1], px[0]);
                    }
                }
            }
            let image = ColorImage::new([w, h], pixels);
            let new_size = egui::Vec2::new(w as f32, h as f32);
            if self.texture.is_none() || self.texture_size != new_size {
                self.texture = Some(ctx.load_texture("vlc-frame", image, TextureOptions::LINEAR));
                self.texture_size = new_size;
            } else if let Some(tex) = &mut self.texture {
                // A full set resizes the texture if the decoder changed
                // resolution; a partial update would panic on that race.
                tex.set(image, TextureOptions::LINEAR);
            }
            self.texture_version = version;
        }
        self.texture.as_ref().map(|t| (t.id(), t.size_vec2()))
    }
}

struct LibGuard {
    instance: *mut c_void,
    release: VlcRelease,
}

impl Drop for LibGuard {
    fn drop(&mut self) {
        if !self.instance.is_null() {
            unsafe { (self.release)(self.instance) }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if !self.player.is_null() {
                (self.f_stop)(self.player);
                if !self.media.is_null() {
                    (self.f_media_release)(self.media);
                    self.media = std::ptr::null_mut();
                }
                (self.player_release)(self.player);
                self.player = std::ptr::null_mut();
            }
            if !self.instance.is_null() {
                if let Ok(f) = self._lib.get::<VlcRelease>(b"libvlc_release\0") {
                    (*f)(self.instance);
                }
                self.instance = std::ptr::null_mut();
            }
            drop(Arc::from_raw(self.opaque as *const FrameBuffer));
        }
    }
}
