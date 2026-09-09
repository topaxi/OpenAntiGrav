//! Scratch probe: what do a `321Go` gantry file's `Anim Transform` nodes
//! actually do (translation/rotation/scale keys and sampled values, plus an
//! approximate scale magnitude read off the composed matrix), not just their
//! key times.
//!
//! Defaults to 2048's own `321Go_2048.vex`; pass an archive spec and a node
//! list to point it at a different file - e.g. Wipeout HD's own inherited
//! copy, to check a claim against the source rather than 2048's re-export of
//! it:
//!
//! ```sh
//! cargo run -q -p oag-game --example gantry_2048_anim_probe -- \
//!     data/extracted/vita/PCSF00007/base/PSP2/data.psarc \
//!     data/billboards/hd_adverts/321go/321go_2048.vex \
//!     Three Two One GO
//!
//! cargo run -q -p oag-game --example gantry_2048_anim_probe -- \
//!     "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC" \
//!     /data/billboards/hd_adverts/321go/321go_startfinish.vex \
//!     polySurface7 pasted__Final_Lap pasted__Go_HD_start_light_321go
//! ```

use oag_vex::vex;

const DEFAULT_VEX: &str = "data/billboards/hd_adverts/321go/321go_2048.vex";

const DEFAULT_NAMES: &[&str] = &[
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

/// Approximate `(x, y, z)` scale off a composed row-major-in-translation
/// matrix, by the length of each basis column - exact for a pure
/// scale/rotate/translate composition with no shear, which is what an
/// authored `Anim Transform` node is.
fn approx_scale(m: &[f32; 16]) -> [f32; 3] {
    let col = |i: usize| -> f32 { (m[i].powi(2) + m[i + 1].powi(2) + m[i + 2].powi(2)).sqrt() };
    [col(0), col(4), col(8)]
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base/PSP2/data.psarc".to_string());
    let vex_path = args.next().unwrap_or_else(|| DEFAULT_VEX.to_string());
    let names: Vec<String> = args.collect();
    let names: Vec<&str> = if names.is_empty() {
        DEFAULT_NAMES.to_vec()
    } else {
        names.iter().map(String::as_str).collect()
    };

    let mut archive = oag_assets::psarc::Archive::open(&path)?;
    let blob = archive.read_path(&vex_path)?;
    let nodes = vex::nodes(&blob)?;
    let classes = vex::classes_of(&blob).ok();
    let anim_class = classes.and_then(|c| c.anim_transform);
    let transform_class = classes.and_then(|c| c.transform);
    println!("anim_transform class: {anim_class:?}, transform class: {transform_class:?}");

    for name in names {
        let Some(node) = nodes.iter().find(|n| n.name.as_deref() == Some(name)) else {
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
        // Sample at, and just either side of, every translation key time and
        // every scale key time - this is what shows a teleport, and what
        // shows whether the scale channel does anything, rather than just
        // printing key times.
        let mut sample_times: Vec<f32> = vec![0.0];
        for &t in anim.translation.times.iter().chain(anim.scale.times.iter()) {
            let secs = f32::from(t) * anim.seconds_per_key;
            sample_times.push((secs - anim.seconds_per_key).max(0.0));
            sample_times.push(secs);
        }
        sample_times.push(6.5);
        sample_times.sort_by(f32::total_cmp);
        sample_times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for t in sample_times {
            let m = anim.sample(t);
            let scale = approx_scale(&m);
            println!(
                "  t={t:.4}s -> translation ({:.3}, {:.3}, {:.3}), scale ({:.3}, {:.3}, {:.3})",
                m[12], m[13], m[14], scale[0], scale[1], scale[2]
            );
        }
    }

    Ok(())
}
