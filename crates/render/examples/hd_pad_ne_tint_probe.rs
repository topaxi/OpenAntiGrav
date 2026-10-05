//! Scratch probe behind [`pads.md`](../../../docs/rendering/pads.md)'s "What
//! binds the `_ne` file": `oag_mesh::mesh::rcs::emissive` reads the tint
//! parameter (`0xe8bcd7f5`) off the **inline** `rcsmodel::Material` record
//! and falls back to white `[1.0, 1.0, 1.0]` when it is absent - which
//! `pads.md` already established for both pad materials (one `parameters`
//! entry each, neither the tint/offset/scroll hash). What that fallback does
//! not rule out: the fragment microcode may still carry an **inline literal**
//! constant feeding the same multiply, the way `emissive`'s own doc comment
//! already records for the coordinate constants of a second material
//! population. This disassembles the pad's own fragment program in full and
//! reports every instruction that reads a `Constant` source, so a baked-in
//! tint (red or otherwise) would show up as a literal float4 here rather
//! than as a parameter this project already checked.
//!
//! Also prints every instruction's source and destination swizzle/write
//! mask, which the first version of this probe omitted -
//! `docs/rendering/pads.md`'s "Next Steps" step 0 asks for exactly this, to
//! settle which alpha channel (the diffuse's, covering 93% of the plate, or
//! `_ne`'s own, covering the ~7% light-bar mask) actually gates the pad's
//! final accumulate, rather than assuming the renderer's already-implemented
//! generic `emissive` shape (`mesh.wgsl`, `glow = second.rgb * tint.rgb *
//! first.a`) is what the pad's own microcode executes.

use oag_rcs::{rcsmaterial, rcsmodel};

use oag_mesh::mesh;
use oag_vex::vex;

const NE_SAMPLER_HASH: u32 = 0xa2d5_55b9;
/// `oag_mesh::mesh::rcs::emissive::TINT`.
const TINT_HASH: u32 = 0xe8bc_d7f5;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/12_sol_2/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let read = |path: &str| mesh::read_blob(&spec, path).ok();

    // One chunk per pad type is enough - `hd_pad_material_dump.rs` already
    // measured the shape is uniform across all 18.
    for (label, class_id) in [
        ("Speedup Pad", classes.speedup_pad),
        ("Weapon Pad", classes.weapon_pad),
    ] {
        let Some(class_id) = class_id else { continue };
        let Some(node) = nodes.iter().find(|n| n.class_id == class_id) else {
            continue;
        };
        let payload = &data[node.payload()];
        let hash = order.u32(payload, 0x30);
        let Some(chunk) = model.mesh(hash) else {
            continue;
        };
        let Some(surface) = chunk.surfaces().next() else {
            continue;
        };
        let slot = surface.material as usize;
        let Some(material) = model.materials.get(slot) else {
            continue;
        };
        println!("== {label}: {} ==", material.name);

        let Some(blob) = read(&format!("/{}", material.name)) else {
            println!("  (material file did not load)");
            continue;
        };
        let word =
            rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, surface.decl.as_ref());
        let key = rcsmaterial::Features::from_pass_word(word);
        let Some(variant) = rcsmaterial::RcsMaterial::parse(&blob)
            .ok()
            .and_then(|m| m.variant(rcsmaterial::Class::Static, key).copied())
        else {
            println!("  (no Static variant for this chunk's own feature word)");
            continue;
        };
        let Some(program) = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
        else {
            println!("  (fragment program did not parse)");
            continue;
        };

        let ne_unit = program
            .declared
            .samplers
            .iter()
            .find(|(h, _)| *h == NE_SAMPLER_HASH)
            .map(|&(_, unit)| unit);
        println!("  _ne sampler binds unit {ne_unit:?}");
        println!("  declared samplers: {:#010x?}", program.declared.samplers);
        let unit2_is_lightmap = program
            .declared
            .samplers
            .iter()
            .any(|&(h, unit)| unit == 2 && h == rcsmaterial::LIGHTMAP_SAMPLER);
        println!(
            "  unit 2 is the lightmap sampler ({:#010x})? {unit2_is_lightmap}",
            rcsmaterial::LIGHTMAP_SAMPLER
        );
        println!("  output_texels(): {:#?}", program.output_texels());

        let tint_patch_slots: Vec<u16> = program.patches(TINT_HASH).collect();
        println!(
            "  parameter {TINT_HASH:#010x} (emissive::TINT) patch slots: {tint_patch_slots:?}"
        );
        println!(
            "  declared parameters ({}): {:#010x?}",
            program.declared.parameters.len(),
            program.declared.parameters
        );
        for &param in &program.declared.parameters {
            let slots: Vec<u16> = program.patches(param).collect();
            let value = material
                .parameters
                .iter()
                .find(|p| p.hash == param)
                .map(|p| p.value);
            if !slots.is_empty() {
                println!("    {param:#010x} patches code slots {slots:?} material value={value:?}");
            }
        }

        println!(
            "  full disassembly ({} instructions):",
            program.instructions.len()
        );
        for (i, insn) in program.instructions.iter().enumerate() {
            let slot = i; // this instruction's own 16-byte-slot ordinal is not tracked directly; report by stream index instead
            let name = insn
                .name()
                .unwrap_or_else(|| Box::leak(format!("op{:#04x}", insn.opcode).into_boxed_str()));
            let dst = format!("{}{}", if insn.dst_half { "H" } else { "R" }, insn.dst);
            let is_tex = insn.is_texture();
            let mut line = format!(
                "    [{slot:>3}] {name:<6} {dst}{}{} ",
                if insn.saturate { "_sat" } else { "" },
                if is_tex {
                    format!(" unit={}", insn.unit)
                } else {
                    String::new()
                }
            );
            let lane = |l: u8| ['x', 'y', 'z', 'w'][usize::from(l) & 3];
            for (slot, src) in insn.sources.iter().take(insn.arity()).enumerate() {
                let tag = match src {
                    rcsmaterial::fragment::Source::Register { index, half } => {
                        format!("{}{index}", if *half { "H" } else { "R" })
                    }
                    rcsmaterial::fragment::Source::Input => format!("IN{}", insn.input),
                    rcsmaterial::fragment::Source::Constant => "C".to_string(),
                    rcsmaterial::fragment::Source::Unknown => "?".to_string(),
                };
                let swiz: String = insn.swizzles[slot].iter().map(|&l| lane(l)).collect();
                let neg = if insn.negate[slot] { "-" } else { "" };
                line.push_str(&format!("{neg}{tag}.{swiz} "));
            }
            if insn.cond != 0x727 {
                line.push_str(&format!("[cond {:#05x}] ", insn.cond));
            }
            line.push_str(&format!(
                "-> .{}",
                (0u8..4)
                    .filter(|c| insn.mask & (1 << c) != 0)
                    .map(lane)
                    .collect::<String>()
            ));
            if let Some(c) = insn.constant {
                line.push_str(&format!("const_slot={:?} value={c:?}", insn.const_slot));
                // Flag if this constant's slot is one the TINT parameter
                // patches at draw time, vs. a genuine file-authored literal.
                if let Some(slot) = insn.const_slot
                    && tint_patch_slots.contains(&slot)
                {
                    line.push_str("  <-- TINT patches this slot at draw time");
                }
            }
            println!("{line}");
        }
    }

    Ok(())
}
