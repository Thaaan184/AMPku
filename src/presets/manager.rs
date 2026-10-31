//! Preset management engine for AMPku.
//!
//! Handles saving, loading, listing, and applying presets to channel strips,
//! disk persistence in standard application data directories, and factory tone fallbacks.

use std::fs;
use std::path::{Path, PathBuf};

use crate::engine::channel::ChannelStrip;
use crate::presets::defaults::{self, FACTORY_PRESET_NAMES};
use crate::presets::schema::PresetData;

/// Errors arising during preset disk operations, parsing, or DSP instantiation.
#[derive(Debug)]
pub enum PresetError {
    /// Filesystem input/output error.
    Io(std::io::Error),
    /// JSON serialization or deserialization failure.
    Json(serde_json::Error),
    /// Preset was not found on disk or among factory tones.
    NotFound(String),
    /// Preset name or file path is invalid.
    InvalidName(String),
    /// DSP effect instantiation or configuration failure.
    Dsp(String),
}

impl std::fmt::Display for PresetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Preset I/O error: {err}"),
            Self::Json(err) => write!(f, "Preset JSON error: {err}"),
            Self::NotFound(name) => write!(f, "Preset not found: '{name}'"),
            Self::InvalidName(msg) => write!(f, "Invalid preset name: {msg}"),
            Self::Dsp(msg) => write!(f, "DSP instantiation error: {msg}"),
        }
    }
}

impl std::error::Error for PresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PresetError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for PresetError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

/// Manager responsible for persisting, discovering, loading, and applying audio presets.
#[derive(Debug, Clone)]
pub struct PresetManager {
    preset_dir: PathBuf,
    current_preset: Option<PresetData>,
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PresetManager {
    /// Create a PresetManager configured with the default platform preset directory.
    ///
    /// - Windows: `%APPDATA%/AMPku/presets`
    /// - Linux/macOS: `$XDG_CONFIG_HOME/ampku/presets` or `~/.config/ampku/presets`
    /// - Fallback: `./presets`
    pub fn new() -> Self {
        let preset_dir = default_preset_directory();
        Self {
            preset_dir,
            current_preset: None,
        }
    }

    /// Create a PresetManager with an explicit directory path (e.g. for testing or portable mode).
    pub fn with_directory(dir: impl Into<PathBuf>) -> Self {
        Self {
            preset_dir: dir.into(),
            current_preset: None,
        }
    }

    /// Reference to the current preset directory path.
    pub fn directory(&self) -> &Path {
        &self.preset_dir
    }

    /// Update the preset directory path.
    pub fn set_directory(&mut self, dir: impl Into<PathBuf>) {
        self.preset_dir = dir.into();
    }

    /// Ensure that the preset directory exists on disk, creating parent directories if necessary.
    pub fn ensure_directory(&self) -> Result<(), PresetError> {
        if !self.preset_dir.exists() {
            fs::create_dir_all(&self.preset_dir)?;
        }
        Ok(())
    }

    /// Reference to the currently loaded preset, if any.
    pub fn current_preset(&self) -> Option<&PresetData> {
        self.current_preset.as_ref()
    }

    /// Set the currently loaded preset.
    pub fn set_current_preset(&mut self, preset: PresetData) {
        self.current_preset = Some(preset);
    }

    /// Return static slice of built-in factory preset names.
    pub fn list_factory_presets() -> &'static [&'static str] {
        &FACTORY_PRESET_NAMES
    }

    /// List all available user presets stored as `.json` files in the preset directory.
    pub fn list_user_presets(&self) -> Result<Vec<String>, PresetError> {
        if !self.preset_dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        for entry in fs::read_dir(&self.preset_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("json") {
                        if let Some(stem) = path.file_stem() {
                            names.push(stem.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Return a merged list of all presets: built-in factory tones first, followed by unique user presets.
    pub fn list_presets(&self) -> Vec<String> {
        let mut list: Vec<String> = FACTORY_PRESET_NAMES
            .iter()
            .map(|&s| s.to_string())
            .collect();

        if let Ok(user_presets) = self.list_user_presets() {
            for user_name in user_presets {
                if !list
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&user_name))
                {
                    list.push(user_name);
                }
            }
        }
        list
    }

    /// Load a preset by name or filename.
    ///
    /// First checks for a matching user preset JSON file on disk.
    /// If not found on disk, falls back to built-in factory presets.
    pub fn load_preset(&mut self, name: &str) -> Result<PresetData, PresetError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(PresetError::InvalidName("Preset name is empty".to_string()));
        }

        // 1. Check if direct file exists on disk
        let filename = if trimmed.ends_with(".json") {
            trimmed.to_string()
        } else {
            format!("{trimmed}.json")
        };
        let file_path = self.preset_dir.join(&filename);

        if file_path.is_file() {
            let data = self.load_from_file(&file_path)?;
            self.current_preset = Some(data.clone());
            return Ok(data);
        }

        // 2. Check case-insensitive match among disk files
        if self.preset_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&self.preset_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(stem) = path.file_stem() {
                            if stem.to_string_lossy().eq_ignore_ascii_case(trimmed) {
                                let data = self.load_from_file(&path)?;
                                self.current_preset = Some(data.clone());
                                return Ok(data);
                            }
                        }
                    }
                }
            }
        }

        // 3. Fall back to factory presets
        if let Some(factory) = defaults::get_factory_preset(trimmed) {
            self.current_preset = Some(factory.clone());
            return Ok(factory);
        }

        Err(PresetError::NotFound(trimmed.to_string()))
    }

    /// Load and deserialize a preset from an explicit filesystem path.
    pub fn load_from_file(&mut self, path: impl AsRef<Path>) -> Result<PresetData, PresetError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let preset: PresetData = serde_json::from_str(&content)?;
        self.current_preset = Some(preset.clone());
        Ok(preset)
    }

    /// Save a preset to disk using its sanitized `.name` attribute.
    ///
    /// Creates the preset directory if needed and writes formatted JSON.
    pub fn save_preset(&mut self, preset: &PresetData) -> Result<PathBuf, PresetError> {
        let safe_name = sanitize_filename(&preset.name);
        if safe_name.is_empty() {
            return Err(PresetError::InvalidName(
                "Preset name contains no valid filename characters".to_string(),
            ));
        }
        let filename = format!("{safe_name}.json");
        self.save_preset_as(preset, &filename)
    }

    /// Save a preset to disk with a specified filename.
    pub fn save_preset_as(
        &mut self,
        preset: &PresetData,
        filename: &str,
    ) -> Result<PathBuf, PresetError> {
        self.ensure_directory()?;

        let mut fname = filename.trim().to_string();
        if !fname.ends_with(".json") {
            fname.push_str(".json");
        }

        let target_path = self.preset_dir.join(&fname);
        let json_text = serde_json::to_string_pretty(preset)?;
        fs::write(&target_path, json_text)?;

        self.current_preset = Some(preset.clone());
        Ok(target_path)
    }

    /// Delete a user preset file from disk by name.
    ///
    /// Returns `Ok(true)` if deleted, `Ok(false)` if the file did not exist.
    pub fn delete_preset(&self, name: &str) -> Result<bool, PresetError> {
        let trimmed = name.trim();
        let filename = if trimmed.ends_with(".json") {
            trimmed.to_string()
        } else {
            format!("{trimmed}.json")
        };
        let file_path = self.preset_dir.join(&filename);

        if file_path.exists() {
            fs::remove_file(file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Serialize a preset to formatted JSON string without writing to disk.
    pub fn export_to_json(&self, preset: &PresetData) -> Result<String, PresetError> {
        serde_json::to_string_pretty(preset).map_err(PresetError::Json)
    }

    /// Parse and validate a preset from raw JSON text.
    pub fn import_from_json(&mut self, json_str: &str) -> Result<PresetData, PresetError> {
        let preset: PresetData = serde_json::from_str(json_str)?;
        self.current_preset = Some(preset.clone());
        Ok(preset)
    }

    /// Write all five factory presets to disk in the presets directory if not already present.
    ///
    /// Returns the number of newly created preset files.
    pub fn save_factory_presets_to_disk(&self) -> Result<usize, PresetError> {
        self.ensure_directory()?;
        let mut written = 0;

        for factory in defaults::get_factory_presets() {
            let filename = format!("{}.json", factory.name);
            let path = self.preset_dir.join(&filename);
            if !path.exists() {
                let json_text = serde_json::to_string_pretty(&factory)?;
                fs::write(&path, json_text)?;
                written += 1;
            }
        }

        Ok(written)
    }

    /// Apply a preset to Guitar and Mic channel strips at the specified sample rate.
    pub fn apply_preset(
        &self,
        preset: &PresetData,
        guitar: &mut ChannelStrip,
        mic: &mut ChannelStrip,
        sample_rate: f32,
    ) -> Result<(), PresetError> {
        preset
            .apply_to_channels(guitar, mic, sample_rate)
            .map_err(PresetError::Dsp)
    }
}

/// Determine default directory path for presets based on host operating system.
fn default_preset_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("AMPku").join("presets");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("ampku").join("presets");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("ampku")
                .join("presets");
        }
    }

    PathBuf::from("./presets")
}

/// Sanitize preset name to form a safe, cross-platform filename.
fn sanitize_filename(name: &str) -> String {
    let mut safe = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => {
                safe.push('_');
            }
            c if c.is_control() => {
                safe.push('_');
            }
            c => safe.push(c),
        }
    }
    let trimmed = safe.trim().trim_matches('.');
    if trimmed.is_empty() {
        "preset".to_string()
    } else {
        trimmed.to_string()
    }
}
