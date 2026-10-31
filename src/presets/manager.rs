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

    /// Validate that a preset name or filename contains no path traversal components,
    /// directory separators, or invalid path tokens.
    ///
    /// Returns the sanitized `.json` filename if valid, or `PresetError::InvalidName`
    /// if traversal or invalid characters are detected.
    pub fn validate_preset_filename(name: &str) -> Result<String, PresetError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(PresetError::InvalidName("Preset name is empty".to_string()));
        }

        // Reject directory separators, parent directory traversal, and null bytes
        if trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
            || trimmed.chars().any(|c| c.is_control() || c == '\0')
        {
            return Err(PresetError::InvalidName(format!(
                "Path traversal detected in preset name: '{name}'"
            )));
        }

        let p = Path::new(trimmed);
        if p.is_absolute() {
            return Err(PresetError::InvalidName(format!(
                "Absolute path not allowed: '{name}'"
            )));
        }

        // Single component check
        let components: Vec<_> = p.components().collect();
        if components.len() != 1 || !matches!(components[0], std::path::Component::Normal(_)) {
            return Err(PresetError::InvalidName(format!(
                "Invalid preset filename component: '{name}'"
            )));
        }

        let filename =
            if trimmed.len() >= 5 && trimmed[trimmed.len() - 5..].eq_ignore_ascii_case(".json") {
                trimmed.to_string()
            } else {
                format!("{trimmed}.json")
            };

        // Re-check normalized filename
        let file_path = Path::new(&filename);
        let file_components: Vec<_> = file_path.components().collect();
        if file_components.len() != 1
            || !matches!(file_components[0], std::path::Component::Normal(_))
        {
            return Err(PresetError::InvalidName(format!(
                "Invalid preset filename: '{name}'"
            )));
        }

        Ok(filename)
    }

    /// Resolve a preset name to a safe, canonical path strictly within `preset_dir`.
    pub fn resolve_preset_path(&self, name: &str) -> Result<PathBuf, PresetError> {
        let filename = Self::validate_preset_filename(name)?;
        let resolved = self.preset_dir.join(&filename);

        if resolved.parent() != Some(&self.preset_dir) {
            return Err(PresetError::InvalidName(format!(
                "Resolved path escapes preset directory: '{name}'"
            )));
        }

        Ok(resolved)
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
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    if file_name.starts_with('.') {
                        continue;
                    }
                }
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

        // Validate name against path traversal
        let file_path = self.resolve_preset_path(trimmed)?;

        // 1. Check if direct file exists on disk
        if file_path.is_file() {
            let data = self.load_from_file(&file_path)?;
            self.current_preset = Some(data.clone());
            return Ok(data);
        }

        // Determine query stem without .json extension for scanning and factory lookup
        let query_stem =
            if trimmed.len() >= 5 && trimmed[trimmed.len() - 5..].eq_ignore_ascii_case(".json") {
                &trimmed[..trimmed.len() - 5]
            } else {
                trimmed
            };

        // 2. Check case-insensitive match among .json files on disk
        if self.preset_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&self.preset_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let is_json = path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("json"))
                            .unwrap_or(false);

                        if is_json {
                            if let Some(stem) = path.file_stem() {
                                if stem.to_string_lossy().eq_ignore_ascii_case(query_stem) {
                                    let data = self.load_from_file(&path)?;
                                    self.current_preset = Some(data.clone());
                                    return Ok(data);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Fall back to factory presets (stripping .json suffix via query_stem)
        if let Some(factory) = defaults::get_factory_preset(query_stem) {
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

    /// Save a preset to an explicit filesystem path using atomic write semantics (.tmp file + rename).
    pub fn save_to_file(
        &self,
        preset: &PresetData,
        path: impl AsRef<Path>,
    ) -> Result<(), PresetError> {
        let path = path.as_ref();
        let json_text = serde_json::to_string_pretty(preset)?;
        atomic_write(path, &json_text)?;
        Ok(())
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

    /// Save a preset to disk with a specified filename using atomic write semantics.
    pub fn save_preset_as(
        &mut self,
        preset: &PresetData,
        filename: &str,
    ) -> Result<PathBuf, PresetError> {
        self.ensure_directory()?;
        let target_path = self.resolve_preset_path(filename)?;
        self.save_to_file(preset, &target_path)?;
        self.current_preset = Some(preset.clone());
        Ok(target_path)
    }

    /// Delete a user preset file from disk by name.
    ///
    /// Returns `Ok(true)` if deleted, `Ok(false)` if the file did not exist.
    pub fn delete_preset(&self, name: &str) -> Result<bool, PresetError> {
        let file_path = self.resolve_preset_path(name)?;
        if file_path.is_file() {
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
                self.save_to_file(&factory, &path)?;
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

/// Atomically write data to `target_path` by first writing to a temporary `.tmp` file
/// in the same directory and renaming it to `target_path`.
fn atomic_write(target_path: &Path, content: &str) -> Result<(), std::io::Error> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() && !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("preset");

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let tmp_path = parent.join(format!(".{file_name}.{}.{nanos}.tmp", std::process::id()));

    let res = (|| -> Result<(), std::io::Error> {
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, target_path)?;
        Ok(())
    })();

    if res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    res
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
    let mut cleaned = trimmed.to_string();
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", "_");
    }
    if cleaned.is_empty() {
        "preset".to_string()
    } else {
        cleaned
    }
}
