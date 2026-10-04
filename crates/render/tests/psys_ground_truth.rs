//! Plays real `.pob` effects off the user's own disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Until 2026-08-12 `oag_render::sparks` carried the collision effect's four
//! emitters as a hand-transcribed `const`. It is now parsed
//! ([`oag_render::psys::Effect`]), and the transcription is gone - so the
//! values it held survive here instead, as the regression guard they always
//! were:
//!
//! - **`the_collision_spark_effect_matches_the_values_it_replaced`** pins
//!   every parameter of all four emitters against what a maintainer read by
//!   hand out of the same file and separately confirmed byte-exact in a live
//!   PPSSPP session. It is a weak signal about *faithfulness* - the same
//!   bytes read twice - and a strong one about the translation from parsed
//!   record to render spec not drifting.
//! - **`a_collision_burst_fills_the_pool_and_both_blend_classes`** is the
//!   check the headless race tests can no longer make: with a real asset, a
//!   burst actually spawns particles and reaches both GPU pipelines. Its
//!   absence is what the "invisible smoke" bug looked like.
//! - **`every_effect_on_the_disc_plays_without_panicking`** runs all 35 -
//!   including the two the rocket work needs, whose trees nest per-particle
//!   children - for enough ticks to exhaust them, which is the only way the
//!   emitter-slot and particle-pool bounds get exercised at all.

use std::path::{Path, PathBuf};

use oag_assets::Archive;
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_render::psys::spawn::Spawn;
use oag_render::psys::streak::StreakDraw;
use oag_render::psys::{
    Blend, ColourMode, ColourScale, Direction, Effect, Render, System, TICK_HZ,
};
use oag_render::sparks;
use oag_vex::pob;

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const SYSTEMS: usize = 35;

/// Wipeout HD/Fury's `.pob` entries, and the four PSARCs that hold them.
///
/// Named rather than walked because this crate does not depend on `oag-disc`;
/// the four are what `python3 scripts/psarc.py list` reports, and the count
/// below is what makes a fifth one appearing a failure rather than a silent
/// gap.
const HD_ARCHIVES: [&str; 4] = ["DATA00", "DATA02", "DATA03", "DATA06"];

/// `.pob` entries across [`HD_ARCHIVES`], duplicates included - four names
/// appear in two archives each.
const HD_SYSTEMS: usize = 88;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn archive() -> Option<Archive> {
    let image = image()?;
    Some(
        Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
            .expect("open Data.wad"),
    )
}

fn effect(archive: &mut Archive, name: &str) -> Effect {
    let blob = archive
        .read_name(&sparks::effect_path(name))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    Effect::parse(&blob, ColourScale::Full).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-4 * b.abs().max(1.0)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_collision_spark_effect_matches_the_values_it_replaced() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, sparks::DAMAGE_EFFECT);

    let names: Vec<&str> = effect
        .emitters
        .iter()
        .filter(|e| !e.template)
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "WO_SHIP_COLL_SPARK_DAMAGE",
            "WO_SHIP_COLL_SPARK",
            "bits",
            "WO_SHIP_COLL_SPARK_TRAIL"
        ]
    );
    // All four are peers, so all four start with the burst - and so do the
    // two sprite templates the sparks and the ember carry, appended after.
    assert_eq!(effect.roots(), [0, 1, 2, 3, 4, 5]);

    let smoke = &effect.emitters[0];
    assert_eq!(smoke.duration_ticks, 32.0);
    assert_eq!(smoke.interval_ticks, (4, 4));
    assert_eq!(smoke.per_emission, (1, 1));
    assert_eq!(smoke.lifetime_ticks, (16.0, 0.0));
    assert!(close(smoke.speed_per_tick.0, 0.048) && smoke.speed_per_tick.1 == 0.0);
    assert_eq!(smoke.direction, Direction::Radial { hemisphere: false });
    assert!(close(smoke.drag_per_tick.x, 0.98));
    // Authored `-0.011232`, but the emitter leaves the `0x200` flag clear.
    assert_eq!(smoke.gravity_per_tick2, 0.0);
    assert_eq!(smoke.render, Render::Billboard);
    assert_eq!(smoke.blend, Blend::AlphaOver);
    assert_eq!(smoke.colour_mode, ColourMode::RandomEntry);
    // Grows 0.5 -> 2.5 over its life, peak alpha 200/255 held to 21.5%.
    assert!(close(smoke.size.scaled_at(0.0), 0.5));
    assert!(close(smoke.size.scaled_at(1.0), 2.5));
    assert!(close(smoke.alpha.scaled_at(0.0), 200.0));
    assert!(close(smoke.alpha.scaled_at(0.2153), 200.0));
    assert!(close(smoke.alpha.scaled_at(1.0), 0.0));

    let bright = &effect.emitters[1];
    assert_eq!(bright.duration_ticks, 5.0);
    assert_eq!(bright.per_emission, (3, 3));
    assert_eq!(bright.lifetime_ticks, (6.0, 3.0));
    assert!(close(bright.speed_per_tick.0, 1.56) && close(bright.speed_per_tick.1, 0.936));
    assert_eq!(bright.direction, Direction::Radial { hemisphere: true });
    assert!(close(bright.drag_per_tick.x, 0.85));
    // The one emitter whose streak stays anchored at the spawn point - what
    // makes the burst read as rays radiating from the impact.
    assert_eq!(bright.render, Render::Streak { from_spawn: true });
    assert_eq!(bright.blend, Blend::Additive);
    // A random size in 0..=0.312, held for the particle's whole life.
    assert_eq!(bright.size.mode, pob::ChannelMode::Random);
    assert_eq!((bright.size.lo, bright.size.hi), (0.0, 0.312));

    let bits = &effect.emitters[2];
    assert_eq!(bits.duration_ticks, 4.0);
    assert_eq!(bits.per_emission, (2, 2));
    assert_eq!(bits.lifetime_ticks, (16.0, 6.0));
    match bits.direction {
        Direction::Aimed {
            elevation, jitter, ..
        } => {
            assert!(close(elevation, 0.628_319));
            assert!(close(jitter, 30.0_f32.to_radians()));
        }
        other => panic!("bits is aimed, not {other:?}"),
    }
    // The only emitter of the four with the gravity flag set - its white
    // debris arcs and falls.
    assert!(close(bits.gravity_per_tick2, -0.015));
    assert_eq!(bits.drag_per_tick, Vec3::ONE);
    assert_eq!(bits.render, Render::Streak { from_spawn: false });
    assert_eq!(bits.colour_mode, ColourMode::OverLife);
    assert!(close(bits.size.scaled_at(0.0), 0.6));
    assert!(bits.size.scaled_at(1.0) < 0.01);

    let embers = &effect.emitters[3];
    assert_eq!(embers.duration_ticks, 32.0);
    assert_eq!(embers.per_emission, (1, 1));
    assert_eq!(embers.lifetime_ticks, (20.0, 10.0));
    assert!(embers.speed_per_tick.0 == 0.0 && close(embers.speed_per_tick.1, 0.3));
    match embers.direction {
        Direction::Aimed {
            elevation, jitter, ..
        } => {
            assert_eq!(elevation, 0.0);
            assert!(close(jitter, 21.82_f32.to_radians()));
        }
        other => panic!("the embers are aimed, not {other:?}"),
    }
    assert!(close(embers.drag_per_tick.x, 0.95));
    assert_eq!(embers.render, Render::Streak { from_spawn: false });
    assert_eq!(embers.size.mode, pob::ChannelMode::Random);
    assert_eq!((embers.size.lo, embers.size.hi), (0.05, 0.2));
    // Six keyframes are authored on a mode-3 channel and the interpreter
    // never reads them - see `docs/formats/pob.md`.
    assert_eq!(embers.size.keys.len(), 6);
}

/// The collision spark's own 256-entry colour tables, against the
/// two-endpoint approximation `oag_render::sparks` carried until
/// 2026-08-12 and `docs/formats/pob.md` kept describing after that date.
///
/// # Why this exists
///
/// [`Effect::parse`] has read every entry of every emitter's real table off
/// the disc since [`the_collision_spark_effect_matches_the_values_it_replaced`]
/// landed - `crates/vex/src/pob.rs` decodes `+0xc4`'s 256 `u32`s, and
/// `EmitterSpec::from_record` normalises all 256 by [`ColourScale`], not
/// just two of them. What was missing was a test that looked past entry `0`
/// and entry `255` to say so: the smoke and bright-spark endpoints below are
/// exactly the ones the previous `Colour::Gradient` constant held, so a test
/// that pinned only those two would pass whether or not the other 254 ever
/// loaded.
///
/// # What the two endpoints alone get wrong
///
/// Both failures are visible, not rounding: they change what a player's eye
/// actually samples.
///
/// - **The smoke ramp is not linear.** [`sparks::DAMAGE_EFFECT`]'s first
///   emitter is [`ColourMode::RandomEntry`] - one of the 256 entries, drawn
///   uniformly by *index* at spawn (`rng.next_u32() & 0xff`, see
///   `crate::psys::System::spawn` in `oag_render`). A caller reasoning from
///   two endpoints alone would assume that uniform index draw also puts a
///   particle's *colour* somewhere uniform between orange and ember grey.
///   It does not: the real table drops most of the way to grey in its first
///   quarter and spends the remaining three quarters barely dimming further
///   (`smoke_ramp_is_not_linear_between_its_endpoints`), so three particles
///   in four spawn some shade of grey ember and only one in four spawns
///   anywhere near the bright orange the two endpoints alone would suggest
///   is half the population.
/// - **The bright spark's hue is not monotonic.** [`sparks::DAMAGE_EFFECT`]'s
///   second emitter runs `(255,194,29)` at entry `0` to `(255,123,0)` at
///   entry `255` - both exactly what the retired `Colour::Gradient` held -
///   but the green channel does not fall monotonically between them: it
///   *rises* to `212` (brighter, more yellow, than either endpoint) around
///   entry `107` before falling back through orange to the darker endpoint
///   (`bright_spark_hue_is_not_monotonic_between_its_endpoints`). A
///   two-point lerp can only ever dim toward the second colour; the real
///   burst brightens toward yellow for roughly its first two-fifths first.
///
/// `bits`, the third emitter, is the control: its table really is one
/// constant white RGBA(1,1,1,1) end to end, matching the retired
/// `Colour::Constant([1.0; 3])` exactly - so this test is not "the old
/// approximation was always wrong", only that two of its four colour
/// sources hid a real curve behind two correctly-read endpoints.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_collision_spark_palette_is_read_from_disc_not_two_endpoints() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, sparks::DAMAGE_EFFECT);

    let smoke = &effect.emitters[0].palette;
    let bright = &effect.emitters[1].palette;
    let bits = &effect.emitters[2].palette;

    // The endpoints themselves: exactly what `Colour::Gradient` held before
    // 2026-08-12 (`git show 4ece5e9c:crates/render/src/sparks.rs`), so the
    // two-point approximation was not a misread - only an undersampling.
    assert!(close(smoke[0][0], 181.0 / 255.0));
    assert!(close(smoke[0][1], 134.0 / 255.0));
    assert!(close(smoke[0][2], 87.0 / 255.0));
    assert!(close(smoke[255][0], 48.0 / 255.0));
    assert!(close(smoke[255][1], 46.0 / 255.0));
    assert!(close(smoke[255][2], 46.0 / 255.0));

    assert!(close(bright[0][0], 1.0));
    assert!(close(bright[0][1], 194.0 / 255.0));
    assert!(close(bright[0][2], 29.0 / 255.0));
    assert!(close(bright[255][0], 1.0));
    assert!(close(bright[255][1], 123.0 / 255.0));
    assert!(close(bright[255][2], 0.0));

    // The control: really constant, not merely close to it.
    for entry in bits.iter() {
        assert_eq!(*entry, [1.0, 1.0, 1.0, 1.0]);
    }
}

/// See [`the_collision_spark_palette_is_read_from_disc_not_two_endpoints`]'s
/// doc comment for why this is a visible difference, not a rounding one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn smoke_ramp_is_not_linear_between_its_endpoints() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, sparks::DAMAGE_EFFECT);
    let smoke = &effect.emitters[0].palette;

    let (r0, r255) = (smoke[0][0], smoke[255][0]);
    let linear_at = |index: usize| r0 + (r255 - r0) * (index as f32 / 255.0);

    // A quarter of the way by index - a quarter of the RandomEntry
    // population - the real table already reads as almost pure grey, while
    // a straight line from the two endpoints is still two-thirds of the way
    // to orange. Real 0.349 against linear's 0.579: an 0.23 gap on a 0..=1
    // channel is a different colour, not a rounding error.
    let real = smoke[64][0];
    let linear = linear_at(64);
    assert!(
        real < 0.40,
        "real r at 1/4 should already read as grey: {real}"
    );
    assert!(
        linear - real > 0.15,
        "the linear model should overshoot by a wide margin here: real {real}, linear {linear}"
    );
}

/// See [`the_collision_spark_palette_is_read_from_disc_not_two_endpoints`]'s
/// doc comment for why this is a visible difference, not a rounding one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn bright_spark_hue_is_not_monotonic_between_its_endpoints() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, sparks::DAMAGE_EFFECT);
    let bright = &effect.emitters[1].palette;

    let (g0, g255) = (bright[0][1], bright[255][1]);
    // Both endpoints are already the brightest and darkest a monotonic ramp
    // could produce; a real interior peak brighter than both is proof the
    // curve is not one.
    let peak = bright.iter().map(|entry| entry[1]).fold(f32::MIN, f32::max);
    assert!(
        peak > g0.max(g255) + 0.03,
        "expected an interior peak clearly above both endpoints: peak {peak}, endpoints {g0}/{g255}"
    );
    // The peak sits inside the table, not at either end - roughly two
    // fifths of the way through the RandomEntry population.
    let peak_index = bright
        .iter()
        .position(|entry| entry[1] == peak)
        .expect("the max we just computed came from this slice");
    assert!(
        (40..170).contains(&peak_index),
        "expected the hue peak in the table's interior, found it at {peak_index}"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_collision_burst_fills_the_pool_and_both_blend_classes() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, sparks::DAMAGE_EFFECT);
    let dt = 1.0 / TICK_HZ;
    let mut rng = Rng::new(1);
    let mut system = System::new();

    system.ignite(&effect, Vec3::ZERO, sparks::severity(80.0));
    // The first tick is every emitter's first emission: 1 smoke + 3 sparks
    // + 2 bits + 1 ember, and the two sprite templates the sparks and the
    // ember start with (`shazam`, `glow` - see `psys::template`).
    system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
    assert_eq!(system.alive_count(), 9);

    let mut peak = 0;
    for _ in 0..4 {
        system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
        peak = peak.max(system.alive_count());
    }
    let (additive, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
    assert!(!additive.is_empty(), "no additive geometry");
    assert!(
        !alpha_over.is_empty(),
        "no alpha-over geometry - the smoke is invisible again"
    );

    // Severity multiplies speed and size, never the count.
    let mut gentle = System::new();
    let mut hard = System::new();
    let (mut r1, mut r2) = (Rng::new(7), Rng::new(7));
    gentle.ignite(&effect, Vec3::ZERO, sparks::severity(0.0));
    hard.ignite(&effect, Vec3::ZERO, sparks::severity(1e6));
    gentle.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut r1);
    hard.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut r2);
    assert_eq!(gentle.alive_count(), hard.alive_count());

    // And the whole burst drains: 32 emitting ticks plus the longest
    // lifetime, with margin.
    for _ in 0..120 {
        system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    assert!(!system.is_running(), "the burst never ended");
    assert!(peak > 9, "the pool never grew past the first emission");
}

/// Every effect on the disc, run until it drains.
///
/// The two the rocket work needs, `WO_ROCKET_EXPLO` and
/// `WO_ROCKET_EXPLO_TRACK`, are the reason this exists: their trees spawn a
/// child system per *particle*, which is the only thing that presses the
/// emitter-slot pool.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_effect_on_the_disc_plays_without_panicking() {
    let Some(mut archive) = archive() else {
        return;
    };
    let count = archive.directory().entries.len();
    let dt = 1.0 / TICK_HZ;
    let mut played = 0;
    let mut busiest = (0usize, String::new());

    for index in 0..count {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index).expect("read");
        let effect = Effect::parse(&blob, ColourScale::Full)
            .unwrap_or_else(|error| panic!("entry {index}: {error}"));

        let mut rng = Rng::new(index as u64 + 1);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        for tick in 0..240 {
            // A moving anchor, so the root emitters exercise the
            // follow-the-caller path rather than sitting still.
            let anchor = Vec3::new(tick as f32 * 0.1, 0.0, 0.0);
            system.advance(&effect, dt, anchor, Vec3::Y, &mut rng);
            if system.alive_count() > busiest.0 {
                busiest = (system.alive_count(), effect.name.clone());
            }
            let (additive, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
            assert!((additive.len() + alpha_over.len()) % 6 == 0);
        }
        played += 1;
    }

    assert_eq!(played, SYSTEMS);
    println!("busiest: {} with {} particles", busiest.1, busiest.0);
}

/// The same translation on Wipeout HD/Fury's effects, which are big-endian.
///
/// [`oag_vex::pob`] reading them is asserted on the disc in
/// `crates/assets/tests/pob_ground_truth.rs`; this is the layer above - that a
/// record read the other way round still becomes an [`EmitterSpec`] this
/// renderer can play, with no field landing outside a range the spec builder
/// accepts.
///
/// **One effect is expected to be refused**, and that is the assertion rather
/// than an exception: `WO_NITRO_SHIP_DEATH` authors `blend_class` 4, which no
/// Pulse file does and nothing has traced. Refusing it by name is the
/// do-not-invent rule holding - an effect with an undecoded blend mode draws
/// nothing and says so, instead of being drawn as one of the three that are
/// decoded.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_effect_translates_or_is_refused_by_name() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }

    let dt = 1.0 / TICK_HZ;
    let (mut seen, mut played) = (0usize, 0usize);
    let mut refused = Vec::new();

    for name in HD_ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{name}.PSARC", path.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|entry| entry.to_ascii_lowercase().ends_with(".pob"))
            .cloned()
            .collect();

        for entry in entries {
            let blob = archive.read_path(&entry).expect("the entry reads");
            seen += 1;
            assert!(
                pob::looks_like_particle_system(&blob),
                "{entry}: not a particle system"
            );
            // HD's palettes run to 255, as the PSP's do and unlike the PS2's.
            let effect = match Effect::parse(&blob, ColourScale::Full) {
                Ok(effect) => effect,
                Err(error) => {
                    refused.push(format!("{entry}: {error}"));
                    continue;
                }
            };

            let mut rng = Rng::new(seen as u64);
            let mut system = System::new();
            system.ignite(&effect, Vec3::ZERO, 1.0);
            for tick in 0..240 {
                let anchor = Vec3::new(tick as f32 * 0.1, 0.0, 0.0);
                system.advance(&effect, dt, anchor, Vec3::Y, &mut rng);
                let (additive, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
                assert!((additive.len() + alpha_over.len()) % 6 == 0);
            }
            played += 1;
        }
    }

    assert_eq!(seen, HD_SYSTEMS, "entries found across {HD_ARCHIVES:?}");
    assert_eq!(refused.len(), 1, "refusals: {refused:?}");
    assert!(
        refused[0].contains("wo_nitro_ship_death") && refused[0].contains("blend class 4"),
        "the one refusal is not the one this test knows about: {}",
        refused[0]
    );
    assert_eq!(played, HD_SYSTEMS - 1);
}

/// Every PSP effect's billboard sprites fit one [`oag_render::psys::Library`]
/// sheet together, and the sprite the Quake's fire draws with is the
/// orange the whole 2026-09-24 fix rests on.
///
/// `fireballs`' colour table starts near white (`(255, 250, 252)`) and
/// only turns orange over the second half of a particle's life; under
/// `GU_TFX_MODULATE` the sprite is what carries the hue. If this sprite
/// ever decoded grey or white again - a wrong offset, a wrong palette - the
/// crest would go back to the white blowout it was.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_sprite_fits_one_sheet_and_the_quake_fire_is_orange() {
    let Some(mut archive) = archive() else {
        return;
    };
    let count = archive.directory().entries.len();
    let mut library = oag_render::psys::Library::new();
    for index in 0..count {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index).expect("read");
        let effect = Effect::parse(&blob, ColourScale::Full).expect("parse");
        let name = effect.name.clone();
        library.insert(&name, effect);
    }
    assert_eq!(library.len(), SYSTEMS);

    let (mut sampled, mut unplaced) = (0, Vec::new());
    for name in library.names() {
        let effect = library.get(name).expect("loaded");
        for spec in &effect.emitters {
            match (spec.render, &spec.sprite, spec.sheet_rect) {
                (Render::Billboard, Some(_), Some(_)) => sampled += 1,
                (Render::Billboard, Some(_), None) => unplaced.push(spec.name.clone()),
                _ => {}
            }
        }
    }
    assert!(
        unplaced.is_empty(),
        "sprites that did not fit: {unplaced:?}"
    );
    println!(
        "psp: {sampled} billboard emitter(s) sampled, {} distinct sprite(s) on the sheet",
        library.sheet().len()
    );

    let quake = library.get("WO_QUAKE").expect("WO_QUAKE");
    let fire = quake
        .emitters
        .iter()
        .find(|spec| spec.name == "fireballs")
        .expect("fireballs");
    let sprite = fire.sprite.as_ref().expect("the fire has a sprite");
    let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
    for texel in sprite.rgba.chunks(4) {
        r += u64::from(texel[0]);
        g += u64::from(texel[1]);
        b += u64::from(texel[2]);
    }
    assert!(
        r > g && g > 2 * b,
        "fireballs' sprite should be orange, summed rgb ({r}, {g}, {b})"
    );
}

/// Shape 3 places a particle on a ring or in a disc round the anchor, not at it
/// (`FUN_088fc634`, read 2026-10-01), and the Bomb's smoke ring **widens while it emits**:
/// its one animated-attribute record scales the extent from `1.007` to `2` over the
/// emitter's 20 ticks. Read live on a PPSSPP detonation (`psp-weapon-pair.py --probe
/// rolled --detonate-bomb-at`, a single run), the particles born at emitter ticks 0, 1, 3, 7
/// and 16 sat 13.1, 13.7, 15.0, 17.6 and 23.4 units from the blast centre on the
/// horizontal plane. Placed at the anchor, as every shape 3 was until then, the smoke was
/// a puff where the original's is a wall.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_bombs_smoke_ring_is_born_on_a_ring_that_widens_as_it_emits() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, "WO_BOMB_SMOKERING");
    let mut rng = Rng::new(11);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    let dt = 1.0 / TICK_HZ;
    // Radii of the particles born this tick - the ones still at the size channel's start.
    let mut born = Vec::new();
    for _ in 0..17 {
        system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
        let (_, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
        let mut radii = Vec::new();
        for quad in alpha_over.chunks(6) {
            let xs = quad.iter().map(|v| v.position[0]);
            let half = (xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)) / 2.0;
            let centre = quad
                .iter()
                .fold(Vec3::ZERO, |sum, v| sum + Vec3::from(v.position))
                / 6.0;
            // The `debris` emitter beside it has no extent: its quads sit at the anchor.
            if half < 0.1 && centre.x.hypot(centre.z) > 4.0 {
                radii.push(centre.x.hypot(centre.z));
            }
        }
        born.push(radii);
    }
    for (tick, expected) in [
        (0usize, 13.1f32),
        (1, 13.7),
        (3, 15.0),
        (7, 17.6),
        (16, 23.4),
    ] {
        assert!(!born[tick].is_empty(), "no particle born at tick {tick}");
        for &radius in &born[tick] {
            assert!(
                (radius - expected).abs() < 0.6,
                "tick {tick}: born {radius} units out, the original's {expected}"
            );
        }
    }
}

/// `WO_REPULSER_BLAST`'s one burst of fifty, read 2026-10-04 (`particle-system.md`, "The
/// emitter's clock and the burst laws"): flag `0x200000` steps the ring `7.2` degrees a
/// bead from one random start, the selector-5 record makes their life `114 * 1.5` ticks,
/// and the emitter's rate `4` spends four of them a frame - so the ring holds for 43
/// frames where the authored life alone, at rate 1, would hold for 114. The PSP's beaded
/// ring is drawn at updates 10 to 40 and gone by 47
/// (`docs/ghidra/functions/psp-pulse-usa/repulser.md`).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_repulser_blast_is_an_even_ring_that_lives_forty_three_frames() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, "WO_REPULSER_BLAST");
    let root = &effect.emitters[0];
    assert_eq!(root.playback.rate, 4.0);
    assert!(root.playback.even_ring && !root.playback.subframe_spread);
    assert!(root.playback.lifetime_animation.is_some());
    let shockrings = self::effect(&mut archive, "WO_MISSILE_EXPLO");
    assert!(shockrings.emitters.iter().any(|e| e.playback.even_ring));
    let trail = self::effect(&mut archive, "WO_MISSILE_HEAD");
    assert!(trail.emitters.iter().any(|e| e.playback.subframe_spread));
    // The absorb's `glow` template authors rate 1 under a root of 0.8, and ages at 0.8.
    let absorb = self::effect(&mut archive, "WO_WEAPON_ABSORB");
    let glow = absorb
        .emitters
        .iter()
        .find(|e| e.template)
        .expect("a template");
    assert_eq!(glow.playback.rate, absorb.emitters[0].playback.rate);
    assert_eq!(glow.playback.rate, 0.8);

    let mut rng = Rng::new(12);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    let dt = 1.0 / TICK_HZ;
    system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
    let (additive, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
    let mut angles: Vec<f32> = additive
        .chunks(6)
        .chain(alpha_over.chunks(6))
        .map(|quad| {
            let centre = quad
                .iter()
                .fold(Vec3::ZERO, |sum, v| sum + Vec3::from(v.position))
                / 6.0;
            centre.z.atan2(centre.x).rem_euclid(std::f32::consts::TAU)
        })
        .collect();
    assert_eq!(angles.len(), 50);
    angles.sort_by(f32::total_cmp);
    for pair in angles.windows(2) {
        assert!(
            (pair[1] - pair[0] - std::f32::consts::TAU / 50.0).abs() < 1e-3,
            "{angles:?}"
        );
    }
    let mut alive = vec![system.alive_count()];
    for _ in 0..45 {
        system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
        alive.push(system.alive_count());
    }
    assert_eq!(alive[42], 50, "{alive:?}");
    assert_eq!(alive[43], 0, "{alive:?}");
}

/// `WO_REPULSER`'s root, read and measured 2026-10-04 (`particle-system.md`, "Shape 8, the
/// class-6 bar and the wave's width"): shape 8 is the ring over `[0, pi]`, and its class 6
/// draws as the pool bar capped by its aspect `0.05`, not the template wedge.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_repulser_wave_is_a_half_ring_of_bars() {
    let Some(mut archive) = archive() else {
        return;
    };
    let effect = effect(&mut archive, "WO_REPULSER");
    let root = &effect.emitters[0];
    assert_eq!(root.streak, StreakDraw::Bar { aspect: 0.05 });
    assert!(matches!(root.spawn, Spawn::Ring { arc, .. } if arc == std::f32::consts::PI));
    let mut rng = Rng::new(3);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    for _ in 0..30 {
        system.advance(&effect, 1.0 / TICK_HZ, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    let (additive, alpha_over) = system.vertices(&effect, Vec3::X, Vec3::Y);
    let bars: Vec<_> = additive.chunks(6).chain(alpha_over.chunks(6)).collect();
    assert!(bars.len() >= 4, "{}", bars.len());
    // The frame's `+Z` (`X x Y`) half: every bar's centre at `z >= 0`, and the
    // template sprites at the anchor do not count against it.
    for quad in bars {
        let centre = quad
            .iter()
            .fold(Vec3::ZERO, |sum, v| sum + Vec3::from(v.position))
            / 6.0;
        assert!(centre.z > -1.0, "{centre}");
    }
}
