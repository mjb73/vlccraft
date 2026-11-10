//! The version number, and the way a new release reaches this copy.
//!
//! VLCCraft is published as GitHub releases: one file per system, each with a
//! `.sha256` beside it. This module asks GitHub what the newest release is,
//! compares it with the number compiled into this build, downloads the file
//! that fits this machine, checks it against the published fingerprint, and
//! puts it where the running copy lives. Nothing here reads or writes VLC's
//! files: only the VLCCraft program itself is replaced.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The number this build carries. `Cargo.toml` is the one place it is written.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

pub const RELEASES_PAGE: &str = "https://github.com/mjb73/vlccraft/releases";
pub const API_LATEST: &str = "https://api.github.com/repos/mjb73/vlccraft/releases/latest";

const USER_AGENT: &str = concat!(
    "VLCCraft/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mjb73/vlccraft)"
);

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

/// `v1.2.3`, `1.2.3-beta.1` and `1.2` all answer with three numbers; anything
/// after a dash is a pre-release note and does not take part in the compare.
pub fn version_key(v: &str) -> (u64, u64, u64) {
    let trimmed = v.trim();
    let trimmed = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let core = trimmed
        .split(['-', '+'])
        .next()
        .unwrap_or(trimmed)
        .trim();
    let mut out = [0u64; 3];
    for (i, part) in core.split('.').take(3).enumerate() {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        out[i] = digits.parse().unwrap_or(0);
    }
    (out[0], out[1], out[2])
}

/// Whether `latest` is a release someone on `current` should move to.
pub fn is_newer(latest: &str, current: &str) -> bool {
    version_key(latest) > version_key(current)
}

/// A number for people to read: `1.0.0` as published, without the leading `v`.
pub fn display_version(tag: &str) -> String {
    tag.trim()
        .strip_prefix('v')
        .or_else(|| tag.trim().strip_prefix('V'))
        .unwrap_or(tag.trim())
        .to_string()
}

/// `2026-11-10T18:05:26Z` becomes `10 November 2026`. An unreadable date is
/// left out rather than guessed at.
pub fn pretty_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let date = iso.split('T').next().unwrap_or("");
    let mut parts = date.split('-');
    let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else {
        return String::new();
    };
    let (Ok(month), Ok(day)) = (m.parse::<usize>(), d.parse::<u32>()) else {
        return String::new();
    };
    if !(1..=12).contains(&month) {
        return String::new();
    }
    let day_and_month = format!("{day} {}", MONTHS[month - 1]);
    if y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()) {
        format!("{day_and_month} {y}")
    } else {
        day_and_month
    }
}

/// How long ago, in words the person can read at a glance.
pub fn ago(now_ms: u64, then_ms: u64) -> String {
    if then_ms == 0 || then_ms > now_ms {
        return "never".into();
    }
    let secs = (now_ms - then_ms) / 1000;
    match secs {
        0..=90 => "just now".into(),
        91..=3_600 => format!("{} minutes ago", (secs / 60).max(1)),
        3_601..=86_400 => {
            let hours = secs / 3600;
            if hours == 1 {
                "1 hour ago".into()
            } else {
                format!("{hours} hours ago")
            }
        }
        _ => {
            let days = secs / 86_400;
            if days == 1 {
                "yesterday".into()
            } else {
                format!("{days} days ago")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// What GitHub says
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

impl Asset {
    pub fn size_text(&self) -> String {
        if self.size >= 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else if self.size >= 1024 {
            format!("{:.0} kB", self.size as f64 / 1024.0)
        } else {
            format!("{} bytes", self.size)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    /// `1.0.0` — the tag without its `v`.
    pub version: String,
    pub tag: String,
    /// The release notes, as published. Shown trimmed to a short paragraph.
    pub notes: String,
    /// `10 November 2026`, or empty when GitHub sent no readable date.
    pub published: String,
    pub page: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// The file this machine would install, if the release carries one.
    pub fn asset_for_this_machine(&self) -> Option<&Asset> {
        let want = Package::for_this_system();
        let arch = std::env::consts::ARCH;
        let mut fallback: Option<&Asset> = None;
        for asset in &self.assets {
            let name = asset.name.to_lowercase();
            if name.ends_with(".sha256") || name.ends_with(".sig") {
                continue;
            }
            if !want.matches(&name) {
                continue;
            }
            // Pick the file that fits this processor; when a release carries
            // only one build, that one is the answer whatever the machine.
            let arm = name.contains("aarch64") || name.contains("arm64");
            let fits = match arch {
                "aarch64" | "arm64" => arm,
                _ => !arm,
            };
            if fits {
                return Some(asset);
            }
            fallback.get_or_insert(asset);
        }
        fallback
    }

    /// The published fingerprint for one file, when the release carries it.
    pub fn checksum_for(&self, asset: &Asset) -> Option<&Asset> {
        let want = format!("{}.sha256", asset.name);
        self.assets.iter().find(|a| a.name == want)
    }

    /// The first few readable lines of the notes, without the markdown marks.
    pub fn notes_excerpt(&self, limit: usize) -> String {
        let mut out = String::new();
        for line in self.notes.lines() {
            let clean = clean_markdown_line(line);
            let clean = clean.trim();
            if clean.is_empty() {
                if !out.ends_with("\n\n") && !out.is_empty() {
                    out.push('\n');
                }
                continue;
            }
            if out.chars().count() + clean.chars().count() > limit {
                break;
            }
            out.push_str(clean);
            out.push('\n');
        }
        out.trim().to_string()
    }
}

fn clean_markdown_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '`' => {}
            '#' if out.is_empty() => {}
            '[' => {}
            ']' => {
                // An old-style link: keep its words, drop its address.
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == ')' {
                            break;
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out.replace("  ", " ").trim_end().to_string()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Package {
    /// Linux: the tarball with `install.sh` and the program inside.
    Tarball,
    /// Windows: the zip with `vlccraft.exe`.
    Zip,
    /// macOS: the disk image with the app bundle.
    DiskImage,
}

impl Package {
    pub fn for_this_system() -> Package {
        if cfg!(target_os = "windows") {
            Package::Zip
        } else if cfg!(target_os = "macos") {
            Package::DiskImage
        } else {
            Package::Tarball
        }
    }

    fn matches(&self, lowercase_name: &str) -> bool {
        match self {
            Package::Tarball => {
                lowercase_name.ends_with(".tar.gz")
                    && (lowercase_name.contains("linux") || lowercase_name.contains("unknown"))
            }
            Package::Zip => {
                lowercase_name.ends_with(".zip")
                    && (lowercase_name.contains("windows") || lowercase_name.contains("win"))
            }
            Package::DiskImage => lowercase_name.ends_with(".dmg"),
        }
    }
}

/// Asks GitHub for the newest published release. Bounded in time and size, and
/// every failure comes back as a sentence the update card can show as-is.
pub fn fetch_latest() -> Result<Release, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(12))
        .build();
    let response = match agent
        .get(API_LATEST)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(403, _)) | Err(ureq::Error::Status(429, _)) => {
            return Err("the release page is limiting checks just now — try again later".into())
        }
        Err(ureq::Error::Status(404, _)) => return Err("no release has been published yet".into()),
        Err(ureq::Error::Status(code, _)) => return Err(format!("the release page answered {code}")),
        Err(_) => return Err("the connection may be down".into()),
    };
    let mut body = String::new();
    if response
        .into_reader()
        .take(512 * 1024)
        .read_to_string(&mut body)
        .is_err()
    {
        return Err("the answer was cut short".into());
    }
    parse_release(&body)
}

pub fn parse_release(body: &str) -> Result<Release, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "the answer could not be read".to_string())?;
    Release::from_json(&value)
}

impl Release {
    fn from_json(value: &serde_json::Value) -> Result<Release, String> {
        let tag = value
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if tag.is_empty() {
            // A draft with no tag, or an answer that is not a release at all.
            return Err("the newest release carries no version tag".into());
        }
        let notes = value
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let assets = value
            .get("assets")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|a| {
                        let name = a.get("name")?.as_str()?.to_string();
                        let url = a.get("browser_download_url")?.as_str()?.to_string();
                        let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                        Some(Asset { name, url, size })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Release {
            version: display_version(&tag),
            published: pretty_date(value.get("published_at").and_then(|v| v.as_str()).unwrap_or("")),
            page: value
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or(RELEASES_PAGE)
                .to_string(),
            tag,
            notes,
            assets,
        })
    }
}

// ---------------------------------------------------------------------------
// Downloading and checking the file
// ---------------------------------------------------------------------------

/// Reads `asset` into `dest`, reporting how much has arrived. The report is a
/// ratio from 0 to 1 (or -1 when the size is unknown) and the bytes so far.
pub fn download(
    asset: &Asset,
    dest: &Path,
    report: &mut dyn FnMut(f64, u64),
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("the download folder could not be made: {e}"))?;
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .build();
    let response = match agent
        .get(&asset.url)
        .set("User-Agent", USER_AGENT)
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Err("that download has gone from the release page".into())
        }
        Err(ureq::Error::Status(code, _)) => return Err(format!("the download answered {code}")),
        Err(_) => return Err("the download could not be started — check the connection".into()),
    };
    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(asset.size);
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("the file could not be written: {e}"))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut got: u64 = 0;
    let mut last_ratio = 0.0f64;
    loop {
        let n = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return Err("the download stopped partway".into()),
        };
        file.write_all(&buffer[..n])
            .map_err(|e| format!("the file could not be written: {e}"))?;
        got += n as u64;
        let ratio = if total > 0 {
            (got as f64 / total as f64).clamp(0.0, 1.0)
        } else {
            -1.0
        };
        // A report per 64 kB would flood the window; a percent at a time is
        // enough for a bar that moves smoothly.
        if ratio < 0.0 || ratio - last_ratio >= 0.02 || got as u64 == total {
            last_ratio = ratio;
            report(ratio, got);
        }
    }
    file.flush()
        .map_err(|e| format!("the file could not be finished: {e}"))?;
    Ok(got)
}

/// SHA-256 of a file, as 64 lowercase hex characters — the form GitHub
/// publishes beside each download.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{e}"))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| format!("{e}"))?;
    Ok(sha256_hex(&data))
}

pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = Vec::with_capacity(data.len() + 72);
    message.extend_from_slice(data);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

/// The `.sha256` file is a fingerprint followed by the file name, sometimes
/// with a `*` or a `sha256:` in front.
pub fn published_fingerprint(text: &str) -> Option<String> {
    let cleaned = text.trim();
    for token in cleaned.split_whitespace() {
        let token = token.trim_start_matches("sha256:").trim_start_matches('*');
        if token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(token.to_ascii_lowercase());
        }
    }
    None
}

pub fn fetch_published_fingerprint(asset: &Asset) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .build();
    let response = agent
        .get(&asset.url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|_| "the checksum could not be downloaded".to_string())?;
    let mut body = String::new();
    response
        .into_reader()
        .take(8 * 1024)
        .read_to_string(&mut body)
        .map_err(|_| "the checksum could not be read".to_string())?;
    published_fingerprint(&body).ok_or_else(|| "the published checksum was empty".to_string())
}

// ---------------------------------------------------------------------------
// Putting it in place
// ---------------------------------------------------------------------------

/// Where a new copy belongs: the program that is running, unless that is a
/// build straight out of the source tree — then the installed copy, because
/// replacing a file in a build folder would fool the next build into skipping
/// its work.
pub fn install_target() -> PathBuf {
    // A portable copy can say where it wants to live.
    if let Ok(custom) = std::env::var("VLCCRAFT_UPDATE_DEST") {
        let custom = custom.trim();
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let in_build_folder = exe.components().any(|c| c.as_os_str() == "target");
        if !in_build_folder {
            return exe;
        }
    }
    let home = crate::platform::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if cfg!(target_os = "windows") {
        let local = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join("AppData/Local"));
        local.join("Programs").join("VLCCraft").join("vlccraft.exe")
    } else if cfg!(target_os = "macos") {
        home.join("Applications").join("VLCCraft.app")
    } else {
        home.join(".local").join("bin").join("vlccraft")
    }
}

pub enum Installed {
    /// The program was replaced; the new copy runs at the next start.
    Replaced { path: PathBuf },
    /// A disk image was fetched and opened; the person drags it across.
    Opened { path: PathBuf },
}

/// Unpacks the downloaded file and puts the program where the running copy
/// lives. `stage` says what is happening, for the progress line.
pub fn install(archive: &Path, stage: &mut dyn FnMut(&str)) -> Result<Installed, String> {
    match Package::for_this_system() {
        Package::DiskImage => {
            // An app inside /Applications cannot replace its own bundle while
            // it runs, and needs a permission prompt to try. Handing the disk
            // image to the system is the supported way for macOS.
            stage("Opening the disk image");
            open_path(archive);
            Ok(Installed::Opened {
                path: archive.to_path_buf(),
            })
        }
        Package::Tarball => {
            stage("Unpacking");
            let work = work_dir("unpack");
            let binary = extract_tarball(archive, &work)?;
            stage("Installing");
            let dest = install_target();
            replace_binary(&binary, &dest)?;
            let _ = std::fs::remove_dir_all(&work);
            Ok(Installed::Replaced { path: dest })
        }
        Package::Zip => {
            stage("Unpacking");
            let work = work_dir("unpack");
            let binary = extract_zip(archive, &work)?;
            stage("Installing");
            let dest = install_target();
            replace_binary(&binary, &dest)?;
            let _ = std::fs::remove_dir_all(&work);
            Ok(Installed::Replaced { path: dest })
        }
    }
}

/// A scratch folder beside the downloads, cleared of an earlier attempt first.
fn work_dir(tag: &str) -> PathBuf {
    let dir = crate::model::data_dir()
        .join("updates")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn extract_tarball(archive: &Path, into: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(into).map_err(|e| format!("the unpack folder could not be made: {e}"))?;
    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .status()
        .map_err(|_| "the downloaded archive could not be unpacked".to_string())?;
    if !status.success() {
        return Err("the downloaded archive could not be unpacked".into());
    }
    find_file(into, "vlccraft").ok_or_else(|| "the archive did not contain the program".to_string())
}

#[cfg(windows)]
fn extract_zip(archive: &Path, into: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(into).map_err(|e| format!("the unpack folder could not be made: {e}"))?;
    let script = format!(
        "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        archive.display(),
        into.display()
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|_| "the downloaded archive could not be unpacked".to_string())?;
    if !status.success() {
        return Err("the downloaded archive could not be unpacked".into());
    }
    find_file(into, "vlccraft.exe").ok_or_else(|| "the archive did not contain the program".to_string())
}

#[cfg(not(windows))]
fn extract_zip(archive: &Path, into: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(into).map_err(|e| format!("the unpack folder could not be made: {e}"))?;
    let status = std::process::Command::new("unzip")
        .arg("-oq")
        .arg(archive)
        .arg("-d")
        .arg(into)
        .status()
        .map_err(|_| "the downloaded archive could not be unpacked".to_string())?;
    if !status.success() {
        return Err("the downloaded archive could not be unpacked".into());
    }
    find_file(into, "vlccraft.exe")
        .or_else(|| find_file(into, "vlccraft"))
        .ok_or_else(|| "the archive did not contain the program".to_string())
}

/// Breadth-first, three folders deep: enough for `vlccraft-1.0.0-.../vlccraft`.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut queue = vec![(dir.to_path_buf(), 0usize)];
    while let Some((current, depth)) = queue.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < 3 {
                    queue.push((path, depth + 1));
                }
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(path);
            }
        }
    }
    None
}

/// Copies the new program over the old one. A running program cannot be
/// written to, but its name can be replaced: the new file is put in place
/// under a temporary name and then renamed over the old one, which the system
/// allows while the old copy is still open.
fn replace_binary(new: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("the program folder could not be made: {e}"))?;
    }
    if dest.is_dir() {
        return Err("the program lives inside an app bundle; open the download instead".into());
    }
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("vlccraft");
    let staging = dest.with_file_name(format!(".{name}.new"));
    std::fs::copy(new, &staging)
        .map_err(|e| format!("the new program could not be written: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(windows)]
    {
        // Windows will not let go of a running program, but will let it be
        // renamed; the leftover is cleared up on the next start.
        if dest.exists() {
            let old = dest.with_file_name(format!("{name}.old"));
            let _ = std::fs::remove_file(&old);
            if std::fs::rename(dest, &old).is_err() {
                let _ = std::fs::remove_file(&staging);
                return Err("the old copy is in use and could not be moved aside".into());
            }
        }
    }
    if let Err(e) = std::fs::rename(&staging, dest) {
        let _ = std::fs::remove_file(&staging);
        return Err(format!("the new program could not be put in place: {e}"));
    }
    Ok(())
}

/// Starts the program at `path`; the old window closes itself only once the
/// new copy is really running.
pub fn launch(path: &Path) -> std::io::Result<()> {
    std::process::Command::new(path).spawn().map(|_| ())
}

pub fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Where downloads are kept while they are checked and unpacked.
pub fn download_dir() -> PathBuf {
    crate::model::data_dir().join("updates")
}

/// A stand-in for a newer release, so the update card can be looked at without
/// waiting for one to be published. Only the picture run asks for it; it can
/// never appear in a window someone is using, and its download link is empty.
pub fn preview_release() -> Release {
    Release {
        version: "1.1.0".into(),
        tag: "v1.1.0".into(),
        notes: "A queue that survives a restart. Subtitle delay per show, remembered\nacross every episode. Keyboard focus no longer escapes the full-screen bands.\nSmaller fixes: the library remembers the view mode, and a moved file offers to\nrelink instead of forgetting its position.".into(),
        published: "18 November 2026".into(),
        page: RELEASES_PAGE.into(),
        assets: vec![Asset {
            name: "vlccraft-1.1.0-x86_64-linux.tar.gz".into(),
            url: String::new(),
            size: 7_335_725,
        }],
    }
}

// ---------------------------------------------------------------------------
// The same two questions, answered in a terminal
// ---------------------------------------------------------------------------

/// `--check-updates`: print what GitHub says, without opening a window.
pub fn cli_check() -> i32 {
    println!("this copy:      {}", CURRENT);
    match fetch_latest() {
        Ok(release) => {
            println!("newest release: {} ({})", release.version, release.tag);
            if !release.published.is_empty() {
                println!("published:      {}", release.published);
            }
            let state = if is_newer(&release.version, CURRENT) {
                "newer"
            } else if version_key(&release.version) == version_key(CURRENT) {
                "newest"
            } else {
                "ahead"
            };
            println!("status:         {state}");
            println!("page:           {}", release.page);
            match release.asset_for_this_machine() {
                Some(asset) => println!("download:       {} ({})", asset.name, asset.size_text()),
                None => println!("download:       none published for this system"),
            }
            0
        }
        Err(reason) => {
            println!("status:         unknown — {reason}");
            1
        }
    }
}

/// `--update`: fetch, check and install the newest release, saying what it did.
pub fn cli_update(force: bool) -> i32 {
    println!("this copy:      {}", CURRENT);
    let release = match fetch_latest() {
        Ok(release) => release,
        Err(reason) => {
            println!("update failed:  {reason}");
            return 1;
        }
    };
    println!("newest release: {} ({})", release.version, release.tag);
    if !is_newer(&release.version, CURRENT) {
        if !force {
            println!("nothing to do:  this copy already carries {}", CURRENT);
            return 0;
        }
        println!("installing anyway, because --force was asked for");
    }
    let Some(asset) = release.asset_for_this_machine() else {
        println!("update failed:  the release has no download for this system");
        return 1;
    };
    let target = install_target();
    println!("download:       {} ({})", asset.name, asset.size_text());
    println!("installing to:  {}", target.display());

    let file = download_dir().join(&asset.name);
    let mut marks = 0;
    match download(asset, &file, &mut |ratio, _bytes| {
        // A terminal is a poor place for a moving bar: one mark per tenth is
        // quieter and still shows that something is arriving.
        let step = (ratio.max(0.0) * 10.0) as i32;
        if step > marks {
            marks = step;
            print!("▇");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }) {
        Ok(bytes) => println!("\ndownloaded {:.1} MB", bytes as f64 / (1024.0 * 1024.0)),
        Err(reason) => {
            println!("update failed:  {reason}");
            return 1;
        }
    }

    match release.checksum_for(asset) {
        Some(sum_asset) => match fetch_published_fingerprint(sum_asset) {
            Ok(want) => match sha256_file(&file) {
                Ok(got) if got == want => println!("checksum:       matches the published fingerprint"),
                Ok(_) => {
                    println!("update failed:  the download does not match its published checksum");
                    let _ = std::fs::remove_file(&file);
                    return 1;
                }
                Err(reason) => {
                    println!("update failed:  the download could not be read back: {reason}");
                    return 1;
                }
            },
            Err(reason) => println!("checksum:       not checked — {reason}"),
        },
        None => println!("checksum:       not checked — the release publishes none for this file"),
    }

    match install(&file, &mut |stage| println!("{stage}…")) {
        Ok(Installed::Replaced { path }) => {
            let _ = std::fs::remove_file(&file);
            println!("done:           {} {} is in place", "VLCCraft", release.version);
            println!("                run {} to use it", path.display());
            0
        }
        Ok(Installed::Opened { path }) => {
            println!("done:           {} opened — drag VLCCraft across to finish", path.display());
            0
        }
        Err(reason) => {
            println!("update failed:  {reason}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_the_way_a_release_page_does() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("v1.1.0", "1.0.9"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("v1.0.0-beta.2", "1.0.0"));
        assert!(is_newer("1.0.0", "0.9"));
        assert_eq!(version_key("v2.3"), (2, 3, 0));
        assert_eq!(display_version("v1.4.2"), "1.4.2");
    }

    #[test]
    fn reads_a_release_answer() {
        let body = r#"{
            "tag_name": "v1.2.0",
            "name": "VLCCraft 1.2.0",
            "body": "**Fixes**\nQueue survives a restart.",
            "published_at": "2026-11-12T09:00:00Z",
            "html_url": "https://github.com/mjb73/vlccraft/releases/tag/v1.2.0",
            "assets": [
                {"name": "vlccraft-1.2.0-x86_64-linux.tar.gz", "browser_download_url": "https://example/a", "size": 1000},
                {"name": "vlccraft-1.2.0-x86_64-linux.tar.gz.sha256", "browser_download_url": "https://example/a.sha256", "size": 90},
                {"name": "vlccraft-1.2.0-windows-x64.zip", "browser_download_url": "https://example/w", "size": 900},
                {"name": "VLCCraft-1.2.0.dmg", "browser_download_url": "https://example/d", "size": 800}
            ]
        }"#;
        let release = parse_release(body).expect("release should parse");
        assert_eq!(release.version, "1.2.0");
        assert_eq!(release.published, "12 November 2026");
        assert_eq!(release.assets.len(), 4);
        let asset = release.asset_for_this_machine().expect("a file for this system");
        assert!(!asset.name.ends_with(".sha256"));
        let checksum = release.checksum_for(asset).expect("the fingerprint beside it");
        assert!(checksum.name.ends_with(".sha256"));
        let excerpt = release.notes_excerpt(200);
        assert!(excerpt.contains("Queue survives a restart"), "{excerpt}");
        assert!(!excerpt.contains('*'));
    }

    #[test]
    fn hashes_like_sha256sum() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
        // Long enough to need more than one block of padding.
        assert_eq!(
            sha256_hex(&vec![b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn reads_a_published_fingerprint() {
        let body = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  vlccraft-1.0.0-x86_64-linux.tar.gz\n";
        assert_eq!(
            published_fingerprint(body).as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        // The `<hash> *<file>` form `sha256sum -b` writes.
        assert!(published_fingerprint(
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855 *a.iso"
        )
        .is_some());
        assert_eq!(published_fingerprint("no fingerprint here"), None);
    }

    #[test]
    fn dates_are_read_as_words_and_bad_ones_are_dropped() {
        assert_eq!(pretty_date("2026-11-10T18:05:26Z"), "10 November 2026");
        assert_eq!(pretty_date("2026-01-02"), "2 January 2026");
        assert_eq!(pretty_date(""), "");
        assert_eq!(pretty_date("last tuesday"), "");
    }

    #[test]
    fn the_age_of_a_check_reads_in_words() {
        let now = 1_000_000_000_000u64;
        assert_eq!(ago(now, 0), "never");
        assert_eq!(ago(now, now - 30_000), "just now");
        assert_eq!(ago(now, now - 300_000), "5 minutes ago");
        assert_eq!(ago(now, now - 7_200_000), "2 hours ago");
        assert_eq!(ago(now, now - 2 * 86_400_000), "2 days ago");
    }
}
