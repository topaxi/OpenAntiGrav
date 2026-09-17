//! Scratch probe: where in `321go_startfinish.rcsmodel`'s own bytes does the
//! digit board's animated `uvOffset` curve live?
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s 2026-09-17 sections
//! locate the runtime write (`AnimCurve_EvaluateChannels`) and trace it to a
//! curve pointer at `material_record + 0x20`, then `+ 0xc` - a field
//! `oag_rcs::rcsmodel::material` parses up to `+0x1c` and stops short of.
//! This probe reads that pointer chain directly off the file for every
//! material, verifies the channel count and descriptors
//! `AnimCurve_EvaluateChannels`/`EdgeAnim_EvaluateClip` read (at the curve's
//! own `inner+0x24` and `curve+0x04`, not guessed from a fixed dump window),
//! and runs `oag_rcs::rcsmodel::coverage` over the file to show what the
//! existing geometry/material parser still never claims.
//!
//! `cargo run -p oag-rcs --example hd_gantry_curve_probe -- <321go_startfinish.rcsmodel>`

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: hd_gantry_curve_probe <321go_startfinish.rcsmodel>");
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));

    let model = oag_rcs::rcsmodel::Model::parse(&data).unwrap_or_else(|e| panic!("parse: {e}"));
    println!(
        "{} bytes, {} mesh chunk(s), {} material(s)",
        data.len(),
        model.meshes.len(),
        model.materials.len()
    );

    let total_submeshes: usize = model
        .meshes
        .iter()
        .map(|m| m.surfaces().map(|s| s.submeshes.len()).sum::<usize>())
        .sum();
    let total_surfaces: usize = model.meshes.iter().map(|m| m.surfaces().count()).sum();
    println!("{total_surfaces} surface(s), {total_submeshes} submesh(es) total");

    // Walk the material table `oag_rcs::rcsmodel`'s own header fields at
    // +0x2c/+0x30 name (`Billboard_UpdateInstanceUvs`'s "animated-target
    // list" is this exact table, not a separate section - confirmed by the
    // material count matching here) and read each record's own undecoded
    // +0x20 field, the curve pointer's own carrier.
    let be32 = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
    let material_count = be32(0x2c) as usize;
    let material_table = be32(0x30) as usize;
    println!("\nheader: material_count={material_count} material_table=0x{material_table:x}");
    for i in 0..material_count {
        let at = be32(material_table + i * 4) as usize;
        let plus20 = be32(at + 0x20);
        println!(
            "material[{i}] at 0x{at:x}: name={:?} +0x20=0x{plus20:x}",
            model.materials.get(i).map(|m| &m.name)
        );
        if plus20 != 0 {
            let target = plus20 as usize;
            if target + 0x10 <= data.len() {
                let word_c = be32(target + 0xc);
                println!("  +0x20 -> 0x{target:x}, [+0xc]=0x{word_c:x} (candidate curve ptr)");
                if word_c != 0 && (word_c as usize) + 0x28 <= data.len() {
                    let curve = word_c as usize;
                    let inner = be32(curve) as usize;
                    println!("    curve @0x{curve:x}: +0x00 (inner ptr)=0x{inner:x}");
                    if inner + 0x26 <= data.len() {
                        // AnimCurve_EvaluateChannels reads the period at
                        // inner+0x04 and the channel count at inner+0x24 -
                        // NOT from the curve struct itself, which only holds
                        // the pointer to `inner` and the descriptor array.
                        // Verifying the count here is what makes "material 1
                        // has two channels" a read rather than a guess at
                        // how many curve+0x04 words to print.
                        let period =
                            f32::from_be_bytes(data[inner + 4..inner + 8].try_into().unwrap());
                        let count = u16::from_be_bytes(
                            data[inner + 0x24..inner + 0x26].try_into().unwrap(),
                        );
                        println!("    inner @0x{inner:x}: period={period} channel_count={count}");
                        // A channel's `target_index` is not into a curve-local
                        // array - AnimCurve_EvaluateChannels indexes the same
                        // material's own +0x34 parameter table
                        // (oag_rcs::rcsmodel::material::parameters' own 0x20
                        // stride). Naming it "uvOffset" is a hash match, not
                        // an assumption that the animated parameter must be
                        // the UV one.
                        let param_table = be32(at + 0x34) as usize;
                        let uv_offset_hash = oag_rcs::rcsmaterial::name_hash("uvOffset");
                        let uv_scale_hash = oag_rcs::rcsmaterial::name_hash("uvScale");
                        for i in 0..count as usize {
                            let off = curve + 4 + i * 2;
                            if off + 2 <= data.len() {
                                let desc =
                                    u16::from_be_bytes(data[off..off + 2].try_into().unwrap());
                                let target_index = ((desc >> 2) & 0x3fff) as usize;
                                let component = desc & 3;
                                let entry_hash = be32(param_table + target_index * 0x20);
                                let name = if entry_hash == uv_offset_hash {
                                    "uvOffset"
                                } else if entry_hash == uv_scale_hash {
                                    "uvScale"
                                } else {
                                    "?"
                                };
                                let axis = [".x", ".y", ".z", ".w"][component as usize];
                                println!(
                                    "      channel[{i}] desc=0x{desc:04x} -> target_index={target_index} component={component} = {name}{axis} (hash=0x{entry_hash:08x})",
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    let coverage = oag_rcs::rcsmodel::coverage(&data);
    println!(
        "coverage: {:.1}% ({} of {} bytes claimed)",
        coverage.fraction() * 100.0,
        coverage.claimed(),
        coverage.len()
    );

    for gap in coverage.gaps(16) {
        println!(
            "\ngap: {} bytes at 0x{:x}, between {:?} and {:?}",
            gap.len, gap.at, gap.after, gap.before
        );
        let end = (gap.at + gap.len.min(256)).min(data.len());
        for (i, chunk) in data[gap.at..end].chunks(16).enumerate() {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
            println!("  {:08x}  {}", gap.at + i * 16, hex.join(" "));
        }
        if gap.len > 256 {
            println!("  ... {} more bytes", gap.len - 256);
        }
    }
}
