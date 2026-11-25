//! The two ways the ribbon's colours reach the screen.

/// How the ribbon's colours reach the screen: the PSP's law, or the PS2's.
///
/// The ribbon is the same preset on both (`Trail_InitPreset`, preset 2, and the PS2
/// build's `0x001f0090`), drawn by the same ring walk. What differs is the last step.
/// The PSP's GE multiplies the baked vertex colours by the per-frame grey ramp
/// (`sceGuAmbient`), which is zero at rest. The PS2's draw (`0x001f0a08`) never reads
/// the ramp: the baked colours go to the GS as they are, so the ribbon shows at rest,
/// and its submit loop skips layer 0, the PSP's stencil-stamped one. See
/// `docs/ghidra/functions/ps2-pulse-eu/engine-trail.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailLaw {
    /// The PSP: every layer, scaled by [`super::Exhaust::layer_alphas`].
    Ramped,
    /// The PS2: layers from [`PS2_FIRST_LAYER`], the baked colours unramped at the
    /// GS's own scale ([`PS2_GS_COLOUR_SCALE`]), stamping no glow mask.
    Baked,
}

impl TrailLaw {
    /// The law a race draws: [`Self::Baked`] where the title takes the PS2's trail
    /// (`oag_title::Looks::ps2_trail`), [`Self::Ramped`] otherwise.
    #[must_use]
    pub fn for_ps2(ps2_trail: bool) -> Self {
        if ps2_trail { Self::Baked } else { Self::Ramped }
    }

    /// The first layer this law submits.
    #[must_use]
    pub const fn first_layer(self) -> usize {
        match self {
            Self::Ramped => 0,
            Self::Baked => PS2_FIRST_LAYER,
        }
    }

    /// The mean colour this law's ribbon adds over its own footprint, for a ribbon texture whose
    /// mean colour is `texel`: each submitted layer's authored colour ([`super::LAYER_COLOUR`])
    /// times the texel and the layer's ramp at full intensity, weighted by the width it covers
    /// ([`super::LAYER_WIDTH`]), and summed, since the layers blend additively. Only the hue is
    /// a ribbon's own; its brightness follows the intensity, so this is not a vertex colour. The
    /// PSP's wide deep-blue layer makes it violet; the PS2 leaves that layer out and it is
    /// pink-magenta (`docs/formats/pulse-absent-effects.md`, "Trail-hit burst colour").
    #[must_use]
    pub fn ribbon_colour(self, texel: [f32; 3]) -> [f32; 3] {
        let ramps = match self {
            Self::Ramped => super::layer_alphas_at(1.0),
            Self::Baked => [PS2_GS_COLOUR_SCALE; super::LAYERS],
        };
        let mut sum = [0.0; 3];
        for (layer, ramp) in ramps.iter().enumerate().skip(self.first_layer()) {
            let covered = ramp * super::LAYER_WIDTH[layer];
            for (c, total) in sum.iter_mut().enumerate() {
                *total += super::LAYER_COLOUR[layer][c] * texel[c] * covered;
            }
        }
        sum
    }
}

/// The first layer the PS2's draw submits: its loop's `index != 0` test leaves out
/// layer 0, the widest one (`LAYER_WIDTH[0]`, the blue `LAYER_COLOUR[0]`).
pub const PS2_FIRST_LAYER: usize = 1;

/// A vertex colour byte on the GS is `0x80 = 1.0`, not `0xff`, and the ribbon's
/// texture function is modulate (`TEX0.TFX = 0`), so the baked `210` the PS2 sends is
/// `210 / 128 = 1.64` times the texel. Our colours are `byte / 255`; this carries them
/// to `byte / 128`. Derived from the GS register layout, not tuned.
pub const PS2_GS_COLOUR_SCALE: f32 = 255.0 / 128.0;
