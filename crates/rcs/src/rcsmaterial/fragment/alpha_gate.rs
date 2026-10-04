//! The parameter a texture unit's alpha gates an additive term with.
//!
//! Split out of `fragment.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`.

use super::{Instruction, Program, Source};

impl Program {
    /// The parameter hash whose value the program multiplies by unit
    /// `unit`'s sampled **alpha** and accumulates, or `None` where no
    /// instruction does.
    ///
    /// The shape is `MAD dst, T.wwww, C, acc`: `T` is the register the
    /// unit's `TEX` wrote, read through a `.wwww` swizzle, and `C` an inline
    /// constant a parameter patches at draw time. It is how both HD pad
    /// programs add the `_ne` light-bar mask (`docs/rendering/pads.md`,
    /// "Which alpha channel gates the accumulate"): `Speedup Pad`'s is
    /// instruction 41 and `Weapon Pad`'s 43 on `12_sol_2`, each patched by
    /// a different parameter (`0x7611a2d8`, `0xce5c4410`) and, on the other
    /// circuits, `Colour` (`0x02ab9f07`) or `W_Cycle` - which is why the
    /// hash is read off the program rather than named here.
    ///
    /// The register is matched by exact `(index, half)`. On `Weapon Pad` a
    /// `TEX H0` lands between the `R0` write and its use, which a half
    /// register may alias; that cannot touch lane `w` of `R0`, and the
    /// disc's own program reads `R0.w` there, so the exact match is the
    /// program's own.
    #[must_use]
    pub fn alpha_gated_parameter(&self, unit: u8) -> Option<u32> {
        let tex_at = self
            .instructions
            .iter()
            .position(|i| i.is_texture() && i.unit == unit)?;
        let tex = &self.instructions[tex_at];
        let wanted = Source::Register {
            index: tex.dst,
            half: tex.dst_half,
        };
        let gated: &Instruction = self.instructions[tex_at + 1..].iter().find(|i| {
            i.name() == Some("MAD")
                && i.sources[0] == wanted
                && i.swizzles[0] == [3; 4]
                && i.sources[1] == Source::Constant
        })?;
        let slot = gated.const_slot?;
        self.parameter_patches
            .iter()
            .find(|&&(at, _)| at == slot)
            .map(|&(_, hash)| hash)
    }
}
