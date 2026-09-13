//! Scratch: decode and print one ship material's resolved fragment program
//! and its declaration, for the "what does the newly-resolved variant
//! actually compute" read in the hull-lighting handover thread.
use oag_rcs::rcsmaterial::{self, Class, Declared, Features, LIT_RACE_PASS, fragment::Program};
use oag_render::mesh;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let path = std::env::args().nth(2).unwrap_or_else(|| {
        "/data/materials/ships/diffuse_with_specular_from_alpha_n_vcol.rcsmaterial".into()
    });

    let Some(blob) = ARCHIVES.iter().find_map(|archive| {
        let spec = format!("{image}:{archive}");
        mesh::read_blob(&spec, &path).ok()
    }) else {
        anyhow::bail!("{path}: not found");
    };
    let parsed = rcsmaterial::RcsMaterial::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    // The key the ship's chunks actually resolve now that `VertexColour1`
    // no longer counts as a colour set: `Ambient`, no lightmap, no colour
    // set - `Features::chunk_word(LIT_RACE_PASS, None)`.
    let key = Features::from_pass_word(Features::chunk_word(LIT_RACE_PASS, None));
    println!("wanted key: {} ({:#010x})", key.name(), key.hash());
    for class in Class::ALL {
        let Some(variant) = parsed.variant(class, key) else {
            continue;
        };
        println!(
            "== {class:?} fragment @ {:#x} (hash {:#x}) ==",
            variant.fragment.offset, variant.fragment.program_hash
        );
        let declared = Declared::parse(&blob, variant.fragment.offset);
        if let Some(d) = &declared {
            println!(
                "declared: {} sampler(s), {} parameter(s)",
                d.samplers.len(),
                d.parameters.len()
            );
            println!("  takes_constant_ambient: {}", d.takes_constant_ambient());
            println!("  takes_directional_light: {}", d.takes_directional_light());
            println!("  samples_lightmap: {}", d.samples_lightmap());
            for &hash in &d.parameters {
                println!("  parameter {hash:#010x}");
            }
        }
        if let Some(program) = Program::parse(&blob, variant.fragment.offset) {
            println!("  specular_exponent: {:?}", program.specular_exponent());
            println!(
                "  specular_exponent_slot: {:?}",
                program.specular_exponent_slot()
            );
            println!("  {} instruction(s):", program.instructions.len());
            for (i, insn) in program.instructions.iter().enumerate() {
                println!("    #{i} {:?} name={:?}", insn, insn.name());
            }
        }
        break;
    }
    Ok(())
}
