//! Whether a texture unit is read as a tangent-space normal.

use super::{Program, Source};

impl Program {
    /// Whether `unit` is sampled as a normal map: some fetch of it writes at
    /// least three lanes and the instruction that follows decodes them with
    /// `MAD r, tap, {2, -1, ..}.xxxx, {2, -1, ..}.yyyy`, that is `2 * tap - 1`.
    ///
    /// **A property of the program, not of the picture's name.** `waves2.gtf`
    /// (Vineta K's sea) is read this way at three coordinates by
    /// `water_noref`, and nothing in its sampler hash says so. The shape is
    /// the one every normal map this project has identified is decoded with
    /// (`docs/formats/rcsmaterial.md`, "Water family").
    #[must_use]
    pub fn samples_as_normal(&self, unit: u8) -> bool {
        self.instructions.iter().enumerate().any(|(i, tap)| {
            tap.is_texture()
                && tap.unit == unit
                && (tap.mask & 0b0111).count_ones() >= 3
                && self.instructions[i + 1..].iter().take(2).any(|next| {
                    next.name() == Some("MAD")
                        && next.sources[0]
                            == Source::Register {
                                index: tap.dst,
                                half: tap.dst_half,
                            }
                        && matches!(next.constant, Some([two, minus_one, ..])
                            if (two - 2.0).abs() < 1e-6 && (minus_one + 1.0).abs() < 1e-6)
                        && next.swizzles[1][0] == 0
                        && next.swizzles[2][0] == 1
                })
        })
    }
}
