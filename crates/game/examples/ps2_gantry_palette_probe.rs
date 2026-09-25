//! Dumps the PS2 start gantry's own palette texture as ASCII, resolved
//! through the same external-texture-set rule
//! `crate::race::gantry::ps2_skin` applies at load. `#` is opaque white
//! (a lit texel), `r` any other opaque colour, `.` alpha under 50.
//!
//! Written for `ps2_start_gantry_ground_truth.rs`'s own size assertion: the
//! PS2 disc's `321go_NOMIP.tga` measures 32x32 against PSP's 16x32, and this
//! is how that was read off the real texels rather than guessed - kept in
//! the tree the way `gantry_2048_probe.rs` is, for the next person who needs
//! to look at a staircase texture by eye instead of through an assertion.
//!
//! ```sh
//! cargo run -p oag-game --example ps2_gantry_palette_probe
//! OAG_IMAGE=data/images/pulse-ps2-eu.chd cargo run -p oag-game --example ps2_gantry_palette_probe
//! ```

use oag_render::mesh;
use oag_vex::vex;

const GANTRY: &str = r"Data\Environments\321_Go\321Go_StartFinish.vex";

fn main() {
    let path =
        std::env::var("OAG_IMAGE").unwrap_or_else(|_| "data/images/pulse-ps2-eu.chd".to_string());
    let mut archives = oag_pulse::open(&path).expect("open as Pulse");
    let data = archives.read_name(GANTRY).expect("the gantry resolves");
    let preceding = archives
        .read_preceding(GANTRY)
        .expect("the preceding archive entry reads");
    let set = mesh::Ps2TextureSet::parse(&preceding).expect("a texture set");

    let nodes = vex::nodes(&data).expect("parses");
    let classes = vex::classes_of(&data).expect("classes");
    for node in nodes.iter().filter(|n| Some(n.class_id) == classes.texture) {
        let name = vex::texture_asset_path(&data[node.payload()]).unwrap_or_default();
        let Some(tex) = set.resolve(&name) else {
            println!("{name}: not resolved");
            continue;
        };
        println!("{name}: {}x{}", tex.width, tex.height);
        if name.to_lowercase().contains("321go") {
            let rgba = tex.rgba().expect("rgba");
            for y in 0..tex.height {
                let mut row = String::new();
                for x in 0..tex.width {
                    let at = ((y * tex.width + x) * 4) as usize;
                    let (r, g, b, a) = (rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]);
                    let c = if a < 50 {
                        '.'
                    } else if r > 200 && g > 200 && b > 200 {
                        '#'
                    } else {
                        'r'
                    };
                    row.push(c);
                }
                println!("{y:3} {row}");
            }
        }
    }
}
