//! Which of a circuit's authored sound emitters reach its magstrip.
//!
//! The question it answers is on
//! `docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md`: is the hum a player
//! hears on a magstrip a per-craft cue, or a track emitter that happens to
//! sit beside the strip? For every track file it gathers the `MagFloor`
//! collision vertices in world space and, for every `sound`/`soundcone`
//! node, the distance from the emitter to the nearest of them, then prints
//! the emitters whose radius covers any of the strip.
//!
//! ```sh
//! cargo run -p oag-vex --example magfloor_emitter_reach -- data/images/pulse-psp-usa.chd
//! ```

use oag_assets::Archive;
use oag_vex::{collision, sound_emitters, vex};

const TRACK_FILES: &[&str] = &["track.vex", "track_reversed.vex"];

fn transform(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    // Row-vector convention, translation in row 3: the layout `world_transforms` returns.
    [
        p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12],
        p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13],
        p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14],
    ]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn main() {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".into());
    let mut archive =
        Archive::open(&format!("{image}:PSP_GAME/USRDIR/Data.wad")).expect("open Data.wad");
    // The two effect models themselves: a sound node authored inside either
    // would be a per-craft cue the code never has to name.
    for name in [
        "Data\\visual_effects\\MagEffect1.vex",
        "Data\\visual_effects\\MagEffect2.vex",
    ] {
        let model = archive.read_name(name).expect("effect model");
        let nodes = vex::nodes(&model).expect("nodes");
        let classes: Vec<u32> = nodes.iter().map(|n| n.class_id).collect();
        println!(
            "{name}: {} nodes, classes {classes:#x?}, {} sound emitters",
            nodes.len(),
            sound_emitters::emitters(&model, &nodes).len()
        );
    }
    for track in 1..=16 {
        for file in TRACK_FILES {
            let name = format!("Data\\Environments\\{track:02}_Track\\{file}");
            let Ok(model) = archive.read_name(&name) else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&model) else {
                continue;
            };
            let world = vex::world_transforms(&model, &nodes);
            let strip: Vec<[f32; 3]> = collision::from_vex(&model)
                .unwrap_or_default()
                .into_iter()
                .filter(|n| n.kind == collision::SurfaceKind::MagFloor)
                .flat_map(|n| {
                    let m = world[n.node_index];
                    n.geometry
                        .meshes
                        .iter()
                        .flat_map(|mesh| mesh.vertices.iter().map(|v| transform(&m, *v)))
                        .collect::<Vec<_>>()
                })
                .collect();
            let emitters = sound_emitters::emitters(&model, &nodes);
            let hums = emitters
                .iter()
                .filter(|e| e.cue.to_ascii_lowercase().contains("hum"))
                .count();
            if strip.is_empty() {
                println!("{name}: no MagFloor collision, {hums} *hum* emitters");
                continue;
            }
            println!("{name}: {hums} *hum* emitters in all");
            println!(
                "{name}: {} MagFloor vertices, {} emitters",
                strip.len(),
                emitters.len()
            );
            for e in &emitters {
                let p = e.position();
                let nearest = strip
                    .iter()
                    .map(|v| distance(p, *v))
                    .fold(f32::INFINITY, f32::min);
                if nearest <= e.radius {
                    println!(
                        "  REACHES  {}:{}  radius {:.1}  nearest strip vertex {:.1}",
                        e.bank, e.cue, e.radius, nearest
                    );
                }
            }
        }
    }
}
