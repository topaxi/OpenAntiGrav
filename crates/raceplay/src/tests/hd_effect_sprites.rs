//! HD's particle sprites are `.gtf` entries under `/data/psys/tex/`, read for
//! the effects its `Title` lists and for no others - and the emitter extent
//! law, which Pulse on the PSP applies to every effect, is lifted for HD per
//! effect the same way.

use oag_title::Title;

use crate::assets::particle_effect;

#[test]
fn only_hd_lists_effects_whose_gtf_sprites_play() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(title.effects.sprites.is_empty(), "{}", title.name);
    }
    assert_eq!(
        oag_hd::TITLE.effects.sprites,
        [
            oag_title::engine_effects::TRACK_BLAST_EFFECT,
            oag_title::engine_effects::QUAKE_EFFECT,
            oag_title::engine_effects::MISSILE_FLARE_EFFECT,
        ]
    );
    let names = oag_hd::TITLE.effects.names();
    for listed in oag_hd::TITLE.effects.sprites {
        assert!(names.contains(listed), "{listed} is loaded by the race");
    }
}

/// Only HD and Pure lift the extent law, and only for the Quake's wave, the
/// one effect a matched capture of the original has shown spreading across the
/// road on each (Pure's `Quake_Update`, `psp-pure-usa/quake.md`).
#[test]
fn only_hd_and_pure_lift_the_extent_law_and_only_for_the_quake() {
    for title in [oag_pulse::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert!(title.effects.extents.is_empty(), "{}", title.name);
    }
    for title in [oag_hd::TITLE, oag_pure::TITLE] {
        assert_eq!(
            title.effects.extents,
            [oag_title::engine_effects::QUAKE_EFFECT],
            "{}",
            title.name
        );
        let names = title.effects.names();
        for listed in title.effects.extents {
            assert!(names.contains(listed), "{listed} is loaded by the race");
        }
    }
}

fn track_blast(listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::TRACK_BLAST_EFFECT,
        false,
        false,
        listed,
        false,
    )
    .expect("the effect loads");
    Some(effect)
}

/// All nine emitters of HD's own `WO_ROCKET_EXPLO_TRACK` name a `.gtf` that
/// decodes, and the same effect not listed reads none of them.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_rocket_wall_burst_reads_nine_sprites_when_listed_and_none_when_not() {
    let Some(listed) = track_blast(true) else {
        return;
    };
    assert_eq!(listed.emitters.len(), 9);
    assert!(listed.emitters.iter().all(|e| e.sprite.is_some()));
    let unlisted = track_blast(false).expect("same image");
    assert!(unlisted.emitters.iter().all(|e| e.sprite.is_none()));
}

fn quake(extent_law: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::QUAKE_EFFECT,
        false,
        extent_law,
        true,
        false,
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD's own `WO_QUAKE` has five emitters, all five sprites decode, and its line
/// emitters spread across the road only when the title lists the effect: the
/// Quake's driver (`Race::advance_quake_visual`) keys its extent feed, frame and
/// severity on `has_extents`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_quake_wave_spreads_by_the_extent_law_when_listed_and_stays_a_point_when_not() {
    let Some(listed) = quake(true) else {
        return;
    };
    assert_eq!(listed.emitters.len(), 5, "WO_QUAKE's five emitters");
    assert!(listed.emitters.iter().all(|e| e.sprite.is_some()));
    assert!(listed.has_extents());
    let unlisted = quake(false).expect("same image");
    assert!(!unlisted.has_extents());
}

fn missile_head(listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::MISSILE_FLARE_EFFECT,
        false,
        false,
        listed,
        false,
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD's flying Missile plays `WO_MISSILE_HEAD`, whose two head emitters name
/// `vandergraf_balls_1024x512_missile`, an 8 x 4 atlas of red filament rings.
/// Drawn without it every particle is the procedural soft disc, which the
/// emitter's white-hot colour ramp blows out to a white blob; listed, the
/// rings are what a player sees (`docs/ghidra/functions/ps3-hdfury-eu/weapons.md`,
/// "The Missile's red rings").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_missile_head_draws_its_ring_atlas_when_listed_and_a_white_disc_when_not() {
    let Some(listed) = missile_head(true) else {
        return;
    };
    let rings = |effect: &oag_fx::psys::Effect| {
        effect
            .emitters
            .iter()
            .filter(|e| e.name == "WO_MISSILE_HEAD" || e.name == "MISSILE_HEAD_TRANSFORMED")
            .filter(|e| e.sprite.is_some())
            .count()
    };
    assert_eq!(rings(&listed), 2, "both head emitters read the atlas");
    let unlisted = missile_head(false).expect("same image");
    assert_eq!(rings(&unlisted), 0);
}

fn trail_hit_burst(far_bar: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::TRAIL_HITSHIP_EFFECT,
        false,
        false,
        false,
        far_bar,
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD, 2048 and Omega list the trail-hit burst for HD's class-6 bar draw; it is a per-effect
/// choice, not a title-wide one.
#[test]
fn hd_2048_and_omega_list_the_trail_hit_burst_for_the_far_bar_draw() {
    use oag_title::engine_effects::{TRAIL_HITSHIP_EFFECT, TRAIL_HITSHIP_RED_EFFECT};
    for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.effects.far_bars,
            [TRAIL_HITSHIP_EFFECT, TRAIL_HITSHIP_RED_EFFECT],
            "{}",
            title.name
        );
    }
}

/// `WO_TRAIL_HITSHIP`'s two emitters are class 6 (`+0x4c8` aspect 1.0 and 2.0). Listed, both
/// load as HD's bar with their own aspect; unlisted, both stay procedural.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_trail_hit_burst_loads_as_far_bars_when_listed_and_procedural_when_not() {
    use oag_fx::psys::streak::StreakDraw;
    let Some(listed) = trail_hit_burst(true) else {
        return;
    };
    let draws: Vec<StreakDraw> = listed.emitters.iter().map(|e| e.streak).collect();
    assert_eq!(
        draws,
        [
            StreakDraw::FarBar { aspect: 1.0 },
            StreakDraw::FarBar { aspect: 2.0 }
        ]
    );
    let scales: Vec<f32> = listed.emitters.iter().map(|e| e.colour_scale).collect();
    assert_eq!(scales, [0.7, 1.0], "`+0xc84` of the two class-6 emitters");
    let unlisted = trail_hit_burst(false).expect("same image");
    assert!(
        unlisted
            .emitters
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural)
    );
}
