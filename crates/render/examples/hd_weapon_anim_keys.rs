//! Prints every Anim Transform node of HD's weapon detonation models: the
//! raw key times and values of each channel, so the track bound to a
//! material's `UV_offset` (`node + 0xc0`) is read off the file. See
//! `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, 2026-10-05.
//!
//! ```sh
//! cargo run -p oag-render --example hd_weapon_anim_keys
//! ```

use oag_mesh::mesh;

const SPEC: &str = "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC";

const MODELS: &[&str] = &[
    "/data/weapons/hd_plasma_ring.vex",
    "/data/weapons/hd_plasma_sphere.vex",
    "/data/weapons/hd_plasma_halo.vex",
    "/data/weapons/hd_missile_explosion.vex",
    "/data/weapons/hd_bomb_sphere.vex",
    "/data/weapons/hd_bomb_sphere_white.vex",
    "/data/weapons/hd_bomb_sphere_bloomring.vex",
    "/data/weapons/hd_bomb_shockwaves.vex",
    "/data/weapons/hd_bomb_halo.vex",
    "/data/weapons/hd_missile_ball_bloomring.vex",
];

fn main() -> anyhow::Result<()> {
    for name in MODELS {
        let data = mesh::read_blob(SPEC, name)?;
        let classes = oag_vex::vex::classes_of(&data)?;
        let nodes = oag_vex::vex::nodes(&data)?;
        let anims = oag_vex::vex::anim_transforms(&data, &nodes);
        println!("{name}: {} nodes", nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            println!(
                "  node {i} class {:#x} name {:?} children {}",
                node.class_id, node.name, node.child_count
            );
            if Some(node.class_id) != classes.anim_transform {
                continue;
            }
            let Some(a) = &anims[i] else {
                println!("    undecodable");
                continue;
            };
            println!(
                "    unit {} loop {} step {} t_base {:?} t_quantum {:?}",
                a.seconds_per_key,
                a.loop_seconds,
                a.step,
                a.translation_base,
                a.translation_quantum
            );
            for (label, c) in [
                ("translation", &a.translation),
                ("rotation", &a.rotation),
                ("scale", &a.scale),
            ] {
                println!("    {label}: {} keys wide {}", c.times.len(), c.wide);
                for (t, v) in c.times.iter().zip(&c.values) {
                    println!("      t {t:>5}  {v:?}");
                }
            }
        }
    }
    Ok(())
}
