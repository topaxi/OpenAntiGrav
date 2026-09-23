//! What the per-frame `LodGroup` switch reads off each disc, through the same
//! builder a race loads with.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(lod_switch_ground_truth)'
//! ```
//!
//! `oag_render::mesh::LodGroups` reads each group's position and switch
//! distances from the payload offsets `LodGroup_SelectChild` (`0x0890be68`)
//! reads on the PSP binary - see `docs/formats/vex.md`, "the switch is found".
//! This pins what those offsets give on every title whose geometry goes
//! through that builder, so a switch distance that decoded as garbage on a
//! port cannot quietly hide a tier at every range.

use std::path::PathBuf;

use oag_render::mesh::{self, Lod, Model};
use oag_vex::vex;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every team's hull, built with the switch on.
fn ships(archives: &mut oag_assets::Archives) -> Vec<(String, Vec<u8>, Model)> {
    oag_pulse::race::TEAMS
        .iter()
        .filter_map(|team| {
            let name = oag_pulse::race::ships::entry_name(team, "Ship");
            let blob = archives.read_name(&name).ok()?;
            let model = mesh::build_with_textures(&name, &blob, None, Lod::Original)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            Some((name, blob, model))
        })
        .collect()
}

/// Every numbered circuit directory's `track.vex` that the disc carries.
fn tracks(archives: &mut oag_assets::Archives) -> Vec<(String, Vec<u8>, Model)> {
    (1..=40)
        .filter_map(|n| {
            let name = format!(r"Data\Environments\{n:02}_Track\track.vex");
            let blob = archives.read_name(&name).ok()?;
            let model = mesh::build_with_textures(&name, &blob, None, Lod::Original)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            Some((name, blob, model))
        })
        .collect()
}

/// How many `LodGroup`s sit under another one - the case
/// `LodGroups::collect` tags with the nearest group only.
fn nested(blob: &[u8]) -> usize {
    let nodes = vex::nodes(blob).expect("the tree walks");
    let Some(class) = vex::classes_of(blob).ok().and_then(|c| c.lod_group) else {
        return 0;
    };
    nodes
        .iter()
        .filter(|n| n.class_id == class)
        .filter(|n| {
            let mut parent = n.parent;
            while let Some(p) = parent {
                if nodes[p].class_id == class {
                    return true;
                }
                parent = nodes[p].parent;
            }
            false
        })
        .count()
}

/// How many of a model's draw calls sit under a coarser tier than the first.
fn coarse_draws(model: &Model) -> usize {
    (model.draws.iter())
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .filter(|d| !model.lod_groups.shows_nearest(d))
        .count()
}

fn every_distance(model: &Model) -> Vec<f32> {
    (0..model.lod_groups.len())
        .flat_map(|g| model.lod_groups.distances(g).to_vec())
        .collect()
}

/// Every hull authors one two-child group, switching at `expected(entry)`,
/// and the coarse child's draws are tagged so the switch can hide them.
fn ships_switch_at(
    mut archives: oag_assets::Archives,
    at_least: usize,
    expected: impl Fn(&str) -> f32,
) {
    let ships = ships(&mut archives);
    assert!(ships.len() >= at_least, "{} hulls reachable", ships.len());
    for (name, blob, model) in &ships {
        println!(
            "{name}: position {:?}, distances {:?}, {} coarse draws",
            model.lod_groups.position(0),
            every_distance(model),
            coarse_draws(model)
        );
        assert_eq!(model.lod_groups.len(), 1, "{name}: one LodGroup");
        assert_eq!(every_distance(model), vec![expected(name)], "{name}");
        assert!(coarse_draws(model) > 0, "{name}: the coarse tier is tagged");
        assert_eq!(nested(blob), 0, "{name}");
    }
}

fn open_pulse(name: &str) -> Option<oag_assets::Archives> {
    let image = image(name)?;
    Some(oag_pulse::open(&image.display().to_string()).expect("the disc opens"))
}

/// The PSP build: every craft at 30 units, the figure `docs/formats/vex.md`
/// quotes for Assegai and Qirex.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_pulse_psp_craft_switches_at_thirty_units() {
    let Some(archives) = open_pulse("pulse-psp-usa.chd") else {
        return;
    };
    ships_switch_at(archives, 8, |_| 30.0);
}

/// The PS2 build authors its own, further distances: 35 units on seven
/// craft and 32.05 on Goteki - at the same payload offset, so the port
/// re-authored the value rather than moving it.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn every_pulse_ps2_craft_switches_at_its_own_distance() {
    let Some(archives) = open_pulse("pulse-ps2-eu.chd") else {
        return;
    };
    ships_switch_at(archives, 8, |name| {
        if name.contains("Goteki") {
            32.045_677
        } else {
            35.0
        }
    });
}

/// Every circuit's groups decode to finite, positive distances, none nested.
fn pulse_tracks_decode(image_name: &str) {
    let Some(image) = image(image_name) else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("the disc opens");
    let tracks = tracks(&mut archives);
    assert!(!tracks.is_empty(), "no circuit found");
    let mut groups = 0;
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for (name, blob, model) in &tracks {
        assert_eq!(nested(blob), 0, "{name}");
        for distance in every_distance(model) {
            assert!(
                distance.is_finite() && distance > 0.0,
                "{name}: switch distance {distance}"
            );
            lo = lo.min(distance);
            hi = hi.max(distance);
        }
        groups += model.lod_groups.len();
    }
    println!(
        "{image_name}: {} circuits, {groups} groups, distances {lo}..={hi}",
        tracks.len()
    );
    assert!(groups > 0, "no circuit authors a LodGroup");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_pulse_psp_circuit_group_decodes_a_positive_distance() {
    pulse_tracks_decode("pulse-psp-usa.chd");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn every_pulse_ps2_circuit_group_decodes_a_positive_distance() {
    pulse_tracks_decode("pulse-ps2-eu.chd");
}

/// `16_Track`'s groups switch between 118.9 and 424.0 units - the figures
/// `docs/formats/vex.md` quotes from `crates/vex/examples/lod_group_probe.rs`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn talons_junction_switches_between_119_and_424_units() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("the disc opens");
    let name = oag_pulse::race::DEFAULT_TRACK;
    let blob = archives.read_name(name).expect("the track reads");
    let model = mesh::build_with_textures(name, &blob, None, Lod::Original).expect("it builds");
    assert_eq!(
        model.lod_groups.len(),
        11,
        "11 LodGroups, as the probe lists"
    );
    let distances = every_distance(&model);
    assert_eq!(distances.len(), 10, "ten two-child groups, one one-child");
    let lo = distances.iter().copied().fold(f32::MAX, f32::min);
    let hi = distances.iter().copied().fold(f32::MIN, f32::max);
    assert!((lo - 118.9).abs() < 0.05, "{lo}");
    assert!((hi - 424.0).abs() < 0.05, "{hi}");
    // Both builds carry the same geometry; `Both` just carries no table.
    let both = mesh::build_with_textures(name, &blob, None, Lod::Both).expect("it builds");
    assert!(both.lod_groups.is_empty());
    assert_eq!(both.indices.len(), model.indices.len());
}

/// Pure's six reachable hulls carry the same shape, switching at 50 units.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn every_reachable_pure_craft_switches_at_fifty_units() {
    let Some(image) = image("pure-psp-usa.chd") else {
        return;
    };
    let archives = oag_pure::open(&image.display().to_string()).expect("the disc opens");
    ships_switch_at(archives, 6, |_| 50.0);
}
