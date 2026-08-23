//! Scratch sweep: every material of every `/data/ships/**.rcsmodel` on the HD
//! disc, so "which material names `engine_flame1.gtf`" is answered by the data
//! rather than by the filename.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_flame_refs flame
//! ```
//!
//! The reproducer for the refutation on `docs/rendering/trail-ribbon.md`: no
//! `.rcsmodel` material on the disc names either per-livery `engine_flame*`
//! file, and every `engineflare.rcsmodel` names a `flame_01.gtf` instead. Its
//! stated limit is that it reads materials **embedded in `.rcsmodel`s**, not
//! standalone `.rcsmaterial` files.
//!
//! It also prints a disc-wide census of the per-instance **parameter table**
//! `oag_formats::rcsmodel::material::parameters` reads, which is what turns
//! "the layout was validated on two files" into a number.
use oag_formats::rcsmodel;
fn main() -> anyhow::Result<()> {
    let iso = "data/images/hdfury-ps3-eu-dec.iso";
    let needle = std::env::args().nth(1).unwrap_or_else(|| "flame".into());
    let (mut models, mut materials, mut with_parameters, mut parameters) = (0u32, 0u32, 0u32, 0u32);
    // A wrong pointer reads whatever bytes are there as floats, so "every value
    // is finite and small" is the cheap check that the walk is on the table
    // rather than beside it.
    let mut implausible = 0u32;
    for a in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{iso}:PS3_GAME/USRDIR/{a}.PSARC");
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            println!("{a}: will not open");
            continue;
        };
        let names: Vec<String> = archive
            .paths()
            .iter()
            .filter(|n| n.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for name in names {
            let Ok(blob) = archive.read_path(&name) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            models += 1;
            for m in &model.materials {
                materials += 1;
                if !m.parameters.is_empty() {
                    with_parameters += 1;
                    parameters += u32::try_from(m.parameters.len()).unwrap_or(0);
                    implausible += u32::try_from(
                        m.parameters
                            .iter()
                            .filter(|p| p.value.iter().any(|v| !v.is_finite() || v.abs() > 1.0e6))
                            .count(),
                    )
                    .unwrap_or(0);
                }
                let hit = m.texture.contains(&needle)
                    || m.second_texture
                        .as_deref()
                        .is_some_and(|t| t.contains(&needle))
                    || m.name.contains(&needle);
                if hit {
                    println!(
                        "{a} {name}\n    material {} -> {} / {:?}",
                        m.name, m.texture, m.second_texture
                    );
                }
            }
        }
    }
    println!(
        "{models} .rcsmodel(s), {materials} material(s), {with_parameters} with a \
         parameter table, {parameters} parameter(s) in all, {implausible} of them not \
         a finite number under 1e6"
    );
    Ok(())
}
