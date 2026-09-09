//! Scratch probe: what mechanism drives Pulse's HUD countdown models,
//! `Data\HUD\Pulse_Ready_Go.vex` and `Data\HUD\Cockpit_321GO.vex`.
//!
//! Not wired into anything; run by hand while working the race-start
//! countdown state machine and launch reaction boost.
//!
//! ```sh
//! cargo run -p oag-render --example ready_go_census
//! ```

use oag_vex::vex;

const ENTRIES: &[&str] = &[
    "Data\\HUD\\Pulse_Ready_Go.vex",
    "Data\\HUD\\Cockpit_321GO.vex",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/pulse-psp-usa.chd".into());
    let spec = format!("{image}:PSP_GAME/USRDIR/Data.wad");
    let mut archive = oag_assets::Archive::open(&spec)?;

    for entry in ENTRIES {
        println!("== {entry} ==");
        let blob = archive.read_name(entry)?;
        for texture in vex::textures(&blob)?.into_iter().flatten() {
            println!(
                "  texture {:?} {}x{} bpp {} mips {}",
                texture.asset_path,
                texture.width,
                texture.height,
                texture.bits_per_pixel,
                texture.mip_count
            );
            let rgba = texture.to_rgba();
            for y in 0..texture.height as usize {
                let row: Vec<String> = (0..texture.width as usize)
                    .map(|x| {
                        let i = (y * texture.width as usize + x) * 4;
                        format!(
                            "{:02x}{:02x}{:02x}{:02x}",
                            rgba[i],
                            rgba[i + 1],
                            rgba[i + 2],
                            rgba[i + 3]
                        )
                    })
                    .collect();
                println!("    row {y:>2}: {}", row.join(" "));
            }
        }
        let nodes = vex::nodes(&blob)?;
        let anims = vex::anim_transforms(&blob, &nodes);
        for (node, anim) in nodes.iter().zip(anims.iter()) {
            if node.class_id == vex::CLASS_ANIM_TRANSFORM {
                match anim {
                    Some(at) => {
                        println!(
                            "  anim-transform {:?}: loop {:.3}s step {}",
                            node.name, at.loop_seconds, at.step
                        );
                        println!(
                            "    translation base {:?} quantum {:?} times {:?} values {:?}",
                            at.translation_base,
                            at.translation_quantum,
                            at.translation.times,
                            at.translation.values
                        );
                        println!(
                            "    rotation times {:?} values {:?}",
                            at.rotation.times, at.rotation.values
                        );
                        println!(
                            "    scale times {:?} values {:?}",
                            at.scale.times, at.scale.values
                        );
                    }
                    None => println!("  anim-transform {:?}: no track", node.name),
                }
                continue;
            }
            if node.class_id != vex::CLASS_MESH {
                continue;
            }
            let payload = &blob[node.payload()];
            let transforms = vex::mesh_tex_transforms(payload);
            println!("  mesh {:?}: {} material(s)", node.name, transforms.len());
            for (i, t) in transforms.iter().enumerate() {
                match t {
                    Some(t) => {
                        println!(
                            "    material {i}: loop {:.3}s step {} seconds_per_key {:.5}",
                            t.loop_seconds, t.step, t.seconds_per_key
                        );
                        println!(
                            "      offset times {:?} values {:?}",
                            t.offset.times, t.offset.values
                        );
                        println!(
                            "      scale  times {:?} values {:?}",
                            t.scale.times, t.scale.values
                        );
                    }
                    None => println!("    material {i}: no texture-transform block"),
                }
            }
        }
    }
    Ok(())
}
