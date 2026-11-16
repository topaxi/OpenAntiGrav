//! Whether the sun's `N.L` reaches the diffuse light unclamped.

use super::{Instruction, PRECISION_SATURATE, Program, Source};
use crate::rcsmaterial::SUN_DIRECTION;

impl Program {
    /// Whether the program multiplies the sun by an **unsaturated** `N.L`: a `DP3` against the
    /// `directionalLight0DirectionWorldSpace` constant with no `_SAT`, whose result a later
    /// unsaturated `MUL`, `MAD` or `ADD` reads (the diffuse sun term, `sun * (lm.a * N.L)`).
    ///
    /// A reader that saturates its result or the source it reads the dot through (the `SRC1`
    /// precision modifier, `PRECISION_SATURATE`) does not count: every lit program also reads the same `DP3` through
    /// `MOV_SAT x8` for the specular's facing term, and a program that saturates its diffuse term
    /// dots again with `DP3_SAT`. Walks the result's lanes through the program in order and stops
    /// where a later instruction overwrites all of them.
    ///
    /// `docs/formats/rcsmaterial.md`, "The sun's N.L: which lightmapped programs leave it
    /// unsaturated".
    #[must_use]
    pub fn sun_diffuse_unsaturated(&self) -> bool {
        let sun: Vec<u16> = self.patches(SUN_DIRECTION).collect();
        self.instructions.iter().enumerate().any(|(i, dot)| {
            dot.name() == Some("DP3")
                && !dot.saturate
                && dot.const_slot.is_some_and(|slot| sun.contains(&slot))
                && self.unsaturated_reader(i)
        })
    }

    /// Whether an unsaturated `MUL`, `MAD` or `ADD` reads a lane the `DP3` at `at` wrote, before
    /// the register's lanes are written again.
    fn unsaturated_reader(&self, at: usize) -> bool {
        let dot = &self.instructions[at];
        let reg = (dot.dst, dot.dst_half);
        for later in &self.instructions[at + 1..] {
            let reads = later.operands().enumerate().any(|(k, source)| {
                matches!(source, Source::Register { index, half } if (index, half) == reg)
                    && read_lanes(later, k) & dot.mask != 0
                    && later.source_precision[k] != PRECISION_SATURATE
            });
            if reads && !later.saturate && matches!(later.name(), Some("MUL" | "MAD" | "ADD")) {
                return true;
            }
            if (later.dst, later.dst_half) == reg && later.mask & dot.mask == dot.mask {
                return false;
            }
        }
        false
    }
}

/// The lanes of its register that operand `k` of `insn` reads, as a mask: the swizzled lane of
/// each output component, or the first three (four for `DP4`) for a dot product.
fn read_lanes(insn: &Instruction, k: usize) -> u8 {
    let swizzle = insn.swizzles[k];
    let components = match insn.name() {
        Some("DP3") => 0b0111,
        Some("DP4") => 0b1111,
        _ => insn.mask,
    };
    (0..4)
        .filter(|c| components & (1 << c) != 0)
        .fold(0u8, |lanes, c| lanes | 1 << swizzle[c])
}
