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
//! **Recovered on 2048, and only there: the trigger.** 2048's Zone HUD widget
//! (`Hud_UpdateZoneSpeedClassWidget`, `0x81197d6c`) walks a seventeen-record
//! table of descending **zone-number** thresholds and writes the matched
//! record's index into the per-craft field `Zone_UpdateStage` (`0x81044cfc`)
//! reads, clamps to the table's last row and shows. That table was read out of
//! the executable at `0x8151faf8` this session and is
//! [`oag_title::ZoneStages`]; [`ZoneGrade::show_zone`] is `Zone_UpdateStage`'s
//! own arithmetic, and a 2048 Zone race in this engine now escalates on it.
//! The bands are wide - `0`-`1`, `2`-`8`, `9`-`16`, `17`-`32`, `33`-`39`, then
//! every five to `90` - so the grade steps every few zones rather than every
//! zone.
//!
//! **Not recovered on HD/Fury.** `Environment_UpdateStageBlend` (`0x003da540`)
//! dispatches on `g_GameState.mode`, and this session read all four of its
//! branches out of the instruction stream: mode `0xe` takes
//! `RaceManager->+0x2e10`, modes `0xd`/`0x15` take per-viewport entries of the
//! same object, and **everything else - Zone included - falls through to
//! `craftArray[n]->+0x640`**, whose writer was not found. Mode `0xe` turned out
//! to be *Detonator*, not Zone: it selects the `Data/Tex/DetonatorMode*.gtf`
//! filename table in `Environment_LoadStageTextures`, and `+0x2e10`'s only two
//! non-incrementing writers are `SPDetonator`'s own two constructors. So HD
//! carries no [`oag_title::RaceDefaults::zone_stages`] and a Zone race on it
//! still rests where its loader leaves it - `Environment_LoadStageTextures`
//! resets `+0x00` to `0` on every load. Per `CLAUDE.md`, 2048's thirteen-stage
//! numbers are not transplanted onto HD's fifteen-stage ladder to fill that in.
//!
//! **What still is not recovered on either title: the cross-fade's own rate.**
//! 2048 fades the current stage toward the *next* one by a factor
//! (`DAT_816c6bc8`) nothing traced writes, so [`ZoneGrade::show_zone`] leaves
//! the weight at rest and the stage changes cleanly rather than easing. That is
//! an absence, not a snap chosen for looks.

use std::sync::Arc;

use oag_formats::effectsettings::{EffectSettings, StagePalette};
use oag_render::{mesh::ModelTexture, mesh_render};

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
    /// How this title turns a zone number into a stage, where that is
    /// recovered. `None` on every title but 2048 - see the module docs.
    stages: Option<&'static oag_title::ZoneStages>,
    /// The "track" set's per-stage texture, one slot per stage, in stage
    /// order. Empty on a title with no located set, and `None` in any slot
    /// whose entry did not decode. See [`Self::stage_art`].
    ///
    /// Positional and never compacted, for the same reason
    /// [`oag_render::mesh::TextureSlots`] is: the index *is* the stage number,
    /// so dropping a slot that failed to decode would silently re-map every
    /// stage above it.
    art: oag_render::mesh::TextureSlots,
}

impl ZoneGrade {
    /// Builds a grade over a parsed table, or `None` for a file that names no
    /// stage at all - which would be a table with nothing to select.
    #[must_use]
    pub fn new(
        entry: String,
        table: EffectSettings,
        stages: Option<&'static oag_title::ZoneStages>,
        art: oag_render::mesh::TextureSlots,
    ) -> Option<Self> {
        let last_stage = *table.stages.keys().next_back()?;
        Some(Self {
            table,
            entry,
            last_stage,
            blend: StageBlend::default(),
            stages,
            art,
        })
    }

    /// The per-stage texture the showing stage indexes, where the title ships
    /// one and it decoded.
    ///
    /// **This is the "track" set, and it draws.** The original samples it
    /// through the `zoneTexInner`/`zoneTexOuter` shader parameters, and
    /// `mesh.wgsl`'s `zone_surface` reproduces the part of that rule whose
    /// inputs are all on the disc - see [`Self::zone_uniform`] and
    /// [`oag_render::mesh_render::Zone`]. The visualiser glow that rides
    /// alongside it needs a runtime-built 256-entry lookup (`zoneTexVis`)
    /// built from a table inside the executable, and is not drawn.
    ///
    /// The **track** set rather than the "general" one because the general
    /// set is fifteen byte-identical flat whites whose alpha is 255
    /// everywhere: it indexes one lookup entry and can display nothing.
    ///
    /// `None` for a stage past the set, or one whose entry did not decode -
    /// the slots are positional, so a hole stays a hole.
    #[must_use]
    pub fn stage_art(&self) -> Option<&Arc<ModelTexture>> {
        self.art.get(self.blend.current as usize)?.as_ref()
    }

    /// How many of this title's per-stage textures decoded, and how many it
    /// numbers.
    #[must_use]
    pub fn stage_art_counts(&self) -> (usize, usize) {
        (self.art.iter().flatten().count(), self.art.len())
    }

    /// The stage this title's ladder puts zone number `zone` on, clamped to the
    /// rows the loaded file actually names.
    ///
    /// `None` on a title with no recovered ladder, which is every title but
    /// 2048.
    #[must_use]
    pub fn stage_for_zone(&self, zone: u16) -> Option<u32> {
        Some(self.stages?.stage_for(zone)?.min(self.last_stage))
    }

    /// Shows the stage zone number `zone` sits on, and answers whether that
    /// changed the picture.
    ///
    /// **This is `Zone_UpdateStage`'s shape, not
    /// [`Self::commit`]'s.** 2048 assigns the clamped index straight into the
    /// showing stage every frame - there is no request/commit gate on that
    /// title, and no weight reset either, so this sets the weight to rest
    /// rather than reproducing HD's `+0x18 = 0`. The two titles genuinely
    /// differ here; see the module docs.
    ///
    /// A no-op on a title with no recovered ladder, which is how HD/Fury keeps
    /// resting on stage `0` rather than escalating off numbers that are not
    /// its own.
    pub fn show_zone(&mut self, zone: u16) -> bool {
        let Some(stage) = self.stage_for_zone(zone) else {
            return false;
        };
        if self.blend.current == stage && self.blend.requested == stage {
            return false;
        }
        self.blend.current = stage;
        self.blend.requested = stage;
        self.blend.weight = 1.0;
        true
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
    /// **HD/Fury's half of the mechanism, and still unreached by a race**: the
    /// request/commit pair belongs to `Environment_UpdateStageBlend`, whose own
    /// Zone-mode source field has no found writer. 2048 goes through
    /// [`Self::show_zone`] instead, which is that title's own shape. See the
    /// module docs.
    pub fn request_stage(&mut self, stage: u32) {
        self.blend.requested = stage.min(self.last_stage);
    }

    /// Sets the cross-fade weight. Unreached by a race: what advances it during
    /// a race is unrecovered on both titles, so [`Self::show_zone`] leaves it at
    /// rest rather than easing between stages. See the module docs.
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

    /// The `float4` HD/Fury hands its shaders as `fogColour`, assembled the
    /// way its own renderer assembles it.
    ///
    /// **This is the one value in this whole table with a traced consumer.**
    /// `Environment_UpdateStageBlend` cross-fades `Scene.Texture Colour` and
    /// `Scene.EQ brightness` and stores them to two separate addresses;
    /// `Scene_PrepareFrame` splats the scalar into the colour's `.w` and
    /// publishes the result as engine shader parameter 7. Confidence 90 on
    /// the binding, 80 on the key attribution - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.
    ///
    /// **Nothing draws it yet, deliberately.** What HD's fragment program
    /// does with `fogColour` is unread, and the authored values are not
    /// fog-shaped - `Scene.Texture Colour` runs to `0.96` here and its
    /// `Track` sibling to `9.0`, which is a multiplier's range. Reported
    /// rather than applied, so the recovered number is visible without a
    /// guess about its meaning standing in for the shader.
    ///
    /// `None` on a file that authors no `Scene.Texture Colour` at all, which
    /// is every 2048 table - that title reauthored the vocabulary.
    #[must_use]
    pub fn scene_tint(&self) -> Option<[f32; 4]> {
        let palette = self.palette();
        let [r, g, b] = palette.scene_texture_colour?;
        // The `.w` is a separate key in the file and the same 16 bytes at
        // runtime; a stage that authors the colour but not the scalar leaves
        // the lane at the zero the struct was reset to.
        Some([r, g, b, palette.scene_eq_brightness.unwrap_or(0.0)])
    }

    /// The Zone shader's own per-stage parameters, for the showing stage.
    ///
    /// # What is fed and what is not
    ///
    /// [`oag_render::mesh_render::Zone`] carries the rule and the full list of
    /// terms left out; the two values assembled here are:
    ///
    /// - `zoneColourTint.xy`, the file's own title-wide `Texture U scale` /
    ///   `Texture V scale`. `Environment_RegisterStageSchema` (`0x003d0b98`,
    ///   `ps3-hdfury-eu`) registers those two keys directly into shader
    ///   parameter 52's storage, so this is the whole of the binding rather
    ///   than a reading of it. Confidence 85.
    /// - `zoneEffectInner`/`zoneEffectOuter`, the showing stage's
    ///   `Scene.Texture Colour`. `Environment_UpdateStageBlend` (`0x003da540`)
    ///   copies stage `n`'s into the inner parameter and stage `n - 1`'s into
    ///   the outer one. Confidence 84.
    ///
    /// **The showing stage's own palette, not [`Self::palette`]'s cross-fade.**
    /// The original does not blend these two in colour space at all: it hands
    /// the shader both stages and picks between them per pixel with a sphere
    /// test. This build never has a stage transition in flight - nothing
    /// advances [`StageBlend::weight`] and no title on this path has a
    /// recovered trigger - so inner and outer are the same stage and the
    /// unblended palette is what the original would compute too.
    ///
    /// `enabled` is `0.0`, and the whole term disappears, unless the file
    /// authors the UV scale *and* the stage authors `Scene.Texture Colour`
    /// *and* the stage's texture decoded. A missing input draws nothing.
    #[must_use]
    pub fn zone_uniform(&self) -> mesh_render::Zone {
        let off = mesh_render::Zone::default();
        let (Some(uv_scale), Some(palette)) = (
            self.table.zone_uv_scale(),
            self.table.stage_palette(self.blend.current),
        ) else {
            return off;
        };
        let (Some([r, g, b]), true) = (palette.scene_texture_colour, self.stage_art().is_some())
        else {
            return off;
        };
        mesh_render::Zone {
            uv_scale,
            enabled: 1.0,
            _pad: 0.0,
            // The `.w` scales the visualiser glow, which this build does not
            // draw for want of `zoneTexVis`. Left at zero rather than filled.
            effect: [r, g, b, 0.0],
        }
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
        let ladder = match self.stages {
            Some(stages) => format!(
                "escalating off this title's own {}-record zone-number ladder, starting on \
                 stage {} ({})",
                stages.records.len(),
                self.blend.current,
                stages.class_for(0).unwrap_or("?"),
            ),
            None => format!(
                "resting on stage {} because what selects a stage during a race is not \
                 recovered on this title",
                self.blend.current,
            ),
        };
        // The one recovered binding, reported so a load says what the
        // original's shader would receive even though nothing draws it.
        let tint = match self.scene_tint() {
            Some([r, g, b, w]) => {
                format!("; fogColour would read [{r}, {g}, {b}, {w}] from Scene.Texture Colour")
            }
            None => String::new(),
        };
        // What the Zone material term will actually add, so a load says
        // whether it draws and, when it does not, which input was missing.
        let zone = {
            let uniform = self.zone_uniform();
            let (decoded, named) = self.stage_art_counts();
            if uniform.enabled > 0.0 {
                let [r, g, b, _] = uniform.effect;
                let [u, v] = uniform.uv_scale;
                // A stage whose `Scene.Texture Colour` is black multiplies the
                // whole term out. Said plainly rather than reported as a draw,
                // because it is the file's own statement and not a failure:
                // `Start` is authored that way.
                let effect = if [r, g, b] == [0.0; 3] {
                    "adds nothing here, this stage authoring Scene.Texture Colour black".to_string()
                } else {
                    format!("draws where the albedo is black, zoneEffect [{r}, {g}, {b}]")
                };
                format!(
                    "; the Zone recolour {effect}: stage {} art of {decoded}/{named}, \
                     zone UV scale [{u}, {v}]",
                    self.blend.current,
                )
            } else {
                let missing = if self.table.zone_uv_scale().is_none() {
                    "the file authors no Texture U/V scale"
                } else if self.stage_art().is_none() {
                    "this stage has no decoded texture"
                } else {
                    "this stage authors no Scene.Texture Colour"
                };
                format!("; the Zone recolour draws nothing - {missing}")
            }
        };
        format!(
            "{}: the Zone colour grade, {stages} stage(s) from {first} to {last}; \
             {ladder}{tint}{zone}",
            self.entry
        )
    }
}

#[cfg(test)]
mod tests;
