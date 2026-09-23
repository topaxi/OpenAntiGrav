//! Measures what the authored PVS would actually cull, on real tracks.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! [ADR-0011](../../../docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md)
//! rests on two numbers, and only one of them was known when it was written.
//!
//! The known one is authored: a mean of **7.6 of 64** sections visible from a
//! section, measured over every track file on both discs by
//! `oag-formats`' `pvs_ground_truth`. That is the ceiling on what the first
//! tier can exclude.
//!
//! The unknown one is ours: **what fraction of a track's draw calls a
//! section governs.** Placement is authored - a `section` node governs its
//! parent's whole subtree, and a draw call inherits its scene node's group
//! (see `oag_vex::pvs::governing_sections`). A draw call outside every
//! group is deliberately never culled, so it passes tier one and still costs
//! a frustum test. If the governed fraction were small, the ordering argument
//! would be sound and the saving would still be nearly nothing - the first
//! tier would be excluding sections that hold hardly any geometry.
//!
//! So this walks real tracks and reports, per track:
//!
//! - how many draw calls a section governs, and how many it does not;
//! - the mean fraction of draw calls surviving tier one, taken over **every
//!   section in turn as the camera's section** rather than one hand-picked
//!   viewpoint, which is the honest average rather than a best case.
//!
//! The second figure is the one that says whether PVS-before-frustum is worth
//! switching on. It is printed rather than asserted at a target: this test
//! exists to produce the measurement the ADR is missing, not to enforce a
//! number nobody has justified yet. The two assertions it does make are the
//! ones that would invalidate the design rather than merely disappoint it - see
//! each `assert!`.

use std::path::{Path, PathBuf};

use oag_render::mesh::{self, Lod};
use oag_render::pvs::{DrawSections, SectionPadding, SwapConflicts, UNPLACED, VisibleSet};
use oag_vex::pvs::TrackPvs;
use oag_vex::{track, vex};

/// Track directories to probe. Named the way the front end names them, via the
/// `location` attribute plus the binary's `%s\%strack%s.vex` template - see
/// `docs/formats/track.md`. Ids that are not on the disc are skipped, so this
/// is a superset rather than a claim about which exist.
const TRACK_IDS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
    "17_Track", "18_Track", "19_Track", "20_Track",
];

/// The four `.vex` files a track directory can hold, from the binary's
/// `%s\%strack%s.vex` template - forward, reversed, and the Zone variants of
/// each. Measuring only `track.vex` covers a quarter of the disc's 40 track
/// files and misses every reversed circuit, which is most of what a player
/// races.
const VARIANTS: &[&str] = &[
    "track.vex",
    "track_reversed.vex",
    "zone_track.vex",
    "zone_track_reversed.vex",
];

fn track_entry_name(track: &str, variant: &str) -> String {
    format!(r"Data\Environments\{track}\{variant}")
}

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_authored_boxes_place_most_of_a_tracks_geometry() {
    // Both discs: the game renders PS2 tracks through the same placement, so
    // the sibling-group rule has to hold there too, not just on the platform
    // it was discovered on. The per-file expectations are identical; only
    // the file count differs (40 PSP, 59 PS2 - reversed circuits share
    // directories on PSP but not everywhere on PS2).
    for (disc, minimum) in [("pulse-psp-usa.chd", 30), ("pulse-ps2-eu.chd", 40)] {
        let Some(image) = image(disc) else { continue };
        println!("==== {disc}");
        sweep(&image, minimum);
    }
}

fn sweep(image: &Path, minimum: usize) {
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut measured = 0usize;
    let mut worst_track = (String::new(), 0.0f64);
    let mut best_track = (String::new(), 100.0f64);
    for id in TRACK_IDS {
        for variant in VARIANTS {
            let name = &format!("{id}/{variant}");
            let entry = track_entry_name(id, variant);
            let Ok(blob) = archives.read_name(&entry) else {
                continue;
            };
            measured += 1;
            let model = mesh::build_with_textures(&entry, &blob, None, Lod::default())
                .expect("decoding track.vex");
            let pvs = TrackPvs::parse(&blob).expect("parsing sections");

            let nodes = vex::nodes(&blob).expect("walking the tree");
            let ai_node = track::find_node(&blob, &nodes).expect("a WO Track node");
            let ai = track::parse(&blob[ai_node.payload()]).expect("parsing the spline");
            let padding = SectionPadding::from_track(&ai);

            let governing =
                oag_vex::pvs::governing_sections(&blob, &nodes).expect("deriving governance");
            let (sections, placement) = DrawSections::place(&model, &governing, &pvs);
            let swaps = SwapConflicts::find(&pvs, &sections, &model);
            let lists = [
                (&sections.opaque, &model.draws),
                (&sections.alpha_tested, &model.alpha_tested_draws),
                (&sections.transparent, &model.transparent_draws),
            ];
            let total: usize = lists.iter().map(|(_, draws)| draws.len()).sum();

            // Average over every section in turn, twice.
            //
            // **`together` overstates the culling and is kept only as the ceiling.**
            // It puts the craft and the camera in the same section, which a chase
            // camera almost never does: the spring lags, so the camera is usually a
            // section or two behind. `apart` models that by putting the camera in a
            // spline neighbour of the craft's section, and it is the figure a
            // running game will actually show - the visible set is then the union
            // of two sections' masks *and* both of their padding neighbourhoods.
            //
            // Measuring only `together` is what made an earlier version of this
            // test report a saving the game did not deliver.
            let ids: Vec<u8> = pvs.ids().collect();
            let survivors = |set: &VisibleSet| -> usize {
                lists
                    .iter()
                    .flat_map(|(list, _)| list.iter())
                    .filter(|&&sections| set.allows(sections))
                    .count()
            };
            let mut together_total = 0usize;
            let mut apart_total = 0usize;
            let mut worst = 0usize;
            for &id in &ids {
                together_total += survivors(&VisibleSet::around(&pvs, &padding, &swaps, id, id));
                // The camera trails into a neighbour: the lowest-numbered section
                // adjacent to this one that is not itself.
                let neighbour = oag_vex::pvs::set_bits(padding.near(id) & !(1u64 << id))
                    .next()
                    .unwrap_or(id);
                let apart = survivors(&VisibleSet::around(&pvs, &padding, &swaps, id, neighbour));
                apart_total += apart;
                worst = worst.max(apart);
            }
            let together = together_total as f64 / ids.len() as f64;
            let mean_survivors = apart_total as f64 / ids.len() as f64;

            println!(
                "== {name}: {total} draw calls, {} governed by a section ({:.1}%), \
             {} always drawn, {} LOD-swap pair(s)",
                placement.placed,
                placement.placed_fraction() * 100.0,
                placement.unplaced,
                swaps.pair_count(),
            );
            println!(
                "   over all {} sections: camera trailing the craft (the live case) \
             {mean_survivors:.0} draw calls reach the frustum test ({:.1}% of \
             {total}), worst section {worst} ({:.1}%); camera in the craft's own \
             section (the ceiling) {together:.0} ({:.1}%)",
                ids.len(),
                mean_survivors * 100.0 / total as f64,
                worst as f64 * 100.0 / total as f64,
                together * 100.0 / total as f64,
            );

            // Not a performance target - a design check. If the authored groups
            // governed almost nothing, the first tier would be excluding sections
            // that hold no geometry, and the ordering argument would be sound but
            // pointless. Half is a low bar deliberately: it fails only if the
            // association rule is broadly not working.
            assert!(
                placement.placed_fraction() > 0.5,
                "{name}: authored groups govern only {:.1}% of draw calls, so PVS \
             culling would barely reduce what the frustum test sees - the \
             sibling-group rule in oag_vex::pvs::governing_sections needs \
             revisiting",
                placement.placed_fraction() * 100.0
            );

            // The safety property, on real data rather than on a fixture: an
            // unplaced draw call must survive even a mask that hides everything.
            let hides_everything = VisibleSet::around(&pvs, &padding, &swaps, UNPLACED, UNPLACED);
            assert!(
                hides_everything.is_everything(),
                "{name}: an unknown section must make everything visible"
            );

            let surviving = mean_survivors * 100.0 / total as f64;
            if surviving > worst_track.1 {
                worst_track = (name.clone(), surviving);
            }
            if surviving < best_track.1 {
                best_track = (name.clone(), surviving);
            }
        }
    }
    assert!(
        measured >= minimum,
        "only {measured} track file(s) found; expected at least {minimum} on this disc"
    );
    println!(
        "== measured {measured} track file(s); best {} at {:.1}% surviving, \
         worst {} at {:.1}%",
        best_track.0, best_track.1, worst_track.0, worst_track.1
    );
}
