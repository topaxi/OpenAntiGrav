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

    /// Whether `unit` is sampled as a **two-lane** normal map: a fetch that
    /// writes its green and its alpha and nothing else, decoded by the same
    /// `2 * tap - 1`, with `x` read from the alpha and `y` from the green
    /// (`MAD r.zw, tap.wywy, ..`). The program rebuilds `z` as
    /// `sqrt(1 - x^2 - y^2)` before the frame.
    ///
    /// The DATA02 circuits' compile of `diffuse_normal_specular` reads its `_n`
    /// file this way (Moa Therma block #7 `@0x9960`: `TEX R0.yw` then `MAD R3.zw,
    /// R0.wywy, {2, -1}.xxxx, {2, -1}.yyyy`), where the four original circuits'
    /// compile reads three lanes ([`Self::samples_as_normal`]).
    /// `docs/formats/rcsmaterial.md`, "The two-lane normal map".
    #[must_use]
    pub fn samples_as_packed_normal(&self, unit: u8) -> bool {
        self.instructions.iter().enumerate().any(|(i, tap)| {
            tap.is_texture()
                && tap.unit == unit
                && tap.mask & 0b1111 == 0b1010
                && self.instructions[i + 1..].iter().take(2).any(|next| {
                    next.name() == Some("MAD")
                        && next.sources[0]
                            == Source::Register {
                                index: tap.dst,
                                half: tap.dst_half,
                            }
                        // `x` from the alpha, `y` from the green, on the two lanes written.
                        && written_lanes(next.mask)
                            .map(|lane| next.swizzles[0][lane])
                            .eq([3, 1])
                        && matches!(next.constant, Some([two, minus_one, ..])
                            if (two - 2.0).abs() < 1e-6 && (minus_one + 1.0).abs() < 1e-6)
                        && next.swizzles[1][0] == 0
                        && next.swizzles[2][0] == 1
                })
        })
    }
}

/// The lanes a write mask names, in order.
fn written_lanes(mask: u8) -> impl Iterator<Item = usize> {
    (0..4).filter(move |lane| mask & (1 << lane) != 0)
}
