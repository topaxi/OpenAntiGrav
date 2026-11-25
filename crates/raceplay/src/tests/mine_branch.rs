//! HD Detonator lays mines on a route fork the way `Detonator_PlaceStageObjects` walks it, and
//! a circuit without a fork places exactly as before
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, "The fork walk").

use super::*;
use oag_core::Rng;
use oag_weapons::detonator::{Field, place_stage};

fn start(track: &str) -> Option<Race> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        track: Some(track.to_string()),
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    Some(Race::start(loaded.setup, loaded.view, loaded.modules))
}

/// Every object of all fourteen stages on `track`, from seed `seed`, as `(stage, kind, entry,
/// lane)` with nothing ever removed between stages.
fn lay_all(race: &Race, seed: u64) -> Vec<(u32, u8, u16, i8)> {
    let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
    let mut field = Field::new();
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for old_stage in 0..14 {
        place_stage(
            &mut field,
            &detonator.survey,
            &detonator.layout,
            &mut rng,
            old_stage,
        );
        for (kind, objects) in [(0u8, field.live_mines()), (1, field.live_bombs())] {
            out.extend(objects.iter().map(|p| (old_stage, kind, p.entry, p.lane)));
        }
    }
    out
}

fn fnv(laid: &[(u32, u8, u16, i8)]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &(stage, kind, entry, lane) in laid {
        for byte in stage
            .to_le_bytes()
            .into_iter()
            .chain([kind])
            .chain(entry.to_le_bytes())
            .chain(lane.to_le_bytes())
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// **A circuit with no fork places bit-identically.** The fourteen stages of Talon's Junction
/// from a fixed seed lay the same objects at the same entries and lanes as the ring-only
/// placement did.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_unforked_circuit_places_as_the_ring_alone_did() {
    let Some(race) = start(r"Data\Environments\talons_junction\track.vex") else {
        return;
    };
    let laid = lay_all(&race, 0x5eed);
    assert_eq!(laid.len(), 1870);
    println!("{} objects, hash {:#018x}", laid.len(), fnv(&laid));
    assert_eq!(
        fnv(&laid),
        0x6170_142b_60e5_ec5e,
        "talons_junction placement moved"
    );
}

/// **A forked circuit gets mines on its alternate route.** On each HD circuit with a split, the
/// fourteen stages lay objects on branch entries: the survey's lap array holds the branch's
/// separated sections between the primary route's and the merge, as the original's walk stores
/// them.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_forked_circuit_lays_mines_on_its_branch() {
    let mut bare = Vec::new();
    for track in super::emp_fork::FORKED {
        let Some(race) = start(track) else {
            return;
        };
        let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
        let ring = detonator.survey.entries.len();
        let in_lap = detonator
            .survey
            .lap
            .iter()
            .filter(|&&i| usize::from(i) >= ring)
            .count();
        let laid = lay_all(&race, 0x5eed);
        let on_branch = laid
            .iter()
            .filter(|&&(_, _, entry, _)| usize::from(entry) >= ring)
            .count();
        println!(
            "{track}: ring {ring}, {in_lap} branch entries in the lap, {} objects, \
             {on_branch} on the branch",
            laid.len()
        );
        if on_branch == 0 {
            bare.push(track);
        }
    }
    assert!(bare.is_empty(), "nothing laid on the branch: {bare:?}");
}

/// **Omega's placer walks the fork the same way** (`FUN_012a2300`, selectors at `0x012a2613`,
/// `0x012a2691`, `0x012a2976`), and its re-shipped HD circuits with a split get branch mines.
/// Omega's `zone_3` decodes to no triangles, so it is not among them.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn an_omega_forked_circuit_lays_mines_on_its_branch() {
    let Some(image) = oag_testdata::image("omega-ps4-eu.pkg") else {
        return;
    };
    for track in [
        r"Data\Environments\05_ubermall\track.vex",
        r"Data\Environments\15_anulpha_pass\track.vex",
    ] {
        let loaded = crate::load(&crate::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: Mode::Detonator,
            track: Some(track.to_string()),
            no_effects: true,
            ..crate::Options::default()
        })
        .expect("loading an Omega Detonator race");
        let race = Race::start(loaded.setup, loaded.view, loaded.modules);
        let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
        let ring = detonator.survey.entries.len();
        let laid = lay_all(&race, 0x5eed);
        let on_branch = laid
            .iter()
            .filter(|&&(_, _, entry, _)| usize::from(entry) >= ring)
            .count();
        println!(
            "omega {track}: {} objects, {on_branch} on the branch",
            laid.len()
        );
        assert!(on_branch > 0, "omega {track}: nothing laid on the branch");
    }
}
