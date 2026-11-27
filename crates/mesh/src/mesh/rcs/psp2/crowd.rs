//! Omega's crowd materials: `crowd_sprite_diffuse_normal_alpha` and `crowd_diffuse_normal`, and
//! their `_shadow` twins.
//!
//! The `_shadow` twins' lit pixel program ends `Shadowed * T * (Ambient + constantAmbientColour +
//! sun * sat(N.L))` (`docs/formats/omega-status.md`, "The crowd's law", confidence 70): the
//! material authors `Shadowed` (`0.12178`) as a factor on the whole picture and `Ambient`
//! (`0.777`) as a term inside the bracket. The generic lit path reads neither, which draws these
//! stands too bright. The plain programs declare neither uniform: they are the same sum with
//! `Shadowed = 1` and `Ambient = 0`, and are admitted only for the frame below.
//!
//! **The frame.** The unshadowed permutations of all four materials weight the normal map's `z`
//! lane by the vertex program's `eyePositionWorldSpace - position`, not by the vertex normal
//! (`oag_rcs::gcn::third_axis_is_eye`, "The crowd sprite's frame"): each figure is lit as turned
//! to the camera. A material is given that frame when its file reads so; one that does not keeps
//! the vertex normal.
//!
//! Carried by a glow-table entry of mode [`PROGRAM_CROWD`]: `tint` is `Ambient`, `program[1]`
//! is `Shadowed`, `offset` is `1` for the eye frame. The fragment stage replaces the light sum
//! for the draw and multiplies the lit result by `Shadowed`. A `_shadow` twin is admitted by
//! material name and by both parameters being authored, so a material of the same name that
//! does not author them keeps the generic reading.

use oag_rcs::gcn;
use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::Report;
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, PROGRAM_CROWD, slots};

/// `~crc32("Shadowed")`.
const SHADOWED: u32 = 0x3699_7860;
/// `~crc32("Ambient")`.
const AMBIENT: u32 = 0x7353_24ab;

/// The twins that author `Shadowed` and `Ambient`.
const FAMILIES: [&str; 3] = [
    "crowd_sprite_diffuse_normal_alpha_shadow",
    "crowd_diffuse_normal_shadow",
    "crowd_diffuse_normal_shadow_bonecount4_normalizets",
];

/// The plain materials, whose programs are the twins' with `Shadowed = 1` and `Ambient = 0`.
const PLAIN: [&str; 4] = [
    "crowd_sprite_diffuse_normal_alpha",
    "crowd_diffuse_normal",
    "crowd_diffuse_normal_bonecount4",
    "crowd_diffuse_normal_bonecount4_normalizets",
];

/// Whether `material` is one of the rigs' skinned twins, whose pixel programs are the unskinned
/// ones' (`gcn_dump` of each pair: 52/119/125/115 instructions for `_NormalizeTS`, 52/104/110/100
/// for the bare `Crowd_Diffuse_Normal`, 60/128/131/123 shadowed) and whose vertex programs read
/// `eyePositionWorldSpace` as theirs do.
fn skinned(material: &Material) -> bool {
    material.name.to_ascii_lowercase().contains("_bonecount4")
}

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

/// The entry of a crowd material: `Ambient` as the tint, `Shadowed` in the program's second lane,
/// `offset` `1` where the material's programs take the eye frame. A plain material is admitted
/// only with that frame, since it is the only thing the entry changes for it.
fn entry(material: &Material, eye_frame: bool) -> Option<Emissive> {
    let stem = material.name.rsplit(['/', '\\']).next()?;
    let stem = stem.trim_suffix(".rcsmaterial");
    let named = |list: &[&str]| list.iter().any(|f| stem.eq_ignore_ascii_case(f));
    let (shadowed, tint) = if named(&FAMILIES) {
        let ambient = material.param(AMBIENT)?;
        let channel = |i: usize| ambient.get(i).copied();
        (
            *material.param(SHADOWED)?.first()?,
            [channel(0)?, channel(1)?, channel(2)?],
        )
    } else if named(&PLAIN) && eye_frame {
        (1.0, [0.0; 3])
    } else {
        return None;
    };
    Some(Emissive {
        tint,
        offset: f32::from(u8::from(eye_frame)),
        scale: 0.0,
        rate: 0.0,
        program: [PROGRAM_CROWD, shadowed],
    })
}

/// Admits each crowd material, writing its entry into `emissive`. Empty on every model but
/// Omega's: only a PS4 model is asked.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    for (index, material) in decoded.materials.iter().enumerate() {
        let eye_frame = ps4
            && (skinned(material)
                || textures(&material.name)
                    .is_some_and(|file| gcn::third_axis_is_eye(&file).is_eye()));
        let admitted = entry(material, eye_frame).filter(|_| ps4);
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
            (Some(entry), Some(_)) => {
                crate::mesh::rcs::census::claim(
                    index,
                    "crowd",
                    crate::mesh::rcs::census::Outcome::Drawn,
                );
                report.crowd_materials += 1;
                report.crowd_eye_frames += usize::from(entry.offset > 0.5);
            }
            (Some(_), None) => {
                crate::mesh::rcs::census::claim(
                    index,
                    "crowd",
                    crate::mesh::rcs::census::Outcome::NotDrawn,
                );
                report.crowd_unread += 1;
            }
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
        let e = entry(&m, false).expect("admitted");
        assert_eq!(e.tint, [0.777; 3]);
        assert_eq!(e.program, [PROGRAM_CROWD, 0.121_78]);
        assert_eq!(e.offset, 0.0);
    }

    #[test]
    fn only_the_two_twins_that_author_both_parameters_are_admitted_without_the_frame() {
        let both = [(SHADOWED, vec![0.1]), (AMBIENT, vec![0.7; 3])];
        for family in FAMILIES {
            assert!(entry(&material(family, &both), false).is_some(), "{family}");
        }
        for plain in PLAIN {
            assert!(entry(&material(plain, &both), false).is_none(), "{plain}");
        }
        let one = [(SHADOWED, vec![0.1])];
        assert!(
            entry(
                &material("crowd_sprite_diffuse_normal_alpha_shadow", &one),
                true
            )
            .is_none()
        );
    }

    #[test]
    fn a_plain_crowd_material_with_the_eye_frame_is_the_twin_with_shadowed_one_and_no_ambient() {
        for plain in PLAIN {
            let e = entry(&material(plain, &[]), true).expect("admitted");
            assert_eq!(e.tint, [0.0; 3]);
            assert_eq!(e.program, [PROGRAM_CROWD, 1.0]);
            assert_eq!(e.offset, 1.0);
        }
        let twin = material(
            FAMILIES[0],
            &[(SHADOWED, vec![0.1]), (AMBIENT, vec![0.7; 3])],
        );
        assert_eq!(entry(&twin, true).expect("admitted").offset, 1.0);
    }
}
