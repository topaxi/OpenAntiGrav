//! HD's particle sprites are `.gtf` entries under `/data/psys/tex/`, read for
//! the effects its `Title` lists and for no others.

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
        [oag_title::engine_effects::TRACK_BLAST_EFFECT]
    );
    let names = oag_hd::TITLE.effects.names();
    for listed in oag_hd::TITLE.effects.sprites {
        assert!(names.contains(listed), "{listed} is loaded by the race");
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
