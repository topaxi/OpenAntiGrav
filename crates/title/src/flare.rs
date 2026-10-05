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
    /// The sprite flare's texture, in the archive set's lower-case path form,
    /// drawn **beside** the model: `/data/tex/engineflare/engine_flare_rich.gtf`
    /// on HD and `.gxt` on 2048 - both executables carry the literal. The
    /// extension says which decoder reads it.
    pub sprite: &'static str,
}
