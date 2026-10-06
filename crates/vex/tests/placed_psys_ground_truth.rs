//! The particle effects Pulse's circuits place themselves, pinned against the
//! disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! `PsysNode_Init` (`0x089156a0`) spawns one instance per `ParticleSystem`
//! node at load, named by the node's `Name` attribute. A live PPSSPP capture on
//! Basilico Black (`01_Track`) caught exactly three `WO_BLUE_WELDER` loads with
//! these three nodes' own matrices. See
//! `docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`.

use std::collections::BTreeMap;

use oag_vex::placed_psys::{self, Placed};
use oag_vex::vex;

fn placed_in(file: &str) -> Option<Vec<Placed>> {
    let path = oag_testdata::image("pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(path.to_str().expect("utf-8 path")).expect("open");
    let blob = archives
        .read_name(&format!(r"Data\Environments\{file}"))
        .expect("track.vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    Some(placed_psys::placed(&blob, &nodes))
}

fn census(placed: &[Placed]) -> BTreeMap<&str, usize> {
    let mut out = BTreeMap::new();
    for p in placed {
        *out.entry(p.name.as_str()).or_default() += 1;
    }
    out
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn the_circuits_place_the_welders_and_the_steam() {
    let expected: &[(&str, &[(&str, usize)])] = &[
        ("01_Track\\track.vex", &[("WO_BLUE_WELDER", 3)]),
        ("01_Track\\track_reversed.vex", &[("WO_BLUE_WELDER", 3)]),
        (
            "05_Track\\track.vex",
            &[("WO_BLUE_WELDER", 6), ("WO_MODESTO_STEAM_A", 2)],
        ),
        (
            "05_Track\\track_reversed.vex",
            &[("WO_BLUE_WELDER", 7), ("WO_MODESTO_STEAM_A", 2)],
        ),
        ("07_Track\\track.vex", &[("WO_MODESTO_STEAM_A", 18)]),
        (
            "07_Track\\track_reversed.vex",
            &[("WO_MODESTO_STEAM_A", 18)],
        ),
        ("16_Track\\track.vex", &[]),
    ];
    for (file, want) in expected {
        let Some(placed) = placed_in(file) else {
            return;
        };
        let want: BTreeMap<&str, usize> = want.iter().copied().collect();
        assert_eq!(census(&placed), want, "{file}");
    }
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn a_welder_sits_where_the_live_capture_put_it() {
    let Some(placed) = placed_in("01_Track\\track.vex") else {
        return;
    };
    // The first node's own matrix, read live off `node+0x50` on PPSSPP, is
    // also its world matrix: its one `Transform` parent is the identity.
    let world = placed[0].world_at(0.0);
    let at = [world[12], world[13], world[14]];
    let want = [-597.4318, 23.906_868, 829.561_95];
    for (got, want) in at.iter().zip(want) {
        assert!((got - want).abs() < 1e-3, "{at:?} vs {want:?}");
    }
    assert!(!placed[0].moves());
}

/// The chain [`Placed::world_at`] composes is the same product
/// [`vex::world_transforms_at`] gives the scenery around it, at any time: a
/// placed effect stays on the geometry it was authored on, moving or not.
#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn a_placed_effect_composes_like_the_scenery_it_sits_in() {
    for file in [
        "05_Track\\track.vex",
        "07_Track\\track.vex",
        "01_Track\\track.vex",
    ] {
        let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
            return;
        };
        let mut archives = oag_pulse::open(path.to_str().expect("utf-8")).expect("open");
        let blob = archives
            .read_name(&format!(r"Data\Environments\{file}"))
            .expect("track.vex");
        let nodes = vex::nodes(&blob).expect("nodes");
        let placed = placed_psys::placed(&blob, &nodes);
        let moving = placed.iter().filter(|p| p.moves()).count();
        for seconds in [0.0, 3.7, 12.25, 61.0] {
            let world = vex::world_transforms_at(&blob, &nodes, seconds);
            for p in &placed {
                let parent = nodes[p.node].parent.map_or(vex::IDENTITY, |q| world[q]);
                let own = vex::transform(&blob[nodes[p.node].payload()], vex::byte_order(&blob))
                    .expect("matrix");
                let want = vex::multiply(&own, &parent);
                let got = p.world_at(seconds);
                for (g, w) in got.iter().zip(want) {
                    assert!((g - w).abs() < 1e-3, "{file} node {} at {seconds}", p.node);
                }
            }
        }
        // Circuit five's two steam vents are the ones under an `Anim Transform`.
        let want_moving = usize::from(file.starts_with("05")) * 2;
        assert_eq!(moving, want_moving, "{file}");
    }
}
