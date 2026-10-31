//! Unit and integration tests for AMPku egui channel strips and modular effect cards.

use ampku::dsp::cab_sim::{CabinetProfile, CabinetSim};
use ampku::dsp::dynamics::{Compressor, NoiseGate};
use ampku::dsp::filters::biquad::{BiquadFilter, FilterType};
use ampku::dsp::saturation::{Distortion, Fuzz, Overdrive};
use ampku::dsp::time::{StereoDelay, StereoReverb};
use ampku::dsp::traits::AudioEffect;
use ampku::dsp::vocal::VocalDeEsser;
use ampku::engine::channel::ChannelStrip;
use ampku::gui::channel_strip::{
    render_channel_strip, render_dual_channel_strips, AvailableEffect,
};
use ampku::gui::effect_card::{render_effect_card, render_effect_card_indexed, EffectCardAction};
use ampku::gui::theme::{
    apply_studio_theme, channel_strip_frame, effect_card_frame, meter_container_frame,
    COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_BG, COLOR_CARD,
};

fn run_in_egui<F: FnOnce(&mut egui::Ui)>(f: F) {
    let ctx = egui::Context::default();
    apply_studio_theme(&ctx);
    let raw_input = egui::RawInput::default();
    let _ = ctx.run(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            f(ui);
        });
    });
}

#[test]
fn test_theme_constants_and_application() {
    assert_eq!(COLOR_BG, egui::Color32::from_rgb(0x18, 0x18, 0x1b));
    assert_eq!(COLOR_CARD, egui::Color32::from_rgb(0x27, 0x27, 0x2a));
    assert_eq!(COLOR_ACCENT_BLUE, egui::Color32::from_rgb(0x38, 0xbd, 0xf8));
    assert_eq!(COLOR_ALERT_RED, egui::Color32::from_rgb(0xef, 0x44, 0x44));

    let ctx = egui::Context::default();
    apply_studio_theme(&ctx);

    let visuals = ctx.style().visuals.clone();
    assert_eq!(visuals.panel_fill, COLOR_BG);
    assert_eq!(visuals.window_fill, COLOR_CARD);
    assert_eq!(visuals.selection.bg_fill, COLOR_ACCENT_BLUE);

    let _card_frame = effect_card_frame(true);
    let _strip_frame = channel_strip_frame();
    let _meter_frame = meter_container_frame();
}

#[test]
fn test_available_effects_instantiation() {
    assert_eq!(AvailableEffect::ALL.len(), 10);

    for effect_kind in AvailableEffect::ALL {
        let boxed = effect_kind.instantiate(48000.0);
        assert!(!boxed.name().is_empty());
        assert!(boxed.is_enabled());
    }
}

#[test]
fn test_all_ten_effect_cards_render() {
    run_in_egui(|ui| {
        let mut noise_gate = NoiseGate::new(48000.0, -40.0, 5.0, 50.0);
        let mut compressor = Compressor::new(48000.0, -20.0, 4.0, 10.0, 100.0, 0.0);
        let mut overdrive = Overdrive::with_sample_rate(4.0, 0.5, 0.7, 48000.0);
        let mut distortion = Distortion::with_sample_rate(8.0, 0.5, 0.7, 48000.0);
        let mut fuzz = Fuzz::with_sample_rate(12.0, 0.5, 0.6, 48000.0);
        let mut biquad = BiquadFilter::new(
            FilterType::Peaking,
            48000.0,
            1000.0,
            std::f32::consts::FRAC_1_SQRT_2,
            3.0,
        );
        let mut cab_sim = CabinetSim::with_sample_rate(CabinetProfile::CelestionV30, 48000.0);
        let mut delay = StereoDelay::with_options(48000.0, 350.0, 0.3, 0.25, 0.2, true, 0.8);
        let mut reverb = StereoReverb::new(48000.0, 0.5, 0.5, 1.0, 0.3);
        let mut de_esser = VocalDeEsser::new(48000.0, 6500.0, -20.0, 4.0);

        let mut effects: Vec<&mut dyn AudioEffect> = vec![
            &mut noise_gate,
            &mut compressor,
            &mut overdrive,
            &mut distortion,
            &mut fuzz,
            &mut biquad,
            &mut cab_sim,
            &mut delay,
            &mut reverb,
            &mut de_esser,
        ];

        let total = effects.len();
        for (i, effect) in effects.iter_mut().enumerate() {
            let action = render_effect_card_indexed(ui, *effect, i, total);
            assert_eq!(action, EffectCardAction::None);
        }
    });
}

#[test]
fn test_effect_card_downcasting_integrity() {
    let mut gate: Box<dyn AudioEffect> = Box::new(NoiseGate::new(48000.0, -35.0, 4.0, 40.0));
    let mut comp: Box<dyn AudioEffect> =
        Box::new(Compressor::new(48000.0, -18.0, 3.5, 8.0, 80.0, 2.0));
    let mut od: Box<dyn AudioEffect> =
        Box::new(Overdrive::with_sample_rate(5.0, 0.6, 0.8, 48000.0));
    let mut dist: Box<dyn AudioEffect> =
        Box::new(Distortion::with_sample_rate(10.0, 0.4, 0.7, 48000.0));
    let mut fuzz: Box<dyn AudioEffect> = Box::new(Fuzz::with_sample_rate(15.0, 0.7, 0.5, 48000.0));
    let mut filter: Box<dyn AudioEffect> = Box::new(BiquadFilter::new(
        FilterType::HighPass,
        48000.0,
        80.0,
        0.707,
        0.0,
    ));
    let mut cab: Box<dyn AudioEffect> = Box::new(CabinetSim::with_sample_rate(
        CabinetProfile::Marshall1960,
        48000.0,
    ));
    let mut delay: Box<dyn AudioEffect> = Box::new(StereoDelay::with_options(
        48000.0, 400.0, 0.4, 0.3, 0.1, false, 0.5,
    ));
    let mut reverb: Box<dyn AudioEffect> =
        Box::new(StereoReverb::new(48000.0, 0.7, 0.4, 0.9, 0.25));
    let mut de_esser: Box<dyn AudioEffect> =
        Box::new(VocalDeEsser::new(48000.0, 7000.0, -22.0, 5.0));

    assert!(gate.as_any().is::<NoiseGate>());
    assert!(gate.as_any_mut().downcast_mut::<NoiseGate>().is_some());

    assert!(comp.as_any().is::<Compressor>());
    assert!(comp.as_any_mut().downcast_mut::<Compressor>().is_some());

    assert!(od.as_any().is::<Overdrive>());
    assert!(od.as_any_mut().downcast_mut::<Overdrive>().is_some());

    assert!(dist.as_any().is::<Distortion>());
    assert!(dist.as_any_mut().downcast_mut::<Distortion>().is_some());

    assert!(fuzz.as_any().is::<Fuzz>());
    assert!(fuzz.as_any_mut().downcast_mut::<Fuzz>().is_some());

    assert!(filter.as_any().is::<BiquadFilter>());
    assert!(filter.as_any_mut().downcast_mut::<BiquadFilter>().is_some());

    assert!(cab.as_any().is::<CabinetSim>());
    assert!(cab.as_any_mut().downcast_mut::<CabinetSim>().is_some());

    assert!(delay.as_any().is::<StereoDelay>());
    assert!(delay.as_any_mut().downcast_mut::<StereoDelay>().is_some());

    assert!(reverb.as_any().is::<StereoReverb>());
    assert!(reverb.as_any_mut().downcast_mut::<StereoReverb>().is_some());

    assert!(de_esser.as_any().is::<VocalDeEsser>());
    assert!(de_esser
        .as_any_mut()
        .downcast_mut::<VocalDeEsser>()
        .is_some());
}

#[test]
fn test_render_channel_strip_empty_rack() {
    let mut channel = ChannelStrip::with_name("Guitar");

    run_in_egui(|ui| {
        render_channel_strip(ui, &mut channel, "Guitar (Ch 1)");
    });

    assert_eq!(channel.name(), "Guitar");
    assert!(!channel.is_muted());
    assert!(!channel.is_solo());
    assert!(channel.is_stereo());
}

#[test]
fn test_render_channel_strip_with_populated_rack() {
    let mut channel = ChannelStrip::with_name("Mic");
    let gate = Box::new(NoiseGate::new(48000.0, -45.0, 5.0, 50.0));
    let comp = Box::new(Compressor::new(48000.0, -20.0, 4.0, 10.0, 100.0, 0.0));
    let de_esser = Box::new(VocalDeEsser::new(48000.0, 6000.0, -20.0, 4.0));

    channel.chain_mut().add_effect(gate);
    channel.chain_mut().add_effect(comp);
    channel.chain_mut().add_effect(de_esser);

    assert_eq!(channel.chain().len(), 3);

    run_in_egui(|ui| {
        render_channel_strip(ui, &mut channel, "Mic (Ch 2)");
    });

    assert_eq!(channel.chain().len(), 3);
}

#[test]
fn test_render_dual_channel_strips() {
    let mut guitar = ChannelStrip::with_name("Guitar");
    let mut mic = ChannelStrip::with_name("Mic");

    guitar
        .chain_mut()
        .add_effect(Box::new(Overdrive::new(4.0, 0.5, 0.7)));
    mic.chain_mut()
        .add_effect(Box::new(VocalDeEsser::new(48000.0, 6500.0, -20.0, 4.0)));

    run_in_egui(|ui| {
        render_dual_channel_strips(ui, &mut guitar, &mut mic);
    });

    assert_eq!(guitar.chain().len(), 1);
    assert_eq!(mic.chain().len(), 1);
}

#[test]
fn test_channel_strip_controls_mutation() {
    let channel = ChannelStrip::with_name("Guitar");

    // Test gain / volume dB
    channel.set_volume_db(-6.0);
    assert!((channel.volume_db() - (-6.0)).abs() < 0.1);

    // Test pan
    channel.set_pan(0.5);
    assert!((channel.pan() - 0.5).abs() < 1e-4);

    // Test mute toggle
    assert!(!channel.is_muted());
    channel.toggle_mute();
    assert!(channel.is_muted());
    channel.toggle_mute();
    assert!(!channel.is_muted());

    // Test solo toggle
    assert!(!channel.is_solo());
    channel.toggle_solo();
    assert!(channel.is_solo());
    channel.toggle_solo();
    assert!(!channel.is_solo());

    // Test mono/stereo
    assert!(channel.is_stereo());
    channel.set_mono(true);
    assert!(channel.is_mono());
    assert!(!channel.is_stereo());
}

#[test]
fn test_render_effect_card_direct() {
    let mut overdrive = Overdrive::with_sample_rate(5.0, 0.5, 0.7, 48000.0);

    run_in_egui(|ui| {
        let action = render_effect_card(ui, &mut overdrive);
        assert_eq!(action, EffectCardAction::None);
    });

    assert!(overdrive.is_enabled());
}
