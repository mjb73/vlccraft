// A player is a window, not a command prompt: on Windows the release build is
// built as a windowed program so double-clicking it does not leave a black
// console behind the interface. The few print-out flags attach to the console
// of whoever ran them, further down.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod defaults;
mod dialogs;
mod instance;
mod library;
mod model;
mod platform;
mod player;
mod settings;
mod sidebar;
mod skin;
mod theme;
mod tools;
mod vlc;

use app::App;
use std::path::PathBuf;

const USAGE: &str = "\
VLCCraft — a calm skin over the VLC already installed on this machine.

Usage: vlccraft [FILE|URL ...] [--screenshot FILE.png]
       vlccraft --set-default [--video-only|--music-only]
       vlccraft --restore-default
       vlccraft --default-status

Opening a file (or a link) plays it. A file handed over while VLCCraft is
already open joins that window's queue instead of starting a second player.

  --screenshot FILE.png   draw 120 frames, save the window as a picture, exit
  --set-default           make VLCCraft the standard player for video and music
  --restore-default       hand video and music back to the earlier player
  --default-status        print which player currently opens video and music
";

/// Windows only: a windowed program owns no console, so the flags that print
/// something borrow the console of the shell that started them.
#[cfg(windows)]
fn borrow_console() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() -> eframe::Result<()> {
    // Nothing has been printed yet, so the handle is still unclaimed: attaching
    // first is what makes `--version` and `--help` visible from a terminal.
    #[cfg(windows)]
    if std::env::args().skip(1).any(|a| {
        matches!(
            a.as_str(),
            "-h" | "--help" | "--version" | "--default-status" | "--restore-default"
        ) || a.starts_with("--set-default")
    }) {
        borrow_console();
    }

    let mut screenshot: Option<PathBuf> = None;
    let mut media: Vec<String> = Vec::new();
    let mut action: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                if let Some(path) = args.next() {
                    screenshot = Some(PathBuf::from(path));
                }
            }
            "--set-default" => action = Some("default:both".into()),
            "--video-only" => action = Some("default:video".into()),
            "--music-only" => action = Some("default:audio".into()),
            "--restore-default" => action = Some("restore".into()),
            "--default-status" => action = Some("status".into()),
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "--version" => {
                println!("VLCCraft {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {
                if let Some(rest) = arg.strip_prefix("--screenshot=") {
                    screenshot = Some(PathBuf::from(rest));
                } else if let Some(rest) = arg.strip_prefix("--set-default=") {
                    action = Some(format!("default:{rest}"));
                } else if !arg.starts_with('-') {
                    media.push(arg);
                }
            }
        }
    }

    // The small headless commands: useful for a first setup script, and the
    // way the player can be checked without opening a window.
    if let Some(action) = action {
        let store = model::Store::new(model::data_dir());
        let accent = store.settings().accent;
        std::process::exit(match action.as_str() {
            "status" => {
                let s = defaults::status();
                println!("available={}", if s.available { "yes" } else { "no" });
                println!("video={}", s.video_name);
                println!("music={}", s.audio_name);
                println!("video_ours={}", if s.video_ours { "yes" } else { "no" });
                println!("music_ours={}", if s.audio_ours { "yes" } else { "no" });
                println!(
                    "can_hand_back={}",
                    if defaults::load_record().can_hand_back() { "yes" } else { "no" }
                );
                if s.all_ours() {
                    println!("VLCCraft opens video and music.");
                } else {
                    println!(
                        "Video opens in {}; music opens in {}.",
                        s.video_name, s.audio_name
                    );
                }
                0
            }
            "restore" => match defaults::restore() {
                Ok(msg) => {
                    println!("{msg}");
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            },
            other => {
                let (video, audio) = match other.trim_start_matches("default:") {
                    "video" => (true, false),
                    "audio" | "music" => (false, true),
                    _ => (true, true),
                };
                match defaults::apply(video, audio, accent) {
                    Ok(msg) => {
                        println!("{msg}");
                        0
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        1
                    }
                }
            }
        });
    }

    let shot_mode = screenshot.is_some();

    // Keep the copy the desktop launches in step with this build. This runs
    // for a screenshot too: the check after every build is the moment the
    // program on disk is newest, and the installed copy must not lag behind it.
    crate::defaults::refresh_launcher();

    // A second launch (a double-clicked file) goes to the window that is
    // already open. Screenshot runs never talk to a running player.
    let mut earlier_build = false;
    if !shot_mode && !media.is_empty() {
        match instance::forward(&media) {
            instance::Forwarded::Delivered => return Ok(()),
            instance::Forwarded::Stale => earlier_build = true,
            instance::Forwarded::Nobody => {}
        }
    }
    // A window from an earlier build is still open behind this one; this
    // window takes over the door so the next file lands here, not there.
    let inbox = if shot_mode {
        None
    } else {
        instance::listen(earlier_build)
    };

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("VLCCraft")
        .with_app_id("vlccraft")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([980.0, 620.0])
        .with_active(!shot_mode)
        .with_resizable(true);

    // A picture can be asked for at a particular size, so the small end of
    // the window can be looked at without touching the person's desktop.
    if shot_mode {
        if let Ok(size) = std::env::var("VLCCRAFT_SHOT_SIZE") {
            if let Some((w, h)) = size.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.trim().parse::<f32>(), h.trim().parse::<f32>()) {
                    viewport = viewport.with_inner_size([w.max(980.0), h.max(620.0)]);
                }
            }
        }
    } else {
        // Open the window the size it was left, when that is asked for. A
        // screenshot always uses the fixed size, so a check never depends on
        // whatever the person last resized to.
        let saved = model::Store::new(model::data_dir()).settings();
        if saved.remember_window {
            if let Some(size) = saved.window_size {
                viewport = viewport.with_inner_size([size[0].max(980.0), size[1].max(620.0)]);
            }
            if saved.window_maximized {
                viewport = viewport.with_maximized(true);
            }
        }
    }

    #[cfg(target_os = "windows")]
    if shot_mode {
        // A covered window still draws on Windows; keep the check out of the way.
        viewport = viewport.with_window_level(eframe::egui::WindowLevel::AlwaysOnBottom);
    }

    let mut options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        // A check must neither inherit nor overwrite the window the person
        // left: eframe would otherwise restore a full screen for the picture
        // and then save our fixed size back over theirs.
        persist_window: !shot_mode,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    if shot_mode {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    let result = eframe::run_native(
        "VLCCraft",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot.take(), media, inbox)))),
    );
    instance::cleanup();
    result
}
