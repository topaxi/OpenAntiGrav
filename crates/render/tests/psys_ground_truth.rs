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
use oag_formats::pob;
use oag_render::psys::{
    Blend, ColourMode, ColourScale, Direction, Effect, Render, System, TICK_HZ,
};
use oag_render::sparks;

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const SYSTEMS: usize = 35;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
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

    let names: Vec<&str> = effect.emitters.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "WO_SHIP_COLL_SPARK_DAMAGE",
            "WO_SHIP_COLL_SPARK",
            "bits",
            "WO_SHIP_COLL_SPARK_TRAIL"
        ]
    );
    // All four are peers, so all four start with the burst.
    assert_eq!(effect.roots(), [0, 1, 2, 3]);

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
    // + 2 bits + 1 ember.
    system.advance(&effect, dt, Vec3::ZERO, &mut rng);
    assert_eq!(system.alive_count(), 7);

    let mut peak = 0;
    for _ in 0..4 {
        system.advance(&effect, dt, Vec3::ZERO, &mut rng);
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
    gentle.advance(&effect, dt, Vec3::ZERO, &mut r1);
    hard.advance(&effect, dt, Vec3::ZERO, &mut r2);
    assert_eq!(gentle.alive_count(), hard.alive_count());

    // And the whole burst drains: 32 emitting ticks plus the longest
    // lifetime, with margin.
    for _ in 0..120 {
        system.advance(&effect, dt, Vec3::ZERO, &mut rng);
    }
    assert!(!system.is_running(), "the burst never ended");
    assert!(peak > 7, "the pool never grew past the first emission");
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
            system.advance(&effect, dt, anchor, &mut rng);
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
