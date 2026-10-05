//! Wipeout 2048's own `.pob` effects, off the user's decrypted Vita package.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! just test-data
//! ```
//!
//! Measured 2026-10-05: 174 `.pob` across the five archives that could hold
//! one - 151 in the base `data.psarc` (`data/particles` 66, `data/particles2048`
//! 85, the same stems with different bytes), 23 in the 1.04 patch's
//! `data2.psarc`, none in either DLC pack or `data1.psarc`. The existing reader
//! takes all of them unchanged; the four it refuses are refused by name.

use std::collections::BTreeSet;

use oag_assets::psarc::Archive;
use oag_core::Rng;
use oag_core::math::Vec3;
use oag_pob as pob;
use oag_render::psys::sprite::Sprite;
use oag_render::psys::{ColourScale, Effect, System, TICK_HZ};

/// Archives that carry `.pob`, and how many each holds.
const ARCHIVES: [(&str, usize); 5] = [
    ("base/PSP2/data.psarc", 151),
    ("dlc1/PSP2/dlc1.psarc", 0),
    ("dlc2/PSP2/dlc2.psarc", 0),
    ("patch-v104/PSP2/data1.psarc", 0),
    ("patch-v104/PSP2/data2.psarc", 23),
];

fn root() -> Option<std::path::PathBuf> {
    oag_testdata::exact("data/extracted/vita/PCSF00007")
}

/// Plays every 2048 effect for four seconds, and holds the refusals to the
/// two the reader knows by name, in both directories that author them.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_2048_effect_plays_or_is_refused_by_name() {
    let Some(root) = root() else {
        return;
    };
    let dt = 1.0 / TICK_HZ;
    let (mut seen, mut played) = (0usize, 0usize);
    let mut refused = Vec::new();
    for (name, expected) in ARCHIVES {
        let mut archive = Archive::open_file(&root.join(name)).expect("the archive opens");
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|entry| entry.to_ascii_lowercase().ends_with(".pob"))
            .cloned()
            .collect();
        assert_eq!(entries.len(), expected, "{name}");
        for entry in &entries {
            let blob = archive.read_path(entry).expect("the entry reads");
            seen += 1;
            assert!(
                pob::looks_like_particle_system(&blob),
                "{entry}: not a particle system"
            );
            // The palettes run to 255, as the PSP's and HD's do and unlike the
            // PS2's 0-127: `Full` is a decision here, not the `_ =>` arm.
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
                assert_eq!((additive.len() + alpha_over.len()) % 6, 0);
            }
            played += 1;
        }
    }
    assert_eq!(seen, 174);
    refused.sort();
    assert_eq!(refused.len(), 4, "refusals: {refused:?}");
    for entry in &refused {
        assert!(
            (entry.contains("WO_BARRIER_COLLISION") && entry.contains("render mode 3"))
                || (entry.contains("WO_NITRO_SHIP_DEATH.pob") && entry.contains("blend class 4")),
            "an unknown refusal: {entry}"
        );
    }
    assert_eq!(played, 170);
}

/// `Full` is right for Vita: some authored colour sits above the PS2's
/// 127 ceiling.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_palettes_run_to_255_so_the_scale_is_full() {
    let Some(root) = root() else {
        return;
    };
    let mut archive = Archive::open_file(&root.join(ARCHIVES[0].0)).expect("opens");
    let blob = archive
        .read_path("data/particles2048/WO_ROCKET_FLARE.pob")
        .expect("reads");
    let effect = Effect::parse(&blob, ColourScale::Full).expect("parses");
    let top = effect
        .emitters
        .iter()
        .flat_map(|spec| spec.palette.iter().flatten().copied())
        .fold(0.0f32, f32::max);
    assert!(top > 0.9, "brightest palette channel {top}");
}

/// Every emitter names a texture, every name is a `.gxt` beside the effect, and
/// every one decodes - so no 2048 emitter is left on the procedural profile
/// for want of its sprite.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_emitter_names_a_sprite_that_ships_and_decodes() {
    let Some(root) = root() else {
        return;
    };
    let mut archive = Archive::open_file(&root.join(ARCHIVES[0].0)).expect("opens");
    let entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.starts_with("data/particles2048/") && p.ends_with(".pob"))
        .cloned()
        .collect();
    let mut stems = BTreeSet::new();
    for entry in &entries {
        let blob = archive.read_path(entry).expect("reads");
        let system = pob::ParticleSystem::parse(&blob).expect("parses");
        for emitter in system.emitters(&blob).expect("walks") {
            let path = system
                .texture_path(&blob, &emitter)
                .unwrap_or_else(|| panic!("{entry}: {} names no texture", emitter.name));
            let stem = path.rsplit('\\').next().unwrap().trim_end_matches(".tga");
            stems.insert(stem.to_ascii_lowercase());
        }
    }
    assert_eq!(stems.len(), 76);
    for stem in &stems {
        let name = format!("data/particles2048/tex/{stem}.gxt");
        let blob = archive
            .read_path(&name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(Sprite::from_gxt(&blob).is_some(), "{name}: does not decode");
    }
}
