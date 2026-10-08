//! Pulse's circuit unlock, against the disc that authors it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test unlock_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! Which circuits carry no `<Unlock>` (three), which grid each other one names, and that
//! `grid0`'s own cells earn the points `grid_points_met` compares. Nothing here writes a
//! circuit or grid name down: the expectations are read off the same definition.

use oag_game::records::{Medal, Store};
use oag_game::unlock::{self, Gate};
use oag_raceplay::catalogue::{self, Track};

const DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

struct Disc {
    tracks: Vec<Track>,
    gate: Gate,
    grid0_cells: Vec<String>,
    grid0_name: String,
}

fn disc() -> Option<Disc> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let blob = opened
        .archives
        .read_name(DEFINITION)
        .expect("the definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition expands");
    let tracks = catalogue::tracks(&xml);
    let grids = oag_game::campaign::read_grids(
        &mut opened.archives,
        oag_pulse::campaign::DEFINITION_ENTRY,
        None,
    )
    .expect("the grids");
    let grid0_cells = grids[0].cells.iter().map(|c| c.name.clone()).collect();
    let grid0_name = grids[0].name.clone();
    Some(Disc {
        tracks,
        gate: Gate::on_grids(grids),
        grid0_cells,
        grid0_name,
    })
}

fn offered(disc: &Disc, records: &Store) -> Vec<String> {
    disc.tracks
        .iter()
        .filter(|track| disc.gate.offers(track, records, oag_pulse::TITLE.name))
        .map(|track| track.id.clone())
        .collect()
}

/// A fresh profile is offered exactly the circuits that author no `<Unlock>`: the list
/// `TrackSelection_PopulateList` produced in the 2026-09-29 PPSSPP walk (three circuits).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_fresh_profile_is_offered_exactly_the_circuits_with_no_unlock() {
    let Some(disc) = disc() else { return };
    let mut want: Vec<String> = disc
        .tracks
        .iter()
        .filter(|track| track.unlock_grid.is_none())
        .map(|track| track.id.clone())
        .collect();
    let mut got = offered(&disc, &Store::default());
    want.sort();
    got.sort();
    assert_eq!(got, want);
    assert_eq!(
        got.len(),
        3,
        "three circuits, as the original's own list: {got:?}"
    );
    assert_eq!(disc.tracks.len(), 24);
}

/// Gold on every cell of `grid0` opens exactly the circuits that name that grid, and nothing
/// that names a later one - the comparison is per named grid, not "any progress".
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn clearing_grid0_opens_exactly_the_circuits_that_name_it() {
    let Some(disc) = disc() else { return };
    let mut records = Store::default();
    for cell in &disc.grid0_cells {
        records.record_campaign(oag_pulse::TITLE.name, cell, Some(Medal::Gold), None);
    }
    let fresh = offered(&disc, &Store::default());
    let after = offered(&disc, &records);
    let opened: Vec<&String> = after.iter().filter(|id| !fresh.contains(id)).collect();
    let named: Vec<&String> = disc
        .tracks
        .iter()
        .filter(|track| {
            track
                .unlock_grid
                .as_deref()
                .is_some_and(|grid| grid.eq_ignore_ascii_case(&disc.grid0_name))
        })
        .map(|track| &track.id)
        .collect();
    assert!(!named.is_empty(), "grid0 gates some circuit");
    assert_eq!(opened, named);
    assert!(after.len() < disc.tracks.len(), "later grids stay closed");
}

/// `--unlock-all` is ours and offers all 24, as the original's dev byte did.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn unlock_all_offers_every_circuit() {
    let Some(disc) = disc() else { return };
    unlock::set_unlock_all(true);
    let all = offered(&disc, &Store::default());
    unlock::set_unlock_all(false);
    assert_eq!(all.len(), 24);
}
