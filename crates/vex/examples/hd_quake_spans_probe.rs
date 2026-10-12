//! Scratch probe: does each HD circuit author a `Quake` span table, and how
//! many records does it carry?
//!
//! ```sh
//! cargo run -q -p oag-vex --example hd_quake_spans_probe
//! ```

use oag_vex::{quake, vex};

fn main() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        println!("skipping: no HD image");
        return;
    };
    for n in 0..7 {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let tracks: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.vex"))
            .cloned()
            .collect();
        for path in tracks {
            let blob = archive.read_path(&path).expect("reads");
            let classes = match vex::classes_of(&blob) {
                Ok(c) => c,
                Err(e) => {
                    println!("DATA0{n} {path}: classes: {e}");
                    continue;
                }
            };
            let Some(class) = classes.quake else {
                println!("DATA0{n} {path}: no quake class id");
                continue;
            };
            let nodes = vex::nodes(&blob).expect("nodes");
            let found: Vec<_> = vex::nodes_by_class(&nodes, class).collect();
            match found.first() {
                None => println!("DATA0{n} {path}: no Quake node"),
                Some(node) => match quake::spans(&blob, node) {
                    Ok(spans) => println!(
                        "DATA0{n} {path}: {} node(s), {} span record(s)",
                        found.len(),
                        spans.len()
                    ),
                    Err(e) => println!("DATA0{n} {path}: spans: {e}"),
                },
            }
        }
    }
}
