//! Every PSP emitter's roll and aspect, read off the disc. `#[ignore]`d: it
//! needs game content (`just test-data`).

use oag_assets::Archive;
use oag_pob::{self as pob, ChannelMode, ParticleSystem};

fn emitters() -> Vec<(String, pob::Emitter)> {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return Vec::new();
    };
    let spec = format!("{}:PSP_GAME/USRDIR/Data.wad", image.display());
    let mut archive = Archive::open(&spec).expect("open archive");
    let mut out = Vec::new();
    for index in 0..archive.directory().entries.len() {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index).expect("read blob");
        let system = ParticleSystem::parse(&blob).expect("parse");
        for emitter in system.emitters(&blob).expect("emitters") {
            out.push((system.name.clone(), emitter));
        }
    }
    out
}

/// Class 3, the draw class the batched sprite routine `ParticleSystem_DrawRolledQuads` rotates.
fn class_3() -> Vec<(String, pob::Emitter)> {
    emitters()
        .into_iter()
        .filter(|(_, emitter)| emitter.draw_class() == Some(3))
        .collect()
}

fn authors_a_roll(emitter: &pob::Emitter) -> bool {
    emitter.rotation_speed.lo != 0.0 || emitter.rotation_speed.hi != 0.0
}

/// The count the lane was drawn on: 45 of the 76 PSP emitters are class 3 and
/// 23 of those author a roll. Mode by mode, those 23 are 4 keyframed, 14
/// random and 5 constant; a change to the reader that loses the roll channel
/// or the draw class fails here.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn forty_five_psp_emitters_are_class_3_and_twenty_three_author_a_roll() {
    let all = emitters();
    if all.is_empty() {
        return;
    }
    assert_eq!(all.len(), 76);
    let class_3 = class_3();
    assert_eq!(class_3.len(), 45);
    let rolling: Vec<_> = class_3.iter().filter(|(_, e)| authors_a_roll(e)).collect();
    assert_eq!(rolling.len(), 23);
    let count = |mode: ChannelMode| {
        rolling
            .iter()
            .filter(|(_, e)| e.rotation_speed.mode == mode)
            .count()
    };
    assert_eq!(
        (
            count(ChannelMode::Keyframed),
            count(ChannelMode::Random),
            count(ChannelMode::Constant)
        ),
        (4, 14, 5)
    );
    // `WO_SHIP_COLL_SPARK_DAMAGE`'s smoke root: a random rate `0..0.105`
    // rad per tick, a random start and a coin - flags `0x4` and `0x8`.
    let (_, smoke) = class_3
        .iter()
        .find(|(system, e)| system == "WO_SHIP_COLL_SPARK_DAMAGE" && e.name == *system)
        .expect("the damage effect's smoke root");
    assert_eq!(smoke.rotation_speed.mode, ChannelMode::Random);
    assert!((smoke.rotation_speed.hi - 0.10472).abs() < 1e-5);
    assert_eq!(smoke.flags & 0xc, 0xc);
}

/// `+0x4c8` is `ParticleSystem_DrawRolledQuads`'s width-over-height, read off every emitter:
/// unity on all but the two Shuriken emitters among the class 3s, which are
/// drawn four times as wide as they are tall and author no roll. (The streak
/// classes 6 and 7 author `2`, `3` and `0.05` in the same word; their draw is
/// not this routine.)
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn only_the_shurikens_class_3_emitters_are_not_square() {
    let all = emitters();
    if all.is_empty() {
        return;
    }
    assert!(
        all.iter()
            .all(|(_, e)| e.aspect.is_finite() && e.aspect > 0.0),
        "an unreadable aspect"
    );
    let class_3 = class_3();
    let stretched: Vec<(&str, f32)> = class_3
        .iter()
        .filter(|(_, e)| e.aspect != 1.0)
        .map(|(system, e)| (system.as_str(), e.aspect))
        .collect();
    assert_eq!(
        stretched,
        [("WO_SHURIKEN_HEAD", 4.0), ("WO_SHURIKEN_TRAIL", 4.0)]
    );
    assert!(
        class_3
            .iter()
            .filter(|(_, e)| e.aspect != 1.0)
            .all(|(_, e)| !authors_a_roll(e))
    );
}

/// `ParticleSystem_DrawEmitterPool` draws no pool particle of an instance
/// whose atlas grid is past 16 cells. The port draws whatever it is given, so
/// this pins that no PSP emitter is one the original would not draw.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn no_psp_emitter_has_an_atlas_grid_past_sixteen_cells() {
    let too_big: Vec<(String, String, (u16, u16))> = emitters()
        .into_iter()
        .filter(|(_, e)| u32::from(e.atlas_grid.0) * u32::from(e.atlas_grid.1) > 16)
        .map(|(system, e)| (system, e.name, e.atlas_grid))
        .collect();
    assert!(too_big.is_empty(), "{too_big:?}");
}
