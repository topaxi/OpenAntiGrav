//! What a fragment program does with an interpolator, read for the hull light.

use super::fragment::{Program, Source};

impl Program {
    /// Whether interpolator `TC<n>` reaches this program's output colour **only
    /// as a factor**: every instruction that reads it is a `MOV` or a `MUL`,
    /// and it is read at least once and reaches the output as a value
    /// ([`Self::output_lit_by`]).
    ///
    /// The shape of `diffuse_vcol`'s `MUL H2.xyz, f[TC0], H0` - the vertex
    /// colour scaling the light - against a baked-light interpolator, which a
    /// lit program `ADD`s or `MAD`s into the light sum. A `MOV` is allowed
    /// because the larger ship programs copy the interpolator into a register
    /// before the `MUL`.
    #[must_use]
    pub fn input_is_only_a_factor(&self, n: u32) -> bool {
        let Ok(input) = u8::try_from(4 + n) else {
            return false;
        };
        let mut reads = self
            .instructions
            .iter()
            .filter(|i| i.input == input && i.operands().any(|s| s == Source::Input))
            .peekable();
        reads.peek().is_some()
            && self.output_lit_by() & (1 << input) != 0
            && reads.all(|i| matches!(i.name(), Some("MOV" | "MUL")))
    }

    /// Whether the program has a `LG2` at all - the `log2` of a `pow(N.H, e)`
    /// specular chain. A program with none carries no specular term, whatever
    /// the shared stand-in exponent would say.
    #[must_use]
    pub fn has_specular_chain(&self) -> bool {
        self.instructions.iter().any(|i| i.name() == Some("LG2"))
    }
}
