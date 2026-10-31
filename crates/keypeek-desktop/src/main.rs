#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use keypeek_presentation::{
    FileSettingsStore, OverlayCommand, ReadOnlySettingsStore, SettingsStore,
};
use keypeek_protocol::{discover_devices, os_layout, scan_all_hid};

/// What this invocation does: run the overlay, or drive a running instance.
enum Invocation {
    Run { config: Option<PathBuf> },
    Control(OverlayCommand),
}

/// Parses the command line into a run or a control invocation.
///
/// Exits with status 2 on an unknown argument, a missing value, or a control
/// command combined with `--config`, so a typo never silently starts KeyPeek
/// with the wrong configuration.
fn parse_args() -> Invocation {
    let mut config = None;
    let mut control = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => match args.next() {
                Some(path) => config = Some(PathBuf::from(path)),
                None => usage_error("--config requires a file path"),
            },
            "--toggle" => control = Some(OverlayCommand::Toggle),
            "--show" => control = Some(OverlayCommand::Show),
            "--hide" => control = Some(OverlayCommand::Hide),
            "--auto" => control = Some(OverlayCommand::Auto),
            "--settings" => control = Some(OverlayCommand::Settings),
            other => usage_error(&format!(
                "unrecognized argument `{other}`; usage: keypeek [--config <path>] \
                 [--toggle|--show|--hide|--auto|--settings]"
            )),
        }
    }

    match (control, config) {
        (Some(_), Some(_)) => usage_error("--config cannot be combined with a control command"),
        (Some(command), None) => Invocation::Control(command),
        (None, config) => Invocation::Run { config },
    }
}

fn usage_error(message: &str) -> ! {
    eprintln!("KeyPeek: {message}");
    std::process::exit(2);
}

#[cfg(unix)]
fn send_control(command: OverlayCommand) {
    if let Err(e) = keypeek_desktop::ipc::send(command) {
        eprintln!(
            "KeyPeek: cannot send `{}` to {}: {e}",
            command.as_str(),
            keypeek_desktop::ipc::socket_path().display()
        );
        std::process::exit(1);
    }
}

#[cfg(not(unix))]
fn send_control(_command: OverlayCommand) {
    eprintln!("KeyPeek: overlay control is only available on Unix systems");
    std::process::exit(1);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match parse_args() {
        Invocation::Control(command) => {
            send_control(command);
            Ok(())
        }
        Invocation::Run { config } => {
            os_layout::init();
            let settings_store: Arc<dyn SettingsStore> = match config {
                Some(path) => Arc::new(ReadOnlySettingsStore::new(path)),
                None => Arc::new(FileSettingsStore::default()),
            };
            let available_devices = discover_devices(scan_all_hid());
            keypeek_desktop::run(settings_store, available_devices)
        }
    }
}
