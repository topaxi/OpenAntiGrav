use super::*;

fn channel(rise: [i32; 4], fall: [i32; 4]) -> ChannelCfg {
    ChannelCfg {
        edges: [450 << 16, 511 << 16, 511 << 16, 511 << 16],
        rise,
        fall,
        scale: 0x10000,
        bias: 0,
    }
}

#[test]
fn the_band_is_picked_by_the_current_value_and_wraps_above_every_edge() {
    let cfg = channel([0; 4], [0; 4]);
    assert_eq!(band_of(&cfg, 0), 0);
    assert_eq!(
        band_of(&cfg, 450 << 16),
        0,
        "at the first edge is still band 0"
    );
    assert_eq!(band_of(&cfg, 451 << 16), 1);
    assert_eq!(band_of(&cfg, 511 << 16), 1);
    // The decompile's last arm: above the third edge and not above the fourth is
    // band 3, above all four is back to band 0.
    let wide = ChannelCfg {
        edges: [10 << 16, 20 << 16, 30 << 16, 40 << 16],
        ..cfg
    };
    assert_eq!(band_of(&wide, 35 << 16), 3);
    assert_eq!(band_of(&wide, 41 << 16), 0);
}

#[test]
fn a_zero_rate_snaps_and_a_slow_rate_slews_by_rate_times_milliseconds() {
    let snap = channel([0; 4], [0; 4]);
    let mut s = Smoother::default();
    s.set_input(&snap, 300);
    s.step(&snap, 16);
    assert_eq!(s.value, 300 << 16, "no rate, no smoothing");

    // 0.2 counts per millisecond, rising: 16 ms moves 3.2 counts.
    let slow = channel([0x3333; 4], [0x3333; 4]);
    let mut s = Smoother::default();
    s.set_input(&slow, 100);
    s.step(&slow, 16);
    assert_eq!(s.value, 0x3333 * 16);
    // Falling uses the fall rates and lands exactly when the step overshoots.
    let mut s = Smoother {
        value: 100 << 16,
        target: 100 << 16,
    };
    s.set_input(&slow, 99);
    s.step(&slow, 16);
    assert_eq!(s.value, 99 << 16);
}

#[test]
fn a_step_is_capped_at_five_seconds() {
    let slow = channel([1; 4], [1; 4]);
    let mut s = Smoother::default();
    s.set_input(&slow, 511);
    s.step(&slow, 1_000_000);
    assert_eq!(s.value, 5000);
}

#[test]
fn channel_inputs_follow_the_per_tick_law_and_only_the_player_has_a_throttle() {
    let player = Craft::channel_inputs(
        Inputs {
            speed_field: 400.0,
            throttle: Some(100.0),
        },
        Law::Hd,
    );
    // trunc(0.5 * 400 + 5 * 2.164) = 210, channels 1 and 2 held at zero,
    // trunc(5.12 * 100) = 512 clamped to 511.
    assert_eq!(player, [Some(210), Some(0), Some(0), Some(511), None]);
    let rival = Craft::channel_inputs(
        Inputs {
            speed_field: 1100.0,
            throttle: None,
        },
        Law::Hd,
    );
    assert_eq!(rival[0], Some(511), "channel 0 clamps at 511");
    assert_eq!(rival[3], None, "an opponent never writes channel 3");
    let idle = Craft::channel_inputs(
        Inputs {
            speed_field: 0.0,
            throttle: Some(0.0),
        },
        Law::Hd,
    );
    assert_eq!(idle[0], Some(10), "the grid value: trunc(5 * 2.164)");
    assert_eq!(idle[3], Some(0));
}

#[test]
fn a_layer_reads_gain_over_unity_and_pitch_through_the_bend_law() {
    let mut gain = vec![0i16; 512];
    let mut pitch = vec![0x200i16; 512];
    gain[100] = 0x400;
    gain[200] = 0x200;
    pitch[100] = 0x400;
    pitch[200] = 0;
    let layer = LayerCfg {
        name: "x".into(),
        kind: 0,
        cue: 0,
        link: -1,
        channel: 0,
        gain,
        pitch,
    };
    assert_eq!(
        level_at(&layer, 100 << 16),
        Level {
            gain: 1.0,
            bend: 0x7fff
        }
    );
    let low = level_at(&layer, 200 << 16);
    assert_eq!(low.gain, 0.5);
    assert_eq!(low.bend, -0x7fff);
    assert_eq!(level_at(&layer, 0).bend, 0, "0x200 is neutral");
    // Past the table the last entry holds.
    assert_eq!(level_at(&layer, 9999 << 16), level_at(&layer, 511 << 16));
}

#[test]
fn a_bend_is_linear_in_semitones_over_the_descriptor_range() {
    assert!(
        (bend_ratio(0x7fff, 12, 12) - 2.0).abs() < 1e-6,
        "full up, an octave"
    );
    assert!(
        (bend_ratio(-0x8000, 12, 12) - 0.5).abs() < 1e-6,
        "full down, an octave"
    );
    assert_eq!(bend_ratio(0x7fff, 0, 0), 1.0, "a zero range is inaudible");
    assert_eq!(bend_ratio(0, 12, 12), 1.0);
    // Different ranges up and down: only the range of the side bent to counts.
    assert!((bend_ratio(0x7fff, 0, 12) - 2.0).abs() < 1e-6);
    assert_eq!(bend_ratio(-0x8000, 0, 12), 1.0);
}

#[test]
fn a_slot_team_selects_the_disc_table_it_is_named_by() {
    assert_eq!(table_name("Goteki"), "goteki");
    assert_eq!(table_name("goteki_c1"), "goteki");
    assert_eq!(table_name("AG_Systems_n1"), "ag_systems");
    assert_eq!(table_name("detonator"), "det");
}

fn team_with_two_layers() -> Arc<Team> {
    let ramp: Vec<i16> = (0..512).map(|x| (x * 2) as i16).collect();
    let fall: Vec<i16> = ramp.iter().map(|g| 0x400 - g).collect();
    let flat = vec![0x200i16; 512];
    let table = Table {
        channels: vec![channel([0; 4], [0; 4]); 4],
        layers: vec![
            LayerCfg {
                name: "falls".into(),
                kind: 0,
                cue: 0,
                link: -1,
                channel: 0,
                gain: fall,
                pitch: flat.clone(),
            },
            LayerCfg {
                name: "rises".into(),
                kind: 0,
                cue: 0,
                link: -1,
                channel: 0,
                gain: ramp,
                pitch: flat,
            },
        ],
    };
    let sound = Arc::new(Sound::new(vec![8000; 2205], 1, 22_050).expect("a sound"));
    let layer = || {
        Some(LayerSound {
            sound: Arc::clone(&sound),
            down: 12,
            up: 12,
        })
    };
    Arc::new(Team {
        law: Law::Hd,
        table,
        sounds: vec![layer(), layer()],
    })
}

fn ears() -> oag_audio::Listener {
    oag_audio::Listener {
        position: [0.0; 3],
        right: [1.0, 0.0, 0.0],
    }
}

fn run(craft: &mut Craft, mixer: &mut Mixer, speed_field: f32, ticks: usize) {
    for _ in 0..ticks {
        craft.tick(
            mixer,
            Inputs {
                speed_field,
                throttle: None,
            },
            true,
            [0.0; 3],
            &ears(),
            false,
            1.0 / 60.0,
        );
    }
}

#[test]
fn the_layers_crossfade_with_speed_and_the_craft_holds_one_voice_per_audible_layer() {
    let mut mixer = Mixer::new(48_000);
    let mut craft = Craft::new(team_with_two_layers());
    run(&mut craft, &mut mixer, 0.0, 10);
    let rest = (craft.level(0).unwrap().gain, craft.level(1).unwrap().gain);
    run(&mut craft, &mut mixer, 1000.0, 10);
    let fast = (craft.level(0).unwrap().gain, craft.level(1).unwrap().gain);
    assert!(
        rest.0 > 0.9 && rest.1 < 0.1,
        "at rest the first layer leads: {rest:?}"
    );
    assert!(
        fast.0 < 0.1 && fast.1 > 0.9,
        "at speed the second leads: {fast:?}"
    );
    // A layer under the audible floor holds no voice: only the leader sounds.
    assert_eq!(craft.open_voices(), 1);
}

#[test]
fn a_silent_layer_releases_its_voice_and_a_stopped_craft_releases_all() {
    let mut mixer = Mixer::new(48_000);
    let mut craft = Craft::new(team_with_two_layers());
    // A craft whose speed pins channel 0 at the top: layer 0 reads gain 0 there.
    run(&mut craft, &mut mixer, 2000.0, 10);
    assert_eq!(
        craft.open_voices(),
        1,
        "only the audible layer holds a voice"
    );
    craft.tick(
        &mut mixer,
        Inputs {
            speed_field: 0.0,
            throttle: None,
        },
        false,
        [0.0; 3],
        &ears(),
        false,
        1.0 / 60.0,
    );
    assert_eq!(craft.open_voices(), 0);
}

#[test]
fn the_distance_factor_follows_the_eight_measured_craft() {
    // (distance from the listener, slot+4 / 1024) read live on the grid.
    let measured = [
        (8.7, 0.9912),
        (34.4, 0.9707),
        (47.5, 0.9375),
        (69.8, 0.8760),
        (86.4, 0.8311),
        (109.2, 0.7705),
        (126.8, 0.7197),
        (147.6, 0.6602),
    ];
    for (distance, factor) in measured {
        let ours = distance_factor(distance);
        assert!(
            (ours - factor).abs() < 0.01,
            "{distance} units: read {factor}, fitted {ours}"
        );
    }
    assert_eq!(distance_factor(1000.0), 0.0, "far craft are silent");
    assert!(
        distance_factor(0.0) < 1.0,
        "the nearest craft never reads full"
    );
}

#[test]
fn an_engine_layer_is_louder_by_the_square_of_its_volume() {
    // `v86 / 1024` enters the original's final gain squared: halving a layer's
    // volume quarters its voice gain, 3 points of the live scan at once.
    let full = ENGINE_BUS_RATIO * (1.0f32 * distance_factor(0.0)).powi(2);
    let half = ENGINE_BUS_RATIO * (0.5f32 * distance_factor(0.0)).powi(2);
    assert!((half / full - 0.25).abs() < 1e-6);
    assert!((ENGINE_BUS_RATIO - 0.462).abs() < 0.001);
}

#[test]
fn the_vita_class_writes_five_channels_and_clamps_at_510() {
    let player = Craft::channel_inputs(
        Inputs {
            speed_field: 400.0,
            throttle: Some(75.0),
        },
        Law::Vita2048,
    );
    // trunc(0.5 * 400 * 1.0), three zeros, trunc(70 * 0.75 / 0.75).
    assert_eq!(player, [Some(200), Some(0), Some(0), Some(0), Some(70)]);
    let rival = Craft::channel_inputs(
        Inputs {
            speed_field: 2000.0,
            throttle: None,
        },
        Law::Vita2048,
    );
    assert_eq!(rival[0], Some(510), "the Vita classes clamp at 510.0");
    assert_eq!(rival[4], None, "an opponent writes no pedal in this port");
}

#[test]
fn a_slot_team_names_its_table_without_its_livery_directory() {
    assert_eq!(table_name(r"Feisar2048\3"), "feisar2048");
    assert_eq!(table_name("Qirex2048/1"), "qirex2048");
    assert_eq!(table_name("goteki_c1"), "goteki");
    assert_eq!(table_name("Detonator"), "det");
    assert_eq!(law_of("auricom2048"), Law::Vita2048);
    assert_eq!(law_of("auricom"), Law::Hd);
}

fn flat_layer(kind: u8, link: i8, channel: usize, gain: i16, pitch: i16) -> LayerCfg {
    LayerCfg {
        name: String::new(),
        kind,
        cue: 0,
        link,
        channel,
        gain: vec![gain; CURVE],
        pitch: vec![pitch; CURVE],
    }
}

const CURVE: usize = 512;

#[test]
fn a_kind_two_layer_scales_and_bends_the_layer_it_links_to_and_plays_nothing_itself() {
    let table = Table {
        channels: vec![channel([0; 4], [0; 4]); CHANNELS],
        layers: vec![
            flat_layer(0, -1, 0, 0x400, 0x200),
            flat_layer(2, 0, 4, 0x200, 0x300),
        ],
    };
    let team = Arc::new(Team {
        law: Law::Vita2048,
        sounds: vec![None, None],
        table,
    });
    let mut craft = Craft::new(team);
    let before = craft.level(0).unwrap();
    assert_eq!((before.gain, before.bend), (1.0, 0), "no modulator has run");
    let mut mixer = Mixer::new(48_000);
    let listener = ears();
    craft.tick(
        &mut mixer,
        Inputs {
            speed_field: 0.0,
            throttle: None,
        },
        true,
        [0.0; 3],
        &listener,
        false,
        1.0 / 60.0,
    );
    let after = craft.level(0).unwrap();
    assert_eq!(after.gain, 0.5, "0x200 over unity");
    assert_eq!(after.bend, ((0x300 - 0x200) * 0x7fff) >> 9);
    assert_eq!(craft.open_voices(), 0, "a modulator owns no voice");
}
