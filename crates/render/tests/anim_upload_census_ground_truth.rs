//! Which race drawables author texture-transform or node animation, so the
//! per-frame upload sites can be checked against them.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all anim_upload_census --no-capture
//! ```
//!
//! Each line is `name: texture tracks / node anims`. The table below pins the
//! models that author texture tracks, because each needs a `write_anims` at its
//! draw site (see `docs/rendering/mesh-animation.md`, "Upload audit").

use oag_mesh::mesh;

/// Every model a race draws through a `Drawable`, in the order the census
/// prints them.
/// `(entry, texture tracks authored)` that each draw site must upload, as
/// measured 2026-10-05. A change here is a change to the audit.
const EXPECTED: &[(&str, usize)] = &[
    (r"Data\Weapons\Rocket.vex", 0),
    (r"Data\Weapons\Pulse_Mine.vex", 1),
    (r"Data\Weapons\Pulse_Bomb.vex", 2),
    (r"Data\Weapons\pulse_muzzleflash.vex", 0),
    (r"Data\Weapons\pulse_plasma_halo1.vex", 1),
    (r"Data\Weapons\pulse_plasma_hemisphere1.vex", 1),
    (r"Data\Weapons\pulse_plasma_hemisphere2.vex", 1),
    (r"Data\Weapons\explosion_hemisphere.vex", 1),
    (r"Data\Weapons\Bomb_Shockwave.vex", 2),
    (r"Data\Weapons\pulse_repulsorwave.vex", 1),
    (r"Data\Weapons\shield.vex", 1),
    (r"Data\visual_effects\MagEffect1.vex", 1),
    (r"Data\visual_effects\MagEffect2.vex", 1),
    (r"Data\Ships\Feisar\Ship.vex", 1),
    (r"Data\Ships\Feisar\shipshield.vex", 1),
    (r"Data\Ships\Feisar\shipboost.vex", 1),
];

const NAMES: &[&str] = &[
    r"Data\Weapons\Rocket.vex",
    r"Data\Weapons\Pulse_Mine.vex",
    r"Data\Weapons\Pulse_Bomb.vex",
    r"Data\Weapons\pulse_muzzleflash.vex",
    r"Data\Weapons\pulse_plasma_halo1.vex",
    r"Data\Weapons\pulse_plasma_hemisphere1.vex",
    r"Data\Weapons\pulse_plasma_hemisphere2.vex",
    r"Data\Weapons\explosion_hemisphere.vex",
    r"Data\Weapons\Bomb_Shockwave.vex",
    r"Data\Weapons\pulse_repulsorwave.vex",
    r"Data\Weapons\shield.vex",
    r"Data\Weapons\vr_shield_cockpit.vex",
    r"Data\visual_effects\MagEffect1.vex",
    r"Data\visual_effects\MagEffect2.vex",
    r"Data\Ships\Feisar\Ship.vex",
    r"Data\Ships\Feisar\shipshield.vex",
    r"Data\Ships\Feisar\shipboost.vex",
    r"Data\Ships\Feisar\Zoneboost.vex",
    r"Data\Ships\Feisar\shipwreck.vex",
    r"Data\Ships\Feisar\Zone.vex",
    r"Data\Ships\Feisar\zonewreck.vex",
    r"Data\Environments\16_Track\track.vex",
];

#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn anim_upload_census() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    for name in NAMES {
        let Ok(blob) = archives.read_name(name) else {
            println!("{name}: absent");
            continue;
        };
        match mesh::build(name, &blob) {
            Ok(m) => {
                if let Some((_, want)) = EXPECTED.iter().find(|(n, _)| n == name) {
                    assert_eq!(m.anim_tracks.len(), *want, "{name}: texture tracks");
                }
                println!("{name}: {} / {}", m.anim_tracks.len(), m.anim_nodes.len());
                for n in &m.anim_nodes {
                    println!("    node {:?}", n.transform);
                }
                for t in &m.anim_tracks {
                    if let mesh::AnimTrack::Psp(t) = t {
                        println!(
                            "    off {:?}@{:?} scale {:?}@{:?} loop {} step {}",
                            t.offset.values, t.offset.times, t.scale.values, t.scale.times,
                            t.loop_seconds, t.step
                        );
                    }
                }
            }
            Err(e) => println!("{name}: build failed: {e}"),
        }
    }
}
