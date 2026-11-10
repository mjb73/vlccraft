//! One running player.
//!
//! When a file is double-clicked the desktop starts `vlccraft <file>` again.
//! A second big window would be the wrong answer, so the newcomer hands the
//! paths to the player that is already open and leaves. If no player is
//! listening it takes the stage itself.
//!
//! The handshake also carries the build. A player left open from an earlier
//! build would hand the file to code that no longer matches the program on
//! disk, which is how a fixed interface keeps looking broken: the window the
//! person is looking at was started before the fix was built. Such a player
//! answers with a different build name, and the newcomer opens its own window
//! and takes over the door instead of feeding the old one.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

/// First line every current player sends as soon as a newcomer connects.
const HELLO: &str = "VLCCRAFT";

fn stamp() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// What became of a hand-over.
pub enum Forwarded {
    /// A player of this same build took the files.
    Delivered,
    /// Some player answered, but from another build.
    Stale,
    /// Nothing was listening on the door.
    Nobody,
}

#[cfg(unix)]
pub fn socket_path() -> PathBuf {
    use std::os::unix::fs::MetadataExt;
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("vlccraft-ipc.sock");
        }
    }
    let uid = std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(0);
    PathBuf::from(format!("/tmp/vlccraft-{uid}.sock"))
}

/// Sends paths to an already-running player and says which build answered.
#[cfg(unix)]
pub fn forward(paths: &[String]) -> Forwarded {
    use std::io::{BufRead, BufReader, Write};
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    if paths.is_empty() {
        return Forwarded::Nobody;
    }
    let Ok(mut stream) = UnixStream::connect(socket_path()) else {
        return Forwarded::Nobody;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(700)));

    // The greeting comes before anything is written, so a player from an
    // older build (which never greets) is never handed a path it would
    // mistake for a file to open.
    let expected = format!("{HELLO} {}", stamp());
    let mut greeting = String::new();
    match stream.try_clone() {
        Ok(clone) => {
            let mut reader = BufReader::new(clone);
            let _ = reader.read_line(&mut greeting);
        }
        Err(_) => return Forwarded::Stale,
    }
    if greeting.trim_end() != expected {
        return Forwarded::Stale;
    }

    let mut payload = String::new();
    for path in paths {
        // One path per line; newlines inside a filename are not supported by
        // any desktop launcher either, so they are flattened here.
        payload.push_str(&path.replace(['\n', '\r'], " "));
        payload.push('\n');
    }
    stream.set_nonblocking(false).ok();
    if stream.write_all(payload.as_bytes()).is_err() {
        return Forwarded::Stale;
    }
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    Forwarded::Delivered
}

/// Starts listening for hand-overs.
///
/// `take_over` is set when an earlier build answered the door: its window is
/// still open, but files opened from now on belong to this one, so the old
/// socket is replaced.
#[cfg(unix)]
pub fn listen(take_over: bool) -> Option<Receiver<Vec<String>>> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    let path = socket_path();
    if path.exists() {
        if take_over {
            let _ = std::fs::remove_file(&path);
        } else if UnixStream::connect(&path).is_ok() {
            return None;
        } else {
            // A socket left behind by a crashed run.
            let _ = std::fs::remove_file(&path);
        }
    }
    let mut listener = UnixListener::bind(&path).ok()?;
    let (tx, rx) = channel();
    let hello = format!("{HELLO} {}\n", stamp());
    let door = path.clone();
    std::thread::spawn(move || {
        use std::io::ErrorKind;
        listener.set_nonblocking(true).ok();
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let mut out = stream;
                    if out.write_all(hello.as_bytes()).is_err() {
                        continue;
                    }
                    let _ = out.flush();
                    let _ = out.set_nonblocking(false);
                    let _ = out.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                    let reader = BufReader::new(out);
                    let mut paths = Vec::new();
                    for line in reader.lines().map_while(|l| l.ok()) {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            paths.push(trimmed);
                        }
                    }
                    if !paths.is_empty() {
                        let _ = tx.send(paths);
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    // Another window of an earlier build can delete this path
                    // when it closes. Put the door back so a later file still
                    // arrives here rather than in a third window.
                    if !door.exists() {
                        if let Ok(fresh) = UnixListener::bind(&door) {
                            fresh.set_nonblocking(true).ok();
                            listener = fresh;
                        }
                    }
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
            }
        }
    });
    Some(rx)
}

// ---------------------------------------------------------------------------
// Windows (and anything else without unix sockets)
// ---------------------------------------------------------------------------
//
// The same handshake over a loopback port: a second launch knocks, the player
// that is already open answers with its build, and the newcomer either hands
// its files over or opens its own window. One port is enough — a desktop has a
// single person at it.

/// The door a second launch knocks on.
#[cfg(not(unix))]
const TCP_PORT: u16 = 47823;

#[cfg(not(unix))]
pub fn forward(paths: &[String]) -> Forwarded {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{Shutdown, SocketAddr, TcpStream};
    use std::time::Duration;

    if paths.is_empty() {
        return Forwarded::Nobody;
    }
    let addr = SocketAddr::from(([127, 0, 0, 1], TCP_PORT));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) else {
        return Forwarded::Nobody;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(700)));

    let expected = format!("{HELLO} {}", stamp());
    let mut greeting = String::new();
    match stream.try_clone() {
        Ok(clone) => {
            let mut reader = BufReader::new(clone);
            let _ = reader.read_line(&mut greeting);
        }
        Err(_) => return Forwarded::Stale,
    }
    if greeting.trim_end() != expected {
        return Forwarded::Stale;
    }

    let mut payload = String::new();
    for path in paths {
        payload.push_str(&path.replace(['\n', '\r'], " "));
        payload.push('\n');
    }
    if stream.write_all(payload.as_bytes()).is_err() {
        return Forwarded::Stale;
    }
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    Forwarded::Delivered
}

#[cfg(not(unix))]
pub fn listen(_take_over: bool) -> Option<Receiver<Vec<String>>> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{SocketAddr, TcpListener};

    // The port is held by whichever window is open. When it is already taken
    // this one leaves the door alone: the window the person is looking at is
    // the right place for a double-clicked file.
    let addr = SocketAddr::from(([127, 0, 0, 1], TCP_PORT));
    let listener = TcpListener::bind(addr).ok()?;
    let (tx, rx) = channel();
    let hello = format!("{HELLO} {}\n", stamp());
    std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let mut out = stream;
                    if out.write_all(hello.as_bytes()).is_err() {
                        continue;
                    }
                    let _ = out.flush();
                    let _ = out.set_nonblocking(false);
                    let _ = out.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                    let reader = BufReader::new(out);
                    let mut paths = Vec::new();
                    for line in reader.lines().map_while(|l| l.ok()) {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            paths.push(trimmed);
                        }
                    }
                    if !paths.is_empty() {
                        let _ = tx.send(paths);
                    }
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
            }
        }
    });
    Some(rx)
}

#[cfg(not(unix))]
pub fn cleanup() {}

/// Removes the socket on a clean exit so a later launch does not have to
/// probe a dead path.
#[cfg(unix)]
pub fn cleanup() {
    // Only a door nobody answers belongs to a run that has gone; a door
    // another window is still holding must stay.
    use std::os::unix::net::UnixStream;
    let path = socket_path();
    if path.exists() && UnixStream::connect(&path).is_err() {
        let _ = std::fs::remove_file(&path);
    }
}

