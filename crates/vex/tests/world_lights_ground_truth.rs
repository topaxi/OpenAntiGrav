//! The point lights Pulse's circuits place, and the two that move.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! `WoPoint_Init` (`0x0892ca04`) walks a `wopoint`'s ancestors for an `Anim
//! Transform`, and its update slot (`0x0892cd00`) copies the node's world translation
//! into the runtime object every frame: a light under one moves with it. Arc Prime
//! authors two, the white pair that sweep with its floor spotlights, in both
//! layouts, and no other Pulse circuit authors any. See
//! `docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "A light that moves".

use oag_vex::vex::{self, Node};
use oag_vex::world_lights::{self, CLASS_WOPOINT, CLASS_WOSPOT, PlacedLight};

/// Every circuit the PSP disc carries, each in both layouts.
const CIRCUITS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "09_Track",
    "10_Track", "13_Track", "14_Track", "16_Track",
];

const LAYOUTS: &[&str] = &["track.vex", "track_reversed.vex"];

struct Circuit {
    blob: Vec<u8>,
    nodes: Vec<Node>,
}

impl Circuit {
    fn lights(&self) -> Vec<PlacedLight> {
        world_lights::placed_lights(&self.blob, &self.nodes)
    }

    /// The node index of each light of [`Self::lights`], in the same order: every
    /// `wopoint`, then every `wospot`.
    fn light_nodes(&self) -> Vec<usize> {
        [CLASS_WOPOINT, CLASS_WOSPOT]
            .into_iter()
            .flat_map(|class| {
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(move |(_, node)| node.class_id == class)
                    .map(|(index, _)| index)
            })
            .collect()
    }
}

fn open(circuit: &str, layout: &str) -> Option<Circuit> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(&format!(r"Data\Environments\{circuit}\{layout}"))
        .ok()?;
    let nodes = vex::nodes(&blob).expect("nodes");
    Some(Circuit { blob, nodes })
}

/// **Arc Prime's two white floor lights are the only lights on the disc that move**,
/// in both layouts.
#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn arc_primes_two_floor_lights_are_the_only_ones_that_move() {
    for circuit in CIRCUITS {
        for layout in LAYOUTS {
            let Some(file) = open(circuit, layout) else {
                continue;
            };
            let moving = file.lights().iter().filter(|l| l.moves()).count();
            let want = if *circuit == "10_Track" { 2 } else { 0 };
            assert_eq!(moving, want, "{circuit}\\{layout}");
        }
    }
}

/// A light placed at a time is where the whole tree's `world_transforms_at` puts its
/// node then: the chain it keeps is the same product, for every light on the disc.
#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn a_placed_light_composes_like_the_scenery_it_sits_in() {
    let mut checked = 0;
    for circuit in CIRCUITS {
        for layout in LAYOUTS {
            let Some(file) = open(circuit, layout) else {
                continue;
            };
            let lights = file.lights();
            let nodes = file.light_nodes();
            assert_eq!(lights.len(), nodes.len(), "{circuit}\\{layout}");
            for seconds in [0.0, 0.5, 2.0, 3.7, 12.25] {
                let world = vex::world_transforms_at(&file.blob, &file.nodes, seconds);
                for (light, node) in lights.iter().zip(&nodes) {
                    let at = light.at(seconds).position;
                    let want = [world[*node][12], world[*node][13], world[*node][14]];
                    for (g, w) in at.iter().zip(want) {
                        assert!(
                            (g - w).abs() < 1e-3,
                            "{circuit}\\{layout} node {node} at {seconds}: {at:?} vs {want:?}"
                        );
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 100, "{checked} light placements checked");
}

/// The sweep, read off the file: the second floor light rests at `(-162.2, 6.8, -219.8)`
/// and is 86 units away, outside its 55-unit reach, by two seconds in.
#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn the_floor_light_has_swept_away_from_where_it_rests_two_seconds_in() {
    let Some(file) = open("10_Track", "track.vex") else {
        return;
    };
    let lights = file.lights();
    let sweeping: Vec<&PlacedLight> = lights.iter().filter(|l| l.moves()).collect();
    assert_eq!(sweeping.len(), 2);
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.1);
    assert!(near(sweeping[0].at(0.0).position, [-472.6, 6.8, -232.8]));
    assert!(near(sweeping[1].at(0.0).position, [-162.2, 6.8, -219.8]));
    assert!(near(sweeping[1].at(2.0).position, [-248.0, 17.9, -227.6]));
    // The same light, the same colour and reach, wherever the clock has put it.
    assert_eq!(
        sweeping[1].at(2.0).outer_radius,
        sweeping[1].at(0.0).outer_radius
    );
    assert_eq!(sweeping[1].at(2.0).colour, sweeping[1].at(0.0).colour);
}
