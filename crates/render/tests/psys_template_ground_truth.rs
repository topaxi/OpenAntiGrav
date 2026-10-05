//! Plays the sprite templates of a real struck-craft effect. `#[ignore]`d: it
//! needs game content (`just test-data`).
//!
//! The numbers are the live PPSSPP capture's (2026-09-30), a hit on the player
//! at severity 2.4: `shazam` half-sizes `9.36, 9.36, 8.73, 6.54, 4.33` over
//! the frames after it starts, then gone; `glow` flickering between `0.748`
//! and `4.8`. See `oag_pob`'s `initial` module and `psys::template`.

use oag_assets::Archive;
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_render::psys::{ColourScale, Effect, System, TICK_HZ};
use oag_render::sparks;

fn effect() -> Option<Effect> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archive = Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
        .expect("open Data.wad");
    let blob = archive
        .read_name(&sparks::effect_path(sparks::DAMAGE_EFFECT))
        .expect("the damage effect");
    Some(Effect::parse(&blob, ColourScale::Full).expect("parse"))
}

/// `(half-width, half-height)` of every additive quad this tick drew, in
/// world units and the camera's own right and up. A template is a
/// `DrawRotatedSprite` quad: `size` tall and `aspect * size` wide before its
/// roll turns it.
fn extents(system: &System, effect: &Effect) -> Vec<(f32, f32)> {
    let (additive, _) = system.vertices(effect, Vec3::X, Vec3::Y);
    let half = |quad: &[oag_mesh::mesh::GpuVertex], axis: usize| {
        let values = quad.iter().map(|v| v.position[axis]);
        (values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)) * 0.5
    };
    additive
        .chunks(6)
        .map(|quad| (half(quad, 0), half(quad, 1)))
        .collect()
}

/// The half-height of the `shazam` quad: the one drawn exactly 1.5 times as
/// wide as it is tall (its `+0xf0` block's `0.5`, roll zero), which is its size
/// channel's value. The `glow` beside it is turning, so its box is neither.
fn tallest(system: &System, effect: &Effect) -> f32 {
    extents(system, effect)
        .into_iter()
        .filter(|&(w, h)| h > 0.0 && (w / h - 1.5).abs() < 1.0e-3)
        .map(|(_, height)| height)
        .fold(0.0, f32::max)
}

/// A struck locator's effect `ticks` updates in, at severity 2.4.
fn system_at(effect: &Effect, ticks: usize) -> System {
    let mut rng = Rng::new(1);
    let mut system = System::new();
    system.ignite(effect, Vec3::ZERO, 2.4);
    for _ in 0..ticks {
        system.advance(effect, 1.0 / TICK_HZ, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    system
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_struck_locators_shazam_flash_is_3_9_times_severity_and_shrinks() {
    let Some(effect) = effect() else {
        return;
    };
    let templates: Vec<&str> = effect
        .emitters
        .iter()
        .filter(|spec| spec.template)
        .map(|spec| spec.name.as_str())
        .collect();
    assert_eq!(templates, ["shazam", "glow"]);
    assert_eq!(effect.skipped_templates(), 0);

    let dt = 1.0 / TICK_HZ;
    let mut rng = Rng::new(1);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 2.4);
    let mut sizes = Vec::new();
    for _ in 0..8 {
        system.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut rng);
        sizes.push(tallest(&system, &effect));
    }
    // The live sequence's first two frames: 3.9 * 2.4 held to 0.287 of the
    // six-tick life, then a linear fall.
    assert!((sizes[0] - 9.36).abs() < 0.05, "{sizes:?}");
    // Live: `shazam` drew aspect 1.500 (its `+0xf0` block's `0.5`), so the quad
    // is 1.5 times as wide as it is tall.
    let first = extents(&system_at(&effect, 1), &effect);
    assert!(
        first
            .iter()
            .any(|&(w, h)| (h - 9.36).abs() < 0.05 && (w - 14.04).abs() < 0.05),
        "{first:?}"
    );
    assert!(sizes[2] < sizes[1] && sizes[3] < sizes[2], "{sizes:?}");
    // And the dark side of it: the same burst at severity 1 is 3.9, not 9.36 -
    // the size follows the severity, the count does not.
    let mut gentle = System::new();
    gentle.ignite(&effect, Vec3::ZERO, 1.0);
    gentle.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut Rng::new(1));
    assert!((tallest(&gentle, &effect) - 3.9).abs() < 0.05);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_templates_are_muted_for_every_other_source() {
    let Some(mut effect) = effect() else {
        return;
    };
    effect.without_pulse_psp_draw();
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 2.4);
    system.advance(
        &effect,
        1.0 / TICK_HZ,
        Vec3::ZERO,
        Vec3::Y,
        &mut Rng::new(1),
    );
    assert_eq!(
        system.alive_count(),
        7,
        "only the four emitters' first emission"
    );
}
