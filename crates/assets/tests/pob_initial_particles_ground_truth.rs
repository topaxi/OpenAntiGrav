//! Reads the sprite templates an emitter starts with (`+0x9a4`/`+0x9a8`) off
//! the discs. `#[ignore]`d: it needs game content (`just test-data`).
//!
//! The record layout is read off Pulse's PSP executable and confirmed live on
//! one file (`WO_SHIP_COLL_SPARK_DAMAGE`); this pins which files carry any at
//! all, so a change to the reader that starts finding templates in a file
//! that has none - or stops finding the two that exist - fails here.

use oag_assets::Archive;
use oag_vex::pob::{self, ChannelMode, ParticleSystem};

/// Every `(system, parent emitter, template)` on `spec`, in directory order.
fn templates(spec: &str) -> Vec<(String, String, pob::Emitter)> {
    let mut archive = Archive::open(spec).expect("open archive");
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
            for template in emitter.initial_particles {
                out.push((system.name.clone(), emitter.name.clone(), template));
            }
        }
    }
    out
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_psp_collision_sparks_carry_a_shazam_and_a_glow() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let found = templates(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()));
    for (system, parent, template) in &found {
        println!(
            "{system} / {parent}: {} life {:?} size {:?} hi {}",
            template.name, template.lifetime_ticks, template.size.mode, template.size.hi
        );
    }

    let damage: Vec<_> = found
        .iter()
        .filter(|(system, ..)| system == "WO_SHIP_COLL_SPARK_DAMAGE")
        .collect();
    assert_eq!(
        damage
            .iter()
            .map(|(_, parent, template)| (parent.as_str(), template.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("WO_SHIP_COLL_SPARK", "shazam"),
            ("WO_SHIP_COLL_SPARK_TRAIL", "glow")
        ],
        "the two templates, on the two emitters the live capture drew them for"
    );

    // The live capture: `shazam` half-size 9.36 at severity 2.4 is 3.9 * 2.4,
    // its life six ticks, additive billboard; `glow` 0.3..2.0 looping every
    // ten ticks for forty.
    let shazam = &damage[0].2;
    assert_eq!(shazam.size.mode, ChannelMode::Keyframed);
    assert!((shazam.size.hi - 3.9).abs() < 1e-6);
    assert_eq!(shazam.lifetime_ticks, (6, 0));
    assert_eq!((shazam.render_mode, shazam.blend_class), (2, 2));
    let glow = &damage[1].2;
    assert!((glow.size.lo - 0.3).abs() < 1e-6 && (glow.size.hi - 2.0).abs() < 1e-6);
    assert!((glow.size.period - 10.0).abs() < 1e-6);
    assert_eq!(glow.lifetime_ticks, (40, 0));
    assert_eq!(glow.size.keys.len(), 6);
    // Every alpha channel is `0..=255`, and the colour table's first entry
    // is what the live particle drew first (`fff5f5ff`, R G B A order).
    assert!((glow.alpha.hi - 255.0).abs() < 1e-6);
    assert_eq!(glow.colours[0], [255, 245, 245, 255]);
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_effects_are_surveyed_for_templates() {
    let Some(image) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    let found = templates(&format!("{}:54748/WADS2.WAD", image.display()));
    for (system, parent, template) in &found {
        println!("{system} / {parent}: {}", template.name);
    }
    println!("ps2: {} template(s)", found.len());
}
