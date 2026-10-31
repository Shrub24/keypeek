//! Local control socket for the running overlay instance.
//!
//! The instance listens on one Unix socket and reads a single command per
//! connection, so any local tool (a compositor key binding, a script) can show,
//! hide, or reset the overlay without knowing anything about QMK, ZMK, or the
//! UI. See `docs/roadmap.md` for why this is not D-Bus.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use keypeek_presentation::{CommandQueue, OverlayCommand};
use keypeek_protocol::UiWake;

/// Path of the control socket of the running instance.
pub fn socket_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("keypeek.sock");
    }
    // Without a runtime directory the temporary directory is shared between
    // users, so the name carries the user to keep their instances apart.
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    std::env::temp_dir().join(format!("keypeek-{user}.sock"))
}

/// Starts the control socket listener for the lifetime of the process.
///
/// Returns the bound path. Failure is not fatal: the overlay keeps working
/// without a control channel, it just cannot be driven from outside.
pub fn spawn(commands: Arc<CommandQueue>, ui_wake: UiWake) -> std::io::Result<PathBuf> {
    let path = socket_path();
    spawn_at(path.clone(), commands, ui_wake)?;
    Ok(path)
}

/// Sends a control command to the running instance.
pub fn send(command: OverlayCommand) -> std::io::Result<()> {
    send_to(&socket_path(), command)
}

fn spawn_at(
    path: PathBuf,
    commands: Arc<CommandQueue>,
    ui_wake: UiWake,
) -> std::io::Result<PathBuf> {
    let listener = bind(&path)?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            handle(stream, &commands, &ui_wake);
        }
    });
    Ok(path)
}

/// Binds the control socket, replacing a socket left behind by a dead process
/// and refusing to take over one that is still served.
fn bind(path: &Path) -> std::io::Result<UnixListener> {
    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            if UnixStream::connect(path).is_ok() {
                return Err(std::io::Error::new(
                    ErrorKind::AddrInUse,
                    format!(
                        "another KeyPeek instance is listening on {}",
                        path.display()
                    ),
                ));
            }
            std::fs::remove_file(path)?;
            UnixListener::bind(path)?
        }
        Err(e) => return Err(e),
    };
    // Only this user may drive the overlay.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

fn handle(stream: UnixStream, commands: &CommandQueue, ui_wake: &UiWake) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    if let Some(command) = OverlayCommand::parse(&line) {
        commands.push(command);
        ui_wake.request_repaint();
    }
}

fn send_to(path: &Path, command: OverlayCommand) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(command.as_str().as_bytes())?;
    stream.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn temp_socket(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keypeek-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    fn wait_for(queue: &CommandQueue) -> Vec<OverlayCommand> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut received = Vec::new();
        while Instant::now() < deadline {
            received = queue.drain();
            if !received.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        received
    }

    #[test]
    fn a_sent_command_reaches_the_queue_and_wakes_the_ui() {
        let path = temp_socket("roundtrip.sock");
        let commands = Arc::new(CommandQueue::default());
        let wakes = Arc::new(AtomicUsize::new(0));
        let ui_wake = UiWake::new({
            let wakes = wakes.clone();
            Arc::new(move || {
                wakes.fetch_add(1, Ordering::Relaxed);
            })
        });

        spawn_at(path.clone(), commands.clone(), ui_wake).expect("listen");
        send_to(&path, OverlayCommand::Toggle).expect("send");

        assert_eq!(wait_for(&commands), [OverlayCommand::Toggle]);
        assert!(wakes.load(Ordering::Relaxed) > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_socket_is_private_to_the_user() {
        let path = temp_socket("private.sock");
        spawn_at(path.clone(), Arc::default(), UiWake::default()).expect("listen");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_unknown_command_is_ignored() {
        let path = temp_socket("unknown.sock");
        let commands = Arc::new(CommandQueue::default());
        spawn_at(path.clone(), commands.clone(), UiWake::default()).expect("listen");

        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(b"explode\n").expect("write");
        drop(stream);

        assert!(wait_for(&commands).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_stale_socket_is_replaced() {
        let path = temp_socket("stale.sock");
        let stale = UnixListener::bind(&path).expect("bind");
        drop(stale);

        spawn_at(path.clone(), Arc::default(), UiWake::default()).expect("replace stale socket");
        assert!(UnixStream::connect(&path).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_served_socket_is_not_taken_over() {
        let path = temp_socket("served.sock");
        let _running = UnixListener::bind(&path).expect("bind");

        let err = spawn_at(path.clone(), Arc::default(), UiWake::default()).expect_err("must fail");
        assert_eq!(err.kind(), ErrorKind::AddrInUse);
        let _ = std::fs::remove_file(&path);
    }
}
