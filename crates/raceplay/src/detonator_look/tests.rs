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
        ..MineEnvelope::default()
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
        ..MineEnvelope::default()
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
        ..MineEnvelope::default()
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
fn a_pinned_live_mine_holds_the_originals_envelope_whatever_the_bands_are() {
    let mut envelopes = MineEnvelopes::default();
    let mines = [laid(5, 0, true), laid(6, 0, true), laid(7, 0, false)];
    let quiet = [0.1_f32; 16];
    let mut stepped = envelopes.advance(&mines, &quiet, 1.0);
    assert!(
        stepped.iter().all(|e| e.brightness() == 0.0),
        "quiet bands never attack"
    );
    let pin = |envelope| MinePin {
        envelope: Some(envelope),
        ..MinePin::default()
    };
    let pins = std::collections::BTreeMap::from([
        ((5, 0), pin((0.5886, 0.568, 8.375))),
        ((7, 0), pin((0.9, 0.9, 1.0))),
    ]);
    hold_pinned(&mines, &mut stepped, &mut [0.0; 3], &pins);
    assert_eq!(stepped[0].brightness(), 2.0 * 0.5886);
    assert_eq!(
        visualiser_constants(true, &stage(), &stepped[0], 0.1).pitch_offset,
        8.375,
        "the pinned PitchOffset reaches the ring"
    );
    assert_eq!(
        stepped[1],
        MineEnvelope::default(),
        "an unpinned mine keeps its own"
    );
    assert_eq!(
        stepped[2],
        MineEnvelope::default(),
        "a dark mine is never pinned"
    );
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
        ..MineEnvelope::default()
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
        ..MineEnvelope::default()
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
    let live = visualiser_constants(true, &stage(), &envelope, 0.9);
    assert_eq!(live.colour, [0.4 * 1.5, 0.0, 0.9 * 1.5]);
    assert_eq!(live.master_volume, 1.0);
    assert_eq!(live.pitch_volume, 0.4, "capped at 0.4");
    assert_eq!(live.bass_hit, 0.125);
    let dark = visualiser_constants(false, &stage(), &envelope, 0.9);
    assert_eq!(dark.colour, [2.0; 3]);
    assert_eq!(
        (dark.master_volume, dark.pitch_volume, dark.bass_hit),
        (0.0, 0.0, 0.0)
    );
}

#[test]
fn a_mine_turns_four_radians_a_second_of_its_age_and_the_live_phase_reproduces() {
    // The grab's live read: age 55.7836 on every mine, `m+0x150` 3.2228.
    assert!((spin_phase(4.0, 55.7836) - 3.2228).abs() < 1e-3);
    assert!((spin_phase(4.0, 0.25) - 1.0).abs() < 1e-6);
    assert!(spin_phase(4.0, 1000.0) < std::f32::consts::TAU);
}

#[test]
fn a_mines_basis_is_the_world_axes_turned_about_its_up_by_the_originals_rows() {
    use oag_core::math::{Mat3, Vec3};
    let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
    // About `+Y` the original's row 0 is `(c, 0, s)`: a quarter turn takes the model's X to `+Z`.
    let quarter = spun_basis(Vec3::Y, std::f32::consts::FRAC_PI_2);
    assert!(close(quarter.x_axis, Vec3::Z), "{quarter:?}");
    assert!(close(quarter.y_axis, Vec3::Y));
    assert!(close(quarter.z_axis, -Vec3::X));
    // At phase 0 it is the world axes whatever the up axis, not a frame built on it.
    let tilted = Vec3::new(0.2, 0.9, 0.1).normalize();
    assert!(spun_basis(tilted, 0.0).abs_diff_eq(Mat3::IDENTITY, 1e-6));
    // The up axis is the one the turn leaves alone.
    assert!(close(spun_basis(tilted, 2.0) * tilted, tilted));
}

#[test]
fn a_mines_age_counts_from_its_laying_and_a_pin_holds_it_live_or_dark() {
    let mut envelopes = MineEnvelopes::default();
    let a = laid(5, 0, true);
    let dark = laid(6, 1, false);
    envelopes.advance(&[a], &[0.0; 16], 1.0);
    envelopes.advance(&[a, dark], &[0.0; 16], 2.0);
    let mut ages = envelopes.ages(&[a, dark], 3.5);
    assert_eq!(ages, [2.5, 1.5]);
    let pins = std::collections::BTreeMap::from([(
        (6, 1),
        MinePin {
            age: Some(55.7002),
            ..MinePin::default()
        },
    )]);
    let mut stepped = vec![MineEnvelope::default(); 2];
    hold_pinned(&[a, dark], &mut stepped, &mut ages, &pins);
    assert_eq!(ages, [2.5, 55.7002], "a dark mine's age is pinned too");
}

/// `MasterVolume` and `PitchVolume` are both the one level `0x00304cc8` returns: `clamp01(5 L)` and
/// `clamp(L, 0, 0.4)`. The original's reads on `mines-ahead` (`detonator-mine-material.md`,
/// "The visualiser's level") give `(0.7069, 0.1414)` and `(1.0, 0.3154)`: five times where unclamped.
#[test]
fn both_visualiser_volumes_are_the_one_whole_spectrum_level() {
    let envelope = MineEnvelope::default();
    let close =
        |(m, p): (f32, f32), (em, ep): (f32, f32)| (m - em).abs() < 1e-4 && (p - ep).abs() < 1e-6;
    for (level, read) in [
        (0.141_378_43, (0.706_892_1, 0.141_378_43)),
        (0.315_428_3, (1.0, 0.315_428_3)),
    ] {
        let v = visualiser_constants(true, &stage(), &envelope, level);
        assert!(
            close((v.master_volume, v.pitch_volume), read),
            "{level}: {v:?}"
        );
    }
    assert_eq!(visualiser_volumes(0.0), (0.0, 0.0));
    assert_eq!(visualiser_volumes(0.7), (1.0, 0.4));
}

/// A pinned mine draws the original's two volumes whatever ours' level is; an unpinned or dark one
/// does not take them.
#[test]
fn a_pinned_live_mine_holds_the_originals_volumes() {
    let mines = [laid(5, 0, true), laid(6, 0, false), laid(7, 0, true)];
    let mut stepped = vec![MineEnvelope::default(); 3];
    let pin = MinePin {
        volumes: Some((1.0, 0.3154)),
        ..MinePin::default()
    };
    let pins = std::collections::BTreeMap::from([((5, 0), pin), ((6, 0), pin)]);
    hold_pinned(&mines, &mut stepped, &mut [0.0; 3], &pins);
    let v = visualiser_constants(true, &stage(), &stepped[0], 0.05);
    assert_eq!((v.master_volume, v.pitch_volume), (1.0, 0.3154));
    assert_eq!(stepped[1].volumes, None, "a dark mine keeps its literals");
    let own = visualiser_constants(true, &stage(), &stepped[2], 0.05);
    assert_eq!((own.master_volume, own.pitch_volume), (0.25, 0.05));
}
