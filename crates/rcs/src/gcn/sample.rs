//! Which texture a pixel program decodes as a tangent-space normal.
//!
//! A normal map is read as `2x - 1`: the program samples the texture and the first thing it does
//! with the result is `v_mad_f32 d, 2.0, x, -1.0`. [`Program::signed_textures`] finds the
//! `image_sample`s whose destination feeds such an instruction and names the API slot of the
//! texture each one reads, which is the slot the reflection's texture resource carries.
//!
//! The slot comes from the header's [`Usage`] records, not from a register convention: the
//! sample's T# is either an inline user SGPR a record names, or a descriptor `s_load`ed through a table
//! pointer a record names: the resource table (slot = offset / 8) or the extended table
//! ([`Usage::TABLE_BASE`] plus the offset names a texture record).

use std::collections::BTreeSet;

use super::isa::{Encoding, Instruction, Operand};
use super::{Program, Usage};

/// What [`Program::signed_textures`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignedTextures {
    /// The API slot of every texture decoded `2x - 1`, once each, ascending.
    pub slots: Vec<u8>,
    /// Samples that feed a `2x - 1` whose T# no usage record placed. Counted, never guessed at.
    pub unresolved: usize,
}

/// What [`signed_samplers`] found in one material file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignedSamplers {
    /// The sampler names (a texture resource's name without its `Texture` suffix, the name a
    /// material instance hashes) that **every** pixel program of the file which decodes any
    /// `2x - 1` decodes so. A normal map, but also what a program that reads two normal maps
    /// scales the same way (the caller keeps what its material instance binds, and names a
    /// surplus as ambiguous).
    pub names: BTreeSet<String>,
    /// Signed samples, over the file, whose T# no usage record placed.
    pub unresolved: usize,
}

/// The engine's shadow depth map, which a program scales `2x - 1` into clip space; never a
/// material's own sampler.
const SHADOW_MAP: &str = "shadowMap";

/// The samplers the pixel programs of a `.rcsmaterial` file decode `2x - 1`.
///
/// **Intersected across programs, not united.** A file carries a program per light and shadow
/// variant, and the variants that add a second `2x - 1` (a diffuse scaled for a reflection, a
/// detail map) do not all add the same one: on `Tree_wind` the normal map is signed in all 9
/// programs and the diffuse in 2.
#[must_use]
pub fn signed_samplers(file: &[u8]) -> SignedSamplers {
    let mut found = SignedSamplers::default();
    let mut common: Option<BTreeSet<String>> = None;
    for (_, program) in super::scan(file) {
        let Ok(program) = program else { continue };
        if program.stage != super::Stage::Pixel {
            continue;
        }
        let signed = program.signed_textures();
        found.unresolved += signed.unresolved;
        let Ok(reflection) = &program.reflection else {
            continue;
        };
        let names: BTreeSet<String> = signed
            .slots
            .iter()
            .filter_map(|&slot| {
                reflection
                    .resources
                    .iter()
                    .find(|r| r.size == 0 && r.slot == u32::from(slot))
            })
            .map(|r| {
                r.name
                    .strip_suffix("Texture")
                    .unwrap_or(&r.name)
                    .to_string()
            })
            .filter(|n| n != SHADOW_MAP)
            .collect();
        if names.is_empty() {
            continue;
        }
        common = Some(match common {
            None => names,
            Some(c) => c.intersection(&names).cloned().collect(),
        });
    }
    found.names = common.unwrap_or_default();
    found
}

fn overlaps(operand: &Operand, first: u8, count: u8) -> bool {
    matches!(*operand, Operand::Vgpr { first: f, count: c }
        if f < first + count && first < f + c)
}

fn decodes_signed(instruction: &Instruction, first: u8, count: u8) -> bool {
    let has = |value: f32| {
        instruction
            .src
            .iter()
            .flatten()
            .any(|s| *s == Operand::Float(value))
    };
    has(2.0)
        && has(-1.0)
        && instruction
            .src
            .iter()
            .flatten()
            .any(|s| overlaps(s, first, count))
}

impl Program<'_> {
    /// The textures this program reads as `2x - 1`. See the module docs.
    #[must_use]
    pub fn signed_textures(&self) -> SignedTextures {
        let mut found = SignedTextures::default();
        for (k, sample) in self.instructions.iter().enumerate() {
            let is_sample = sample.encoding == Encoding::Mimg
                && sample.name.is_some_and(|n| n.contains("sample"));
            let (Some(Operand::Vgpr { first, count }), Some(Operand::Sgpr { first: t, .. })) =
                (sample.dst[0], sample.src[1])
            else {
                continue;
            };
            if !is_sample
                || !self.instructions[k + 1..]
                    .iter()
                    .any(|i| decodes_signed(i, first, count))
            {
                continue;
            }
            match self.texture_slot(k, t) {
                Some(slot) => {
                    if let Err(at) = found.slots.binary_search(&slot) {
                        found.slots.insert(at, slot);
                    }
                }
                None => found.unresolved += 1,
            }
        }
        found
    }

    /// The API slot of the texture whose T# starts at SGPR `t` when instruction `k` samples it.
    fn texture_slot(&self, k: usize, t: u8) -> Option<u8> {
        let record = |kind: u8, register: u32| {
            self.usage
                .iter()
                .find(|u: &&Usage| u.kind == kind && u32::from(u.register) == register)
        };
        let load = self.instructions[..k]
            .iter()
            .rev()
            .find(|i| matches!(i.dst[0], Some(Operand::Sgpr { first, count: 8 }) if first == t));
        let Some(load) = load else {
            return record(Usage::TEXTURE, u32::from(t)).map(|u| u.slot);
        };
        let (Some(Operand::Sgpr { first: base, .. }), Some(Operand::Imm(offset))) =
            (load.src[0], load.src[1])
        else {
            return None;
        };
        if record(Usage::RESOURCE_TABLE, u32::from(base)).is_some() {
            return u8::try_from(offset / 8).ok();
        }
        record(Usage::EXTENDED_TABLE, u32::from(base))?;
        record(Usage::TEXTURE, Usage::TABLE_BASE + offset).map(|u| u.slot)
    }
}
