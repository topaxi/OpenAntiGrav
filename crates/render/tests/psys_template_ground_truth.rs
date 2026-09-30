//! Plays the sprite templates of a real struck-craft effect. `#[ignore]`d: it
//! needs game content (`just test-data`).
//!
//! The numbers are the live PPSSPP capture's (2026-09-30), a hit on the player
//! at severity 2.4: `shazam` half-sizes `9.36, 9.36, 8.73, 6.54, 4.33` over
//! the frames after it starts, then gone; `glow` flickering between `0.748`
//! and `4.8`. See `oag_vex::pob`'s `initial` module and `psys::template`.

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

/// The half-size of the widest additive quad this tick drew, in world units.
fn widest(system: &System, effect: &Effect) -> f32 {
    let (additive, _) = system.vertices(effect, Vec3::X, Vec3::Y);
    additive
        .chunks(6)
        .map(|quad| {
            let xs = quad.iter().map(|v| v.position[0]);
            let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(l, h), x| (l.min(x), h.max(x)));
            (hi - lo) * 0.5
        })
        .fold(0.0, f32::max)
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
        sizes.push(widest(&system, &effect));
    }
    // The live sequence's first two frames: 3.9 * 2.4 held to 0.287 of the
    // six-tick life, then a linear fall.
    assert!((sizes[0] - 9.36).abs() < 0.05, "{sizes:?}");
    assert!(sizes[2] < sizes[1] && sizes[3] < sizes[2], "{sizes:?}");
    // And the dark side of it: the same burst at severity 1 is 3.9, not 9.36 -
    // the size follows the severity, the count does not.
    let mut gentle = System::new();
    gentle.ignite(&effect, Vec3::ZERO, 1.0);
    gentle.advance(&effect, dt, Vec3::ZERO, Vec3::Y, &mut Rng::new(1));
    assert!((widest(&gentle, &effect) - 3.9).abs() < 0.05);
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
