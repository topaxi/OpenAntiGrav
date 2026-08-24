//! [`Flame`]: the five numbers Wipeout HD's engine-flare material authors, and
//! what its own fragment program does with them.
//!
//! **Every value here is read off the disc, none is fitted.** The program is
//! `data/materials/ships/engines/flame_test.rcsmaterial`, disassembled
//! instruction by instruction on
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`; the *values* it
//! patches into its inline constants are in the material record inside each
//! craft's own `engineflare.rcsmodel`, which
//! [`oag_formats::rcsmodel::material::parameters`] decodes.
//!
//! The whole program, with the patch chain resolved (`fslot` -> entry index ->
//! code slot) so each constant is attributed rather than guessed:
//!
//! ```text
//! rim   = saturate(1 - dot(normalize(V), normalize(N)))
//! f     = pow(rim, power1) * scale1 + min1
//! alpha = alpha_scale * (1 - f) * VertexColour1.w
//! rgb   = texture(uv).rgb * colour_scale
//! ```
//!
//! and the output is that `(rgb, alpha)` under the material's own additive
//! blend. Three things are worth saying plainly about it:
//!
//! - **The sampled alpha is not used.** The program's `TEX H0.xyz` writes three
//!   components; the fourth is `VertexColour1.w` from the moment `MOV H0.w,
//!   f[TC2]` puts it there. Multiplying the texture's alpha in - which is what
//!   the generic path does - is what kept the flame at roughly a third of its
//!   brightness.
//! - **The rim term is a modest modulation, not the shape.** With the shipped
//!   numbers it spans `2*(1-0.45)` = 1.1 face-on to `2*(1-0.75)` = 0.5
//!   edge-on. The flame's shape is the vertex ramp; this is a 2.2x tilt across
//!   it.
//! - **The normal and the view vector are the model's, not the world's.** HD's
//!   vertex program dots an untransformed `normal` attribute against
//!   `eyePositionWorldSpace - position`, which only makes sense if the engine
//!   passes the eye in model space; this renderer computes the same angle in
//!   world space, from the world normal and the real camera, which is the same
//!   number under a rigid transform.
//!
//! **The surface scroll is drawn as of 2026-08-24**, now that `time`'s provider
//! is read: it is entry 0 of the executable's own 81-name engine shader
//! parameter table, and every draw-state builder writes the frame context's
//! global seconds clock into it - `renderer.md`, "the engine's own parameter
//! table". So the program's two taps are both known and both drawn:
//!
//! ```text
//! noise = texture(u, v + Speed * time).a      <- Speed 2.0, authored
//! rgb   = texture(2u, 2v + noise).rgb * colour_scale
//! ```
//!
//! The doubling on the colour tap is the program's, not a misread: block #1 and
//! block #2 of `flame_test.rcsmaterial` schedule it through entirely different
//! registers and arrive at the same pair. This renderer's clock is the race
//! tick over 60, never the wall clock, so a replay of the same tick draws the
//! same frame - the same clock the scenery's texture animation already rides.

use oag_formats::rcsmodel;

/// `!crc32("power1")` - the rim exponent.
pub const POWER1: u32 = 0xaa5e_39a1;
/// `!crc32("scale1")` - what the rim term is scaled by before the floor.
pub const SCALE1: u32 = 0x961a_1154;
/// `!crc32("min1")` - the floor added to it.
pub const MIN1: u32 = 0xfa51_b981;

/// The parameter that scales the whole alpha term, patched into the inline
/// constant of `MAD H1.w, -H0.xxxx, {c}.xxxx, {c}.xxxx`.
///
/// **No preimage yet.** Two wordlist passes over the obvious spellings found
/// none, and a hash with no name is still a value with a code slot: what it
/// does is read out of the instruction that uses it, not out of its name.
pub const ALPHA_SCALE: u32 = 0x92fc_84bf;

/// The parameter that multiplies the sampled colour, at `MUL H0.xyz, H0, {c}`.
///
/// No preimage either; see [`ALPHA_SCALE`].
pub const COLOUR_SCALE: u32 = 0x17d9_b3d3;

/// `!crc32("Speed")` - what the surface scroll's clock is multiplied by.
///
/// 2.0 on all fourteen craft, so the noise tap advances two whole texture
/// repeats a second against the engine's own seconds clock.
pub const SPEED: u32 = 0x3118_2e0d;

/// What one craft's flare material says its flame looks like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flame {
    /// [`POWER1`]: the exponent the rim term is raised to. 10.0 on the disc.
    pub rim_power: f32,
    /// [`SCALE1`]: 0.3 on the disc.
    pub rim_scale: f32,
    /// [`MIN1`]: 0.45 on the disc.
    pub rim_min: f32,
    /// [`ALPHA_SCALE`]: 2.0 on the disc.
    pub alpha_scale: f32,
    /// [`COLOUR_SCALE`]: 1.0 on the disc.
    pub colour_scale: f32,
    /// [`SPEED`]: 2.0 on the disc. Multiplies `time` - engine parameter slot
    /// 0, a global seconds clock - into the noise tap's `v`.
    pub scroll_speed: f32,
}

impl Flame {
    /// Reads all five off a material's own parameter table, or answers `None`.
    ///
    /// **All five or nothing**, and that is the gate this whole shading path
    /// hangs on: a material that does not declare the set the flame program
    /// reads is not a flame, and filling a missing one in with the number the
    /// disc happens to use elsewhere would be exactly the invented constant
    /// this project keeps out. A caller that gets `None` draws the model
    /// through the ordinary path and says so.
    #[must_use]
    pub fn from_material(material: &rcsmodel::Material) -> Option<Self> {
        let value = |hash| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value[0])
        };
        Some(Self {
            rim_power: value(POWER1)?,
            rim_scale: value(SCALE1)?,
            rim_min: value(MIN1)?,
            alpha_scale: value(ALPHA_SCALE)?,
            colour_scale: value(COLOUR_SCALE)?,
            scroll_speed: value(SPEED)?,
        })
    }

    /// One line for a load report, naming every number and where it came from.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "flame shading off the material's own parameters: alpha = {:.2} * (1 - \
             (rim^{:.0} * {:.2} + {:.2})) * the vertex ramp, colour = texture x {:.2}, \
             sampled alpha unused, noise tap scrolling at {:.2} x the engine clock \
             - the program's own arithmetic, no fitted term",
            self.alpha_scale,
            self.rim_power,
            self.rim_scale,
            self.rim_min,
            self.colour_scale,
            self.scroll_speed
        )
    }
}

#[cfg(test)]
mod tests;
