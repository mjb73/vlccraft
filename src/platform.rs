//! The handful of things that differ between Linux, Windows and macOS.
//!
//! The interface is the same everywhere; what changes is where the player keeps
//! its data, where the system's VLC puts its library and its program, and which
//! folder holds VLC's plugins. Each answer is a list of candidates in the order
//! they are worth trying, so an unusual installation still has a chance.

use std::path::PathBuf;

/// The person's home folder, whichever name the system gives it.
pub fn home_dir() -> Option<PathBuf> {
    for key in ["HOME", "USERPROFILE"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }
    // Windows keeps the profile in HOMEDRIVE + HOMEPATH when USERPROFILE is
    // missing, which happens in some service contexts.
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        if !drive.is_empty() && !path.is_empty() {
            return Some(PathBuf::from(format!("{drive}{path}")));
        }
    }
    std::env::current_dir().ok()
}

/// Where the player keeps settings, the library index and resume positions.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("VLCCRAFT_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("VLCCraft");
            }
        }
        if let Some(home) = home_dir() {
            return home.join("AppData/Roaming/VLCCraft");
        }
        return PathBuf::from("VLCCraft");
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            return home.join("Library/Application Support/VLCCraft");
        }
        return PathBuf::from("VLCCraft");
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("vlccraft");
            }
        }
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/share/vlccraft")
    }
}

/// The names `libloading` should try when opening the system VLC library.
///
/// Nothing is bundled and nothing is patched: the library that the installed
/// player already provides is the one used, so a package update flows through.
pub fn vlc_library_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // VLC's own installer folder is the normal home of libvlc.dll. Next to
        // this program and on PATH are tried first, so a portable copy wins.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                out.push(dir.join("libvlc.dll"));
            }
        }
        out.push(PathBuf::from("libvlc.dll"));
        for base in ["PROGRAMFILES", "PROGRAMFILES(X86)", "ProgramW6432"] {
            if let Ok(pf) = std::env::var(base) {
                if !pf.is_empty() {
                    out.push(PathBuf::from(pf).join("VideoLAN/VLC/libvlc.dll"));
                }
            }
        }
        out.push(PathBuf::from("C:/Program Files/VideoLAN/VLC/libvlc.dll"));
        out.push(PathBuf::from("C:/Program Files (x86)/VideoLAN/VLC/libvlc.dll"));
    }

    #[cfg(target_os = "macos")]
    {
        // VLC for macOS ships as a bundle; the library lives inside it.
        for app in [
            "/Applications/VLC.app/Contents/MacOS/lib",
            "/Applications/VLC.app/Contents/MacOS",
        ] {
            out.push(PathBuf::from(app).join("libvlc.5.dylib"));
            out.push(PathBuf::from(app).join("libvlc.dylib"));
        }
        if let Some(home) = home_dir() {
            let app = home.join("Applications/VLC.app/Contents/MacOS/lib");
            out.push(app.join("libvlc.5.dylib"));
            out.push(app.join("libvlc.dylib"));
        }
        out.push(PathBuf::from("libvlc.5.dylib"));
        out.push(PathBuf::from("libvlc.dylib"));
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        out.push(PathBuf::from("libvlc.so.5"));
        out.push(PathBuf::from("libvlc.so"));
    }

    out
}

/// The page a person is sent to when VLC has to be installed by hand.
pub fn vlc_download_url() -> &'static str {
    "https://www.videolan.org/vlc/"
}

/// The name of the library, for the "what was looked for" line.
pub fn vlc_library_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "libvlc.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libvlc inside VLC.app"
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        "libvlc.so.5"
    }
}

/// How this computer installs VLC, when there is a package manager to ask.
///
/// Detected from `/etc/os-release` rather than guessed, so the command in the
/// setup card is one that actually exists on the machine in front of the person.
pub fn vlc_install_command() -> Option<&'static str> {
    #[cfg(target_os = "windows")]
    {
        None
    }
    #[cfg(target_os = "macos")]
    {
        None
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let mut id = String::new();
        let mut like = String::new();
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("ID=") {
                id = v.trim_matches('"').to_lowercase();
            } else if let Some(v) = line.strip_prefix("ID_LIKE=") {
                like = v.trim_matches('"').to_lowercase();
            }
        }
        let family = format!("{id} {like}");
        if family.contains("arch") || family.contains("manjaro") {
            Some("sudo pacman -S vlc")
        } else if family.contains("fedora") || family.contains("rhel") || family.contains("centos") {
            Some("sudo dnf install vlc")
        } else if family.contains("suse") {
            Some("sudo zypper install vlc")
        } else if family.contains("alpine") {
            Some("sudo apk add vlc")
        } else if family.contains("debian")
            || family.contains("ubuntu")
            || family.contains("mint")
            || family.contains("pop")
            || family.contains("elementary")
        {
            Some("sudo apt install vlc")
        } else {
            None
        }
    }
}

/// What to tell the person when the library could not be opened.
pub fn vlc_missing_hint() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Install VLC from videolan.org (64-bit), then start VLCCraft again. The player uses that installation; nothing is bundled."
    }
    #[cfg(target_os = "macos")]
    {
        "Install VLC into Applications from videolan.org, then start VLCCraft again. The player uses that installation; nothing is bundled."
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        "Install VLC from your software store or videolan.org, then start VLCCraft again. The player uses that installation; nothing is bundled."
    }
}

/// Where the system player's programs can be, in the order worth trying.
///
/// Only the command-line side is used here: VLC is asked to decode, convert or
/// stream with no interface of its own, because this app is the interface.
pub fn vlc_binary_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let names = ["vlc.exe", "cvlc.exe"];
        for base in ["PROGRAMFILES", "PROGRAMFILES(X86)", "ProgramW6432"] {
            if let Ok(pf) = std::env::var(base) {
                if !pf.is_empty() {
                    for name in names {
                        out.push(PathBuf::from(&pf).join("VideoLAN/VLC").join(name));
                    }
                }
            }
        }
        for name in names {
            out.push(PathBuf::from("C:/Program Files/VideoLAN/VLC").join(name));
            out.push(PathBuf::from("C:/Program Files (x86)/VideoLAN/VLC").join(name));
        }
    }

    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from("/Applications/VLC.app/Contents/MacOS/VLC"));
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        out.push(PathBuf::from("/usr/bin/cvlc"));
        out.push(PathBuf::from("/usr/bin/vlc"));
    }

    out
}

/// The plain names to look for on PATH, per system.
pub fn vlc_binary_names() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["cvlc.exe", "vlc.exe"]
    }
    #[cfg(not(target_os = "windows"))]
    {
        &["cvlc", "vlc"]
    }
}

/// Where VLC keeps its plugins, which the extensions page counts.
pub fn vlc_plugins_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for base in ["PROGRAMFILES", "PROGRAMFILES(X86)", "ProgramW6432"] {
            if let Ok(pf) = std::env::var(base) {
                if !pf.is_empty() {
                    out.push(PathBuf::from(&pf).join("VideoLAN/VLC/plugins"));
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from("/Applications/VLC.app/Contents/MacOS/plugins"));
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        out.push(PathBuf::from("/usr/lib/vlc/plugins"));
        out.push(PathBuf::from("/usr/lib64/vlc/plugins"));
        out.push(PathBuf::from("/usr/lib/x86_64-linux-gnu/vlc/plugins"));
        out.push(PathBuf::from("/usr/local/lib/vlc/plugins"));
    }

    out
}

/// Whether this system has a way for an app to take over double-clicked files.
///
/// Linux does, through the desktop entry this app writes. macOS and Windows
/// decide that in the system's own settings, so the app says so instead of
/// pretending to have a switch.
pub fn can_set_default_player() -> bool {
    cfg!(target_os = "linux")
}

/// The system name, for the About row and the settings text.
pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
}

/// Where a newly browsed folder should start.
pub fn browse_start() -> PathBuf {
    home_dir().unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            PathBuf::from("C:/")
        } else {
            PathBuf::from("/")
        }
    })
}
