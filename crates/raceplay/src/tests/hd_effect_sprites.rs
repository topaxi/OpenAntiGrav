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
            oag_title::engine_effects::QUAKE_EFFECT
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
