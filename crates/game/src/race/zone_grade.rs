//! The Zone colour grade: which stage of the `.effectSettings` ladder applies,
//! and what that stage does to the circuit's fog and light rig.
//!
//! # What this is
//!
//! A Zone race escalates. The disc's own `MSC_EVENT_ZONE` text says the top
//! speed rises every ten seconds, and both titles that ship an
//! `.effectSettings` table ship it so the *look* rises with the speed: one
//! full palette per speed class, layered over whichever circuit is racing.
//! [`oag_formats::effectsettings`] reads the file; this is where a stage of it
//! reaches the renderer, on the same path
//! `crate::race::load::environment::envsettings_fog` already drives from a
//! circuit's own `.envsettings`.
//!
//! # What is recovered and what is not
//!
//! **Recovered (confidence 80-85)**: the runtime shape. HD/Fury keeps a
//! two-entry array whose entry holds a *current* stage at `+0x00`, a
//! *requested* stage at `+0x04` and a blend weight at `+0x18`, and
//! `Environment_UpdateStageBlend` (`0x003da540`, `ps3-hdfury-eu`) is a
//! request/commit pair: when the two indices differ it draws a transition
//! effect, copies `+0x04` into `+0x00` and zeroes the weight. Two independent
//! sites then cross-fade stage `n` against stage `n - 1`, saturating at zero.
//! [`StageBlend`] is those three fields and nothing else. See
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.
//!
//! **Not recovered: the trigger.** What maps a live race onto a stage index -
//! ship speed, elapsed time, anything - is unknown on both titles. 2048's own
//! writer is its Zone HUD's Speed Class widget, driven through a 17-entry
//! threshold table that read inconsistently on direct decode and was left
//! unresolved rather than force-fit; HD's requested value was traced to
//! `g_GameState.mode`-dependent tables whose *contents* were never read. So
//! **nothing here advances a stage on its own**: [`ZoneGrade::request_stage`]
//! and [`ZoneGrade::set_weight`] exist and no race calls them. A Zone race
//! rests on stage `0`, which is exactly what HD's own loader leaves behind -
//! `Environment_LoadStageTextures` resets both entries' `+0x00` to `0` every
//! time the file loads. Per `CLAUDE.md`, inventing the thresholds would be a
//! fitted constant presented as a recovered fact.

use oag_formats::effectsettings::{EffectSettings, StagePalette};
use oag_render::mesh_render;

/// The three fields HD/Fury's own runtime keeps per entity, and no others.
///
/// Named for what the traced code does with them rather than for their
/// offsets: `+0x00` is the stage being shown, `+0x04` the stage asked for, and
/// `+0x18` the weight the cross-fade runs at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageBlend {
    /// `+0x00`: the stage currently applied.
    pub current: u32,
    /// `+0x04`: the stage asked for, committed into [`Self::current`] by
    /// [`ZoneGrade::commit`] when the two differ.
    pub requested: u32,
    /// `+0x18`: the current stage's own share of the cross-fade, `1.0` for the
    /// current stage alone and `0.0` for the stage before it - the convention
    /// [`oag_formats::effectsettings::cross_fade_rgba8`] documents.
    ///
    /// **The direction is open.** That function's own recovered doc reads
    /// `1.0` "at the moment a stage becomes current", while the commit
    /// `+0x18 = 0` writes zero at exactly that moment. Both cannot be the same
    /// quantity with the same meaning, and nothing recovered says what
    /// advances the field afterwards. This build keeps `cross_fade_rgba8`'s
    /// convention, rests at `1.0`, and zeroes on commit the way the traced
    /// store does - which leaves a freshly committed stage showing its
    /// predecessor until something raises the weight, and nothing does yet.
    pub weight: f32,
}

impl Default for StageBlend {
    /// Stage zero, fully applied, nothing pending - the state HD's own loader
    /// leaves behind, since `Environment_LoadStageTextures` resets `+0x00` to
    /// `0` unconditionally on every load.
    fn default() -> Self {
        Self {
            current: 0,
            requested: 0,
            weight: 1.0,
        }
    }
}

/// A title's stage table, plus which stage of it is showing.
#[derive(Debug, Clone)]
pub struct ZoneGrade {
    /// The parsed file, kept whole rather than reduced to the palettes: the
    /// stage *names* are what a report says, and the keys this build does not
    /// draw are still readable rather than dropped on load.
    table: EffectSettings,
    /// The archive entry it was read from, for the load report.
    entry: String,
    /// The highest stage the file names, which is what a request is clamped
    /// to. HD's own runtime clamps too - 2048's `Zone_UpdateStage` clamps to
    /// `0xc`, an exact match against its own thirteen-stage table.
    last_stage: u32,
    /// Which stage applies. See [`StageBlend`].
    blend: StageBlend,
}

impl ZoneGrade {
    /// Builds a grade over a parsed table, or `None` for a file that names no
    /// stage at all - which would be a table with nothing to select.
    #[must_use]
    pub fn new(entry: String, table: EffectSettings) -> Option<Self> {
        let last_stage = *table.stages.keys().next_back()?;
        Some(Self {
            table,
            entry,
            last_stage,
            blend: StageBlend::default(),
        })
    }

    /// Which stage is showing, which is asked for, and at what weight.
    #[must_use]
    pub fn blend(&self) -> StageBlend {
        self.blend
    }

    /// The highest stage this file names.
    #[must_use]
    pub fn last_stage(&self) -> u32 {
        self.last_stage
    }

    /// Asks for a stage, clamped to the ladder this file actually names.
    ///
    /// **Nothing in a race calls this.** The mapping from a live race onto a
    /// stage index is not recovered on either title - see the module docs - so
    /// this is the seam a recovered trigger would attach to, and until then it
    /// is reached only by a test.
    pub fn request_stage(&mut self, stage: u32) {
        self.blend.requested = stage.min(self.last_stage);
    }

    /// Sets the cross-fade weight. Unreached by a race, for the same reason
    /// [`Self::request_stage`] is.
    pub fn set_weight(&mut self, weight: f32) {
        self.blend.weight = weight;
    }

    /// Applies a pending request, the way `Environment_UpdateStageBlend` does:
    /// when the requested stage differs from the current one, it becomes the
    /// current one and the weight resets.
    ///
    /// Answers whether a stage change actually happened, which is what the
    /// traced code gates its transition effect on. **That effect is not drawn
    /// here**: the call it makes (`FUN_0067a7d8`) has not been identified, so
    /// there is nothing to fire and nothing is invented in its place.
    pub fn commit(&mut self) -> bool {
        if self.blend.requested == self.blend.current {
            return false;
        }
        self.blend.current = self.blend.requested;
        // `+0x18 = 0` at the commit, verbatim. See `StageBlend::weight` for
        // why that leaves the new stage showing its predecessor.
        self.blend.weight = 0.0;
        true
    }

    /// The palette showing right now: the current stage cross-faded against
    /// the stage before it at the current weight.
    #[must_use]
    pub fn palette(&self) -> StagePalette {
        self.table
            .blended_palette(self.blend.current, self.blend.weight)
            .unwrap_or_default()
    }

    /// The fog this stage authors, or `base` where it authors none.
    ///
    /// **A zero density leaves the circuit's own fog standing.** Stage `Start`
    /// authors `Fog density`=`0.000000`, and this build reads that as "this
    /// stage adds no fog of its own" rather than "this stage switches the
    /// circuit's fog off" - the same `density <= 0.0` guard
    /// `envsettings_fog` already applies to a circuit file. Which of the two
    /// the original means is unread, and so is what selects between the
    /// `Fog`, `Alt Fog` and `Track Fog` blocks each stage authors; the primary
    /// pair is used, exactly as `.envsettings`' own reader uses the primary
    /// pair and ignores its alternate.
    #[must_use]
    pub fn fog(&self, base: Option<mesh_render::Fog>) -> Option<mesh_render::Fog> {
        let palette = self.palette();
        let (Some(colour), Some(density)) = (palette.fog_colour, palette.fog_density) else {
            return base;
        };
        if density <= 0.0 {
            return base;
        }
        Some(mesh_render::Fog::authored_exp2(colour, density))
    }

    /// The circuit's light rig with this stage's colour terms laid over it.
    ///
    /// **A tint, never a rig of its own.** This schema has no `Sun direction`
    /// key at all - checked against the executable's own 73-entry vocabulary,
    /// not just against the shipped files - so a stage can only recolour a
    /// direction some other file already stated. A circuit that authors no
    /// `.envsettings` rig is handed back untouched: there would be nothing to
    /// aim the tinted sun along, and inventing a direction is exactly what
    /// `mesh_render::Light::stand_in` already declares itself to be.
    #[must_use]
    pub fn light(&self, base: mesh_render::Light) -> mesh_render::Light {
        if base.enabled == 0.0 {
            return base;
        }
        let palette = self.palette();
        mesh_render::Light::authored(
            base.direction,
            palette.sun_colour.unwrap_or(base.sun),
            palette.ambient_colour.unwrap_or(base.ambient),
            palette.prelit_scale.unwrap_or(base.prelit_scale),
            palette.prelit_power.unwrap_or(base.prelit_power),
            // Not in this schema at all, so the circuit's own value stands.
            base.specular_scale,
        )
    }

    /// What the load report says about this table.
    #[must_use]
    pub fn describe(&self) -> String {
        let stages = self.table.stages.len();
        let first = self
            .table
            .stages
            .values()
            .next()
            .map_or("?", |stage| stage.name.as_str());
        let last = self
            .table
            .stages
            .values()
            .next_back()
            .map_or("?", |stage| stage.name.as_str());
        format!(
            "{}: the Zone colour grade, {stages} stage(s) from {first} to {last}; resting on \
             stage {} because what selects a stage during a race is not recovered on either \
             title that ships this table",
            self.entry, self.blend.current,
        )
    }
}

#[cfg(test)]
mod tests;
