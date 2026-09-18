//! Does any `.vex` node on Amphiseum sit at the world-space position the
//! live SPU vertex-light buffer holds stationary across five snapshots -
//! `(-15.340508, -50.138016, -176.605759)`
//! (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`Enable_spu_vertex_
//! light` is read at 14 sites...", the live-capture addendum)?
//!
//! If a `Section`/`WeaponPad`/`SpeedupPad`/`WOPoint`/anything node's own
//! world transform lands there, the buffer is authored markers, not a
//! computed light list, and the "SPU vertex light" reading collapses. If
//! nothing on the circuit sits there, that is evidence the position is
//! computed rather than read off an authored node - checked, not assumed.
//!
//! Sweeps every node of every class in every `.vex` under `amphiseum/`,
//! the same file set `light_census.rs` already swept for `PointLight`.

use oag_vex::vex;

const TARGET: [f32; 3] = [-15.340508, -50.138016, -176.605_76];
const EPSILON: f32 = 5.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .expect("usage: hd_amphiseum_spu_light_marker_check <image>");

    let mut checked = 0usize;
    let mut closest: Option<(f32, String, u32, [f32; 3])> = None;

    for archive in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.contains("/environments/amphiseum/") && p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&bytes) else {
                continue;
            };
            let transforms = vex::world_transforms(&bytes, &nodes);
            for (node, m) in nodes.iter().zip(transforms.iter()) {
                checked += 1;
                let pos = [m[12], m[13], m[14]];
                let d2 = (0..3).map(|i| (pos[i] - TARGET[i]).powi(2)).sum::<f32>();
                let d = d2.sqrt();
                if closest.as_ref().is_none_or(|(best, ..)| d < *best) {
                    closest = Some((
                        d,
                        format!("{path} class {:#x}", node.class_id),
                        node.class_id,
                        pos,
                    ));
                }
                if d < EPSILON {
                    println!(
                        "MATCH within {EPSILON}: {path} class {:#x} at ({:.3}, {:.3}, {:.3}), distance {:.4}",
                        node.class_id, pos[0], pos[1], pos[2], d
                    );
                }
            }
        }
    }
    println!("checked {checked} node transforms across Amphiseum's .vex files");
    if let Some((d, label, _class, pos)) = closest {
        println!(
            "closest of all: {label} at ({:.3}, {:.3}, {:.3}), distance {:.4}",
            pos[0], pos[1], pos[2], d
        );
    }
}
