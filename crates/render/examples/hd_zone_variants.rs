//! Which variants of a material carry the `ZoneMode` token, and what zone inputs
//! their fragment programs declare.
//!
//! ```sh
//! cargo run -p oag-render --example hd_zone_variants -- materials/mag_effect_loop_opaque.rcsmaterial
//! ```

use oag_rcs::rcsmaterial;

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    let zone_names = [
        "zoneEffect",
        "zoneColourTint",
        "zoneBase",
        "zoneBaseAlt",
        "zoneOrigin",
        "zoneEffectInner",
        "zoneEffectOuter",
    ];
    let zone_samplers = [
        "zoneTexInner",
        "zoneAnisoPalette",
        "zoneTexVis",
        "zoneTexOuter",
    ];
    for archive in ["DATA00", "DATA01", "DATA02", "DATA03"] {
        let Ok(mut open) =
            oag_assets::psarc::Archive::open(&format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC"))
        else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| wanted.iter().any(|w| p.ends_with(w.as_str())))
            .cloned()
            .collect();
        for path in paths {
            let blob = open.read_path(&path)?;
            let m = rcsmaterial::RcsMaterial::parse(&blob)?;
            println!("{archive} {path}: {} variants", m.variants.len());
            for (i, v) in m.variants.iter().enumerate() {
                let Some(d) = rcsmaterial::Declared::parse(&blob, v.fragment.offset) else {
                    continue;
                };
                let zp: Vec<&str> = zone_names
                    .iter()
                    .copied()
                    .filter(|n| d.parameters.contains(&rcsmaterial::name_hash(n)))
                    .collect();
                let zs: Vec<&str> = zone_samplers
                    .iter()
                    .copied()
                    .filter(|n| {
                        d.samplers
                            .iter()
                            .any(|&(s, _)| s == rcsmaterial::name_hash(n))
                    })
                    .collect();
                let class = v.class.map_or("?".to_string(), |c| c.name().to_string());
                let has = |n: &str| {
                    d.samplers
                        .iter()
                        .any(|&(s, _)| s == rcsmaterial::name_hash(n))
                };
                let wave =
                    has("ds_mag_wave_c") || d.samplers.iter().any(|&(s, _)| s == 0x85c9_fd48);
                let emissive = d.samplers.iter().any(|&(s, _)| s == 0x1202_d8df);
                let ramp = d.samplers.iter().any(|&(s, _)| s == 0xcc98_c527);
                let time = d.parameters.contains(&0x906b_67ba);
                println!(
                    "  #{i} class {class} feat {:#010x} zoneParams {zp:?} zoneSamplers {zs:?} wave {wave} emissive {emissive} ramp {ramp} time {time}",
                    v.feature_hash
                );
            }
        }
    }
    Ok(())
}
