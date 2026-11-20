//! Which Omega material files write `1 - v` into their diffuse coordinate.
//!
//! A vertex program fetches `Uv1` into two vector registers `vU`, `vU+1`, and the pixel program
//! samples its diffuse texture at the interpolated parameter the vertex program exports. A
//! program that flips the coordinate computes `v_sub_f32 d, 1.0, vU+1` and exports `(vU, d)`:
//! Modesto Heights' `Track_Wall` starts `v_sub_f32 v0, 1.0, v9` and ends `exp param1 v8, v0, v1,
//! v1` (`docs/formats/omega-status.md`, "The diffuse coordinate that writes `1 - v`"). 2048's
//! Vita programs write the same `1 - v` (`gxp::texcoord_flip`).
//!
//! **Only that shape is read**: an `exp paramN` of a program that fetches `Uv1` whose first lane
//! is `vU`, untouched since the fetch, and whose second lane was last written by a plain
//! `v_sub_f32` of the literal 1.0 and `vU+1`, with `vU+1` itself untouched between the fetch and
//! that subtract. A flip folded into a multiply-add, or after a transform of the coordinate, reads
//! as not flipping: the direction that leaves a coordinate alone.
//!
//! [`flips_v`] is per file, never by name: a file flips when it has a program that fetches `Uv1`
//! and every such program flips.

use super::isa::{Detail, Operand};
use super::{Program, Stage};

/// The vertex input the diffuse coordinate is.
const UV: &str = "Uv1";

/// What [`flips_v`] counted in one material file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TexcoordFlip {
    /// Vertex programs that fetch `Uv1`.
    pub programs: usize,
    /// Of those, programs that export `(u, 1 - v)`.
    pub flipping: usize,
}

impl TexcoordFlip {
    /// At least one program fetches `Uv1` and every one that does flips it.
    #[must_use]
    pub fn is_flip(&self) -> bool {
        self.programs > 0 && self.flipping == self.programs
    }

    /// Some programs flip and some do not: the file is left alone, and worth a look.
    #[must_use]
    pub fn is_mixed(&self) -> bool {
        self.flipping > 0 && self.flipping < self.programs
    }
}

impl Program<'_> {
    /// Whether the program fetches `Uv1` and exports `(u, 1 - v)` of it to a parameter.
    /// `None` when it fetches no `Uv1`.
    #[must_use]
    pub fn exports_flipped_uv(&self) -> Option<bool> {
        let (input, _) = self.input_named(UV)?;
        if input.size < 2 {
            return Some(false);
        }
        let (u, v) = (input.vgpr, input.vgpr + 1);
        Some(self.instructions.iter().enumerate().any(|(k, i)| {
            let Detail::Export { target, en, .. } = i.detail else {
                return false;
            };
            if target < 32 || en & 0b11 != 0b11 {
                return false;
            }
            let (Some(Operand::Vgpr { first: x, count: 1 }), Some(Operand::Vgpr { first: y, .. })) =
                (i.src[0], i.src[1])
            else {
                return false;
            };
            x == u && self.writer_of(k, u).is_none() && self.is_one_minus(k, y, v)
        }))
    }

    /// Whether register `y`, as read by instruction `k`, was last written by
    /// `v_sub_f32 y, 1.0, v` with `v` still the fetched coordinate.
    fn is_one_minus(&self, k: usize, y: u8, v: u8) -> bool {
        let Some((at, sub)) = self.writer_of(k, y) else {
            return false;
        };
        sub.name == Some("v_sub_f32")
            && sub.src[0] == Some(Operand::Float(1.0))
            && sub.src[1] == Some(Operand::Vgpr { first: v, count: 1 })
            && sub.neg == 0
            && sub.abs == 0
            && sub.omod == 0
            && !sub.clamp
            && self.writer_of(at, v).is_none()
    }
}

/// Reads a `.rcsmaterial` file's vertex programs for the `(u, 1 - v)` export of `Uv1`.
#[must_use]
pub fn flips_v(file: &[u8]) -> TexcoordFlip {
    let mut found = TexcoordFlip::default();
    for (_, program) in super::scan(file) {
        let Ok(program) = program else { continue };
        if program.stage != Stage::Vertex {
            continue;
        }
        if let Some(flips) = program.exports_flipped_uv() {
            found.programs += 1;
            found.flipping += usize::from(flips);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::super::isa::decode_all;
    use super::*;
    use crate::gcn::{Attribute, Reflection, VertexInput};

    fn vop2(op: u32, vdst: u32, src0: u32, vsrc1: u32) -> u32 {
        op << 25 | vdst << 17 | vsrc1 << 9 | src0
    }

    fn vop1(op: u32, vdst: u32, src0: u32) -> u32 {
        0x7e00_0000 | vdst << 17 | op << 9 | src0
    }

    /// `exp param<target> v<x>, v<y>, v<z>, v<w>` with all four lanes enabled.
    fn export(target: u32, lanes: [u32; 4]) -> [u32; 2] {
        [
            0xf800_0000 | (32 + target) << 4 | 0xf,
            lanes[3] << 24 | lanes[2] << 16 | lanes[1] << 8 | lanes[0],
        ]
    }

    /// A vertex program fetching `Uv1` into `v8..v9`, running `words`.
    fn vertex(words: &[u32]) -> Program<'static> {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let uv = Attribute {
            kind: 1 << 24 | 1,
            name: UV.to_string(),
            semantic: String::new(),
        };
        Program {
            at: 0,
            code_at: 0,
            code: &[],
            stage: Stage::Vertex,
            info: 0,
            instructions: decode_all(&bytes).expect("decodes"),
            reflection: Ok(Reflection {
                inputs: vec![uv],
                ..Reflection::default()
            }),
            vertex_inputs: Ok(vec![VertexInput {
                semantic: 1,
                vgpr: 8,
                size: 2,
                unread: 0,
            }]),
            usage: Vec::new(),
        }
    }

    // `v_sub_f32` is VOP2 op 4; `v_subrev_f32` op 5; `v_mov_b32` is VOP1 op 1. The inline 1.0 is
    // source code 242.
    const ONE: u32 = 242;

    #[test]
    fn the_flip_shape_is_one_minus_the_fetched_v_exported_beside_the_fetched_u() {
        let mut words = vec![vop2(4, 0, ONE, 9)];
        words.extend(export(1, [8, 0, 1, 1]));
        assert_eq!(vertex(&words).exports_flipped_uv(), Some(true));
    }

    #[test]
    fn a_v_overwritten_before_the_subtract_is_not_the_fetched_coordinate() {
        let mut words = vec![vop1(1, 9, 128), vop2(4, 0, ONE, 9)];
        words.extend(export(1, [8, 0, 1, 1]));
        assert_eq!(vertex(&words).exports_flipped_uv(), Some(false));
    }

    #[test]
    fn a_v_overwritten_after_the_subtract_still_flips() {
        let mut words = vec![vop2(4, 0, ONE, 9), vop1(1, 9, 128)];
        words.extend(export(1, [8, 0, 1, 1]));
        assert_eq!(vertex(&words).exports_flipped_uv(), Some(true));
    }

    #[test]
    fn the_unflipped_export_and_the_wrong_subtract_do_not_flip() {
        let mut direct = Vec::new();
        direct.extend(export(1, [8, 9, 1, 1]));
        assert_eq!(vertex(&direct).exports_flipped_uv(), Some(false));
        // `v_subrev_f32 v0, 1.0, v9` is `v9 - 1`.
        let mut reversed = vec![vop2(5, 0, ONE, 9)];
        reversed.extend(export(1, [8, 0, 1, 1]));
        assert_eq!(vertex(&reversed).exports_flipped_uv(), Some(false));
        // Another register than the fetched `v`.
        let mut other = vec![vop2(4, 0, ONE, 10)];
        other.extend(export(1, [8, 0, 1, 1]));
        assert_eq!(vertex(&other).exports_flipped_uv(), Some(false));
    }

    #[test]
    fn a_program_without_uv1_reports_none() {
        let mut program = vertex(&export(1, [8, 9, 1, 1]));
        program.vertex_inputs = Ok(Vec::new());
        assert_eq!(program.exports_flipped_uv(), None);
    }

    #[test]
    fn a_file_flips_when_every_program_that_fetches_uv1_does() {
        let found = TexcoordFlip {
            programs: 3,
            flipping: 3,
        };
        assert!(found.is_flip() && !found.is_mixed());
        let mixed = TexcoordFlip {
            programs: 3,
            flipping: 1,
        };
        assert!(!mixed.is_flip() && mixed.is_mixed());
        assert!(!TexcoordFlip::default().is_flip());
    }
}
