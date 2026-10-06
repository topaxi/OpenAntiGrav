//! What the 2048 scroll plan reaches per circuit: glow layers, plain scrolls,
//! and the scrolls inherited from HD's vertex law.
//!
//! ```sh
//! cargo run -p oag-render --example psp2_scroll_reach -- <package.psarc>...
//! ```

use oag_mesh::mesh::rcs::psp2;

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let mut archive = oag_assets::psarc::Archive::open(&path)?;
        let tracks: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                p.to_ascii_lowercase().ends_with("/track.rcsmodel")
                    || p.to_ascii_lowercase().ends_with("/trackzone.rcsmodel")
                    || p.to_ascii_lowercase().contains("/track") && p.ends_with(".rcsmodel")
            })
            .cloned()
            .collect();
        for track in tracks {
            let base = track.trim_end_matches(".rcsmodel").to_string();
            let model = archive.read_path(&track)?;
            let skeleton = archive.read_path(&format!("{base}.rcsskeleton")).ok();
            let clip = archive.read_path(&format!("{base}.rcsanimclip")).ok();
            let animation = skeleton
                .as_deref()
                .and_then(|s| psp2::Animation::parse(s, clip.as_deref()).ok());
            let Ok((model, report)) = psp2::build(&track, &model, animation.as_ref(), &mut |p| {
                archive.read_path(p).ok()
            }) else {
                continue;
            };
            println!(
                "{track}: glow {} plain {} inherited {} unread {}",
                report.glow_layers,
                report.scrolling_materials,
                report.inherited_scrolls,
                report.inherited_unread
            );
            if std::env::var_os("OAG_SCROLL_DRAWS").is_some() {
                for d in model.draws.iter().filter(|d| !d.range.is_empty()) {
                    let v = model.vertices[model.indices[d.range.start as usize] as usize];
                    let glow = v.slots & oag_mesh::mesh::slots::ADD_SECOND != 0;
                    if v.anim != 0 || glow {
                        println!(
                            "  draw {} tris {} centre {:?} radius {:.1}",
                            if glow { "glow" } else { "scroll" },
                            d.range.len() / 3,
                            d.bounds.centre,
                            d.bounds.radius
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
