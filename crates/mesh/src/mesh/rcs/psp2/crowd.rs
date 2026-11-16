//! Omega's `_shadow` crowd materials: `crowd_sprite_diffuse_normal_alpha_shadow` and
//! `crowd_diffuse_normal_shadow`.
//!
//! Their lit pixel program ends `Shadowed * T * (Ambient + constantAmbientColour + sun *
//! sat(N.L))` (`docs/formats/omega-status.md`, "The crowd's law", confidence 70): the material
//! authors `Shadowed` (`0.12178`) as a factor on the whole picture and `Ambient` (`0.777`) as a
//! term inside the bracket. The generic lit path reads neither, which draws these stands too
//! bright. The plain `crowd_sprite_diffuse_normal_alpha` and `crowd_diffuse_normal` programs
//! declare neither uniform and are not admitted (`gcn_dump` of both materials' `Shdr` tables).
//!
//! Carried by a glow-table entry of mode [`PROGRAM_CROWD`]: `tint` is `Ambient`, `program[1]`
//! is `Shadowed`. The fragment stage replaces the light sum for the draw and multiplies the
//! lit result by `Shadowed`. Admitted by material name and by both parameters being authored,
//! so a material of the same name that does not author them keeps the generic reading.

use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::Report;
use crate::mesh::{Emissive, PROGRAM_CROWD, slots};

/// `~crc32("Shadowed")`.
const SHADOWED: u32 = 0x3699_7860;
/// `~crc32("Ambient")`.
const AMBIENT: u32 = 0x7353_24ab;

const FAMILIES: [&str; 2] = [
    "crowd_sprite_diffuse_normal_alpha_shadow",
    "crowd_diffuse_normal_shadow",
];

/// Per material, the glow-table slot (entry index plus one) it was admitted with.
#[derive(Default)]
pub(super) struct Plan {
    slots: Vec<u32>,
}

impl Plan {
    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        material
            .and_then(|m| self.slots.get(m))
            .map_or(0, |&slot| slot << slots::MATERIAL_SHIFT)
    }
}

/// The entry of a crowd material: `Ambient` as the tint, `Shadowed` in the program's second lane.
fn entry(material: &Material) -> Option<Emissive> {
    let stem = material.name.rsplit(['/', '\\']).next()?;
    let stem = stem.trim_suffix(".rcsmaterial");
    if !FAMILIES.iter().any(|f| stem.eq_ignore_ascii_case(f)) {
        return None;
    }
    let shadowed = *material.param(SHADOWED)?.first()?;
    let ambient = material.param(AMBIENT)?;
    let channel = |i: usize| ambient.get(i).copied();
    Some(Emissive {
        tint: [channel(0)?, channel(1)?, channel(2)?],
        offset: 0.0,
        scale: 0.0,
        rate: 0.0,
        program: [PROGRAM_CROWD, shadowed],
    })
}

/// Admits each crowd material that authors `Shadowed` and `Ambient`, writing its entry into
/// `emissive`. Empty on every model but Omega's: only a PS4 model is asked.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    emissive: &mut Vec<Emissive>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    for material in &decoded.materials {
        let admitted = entry(material).filter(|_| ps4);
        let slot = admitted.and_then(|entry| {
            let at = emissive.iter().position(|e| *e == entry).or_else(|| {
                (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                    emissive.push(entry);
                    emissive.len() - 1
                })
            });
            at.and_then(|at| u32::try_from(at + 1).ok())
        });
        match (admitted, slot) {
            (Some(_), Some(_)) => report.crowd_materials += 1,
            (Some(_), None) => report.crowd_unread += 1,
            _ => {}
        }
        out.slots.push(slot.unwrap_or(0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;
    use oag_rcs::rcsmodel::psp2::material::Param;

    fn material(name: &str, params: &[(u32, Vec<f32>)]) -> Material {
        Material {
            name: format!("data/environments2048/sol/materials/{name}.rcsmaterial"),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: params
                .iter()
                .map(|(hash, v)| Param {
                    hash: *hash,
                    bits: v.iter().map(|c| c.to_bits()).collect(),
                })
                .collect(),
            samplers: Vec::new(),
            state: None,
        }
    }

    #[test]
    fn the_parameter_constants_are_the_hashes_of_their_names() {
        assert_eq!(SHADOWED, name_hash("Shadowed"));
        assert_eq!(AMBIENT, name_hash("Ambient"));
    }

    #[test]
    fn a_crowd_material_carries_ambient_as_tint_and_shadowed_as_the_second_lane() {
        let m = material(
            "crowd_sprite_diffuse_normal_alpha_shadow",
            &[(SHADOWED, vec![0.121_78]), (AMBIENT, vec![0.777; 3])],
        );
        let e = entry(&m).expect("admitted");
        assert_eq!(e.tint, [0.777; 3]);
        assert_eq!(e.program, [PROGRAM_CROWD, 0.121_78]);
    }

    #[test]
    fn only_the_two_families_that_author_both_parameters_are_admitted() {
        let both = [(SHADOWED, vec![0.1]), (AMBIENT, vec![0.7; 3])];
        for family in FAMILIES {
            assert!(entry(&material(family, &both)).is_some(), "{family}");
        }
        assert!(entry(&material("crowd_sprite_diffuse_normal_alpha", &both)).is_none());
        assert!(entry(&material("crowd_diffuse_normal", &both)).is_none());
        let one = [(SHADOWED, vec![0.1])];
        assert!(entry(&material("crowd_sprite_diffuse_normal_alpha_shadow", &one)).is_none());
    }
}
