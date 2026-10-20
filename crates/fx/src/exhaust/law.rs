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

/// The first layer the PS2's draw submits: its loop's `index != 0` test leaves out
/// layer 0, the widest one (`LAYER_WIDTH[0]`, the blue `LAYER_COLOUR[0]`).
pub const PS2_FIRST_LAYER: usize = 1;

/// A vertex colour byte on the GS is `0x80 = 1.0`, not `0xff`, and the ribbon's
/// texture function is modulate (`TEX0.TFX = 0`), so the baked `210` the PS2 sends is
/// `210 / 128 = 1.64` times the texel. Our colours are `byte / 255`; this carries them
/// to `byte / 128`. Derived from the GS register layout, not tuned.
pub const PS2_GS_COLOUR_SCALE: f32 = 255.0 / 128.0;
