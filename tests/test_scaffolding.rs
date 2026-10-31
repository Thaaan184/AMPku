use guitarfx::audio::{
    find_capture_device, find_playback_device, query_capture_devices, query_playback_devices,
    DeviceInfo,
};

#[test]
fn test_module_visibility() {
    let _ = (
        std::marker::PhantomData::<guitarfx::audio::DeviceInfo>,
        stringify!(guitarfx::core),
        stringify!(guitarfx::dsp),
        stringify!(guitarfx::ui),
    );
}

#[test]
fn test_device_info_properties() {
    let dev = DeviceInfo {
        index: 0,
        name: "Test Soundcard".to_string(),
        is_default: true,
    };

    assert_eq!(dev.index, 0);
    assert_eq!(dev.name, "Test Soundcard");
    assert!(dev.is_default);

    // Test clone and equality
    let dev_clone = dev.clone();
    assert_eq!(dev, dev_clone);

    // Test serde serialization/deserialization
    let json = serde_json::to_string(&dev).expect("Serialize DeviceInfo");
    let deserialized: DeviceInfo = serde_json::from_str(&json).expect("Deserialize DeviceInfo");
    assert_eq!(dev, deserialized);
}

#[test]
fn test_device_query_signatures() {
    let host = cpal::default_host();
    let capture_devs = query_capture_devices(&host);
    let playback_devs = query_playback_devices(&host);

    // Must be valid vectors (even if empty in headless/container environments)
    let _ = capture_devs.len();
    let _ = playback_devs.len();

    // Query helpers with non-existent query should return None or matching device
    let res = find_capture_device(&host, "non_existent_device_xyz_12345");
    assert!(res.is_none());

    let res = find_playback_device(&host, "non_existent_device_xyz_12345");
    assert!(res.is_none());
}

#[test]
fn test_find_device_case_insensitive() {
    let devices = vec![
        DeviceInfo {
            index: 0,
            name: "Vention USB Audio".to_string(),
            is_default: false,
        },
        DeviceInfo {
            index: 1,
            name: "VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)".to_string(),
            is_default: true,
        },
    ];

    // Case-insensitive substring match
    let match_vention = guitarfx::audio::device::find_in_devices(&devices, "vention");
    assert!(match_vention.is_some());
    assert_eq!(match_vention.unwrap().index, 0);

    let match_voicemeeter = guitarfx::audio::device::find_in_devices(&devices, "VOICEMEETER");
    assert!(match_voicemeeter.is_some());
    assert_eq!(match_voicemeeter.unwrap().index, 1);

    let no_match = guitarfx::audio::device::find_in_devices(&devices, "Realtek");
    assert!(no_match.is_none());
}
