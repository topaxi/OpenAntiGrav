//! Scratch: decode and print every drawn material a ship `.vex` resolves -
//! the "what does the resolved variant actually compute" read behind
//! `docs/rendering/hd-ship-materials.md`.
//!
//! ```sh
//! cargo run -p oag-render --example hd_ship_material_dump -- data/images/hdfury-ps3-eu-dec.iso /data/ships/feisar_c1/ship.vex
//! cargo run -p oag-render --example hd_ship_material_dump -- data/images/hdfury-ps3-eu-dec.iso /data/ships/feisar_c1/ship.vex diffuse_with_specular_from_alpha_n_vcol.rcsmaterial
//! ```
//!
//! With a third argument (a material's leaf file name), only that one
//! material prints its full instruction listing; otherwise every resolved
//! material on the `.vex` prints its summary row only.
use oag_mesh::mesh;
use oag_rcs::{
    rcsmaterial::{self, Class, Declared, LIT_RACE_PASS, fragment::Program, vertex},
    rcsmodel,
};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

/// `~crc32("VertexColour1")` - see `oag_rcs::rcsmodel::VertexDecl`'s own doc
/// comment for the attribute this hash names and why it is not HD's baked
/// colour set.
const VERTEX_COLOUR_1: u32 = 0x7493_d450;

fn find_blob(image: &str, path: &str) -> Option<(String, Vec<u8>)> {
    ARCHIVES.iter().find_map(|archive| {
        let spec = format!("{image}:{archive}");
        mesh::read_blob(&spec, path).ok().map(|d| (spec, d))
    })
}

/// Every interpolator a fragment program's `input` field can name - see
/// `rcsmaterial::fragment::Instruction::input`'s own doc comment.
fn interpolator_name(bit: u8) -> String {
    match bit {
        0 => "POS".into(),
        1 => "COL0".into(),
        2 => "COL1".into(),
        3 => "FOGC".into(),
        0xe => "FACING".into(),
        n if n >= 4 => format!("TC{}", n - 4),
        n => format!("f[{n}]"),
    }
}

fn bitmask_names(mask: u16) -> Vec<String> {
    (0..16u8)
        .filter(|b| mask & (1 << b) != 0)
        .map(interpolator_name)
        .collect()
}

fn print_texel(label: &str, texel: rcsmaterial::fragment::Texel) {
    match texel {
        rcsmaterial::fragment::Texel::Untraced => println!("    {label}: untraced"),
        rcsmaterial::fragment::Texel::Unit { unit, channel } => println!(
            "    {label}: unit {unit}{}",
            channel.map_or(String::new(), |c| format!(
                ".{}",
                "xyzw".as_bytes()[c as usize] as char
            ))
        ),
        rcsmaterial::fragment::Texel::Mixed => println!("    {label}: mixed units"),
    }
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let vex = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/ships/feisar_c1/ship.vex".into());
    let only = std::env::args().nth(3);

    let Some((spec, data)) = find_blob(&image, &vex) else {
        anyhow::bail!("{vex}: not found");
    };
    let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &vex, &data) else {
        anyhow::bail!("{vex}: no sibling .rcsmodel");
    };
    let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
        Default::default();
    let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
    for mesh in &model.meshes {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        *chunks_of.entry(mesh.material).or_default() += 1;
    }

    for (slot, material) in model.materials.iter().enumerate() {
        let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
        let chunks = chunks_of.get(&ordinal).copied().unwrap_or(0);
        if chunks == 0 {
            continue;
        }
        let leaf = material.name.rsplit('/').next().unwrap_or(&material.name);
        if let Some(only) = &only
            && leaf != only
        {
            continue;
        }
        let Some((_, blob)) = find_blob(&image, &format!("/{}", material.name)) else {
            println!("{} [{slot}]: material file not found", material.name);
            continue;
        };
        let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
            println!(
                "{} [{slot}]: does not parse as an .rcsmaterial",
                material.name
            );
            continue;
        };
        let decl = decl_of.get(&ordinal).copied().flatten();
        let word = rcsmaterial::Features::chunk_word(LIT_RACE_PASS, decl);
        let key = rcsmaterial::Features::from_pass_word(word);
        let Some((class, variant)) = Class::ALL
            .into_iter()
            .find_map(|c| parsed.variant(c, key).map(|v| (c, v)))
        else {
            println!(
                "{} [{slot}] ({chunks} chunk(s)): does not resolve {} under any class",
                material.name,
                key.name()
            );
            continue;
        };

        println!(
            "== {} [{slot}] ({chunks} chunk(s)) - {class:?}/{} @ {:#x} ==",
            material.name,
            key.name(),
            variant.fragment.offset
        );
        // The model's own sampler table: (hash, path), by ordinal.
        for (i, (hash, path)) in material.samplers.iter().enumerate() {
            println!(
                "  model sampler[{i}]: {hash:#010x} -> {}",
                path.as_deref().unwrap_or("(none)")
            );
        }
        // The model's own authored parameter table - what `emissive()` reads
        // for `TINT`/`OFFSET`/`SCALE` (`0xe8bcd7f5`/`0x78256a45`/`0x78787596`)
        // and `roles()` for `SpecularPower` (`0x81e0e773`).
        for p in &material.parameters {
            println!("  model param {:#010x}: {:?}", p.hash, p.value);
        }

        let declared = Declared::parse(&blob, variant.fragment.offset);
        if let Some(d) = &declared {
            println!(
                "  declared: {} sampler(s) {:?}, {} parameter(s)",
                d.samplers.len(),
                d.samplers,
                d.parameters.len()
            );
            println!(
                "  takes_constant_ambient={} takes_directional_light={} samples_lightmap={}",
                d.takes_constant_ambient(),
                d.takes_directional_light(),
                d.samples_lightmap()
            );
        }

        if let Some(program) = Program::parse(&blob, variant.fragment.offset) {
            let texels = program.output_texels();
            println!("  output_texels:");
            print_texel("x", texels[0]);
            print_texel("y", texels[1]);
            print_texel("z", texels[2]);
            print_texel("w (alpha)", texels[3]);
            println!(
                "  interpolators (addressed): {:?}",
                bitmask_names(program.interpolators())
            );
            println!(
                "  output_lit_by (reach output as a value): {:?}",
                bitmask_names(program.output_lit_by())
            );
            if let Some(d) = &declared {
                for &(hash, unit) in &d.samplers {
                    println!(
                        "  accumulates(unit {unit}, sampler {hash:#010x}) = {}",
                        program.accumulates(unit as u8)
                    );
                }
            }
            println!(
                "  specular_exponent: {:?} (slot {:?})",
                program.specular_exponent(),
                program.specular_exponent_slot()
            );
            if only.is_some() {
                println!("  {} instruction(s):", program.instructions.len());
                for (i, insn) in program.instructions.iter().enumerate() {
                    println!("    #{i} {:?} name={:?}", insn, insn.name());
                }
            }
        }

        // The vertex half: where VertexColour1 lands, if anywhere.
        if let Some(vprog) = vertex::Program::of(&blob, variant.vertex) {
            match vprog.attribute_slot(VERTEX_COLOUR_1) {
                Some(v_slot) => {
                    println!("  vertex: VertexColour1 arrives in v[{v_slot}]");
                    let writes: Vec<String> = vprog
                        .instructions
                        .iter()
                        .filter(|i| {
                            u32::from(i.input) == v_slot
                                && i.dest_is_output
                                && i.sources.iter().any(|&s| s & 3 == 2 /* SOURCE_INPUT */)
                        })
                        .map(|i| format!("o[{}]", i.dest))
                        .collect();
                    println!("  vertex: VertexColour1 read into output register(s): {writes:?}");
                }
                None => println!("  vertex: does not declare VertexColour1"),
            }
        }
        println!();
    }
    Ok(())
}
