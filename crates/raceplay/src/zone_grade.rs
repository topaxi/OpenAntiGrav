//! The Zone colour grade: which stage of the `.effectSettings` ladder applies,
//! and what that stage does to the circuit's fog and light rig.
//!
//! # What this is
//!
//! A Zone race escalates. The disc's own `MSC_EVENT_ZONE` text says the top
//! speed rises every ten seconds, and both titles that ship an
//! `.effectSettings` table ship it so the *look* rises with the speed: one
//! full palette per speed class, layered over whichever circuit is racing.
//! [`oag_tables::effectsettings`] reads the file; this is where a stage of it
//! reaches the renderer, on the same path
//! `crate::load::environment::envsettings_fog` already drives from a
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
//! [`StageBlend`] is those three fields; the sphere the same entry advances
//! beside them is [`Wavefront`]. See
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.
//!
//! **Recovered on both 2048 and HD/Fury: the trigger.** 2048's Zone HUD widget
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
//! **HD/Fury's own writer of `craftArray[n]->+0x640` was found too, on the
//! same search that recovered its HUD's speed-class table.**
//! `Hud_UpdateZoneSpeedClass` (`0x00049718`) - the function that names the
//! current speed class beside the zone number - walks a fourteen-record
//! `{ u32 zoneThreshold, u32 stringIdPointer }` table
//! (`g_ZoneSpeedClassTable`, `0x00860d44`) and its last instruction is
//! `stw r3, 0x640(r29)`, exactly the field `Environment_UpdateStageBlend`
//! (`0x003da540`) reads on every mode but the three it special-cases (mode
//! `0xe`, which turned out to be *Detonator* rather than Zone, and modes
//! `0xd`/`0x15`, which take a per-viewport entry instead). So HD has the same
//! architecture 2048 does: the HUD widget drives the grade. This is
//! [`oag_title::ZoneStages`] again, HD's own fourteen-record table rather than
//! 2048's seventeen; [`oag_title::RaceDefaults::zone_stages`] is filled in for
//! HD/Fury, and a Zone race on it escalates the same way 2048's does. Full
//! evidence, the three checks against the running game and the reproduce
//! script are in
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md`; a
//! 40,000-tick capture confirms the grade actually steps rung for rung in a
//! long race (`docs/formats/psp-audio.md`'s "the two ladders can overlap"
//! section links the same finding from the announcer side).
//!
//! **The transition itself is recovered on HD/Fury and unread on 2048.** A
//! Zone stage change on HD is not a cross-fade in colour space: it is a sphere
//! centred on the local craft whose radius grows every frame, the new stage's
//! colours inside it and the previous stage's outside, beside a colour weight
//! that ramps the fog and rig over a hundred frames. Every number of it is the
//! executable's own - [`oag_hd::race::ZONE_TRANSITION`], read at instruction
//! level and reproduced live on RPCS3 to the tenth - and [`ZoneGrade::follow`]
//! is where it reaches the renderer. 2048 fades the current stage toward the
//! *next* one by a factor (`DAT_816c6bc8`) nothing traced writes, so that
//! title carries no [`oag_title::ZoneTransition`] and [`ZoneGrade::show_zone`]
//! leaves its weight at rest: the stage changes cleanly rather than easing.
//! That is an absence, not a snap chosen for looks.

use std::sync::Arc;

use oag_mesh::mesh::ModelTexture;

use oag_mesh::mesh_render;
use oag_tables::effectsettings::{EffectSettings, StagePalette};

/// The two stage indices HD/Fury's own runtime keeps per entity, and the
/// colour weight beside them.
///
/// Named for what the traced code does with them rather than for their
/// offsets: `+0x00` is the stage being shown, `+0x04` the stage asked for, and
/// `+0x18` the weight the cross-fade runs at. The same entry also holds the
/// transition sphere's radius (`+0x08`), speed (`+0x10`) and acceleration
/// (`+0x14`); those are [`Wavefront`]'s, derived per frame rather than stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageBlend {
    /// `+0x00`: the stage currently applied.
    pub current: u32,
    /// `+0x04`: the stage asked for, committed into [`Self::current`] by
    /// [`ZoneGrade::commit`] when the two differ.
    pub requested: u32,
    /// `+0x18`: the current stage's own share of the cross-fade, `1.0` for the
    /// current stage alone and `0.0` for the stage before it - the convention
    /// [`oag_tables::effectsettings::cross_fade_rgba8`] documents.
    ///
    /// **The direction is answered, 2026-09-15.** The commit writes `+0x18 =
    /// 0` and every later frame adds `0.01f` to it until it clamps at `1.0`
    /// (`Environment_UpdateStageBlend`, `0x003dd398`; reproduced live), so
    /// the weight is the *new* stage's share, rising from nothing on the
    /// commit frame to the whole palette a hundred frames later. The half of
    /// `cross_fade_rgba8`'s recovered doc that reads `1.0` "at the moment a
    /// stage becomes current" was the stale one. On a title with a read
    /// transition [`ZoneGrade::follow`] derives this every frame; on one
    /// without, it rests at `1.0`.
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

/// Where the stage-transition sphere is this frame: what is outside it, how
/// far it has grown, and what it is centred on.
///
/// **Derived from the race's own zone clock, never counted.** HD/Fury keeps
/// these as per-entity fields it advances once a frame; this port has no
/// per-frame hook that every path shares - a `--screenshot` capture builds
/// its scene after the whole tick loop has run - so instead
/// [`ZoneGrade::follow`] computes the frame count from the zone the stage
/// stepped at and the time since, both of which `World` already carries. The
/// result is the same on the windowed loop, the capture's single sync and a
/// paused race, and adds nothing to `World`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wavefront {
    /// The stage outside the sphere - the one that was showing before the
    /// last step. Equal to [`StageBlend::current`] when no transition is in
    /// flight, which makes the shader's sphere test a no-op.
    pub previous: u32,
    /// Frames since the stage stepped - `k` in `r_k = 0.1 + 0.5k +
    /// 0.05k(k - 1)`. Saturated at [`u32::MAX`] once a transition is
    /// settled.
    pub frames: u32,
    /// The sphere's centre: the local craft's world position, re-read every
    /// frame the way `Scene_PrepareFrame` (`0x003ad8dc`) rewrites
    /// `zoneOrigin` from the craft's transform. Measured live, 92: the
    /// pointer was the player's own craft on 210 of 210 frames and moved on
    /// 207 of 207 consecutive pairs.
    pub origin: [f32; 3],
    /// The sphere's radius this frame by the title's law, in whatever unit
    /// the world is drawn in - see [`oag_hd::race::ZONE_TRANSITION`] for
    /// the one thing about it not measured. Zero on a title with no read
    /// transition.
    pub radius: f32,
}

impl Wavefront {
    /// No transition in flight: `previous == current`, at the cap.
    fn settled(current: u32, radius: f32) -> Self {
        Self {
            previous: current,
            frames: u32::MAX,
            origin: [0.0; 3],
            radius,
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
    /// How a stage step sweeps the world on this title, where that is read.
    /// `None` on every title but HD/Fury - see [`oag_title::ZoneTransition`]
    /// and [`Self::with_transition`].
    transition: Option<&'static oag_title::ZoneTransition>,
    /// Where the transition sphere is this frame. See [`Wavefront`] and
    /// [`Self::follow`].
    wavefront: Wavefront,
    /// The "track" set's per-stage texture, one slot per stage, in stage
    /// order. Empty on a title with no located set, and `None` in any slot
    /// whose entry did not decode. See [`Self::stage_art`].
    ///
    /// Positional and never compacted, for the same reason
    /// [`oag_mesh::mesh::TextureSlots`] is: the index *is* the stage number,
    /// so dropping a slot that failed to decode would silently re-map every
    /// stage above it.
    art: oag_mesh::mesh::TextureSlots,
    /// The "general" set's per-stage texture, `zoneMode<n>.gtf`, on the same
    /// terms as [`Self::art`]. The one a chunk without the track bit samples
    /// - see [`Self::stage_art_pair`].
    scene_art: oag_mesh::mesh::TextureSlots,
    /// Set by [`Self::pin_stage`]: once true, [`Self::show_zone`] is a no-op
    /// for the rest of this grade's life, so `--zone-stage` wins over the
    /// title's own ladder instead of being overwritten by it on the next
    /// frame that steps the zone counter. See [`Self::pin_stage`].
    pinned: bool,
}

impl ZoneGrade {
    /// Builds a grade over a parsed table, or `None` for a file that names no
    /// stage at all - which would be a table with nothing to select.
    #[must_use]
    pub fn new(
        entry: String,
        table: EffectSettings,
        stages: Option<&'static oag_title::ZoneStages>,
        art: oag_mesh::mesh::TextureSlots,
        scene_art: oag_mesh::mesh::TextureSlots,
    ) -> Option<Self> {
        let last_stage = *table.stages.keys().next_back()?;
        Some(Self {
            table,
            entry,
            last_stage,
            blend: StageBlend::default(),
            stages,
            transition: None,
            wavefront: Wavefront::settled(0, 0.0),
            art,
            scene_art,
            pinned: false,
        })
    }

    /// Gives this grade the title's stage-transition law, where the title
    /// has one read. See [`oag_title::RaceDefaults::zone_transition`].
    ///
    /// Without one, a stage step shows the new stage whole on the frame it
    /// steps, which is what the ladder did before any transition was
    /// recovered and remains what 2048 gets.
    #[must_use]
    pub fn with_transition(
        mut self,
        transition: Option<&'static oag_title::ZoneTransition>,
    ) -> Self {
        self.transition = transition;
        self.wavefront = Wavefront::settled(self.blend.current, self.settled_radius());
        self
    }

    /// The radius a settled wavefront reports: the law's cap, or zero on a
    /// title with no law. Either way `previous == current` there, so the
    /// shader's sphere test selects the same colours on both sides.
    fn settled_radius(&self) -> f32 {
        self.transition.map_or(0.0, |law| law.radius_cap)
    }

    /// Where the transition sphere is this frame. See [`Wavefront`].
    #[must_use]
    pub fn wavefront(&self) -> Wavefront {
        self.wavefront
    }

    /// Leaves the showing stage whole: no transition in flight, weight at
    /// rest, the sphere at its cap.
    fn settle(&mut self) {
        let origin = self.wavefront.origin;
        self.wavefront = Wavefront::settled(self.blend.current, self.settled_radius());
        self.wavefront.origin = origin;
        self.blend.weight = 1.0;
    }

    /// The zone number the showing stage began at on this title's ladder, or
    /// `None` when the ladder does not put `zone` on the showing stage at
    /// all - which is a grade that has not been pointed at `zone` yet.
    ///
    /// The lowest threshold whose (clamped) stage is the showing one, so a
    /// ladder that keeps stepping past the file's own last stage does not
    /// restart the transition on rungs that change nothing.
    fn stage_start_zone(&self, zone: u16) -> Option<u16> {
        let stages = self.stages?;
        if self.stage_for_zone(zone)? != self.blend.current {
            return None;
        }
        stages
            .records
            .iter()
            .rev()
            .map(|&(at, _)| at)
            .find(|&at| self.stage_for_zone(at) == Some(self.blend.current))
    }

    /// Points the transition at where the race is: derives the frames since
    /// the showing stage stepped from the zone counter and the zone clock,
    /// and re-centres the sphere on the craft.
    ///
    /// Call once a frame after [`Self::show_zone`], with the race's zone
    /// number, its `zone_timer` (seconds accumulated toward the next step),
    /// the fixed timestep and the local craft's world position.
    ///
    /// # The law, and what is assumed
    ///
    /// The original advances its radius, speed and weight **per frame**, on
    /// a variable timestep it never reads (`Environment_UpdateStageBlend`,
    /// thirtieth pass), and freezes them while `g_GamePaused` is set. This
    /// port runs a fixed 60 Hz tick ([ADR-0007]), so a frame here is a tick:
    /// `k` = ticks since the step, which on the shipped defaults puts the
    /// radius at `207` one second in and the weight at `1.0` after 100
    /// ticks. A paused race stops ticking, so the sphere freezes with it, as
    /// measured. **That a PS3 frame and a tick here are the same length is
    /// the one substitution made**, stated rather than measured.
    ///
    /// **The opening stage is shown whole, by this port's choice.** A race
    /// opens on the ladder's zone-0 stage (`Sub Venom` on HD), and whether
    /// the original runs a sphere out of `Start`'s black during the countdown
    /// or has already settled by the time the player sees the track is
    /// unmeasured; the maintainer's own observation is the cyan already on
    /// the start line, which is what settling gives. So a stage whose ladder
    /// threshold is zone `0` carries no transition.
    ///
    /// [ADR-0007]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0007-fixed-timestep-vs-original.md
    pub fn follow(&mut self, zone: u16, seconds_into_zone: f32, dt: f32, origin: [f32; 3]) {
        self.wavefront.origin = origin;
        // No law read on this title, or a `--zone-stage` pin: the showing
        // stage is whole.
        let (Some(law), false) = (self.transition, self.pinned) else {
            self.settle();
            return;
        };
        let Some(start) = self.stage_start_zone(zone) else {
            self.settle();
            return;
        };
        if start == 0 {
            self.settle();
            return;
        }
        let ticks_per_step = (oag_race::zone::STEP_SECONDS / dt).round();
        let ticks_into_zone = (seconds_into_zone / dt).round();
        let frames = f32::from(zone - start) * ticks_per_step + ticks_into_zone;
        // `as` saturates, and the count is non-negative by construction.
        let frames = frames.max(0.0) as u32;
        self.wavefront.previous = self.stage_for_zone(start - 1).unwrap_or(self.blend.current);
        self.wavefront.frames = frames;
        self.wavefront.radius = law.radius_after(frames);
        self.blend.weight = law.weight_after(frames);
    }

    /// The "track" set's texture for the showing stage, where the title
    /// ships one and it decoded.
    ///
    /// **The set with the art in it**, sampled by every chunk whose own
    /// render-block flags carry the track bit (124 of Talon's Junction's
    /// 983) through the `zoneTexInner`/`zoneTexOuter` shader parameters. The
    /// other 859 sample [`Self::stage_scene_art`], the "general" set: fifteen
    /// byte-identical flat whites whose alpha is 255 everywhere, so a scene
    /// chunk's surface is its `Scene.Texture Colour` flat plus the rim terms.
    /// `mesh.wgsl`'s `zone_sample` picks per fragment; see
    /// [`Self::zone_uniform`] and [`oag_mesh::mesh_render::Zone`].
    ///
    /// `None` for a stage past the set, or one whose entry did not decode -
    /// the slots are positional, so a hole stays a hole.
    #[must_use]
    pub fn stage_art(&self) -> Option<&Arc<ModelTexture>> {
        self.art_at(self.blend.current)
    }

    /// The "general" set's texture for the showing stage, on the same terms
    /// as [`Self::stage_art`].
    #[must_use]
    pub fn stage_scene_art(&self) -> Option<&Arc<ModelTexture>> {
        self.scene_art_at(self.blend.current)
    }

    /// [`Self::stage_art`] at an arbitrary stage rather than the showing one
    /// - what [`Self::stage_art_pair`] needs for the stage being swept out.
    fn art_at(&self, stage: u32) -> Option<&Arc<ModelTexture>> {
        self.art.get(stage as usize)?.as_ref()
    }

    /// [`Self::stage_scene_art`] at an arbitrary stage, on the same terms as
    /// [`Self::art_at`].
    fn scene_art_at(&self, stage: u32) -> Option<&Arc<ModelTexture>> {
        self.scene_art.get(stage as usize)?.as_ref()
    }

    /// All four of the showing stage's and the stage being swept out's own
    /// textures, as a drawable binds them - see
    /// [`oag_mesh::mesh_render::zone::StageArt`].
    ///
    /// **The Outer pair falls back to the Inner one, per slot** - the same
    /// fallback [`Self::zone_uniform`] already gives its own Outer palette,
    /// `stage_palette(previous).unwrap_or(palette)`. A stage with nothing to
    /// sweep out (no transition read, no previous stage in the ladder, or a
    /// previous stage whose own texture did not decode) must not fall
    /// through to [`mesh_render::zone::StageArt`]'s black placeholder: that
    /// reads as "no texture", and painting the world black outside the
    /// sphere is a worse wrong picture than the sphere test being a no-op,
    /// which is what every other draw before a stage change already is.
    #[must_use]
    pub fn stage_art_pair(&self) -> mesh_render::zone::StageArt {
        let track = self.stage_art().cloned();
        let scene = self.stage_scene_art().cloned();
        let track_outer = self
            .art_at(self.wavefront.previous)
            .cloned()
            .or_else(|| track.clone());
        let scene_outer = self
            .scene_art_at(self.wavefront.previous)
            .cloned()
            .or_else(|| scene.clone());
        mesh_render::zone::StageArt {
            track,
            scene,
            track_outer,
            scene_outer,
        }
    }

    /// How many of this title's per-stage "track" textures decoded, and how
    /// many it numbers.
    #[must_use]
    pub fn stage_art_counts(&self) -> (usize, usize) {
        (self.art.iter().flatten().count(), self.art.len())
    }

    /// The same for the "general" set.
    #[must_use]
    pub fn stage_scene_art_counts(&self) -> (usize, usize) {
        (
            self.scene_art.iter().flatten().count(),
            self.scene_art.len(),
        )
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

    /// How many zones away the next speed class is, or `None` on the top rung
    /// or a title with no recovered ladder.
    ///
    /// The HUD's "next speed class" bar is drawn beside **the zone number that
    /// class starts at** - the maintainer's own observation of the original -
    /// so this is the row of the ladder it belongs on, counted from the current
    /// one.
    #[must_use]
    pub fn zones_to_next_stage(&self, zone: u16) -> Option<u32> {
        Some(u32::from(self.stages?.next_zone(zone)?.checked_sub(zone)?))
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
    /// its own. Also a no-op once [`Self::pin_stage`] has pinned this grade -
    /// the override wins over the ladder rather than being overwritten by it
    /// on the next call.
    pub fn show_zone(&mut self, zone: u16) -> bool {
        if self.pinned {
            return false;
        }
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

    /// Sets the cross-fade weight directly. A race never calls this: on a
    /// title with a read transition [`Self::follow`] derives the weight every
    /// frame, and on one without, [`Self::show_zone`] leaves it at rest. For
    /// tests, and for reading one stage's palette at a chosen mix.
    pub fn set_weight(&mut self, weight: f32) {
        self.blend.weight = weight;
    }

    /// Applies a pending request, the way `Environment_UpdateStageBlend` does:
    /// when the requested stage differs from the current one, it becomes the
    /// current one, the weight resets and the transition sphere restarts at
    /// its first radius with the old stage outside it.
    ///
    /// Answers whether a stage change actually happened, which is what the
    /// traced code gates its transition on. The per-frame advance of that
    /// transition is [`Self::follow`]'s; the call the commit also makes
    /// (`FUN_0067a7d8`) is still unidentified and nothing is invented for it.
    pub fn commit(&mut self) -> bool {
        if self.blend.requested == self.blend.current {
            return false;
        }
        let previous = self.blend.current;
        self.blend.current = self.blend.requested;
        // `+0x18 = 0` at the commit, verbatim: the new stage starts with no
        // share of the palette, and the sphere starts at its first radius
        // with the old stage outside it. See `StageBlend::weight` and
        // `Wavefront`.
        self.blend.weight = 0.0;
        self.wavefront.previous = previous;
        self.wavefront.frames = 0;
        self.wavefront.radius = self.transition.map_or(0.0, |law| law.radius_after(0));
        true
    }

    /// Shows the committed stage whole: the weight at rest and no sphere in
    /// flight. What the loader does to an opening stage and what
    /// [`Self::pin_stage`] does to a pinned one.
    pub fn show_whole(&mut self) {
        self.settle();
    }

    /// The `--zone-stage` development override: commits `stage` whole and
    /// then holds this grade there for the rest of its life.
    ///
    /// **Wins over the ladder, on any title.** [`Self::show_zone`] is called
    /// once a frame from `crate::Scene::sync_zone_grade` and
    /// would otherwise re-derive the stage from the zone counter on the very
    /// next call - `zone 0` already maps to a non-zero stage on both titles
    /// with a recovered ladder (see the module docs), so the override needs
    /// this rather than a one-time commit at load. There is no per-title
    /// branch here: a title with no recovered ladder already leaves
    /// [`Self::show_zone`] a no-op, so pinning changes nothing there beyond
    /// making that explicit.
    pub fn pin_stage(&mut self, stage: u32) {
        self.request_stage(stage);
        self.commit();
        // A pinned stage is shown whole, not mid-transition - the same reason
        // the opening-stage load logic settles after a commit zeroes the
        // weight.
        self.settle();
        self.pinned = true;
        // Once, here, rather than every frame `show_zone` now declines to
        // step - the same "log the edge, not the state" rule
        // `Scene::sync_zone_grade` follows for an actual step.
        log::info!(
            "zone: --zone-stage pins the colour grade to stage {}; the ladder will not step it \
             for the rest of this run",
            self.blend.current,
        );
    }

    /// The palette showing right now: the current stage cross-faded against
    /// the stage the transition is sweeping out at the current weight.
    ///
    /// The stage outside the sphere is [`Wavefront::previous`] - one rung
    /// down the ladder in play, and the showing stage itself when nothing is
    /// in flight, where the cross-fade is the identity whatever the weight.
    #[must_use]
    pub fn palette(&self) -> StagePalette {
        let Some(current) = self.table.stage_palette(self.blend.current) else {
            return StagePalette::default();
        };
        let previous = self
            .table
            .stage_palette(self.wavefront.previous)
            .unwrap_or(current);
        current.cross_fade(previous, self.blend.weight)
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
        let graded = mesh_render::Light::authored(
            base.direction,
            palette.sun_colour.unwrap_or(base.sun),
            palette.ambient_colour.unwrap_or(base.ambient),
            palette.prelit_scale.unwrap_or(base.prelit_scale),
            palette.prelit_power.unwrap_or(base.prelit_power),
            // Not in this schema at all, so the circuit's own value stands.
            base.specular_scale,
        );
        // **`authored` clears Omega's prelit flag and bias**, so a graded
        // Omega rig would silently go back to HD's combination. Omega ships no
        // Zone palette today (`zone_palette: None`), so nothing reaches this
        // with the flag set; the carry keeps it that way when one does.
        if base.nova > 0.5 {
            graded.with_nova_prelit(
                graded.prelit_scale[0],
                base.prelit_bias,
                graded.prelit_power[0],
            )
        } else {
            graded
        }
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
    /// [`oag_mesh::mesh_render::Zone`] carries the rule and the full list of
    /// terms left out; the two values assembled here are:
    ///
    /// - `zoneColourTint.xy`, the file's own title-wide `Texture U scale` /
    ///   `Texture V scale`. `Environment_RegisterStageSchema` (`0x003d0b98`,
    ///   `ps3-hdfury-eu`) registers those two keys directly into shader
    ///   parameter 52's storage, so this is the whole of the binding rather
    ///   than a reading of it. Confidence 85.
    /// - `zoneEffectInner`/`zoneEffectOuter`, the showing stage's
    ///   **`Texture Colour`** - `Track.` and `Scene.` both.
    ///   `Environment_UpdateStageBlend` (`0x003da540`) copies stage `n`'s
    ///   into the inner parameter and stage `n - 1`'s into the outer one.
    ///   Confidence 84.
    ///
    /// **Both groups, and the chunk decides which it reads.** HD publishes
    /// these parameters twice and the two publications are paired: one binds
    /// the `zoneMode*` textures beside the `Scene.*` colours, the other binds
    /// `zoneModeTrack*` beside the `Track.*` ones (read out of
    /// `FUN_003ff860`'s two blocks, confidence 85), and which block a chunk
    /// goes through is bit 0 of its own render-block flags in the
    /// `.rcsmodel` - `oag_rcs::rcsmodel::Mesh::is_track`, set on 124 of
    /// Talon's Junction's 983 chunks. [`mesh_render::Zone::track`] and
    /// [`mesh_render::Zone::scene`] carry one group each and `mesh.wgsl`
    /// selects per fragment. It matters: `Scene.Texture Colour` is authored
    /// black on stage `Start` and its `Track` sibling is `9.0` there, which
    /// is a dark environment around a lit road - the original's own opening
    /// frame. Until 2026-09-15 this build bound the track group to every
    /// chunk and drew the whole circuit in the road's colours.
    ///
    /// **Two stages' own palettes, not [`Self::palette`]'s cross-fade.** The
    /// original does not blend these in colour space at all: it hands the
    /// shader both stages - the showing one as `zone*Inner`, the one being
    /// swept out as `zone*Outer` - and picks between them per pixel with a
    /// sphere test against `zoneOrigin` and `zoneColourTint.w`. Those are
    /// [`mesh_render::Zone::origin`] and [`mesh_render::Zone::radius`] here,
    /// from [`Self::wavefront`]; the outer pair is [`Wavefront::previous`]'s
    /// palette, and equal to the inner pair whenever nothing is in flight.
    ///
    /// `enabled` is `0.0`, and the whole term disappears, unless the file
    /// authors the UV scale *and* the stage authors `Track.Texture Colour`
    /// *and* the stage's track texture decoded. A missing input draws
    /// nothing. The scene group is not part of that gate: a stage authoring
    /// no `Scene.*` key, or whose general texture did not decode, draws its
    /// scene chunks black - which is what `Start` authors anyway - and the
    /// load report says so.
    #[must_use]
    pub fn zone_uniform(&self) -> mesh_render::Zone {
        /// An authored rgb as a `float4` with an unused `.w`, or all-zero
        /// where the stage authors none.
        fn rgb0(colour: Option<[f32; 3]>) -> [f32; 4] {
            let [r, g, b] = colour.unwrap_or([0.0; 3]);
            [r, g, b, 0.0]
        }
        /// One publication's three parameters from its three keys.
        ///
        /// `effect.w` scales the visualiser glow: `EQ brightness`, `0.0` on
        /// `Start` and `20.0` from `Sub Venom` on for the track group.
        /// `base`/`base_alt` are `zoneBase*`/`zoneBaseAlt*`, the two rim-lit
        /// summands of the surface: `zoneBase.rgb * rim^10 +
        /// zoneBaseAlt.rgb * rim^5`, both exponents inline literals in the
        /// microcode. **Not a separate family.** These are consumed by
        /// 20,084 of the 20,214 Zone-bearing fragment blocks on the disc -
        /// more than `zoneTexInner` is - and co-occur with a sampled
        /// `zoneTex*` in 18,032 of them. Every one of the twelve racing
        /// circuits carries this shape and nothing else. See
        /// `oag_mesh::mesh_render::Zone` for the census and the second,
        /// arena-only shape it is not. Zero where the stage authors nothing,
        /// which is the identity on a summand.
        fn group(
            texture_colour: Option<[f32; 3]>,
            eq_brightness: Option<f32>,
            base_colour_highlight: Option<[f32; 3]>,
            base_colour: Option<[f32; 3]>,
        ) -> mesh_render::ZoneSet {
            let [r, g, b] = texture_colour.unwrap_or([0.0; 3]);
            mesh_render::ZoneSet {
                effect: [r, g, b, eq_brightness.unwrap_or(0.0)],
                base: rgb0(base_colour_highlight),
                base_alt: rgb0(base_colour),
            }
        }
        /// Both of one stage's publications.
        fn groups(palette: &StagePalette) -> (mesh_render::ZoneSet, mesh_render::ZoneSet) {
            (
                group(
                    palette.track_texture_colour,
                    palette.track_eq_brightness,
                    palette.track_base_colour_highlight,
                    palette.track_base_colour,
                ),
                group(
                    palette.scene_texture_colour,
                    palette.scene_eq_brightness,
                    palette.scene_base_colour_highlight,
                    palette.scene_base_colour,
                ),
            )
        }
        let off = mesh_render::Zone::default();
        let (Some(uv_scale), Some(palette)) = (
            self.table.zone_uv_scale(),
            self.table.stage_palette(self.blend.current),
        ) else {
            return off;
        };
        if palette.track_texture_colour.is_none() || self.stage_art().is_none() {
            return off;
        }
        // The stage outside the sphere; the showing stage's own palette where
        // the previous one is not in the file, so the test selects the same
        // colours on both sides rather than a black one outside.
        let outer = self
            .table
            .stage_palette(self.wavefront.previous)
            .unwrap_or(palette);
        let (track, scene) = groups(&palette);
        let (track_outer, scene_outer) = groups(&outer);
        let [x, y, z] = self.wavefront.origin;
        mesh_render::Zone {
            uv_scale,
            enabled: 1.0,
            radius: self.wavefront.radius,
            origin: [x, y, z, 1.0],
            track,
            scene,
            track_outer,
            scene_outer,
        }
    }

    /// The showing stage's own `EQ colour tint`, for tinting the visualiser
    /// glow - see `oag_mesh::mesh_render::zone::write_vis`.
    ///
    /// **Naming, not a traced shader input.** `EQ colour tint` and its
    /// `EQ analogue colour tint` sibling are parsed and land in this table's
    /// own per-stage struct, but no consumer of either is traced in the
    /// executable - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// row for `+0x150`. Confidence 70 on "this is the spectrum's own
    /// colour", from the name and the `EQ` cluster's own reading in
    /// `docs/formats/effectsettings.md`. The showing stage's own palette
    /// rather than [`Self::palette`]'s cross-fade, on the same terms
    /// [`Self::zone_uniform`] already reads unblended.
    #[must_use]
    pub fn eq_tint(&self) -> Option<[u8; 3]> {
        let palette = self.table.stage_palette(self.blend.current)?;
        let [r, g, b, _] = palette.eq_colour_tint?;
        Some([r, g, b])
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
            let (scene_decoded, scene_named) = self.stage_scene_art_counts();
            if uniform.enabled > 0.0 {
                let [r, g, b, _] = uniform.track.effect;
                let [sr, sg, sb, _] = uniform.scene.effect;
                let [u, v] = uniform.uv_scale;
                // A stage whose `Texture Colour` is black multiplies that
                // group's texture term out. Said plainly rather than reported
                // as a draw, because it is the file's own statement and not a
                // failure: `Start` authors Scene that way, so its environment
                // is dark around a lit road.
                let describe = |group: &str, [r, g, b]: [f32; 3]| {
                    if [r, g, b] == [0.0; 3] {
                        format!(
                            "{group} chunks black, this stage authoring {group}.Texture Colour black"
                        )
                    } else {
                        format!("{group} chunks zoneEffect [{r}, {g}, {b}]")
                    }
                };
                format!(
                    "; the Zone recolour draws {} and {}: stage {} art of {decoded}/{named} \
                     track and {scene_decoded}/{scene_named} scene, zone UV scale [{u}, {v}]",
                    describe("Track", [r, g, b]),
                    describe("Scene", [sr, sg, sb]),
                    self.blend.current,
                )
            } else {
                let missing = if self.table.zone_uv_scale().is_none() {
                    "the file authors no Texture U/V scale"
                } else if self.stage_art().is_none() {
                    "this stage has no decoded texture"
                } else {
                    "this stage authors no Track.Texture Colour"
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
