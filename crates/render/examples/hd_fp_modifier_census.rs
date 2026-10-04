//! Disc-wide census of the fragment decoder's source modifiers (negate), behind `docs/rendering/pads.md`'s pad chain: the decoder dropped the
//! negate bit until 2026-10-04, which made the pads' `EX2(-(d*k)^2)` fog
//! curve read as a saturate that is always 1. This counts, over every
//! fragment block on the disc, how often each (mnemonic, source slot)
//! carries the negate. An `abs` decode was tried and dropped: NV40's
//! `SRC0_ABS`/`SRC1_ABS`/`SRC2_ABS` bit positions, read from memory of
//! `nvfx_shader.h`, came out on 22% of ADD src0 and 0% of MUL src0 yet 4% of
//! MUL src1, which is not what an abs looks like, so no abs is claimed.

use std::collections::BTreeMap;

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::Program;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut table: BTreeMap<(String, usize, &str), usize> = BTreeMap::new();
    let mut total: BTreeMap<(String, usize), usize> = BTreeMap::new();
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
                    let name = insn
                        .name()
                        .map_or_else(|| format!("op{:#04x}", insn.opcode), str::to_string);
                    for slot in 0..insn.arity() {
                        *total.entry((name.clone(), slot)).or_default() += 1;
                        if insn.negate[slot] {
                            *table.entry((name.clone(), slot, "neg")).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    for ((name, slot, kind), n) in &table {
        let of = total[&(name.clone(), *slot)];
        println!("{name:8} src{slot} {kind}: {n} of {of}");
    }
    Ok(())
}
