//! Reading back the frame's GPU timing rings and feeding the dynamic-resolution
//! controller.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, and a real seam besides: this is everything
//! [ADR-0042] and [ADR-0043] added in one place - four rings read together, a
//! `drs::Cost` assembled only from readings that name the *same* frame, and
//! the once-per-spell "unreachable" line.
//!
//! [ADR-0042]: ../../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md
//! [ADR-0043]: ../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md

use log::{trace, warn};
use oag_game::{drs, settings};

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Reads back the frame's GPU timing rings and feeds the controller.
    ///
    /// This module's own doc comment says why it is here rather than in
    /// `frame.rs`.
    ///
    /// **All four rings are read before any of them is acted on.** Since
    /// ADR-0042 the controller is fed a `drs::Cost` built from more than one
    /// of them, and a partial sum reads as a cheap frame - which steps the
    /// scale the wrong way. The same rule `PassTimer` already states for one
    /// ring, one level up: a gap in the samples beats a wrong number in them.
    ///
    /// `render_profile` and `drs_limits` are both computed once in `frame`,
    /// before the encoder that used them was built, and passed in rather than
    /// recomputed - `render_profile` in particular walks `settings` and
    /// `self.shell`, neither of which changed mid-frame, so recomputing it
    /// here would be a second read of the same answer with a chance to
    /// disagree with the first.
    pub(super) fn read_timing_and_feed_drs(
        &mut self,
        render_profile: &settings::RenderProfile,
        drs_limits: drs::Limits,
    ) {
        let read = |timer: &mut Option<oag_render::timing::PassTimer>, device| {
            timer.as_mut().and_then(|timer| timer.read(device))
        };
        let scene_reading = read(&mut self.pass_timer, &self.gpu.device);
        let blur_reading = read(&mut self.blur_timer, &self.gpu.device);
        let hd_bloom_reading = read(&mut self.hd_bloom_timer, &self.gpu.device);
        let upscale_reading = read(&mut self.upscale_timer, &self.gpu.device);
        // The scene's own fact, not the settings row: `render_profile` names
        // no `hd_bloom` field at all, because whether the chain exists is a
        // per-*circuit* decision baked into `race::Scene::new`, not a player
        // setting. See `race::Scene::has_hd_bloom`.
        let has_hd_bloom = matches!(&self.stage, Stage::Race(stage) if stage.scene.has_hd_bloom());

        if let Some(reading) = &upscale_reading
            && reading.frame > self.stall_frame
            && reading.seconds > 0.0
        {
            self.upscale_cost.record(reading.seconds);
            trace!(
                "fsr3 chain: frame {} took {:.3} ms, read on frame {}",
                reading.frame,
                reading.seconds * 1000.0,
                self.frame_index
            );
        }
        if let Some(reading) = &blur_reading
            && reading.frame > self.stall_frame
            && reading.seconds > 0.0
        {
            self.blur_cost.record(reading.seconds);
            trace!(
                "motion blur chain: frame {} took {:.3} ms, read on frame {}",
                reading.frame,
                reading.seconds * 1000.0,
                self.frame_index
            );
        }
        if let Some(reading) = &hd_bloom_reading
            && reading.frame > self.stall_frame
            && reading.seconds > 0.0
        {
            self.hd_bloom_cost.record(reading.seconds);
            trace!(
                "hd bloom chain: frame {} took {:.3} ms, read on frame {}",
                reading.frame,
                reading.seconds * 1000.0,
                self.frame_index
            );
        }
        let Some(reading) = &scene_reading else {
            return;
        };
        if reading.frame > self.stall_frame {
            self.scene_cost.record(reading.seconds);
            // **Only the parts that name this same frame.** The four rings
            // are the same size, claimed on the same frames and resolved into
            // the same encoder, so in a running race they come back together;
            // a mismatch means one of them skipped a slot, and a `Cost`
            // assembled across two frames is not a frame's cost.
            let matching = |other: &Option<oag_render::timing::Reading>| {
                other
                    .as_ref()
                    .filter(|other| other.frame == reading.frame)
                    .map(|other| other.seconds)
            };
            let blur = matching(&blur_reading).unwrap_or(0.0);
            let hd_bloom = matching(&hd_bloom_reading);
            let fixed = matching(&upscale_reading);
            // **Absent-because-it-did-not-run is zero; absent-because-it-was-not-measured
            // is a skipped frame**, and the two are told apart by whether
            // anything temporal is resolving at all. Treating an unmeasured
            // chain as free would inflate the budget by the largest single
            // term in it.
            let fixed = match (fixed, render_profile.reconstruction.is_temporal()) {
                (Some(seconds), _) => Some(seconds),
                (None, false) => Some(0.0),
                (None, true) => None,
            };
            // The same shape of question for `hd_bloom`, answered off the
            // scene's own `has_hd_bloom` rather than a settings row - see
            // ADR-0043.
            let hd_bloom = match (hd_bloom, has_hd_bloom) {
                (Some(seconds), _) => Some(seconds),
                (None, false) => Some(0.0),
                (None, true) => None,
            };
            if let (Some(fixed), Some(hd_bloom)) = (fixed, hd_bloom) {
                let cost = drs::Cost {
                    scalable: reading.seconds + blur + hd_bloom,
                    fixed,
                };
                // The same reading, inside the same guard: a load lands in one
                // frame's timing and would otherwise drive the scale to the
                // floor and take seconds to climb back.
                if let Some(extent) = self.drs.record(cost, drs_limits) {
                    trace!(
                        "dynamic resolution: {}x{} ({:.0}% of {}x{}) - scalable {:.3} ms, fixed {:.3} ms",
                        extent.0,
                        extent.1,
                        self.drs.scale() * 100.0,
                        drs_limits.ceiling().0,
                        drs_limits.ceiling().1,
                        cost.scalable * 1000.0,
                        cost.fixed * 1000.0
                    );
                }
                // Said once per spell rather than per frame - see
                // `drs::Controller::unreachable`.
                if self.drs.unreachable() && !self.drs_unreachable_said {
                    warn!(
                        "dynamic resolution: {} fps is out of reach here - {:.3} ms of this \
                         frame is fixed cost the render scale cannot shrink, against a \
                         {:.3} ms budget. Lower the target, or the rows that feed the \
                         fixed cost (reconstruction, msaa)",
                        drs_limits.target(),
                        cost.fixed * 1000.0,
                        drs_limits.target().period().unwrap_or(0.0) * 1000.0
                    );
                }
                self.drs_unreachable_said = self.drs.unreachable();
            }
        }
        trace!(
            "scene pass: frame {} took {:.3} ms, read on frame {}",
            reading.frame,
            reading.seconds * 1000.0,
            self.frame_index
        );
    }
}
