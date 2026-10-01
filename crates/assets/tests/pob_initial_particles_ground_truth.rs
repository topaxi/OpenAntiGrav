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

    // The draw class is 3, a rotating sprite, so the `+0xf0` block is its
    // aspect term and the roll channel its angle. Live on PPSSPP: `shazam`
    // aspect `1.5`, roll `0`; `glow` aspect `1.7`, roll `2 pi` falling ~`0.7`
    // a tick, flags `0x30` (random start, angle not rate).
    let stretch = |t: &pob::Emitter| t.stretch.clone().expect("a template has a stretch block");
    assert_eq!(
        (stretch(shazam).mode, stretch(shazam).hi),
        (ChannelMode::Constant, 0.5)
    );
    assert_eq!(
        (stretch(glow).mode, stretch(glow).hi),
        (ChannelMode::Constant, 0.7)
    );
    assert_eq!(
        (shazam.rotation_speed.lo, shazam.rotation_speed.hi),
        (0.0, 0.0)
    );
    assert!((glow.rotation_speed.hi - std::f32::consts::TAU).abs() < 1e-5);
    assert_eq!(glow.rotation_speed.keys.len(), 9);
    assert_eq!((shazam.flags, glow.flags), (0, 0x30));
}

/// Every one of the 31 templates on the PSP disc draws through class 3, which
/// is what the rotating quad is for.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_template_is_a_rotating_sprite_with_a_stretch_block() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let found = templates(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()));
    assert!(!found.is_empty());
    for (system, parent, template) in &found {
        assert_eq!(
            template.draw_class(),
            Some(3),
            "{system} / {parent} / {}",
            template.name
        );
        assert!(
            template.stretch.is_some(),
            "{system} / {parent} / {}",
            template.name
        );
    }
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

/// The one keyed stretch on the disc. Live on PPSSPP, a Mine's `BANG` drew
/// aspects `2.130, 5.272, 8.468, 11.623, 14.763` at ticks 0..4 of its six
/// (`ParticleSystem_DrawParticle`'s `particle+0x64`), so the aspect is `1 + v`
/// for the channel's `v` - the law a constant `0.5` or `0.7` cannot test - and
/// the quad is up to 49 units wide at that tick's half-size `3.35`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_mine_bangs_keyed_stretch_reads_the_live_aspect() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let found = templates(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()));
    let (_, _, bang) = found
        .iter()
        .find(|(system, _, template)| system == "WO_MINE_EXPLO" && template.name == "BANG")
        .expect("the Mine's BANG");
    let stretch = bang.stretch.as_ref().expect("a stretch block");
    assert_eq!(stretch.mode, ChannelMode::Keyframed);
    assert_eq!(bang.lifetime_ticks, (6, 0));
    for (tick, live) in [2.130, 5.272, 8.468, 11.623, 14.763]
        .into_iter()
        .enumerate()
    {
        let aspect = 1.0 + stretch.scaled_at(tick as f32 / 6.0);
        assert!(
            (aspect - live).abs() < 0.1,
            "tick {tick}: {aspect} vs {live}"
        );
    }
}

/// `ParticleSystem_DrawParticle` binds `*(owner + 8) + 0x890` for a template
/// particle - the template record's own texture block - so each template draws
/// with a sprite of its own. The explosion's `Glow` hangs on `SHIP_DEBRIS`
/// (a 128x64 4 bpp grey atlas) and binds a 32x32 8 bpp radial glow: the GE dump
/// of the running original (`TEXSIZE 0x505`, `CLUT8`, a grey ramp whose palette
/// alpha equals its colour) drew exactly that, and the parent's sprite drew
/// nothing like it. The collision sparks' templates share their parent's pool
/// by authoring, which is what had been read as a rule.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_template_carries_its_own_sprite_and_the_explosions_glow_is_not_its_parents() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let spec = format!("{}:PSP_GAME/USRDIR/Data.wad", image.display());
    let mut archive = Archive::open(&spec).expect("open archive");
    let mut checked = 0;
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
            for template in &emitter.initial_particles {
                let own = system.template_texture(&blob, template).unwrap_or_else(|| {
                    panic!("{} / {}: no texture block", system.name, template.name)
                });
                assert!(own.width >= 32 && own.height >= 32, "{}", template.name);
                checked += 1;
                if system.name == "WO_SHIP_EXPLOSION" {
                    let parent = system
                        .embedded_texture(&blob, &emitter)
                        .expect("the parent's sprite");
                    assert_eq!(
                        (parent.width, parent.height, parent.bits_per_pixel),
                        (128, 64, 4)
                    );
                    assert_eq!(
                        (own.width, own.height, own.bits_per_pixel, own.levels),
                        (32, 32, 8, 3)
                    );
                    let rgba = own.rgba8();
                    let centre = (16 * 32 + 16) * 4;
                    assert_eq!(&rgba[centre..centre + 4], &[255, 255, 255, 255]);
                    assert_eq!(&rgba[..4], &[0, 0, 0, 0], "black at the corner");
                }
            }
        }
    }
    assert_eq!(checked, 28, "the corpus's templates");
}
