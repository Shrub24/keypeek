#[cfg(feature = "directories")]
use directories::ProjectDirs;
use ini::Ini;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

#[derive(Debug)]
pub struct ParseSettingsError;

/// Output the overlay is shown on.
///
/// Persisted by name rather than index: indices shift when a display is
/// plugged in or reordered, names do not.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum MonitorSelection {
    /// Let the display server pick (Wayland has no primary-output flag).
    #[default]
    Primary,
    Named(String),
}

impl fmt::Display for MonitorSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Named(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for MonitorSelection {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "" | "Primary" => Self::Primary,
            name => Self::Named(name.to_string()),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Bottom,
    Top,
    Center,
}

impl WindowPosition {
    pub const ALL: [Self; 7] = [
        Self::Center,
        Self::Top,
        Self::Bottom,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    /// Alignment plus displacement for this anchor.
    ///
    /// Each offset is the distance inward from the edge the anchor uses on that
    /// axis, so a negative value moves the overlay outward, past the edge. The
    /// three anchors without a horizontal edge treat `offset_x` as a plain
    /// displacement.
    pub fn anchor_and_offset(self, offset_x: f32, offset_y: f32) -> (egui::Align2, egui::Vec2) {
        match self {
            Self::TopLeft => (egui::Align2::LEFT_TOP, egui::vec2(offset_x, offset_y)),
            Self::TopRight => (egui::Align2::RIGHT_TOP, egui::vec2(-offset_x, offset_y)),
            Self::BottomLeft => (egui::Align2::LEFT_BOTTOM, egui::vec2(offset_x, -offset_y)),
            Self::BottomRight => (egui::Align2::RIGHT_BOTTOM, egui::vec2(-offset_x, -offset_y)),
            Self::Bottom => (egui::Align2::CENTER_BOTTOM, egui::vec2(offset_x, -offset_y)),
            Self::Top => (egui::Align2::CENTER_TOP, egui::vec2(offset_x, offset_y)),
            Self::Center => (egui::Align2::CENTER_CENTER, egui::vec2(offset_x, offset_y)),
        }
    }
}

impl fmt::Display for WindowPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WindowPosition::TopLeft => "Top Left",
                WindowPosition::TopRight => "Top Right",
                WindowPosition::BottomLeft => "Bottom Left",
                WindowPosition::BottomRight => "Bottom Right",
                WindowPosition::Bottom => "Bottom",
                WindowPosition::Top => "Top",
                WindowPosition::Center => "Center",
            }
        )
    }
}

impl FromStr for WindowPosition {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Top Left" => Ok(WindowPosition::TopLeft),
            "Top Right" => Ok(WindowPosition::TopRight),
            "Bottom Left" => Ok(WindowPosition::BottomLeft),
            "Bottom Right" => Ok(WindowPosition::BottomRight),
            "Bottom" => Ok(WindowPosition::Bottom),
            "Top" => Ok(WindowPosition::Top),
            "Center" => Ok(WindowPosition::Center),
            _ => Err(ParseSettingsError),
        }
    }
}

/// How a key with more than one legend (a native Shift pair, or an OS-resolved
/// RAlt result) is displayed. An enum, not two bools: only three of the four
/// combinations are meaningful.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegendMode {
    /// Base+Shifted stacked, as always (default).
    Stacked,
    /// Only what a plain tap produces.
    Single,
    /// Single, plus live-swap to the Shift/RAlt result while that modifier
    /// is physically held anywhere on the keyboard.
    SingleLive,
}

impl LegendMode {
    pub const ALL: [LegendMode; 3] = [
        LegendMode::Stacked,
        LegendMode::Single,
        LegendMode::SingleLive,
    ];
}

impl fmt::Display for LegendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LegendMode::Stacked => "Stacked",
                LegendMode::Single => "Single",
                LegendMode::SingleLive => "Single + live preview",
            }
        )
    }
}

impl FromStr for LegendMode {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Stacked" => Ok(LegendMode::Stacked),
            "Single" => Ok(LegendMode::Single),
            "Single + live preview" => Ok(LegendMode::SingleLive),
            _ => Err(ParseSettingsError),
        }
    }
}

/// Bitmask of the layers the overlay is shown for; bit `i` corresponds to layer `i`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LayerMask(u32);

impl LayerMask {
    pub const ALL: Self = Self(u32::MAX);

    pub fn bits(self) -> u32 {
        self.0
    }

    /// Whether any layer of the `layers` bitmask is shown.
    pub fn contains_any(self, layers: u32) -> bool {
        self.0 & layers != 0
    }

    pub fn set(&mut self, layers: u32, shown: bool) {
        if shown {
            self.0 |= layers;
        } else {
            self.0 &= !layers;
        }
    }
}

impl fmt::Display for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}

impl FromStr for LayerMask {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        u32::from_str_radix(value.strip_prefix("0x").unwrap_or(value), 16)
            .map(Self)
            .map_err(|_| ParseSettingsError)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl fmt::Display for ThemeColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{},{}", self.r, self.g, self.b, self.a)
    }
}

impl FromStr for ThemeColor {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(',').map(str::trim);
        let (Some(r), Some(g), Some(b), Some(a)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ParseSettingsError);
        };

        if parts.next().is_some() {
            return Err(ParseSettingsError);
        }

        Ok(Self {
            r: r.parse().map_err(|_| ParseSettingsError)?,
            g: g.parse().map_err(|_| ParseSettingsError)?,
            b: b.parse().map_err(|_| ParseSettingsError)?,
            a: a.parse().map_err(|_| ParseSettingsError)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ThemeSettings {
    pub layer_colors: [ThemeColor; 7],
    pub font_color: ThemeColor,
}

impl ThemeSettings {
    /// Layers from here up share `layer_colors`' last entry and a single visibility bit.
    pub const OTHER_LAYERS: u8 = 6;

    pub fn layer_color(&self, layer: u8) -> ThemeColor {
        *self
            .layer_colors
            .get(layer as usize)
            .unwrap_or(&self.layer_colors[Self::OTHER_LAYERS as usize])
    }
}

impl Default for ThemeSettings {
    fn default() -> Self {
        const ALPHA: u8 = 239;
        Self {
            layer_colors: [
                ThemeColor::new(83, 83, 83, ALPHA),
                ThemeColor::new(80, 140, 115, ALPHA),
                ThemeColor::new(100, 115, 150, ALPHA),
                ThemeColor::new(140, 110, 150, ALPHA),
                ThemeColor::new(95, 121, 127, ALPHA),
                ThemeColor::new(147, 137, 110, ALPHA),
                ThemeColor::new(127, 127, 127, ALPHA),
            ],
            font_color: ThemeColor::new(255, 255, 255, 255),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    pub size: i32,
    pub font_size_multiplier: f32,
    pub auto_fit_before_ellipsis: bool,
    pub position: WindowPosition,
    pub timeout: i64,
    pub activation_delay: u32,
    /// Distance inward from the anchored edge; negative moves past it.
    pub offset_x: i32,
    pub offset_y: i32,
    pub visible_layers: LayerMask,
    pub theme: ThemeSettings,
    pub legend_mode: LegendMode,
    pub monitor: MonitorSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: 60,
            font_size_multiplier: 1.0,
            auto_fit_before_ellipsis: false,
            #[cfg(target_arch = "wasm32")]
            position: WindowPosition::Center,
            #[cfg(not(target_arch = "wasm32"))]
            position: WindowPosition::BottomRight,
            timeout: 2000,
            activation_delay: 0,
            offset_x: 10,
            offset_y: 10,
            visible_layers: LayerMask::ALL,
            theme: ThemeSettings::default(),
            legend_mode: LegendMode::Stacked,
            monitor: MonitorSelection::default(),
        }
    }
}

impl Settings {
    /// Upper bound for `activation_delay`, shared with the settings UI.
    pub const MAX_ACTIVATION_DELAY_MS: u32 = 3_000;

    pub fn config_file_path() -> Option<PathBuf> {
        FileSettingsStore::default_config_path()
    }

    pub fn save(&self) -> std::io::Result<()> {
        FileSettingsStore::default()
            .save(self)
            .map_err(io::Error::other)
    }

    pub fn load() -> Option<Self> {
        FileSettingsStore::default().load_file()
    }

    pub fn to_ini(&self) -> Ini {
        let mut conf = Ini::new();
        let mut section = conf.with_section(Some("settings"));
        section.set("size", self.size.to_string());
        section.set(
            "font_size_multiplier",
            self.font_size_multiplier.to_string(),
        );
        section.set(
            "auto_fit_before_ellipsis",
            self.auto_fit_before_ellipsis.to_string(),
        );
        section.set("position", self.position.to_string());
        section.set("timeout", self.timeout.to_string());
        section.set("activation_delay", self.activation_delay.to_string());
        section.set("offset_x", self.offset_x.to_string());
        section.set("offset_y", self.offset_y.to_string());
        section.set("visible_layers", self.visible_layers.to_string());
        for (index, color) in self.theme.layer_colors.iter().enumerate() {
            section.set(format!("layer_color_{index}"), color.to_string());
        }
        section.set("font_color", self.theme.font_color.to_string());
        section.set("legend_mode", self.legend_mode.to_string());
        section.set("monitor", self.monitor.to_string());
        conf
    }

    pub fn to_ini_string(&self) -> String {
        let conf = self.to_ini();
        let mut buf = Vec::new();
        let _ = conf.write_to(&mut buf);
        String::from_utf8(buf).unwrap_or_default()
    }

    pub fn from_ini(conf: &Ini) -> Option<Self> {
        let section = conf.section(Some("settings"))?;
        let mut s = Settings::default();
        if let Some(val) = section.get("size") {
            s.size = val.parse().unwrap_or(s.size);
        }
        if let Some(val) = section.get("font_size_multiplier") {
            let parsed = val.parse::<f32>().unwrap_or(s.font_size_multiplier);
            s.font_size_multiplier = parsed.clamp(0.1, 2.0);
        }
        if let Some(val) = section.get("auto_fit_before_ellipsis") {
            s.auto_fit_before_ellipsis = val.parse().unwrap_or(s.auto_fit_before_ellipsis);
        }
        if let Some(val) = section.get("position") {
            if let Ok(parsed) = val.parse() {
                s.position = parsed;
            }
        }
        if let Some(val) = section.get("timeout") {
            let parsed = val.parse::<i64>().unwrap_or(s.timeout);
            s.timeout = if parsed < 0 {
                -1
            } else {
                parsed.clamp(0, 14_999)
            };
        }
        if let Some(val) = section.get("activation_delay") {
            let parsed = val.parse::<u32>().unwrap_or(s.activation_delay);
            s.activation_delay = parsed.min(Self::MAX_ACTIVATION_DELAY_MS);
        }
        // `margin` is the pre-offset single inset; keep honouring it so an older
        // file keeps its placement.
        if let Some(val) = section.get("margin") {
            let margin = val.parse().unwrap_or(s.offset_x);
            s.offset_x = margin;
            s.offset_y = margin;
        }
        if let Some(val) = section.get("offset_x") {
            s.offset_x = val.parse().unwrap_or(s.offset_x);
        }
        if let Some(val) = section.get("offset_y") {
            s.offset_y = val.parse().unwrap_or(s.offset_y);
        }
        if let Some(val) = section.get("visible_layers") {
            s.visible_layers = val.parse().unwrap_or(s.visible_layers);
        }
        for index in 0..s.theme.layer_colors.len() {
            if let Some(val) = section.get(format!("layer_color_{index}")) {
                if let Ok(parsed) = val.parse() {
                    s.theme.layer_colors[index] = parsed;
                }
            }
        }
        if let Some(val) = section.get("font_color") {
            if let Ok(parsed) = val.parse() {
                s.theme.font_color = parsed;
            }
        }
        if let Some(val) = section.get("legend_mode") {
            if let Ok(parsed) = val.parse() {
                s.legend_mode = parsed;
            }
        }
        if let Some(val) = section.get("monitor") {
            if let Ok(parsed) = val.parse() {
                s.monitor = parsed;
            }
        }
        Some(s)
    }

    pub fn from_ini_str(content: &str) -> Option<Self> {
        let conf = Ini::load_from_str(content).ok()?;
        Self::from_ini(&conf)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let conf = self.to_ini();
        conf.write_to_file(path)
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Option<Self> {
        let conf = Ini::load_from_file(path).ok()?;
        Self::from_ini(&conf)
    }
}

/// Storage interface for application settings.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings) -> Result<(), String>;
}

/// In-memory settings store for tests and environments without disk access.
#[derive(Debug)]
pub struct MemorySettingsStore {
    settings: RwLock<Settings>,
}

impl MemorySettingsStore {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: RwLock::new(settings),
        }
    }
}

impl Default for MemorySettingsStore {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl SettingsStore for MemorySettingsStore {
    fn load(&self) -> Settings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    fn save(&self, settings: &Settings) -> Result<(), String> {
        match self.settings.write() {
            Ok(mut guard) => {
                *guard = settings.clone();
                Ok(())
            }
            Err(e) => Err(format!("MemorySettingsStore lock poisoned: {e}")),
        }
    }
}

/// File-backed settings store using INI format on the local filesystem.
#[derive(Clone, Debug, Default)]
pub struct FileSettingsStore {
    custom_path: Option<PathBuf>,
}

/// Settings store backed by an explicitly chosen file that is never written to.
///
/// The file is authoritative: loading it does not fall back to the default
/// configuration path, and saves are ignored. Used for declaratively generated
/// configuration (for example a Nix store path), which the process may not own.
#[derive(Clone, Debug)]
pub struct ReadOnlySettingsStore {
    path: PathBuf,
}

impl ReadOnlySettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl SettingsStore for ReadOnlySettingsStore {
    fn load(&self) -> Settings {
        Settings::load_from_file(&self.path).unwrap_or_default()
    }

    fn save(&self, _settings: &Settings) -> Result<(), String> {
        Ok(())
    }
}

impl FileSettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            custom_path: Some(path.into()),
        }
    }

    #[cfg(feature = "directories")]
    pub fn default_config_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "srwi", "KeyPeek")
            .map(|dirs| dirs.config_dir().join("settings.ini"))
    }

    #[cfg(not(feature = "directories"))]
    pub fn default_config_path() -> Option<PathBuf> {
        None
    }

    pub fn resolve_path(&self) -> Option<PathBuf> {
        self.custom_path.clone().or_else(Self::default_config_path)
    }

    pub fn load_file(&self) -> Option<Settings> {
        if let Some(path) = self.resolve_path() {
            if let Some(settings) = Settings::load_from_file(&path) {
                return Some(settings);
            }
        }
        Settings::load_from_file("settings.ini")
    }
}

impl SettingsStore for FileSettingsStore {
    fn load(&self) -> Settings {
        self.load_file().unwrap_or_default()
    }

    fn save(&self, settings: &Settings) -> Result<(), String> {
        let path = self
            .resolve_path()
            .ok_or_else(|| "could not determine the KeyPeek config directory".to_string())?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create config directory {}: {e}",
                    parent.display()
                )
            })?;
        }

        settings
            .save_to_file(&path)
            .map_err(|e| format!("failed to save settings to {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_ini_round_trip() {
        let mut original = Settings {
            size: 75,
            font_size_multiplier: 1.3,
            auto_fit_before_ellipsis: true,
            position: WindowPosition::TopLeft,
            timeout: 3500,
            activation_delay: 200,
            offset_x: 25,
            offset_y: 4,
            visible_layers: LayerMask(0x0000000f),
            legend_mode: LegendMode::SingleLive,
            ..Default::default()
        };
        original.theme.font_color = ThemeColor::new(10, 20, 30, 40);
        original.theme.layer_colors[0] = ThemeColor::new(50, 60, 70, 80);

        let ini_text = original.to_ini_string();
        let loaded = Settings::from_ini_str(&ini_text).expect("should parse ini");

        assert_eq!(original, loaded);
    }

    #[test]
    fn test_memory_settings_store() {
        let store = MemorySettingsStore::default();
        let initial = store.load();
        assert_eq!(initial, Settings::default());

        let mut updated = initial.clone();
        updated.timeout = 5000;
        updated.position = WindowPosition::Bottom;

        store.save(&updated).expect("save should succeed");
        assert_eq!(store.load(), updated);
    }

    #[test]
    fn test_file_settings_store_custom_path() {
        let temp_dir = std::env::temp_dir().join(format!("keypeek_test_{}", std::process::id()));
        let file_path = temp_dir.join("test_settings.ini");

        let store = FileSettingsStore::new(&file_path);
        let settings = Settings {
            size: 80,
            position: WindowPosition::TopRight,
            ..Default::default()
        };

        store
            .save(&settings)
            .expect("saving to temp file should succeed");
        assert!(file_path.exists());

        let loaded = store.load();
        assert_eq!(loaded, settings);

        let _ = fs::remove_file(&file_path);
        let _ = fs::remove_dir(&temp_dir);
    }

    #[test]
    fn test_read_only_store_loads_explicit_file_and_ignores_saves() {
        let temp_dir = std::env::temp_dir().join(format!("keypeek_ro_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("temp dir");
        let file_path = temp_dir.join("generated.ini");

        let settings = Settings {
            size: 42,
            ..Default::default()
        };
        settings.save_to_file(&file_path).expect("write fixture");
        let original = fs::read_to_string(&file_path).expect("read fixture");

        let store = ReadOnlySettingsStore::new(&file_path);
        assert_eq!(store.load().size, 42);

        store
            .save(&Settings {
                size: 99,
                ..Default::default()
            })
            .expect("saves are accepted and dropped");
        assert_eq!(fs::read_to_string(&file_path).expect("reread"), original);
        assert_eq!(store.load().size, 42);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_only_store_does_not_fall_back_to_default_path() {
        let store = ReadOnlySettingsStore::new("/nonexistent/keypeek/generated.ini");
        assert_eq!(store.load(), Settings::default());
    }

    #[test]
    fn monitor_selection_round_trips_through_ini() {
        let settings = Settings {
            monitor: MonitorSelection::Named("DP-1".to_string()),
            ..Default::default()
        };
        let parsed = Settings::from_ini_str(&settings.to_ini_string()).expect("parse");
        assert_eq!(parsed.monitor, MonitorSelection::Named("DP-1".to_string()));

        let primary = Settings::default();
        let parsed = Settings::from_ini_str(&primary.to_ini_string()).expect("parse");
        assert_eq!(parsed.monitor, MonitorSelection::Primary);
    }

    #[test]
    fn offsets_apply_inward_per_axis() {
        use WindowPosition::*;
        let cases = [
            (TopLeft, (40.0, 12.0)),
            (TopRight, (-40.0, 12.0)),
            (BottomLeft, (40.0, -12.0)),
            (BottomRight, (-40.0, -12.0)),
            (Bottom, (40.0, -12.0)),
            (Top, (40.0, 12.0)),
            (Center, (40.0, 12.0)),
        ];
        for (position, expected) in cases {
            let (_, offset) = position.anchor_and_offset(40.0, 12.0);
            assert_eq!((offset.x, offset.y), expected, "anchor {position:?}");
        }
    }

    #[test]
    fn negative_offsets_move_outward() {
        let (align, offset) = WindowPosition::BottomRight.anchor_and_offset(-10.0, -5.0);
        assert_eq!(align, egui::Align2::RIGHT_BOTTOM);
        assert_eq!((offset.x, offset.y), (10.0, 5.0));
    }

    #[test]
    fn legacy_margin_sets_both_offsets() {
        let legacy = "[settings]\nposition = BottomRight\nmargin = 33\n";
        let parsed = Settings::from_ini_str(legacy).expect("parse");
        assert_eq!((parsed.offset_x, parsed.offset_y), (33, 33));
    }

    #[test]
    fn offsets_win_over_a_stale_margin() {
        let text = "[settings]\nmargin = 33\noffset_x = 7\noffset_y = 2\n";
        let parsed = Settings::from_ini_str(text).expect("parse");
        assert_eq!((parsed.offset_x, parsed.offset_y), (7, 2));
    }

    #[test]
    fn saving_emits_offsets_not_margin() {
        let text = Settings::default().to_ini_string();
        assert!(text.contains("offset_x"));
        assert!(text.contains("offset_y"));
        assert!(!text.contains("margin"));
    }

    #[test]
    fn monitor_selection_parses_any_non_primary_name() {
        let parsed: MonitorSelection = "HDMI-A-1".parse().expect("named output");
        assert_eq!(parsed, MonitorSelection::Named("HDMI-A-1".to_string()));

        let empty: MonitorSelection = "".parse().expect("empty is primary");
        assert_eq!(empty, MonitorSelection::Primary);

        let primary: MonitorSelection = "Primary".parse().expect("primary");
        assert_eq!(primary, MonitorSelection::Primary);
    }
}
