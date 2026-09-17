//! Ad hoc: does a `.vex` file author any `AmbientLight`/`DirectionalLight`/
//! `PointLight` nodes, and how many.
//!
//! One-off diagnostic for `lane/hd-ambient-light`'s Amphiseum ceiling-hue
//! investigation - checking whether HD/Fury's own `.vex` files carry the
//! same light-rig classes Pulse's do (`docs/formats/lighting.md`), since
//! `crates/render`/`crates/game` implement no dynamic point-light term at
//! all. Not wired into anything; run with:
//!
//! ```sh
//! cargo run -p oag-render --example light_census -- \
//!   data/images/hdfury-ps3-eu-dec.iso amphiseum
//! ```

use oag_vex::vex;

const AMBIENT: u32 = vex::CLASS_AMBIENT_LIGHT;
const DIRECTIONAL: u32 = vex::CLASS_DIRECTIONAL_LIGHT;
const POINT: u32 = vex::CLASS_POINT_LIGHT;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("usage: light_census <image> <circuit>");
    let circuit = args.next().expect("usage: light_census <image> <circuit>");

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
            .filter(|p| p.contains(&format!("/environments/{circuit}/")) && p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&bytes) else {
                continue;
            };
            let ambient = nodes.iter().filter(|n| n.class_id == AMBIENT).count();
            let directional = nodes.iter().filter(|n| n.class_id == DIRECTIONAL).count();
            let point = nodes.iter().filter(|n| n.class_id == POINT).count();
            println!(
                "{path}: {} nodes total, {ambient} AmbientLight, {directional} DirectionalLight, {point} PointLight",
                nodes.len()
            );
        }
    }
}
