//! What a caller asks a built [`Scene`] after the fact: the Zone ladder, the
//! resize a window drives, and what its pipelines were actually built with.
//!
//! Split out of `scene.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` - and a real seam: [`Scene::new`] is a single
//! several-hundred-line construction, and everything here is instead a small,
//! independent question about a `Scene` that already exists.

use log::debug;

use super::Scene;
use super::frame::{depth_texture, msaa_color_texture};
use super::motion::{self, Attachments};
use crate::Race;

impl Scene {
    /// How many of the track's placeholder materials were pointed at an advert
    /// card when this scene was built.
    #[cfg(test)]
    pub(crate) fn adverts_rebound(&self) -> usize {
        self.adverts
            .as_ref()
            .map_or(0, crate::adverts::Cards::rebound)
    }

    /// This scene's pipeline cache, as `(asked for, reused, distinct built)`.
    #[must_use]
    pub fn build_cache(&self) -> (u32, u32, usize) {
        self.build_cache
    }

    /// How many rows down the ladder the next speed class begins, or `None`
    /// with no grade, no recovered ladder, or on the top rung.
    ///
    /// See [`crate::zone_grade::ZoneGrade::zones_to_next_stage`].
    #[must_use]
    pub fn zones_to_next_stage(&self, zone: u16) -> Option<u32> {
        self.zone_grade.as_ref()?.zones_to_next_stage(zone)
    }

    /// Which rung of the Zone ladder this scene's grade is showing, or `None`
    /// outside Zone mode and on a title shipping no stage table.
    ///
    /// The HUD's speed-class name reads this rather than the zone counter, so
    /// the two halves of the escalation - the colour grade and the word on
    /// screen - move together and are wrong together. See
    /// `oag_hd::hud::ZONE_SPEED_CLASSES` for why the zone counter is not the
    /// right question to ask.
    #[must_use]
    pub fn zone_stage(&self) -> Option<u32> {
        Some(self.zone_grade.as_ref()?.blend().current)
    }

    /// Points the Zone colour grade at the zone the race has reached, and
    /// answers whether that moved it.
    ///
    /// Called once a frame, which is where the original does it:
    /// `Zone_UpdateStage` (`0x81044cfc`, `vita-2048-eu-v104`) runs from the
    /// main render-update loop every frame, reads the stage index off the
    /// craft and shows it. A no-op outside Zone, on a title shipping no stage
    /// table, and on one whose zone-to-stage ladder is unrecovered - see
    /// [`crate::zone_grade::ZoneGrade::show_zone`].
    ///
    /// **One precedence question is untested rather than answered.** `frame`
    /// samples the circuit's own `fogCube` volumes *after* laying the grade
    /// over the authored fog, and a volume the camera sits inside replaces the
    /// result outright. No title reaches that today - the two that ship a
    /// stage table both load their geometry through the PSARC path, which
    /// parses no `fogCube` at all, so `fog_volumes` is empty on both - and
    /// which of the two the original prefers is not recovered, since
    /// `Environment_UpdateStageBlend`'s own consumer is still unfound. Worth
    /// knowing before a title with both ever loads.
    pub fn sync_zone_grade(&mut self, race: &Race) -> bool {
        let zone = race.sim.world.primary_race().zone;
        let Some(grade) = self.zone_grade.as_mut() else {
            return false;
        };
        let stepped = grade.show_zone(zone);
        if stepped {
            // Once per stage change, which is at most fifteen times a race -
            // the same "log the edge, not the state" rule the announcer cue
            // follows. It is also the only headless evidence that the grade
            // moved at all, since a `--screenshot` cannot say so on its own.
            debug!(
                "zone {zone}: the colour grade steps to stage {}",
                grade.blend().current
            );
        }
        // Where the transition sphere is this frame, from the race's own
        // zone clock rather than a counter of calls - so this reads the same
        // whether it runs once a frame in the window or once at the end of a
        // headless capture. Centred on the player's craft, which is what
        // `Scene_PrepareFrame` re-reads `zoneOrigin` from every frame. See
        // `ZoneGrade::follow`.
        grade.follow(
            zone,
            race.sim.world.primary_race().zone_timer,
            race.dt(),
            race.ship().physics.body.position.to_array(),
        );
        stepped
    }

    /// Rebuilds bind group 2 on every drawable that reads the Zone effect
    /// for real - the track, its collision wireframe, its pads and its
    /// weapon pads - with the showing stage's own four textures. Ships,
    /// boost, flares, shields and weapon models are not among them: their
    /// materials carry no Zone block at all, so their stage art is never
    /// sampled and rebuilding it would be wasted uploads for no picture
    /// change - see `oag_mesh::mesh_render::Zone`'s own census.
    ///
    /// **Call this once, right after [`Self::sync_zone_grade`] answers
    /// `true`** - the stage-change edge, never every frame. See
    /// `oag_mesh::mesh_render::zone::rebind`'s own doc comment for why a
    /// per-frame call would still draw correctly and why it would still be
    /// the wrong thing to do.
    pub fn rebind_zone_art(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(grade) = &self.zone_grade else {
            return;
        };
        let stage = grade.stage_art_pair();
        self.track.rebind_zone(device, queue, &stage);
        for drawable in [
            self.sky.as_mut(),
            self.collision.as_mut(),
            self.pads.as_mut(),
            self.weapon_pads.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.rebind_zone(device, queue, &stage);
        }
    }

    /// Rebuilds the depth buffer, and the MSAA colour target if there is one,
    /// for a new viewport size.
    ///
    /// A colour or depth attachment whose size does not match the others is a
    /// validation error, so this is not optional on resize.
    pub fn resize(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) {
        let sample_count = self.msaa.samples();
        // Under the HD chain every scene pipeline was built against the
        // linear float format, so the MSAA attachment has to match it, and
        // the chain's own targets track the viewport.
        let format = if self.draws_linear() {
            oag_gpu::formats::SCENE_FORMAT
        } else {
            format
        };
        if let Some(hd) = &mut self.hd {
            hd.resize(device, size);
        }
        if let Some(omega) = &mut self.omega {
            omega.resize(device, size);
        }
        self.depth = depth_texture(device, size, sample_count);
        self.velocity = motion::velocity_texture(device, size, sample_count);
        self.msaa_color = msaa_color_texture(device, format, size, sample_count);
        self.attachment_views =
            Attachments::new(&self.depth, &self.velocity, self.msaa_color.as_ref());
    }

    /// What this scene's pipelines were actually built with, for the restart
    /// note - see [`Self::msaa_color`].
    #[must_use]
    pub fn msaa(&self) -> oag_display::display::Msaa {
        self.msaa
    }

    /// Whether this scene has Wipeout HD/Fury's read bloom chain.
    ///
    /// **Fixed for the scene's lifetime**, decided at [`Self::new`] by whether
    /// the circuit authors an `HDR and Bloom` block - never Pulse or Pure, and
    /// not every HD circuit either. Lets `Session::read_timing_and_feed_drs`
    /// tell "the ring was full" from "there is no chain to measure", the same
    /// distinction `reconstruction.is_temporal()` draws for FSR 3.1. See
    /// [ADR-0043](../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md).
    #[must_use]
    pub fn has_hd_bloom(&self) -> bool {
        self.hd.is_some()
    }

    /// Whether this circuit stands a start gantry over its own start line.
    ///
    /// True exactly when the track authored a mount `race::gantry::place`
    /// could measure and the model loaded. What reads it is the **HUD**
    /// countdown: the maintainer's own account of the original is that the
    /// `3`, `2`, `1`, `GO` is on the gantry, so the screen overlay stands
    /// down where the gantry is showing it and stays for a circuit with no
    /// gantry to show it on. See `oag_hud::countdown`.
    #[must_use]
    pub fn draws_gantry(&self) -> bool {
        self.gantry.is_some()
    }
}
