//! How a title draws the adverts its circuits' billboard slots show.
//!
//! A circuit's `trackstartup.xml` names up to eight slots; on Pulse and HD the
//! engine draws each slot's model through the model's own `Camera` node into a
//! small offscreen target, and the track's `billboardN` quad samples it. The
//! target, clip planes and whether the colour-slot pool is read differ per title
//! and are each a measurement, so they live here. `None` on
//! [`crate::Title::adverts`] is a title whose adverts are not drawn (not measured,
//! or not shipped); its placeholder quads draw nothing. Every measurement below is
//! on `docs/rendering/start-gantry.md`.

/// What one title's advert pass measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adverts {
    /// The offscreen target every advert is drawn into, in pixels. Pulse: 128 x 128
    /// (`FUN_0891fe14(object + 0x98, 0x80, 0x80)` and a live GE dump's `FBPTR`
    /// width). HD: 512 x 256 (the viewport literal in `Billboard_UpdateAndRender`
    /// and the target object read live on RPCS3).
    pub target: (u32, u32),
    /// The near plane of the advert's projection.
    pub near: f32,
    /// The far plane of the advert's projection.
    pub far: f32,
    /// Whether a slot that names a colour draws an advert from the engine's pool
    /// (`oag_tables::billboard_pool`). `false` where the pool order is not
    /// measured: those slots stay undrawn and the load report names them.
    pub colour_pool: bool,
    /// Whether the track draws that show a target sample it with V negated.
    ///
    /// Pulse does: a live PPSSPP dump of Talon's Junction puts `TEXSCALE (1, -1)`
    /// and `TEXOFFSET (0, 0)` on the prims that read the 128 x 128 target and `(1,
    /// 1)` or an authored animation scale on every other textured prim (547 prims).
    /// The target holds the picture top row first and the track's UVs run V up. HD
    /// authors its billboard quads with V running down, so it needs none.
    pub flip_v: bool,
    /// Whether the track draws that show a target sample it with its texture
    /// coordinates wrapping, not clamped to the edge.
    ///
    /// HD, measured on RPCS3 (`03_track`): both draws that read the 512 x 256
    /// `R5G6B5` target carry texture address `0x60010101` (wrap on U, V, W) and
    /// filter `0x02062000` (linear, `LINEAR_MIPMAP_LINEAR` over one level, LOD bias
    /// 0), no anisotropy. A quad whose coordinates pass 0..1 shows the picture
    /// repeated, where a clamp smears the edge texel. Pulse is `true`, measured on
    /// PPSSPP: every draw sampling the 128 x 128 target has `TEXWRAP` 0 (repeat) and
    /// filter `0x107` (linear, one level).
    pub wrap: bool,
    /// Whether a Zone race swaps the per-slot adverts for one shared texture. HD's
    /// `Billboard_LoadModelAndBind` takes a second branch while the zone effects are
    /// on, binding a single `"billboard"` texture to every slot but 8 instead of a
    /// target per slot. What that texture is has not been read, so such a title draws
    /// no advert in Zone and says so.
    pub zone_shares_one_texture: bool,
    /// Whether slot 8, the start gantry, is drawn as a card like the other slots
    /// instead of as a model stood on the track's mount.
    ///
    /// Pulse (both pressings), measured on PPSSPP: a GE dump draws the gantry as pass
    /// B into the 128 x 128 target, and the software renderer shows the board opaque.
    ///
    /// HD, measured on RPCS3 (Talon's Junction, `docs/rendering/start-gantry.md`,
    /// "Wipeout HD: the gantry is a card"): the frame's second 512 x 256 colour
    /// target is drawn with `321_go_64.gtf` (64 x 128 DXT5, 8 mips, trilinear)
    /// through a `texture * constant` program and the track's `billboard8` quad
    /// samples it through the lit track program. A lit `GO` read 132 of 255 against a
    /// full white from the model route, whose white sits above the bloom gate's knee
    /// and smears into a halo.
    pub gantry_card: bool,
    /// Whether the pass that draws slot 8's model into its card takes no light.
    ///
    /// HD, measured on RPCS3: the card passes are `TEX; MUL H0.xyz, H0, {1,1,1,1}`
    /// with no lighting term; the track's `billboard8` quad lights the card once,
    /// afterwards. Pulse, measured on PPSSPP (`pulse-card-verify`): `LIGHTINGENABLE`
    /// is 0 on every prim drawn into the 128 x 128 target in four dumps (three
    /// circuits and a second boot), `TEXFUNC` modulate, while the same frames' main
    /// pass has it set on part of its prims.
    pub card_unlit: bool,
    /// Whether every advert card (slots other than 8, which is `card_unlit`) is
    /// drawn with no lighting of its own, because the original's card passes are
    /// `TEX; MUL H0.xyz, H0, {1,1,1,1}` and nothing else: the track quad that shows
    /// the card lights it once. `false` draws a card through the stand-in key and
    /// fill rig.
    ///
    /// HD, measured on RPCS3 (Talon's Junction, four hooked frames over two boots):
    /// the frame next to the grid draws two 512 x 256 card targets (slots 7 and 8, 23
    /// and 19 passes); two frames with the craft among the hoardings draw four (10,
    /// 22, 42 and 46 passes). Every one of those 120 passes and the grid frame's 42
    /// has one of two fragment programs: `TEX H0.xyz; MUL H0.xyz, H0, {1,1,1,1}; MOV
    /// H0.w, 0` (opaque) or `TEX H0; MUL H0.xyz, H0, {1,1,1,1}; MOV H0.w, H0`
    /// (blended), the constant patched to one per draw. Pulse, measured on PPSSPP
    /// (`pulse-mount-11`): the advert passes run into the same 128 x 128 target as
    /// slot 8 and every prim of every run (slot 7 on `16_Track`'s home straight, four
    /// adverts near `01_Track`'s grid) has `LIGHTINGENABLE` 0.
    pub adverts_unlit: bool,
    /// What the target the cards are drawn into stores of a card's colour: HD's linear-light
    /// 5-6-5, Pulse's truncated gamma 5-6-5, or the full 8 bits. See [`CardStore`].
    pub card_store: CardStore,
    /// The board slot 8 shows instead of the circuit's own while the race is a
    /// Zone race, or `None` where the title keeps one gantry for every mode.
    pub zone_gantry: Option<ZoneGantry>,
    /// Where the numbers came from.
    pub origin: crate::Origin,
}

/// What a card target stores of the colour drawn into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardStore {
    /// The full 8 bits per channel: where the card format is not measured.
    Rgba8,
    /// A 5-6-5 target holding linear light, so a card's colour reaches the track quad
    /// rounded to 31, 63 and 31 levels of linear light.
    ///
    /// HD, measured on RPCS3 (Chenghou Project; `start-gantry.md`, "The card is 5-6-5
    /// in linear light"): the quad draws read the 512 x 256 targets as `R5G6B5`
    /// (`fmt 0x1a429`), and a GDB read of the slot 8 card (`0x02ae0000`, pitch
    /// `0x400`, big-endian words) is the panel texture decoded to linear light and
    /// rounded to the nearest level: 85 to 90 % of its pixels equal that rounding
    /// (green 0.98 and blue 0.97 correlated), where ours kept the gamma texel at 8
    /// bits. The panel's red, 25 of 255, is 0.0098 linear, under a third of the first
    /// level, so most of it rounds to 0 and the quad's red reads about 13 lower.
    R5G6B5Linear,
    /// A 5-6-5 target holding gamma-encoded values, truncated to 5, 6 and 5 bits the way
    /// the PSP's GE writes a 16-bit target.
    ///
    /// Pulse, measured on PPSSPP (`start-gantry.md`, "Pulse's card is 5-6-5, truncated"):
    /// a GE dump of the grid shows every prim into the 128 x 128 target at `0x154000`
    /// with `FRAMEBUFPIXFORMAT` 0, and the dither matrix registers all zero.
    R5G6B5Gamma,
}

/// A Zone race's own start board.
///
/// **HD, measured on RPCS3 (Vineta K, one boot per mode):** `GetMode()` 6 shows a
/// board of its own before the release (a dark panel on which a loop icon
/// assembles from light pieces) where modes 3 and 8 show `3 2 1 GO`. Confidence 85
/// for "the mode picks another file"; the file is inferred (80) from the picture,
/// the names on the disc and the PS4 binary's mode table
/// (`docs/ghidra/functions/ps4-omega-eu/billboards.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneGantry {
    /// The model's archive path, spelled as a `trackstartup.xml` names one.
    pub entry: &'static str,
    /// Ticks the board's assembly runs before the release (HD: 210, about 3.5 s off
    /// the capture; one boot, so a few ticks either way).
    pub assembly_ticks: u64,
}
