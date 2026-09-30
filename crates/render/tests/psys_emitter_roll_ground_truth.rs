//! Plays the emitters' own class 3 particles off the PSP disc. `#[ignore]`d:
//! it needs game content (`just test-data`).
//!
//! The law is `ParticleSystem_DrawRolledQuads`'s and `ParticleSystem_UpdateParticles`'s, read in
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "An emitter's own
//! particles"; see `oag_render::psys::roll`.

use oag_assets::Archive;
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_render::mesh::GpuVertex;
use oag_render::psys::{ColourScale, Effect, System, TICK_HZ};
use oag_vex::pob;

fn effects() -> Vec<(String, Effect)> {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return Vec::new();
    };
    let mut archive = Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
        .expect("open Data.wad");
    let mut out = Vec::new();
    for index in 0..archive.directory().entries.len() {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index).expect("read blob");
        let system = pob::ParticleSystem::parse(&blob).expect("parse");
        let effect = Effect::parse(&blob, ColourScale::Full).expect("effect");
        out.push((system.name, effect));
    }
    out
}

fn effect(name: &str) -> Option<Effect> {
    effects()
        .into_iter()
        .find_map(|(n, effect)| (n == name).then_some(effect))
}

/// The quads an effect draws `ticks` updates in, additive and alpha-over.
fn quads(effect: &Effect, ticks: usize) -> Vec<GpuVertex> {
    let mut rng = Rng::new(1);
    let mut system = System::new();
    system.ignite(effect, Vec3::ZERO, 1.0);
    for _ in 0..ticks {
        system.advance(effect, 1.0 / TICK_HZ, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    let (mut all, over) = system.vertices(effect, Vec3::X, Vec3::Y);
    all.extend(over);
    all
}

/// Every class 3 emitter of the disc rotates and none of the others does; a
/// spec that loses its rotation is a spec drawn as the plain square again.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_class_3_emitter_is_given_the_rotation_and_nothing_else_is() {
    let effects = effects();
    if effects.is_empty() {
        return;
    }
    let (mut rotating, mut others) = (0, 0);
    for (name, effect) in &effects {
        for spec in effect.emitters.iter().filter(|spec| !spec.template) {
            match (spec.rotation.is_some(), spec.render) {
                (true, oag_render::psys::Render::Billboard) => rotating += 1,
                (false, _) => others += 1,
                (true, render) => panic!("{name} / {}: {render:?} rotates", spec.name),
            }
        }
    }
    // 45 class 3 emitters; the billboards that are class 1 or 2 are in the rest.
    assert_eq!(rotating, 45);
    assert_eq!(rotating + others, 76);
}

/// The Shuriken's head is aspect 4, no roll: every quad it draws is four
/// times as wide as it is tall, corner to corner.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_shurikens_head_is_four_times_as_wide_as_it_is_tall() {
    let Some(effect) = effect("WO_SHURIKEN_HEAD") else {
        return;
    };
    let quads = quads(&effect, 3);
    assert!(!quads.is_empty());
    for quad in quads.chunks(6) {
        let span = |axis: usize| {
            let values = quad.iter().map(|v| v.position[axis]);
            values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
        };
        assert!((span(0) / span(1) - 4.0).abs() < 1.0e-3, "{quad:?}");
    }
}

/// The damage effect's smoke root rolls a random rate from a random start:
/// its quads are not axis-aligned, which a dropped rotation would leave them.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_damage_effects_smoke_is_turned() {
    let Some(effect) = effect("WO_SHIP_COLL_SPARK_DAMAGE") else {
        return;
    };
    let turned = |effect: &Effect| {
        quads(effect, 12).chunks(6).any(|quad| {
            let (a, b) = (quad[0].position, quad[1].position);
            (a[1] - b[1]).abs() > 1.0e-3 && (a[0] - b[0]).abs() > 1.0e-3
        })
    };
    assert!(turned(&effect));
    let mut plain = effect.clone();
    plain.without_pulse_psp_draw();
    assert!(
        plain.emitters.iter().all(|spec| spec.rotation.is_none()),
        "another source keeps the rotation"
    );
}

/// `WO_MISSILE_HEAD`'s two templates only: a `redbar` (6000 ticks) and a
/// `glow` (3600, 34 units), with the emitters' own particles switched off.
fn missile_head_templates() -> Option<std::sync::Arc<Effect>> {
    let mut effect = effect("WO_MISSILE_HEAD")?;
    for spec in effect.emitters.iter_mut().filter(|spec| !spec.template) {
        spec.per_emission = (0, 0);
    }
    Some(std::sync::Arc::new(effect))
}

/// The x extent of everything the stage draws.
fn x_extent(stage: &oag_render::psys::Stage) -> f32 {
    let (mut all, over) = stage.vertices(Vec3::X, Vec3::Y);
    all.extend(over);
    let xs = all.iter().map(|v| v.position[0]);
    xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)
}

/// A missile in flight: its `glow` and `redbar` ride it. They were spawned
/// once at the muzzle and left there - a 34-unit glow hanging at the launch
/// point for the minute the template lives - until
/// `ParticleSystem_UpdateParticleFields`'s copy of the instance position was
/// played.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_missiles_templates_ride_it_and_go_when_it_does() {
    let Some(effect) = missile_head_templates() else {
        return;
    };
    let templates = effect.emitters.iter().filter(|spec| spec.template).count();
    assert_eq!(templates, 2, "redbar and glow");
    let run = |end: fn(&mut oag_render::psys::Stage, oag_render::psys::Playing)| {
        let mut stage = oag_render::psys::Stage::new();
        let mut rng = Rng::new(1);
        let playing = stage.attach(&effect, Vec3::ZERO, 1.0).expect("attach");
        for tick in 0..60 {
            stage.follow(playing, Vec3::new(tick as f32 * 10.0, 0.0, 0.0));
            stage.advance(1.0 / TICK_HZ, &mut rng);
        }
        let flying = (x_extent(&stage), stage.alive_count());
        end(&mut stage, playing);
        stage.advance(1.0 / TICK_HZ, &mut rng);
        (flying, stage.alive_count())
    };
    let (flying, released) = run(oag_render::psys::Stage::release);
    assert!(
        flying.0 < 150.0,
        "the templates hang back: drawn {} units wide over 590 of flight",
        flying.0
    );
    assert_eq!(flying.1, 2);
    assert_eq!(released, 0, "a release leaves its templates alive");
    // Not released - an instance that ran out on its own - they finish.
    let (_, detached) = run(oag_render::psys::Stage::detach);
    assert_eq!(detached, 2);
}
