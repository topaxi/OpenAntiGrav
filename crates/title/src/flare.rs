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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Whether the craft's `EngineLightData.xml` hangs an SPU vertex light off
    /// the nozzle. HD: yes, measured (`EngineFlare_SubmitSpuLight`). 2048: no -
    /// the SPU is the PS3's, its Vita consumer is unread, and the 20 native
    /// craft all author `Distance 3` / `Radius 30` against HD's 0.7-2.0 on a
    /// hull of the same length (13.8 against 13.8 units), which floods the
    /// circuit white when drawn on HD's law.
    pub engine_light: bool,
    /// Whether the original submits only the camera-facing triangles of the
    /// flare's shapes, so a draw is single-sided although its material
    /// disables culling. HD: **measured** (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`,
    /// "The flare shapes are single-sided"): of the 5 shapes of a live
    /// thrust frame, 498 of 499 submitted triangles face the eye by their
    /// authored winding, and the frame submits 49-57 % of the model's
    /// triangles. 2048 and Omega: unmeasured, so `false`.
    pub front_faces_only: bool,
}
