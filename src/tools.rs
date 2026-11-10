//! Convert and stream-out jobs.
//!
//! Both drive the *installed* VLC executable in a separate process, exactly the
//! way the player's own Convert / Stream dialogs do. Nothing in /usr is read,
//! patched or replaced: the job is a child of this process, its log lives in
//! the app's own data folder, and stopping it only signals that child.

use crate::model::data_dir;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Convert,
    Stream,
}

pub struct ConversionPreset {
    pub name: String,
    pub detail: String,
    pub ext: &'static str,
    pub sout: &'static str,
}

pub fn conversion_presets() -> Vec<ConversionPreset> {
    vec![
        ConversionPreset {
            name: "MP4 · H.264 1080p".into(),
            detail: "Plays everywhere · about 4 Mbit/s".into(),
            ext: "mp4",
            sout: "#transcode{vcodec=h264,vb=4000,acodec=mp4a,ab=160,channels=2,samplerate=48000}:standard{access=file,mux=mp4,dst=%DST%}",
        },
        ConversionPreset {
            name: "MP4 · H.264 720p".into(),
            detail: "Smaller file · about 2.5 Mbit/s".into(),
            ext: "mp4",
            sout: "#transcode{vcodec=h264,vb=2500,width=1280,height=720,acodec=mp4a,ab=128,channels=2,samplerate=48000}:standard{access=file,mux=mp4,dst=%DST%}",
        },
        ConversionPreset {
            name: "WebM · VP8 + Vorbis".into(),
            detail: "Open format · good for the web".into(),
            ext: "webm",
            sout: "#transcode{vcodec=VP80,vb=2500,acodec=vorb,ab=128,channels=2,samplerate=48000}:standard{access=file,mux=webm,dst=%DST%}",
        },
        ConversionPreset {
            name: "Audio only · MP3 192".into(),
            detail: "Strip the picture · 192 kbit/s".into(),
            ext: "mp3",
            sout: "#transcode{acodec=mp3,ab=192,channels=2,samplerate=44100}:standard{access=file,mux=raw,dst=%DST%}",
        },
        ConversionPreset {
            name: "Audio only · M4A 192".into(),
            detail: "Strip the picture · AAC in MP4".into(),
            ext: "m4a",
            sout: "#transcode{acodec=mp4a,ab=192,channels=2,samplerate=48000}:standard{access=file,mux=mp4,dst=%DST%}",
        },
    ]
}

pub struct SideJob {
    pub kind: JobKind,
    pub child: Child,
    pub log: PathBuf,
    pub label: String,
    pub started: Instant,
    pub url: Option<String>,
}

impl SideJob {
    pub fn elapsed_secs(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// Stop the child we started. No other process is touched.
    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for SideJob {
    /// Closing the window must not leave a stream running (or a converter
    /// writing) after the interface is gone: the child belongs to this app.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn vlc_binary() -> Result<PathBuf, String> {
    // The official player, not a bundled copy: the same engine the system
    // already has, told to run without an interface of its own.
    for candidate in crate::platform::vlc_binary_candidates() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    // Then whatever the PATH offers, using the separator this system uses.
    let names = crate::platform::vlc_binary_names();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in names {
                let p = dir.join(name);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Err(format!(
        "VLC could not be found, so there is no engine to convert or stream with. {}",
        crate::platform::vlc_missing_hint()
    ))
}

fn check_dst(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Choose where the new file should go".into());
    }
    if path.contains('{') || path.contains('}') || path.contains(',') {
        return Err("A path cannot contain braces or commas in VLC's output chain — rename the folder or file".into());
    }
    let parent = Path::new(path).parent().map(|p| p.to_path_buf());
    if let Some(parent) = parent {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!("The folder {} does not exist yet", parent.display()));
        }
    }
    Ok(())
}

fn spawn(
    kind: JobKind,
    args: Vec<String>,
    log: PathBuf,
    label: String,
    url: Option<String>,
) -> Result<SideJob, String> {
    let bin = vlc_binary()?;
    let dir = data_dir().join("jobs");
    let _ = std::fs::create_dir_all(&dir);
    let out = File::create(&log).map_err(|e| format!("Could not write the job log: {e}"))?;
    let err = out.try_clone().map_err(|e| e.to_string())?;

    // VLC is told not to open an interface of its own: this app is the
    // interface. It only decodes, converts or streams the media it is given.
    let mut cmd = Command::new(&bin);
    cmd.arg("-I")
        .arg("dummy")
        .arg("--no-video-title-show")
        .arg("--no-osd")
        .arg("--no-snapshot-preview")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", bin.display()))?;

    Ok(SideJob {
        kind,
        child,
        log,
        label,
        started: Instant::now(),
        url,
    })
}

/// Convert one file to a new file. The source is never modified.
pub fn start_convert(
    input: &str,
    output: &str,
    preset: &ConversionPreset,
) -> Result<SideJob, String> {
    if !Path::new(input).exists() {
        return Err("The file to convert is not there any more".into());
    }
    check_dst(output)?;
    if let (Ok(a), Ok(b)) = (
        std::fs::canonicalize(input),
        std::fs::canonicalize(output),
    ) {
        if a == b {
            return Err("Pick a different name: the original is never overwritten".into());
        }
    }
    let sout = preset.sout.replace("%DST%", output);
    let log = data_dir().join("jobs").join(format!(
        "convert-{}.log",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    ));
    spawn(
        JobKind::Convert,
        vec![
            "--sout".into(),
            sout,
            "--sout-keep".into(),
            input.to_string(),
            "vlc://quit".into(),
        ],
        log,
        format!("Converting to {}", file_label(output)),
        None,
    )
}

/// Stream one file over HTTP, on the local network.
pub fn start_stream(input: &str, port: u16, transcode: bool) -> Result<SideJob, String> {
    if input.trim().is_empty() {
        return Err("Choose what to stream".into());
    }
    if !input.contains("://") && !Path::new(input).exists() {
        return Err("The file to stream is not there any more".into());
    }
    let chain = if transcode {
        format!("#transcode{{vcodec=h264,vb=2500,acodec=mp3,ab=128,channels=2,samplerate=44100}}:standard{{access=http,mux=ts,dst=0.0.0.0:{port}}}")
    } else {
        format!("#standard{{access=http,mux=ts,dst=0.0.0.0:{port}}}")
    };
    let url = format!("http://{}:{}/", local_ip(), port);
    let log = data_dir().join("jobs").join(format!(
        "stream-{}.log",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    ));
    spawn(
        JobKind::Stream,
        vec![
            "--loop".into(),
            "--sout".into(),
            chain,
            "--sout-keep".into(),
            input.to_string(),
        ],
        log,
        format!("Streaming on port {port}"),
        Some(url),
    )
}

pub fn file_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// The address a TV on the same network should open. A connected UDP socket
/// picks the outgoing interface without sending a single packet.
pub fn local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "this computer".into())
}

#[allow(dead_code)]
pub fn log_tail(path: &Path, lines: usize) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_a_file_through_the_installed_vlc() {
        let src = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/media/Films/Desert Road.mp4"
        );
        // Converting needs both the installed VLC and a clip to convert. The
        // clips are not in the repository, and CI has no VLC, so the test says
        // it was skipped instead of failing for a reason that is not the code.
        if !std::path::Path::new(src).exists() || crate::vlc::probe().is_err() {
            eprintln!("skipped: needs the demo clip and an installed VLC");
            return;
        }
        let out = concat!(env!("CARGO_MANIFEST_DIR"), "/shots/conv-unit.mp4");
        let _ = std::fs::remove_file(out);
        let log_dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/shots"));
        std::env::set_var("VLCCRAFT_DATA_DIR", &log_dir);
        let preset = conversion_presets().remove(1);
        let mut job = start_convert(src, out, &preset).expect("job starts");
        let status = job.child.wait().expect("child exits");
        assert!(status.success(), "VLC exited with {status}");
        let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
        assert!(size > 100_000, "output was {size} bytes");
        let _ = std::fs::remove_file(out);
    }
}
