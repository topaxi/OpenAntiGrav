//! [`Emissive`]: the additive glow one Wipeout HD material carries.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What decides
//! *which* materials get one is `super::rcs::emissive`, and the shader half is
//! `mesh_render::Emissives`.

/// One material's additive glow layer: what Wipeout HD's emissive family
/// samples from unit 1, and how it moves.
///
/// The fragment programs compute `unit1(u, (v + a) * b + time) * tint` and add
/// it to the albedo, gated by the diffuse alpha - see
/// [`slots::ADD_SECOND`](slots::ADD_SECOND) and, for the microcode,
/// `docs/formats/rcsmaterial.md`. All four numbers are authored per material
/// and none can be defaulted away: over 311 records the disc carries **37
/// distinct tints** (only 108 of them white) and **34 distinct rates**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emissive {
    /// The float3 the sample is multiplied by, parameter `0xe8bcd7f5`. No
    /// preimage yet; `crates/render/examples/hd_param_names.rs` is the sweep.
    pub tint: [f32; 3],
    /// `a`, parameter `0x78256a45`: what is added to `v` before the scale.
    pub offset: f32,
    /// `b`, parameter `0x78787596`: what that sum is scaled by. The clock is
    /// added *after* it, so the surface scrolls one texture unit per second
    /// whatever `b` is, and `b` is what sets how much of the texture that is.
    ///
    /// **Defaults to `1.0`, never `0.0`.** Zero is the most destructive value
    /// in the range rather than a neutral one: it annihilates the surface's own
    /// `v`, leaving every pixel on one row of the texture and that row marching
    /// down it once the clock advances.
    pub scale: f32,
    /// Whether the clock moves this layer: `1.0` where the material authored
    /// its own `a`/`b` pair and `0.0` where it did not.
    ///
    /// **Not the same question as whether the program declares `time`.** A
    /// material can read the clock and still keep its coordinate constants as
    /// the shader's own inline literals rather than in its record. Such a layer
    /// is drawn still, which under-claims rather than sliding a texture at a
    /// rate nobody read. (`nr_billboardholographicscanlines` is the example the
    /// first cut named; all three of its shapes now have their own path with
    /// the rates read, `mesh::rcs::holo_billboard`.)
    pub rate: f32,
    /// A program-shaped reading of the layer, `[mode, glow u scale]`; `[0, 0]`
    /// is the additive glow every field above describes. The mode is one of
    /// [`PROGRAM_BILLBOARD`] and [`PROGRAM_BW_GLASS`], read by `shade.wesl` off
    /// the entry the vertex's material index names; the second lane is the
    /// glow tap's `u` scale (zero means 1), which `a`/`b` have no room for.
    ///
    /// **A lane of its own rather than a `rate` band**: every band of `rate`
    /// above 1.5 is read by some test with no role bit in front of it (the
    /// Zone exemption, the hull probe), and the clock multiplies `rate` too.
    pub program: [f32; 2],
}

/// [`Emissive::program`]'s mode for `cf_vex_billboard`: the first texture
/// multiplied by `k * (glow.a + light * glow.rgb)` instead of the glow being
/// added. See `super::rcs::billboard_glass`.
pub const PROGRAM_BILLBOARD: f32 = 1.0;

/// [`Emissive::program`]'s mode for `glasstest`: the second texture
/// (`bwgradient`, at the second coordinate set) multiplies the lit sum and the
/// specular instead of being added. See `super::rcs::billboard_glass`.
pub const PROGRAM_BW_GLASS: f32 = 2.0;

/// The `rate` an entry of [`Model::emissive`](super::Model::emissive) carries
/// to say "this surface takes no Zone grade": a value no material's own entry
/// reaches (rates are `0`, `1` and the water family's `2`), read by
/// `shade.wesl` off the entry the vertex's material index names.
///
/// **Chosen, not measured.** Wipeout HD's Zone variant of the start gantry's
/// quad material (`sign_emissive_glow`, block #7) samples no albedo at all, yet
/// the original's Zone races show the gantry's board on that quad (RPCS3,
/// 2026-10-08), so the blanket Zone recolour `shade.wesl` applies to every
/// surface is switched off for the quad that shows a card. How the original
/// keeps the board is unread; see `docs/rendering/start-gantry.md`.
pub const ZONE_EXEMPT_RATE: f32 = 4.0;

/// The `rate` of an entry that marks a hull material reading the reflection probe, with
/// the material's own reflectivity in `tint.x`: read by `shade.wesl` off the entry the
/// vertex's material index names. See `super::rcs::probe_lit`.
///
/// **A value above [`ZONE_EXEMPT_RATE`]**, so the Zone-exemption test is a band
/// (`3.5 < rate < 7.5`) and a reflective hull still takes the Zone grade.
pub const REFLECTION_RATE: f32 = 8.0;

/// The `rate` of an entry that marks scenery glass reading the circuit's sky reflection map
/// (`dc_windows_a`'s block #3), the material's own weight (`0x5e5b1937`) in `tint.x`: read by
/// `shade.wesl` off the entry the vertex's material index names. See `super::rcs::window_glass`.
pub const WINDOW_REFLECTION_RATE: f32 = 9.0;
