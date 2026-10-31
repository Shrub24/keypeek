//! Session state that survives a restart but is not configuration.
//!
//! Configuration is authoritative and may be read-only (a generated Nix file);
//! state is a consequence of use and lives in the user's state directory.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use keypeek_protocol::DeviceIdentity;

/// Remembers the keyboard used in an earlier session.
pub trait SessionStateStore: Send + Sync {
    fn last_device(&self) -> Option<DeviceIdentity>;
    fn set_last_device(&self, identity: &DeviceIdentity);
}

/// File-backed session state, stored as one line under the state directory.
#[cfg(feature = "directories")]
#[derive(Clone, Debug)]
pub struct FileSessionState {
    path: PathBuf,
}

#[cfg(feature = "directories")]
impl FileSessionState {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `<state dir>/keypeek/last-device`, for example
    /// `~/.local/state/keypeek/last-device`.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("dev", "srwi", "KeyPeek")
            .map(|dirs| dirs.state_dir().map(|dir| dir.join("last-device")))
            .flatten()
    }
}

#[cfg(feature = "directories")]
impl SessionStateStore for FileSessionState {
    fn last_device(&self) -> Option<DeviceIdentity> {
        let contents = std::fs::read_to_string(&self.path).ok()?;
        DeviceIdentity::parse(&contents)
    }

    fn set_last_device(&self, identity: &DeviceIdentity) {
        if let Err(e) = write(&self.path, &identity.to_wire()) {
            // Losing the record only costs an automatic connect next time.
            eprintln!(
                "KeyPeek: could not record the last device in {}: {e}",
                self.path.display()
            );
        }
    }
}

#[cfg(feature = "directories")]
fn write(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

/// In-memory session state for tests and the browser, where no durable
/// storage is available.
#[derive(Default)]
pub struct MemorySessionState(Mutex<Option<DeviceIdentity>>);

impl SessionStateStore for MemorySessionState {
    fn last_device(&self) -> Option<DeviceIdentity> {
        self.0.lock().ok().and_then(|last| last.clone())
    }

    fn set_last_device(&self, identity: &DeviceIdentity) {
        if let Ok(mut last) = self.0.lock() {
            *last = Some(identity.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keypeek_protocol::{ConnectionSpec, DiscoveredDevice};

    fn device(driver_id: &'static str, vid: u16, pid: u16, name: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            base_name: name.to_string(),
            vid,
            pid,
            driver_id,
            protocol_label: driver_id,
            requires_layout_file: false,
            spec: ConnectionSpec::Vial { vid, pid },
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keypeek-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn identity_round_trips_through_the_file_store() {
        let path = temp_path("last-device");
        let store = FileSessionState::new(&path);
        assert_eq!(store.last_device(), None);

        let identity = DeviceIdentity::from_device(&device("zmk", 0x1d50, 0x615e, "Corne"));
        store.set_last_device(&identity);

        let reloaded = FileSessionState::new(&path)
            .last_device()
            .expect("remembered");
        assert_eq!(reloaded, identity);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_file_store_ignores_unreadable_content() {
        let path = temp_path("garbage");
        std::fs::write(&path, "not an identity\n").expect("write");
        assert_eq!(FileSessionState::new(&path).last_device(), None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_file_store_creates_missing_directories() {
        let path = temp_path("nested/deeper/last-device");
        let store = FileSessionState::new(&path);
        let identity = DeviceIdentity::from_device(&device("zmk", 0x1d50, 0x615e, "Corne"));
        store.set_last_device(&identity);
        assert_eq!(store.last_device(), Some(identity));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn memory_store_remembers_within_the_process() {
        let store = MemorySessionState::default();
        assert_eq!(store.last_device(), None);
        let identity = DeviceIdentity::from_device(&device("zmk", 0x1d50, 0x615e, "Corne"));
        store.set_last_device(&identity);
        assert_eq!(store.last_device(), Some(identity));
    }
}
