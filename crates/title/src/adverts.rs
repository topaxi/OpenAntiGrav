//! How a title draws the adverts its circuits' billboard slots show.
//!
//! A circuit's `trackstartup.xml` names up to eight slots; on Pulse and Wipeout HD
//! the engine draws each slot's model through the model's own `Camera` node into
//! a small offscreen target, and the track's `billboardN` quad samples that
//! target. What differs per title is the target, the clip planes and whether the
//! colour-slot pool is read, and each is a measurement, so each lives here rather
//! than as a constant in the renderer. `None` on [`crate::Title::adverts`] is a
//! title whose adverts are not drawn - not measured, or not shipped - and its
//! placeholder quads draw nothing.

/// What one title's advert pass measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adverts {
    /// The offscreen target every advert is drawn into, in pixels.
    ///
    /// Pulse: 128 x 128 (`FUN_0891fe14(object + 0x98, 0x80, 0x80)`, and the
    /// `FBPTR` width of a live GE dump). Wipeout HD: 512 x 256 (the literal
    /// viewport in `Billboard_UpdateAndRender`, and the `0x200` / `0x100` of the
    /// target object read live on RPCS3).
    pub target: (u32, u32),
    /// The near plane of the advert's projection.
    pub near: f32,
    /// The far plane of the advert's projection.
    pub far: f32,
    /// Whether a slot that names a colour draws an advert from the engine's pool
    /// (`oag_tables::billboard_pool`). `false` is a title whose pool order is not
    /// measured: those slots stay undrawn and the load report names them.
    pub colour_pool: bool,
    /// Whether the track draws that show a target sample it with their V
    /// coordinate negated.
    ///
    /// Pulse does: a live PPSSPP dump of Talon's Junction puts `TEXSCALE (1, -1)`
    /// and `TEXOFFSET (0, 0)` on the prims that read the 128 x 128 target and
    /// `(1, 1)` or an authored animation scale on every other textured prim
    /// (547 prims). The target holds the picture top row first and the track's
    /// UVs run V up, so the engine flips it on exactly those draws. HD authors
    /// its billboard quads with V running down (read off the track's vertices), so
    /// it needs none.
    pub flip_v: bool,
    /// Whether the track draws that show a target sample it with its texture
    /// coordinates wrapping, rather than clamped to the target's edge.
    ///
    /// Wipeout HD: **measured on RPCS3 (2026-10-18, `03_track`, one hooked
    /// boot)**: both draws that read a 512 x 256 `R5G6B5` target carry texture
    /// address `0x60010101` (wrap on U, V and W) and filter `0x02062000` (linear
    /// magnification, `LINEAR_MIPMAP_LINEAR` minification over the target's one
    /// level, LOD bias 0), with no anisotropy in the control word. A track quad
    /// whose coordinates run past 0..1 (a side panel tiling the banner) shows
    /// the picture repeated, where a clamp shows the edge texel smeared. Pulse
    /// is `true`, **measured on PPSSPP (2026-10-18)**: every draw that samples
    /// its 128 x 128 target has `TEXWRAP` 0 (repeat on U and V) and filter
    /// `0x107` (linear, one level) in the GE list.
    pub wrap: bool,
    /// Whether a Zone race swaps the per-slot adverts for one shared texture.
    ///
    /// Wipeout HD's `Billboard_LoadModelAndBind` takes a second branch while the
    /// zone effects are on: it binds a single `"billboard"` texture to every slot
    /// but 8 instead of building a target per slot. What that texture is has not
    /// been read, so on such a title a Zone race draws no advert and says so.
    pub zone_shares_one_texture: bool,
    /// Whether slot 8 - the start gantry - is drawn as a card like the other
    /// slots instead of as a model stood on the track's mount.
    ///
    /// Wipeout Pulse (both pressings): **measured on PPSSPP** - the 2026-10-06 GE dump
    /// draws the gantry as pass B into the 128 x 128 target, and the software
    /// renderer shows the board opaque (2026-10-18).
    ///
    /// Wipeout HD: **measured on RPCS3 (2026-10-08, Talon's Junction, one hooked
    /// boot)**: the frame's second 512 x 256 colour target is drawn with
    /// `321_go_64.gtf` (64 x 128 DXT5, 8 mips, trilinear) through a
    /// `texture * constant` program, and the track's `billboard8` quad then
    /// samples it through the ordinary lit track program. A lit `GO` read 132 of
    /// 255 on screen against a full white from the model route, whose white sits
    /// above the bloom gate's knee and smears into a halo.
    pub gantry_card: bool,
    /// The board slot 8 shows instead of the circuit's own while the race is a
    /// Zone race, or `None` where the title keeps one gantry for every mode.
    pub zone_gantry: Option<ZoneGantry>,
    /// Where the numbers came from.
    pub origin: crate::Origin,
}

/// A Zone race's own start board.
///
/// **Wipeout HD, measured on RPCS3 (2026-10-08, Vineta K, one boot per mode):**
/// `GetMode()` 6 shows a board of its own before the release - a dark panel on
/// which a loop icon assembles from light pieces - where modes 3 and 8 show
/// `3 2 1 GO`. Confidence 85 for "the mode picks another file"; the file is
/// inferred (80) from the picture, the names on the disc and the PS4 binary's
/// mode table (`docs/ghidra/functions/ps4-omega-eu/billboards.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneGantry {
    /// The model's archive path, spelled as a `trackstartup.xml` names one.
    pub entry: &'static str,
    /// Ticks the board's assembly runs before the release (HD: 210, read off the
    /// capture as about 3.5 s; one boot, so a few ticks either way).
    pub assembly_ticks: u64,
}
