//! Where a title keeps the **engine flare** - the light at the nozzle itself,
//! as distinct from the ribbon behind it that [`crate::exhaust`] answers for.
//!
//! **A second axis rather than a variant of the first**, because the two
//! questions come apart on the disc: Wipeout HD authors *both* its ribbon and
//! its flare as assets, but they are different assets of different shapes - one
//! shared one-triangle ribbon template under `/data/ribboneffects/`, and one
//! **per-team** flare model under each craft's own directory. A single enum
//! could not carry a per-team stem and a shared path in the same variant
//! without one of the two titles reading the wrong field.
//!
//! Pulse and Pure name a sprite texture, exactly as they name the ribbon's -
//! `Texture_LoadEngineFlare` at `0x08a84c80` is the literal, and the flare is
//! six vertices of camera-facing quad. Wipeout HD does not carry that name at
//! all, which is why every HD load report said "the flare falls back to a
//! procedural glow" until this axis existed. See
//! [`trail-ribbon.md`](../../../docs/rendering/trail-ribbon.md), "HD's flare is
//! a model, not a sprite".

/// How this title supplies the engine flare.
///
/// The three answers are exclusive for the reason [`crate::exhaust::Exhaust`]'s
/// are: a title names a sprite, or it authors a model, or neither has been
/// located. No source measured so far does two of them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flare {
    /// The flare's sprite texture, by the name the executable itself carries.
    ///
    /// `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` on Pulse and Pure.
    /// The PS2 build loads the same literal and finds it under `.pct`;
    /// `oag_pulse::read_image` does the extension rewrite.
    Sprite(&'static str),

    /// The flare as a **per-team model**, for a title that authors one.
    ///
    /// See [`Authored`]. The path is composed per craft rather than carried
    /// here, because every one of Wipeout HD's fourteen craft ships its own.
    PerTeam(Authored),

    /// This title's engine flare has not been located at all.
    ///
    /// Reported, never silently swapped for a stand-in - the same rule
    /// [`crate::exhaust::Exhaust::Unread`] follows.
    Unread,
}

/// A per-team flare model: the stem beside the hull, and the two node groups
/// its tree is authored in.
///
/// **The group names are asset facts and belong beside the stem**, not in a
/// loader: every one of Wipeout HD's fourteen `engineflare.vex` files carries
/// the same two `Transform` nodes under `root`, and which of the two a shape
/// hangs under is the only thing in the file that says when it is drawn.
///
/// ```text
/// root
///   EF_Boost            <- boost
///     Joint_BoostLeft  -> ef_BoostLeftShape, ef_SpikesLeftShape
///     Joint_BoostRight -> ef_BoostRightShape, ef_SpikesRightShape
///     Joint_Diamonds   -> ef_DiamondsShape
///   EF_Main             <- always
///     ef_OuterShape, ef_InnerShape, ef_Spikes1Shape .. ef_Spikes3Shape
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Authored {
    /// The model's stem beside the hull, as `entry_name` composes it -
    /// `engineflare`, giving `Data\Ships\<Team>\engineflare.vex` and the
    /// `.rcsmodel` beside it.
    pub stem: &'static str,
    /// The node group drawn whenever the craft is in the race: `EF_Main`.
    pub always: &'static str,
    /// The node group that is the **boost plume**: `EF_Boost`.
    ///
    /// Wipeout HD ships no `shipboost.vex`, which is what a sweep for one
    /// concluded before this group was found - the plume is not a file of its
    /// own there, it is a subtree of the flare.
    pub boost: &'static str,
    /// The shapes under [`Self::always`] that flicker on their own uniform
    /// scale instead of riding the group's: `ef_Spikes1Shape` to
    /// `ef_Spikes3Shape`. Each is split out of the group into a part of its
    /// own - `EngineFlare_PlaceShapes` gives every one a fresh
    /// `Spikes Random Scale` draw per frame - and `always` keeps the rest.
    pub main_spikes: &'static [&'static str],
    /// The second spelling of [`Self::main_spikes`], slot for slot (empty for a
    /// title with none). Wipeout Fury's Feisar concept and neon craft author
    /// the same three shapes as `ef_SpikesShape1` to `ef_SpikesShape3`, with the
    /// number after `Shape`; the census over every `engineflare.vex` HD ships
    /// finds no other deviation. A slot takes the first spelling the file
    /// carries, so the flicker stream stays keyed on the slot.
    pub main_spikes_alt: &'static [&'static str],
    /// The same for [`Self::boost`]: `ef_SpikesLeftShape` and
    /// `ef_SpikesRightShape`.
    pub boost_spikes: &'static [&'static str],
    /// The shapes under [`Self::boost`] that hang from a `Joint_*` node of their
    /// own: `ef_BoostLeftShape`, `ef_BoostRightShape` and `ef_DiamondsShape`.
    /// `EngineFlare_PlaceShapes` scales each one's length by the boost blend
    /// about its own joint (read live on Feisar Fury: three draws at the joints'
    /// authored translations), so each is a part of its own, placed at its node,
    /// instead of a share of the group at the locator. Empty for a container
    /// whose node transforms are not composed (2048's Vita files).
    pub boost_shapes: &'static [&'static str],
    /// The sprite flare's texture, in the archive set's lower-case path form,
    /// drawn **beside** the model: `/data/tex/engineflare/engine_flare_rich.gtf`
    /// on HD and `.gxt` on 2048 - both executables carry the literal. The
    /// extension says which decoder reads it.
    pub sprite: &'static str,
    /// The craft's engine light: the point light `EngineLightData.xml` hangs
    /// behind the nozzle, with the colours and falloff the executable carries
    /// (`EngineFlare_SubmitSpuLight` on HD, `EngineFlare_SubmitVertexLight` on
    /// 2048 and Omega), or `None` for a title whose light is unread.
    pub engine_light: Option<EngineLightLaw>,
    /// Whether the original submits only the camera-facing triangles of the
    /// flare's shapes, so a draw is single-sided although its material
    /// disables culling. HD: **measured** (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`,
    /// "The flare shapes are single-sided"): of the 5 shapes of a live
    /// thrust frame, 498 of 499 submitted triangles face the eye by their
    /// authored winding, and the frame submits 49-57 % of the model's
    /// triangles. 2048 and Omega: unmeasured, so `false`.
    pub front_faces_only: bool,
}

/// A craft's light colour at rest and how hard a boost drives it:
/// `colour * (1 + gain * boost_blend)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    /// Linear light colour, in the executable's own magnitudes.
    pub colour: [f32; 3],
    /// The boost gain: `10` on the HD-era bases, `2.2` on 2048's.
    pub gain: f32,
}

impl Tint {
    /// The HD-era hull on a classic skin, `(4, 10, 40)` with the HD-era gain
    /// `10`: the constants all three executables carry
    /// (`EngineFlare_SubmitSpuLight` `0x0029ff28` on HD, `EngineFlare_SubmitVertexLight`
    /// `0x0173e350` on Omega 1.07, `0x812726d6` on 2048).
    pub const HD_CLASSIC: Self = Self {
        colour: [4.0, 10.0, 40.0],
        gain: 10.0,
    };
    /// The HD-era hull on a Fury skin: the classic one's red and blue swapped.
    pub const HD_FURY: Self = Self {
        colour: [40.0, 10.0, 4.0],
        gain: 10.0,
    };
    /// The 2048-era craft, `(3.0, 0.5, 0.1)` with gain `2.2`: Omega and 2048
    /// carry it, HD does not.
    pub const ERA_2048: Self = Self {
        colour: [3.0, 0.5, 0.1],
        gain: 2.2,
    };
}

/// The 2048-era craft's light, a different colour and a much wider reach: a
/// dim red light three units behind the nozzle that reaches the floor, where
/// the HD-era one is a tight strong one that reaches only the housing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Era2048 {
    /// The tint a 2048-era craft carries.
    pub tint: Tint,
    /// The ship directory of this title's 2048-era roster, compared
    /// case-blind with the craft's own. The original picks the tint by the
    /// team's `GameVersion="2048"`, which only that roster authors.
    pub roster_dir: &'static str,
}

/// Which circuits a mag strip recolours the light on: the original gates it on
/// the game mode being a 2048-lineage one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripGate {
    /// Every circuit: the title is 2048, whose events are all 2048 modes
    /// (`GameMode_IsHdLineage`, `0x81000930`, is false for each).
    EveryCircuit,
    /// A circuit of one of [`crate::RaceDefaults::circuit_lineages`]: Omega
    /// runs HD's and 2048's games from one executable, and the 2048 game's
    /// circuits are the 2048-lineage ones.
    LineageCircuit,
}

/// The colour swap a craft's light makes while it is over a mag strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagStrip {
    /// The light's colour over the strip, replacing the tint's.
    pub colour: [f32; 3],
    /// The tint's boost gain is multiplied by this over the strip.
    pub gain_scale: f32,
    /// Where it applies.
    pub gate: StripGate,
}

/// How the light wanders: the executable redraws a vector every flare update,
/// `(x, y, 0, range)`, uniform either side of zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wander {
    /// Half-range of the `x` and `y` draws, world units.
    pub position: f32,
    /// Half-range of the draw added to `Radius`.
    pub range: f32,
    /// Whether the boost calms it: the vector is scaled by `1 - boost_blend`.
    /// Read on 2048 and Omega; HD's is unread, so it keeps its jitter whole.
    pub calmed_by_boost: bool,
}

/// The per-title law of a craft's engine light, as data (ADR-0058).
///
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` ("The captured buffer's
/// producer is found") for HD, `docs/ghidra/functions/ps4-omega-eu/engine-light.md`
/// and `docs/ghidra/functions/vita-2048-eu-v104/engine-light.md` for the other
/// two. The light's position is `flare_node.pos - flare_node.z * Distance`
/// plus the wander and its range `Radius` plus the wander, on every title.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineLightLaw {
    /// The HD-era hull on its classic skin.
    pub classic: Tint,
    /// The HD-era hull on a Fury skin (`concept1`, `nitro`, `chrome_c1`, `detonator`).
    pub fury: Tint,
    /// The 2048-era craft, on a title that has them.
    pub era_2048: Option<Era2048>,
    /// The colour a mag strip gives it, on a title that has the branch.
    pub mag_strip: Option<MagStrip>,
    /// How the light wanders.
    pub wander: Wander,
}
