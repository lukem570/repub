//! Application settings and preferences.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Application settings that persist between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Recent files
    pub recent_files: Vec<PathBuf>,
    /// Maximum recent files to track
    #[serde(default = "default_max_recent")]
    pub max_recent_files: usize,
    /// Default page size
    #[serde(default = "default_page_size")]
    pub default_page_size: String,
    /// Default page orientation
    #[serde(default = "default_orientation")]
    pub default_orientation: String,
    /// Show rulers by default
    #[serde(default = "default_true")]
    pub show_rulers: bool,
    /// Show guides by default
    #[serde(default = "default_true")]
    pub show_guides: bool,
    /// Default zoom level
    #[serde(default = "default_zoom")]
    pub default_zoom: f32,
    /// Auto-save interval in seconds (0 = disabled)
    #[serde(default)]
    pub autosave_interval: u64,
    /// Application theme
    #[serde(default)]
    pub theme: crate::theme::ThemeMode,
    /// Selection pane (objects on the page, show/hide)
    #[serde(default)]
    pub selection_pane_visible: bool,
    /// Measurement unit key ("in", "pt", "mm")
    #[serde(default = "default_units")]
    pub measurement_unit: String,
    /// Window width
    #[serde(default = "default_window_width")]
    pub window_width: f32,
    /// Window height
    #[serde(default = "default_window_height")]
    pub window_height: f32,
    /// Left panel width
    #[serde(default = "default_panel_width")]
    pub left_panel_width: f32,
    /// Left panel visible
    #[serde(default = "default_true")]
    pub left_panel_visible: bool,
    /// Version of the pane layout defaults this file was written with; see `migrate`.
    #[serde(default)]
    pub layout_version: u32,
    /// Whether `save` writes to disk. False for in-memory settings (tests).
    #[serde(skip, default = "default_true")]
    persist: bool,
}

/// Current pane layout defaults version.
const LAYOUT_VERSION: u32 = 1;

fn default_max_recent() -> usize {
    10
}
fn default_page_size() -> String {
    "letter".to_string()
}
fn default_orientation() -> String {
    "portrait".to_string()
}
fn default_true() -> bool {
    true
}
fn default_zoom() -> f32 {
    1.0
}
fn default_window_width() -> f32 {
    1280.0
}
fn default_window_height() -> f32 {
    800.0
}
fn default_panel_width() -> f32 {
    200.0
}
/// Publisher's Page Navigation pane is about 100-120 pt wide.
fn default_pages_pane_width() -> f32 {
    120.0
}
/// Publisher has no permanent properties pane; ours is opt-in via View > Format Pane.
fn default_units() -> String {
    "in".to_owned()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            max_recent_files: default_max_recent(),
            default_page_size: default_page_size(),
            default_orientation: default_orientation(),
            show_rulers: true,
            show_guides: true,
            default_zoom: default_zoom(),
            autosave_interval: 0,
            theme: crate::theme::ThemeMode::Light,
            selection_pane_visible: false,
            measurement_unit: default_units(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            left_panel_width: default_pages_pane_width(),
            left_panel_visible: true,
            layout_version: LAYOUT_VERSION,
            persist: true,
        }
    }
}

/// Where the browser keeps the settings.
#[cfg(target_arch = "wasm32")]
const WEB_KEY: &str = "repub.settings";

#[cfg(target_arch = "wasm32")]
fn web_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

impl Settings {
    pub fn load() -> Self {
        #[cfg(target_arch = "wasm32")]
        let stored = web_storage().and_then(|s| s.get_item(WEB_KEY).ok().flatten());
        #[cfg(not(target_arch = "wasm32"))]
        let stored = Self::config_path()
            .filter(|path| path.exists())
            .and_then(|path| std::fs::read_to_string(path).ok());
        if let Some(Ok(mut settings)) = stored.map(|json| serde_json::from_str::<Settings>(&json)) {
            settings.migrate();
            return settings;
        }
        Self::default()
    }

    /// Bring settings saved by older versions up to the current pane defaults, once.
    fn migrate(&mut self) {
        if self.layout_version < 1 {
            // v1: Publisher-sized Pages pane
            if self.left_panel_width > 160.0 {
                self.left_panel_width = default_pages_pane_width();
            }
        }
        if self.layout_version != LAYOUT_VERSION {
            self.layout_version = LAYOUT_VERSION;
            self.save();
        }
    }

    /// Default settings that are never read from or written to disk.
    pub fn in_memory() -> Self {
        Self {
            persist: false,
            ..Self::default()
        }
    }

    pub fn save(&self) {
        if !self.persist {
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(self) else {
            return;
        };
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) = web_storage() {
            let _ = storage.set_item(WEB_KEY, &json);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, json);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Get the config file path.
    fn config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "repub", "Repub")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    /// Add a file to recent files.
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);
        // Add to front
        self.recent_files.insert(0, path);
        // Trim to max
        self.recent_files.truncate(self.max_recent_files);
    }

    /// Clear recent files.
    pub fn clear_recent_files(&mut self) {
        self.recent_files.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(settings.show_rulers);
        assert_eq!(settings.default_zoom, 1.0);
    }

    #[test]
    fn test_add_recent_file() {
        let mut settings = Settings::default();
        settings.max_recent_files = 3;

        settings.add_recent_file(PathBuf::from("/file1.repub"));
        settings.add_recent_file(PathBuf::from("/file2.repub"));
        settings.add_recent_file(PathBuf::from("/file3.repub"));
        settings.add_recent_file(PathBuf::from("/file4.repub"));

        assert_eq!(settings.recent_files.len(), 3);
        assert_eq!(settings.recent_files[0], PathBuf::from("/file4.repub"));
    }
}
