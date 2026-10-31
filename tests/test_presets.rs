use std::fs;
use std::path::PathBuf;

use ampku::core::frame::AudioFrame;
use ampku::engine::channel::ChannelStrip;
use ampku::presets::defaults::{
    acoustic_chime, clean_ambient, clear_vocal, get_factory_preset, get_factory_presets,
    modern_lead, texas_blues, FACTORY_PRESET_NAMES,
};
use ampku::presets::manager::{PresetError, PresetManager};
use ampku::presets::schema::{ChannelPreset, EffectPreset, MasterPreset, PresetData};

/// Temporary directory helper for isolated disk persistence tests.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("ampku_preset_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_factory_presets_validity() {
    let presets = get_factory_presets();
    assert!(presets.len() >= 4);
    let json = serde_json::to_string(&presets[0]).unwrap();
    let deserialized: PresetData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, presets[0].name);
}

#[test]
fn test_all_five_factory_presets_exist() {
    let presets = get_factory_presets();
    assert_eq!(presets.len(), 5);

    let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"TexasBlues"));
    assert!(names.contains(&"ModernLead"));
    assert!(names.contains(&"CleanAmbient"));
    assert!(names.contains(&"AcousticChime"));
    assert!(names.contains(&"ClearVocal"));

    assert_eq!(FACTORY_PRESET_NAMES.len(), 5);
    for expected in &FACTORY_PRESET_NAMES {
        assert!(get_factory_preset(expected).is_some());
    }

    // Case-insensitive lookup
    assert!(get_factory_preset("texasblues").is_some());
    assert!(get_factory_preset("MODERN_LEAD").is_some());
    assert!(get_factory_preset("Clean Ambient").is_some());
    assert!(get_factory_preset("nonexistent_tone").is_none());
}

#[test]
fn test_factory_preset_constructors_direct() {
    let tb = texas_blues();
    assert_eq!(tb.name, "TexasBlues");
    assert_eq!(tb.category, "Blues");
    assert!(!tb.guitar_channel.effects.is_empty());
    assert!(!tb.mic_channel.effects.is_empty());

    let ml = modern_lead();
    assert_eq!(ml.name, "ModernLead");
    assert_eq!(ml.category, "Rock");

    let ca = clean_ambient();
    assert_eq!(ca.name, "CleanAmbient");
    assert_eq!(ca.category, "Ambient");

    let ac = acoustic_chime();
    assert_eq!(ac.name, "AcousticChime");
    assert_eq!(ac.category, "Acoustic");

    let cv = clear_vocal();
    assert_eq!(cv.name, "ClearVocal");
    assert_eq!(cv.category, "Vocal");
}

#[test]
fn test_all_factory_presets_roundtrip_json() {
    for preset in get_factory_presets() {
        let json = preset.to_json().expect("failed to serialize preset");
        assert!(!json.is_empty());

        let deserialized = PresetData::from_json(&json).expect("failed to deserialize preset");
        assert_eq!(deserialized.name, preset.name);
        assert_eq!(deserialized.version, preset.version);
        assert_eq!(deserialized.category, preset.category);
        assert_eq!(
            deserialized.guitar_channel.volume,
            preset.guitar_channel.volume
        );
        assert_eq!(deserialized.guitar_channel.pan, preset.guitar_channel.pan);
        assert_eq!(
            deserialized.guitar_channel.effects.len(),
            preset.guitar_channel.effects.len()
        );
        assert_eq!(deserialized.mic_channel.volume, preset.mic_channel.volume);
        assert_eq!(deserialized.mic_channel.pan, preset.mic_channel.pan);
        assert_eq!(
            deserialized.mic_channel.effects.len(),
            preset.mic_channel.effects.len()
        );
        assert_eq!(deserialized, preset);
    }
}

#[test]
fn test_spec_json_compatibility() {
    // Exact schema format from SDD Section 5
    let spec_json = r#"{
  "version": 1,
  "name": "British Crunch & Vocal",
  "guitar_channel": {
    "volume": 0.85,
    "pan": 0.0,
    "effects": [
      {
        "type": "NoiseGate",
        "bypass": false,
        "params": { "threshold_db": -50.0, "release_ms": 100.0 }
      },
      {
        "type": "Overdrive",
        "bypass": false,
        "params": { "drive": 0.65, "tone": 0.5, "level": 0.8 }
      },
      {
        "type": "CabSimIR",
        "bypass": false,
        "params": { "profile": "Celestion_V30_4x12", "mix": 1.0 }
      },
      {
        "type": "Delay",
        "bypass": true,
        "params": { "time_ms": 350.0, "feedback": 0.3, "mix": 0.2 }
      }
    ]
  },
  "mic_channel": {
    "volume": 0.9,
    "pan": 0.0,
    "effects": [
      {
        "type": "HighPassFilter",
        "bypass": false,
        "params": { "cutoff_hz": 80.0 }
      },
      {
        "type": "VocalCompressor",
        "bypass": false,
        "params": { "threshold_db": -18.0, "ratio": 3.0, "makeup_gain": 2.0 }
      }
    ]
  }
}"#;

    let preset: PresetData =
        serde_json::from_str(spec_json).expect("SDD spec JSON failed to parse");
    assert_eq!(preset.name, "British Crunch & Vocal");
    assert_eq!(preset.version, 1);
    assert_eq!(preset.guitar_channel.volume, 0.85);
    assert_eq!(preset.guitar_channel.effects.len(), 4);
    assert_eq!(preset.guitar_channel.effects[0].effect_type, "NoiseGate");
    assert!(!preset.guitar_channel.effects[0].bypass);
    assert!(preset.guitar_channel.effects[3].bypass);
    assert_eq!(preset.mic_channel.volume, 0.9);
    assert_eq!(preset.mic_channel.effects.len(), 2);

    // Verify all effects in SDD spec instantiate successfully
    let sample_rate = 48000.0;
    for effect in &preset.guitar_channel.effects {
        let boxed = effect
            .instantiate(sample_rate)
            .expect("failed to instantiate guitar effect");
        assert_eq!(boxed.is_enabled(), !effect.bypass);
    }
    for effect in &preset.mic_channel.effects {
        let boxed = effect
            .instantiate(sample_rate)
            .expect("failed to instantiate mic effect");
        assert_eq!(boxed.is_enabled(), !effect.bypass);
    }
}

#[test]
fn test_preset_manager_save_load_list_delete() {
    let temp = TempDir::new("save_load");
    let mut manager = PresetManager::with_directory(&temp.path);

    assert_eq!(manager.directory(), &temp.path);
    assert!(manager.current_preset().is_none());

    // Initially list_user_presets is empty
    let initial_user = manager.list_user_presets().unwrap();
    assert!(initial_user.is_empty());

    // list_presets contains the 5 factory tones
    let all_presets = manager.list_presets();
    assert_eq!(all_presets.len(), 5);

    // Create and save custom preset
    let custom = PresetData::new(
        "My Custom Rig",
        ChannelPreset::new(0.9, -0.2).with_effects(vec![
            EffectPreset::overdrive(6.0, 0.5, 0.8, false),
            EffectPreset::reverb(0.6, 0.4, 0.9, 0.25, false),
        ]),
        ChannelPreset::new(1.0, 0.0).with_effects(vec![EffectPreset::vocal_high_pass(80.0, false)]),
    )
    .with_description("Custom studio configuration")
    .with_category("User");

    let saved_path = manager.save_preset(&custom).expect("failed to save preset");
    assert!(saved_path.is_file());
    assert_eq!(manager.current_preset().unwrap().name, "My Custom Rig");

    // list_user_presets now contains "My Custom Rig"
    let user_presets = manager.list_user_presets().unwrap();
    assert_eq!(user_presets.len(), 1);
    assert_eq!(user_presets[0], "My Custom Rig");

    // list_presets contains factory tones + custom
    let merged = manager.list_presets();
    assert_eq!(merged.len(), 6);
    assert!(merged.contains(&"My Custom Rig".to_string()));

    // Load preset by name
    let mut manager2 = PresetManager::with_directory(&temp.path);
    let loaded = manager2
        .load_preset("My Custom Rig")
        .expect("failed to load saved preset");
    assert_eq!(loaded.name, "My Custom Rig");
    assert_eq!(loaded.guitar_channel.volume, 0.9);
    assert_eq!(loaded.guitar_channel.pan, -0.2);
    assert_eq!(loaded.guitar_channel.effects.len(), 2);

    // Delete preset
    let deleted = manager2
        .delete_preset("My Custom Rig")
        .expect("failed to delete preset");
    assert!(deleted);
    assert!(!saved_path.exists());
    assert_eq!(manager2.list_user_presets().unwrap().len(), 0);

    // Deleting again returns false
    let deleted_again = manager2.delete_preset("My Custom Rig").unwrap();
    assert!(!deleted_again);
}

#[test]
fn test_preset_manager_factory_fallback_and_user_override() {
    let temp = TempDir::new("fallback_override");
    let mut manager = PresetManager::with_directory(&temp.path);

    // 1. Loading factory preset when not on disk falls back cleanly
    let loaded_factory = manager
        .load_preset("TexasBlues")
        .expect("should fall back to factory preset");
    assert_eq!(loaded_factory.name, "TexasBlues");
    assert_eq!(loaded_factory.guitar_channel.effects.len(), 4);

    // 2. User saves custom preset with the same name "TexasBlues"
    let mut modified = loaded_factory.clone();
    modified.description = "User overridden Texas Blues".to_string();
    modified.guitar_channel.volume = 0.99;
    manager
        .save_preset(&modified)
        .expect("failed to save override preset");

    // 3. Next load prefers disk version over built-in factory default
    let reloaded = manager
        .load_preset("TexasBlues")
        .expect("failed to load preset");
    assert_eq!(reloaded.description, "User overridden Texas Blues");
    assert_eq!(reloaded.guitar_channel.volume, 0.99);

    // 4. Loading nonexistent returns NotFound
    let not_found = manager.load_preset("UnknownTones");
    assert!(matches!(not_found, Err(PresetError::NotFound(_))));
}

#[test]
fn test_preset_manager_save_factory_presets_to_disk() {
    let temp = TempDir::new("save_factory");
    let manager = PresetManager::with_directory(&temp.path);

    // First call saves 5 presets to disk
    let written = manager
        .save_factory_presets_to_disk()
        .expect("failed to save factory presets");
    assert_eq!(written, 5);

    for name in &FACTORY_PRESET_NAMES {
        let file_path = temp.path.join(format!("{name}.json"));
        assert!(file_path.is_file(), "missing factory file: {file_path:?}");
    }

    // Second call is idempotent; writes 0
    let written2 = manager
        .save_factory_presets_to_disk()
        .expect("failed on second call");
    assert_eq!(written2, 0);
}

#[test]
fn test_all_effects_instantiation_and_audio_processing() {
    let sample_rate = 48000.0;
    let presets = get_factory_presets();

    let mut frame = AudioFrame::new(0.5, -0.5);

    for preset in &presets {
        // Guitar channel effects
        for effect_config in &preset.guitar_channel.effects {
            let mut effect = effect_config.instantiate(sample_rate).unwrap_or_else(|e| {
                panic!(
                    "Failed to instantiate effect '{}' in preset '{}': {e}",
                    effect_config.effect_type, preset.name
                );
            });
            frame = effect.process_frame(frame);
            assert!(frame.left.is_finite());
            assert!(frame.right.is_finite());
            effect.reset();
        }

        // Mic channel effects
        for effect_config in &preset.mic_channel.effects {
            let mut effect = effect_config.instantiate(sample_rate).unwrap_or_else(|e| {
                panic!(
                    "Failed to instantiate effect '{}' in preset '{}': {e}",
                    effect_config.effect_type, preset.name
                );
            });
            frame = effect.process_frame(frame);
            assert!(frame.left.is_finite());
            assert!(frame.right.is_finite());
            effect.reset();
        }
    }
}

#[test]
fn test_apply_preset_to_channels_end_to_end() {
    let sample_rate = 48000.0;
    let preset = modern_lead();

    let mut guitar_strip = ChannelStrip::with_sample_rate("Guitar", sample_rate);
    let mut mic_strip = ChannelStrip::with_sample_rate("Mic", sample_rate);

    // Apply preset
    preset
        .apply_to_channels(&mut guitar_strip, &mut mic_strip, sample_rate)
        .expect("failed to apply preset");

    assert_eq!(guitar_strip.gain(), 0.80);
    assert_eq!(guitar_strip.pan(), 0.0);
    assert_eq!(guitar_strip.chain().len(), 5);

    assert_eq!(mic_strip.gain(), 0.85);
    assert_eq!(mic_strip.pan(), 0.0);
    assert_eq!(mic_strip.chain().len(), 3);

    // Process audio blocks through both channels
    let input_guitar = [AudioFrame::new(0.2, 0.2); 128];
    let mut processed_guitar = input_guitar;
    guitar_strip.process_block(&mut processed_guitar);

    for f in &processed_guitar {
        assert!(f.left.is_finite());
        assert!(f.right.is_finite());
    }

    let input_mic = [AudioFrame::new(0.1, 0.1); 128];
    let mut processed_mic = input_mic;
    mic_strip.process_block(&mut processed_mic);

    for f in &processed_mic {
        assert!(f.left.is_finite());
        assert!(f.right.is_finite());
    }
}

#[test]
fn test_preset_manager_error_handling() {
    let temp = TempDir::new("errors");
    let mut manager = PresetManager::with_directory(&temp.path);

    // Empty name error
    let err = manager.load_preset("   ");
    assert!(matches!(err, Err(PresetError::InvalidName(_))));

    // Corrupted JSON error
    let corrupt_file = temp.path.join("broken.json");
    fs::write(&corrupt_file, "{ not valid json").unwrap();
    let err2 = manager.load_from_file(&corrupt_file);
    assert!(matches!(err2, Err(PresetError::Json(_))));

    // Unknown effect type error
    let invalid_effect = EffectPreset::new("NonExistentDSPUnit", false, serde_json::json!({}));
    match invalid_effect.instantiate(48000.0) {
        Err(err_msg) => assert!(err_msg.contains("NonExistentDSPUnit")),
        Ok(_) => panic!("expected unknown effect instantiation to fail"),
    }
}

#[test]
fn test_master_preset_serde_and_defaults() {
    let default_master = MasterPreset::default();
    assert_eq!(default_master.volume, 1.0);
    assert_eq!(default_master.limiter_threshold_db, -0.5);
    assert_eq!(default_master.limiter_ceiling_db, -0.1);
    assert_eq!(default_master.limiter_release_ms, 50.0);

    let json = serde_json::to_string(&default_master).unwrap();
    let parsed: MasterPreset = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, default_master);
}

#[test]
fn test_path_traversal_rejection() {
    let temp = TempDir::new("traversal");
    let mut manager = PresetManager::with_directory(&temp.path);
    let preset = texas_blues();

    let malicious_paths = [
        "../escaped",
        "../../etc/passwd",
        r"..\escaped",
        r"..\..\windows\system32",
        "/etc/passwd",
        "/absolute/path/tone",
        "subdir/tone",
        r"subdir\tone",
        "..",
        ".",
        "nested/../tone",
        "tone/..",
        "bad\0name",
    ];

    for bad_path in malicious_paths {
        // 1. load_preset rejects path traversal
        let load_res = manager.load_preset(bad_path);
        assert!(
            matches!(load_res, Err(PresetError::InvalidName(_))),
            "load_preset should reject '{bad_path}', got: {load_res:?}"
        );

        // 2. save_preset_as rejects path traversal
        let save_res = manager.save_preset_as(&preset, bad_path);
        assert!(
            matches!(save_res, Err(PresetError::InvalidName(_))),
            "save_preset_as should reject '{bad_path}', got: {save_res:?}"
        );

        // 3. delete_preset rejects path traversal
        let del_res = manager.delete_preset(bad_path);
        assert!(
            matches!(del_res, Err(PresetError::InvalidName(_))),
            "delete_preset should reject '{bad_path}', got: {del_res:?}"
        );

        // 4. validate_preset_filename helper rejects path traversal
        let val_res = PresetManager::validate_preset_filename(bad_path);
        assert!(
            matches!(val_res, Err(PresetError::InvalidName(_))),
            "validate_preset_filename should reject '{bad_path}', got: {val_res:?}"
        );
    }
}

#[test]
fn test_non_json_files_ignored_in_disk_scan() {
    let temp = TempDir::new("non_json_ignore");
    let mut manager = PresetManager::with_directory(&temp.path);

    // Create non-JSON files with invalid JSON contents matching factory preset stem
    fs::write(
        temp.path.join("TexasBlues.txt"),
        "This is plain text, not valid JSON { broken",
    )
    .unwrap();
    fs::write(
        temp.path.join("TexasBlues.wav"),
        b"RIFF\x24\x00\x00\x00WAVEfmt ",
    )
    .unwrap();
    fs::write(
        temp.path.join("TexasBlues.bak"),
        "backup corrupt binary data",
    )
    .unwrap();
    fs::write(temp.path.join("CustomTone.txt"), "non-json user file").unwrap();

    // 1. load_preset ignores TexasBlues.txt / .wav / .bak and falls back to factory preset
    let loaded = manager
        .load_preset("TexasBlues")
        .expect("should ignore non-json files and fall back to factory preset");
    assert_eq!(loaded.name, "TexasBlues");
    assert_eq!(loaded.category, "Blues");

    // 2. load_preset ignores CustomTone.txt and returns NotFound, NOT Json parse error
    let not_found = manager.load_preset("CustomTone");
    assert!(
        matches!(not_found, Err(PresetError::NotFound(_))),
        "should return NotFound for non-json CustomTone.txt, got: {not_found:?}"
    );

    // 3. list_user_presets ignores all non-json files
    let user_list = manager.list_user_presets().unwrap();
    assert!(user_list.is_empty(), "non-json files must not be listed");
}

#[test]
fn test_factory_lookup_with_json_suffix() {
    let temp = TempDir::new("factory_json_suffix");
    let mut manager = PresetManager::with_directory(&temp.path);

    for &name in &FACTORY_PRESET_NAMES {
        // Querying with lowercase .json suffix
        let query = format!("{name}.json");
        let loaded = manager
            .load_preset(&query)
            .unwrap_or_else(|e| panic!("failed to load factory preset with '{query}': {e}"));
        assert_eq!(loaded.name, name);

        // Querying with uppercase .JSON suffix
        let query_upper = format!("{name}.JSON");
        let loaded_upper = manager
            .load_preset(&query_upper)
            .unwrap_or_else(|e| panic!("failed to load factory preset with '{query_upper}': {e}"));
        assert_eq!(loaded_upper.name, name);
    }

    // Also test case-insensitive factory name with .json
    let ca = manager
        .load_preset("clean ambient.json")
        .expect("should load clean ambient.json");
    assert_eq!(ca.name, "CleanAmbient");

    let tb = manager
        .load_preset("texas_blues.json")
        .expect("should load texas_blues.json");
    assert_eq!(tb.name, "TexasBlues");
}

#[test]
fn test_atomic_write_integrity() {
    let temp = TempDir::new("atomic_write");
    let mut manager = PresetManager::with_directory(&temp.path);

    let custom = PresetData::new(
        "AtomicTone",
        ChannelPreset::new(0.88, -0.1),
        ChannelPreset::new(0.92, 0.1),
    )
    .with_description("Atomic write verification");

    // 1. Initial save via save_preset_as
    let saved_path = manager
        .save_preset_as(&custom, "AtomicTone")
        .expect("failed to save preset atomically");
    assert!(saved_path.is_file());

    // Verify written content is valid and matches
    let content = fs::read_to_string(&saved_path).unwrap();
    let reloaded: PresetData = serde_json::from_str(&content).unwrap();
    assert_eq!(reloaded.name, "AtomicTone");
    assert_eq!(reloaded.guitar_channel.volume, 0.88);

    // Verify no temporary files remain in directory
    for entry in fs::read_dir(&temp.path).unwrap() {
        let entry = entry.unwrap();
        let fname = entry.file_name().to_string_lossy().into_owned();
        assert!(
            !fname.ends_with(".tmp"),
            "temporary file left behind: {fname}"
        );
    }

    // 2. Atomic overwrite test: modify preset and save again
    let mut updated = custom.clone();
    updated.guitar_channel.volume = 0.77;
    updated.description = "Updated atomic tone".to_string();

    manager
        .save_preset_as(&updated, "AtomicTone.json")
        .expect("failed to overwrite preset atomically");

    let updated_content = fs::read_to_string(&saved_path).unwrap();
    let updated_reloaded: PresetData = serde_json::from_str(&updated_content).unwrap();
    assert_eq!(updated_reloaded.guitar_channel.volume, 0.77);
    assert_eq!(updated_reloaded.description, "Updated atomic tone");

    // Again verify no .tmp files exist
    for entry in fs::read_dir(&temp.path).unwrap() {
        let entry = entry.unwrap();
        let fname = entry.file_name().to_string_lossy().into_owned();
        assert!(
            !fname.ends_with(".tmp"),
            "temporary file left behind after overwrite: {fname}"
        );
    }

    // 3. Direct save_to_file with nested parent directories
    let nested_path = temp.path.join("subfolder").join("NestedTone.json");
    manager
        .save_to_file(&custom, &nested_path)
        .expect("save_to_file should create parents and write atomically");
    assert!(nested_path.is_file());
    let loaded_nested = manager
        .load_from_file(&nested_path)
        .expect("failed to load nested file");
    assert_eq!(loaded_nested.name, "AtomicTone");
}
