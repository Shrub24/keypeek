#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use keypeek_presentation::{FileSettingsStore, ReadOnlySettingsStore, SettingsStore};
use keypeek_protocol::{discover_devices, os_layout, scan_all_hid};

/// Returns the path passed with `--config`, if any.
///
/// Exits with status 2 on an unknown argument or a missing value, so a typo
/// never silently starts KeyPeek with the default configuration.
fn config_path_from_args() -> Option<PathBuf> {
    let mut config_path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => match args.next() {
                Some(path) => config_path = Some(PathBuf::from(path)),
                None => usage_error("--config requires a file path"),
            },
            other => usage_error(&format!(
                "unrecognized argument `{other}`; usage: keypeek [--config <path>]"
            )),
        }
    }
    config_path
}

fn usage_error(message: &str) -> ! {
    eprintln!("KeyPeek: {message}");
    std::process::exit(2);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    os_layout::init();
    let settings_store: Arc<dyn SettingsStore> = match config_path_from_args() {
        Some(path) => Arc::new(ReadOnlySettingsStore::new(path)),
        None => Arc::new(FileSettingsStore::default()),
    };
    let available_devices = discover_devices(scan_all_hid());
    keypeek_desktop::run(settings_store, available_devices)
}
