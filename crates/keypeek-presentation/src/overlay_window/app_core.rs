//! Shared application core state, connection lifecycle, and editor synchronization.

use std::sync::Arc;
use web_time::Instant;

use super::commands::{OverlayCommand, OverlayOverride};
use super::state::{SettingsState, UiState};
use super::ui_overlay::OverlayView;
use crate::key_paint::KeyPaintStyle;
use crate::keymap_editor::{EditorProfile, EditorState};
use crate::session_state::SessionStateStore;
use crate::settings::SettingsStore;
use keypeek_protocol::connection_manager::{
    ConnectOutcome, ConnectionEvent, DeviceConnectionManager,
};
use keypeek_protocol::{DeviceIdentity, DiscoveredDevice, Keyboard, UiWake};

/// Core state and business logic shared across presentation targets (Desktop and Web).
///
/// Encapsulates:
/// - UI alert state and visibility
/// - Active and draft settings
/// - Connection manager lifecycle and event dispatching
/// - Keymap editor lifecycle
pub struct KeyPeekAppCore {
    pub ui: UiState,
    pub settings: SettingsState,
    pub settings_store: Arc<dyn SettingsStore>,
    pub session_store: Arc<dyn SessionStateStore>,
    pub connection_mgr: DeviceConnectionManager,
    pub editor: EditorState,
}

impl KeyPeekAppCore {
    /// Creates a new `KeyPeekAppCore` with loaded settings and an initialized connection manager.    ///
    /// The keyboard used in the previous session is reconnected when it is present.
    pub fn new(
        ui_wake: UiWake,
        settings_store: Arc<dyn SettingsStore>,
        session_store: Arc<dyn SessionStateStore>,
        available_devices: Vec<DiscoveredDevice>,
        settings_visible: bool,
    ) -> Self {
        let base_settings = settings_store.load();
        let mut connection_mgr = DeviceConnectionManager::new(available_devices, ui_wake);
        let remembered = session_store
            .last_device()
            .and_then(|identity| matching_device_index(&connection_mgr, &identity));
        if let Some(index) = remembered {
            connection_mgr.select_device(index);
        }

        let mut core = Self {
            ui: UiState {
                settings_visible,
                settings_error: None,
                settings_warning: None,
                overlay_override: OverlayOverride::default(),
                available_monitors: Vec::new(),
            },
            settings: SettingsState::new(base_settings),
            settings_store,
            session_store,
            connection_mgr,
            editor: EditorState::new(),
        };

        // Only reconnect to the remembered keyboard; connecting to a device the
        // user did not choose is worse than starting disconnected.
        if remembered.is_some() {
            let _ = core.connect_selected_device();
        }

        core
    }

    /// Whether any modal or editor window/sidebar is open.
    pub fn is_any_window_open(&self) -> bool {
        self.ui.settings_visible || self.editor.is_open()
    }

    /// Persists the active settings to the configured settings store.
    pub fn persist_settings(&self) {
        if let Err(e) = self.settings_store.save(&self.settings.active) {
            eprintln!("Failed to save settings: {e}");
        }
    }

    /// Commits modified draft settings to active settings, updates connected keyboard config,
    /// and persists to the settings store. Returns `true` if changes were committed.
    pub fn sync_visual_settings(&mut self) -> bool {
        if self.settings.commit_draft() {
            if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
                keyboard.set_config(self.settings.overlay_config());
            }
            self.persist_settings();
            true
        } else {
            false
        }
    }

    /// Closes the editor window/sidebar and releases any device write lock.
    pub fn close_editor(&mut self) {
        self.editor
            .close(self.connection_mgr.connected_keyboard().as_deref());
    }

    /// Requests the editor window to close, initiating a ZMK save first if changes
    /// are pending; otherwise closes immediately.
    pub fn request_close_editor(&mut self) -> bool {
        self.editor
            .handle_close_request(self.connection_mgr.connected_keyboard().as_deref())
    }

    /// Switches the active keyboard layout and saves the preferred layout name.
    pub fn switch_layout(&mut self, name: &str) {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            if let Err(e) = keyboard.switch_layout(name) {
                self.ui.set_error(e);
            } else {
                self.connection_mgr
                    .set_preferred_layout_name(Some(name.to_string()));
            }
        }
    }

    /// Disconnects the active keyboard connection, tears down protocol session,
    /// and closes the keymap editor.
    pub fn disconnect_from_ui(&mut self) {
        self.close_editor();
        self.connection_mgr.disconnect();
        self.ui.clear_alerts();
    }

    /// Initiates connection for the currently selected device in connection_mgr.
    pub fn connect_selected_device(&mut self) -> ConnectOutcome {
        let outcome = self.connection_mgr.connect(self.settings.overlay_config());
        match &outcome {
            ConnectOutcome::Started => {
                self.ui.settings_error = None;
            }
            ConnectOutcome::AlreadyConnected => {
                self.ui
                    .set_warning("Already connected. Disconnect first to switch devices.");
            }
            ConnectOutcome::Failed(e) => {
                self.ui.set_error(e.clone());
            }
            ConnectOutcome::RequiresLayoutFile => {}
        }
        outcome
    }

    /// Checks if the keyboard overlay should be visible according to the manual
    /// override, open windows, or active layer hold timeouts.
    pub fn is_overlay_visible(&self) -> bool {
        let Some(keyboard) = self.connection_mgr.connected_keyboard() else {
            // Nothing to draw without a connected keyboard.
            return false;
        };
        if self.is_any_window_open() {
            // Windows need the overlay: the key editor selects keys by clicking them.
            return true;
        }
        match self.ui.overlay_override {
            OverlayOverride::Auto => keyboard.overlay_is_visible(Instant::now()),
            OverlayOverride::Shown => true,
            OverlayOverride::Hidden => false,
        }
    }

    /// Applies a control request from the tray, the command line, or the control socket.
    pub fn apply_overlay_command(&mut self, command: OverlayCommand) {
        if command == OverlayCommand::Settings {
            self.ui.settings_visible = true;
            return;
        }
        self.ui.overlay_override = self
            .ui
            .overlay_override
            .apply(command, self.is_overlay_visible());
    }

    /// Records the outputs the platform offers as overlay targets.
    pub fn set_available_monitors(&mut self, mut monitors: Vec<String>) {
        monitors.sort();
        monitors.dedup();
        if self.ui.available_monitors != monitors {
            self.ui.available_monitors = monitors;
        }
    }

    /// Records the identity of the keyboard that just connected, so the next start
    /// reconnects to it.
    fn remember_connected_device(&self) {
        if let Some(device) = self.connection_mgr.selected_device() {
            self.session_store
                .set_last_device(&DeviceIdentity::from_device(device));
        }
    }

    /// Schedules a repaint when the overlay visibility timer expires.
    pub fn schedule_overlay_repaint(&self, ctx: &egui::Context) {
        if self.is_any_window_open() {
            return;
        }

        let Some(keyboard) = self.connection_mgr.connected_keyboard() else {
            return;
        };

        if let Some(delay) = keyboard.overlay_changes_in(Instant::now()) {
            ctx.request_repaint_after(delay);
        }
    }

    /// Polls the connected keyboard, advances the connection state machine,
    /// and dispatches standard lifecycle events (`Connected`, `ConnectionFailed`,
    /// `Disconnected`, `DeviceLocked`).
    ///
    /// Returns any emitted [`ConnectionEvent`] for platform-specific handling
    /// (e.g. file picking or clearing platform state).
    pub fn update_connection(&mut self, ctx: &egui::Context) -> Option<ConnectionEvent> {
        if let Some(keyboard) = self.connection_mgr.connected_keyboard() {
            keyboard.poll();
        }

        let event = self
            .connection_mgr
            .update(self.settings.overlay_config(), |d| {
                ctx.request_repaint_after(d);
            });

        if let Some(event) = &event {
            match event {
                ConnectionEvent::Connected => {
                    self.ui.clear_alerts();
                    self.sync_visual_settings();
                    self.remember_connected_device();
                }
                ConnectionEvent::ConnectionFailed(e) => {
                    self.ui.set_error(e.clone());
                }
                ConnectionEvent::Disconnected => {
                    self.close_editor();
                }
                ConnectionEvent::DeviceLocked(dev) => {
                    self.ui.set_error(dev.lock_message());
                }
                ConnectionEvent::RequiresLayoutFile(_) => {}
            }
        }

        if !self.connection_mgr.is_connected() && self.editor.is_open() {
            self.close_editor();
        }

        event
    }

    /// Creates an `OverlayView` component with active settings and the keymap editor.
    pub fn overlay_view<'a>(
        &'a mut self,
        keyboard: &'a Keyboard,
        size: f32,
        hit_test_enabled: bool,
    ) -> OverlayView<'a> {
        OverlayView::new(
            keyboard,
            &mut self.editor,
            &self.settings.active,
            size,
            hit_test_enabled,
        )
    }

    /// Creates a `KeyPaintStyle` for the specified unit size from active settings.
    pub fn paint_style(&self, unit: f32) -> KeyPaintStyle {
        self.settings.paint_style(unit)
    }

    /// Draws the keymap editor in a floating window (desktop).
    pub fn draw_editor_window(
        &mut self,
        ctx: &egui::Context,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
    ) {
        if self.editor.is_open() {
            let style = self.paint_style(crate::keymap_editor::KEY_UNIT);
            self.editor.draw_window(ctx, keyboard, profile, &style);
        }
    }

    /// Draws the keymap editor in a sidebar panel (web).
    pub fn draw_editor_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        keyboard: &Keyboard,
        profile: &dyn EditorProfile,
    ) {
        if self.editor.is_open() {
            let style = self.paint_style(crate::keymap_editor::KEY_UNIT);
            self.editor.draw_sidebar(ui, keyboard, profile, &style);
        }
    }
}

/// Index of the discovered device matching `identity`, if one is present.
fn matching_device_index(
    manager: &DeviceConnectionManager,
    identity: &DeviceIdentity,
) -> Option<usize> {
    manager
        .available_devices()
        .iter()
        .position(|device| identity.matches(device))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap_editor::EditTarget;
    use crate::session_state::MemorySessionState;
    use crate::settings::MemorySettingsStore;

    fn make_test_core(settings_visible: bool) -> KeyPeekAppCore {
        make_test_core_with(
            settings_visible,
            Vec::new(),
            Arc::new(MemorySessionState::default()),
        )
    }

    fn make_test_core_with(
        settings_visible: bool,
        devices: Vec<DiscoveredDevice>,
        session_store: Arc<dyn SessionStateStore>,
    ) -> KeyPeekAppCore {
        let ui_wake = UiWake::new(Arc::new(|| {}));
        let store = Arc::new(MemorySettingsStore::default());
        KeyPeekAppCore::new(ui_wake, store, session_store, devices, settings_visible)
    }

    fn test_device(name: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            base_name: name.to_string(),
            vid: 0x1d50,
            pid: 0x615e,
            driver_id: "zmk",
            protocol_label: "ZMK Serial",
            requires_layout_file: false,
            spec: keypeek_protocol::test_utils::test_spec(),
        }
    }

    #[test]
    fn a_remembered_keyboard_is_selected_on_start() {
        let session = Arc::new(MemorySessionState::default());
        session.set_last_device(&DeviceIdentity::from_device(&test_device("Corne")));
        let core = make_test_core_with(
            false,
            vec![test_device("Lily58"), test_device("Corne")],
            session,
        );

        let selected = core
            .connection_mgr
            .selected_device()
            .map(|d| d.base_name.clone());
        assert_eq!(selected.as_deref(), Some("Corne"));
    }

    #[test]
    fn an_absent_remembered_keyboard_leaves_nothing_selected() {
        let session = Arc::new(MemorySessionState::default());
        session.set_last_device(&DeviceIdentity::from_device(&test_device("Corne")));
        let core = make_test_core_with(false, vec![test_device("Lily58")], session);

        assert!(core.connection_mgr.selected_device().is_none());
        assert!(!core.ui.settings_visible);
    }

    #[test]
    fn a_connected_keyboard_is_remembered() {
        let session = Arc::new(MemorySessionState::default());
        let mut core = make_test_core_with(false, vec![test_device("Corne")], session.clone());
        core.connection_mgr.select_device(0);

        core.remember_connected_device();

        let remembered = session.last_device().expect("remembered");
        assert_eq!(remembered.name, "Corne");
        assert_eq!(
            remembered,
            DeviceIdentity::from_device(&test_device("Corne"))
        );
    }

    #[test]
    fn test_app_core_initialization() {
        let core = make_test_core(false);
        assert!(!core.is_any_window_open());
        assert!(!core.ui.settings_visible);
        assert!(core.connection_mgr.available_devices().is_empty());
    }

    #[test]
    fn test_is_any_window_open() {
        let mut core = make_test_core(false);
        assert!(!core.is_any_window_open());

        core.ui.settings_visible = true;
        assert!(core.is_any_window_open());

        core.ui.settings_visible = false;
        assert!(!core.is_any_window_open());

        core.editor.target = Some(EditTarget::new(0, 0, 0));
        assert!(core.is_any_window_open());
    }

    #[test]
    fn test_sync_visual_settings_persists() {
        let mut core = make_test_core(false);
        assert!(!core.sync_visual_settings());

        core.settings.draft.timeout = 7777;
        assert!(core.sync_visual_settings());
        assert_eq!(core.settings.active.timeout, 7777);

        let reloaded = core.settings_store.load();
        assert_eq!(reloaded.timeout, 7777);

        assert!(!core.sync_visual_settings());
    }

    #[test]
    fn test_disconnect_from_ui_clears_alerts() {
        let mut core = make_test_core(false);
        core.ui.set_error("Connection timed out");
        assert!(core.ui.settings_error.is_some());

        core.disconnect_from_ui();
        assert!(core.ui.settings_error.is_none());
        assert!(!core.editor.is_open());
    }

    #[test]
    fn test_connect_selected_device_without_selection() {
        let mut core = make_test_core(false);
        let outcome = core.connect_selected_device();
        assert!(matches!(outcome, ConnectOutcome::Failed(_)));
        assert!(core.ui.settings_error.is_some());
    }
}
