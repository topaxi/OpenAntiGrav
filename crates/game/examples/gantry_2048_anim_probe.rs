//! Scratch probe: what do 2048's own `321Go_2048.vex` per-glyph `Anim
//! Transform` nodes actually do (translation/rotation/scale keys and
//! sampled values), not just their key times.
//!
//! ```sh
//! cargo run -q -p oag-game --example gantry_2048_anim_probe -- <base/PSP2/data.psarc>
//! ```

use oag_formats::vex;

const VEX: &str = "data/billboards/hd_adverts/321go/321go_2048.vex";

const NAMES: &[&str] = &[
    "Three",
    "Two",
    "One",
    "GO",
    "start_light_background2",
    "start_light_background3",
    "Stripe_01",
    "Stripe_02",
    "Stripe_03",
    "Final_F",
    "Lap_L",
    "polySurface7",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base/PSP2/data.psarc".to_string());
    let mut archive = oag_assets::psarc::Archive::open(&path)?;
    let blob = archive.read_path(VEX)?;
    let nodes = vex::nodes(&blob)?;
    let classes = vex::classes_of(&blob).ok();
    let anim_class = classes.and_then(|c| c.anim_transform);
    let transform_class = classes.and_then(|c| c.transform);
    println!("anim_transform class: {anim_class:?}, transform class: {transform_class:?}");

    for name in NAMES {
        let Some(node) = nodes.iter().find(|n| n.name.as_deref() == Some(*name)) else {
            println!("=== {name}: NOT FOUND ===");
            continue;
        };
        println!(
            "=== {name} (class {:#x}, depth {}) ===",
            node.class_id, node.depth
        );
        let Some(anim) = vex::anim_transform_of(&blob, node) else {
            println!("  no Anim Transform decode (static Transform node)");
            continue;
        };
        println!(
            "  seconds_per_key {:.5}, translation times {:?}",
            anim.seconds_per_key, anim.translation.times
        );
        println!("  translation_base {:?}", anim.translation_base);
        println!("  translation_quantum {:?}", anim.translation_quantum);
        println!("  rotation times {:?}", anim.rotation.times);
        println!("  scale times {:?}", anim.scale.times);
        // Sample at, and just either side of, every translation key time -
        // this is what actually shows a teleport rather than a smooth move.
        let mut sample_times: Vec<f32> = vec![0.0];
        for &t in &anim.translation.times {
            let secs = f32::from(t) * anim.seconds_per_key;
            sample_times.push((secs - anim.seconds_per_key).max(0.0));
            sample_times.push(secs);
        }
        sample_times.push(6.5);
        sample_times.sort_by(f32::total_cmp);
        sample_times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for t in sample_times {
            let m = anim.sample(t);
            println!(
                "  t={t:.4}s -> translation ({:.3}, {:.3}, {:.3})",
                m[12], m[13], m[14]
            );
        }
    }

    Ok(())
}
