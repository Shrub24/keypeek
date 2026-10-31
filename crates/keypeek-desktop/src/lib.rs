//! Native desktop platform runtime, tray integration, and overlay hosting for KeyPeek.

use std::sync::Arc;

use keypeek_presentation::{
    FileSessionState, MemorySessionState, SessionStateStore, SettingsStore,
};
use keypeek_protocol::DiscoveredDevice;

pub mod eframe_host;
#[cfg(unix)]
pub mod ipc;
pub mod tray;

#[cfg(target_os = "linux")]
pub mod wayland;

/// Session state for the desktop runners: file-backed in the user's state
/// directory, in memory when no such directory is available.
pub fn session_state_store() -> Arc<dyn SessionStateStore> {
    match FileSessionState::default_path() {
        Some(path) => Arc::new(FileSessionState::new(path)),
        None => Arc::new(MemorySessionState::default()),
    }
}

pub fn run(
    settings_store: Arc<dyn SettingsStore>,
    devices: Vec<DiscoveredDevice>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        // `WAYLAND_DISPLAY` is unset under XWayland, so X11 falls through to eframe below.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            match wayland::run(settings_store.clone(), devices.clone()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // No wlr-layer-shell (e.g. GNOME/Mutter): fall back to eframe on
                    // XWayland, since native Wayland ignores always-on-top.
                    eprintln!(
                        "KeyPeek: Wayland layer-shell host unavailable ({e}); \
                         falling back to eframe on XWayland for always-on-top."
                    );
                    return Ok(eframe_host::run(settings_store, devices, true)?);
                }
            }
        }
    }

    eframe_host::run(settings_store, devices, false)?;
    Ok(())
}
