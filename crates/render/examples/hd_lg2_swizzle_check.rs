//! Scratch check: is `LG2`'s own read swizzle always uniform (a true
//! broadcast of one lane), or does it vary across the four positions? Feeds
//! whether `hd_specular_unresolved_reasons.rs`'s `read_mask` doc comment
//! ("reads one lane... broadcast") matches what the code actually computes
//! (an OR across every masked output lane's swizzle entry, which only
//! collapses to a single lane when the swizzle is uniform).

use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::Program;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut total = 0usize;
    let mut nonuniform = 0usize;
    for archive in ARCHIVES {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(mut handle) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = handle
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handle.read_path(&path) else {
                continue;
            };
            let Ok(material) = RcsMaterial::parse(&data) else {
                continue;
            };
            for variant in &material.variants {
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    continue;
                };
                for insn in &program.instructions {
                    if insn.name() == Some("LG2") {
                        total += 1;
                        let sw = insn.swizzles[0];
                        if sw[0] != sw[1] || sw[0] != sw[2] || sw[0] != sw[3] {
                            nonuniform += 1;
                        }
                    }
                }
            }
        }
    }
    println!("{total} LG2 instructions, {nonuniform} with non-uniform swizzle");
    Ok(())
}
