use super::*;

fn stage() -> DetonatorColours {
    DetonatorColours {
        mine: [122.0 / 255.0, 25.0 / 255.0, 1.0],
        mine_electricity: [0.4, 0.0, 0.9],
        bomb_inner: [1.0, 0.5, 0.25],
        bomb_outer: [1.0, 0.4, 0.1],
    }
}

#[test]
fn a_band_that_rises_by_more_than_point_three_takes_the_envelope_to_it() {
    assert_eq!(follow(0.0, 0.31), 0.31);
    assert_eq!(follow(0.5, 0.9), 0.9);
}

#[test]
fn a_band_that_rises_by_exactly_point_three_or_less_does_not_attack() {
    // `s + 0.3 < band` is strict: 0.5 + 0.3 = 0.8 is not below 0.8.
    assert_eq!(follow(0.5, 0.8), 0.5 * DECAY);
    assert_eq!(follow(0.5, 0.2), 0.5 * DECAY);
}

#[test]
fn the_envelope_falls_five_percent_an_update_and_the_band_is_clamped_first() {
    let mut level = 1.0;
    for _ in 0..2 {
        level = follow(level, 0.0);
    }
    assert!((level - 0.9025).abs() < 1e-6);
    // A band past one is held to one, and a negative one to zero.
    assert_eq!(follow(0.0, 7.0), 1.0);
    assert_eq!(follow(0.0, -3.0), 0.0);
}

#[test]
fn a_mine_follows_a_low_band_with_its_football_and_the_band_eleven_above_with_its_spikes() {
    let mut levels = [0.0_f32; 16];
    levels[2] = 0.8;
    levels[13] = 0.6;
    let mut envelope = MineEnvelope::default();
    envelope.step(&levels, 2);
    assert_eq!(envelope.football, 0.8);
    assert_eq!(envelope.spikes, 0.6);
    // Another low band hears neither.
    let mut other = MineEnvelope::default();
    other.step(&levels, 1);
    assert_eq!(other, MineEnvelope::default());
}

#[test]
fn a_band_past_the_sixteen_is_silent() {
    assert_eq!(band_level(&[0.5; 16], 16), 0.0);
    assert_eq!(band_level(&[], 0), 0.0);
}

#[test]
fn brightness_is_twice_the_football_envelope() {
    let envelope = MineEnvelope {
        football: 0.4,
        spikes: 1.0,
    };
    assert_eq!(envelope.brightness(), 0.8);
}

#[test]
fn every_mine_picks_one_of_four_bands_and_the_pick_is_stable() {
    let mut seen = [false; BAND_CHOICES];
    for entry in 0..64_u16 {
        for lane in -2..=2_i8 {
            let band = mine_band(entry, lane);
            assert!(band < BAND_CHOICES);
            assert_eq!(band, mine_band(entry, lane));
            seen[band] = true;
        }
    }
    assert_eq!(seen, [true; BAND_CHOICES]);
}

#[test]
fn a_live_mine_takes_the_stage_colour_and_the_envelope() {
    let envelope = MineEnvelope {
        football: 0.5,
        spikes: 0.0,
    };
    for part in [MinePart::Football, MinePart::Spikes] {
        let c = mine_constants(true, part, &stage(), &envelope);
        assert_eq!(c.colour, stage().mine);
        assert_eq!(c.brightness, 1.0);
    }
}

#[test]
fn a_dark_mine_takes_its_literals_whatever_the_stage_and_the_music() {
    let loud = MineEnvelope {
        football: 1.0,
        spikes: 1.0,
    };
    let football = mine_constants(false, MinePart::Football, &stage(), &loud);
    assert_eq!((football.colour, football.brightness), ([2.0; 3], 0.0));
    let spikes = mine_constants(false, MinePart::Spikes, &stage(), &loud);
    assert_eq!((spikes.colour, spikes.brightness), ([10.0; 3], 0.0));
}

#[test]
fn a_bomb_takes_inner_and_outer_and_its_flash_lifts_both_brightnesses() {
    let rest = bomb_constants(&stage(), 0.0);
    assert_eq!(rest.inner, stage().bomb_inner);
    assert_eq!(rest.outer, stage().bomb_outer);
    assert_eq!(
        (rest.body_brightness, rest.equaliser_brightness),
        (0.2, 0.8)
    );
    let lit = bomb_constants(&stage(), 0.5);
    assert_eq!((lit.body_brightness, lit.equaliser_brightness), (1.2, 1.8));
}

fn laid(entry: u16, lane: i8, live: bool) -> LaidMine {
    LaidMine { entry, lane, live }
}

#[test]
fn an_envelope_steps_once_per_clock_value_and_a_repeated_clock_changes_nothing() {
    let mut envelopes = MineEnvelopes::default();
    let mine = [laid(5, 0, true)];
    let band = mine_band(5, 0);
    let mut levels = [0.0_f32; 16];
    levels[band] = 1.0;
    let first = envelopes.advance(&mine, &levels, 1.0)[0];
    assert_eq!(first.football, 1.0);
    // The same tick again (a second view, a frame between ticks) with the band gone quiet.
    let again = envelopes.advance(&mine, &[0.0; 16], 1.0)[0];
    assert_eq!(again, first);
    // The next tick lets it fall five percent.
    let next = envelopes.advance(&mine, &[0.0; 16], 1.0 + 1.0 / 60.0)[0];
    assert_eq!(next.football, DECAY);
}

#[test]
fn an_envelope_belongs_to_its_mine_not_to_a_pool_slot() {
    let mut envelopes = MineEnvelopes::default();
    let (a, b) = (laid(3, 0, true), laid(9, 1, true));
    let mut levels = [0.0_f32; 16];
    levels[mine_band(3, 0)] = 0.9;
    let before = envelopes.advance(&[a, b], &levels, 1.0);
    // `a` is shot and `b` slides into slot 0: it keeps its own envelope.
    let after = envelopes.advance(&[b], &[0.0; 16], 2.0);
    assert_eq!(after[0].football, before[1].football * DECAY);
    // `a` is gone for good: laid again later it starts from nothing.
    let again = envelopes.advance(&[a], &[0.0; 16], 3.0);
    assert_eq!(again[0], MineEnvelope::default());
}

#[test]
fn a_dark_mine_never_steps() {
    let mut envelopes = MineEnvelopes::default();
    let dark = [laid(4, 0, false)];
    let envelope = envelopes.advance(&dark, &[1.0; 16], 1.0)[0];
    assert_eq!(envelope, MineEnvelope::default());
}

#[test]
fn the_football_and_spikes_sizes_follow_the_envelopes_the_way_the_original_writes_them() {
    // The values one mine held in the original's heap (`m+0xf4`, `m+0xf8`, `m+0xd8`), Talon's Junction,
    // lap clock 2.7 s: `scale = clamp(2.3 s, 1, 2.3)` read back as 1.3537.
    let held = MineEnvelope {
        football: 0.5886,
        spikes: 0.568,
    };
    assert!((held.football_scale() - 1.3537).abs() < 1e-3);
    assert!(
        (held.spike_scale() - 2.272).abs() < 1e-3,
        "4 t, under scale + 0.95"
    );
    assert!((held.brightness() - 1.1771).abs() < 1e-3, "m+0x100");
    // A silent mine is size 1 and the spikes never pass the football by more than 0.95.
    let silent = MineEnvelope::default();
    assert_eq!((silent.football_scale(), silent.spike_scale()), (1.0, 1.0));
    let loud = MineEnvelope {
        football: 1.0,
        spikes: 1.0,
    };
    assert_eq!(loud.football_scale(), 2.3);
    assert!((loud.spike_scale() - 3.25).abs() < 1e-6);
}

#[test]
fn a_live_visualiser_takes_the_stage_colour_times_one_and_a_half_and_a_dark_one_the_literals() {
    let envelope = MineEnvelope {
        football: 0.5,
        ..MineEnvelope::default()
    };
    let mut levels = [0.0_f32; 16];
    levels[0] = 0.1;
    levels[1] = 0.9;
    let live = visualiser_constants(true, &stage(), &envelope, &levels);
    assert_eq!(live.colour, [0.4 * 1.5, 0.0, 0.9 * 1.5]);
    assert_eq!(live.master_volume, 0.5);
    assert_eq!(live.pitch_volume, 0.4, "capped at 0.4");
    assert_eq!(live.bass_hit, 0.125);
    let dark = visualiser_constants(false, &stage(), &envelope, &levels);
    assert_eq!(dark.colour, [2.0; 3]);
    assert_eq!(
        (dark.master_volume, dark.pitch_volume, dark.bass_hit),
        (0.0, 0.0, 0.0)
    );
}
