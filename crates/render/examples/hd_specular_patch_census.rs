//! When `Program::specular_exponent()` resolves to a literal `0.0`, is that
//! code slot patched by `SpecularPower` at draw time, and does the material's
//! own `.rcsmodel` instance author a non-zero value for it?
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have no Lambert
//! diffuse either" names `0.0` the strongest candidate for "patched at draw
//! time rather than baked in the file" - `pow(x, 0) = 1` is not a plausible
//! authored shininess. This is that hypothesis checked rather than assumed:
//! across 16 circuits, every material whose `0.0` chain patches from
//! `SpecularPower` also authors a non-zero value for it (30 to 100, none of
//! them a round number the shared-literal population - `5`/`10`/`32`/`40`/
//! `300` - carries), and `mesh::rcs::skin::roles` now reads that value
//! instead of falling back to the shared stand-in.
//!
//! It also checks a second thing `fragment::Program::patches`'s own doc
//! comment leans on: that no two distinct declared parameters of a resolved
//! block ever patch the same code slot, over every pair of every block
//! reached here.

use oag_rcs::{rcsmaterial, rcsmodel};
use oag_render::mesh;

const CIRCUITS: &[(&str, &str)] = &[
    ("DATA00", "/data/environments/amphiseum/track.vex"),
    ("DATA00", "/data/environments/modesto_heights/track.vex"),
    ("DATA00", "/data/environments/talons_junction/track.vex"),
    ("DATA00", "/data/environments/tech_de_ra/track.vex"),
    ("DATA00", "/data/environments/zone_1/track.vex"),
    ("DATA00", "/data/environments/zone_2/track.vex"),
    ("DATA00", "/data/environments/zone_3/track.vex"),
    ("DATA00", "/data/environments/zone_4/track.vex"),
    ("DATA02", "/data/environments/04_chenghou_project/track.vex"),
    ("DATA02", "/data/environments/01_vineta_k/track.vex"),
    ("DATA02", "/data/environments/15_anulpha_pass/track.vex"),
    ("DATA02", "/data/environments/03_track/track.vex"),
    ("DATA02", "/data/environments/02_track/track.vex"),
    ("DATA02", "/data/environments/10_sebenco_climb/track.vex"),
    ("DATA02", "/data/environments/05_ubermall/track.vex"),
    ("DATA02", "/data/environments/12_sol_2/track.vex"),
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut zero = 0usize;
    let mut zero_patched_by_specular_power = 0usize;
    let mut zero_authored_nonzero = 0usize;
    let mut zero_authored_zero_or_absent = 0usize;
    let mut other_nonzero = 0usize;
    let mut blocks_checked = 0usize;
    let mut slot_collisions = 0usize;

    for (archive, name) in CIRCUITS {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(data) = mesh::read_blob(&spec, name) else {
            continue;
        };
        let Ok(Some((model, _))) = mesh::rcs::scene_from(&spec, name, &data) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, name, &data) else {
            continue;
        };
        let Ok(source) = rcsmodel::Model::parse(&geometry) else {
            continue;
        };

        for (slot, variant) in model.material_variants.iter().enumerate() {
            let Some(variant) = variant else { continue };
            let Some(material) = source.materials.get(slot) else {
                continue;
            };
            let Ok(blob) = mesh::read_blob(&spec, &format!("/{}", material.name)) else {
                continue;
            };
            let Some(program) =
                rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
            else {
                continue;
            };
            blocks_checked += 1;
            let patched_hashes: Vec<u32> = program
                .declared
                .parameter_patches
                .iter()
                .filter(|&&(_, vreg, _)| vreg == 0xffff)
                .map(|&(hash, ..)| hash)
                .collect();
            for i in 0..patched_hashes.len() {
                for other in &patched_hashes[i + 1..] {
                    if patched_hashes[i] == *other {
                        continue;
                    }
                    let a: Vec<u16> = program.patches(patched_hashes[i]).collect();
                    if program.patches(*other).any(|s| a.contains(&s)) {
                        slot_collisions += 1;
                    }
                }
            }
            let Some(exponent) = program.specular_exponent() else {
                continue;
            };
            if exponent != 0.0 {
                other_nonzero += 1;
                continue;
            }
            zero += 1;
            let patched = program.specular_exponent_slot().is_some_and(|slot| {
                program
                    .patches(rcsmaterial::SPECULAR_POWER)
                    .any(|s| s == slot)
            });
            if patched {
                zero_patched_by_specular_power += 1;
            }
            let authored = material
                .parameters
                .iter()
                .find(|p| p.hash == rcsmaterial::SPECULAR_POWER)
                .map(|p| p.value[0]);
            match authored {
                Some(v) if v != 0.0 => {
                    zero_authored_nonzero += 1;
                    println!(
                        "{name} slot {slot} ({}): authored SpecularPower = {v}, patched = {patched}",
                        material.name
                    );
                }
                _ => zero_authored_zero_or_absent += 1,
            }
        }
    }

    println!("---");
    println!("other non-zero resolved exponents: {other_nonzero}");
    println!("resolved 0.0: {zero}");
    println!("  of those, code slot patched by SpecularPower: {zero_patched_by_specular_power}");
    println!("  of those, model authors a non-zero SpecularPower: {zero_authored_nonzero}");
    println!("  of those, model authors zero or nothing: {zero_authored_zero_or_absent}");
    println!("blocks checked for a shared patch slot: {blocks_checked}");
    println!("two parameters patching the same slot: {slot_collisions}");
    Ok(())
}
