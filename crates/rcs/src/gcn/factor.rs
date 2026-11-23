//! Whether a material takes a vertex input as a factor on its light.
//!
//! A hull program fetches `VertexColour1`, exports it unchanged to a parameter, and its pixel
//! program multiplies the interpolated colour into the diffuse sample and the light
//! (`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "Recipe"). A track program that adds it
//! to the light reads the same fetch and the same export, so the vertex half alone does not
//! separate them: [`input_is_factor`] also follows the interpolant through the pixel programs.
//!
//! The pixel half is conservative in one direction. The lanes the interpolant lands in are
//! tracked until the register is written again, and each read must be a multiplicand of a
//! `mul`, `mac`, `mad` or `fma` (the first two sources). An addend, a compare, a move or an
//! opcode the table does not name fails the file.

use super::isa::{Detail, Instruction, Operand};
use super::{Program, Stage};

/// What [`input_is_factor`] counted in one material file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Factor {
    /// Vertex programs that fetch the input.
    pub fetching: usize,
    /// Of those, ones that export it unchanged to a parameter.
    pub exporting: usize,
    /// Pixel programs that read the parameter the vertex program before them exports.
    pub reading: usize,
    /// Of those, ones that read every lane only as a multiplicand.
    pub factor_only: usize,
    /// Of those, ones that take the scene's light (`liveLighting0diffuse`): the colour is a
    /// factor on a light, not on a flat texture.
    pub lit: usize,
}

impl Factor {
    /// Every fetching program exports it, at least one pixel program reads it, every
    /// pixel program that reads it reads it as a factor, and one of them is lit.
    #[must_use]
    pub fn is_factor(&self) -> bool {
        self.fetching > 0
            && self.exporting == self.fetching
            && self.reading > 0
            && self.factor_only == self.reading
            && self.lit > 0
    }
}

/// Reads the `.rcsmaterial` file's programs for `name`, a vertex input of the reflection
/// (`VertexColour1`). A file lists each vertex program directly before the pixel program it
/// feeds, so a pixel program is read against the parameter of the vertex program before it.
#[must_use]
pub fn input_is_factor(file: &[u8], name: &str) -> Factor {
    let mut found = Factor::default();
    let mut param = None;
    for (_, program) in super::scan(file).iter() {
        let Ok(program) = program else { continue };
        match program.stage {
            Stage::Vertex => {
                param = None;
                if program.input_named(name).is_some() {
                    found.fetching += 1;
                    param = program.exported_input(name);
                    found.exporting += usize::from(param.is_some());
                }
            }
            Stage::Pixel => {
                if let Some(param) = param.take()
                    && program.reads_attribute(param)
                {
                    found.reading += 1;
                    let factor = program.attribute_is_only_a_factor(param);
                    found.factor_only += usize::from(factor);
                    found.lit += usize::from(factor && program.names_constant(LIGHT));
                }
            }
            Stage::Other(_) => {}
        }
    }
    found
}

/// The constant every lit pixel program of a hull or track reads its sun colour from.
const LIGHT: &str = "liveLighting0diffuse";

impl Program {
    /// Whether the reflection declares a constant called `name`.
    #[must_use]
    pub fn names_constant(&self, name: &str) -> bool {
        self.reflection
            .as_ref()
            .is_ok_and(|r| r.constants.iter().any(|c| c.name == name))
    }

    /// The parameter a vertex program exports the input `name` to, when three exported lanes
    /// are the registers it was fetched into.
    #[must_use]
    pub fn exported_input(&self, name: &str) -> Option<u8> {
        let (input, _) = self.input_named(name)?;
        let lanes: Vec<u8> = (input.vgpr..input.vgpr + input.size.min(3)).collect();
        self.instructions.iter().find_map(|i| {
            let Detail::Export { target, .. } = i.detail else {
                return None;
            };
            let sources: Vec<u8> = i
                .src
                .iter()
                .flatten()
                .filter_map(|s| match *s {
                    Operand::Vgpr { first, count: 1 } => Some(first),
                    _ => None,
                })
                .collect();
            (target >= 32
                && sources.get(..lanes.len()) == Some(lanes.as_slice())
                && lanes.len() == 3)
                .then(|| target - 32)
        })
    }

    /// Whether the program interpolates parameter `param`.
    #[must_use]
    pub fn reads_attribute(&self, param: u8) -> bool {
        self.instructions
            .iter()
            .any(|i| matches!(i.detail, Detail::Interp { attr, .. } if attr == param))
    }

    /// Whether every lane of `attr{param}.xyz` is read only as a multiplicand.
    #[must_use]
    pub fn attribute_is_only_a_factor(&self, param: u8) -> bool {
        let mut live: Vec<u8> = Vec::new();
        let mut lanes = 0;
        for i in &self.instructions {
            if let Detail::Interp { attr, chan } = i.detail
                && attr == param
                && chan < 3
            {
                if let Some(Operand::Vgpr { first, .. }) = i.dst[0] {
                    if !live.contains(&first) {
                        live.push(first);
                    }
                    lanes += 1;
                }
                continue;
            }
            let read = i.src.iter().flatten().any(|s| touches(s, &live));
            if read && !multiplicand(i, &live) {
                return false;
            }
            if let Some(Operand::Vgpr { first, count }) = i.dst[0] {
                live.retain(|&r| r < first || r >= first + count);
            }
        }
        lanes >= 6
    }
}

fn touches(operand: &Operand, live: &[u8]) -> bool {
    matches!(*operand, Operand::Vgpr { first, count } if live.iter().any(|&r| (first..first + count).contains(&r)))
}

fn multiplicand(i: &Instruction, live: &[u8]) -> bool {
    let Some(name) = i.name else { return false };
    let product = ["mul", "mac", "mad", "fma"]
        .iter()
        .any(|p| name.contains(p));
    product
        && i.src
            .iter()
            .enumerate()
            .filter(|(_, s)| s.as_ref().is_some_and(|s| touches(s, live)))
            .all(|(k, _)| k < 2)
}

#[cfg(test)]
mod tests {
    use super::super::isa::decode_all;
    use super::*;

    const V: u32 = 256;

    fn interp(op: u32, vdst: u32, attr: u32, chan: u32) -> u32 {
        0b11_0010 << 26 | vdst << 18 | op << 16 | attr << 10 | chan << 8
    }

    fn vop2(op: u32, vdst: u32, src0: u32, vsrc1: u32) -> u32 {
        op << 25 | vdst << 17 | vsrc1 << 9 | src0
    }

    /// A pixel program whose three lanes of `attr2` land in `v4..v6` and are then used by
    /// `last`, a VOP2 reading `v4`.
    fn pixel(last: u32) -> Program {
        let mut words = Vec::new();
        for lane in 0..3 {
            words.push(interp(0, 4 + lane, 2, lane));
            words.push(interp(1, 4 + lane, 2, lane));
        }
        words.push(vop2(8, 8, V + 4, 9));
        words.push(vop2(8, 9, V + 5, 9));
        words.push(vop2(8, 10, V + 6, 9));
        words.push(last);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        Program {
            at: 0,
            code_at: 0,
            stage: Stage::Pixel,
            info: 0,
            instructions: decode_all(&bytes).expect("decodes"),
            reflection: Err(super::super::Error::StageUnread(0)),
            vertex_inputs: Ok(Vec::new()),
            usage: Vec::new(),
        }
    }

    #[test]
    fn lanes_read_only_by_multiplies_are_a_factor() {
        let program = pixel(vop2(8, 11, V + 4, 12));
        assert!(program.reads_attribute(2));
        assert!(!program.reads_attribute(3));
        assert!(program.attribute_is_only_a_factor(2));
    }

    #[test]
    fn a_lane_added_to_the_light_is_not_a_factor() {
        let program = pixel(vop2(3, 11, V + 4, 12));
        assert_eq!(
            program.instructions.last().and_then(|i| i.name),
            Some("v_add_f32")
        );
        assert!(!program.attribute_is_only_a_factor(2));
    }

    #[test]
    fn a_lane_overwritten_before_an_add_is_no_longer_read() {
        let mut program = pixel(vop2(3, 11, V + 4, 12));
        let before = program.instructions.len() - 1;
        let overwrite = decode_all(&vop2(8, 4, V + 13, 14).to_le_bytes()).expect("decodes");
        program.instructions.splice(before..before, overwrite);
        assert!(program.attribute_is_only_a_factor(2));
    }
}
