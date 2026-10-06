//! Wipeout: Omega Collection's own `.pob` effects and `.gnf` sprites, off the
//! user's decrypted PS4 extraction.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! just test-data
//! ```
//!
//! Measured 2026-10-05: `data00.psarc` and the patch's `data05.psarc` each
//! carry the same 97 `Data/particles` effects and 112 `Data/particles2048`
//! effects; the base alone adds 2 in each `Get` subdirectory, whose `Get\Tex`
//! sprites ship nowhere, and nothing loads them. The existing reader takes the PS4
//! dialect unchanged; what it refuses, it refuses by name: `WO_NITRO_SHIP_DEATH`
//! (blend class 4) and `WO_BARRIER_COLLISION` (render mode 3), in each directory.
//! The one thing Omega adds, blend class 8 (the heat-haze and shock-distortion
//! emitters its `psys_normal_heathaze` shader draws), is read as
//! [`Blend::Distort`]: simulated and named, not drawn.

use std::collections::{BTreeMap, BTreeSet};

use oag_assets::psarc::Archive;
use oag_fx::psys::sprite::Sprite;
use oag_fx::psys::{Blend, ColourScale, Effect};
use oag_pob as pob;

const ARCHIVES: [&str; 2] = [
    "data/extracted/ps4/omega-eu/uroot/data00.psarc",
    "data/extracted/ps4/omega-eu-patch/uroot/data05.psarc",
];

/// Each directory with its file count and how many parse, in both archives.
const DIRECTORIES: [(&str, usize, usize); 2] =
    [("Data/particles", 97, 95), ("Data/particles2048", 112, 110)];

/// The `Get` subdirectories only the base archive has: 2 effects each, both parse.
const GET: [&str; 2] = ["Data/particles/Get", "Data/particles2048/Get"];

fn open(rel: &str) -> Option<Archive> {
    let path = oag_testdata::exact(rel)?;
    Some(Archive::open_file(&path).expect("the archive opens"))
}

/// The counts above hold in both archives, and every refusal is a known one.
#[test]
#[ignore = "needs the PS4 extraction in data/extracted/ps4/"]
fn every_omega_effect_parses_or_is_refused_by_name() {
    for rel in ARCHIVES {
        let Some(mut archive) = open(rel) else {
            return;
        };
        let mut tally: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".pob"))
            .cloned()
            .collect();
        for entry in &entries {
            let blob = archive.read_path(entry).expect("the entry reads");
            assert!(
                pob::looks_like_particle_system(&blob),
                "{rel}: {entry}: not a particle system"
            );
            let dir = entry.rsplit_once('/').expect("a directory").0.to_string();
            let row = tally.entry(dir).or_default();
            row.0 += 1;
            match Effect::parse(&blob, ColourScale::Full) {
                Ok(_) => row.1 += 1,
                Err(error) => {
                    let why = error.to_string();
                    assert!(
                        (entry.contains("WO_NITRO_SHIP_DEATH") && why.contains("blend class 4"))
                            || (entry.contains("WO_BARRIER_COLLISION")
                                && why.contains("render mode 3")),
                        "{rel}: {entry}: an unknown refusal: {why}"
                    );
                }
            }
        }
        let mut want: BTreeMap<String, (usize, usize)> = DIRECTORIES
            .iter()
            .map(|&(dir, files, parsed)| (dir.to_string(), (files, parsed)))
            .collect();
        if rel == ARCHIVES[0] {
            want.extend(GET.iter().map(|dir| (dir.to_string(), (2, 2))));
        }
        assert_eq!(tally, want, "{rel}");
    }
}

/// Every emitter names a sprite, and the `.gnf` at the authored path - under
/// `Data/` whichever directory the effect itself sits in - ships and decodes.
#[test]
#[ignore = "needs the PS4 extraction in data/extracted/ps4/"]
fn every_emitter_names_a_gnf_that_ships_and_decodes() {
    let Some(mut archive) = open(ARCHIVES[0]) else {
        return;
    };
    let entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".pob"))
        .cloned()
        .collect();
    let mut sprites = BTreeSet::new();
    for entry in entries.iter().filter(|e| !e.contains("/Get/")) {
        let blob = archive.read_path(entry).expect("reads");
        let Ok(system) = pob::ParticleSystem::parse(&blob) else {
            continue;
        };
        for emitter in system.emitters(&blob).expect("walks") {
            let authored = system
                .texture_path(&blob, &emitter)
                .unwrap_or_else(|| panic!("{entry}: {} names no texture", emitter.name));
            let at = authored
                .to_ascii_lowercase()
                .find("data\\")
                .expect("under Data");
            let name = authored[at..].replace('\\', "/");
            sprites.insert(name.trim_end_matches(".tga").to_string() + ".gnf");
        }
    }
    assert!(sprites.len() > 100, "{} sprite names", sprites.len());
    let mut absent = Vec::new();
    for name in &sprites {
        match archive.read_path(name) {
            Ok(blob) => assert!(Sprite::from_gnf(&blob).is_some(), "{name}: does not decode"),
            Err(_) => absent.push(name.clone()),
        }
    }
    // `WO_START_LASER` names a sprite that ships in no archive, under any
    // directory - an honest absence, drawn procedurally if it were ever wired.
    assert_eq!(
        absent,
        ["Data/particles2048/Tex/WO2048_Start_Laser.gnf"],
        "of {} sprites",
        sprites.len()
    );
}

/// The five explosions the player sees parse, their `shockdistort` emitter is
/// the one that draws nothing, and every other emitter keeps a drawn blend.
#[test]
#[ignore = "needs the PS4 extraction in data/extracted/ps4/"]
fn the_explosions_play_with_only_their_distortion_emitter_undrawn() {
    let Some(mut archive) = open(ARCHIVES[1]) else {
        return;
    };
    for name in [
        "WO_ROCKET_EXPLO",
        "WO_ROCKET_EXPLO_TRACK",
        "WO_MISSILE_EXPLO",
        "WO_BOMB_SMOKERING",
        "WO_PLASMA_LIGHTNING_EXPAND",
    ] {
        let blob = archive
            .read_path(&format!("Data/particles/{name}.pob"))
            .expect("the effect ships");
        let effect = Effect::parse(&blob, ColourScale::Full).expect("it parses");
        assert_eq!(
            effect.undrawn_emitters().collect::<Vec<_>>(),
            ["shockdistort"],
            "{name}"
        );
        assert!(
            effect
                .emitters
                .iter()
                .filter(|spec| spec.blend != Blend::Distort)
                .count()
                >= 1,
            "{name}: the effect has a drawn emitter besides the distortion"
        );
    }
}
