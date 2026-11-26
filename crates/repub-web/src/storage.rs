//! Local storage for web.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get localStorage.
fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Save a string to localStorage.
pub fn save_string(key: &str, value: &str) -> Result<(), JsValue> {
    let storage = get_storage().ok_or("No localStorage")?;
    storage.set_item(key, value)
}

/// Load a string from localStorage.
pub fn load_string(key: &str) -> Option<String> {
    let storage = get_storage()?;
    storage.get_item(key).ok()?
}

/// Remove an item from localStorage.
pub fn remove(key: &str) -> Result<(), JsValue> {
    let storage = get_storage().ok_or("No localStorage")?;
    storage.remove_item(key)
}

/// Save a serializable value to localStorage.
pub fn save<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value)
        .map_err(|e| e.to_string())?;
    save_string(key, &json)
        .map_err(|e| format!("{:?}", e))
}

/// Load a deserializable value from localStorage.
pub fn load<T: for<'de> Deserialize<'de>>(key: &str) -> Option<T> {
    let json = load_string(key)?;
    serde_json::from_str(&json).ok()
}

/// Web application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSettings {
    /// Recent documents (name -> JSON)
    pub recent_documents: HashMap<String, String>,
    /// Maximum recent documents
    pub max_recent: usize,
    /// Default page size
    pub default_page_size: String,
    /// Default orientation
    pub default_orientation: String,
    /// Show rulers
    pub show_rulers: bool,
    /// Show guides
    pub show_guides: bool,
    /// Default zoom
    pub default_zoom: f32,
    /// Auto-save enabled
    pub auto_save: bool,
    /// Auto-save interval (seconds)
    pub auto_save_interval: u32,
    /// Dark mode
    pub dark_mode: bool,
    /// Touch mode (larger controls)
    pub touch_mode: bool,
}

impl Default for WebSettings {
    fn default() -> Self {
        Self {
            recent_documents: HashMap::new(),
            max_recent: 10,
            default_page_size: "letter".to_string(),
            default_orientation: "portrait".to_string(),
            show_rulers: true,
            show_guides: true,
            default_zoom: 1.0,
            auto_save: true,
            auto_save_interval: 30,
            dark_mode: false,
            touch_mode: false,
        }
    }
}

impl WebSettings {
    const STORAGE_KEY: &'static str = "repub_settings";

    /// Load settings from localStorage.
    pub fn load() -> Self {
        load::<WebSettings>(Self::STORAGE_KEY).unwrap_or_default()
    }

    /// Save settings to localStorage.
    pub fn save(&self) {
        let _ = save(Self::STORAGE_KEY, self);
    }

    /// Add a recent document.
    pub fn add_recent(&mut self, name: String, json: String) {
        // Remove if already exists
        self.recent_documents.remove(&name);
        
        // Add new
        self.recent_documents.insert(name, json);
        
        // Trim if too many
        while self.recent_documents.len() > self.max_recent {
            // Remove oldest (arbitrary since HashMap doesn't preserve order)
            if let Some(key) = self.recent_documents.keys().next().cloned() {
                self.recent_documents.remove(&key);
            }
        }
    }

    /// Get a recent document by name.
    pub fn get_recent(&self, name: &str) -> Option<&String> {
        self.recent_documents.get(name)
    }

    /// Clear recent documents.
    pub fn clear_recent(&mut self) {
        self.recent_documents.clear();
    }
}

/// Auto-save state.
pub struct AutoSave {
    /// Current document name
    name: String,
    /// Last save time
    last_save: f64,
    /// Save interval (ms)
    interval_ms: f64,
}

impl AutoSave {
    const STORAGE_PREFIX: &'static str = "repub_autosave_";

    /// Create a new auto-save handler.
    pub fn new(name: &str, interval_seconds: u32) -> Self {
        Self {
            name: name.to_string(),
            last_save: current_time_ms(),
            interval_ms: (interval_seconds * 1000) as f64,
        }
    }

    /// Check if it's time to auto-save.
    pub fn should_save(&self) -> bool {
        let now = current_time_ms();
        now - self.last_save >= self.interval_ms
    }

    /// Save document JSON.
    pub fn save(&mut self, json: &str) -> Result<(), String> {
        let key = format!("{}{}", Self::STORAGE_PREFIX, self.name);
        save_string(&key, json).map_err(|e| format!("{:?}", e))?;
        self.last_save = current_time_ms();
        Ok(())
    }

    /// Load auto-saved document.
    pub fn load(name: &str) -> Option<String> {
        let key = format!("{}{}", Self::STORAGE_PREFIX, name);
        load_string(&key)
    }

    /// Clear auto-save.
    pub fn clear(name: &str) {
        let key = format!("{}{}", Self::STORAGE_PREFIX, name);
        let _ = remove(&key);
    }

    /// List all auto-saved documents.
    pub fn list_autosaves() -> Vec<String> {
        let mut names = Vec::new();
        
        if let Some(storage) = get_storage() {
            if let Ok(len) = storage.length() {
                for i in 0..len {
                    if let Ok(Some(key)) = storage.key(i) {
                        if key.starts_with(Self::STORAGE_PREFIX) {
                            let name = key.strip_prefix(Self::STORAGE_PREFIX)
                                .unwrap_or(&key)
                                .to_string();
                            names.push(name);
                        }
                    }
                }
            }
        }
        
        names
    }
}

/// Get current time in milliseconds.
fn current_time_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_settings_default() {
        let settings = WebSettings::default();
        assert!(settings.show_rulers);
        assert_eq!(settings.default_zoom, 1.0);
    }

    #[wasm_bindgen_test]
    fn test_settings_save_load() {
        let mut settings = WebSettings::default();
        settings.dark_mode = true;
        settings.save();
        
        let loaded = WebSettings::load();
        assert!(loaded.dark_mode);
    }

    #[wasm_bindgen_test]
    fn test_storage_string() {
        let _ = save_string("test_key", "test_value");
        let value = load_string("test_key");
        assert_eq!(value, Some("test_value".to_string()));
        let _ = remove("test_key");
    }
}
