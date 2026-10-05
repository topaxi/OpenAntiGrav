//! Reading back the frame's GPU timing rings and feeding the dynamic-resolution
//! controller.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, and a real seam besides: this is everything
//! [ADR-0042], [ADR-0043] and [ADR-0045] added in one place - five rings read
//! together, a `drs::Cost` assembled only from readings that name the *same*
//! frame, and the once-per-spell "unreachable" line. The three free functions
//! at the top are the claiming half of the same plumbing, here rather than at
//! their call site for the borrow reason their own docs give.
//!
//! [ADR-0042]: ../../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md
//! [ADR-0043]: ../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md
//! [ADR-0045]: ../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md

use log::{trace, warn};
use oag_game::settings;
use oag_present::drs;

use crate::stage::Stage;

use super::Session;

/// Take a slot on both FSR 3.1 rings for `frame`, or on neither.
///
/// **Free functions rather than `Session` methods**, all three of these: the
/// call site is beside `self.framebuffer.resolve_scene(..)`, which borrows
/// `self.framebuffer` mutably, and a method taking `&self` would borrow the
/// framebuffer along with the timers. Taking the two rings by reference
/// borrows exactly the two fields involved.
///
/// Both or neither, on the same frame index: since
/// [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)
/// the chain is two compute passes, and [`Session::feed_drs`] pairs their
/// readings by frame. A ring claimed without its partner is a frame the
/// controller cannot assemble a `Cost` for.
pub(super) fn claim_upscale(
    scaled: Option<&mut oag_gpu::timing::PassTimer>,
    presented: Option<&mut oag_gpu::timing::PassTimer>,
    frame: u64,
) {
    let mut rings = [scaled, presented];
    for timer in rings.iter_mut().flatten() {
        timer.begin(frame);
    }
    // **A half-claim is given straight back, and it is worse than no claim
    // at all.** A ring whose four slots are all in flight claims nothing and
    // `compute_writes` then returns `None` - which makes
    // [`upscale_timestamps`] hand the chain no timestamps, so *neither* pass
    // writes one. The ring that did claim would then resolve a pair nothing
    // wrote, and an unwritten pair does not read back as zero: its value is
    // unspecified and the query set is not cleared between frames. That is
    // the "plausible number attributed to the wrong frame" this whole
    // arrangement exists to avoid, and with one ring it could not happen -
    // claiming and writing were the same condition.
    if rings
        .iter()
        .flatten()
        .any(|timer| timer.compute_writes().is_none())
    {
        for timer in rings.iter_mut().flatten() {
            timer.abandon();
        }
    }
}

/// Give both claims back, for a frame the chain did not encode.
///
/// A pair claimed and never written does not read back as zero - its value is
/// unspecified and the query set is not cleared between frames - so an
/// unwritten claim has to be abandoned rather than resolved. See
/// `Framebuffer::resolve_scene`'s return value, which is what gates this.
pub(super) fn abandon_upscale(
    scaled: Option<&mut oag_gpu::timing::PassTimer>,
    presented: Option<&mut oag_gpu::timing::PassTimer>,
) {
    for timer in [scaled, presented].into_iter().flatten() {
        timer.abandon();
    }
}

/// Copy both halves' pairs out of their query sets, into the encoder the
/// passes were recorded in.
///
/// **Paired for the same reason the claim is, and with a sharper edge.** A
/// slot claimed and never resolved never comes back, and after four of them
/// the ring is dead for the run - so a resolve that reaches one ring and not
/// the other silently ends measurement. That is not hypothetical: while
/// ADR-0045 was being written, an edit put this ring's `resolve` at the
/// *claim* site by matching the wrong line, it compiled, and the whole test
/// suite passed - nothing in it reaches the frame loop's timer plumbing.
/// One function that takes both is what makes that edit impossible rather
/// than merely unlikely.
pub(super) fn resolve_upscale(
    scaled: Option<&mut oag_gpu::timing::PassTimer>,
    presented: Option<&mut oag_gpu::timing::PassTimer>,
    encoder: &mut wgpu::CommandEncoder,
) {
    for timer in [scaled, presented].into_iter().flatten() {
        timer.resolve(encoder);
    }
}

/// One pair around each half of the chain, on the frames [`claim_upscale`]
/// took slots for.
///
/// `None` unless *both* rings have a slot: a device that gave one timer and
/// not the other cannot happen - both are the same `PassTimer::new` against
/// the same device - but a ring whose four slots are all in flight can, and
/// timing half a chain is worse than timing none of it. [`claim_upscale`]
/// has already given a lone claim back by the time this runs, so the `?`s
/// below are both-or-neither in practice; they are written to be safe on
/// their own regardless, because this is the function the chain's timing
/// actually flows through.
pub(super) fn upscale_timestamps<'a>(
    scaled: Option<&'a oag_gpu::timing::PassTimer>,
    presented: Option<&'a oag_gpu::timing::PassTimer>,
) -> Option<oag_post::fsr3::ChainTimestamps<'a>> {
    let scaled = scaled?.compute_writes()?;
    let presented = presented?.compute_writes()?;
    Some(oag_post::fsr3::ChainTimestamps { scaled, presented })
}

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
    ///
    /// `frame_seconds` comes the same way and for a sharper version of the
    /// same reason: it is the one `elapsed` this loop reads a clock for, the
    /// same one `Session::meter` is fed, and `drs::Residual` may only be
    /// handed a wall-clock reading somebody else took - see [ADR-0044]. It is
    /// `None` on a frame that carried a load, which is the same guard
    /// `meter.clear()` is behind.
    ///
    /// **The instantaneous frame time rather than `meter.stats().mean_ms`**,
    /// which is the number the overlay's `OTHER` row subtracts from. A mean
    /// over the last hundred-odd frames against a single frame's timed passes
    /// is two signals a hundred frames out of phase, and the residual only
    /// ever tightens on a reading *below* what it holds, so that phase error
    /// is not noise that averages out - every dip it invents is kept. What is
    /// paired here is one frame against the passes of the frame before it -
    /// `PassTimer` resolves exactly one frame late, measured over 5,306
    /// consecutive readings, and that offset is the reason
    /// `drs::residual::LEARN_RATE`'s own note gives for a reading that can
    /// read low while the cost is moving. One frame of skew, not a hundred,
    /// and the estimate's own smoothing is then the only smoothing.
    ///
    /// [ADR-0044]: ../../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md
    pub(super) fn read_timing_and_feed_drs(
        &mut self,
        render_profile: &settings::RenderProfile,
        frame_seconds: Option<f32>,
        drs_limits: drs::Limits,
    ) {
        // **Drained, not read one at a time.** `PassTimer::read` hands back
        // the oldest ready reading and leaves the rest for next frame, and
        // that is exactly what let the controller go silent for a whole
        // race: the four rings are polled one after another, so a GPU
        // completion landing between two of those polls reaches the later
        // ring this frame and the earlier ring next frame - after which the
        // earlier ring holds a backlog of one, returns the older reading on
        // every call, and its reading never names the same frame as the
        // other three again until a frame arrives with nothing ready at all.
        // Under vsync at a steady rate that is never. `drain` takes
        // everything that is ready, so no ring can carry a backlog and two
        // rings can disagree by at most the one frame the race itself costs.
        // See `PassTimer::drain`.
        let drain = |timer: &mut Option<oag_gpu::timing::PassTimer>, device| {
            let mut readings = [None; oag_gpu::timing::PassTimer::SLOTS];
            if let Some(timer) = timer.as_mut() {
                timer.drain(device, &mut readings);
            }
            readings
        };
        let scene_readings = drain(&mut self.pass_timer, &self.gpu.device);
        let blur_readings = drain(&mut self.blur_timer, &self.gpu.device);
        let hd_bloom_readings = drain(&mut self.hd_bloom_timer, &self.gpu.device);
        let upscale_readings = drain(&mut self.upscale_timer, &self.gpu.device);
        let upscale_presented_readings = drain(&mut self.upscale_presented_timer, &self.gpu.device);
        // The scene's own fact, not the settings row: `render_profile` names
        // no `hd_bloom` field at all, because whether the chain exists is a
        // per-*circuit* decision baked into `race::Scene::new`, not a player
        // setting. See `race::Scene::has_hd_bloom`.
        let has_hd_bloom = matches!(&self.stage, Stage::Race(stage) if stage.scene.has_hd_bloom());

        for reading in upscale_readings.iter().flatten() {
            if reading.frame > self.stall_frame && reading.seconds > 0.0 {
                self.upscale_cost.record(reading.seconds);
                trace!(
                    "fsr3 chain, render resolution: frame {} took {:.3} ms, read on frame {}",
                    reading.frame,
                    reading.seconds * 1000.0,
                    self.frame_index
                );
            }
        }
        for reading in upscale_presented_readings.iter().flatten() {
            if reading.frame > self.stall_frame && reading.seconds > 0.0 {
                self.upscale_presented_cost.record(reading.seconds);
                trace!(
                    "fsr3 chain, presentation resolution: frame {} took {:.3} ms, read on frame {}",
                    reading.frame,
                    reading.seconds * 1000.0,
                    self.frame_index
                );
            }
        }
        for reading in blur_readings.iter().flatten() {
            if reading.frame > self.stall_frame && reading.seconds > 0.0 {
                self.blur_cost.record(reading.seconds);
                trace!(
                    "motion blur chain: frame {} took {:.3} ms, read on frame {}",
                    reading.frame,
                    reading.seconds * 1000.0,
                    self.frame_index
                );
            }
        }
        for reading in hd_bloom_readings.iter().flatten() {
            if reading.frame > self.stall_frame && reading.seconds > 0.0 {
                self.hd_bloom_cost.record(reading.seconds);
                trace!(
                    "hd bloom chain: frame {} took {:.3} ms, read on frame {}",
                    reading.frame,
                    reading.seconds * 1000.0,
                    self.frame_index
                );
            }
        }
        // Oldest first, which is the order `drain` fills in: a backlog of
        // scene readings is a backlog of frames, and the controller's
        // cooldown and patience count readings as frames. Each is paired
        // with this frame's wall clock, which a backlog makes two frames of
        // skew rather than one - rare now that draining is what stops a
        // backlog forming, and the residual only ever tightens on a reading
        // the frame can prove, so a stale pairing costs at most one
        // observation of the same upper bound twice.
        for reading in scene_readings.iter().flatten() {
            self.feed_drs(
                reading,
                &blur_readings,
                &hd_bloom_readings,
                &upscale_readings,
                &upscale_presented_readings,
                has_hd_bloom,
                render_profile,
                frame_seconds,
                drs_limits,
            );
        }
    }

    /// One scene reading into the controller, with the other rings' readings
    /// for the same frame beside it.
    ///
    /// Split from [`Self::read_timing_and_feed_drs`] so the loop over a
    /// drained backlog stays one line; the body is the per-frame half.
    #[expect(
        clippy::too_many_arguments,
        reason = "four rings' readings and three frame-level facts, all of \
                  which the caller computed once for the whole drain"
    )]
    fn feed_drs(
        &mut self,
        reading: &oag_gpu::timing::Reading,
        blur_readings: &[Option<oag_gpu::timing::Reading>],
        hd_bloom_readings: &[Option<oag_gpu::timing::Reading>],
        upscale_readings: &[Option<oag_gpu::timing::Reading>],
        upscale_presented_readings: &[Option<oag_gpu::timing::Reading>],
        has_hd_bloom: bool,
        render_profile: &settings::RenderProfile,
        frame_seconds: Option<f32>,
        drs_limits: drs::Limits,
    ) {
        if reading.frame > self.stall_frame {
            self.scene_cost.record(reading.seconds);
            // **Only the parts that name this same frame.** The four rings
            // are the same size, claimed on the same frames and resolved into
            // the same encoder, so in a running race they come back together;
            // a mismatch means one of them skipped a slot, and a `Cost`
            // assembled across two frames is not a frame's cost.
            let matching = |others: &[Option<oag_gpu::timing::Reading>]| {
                others
                    .iter()
                    .flatten()
                    .find(|other| other.frame == reading.frame)
                    .map(|other| other.seconds)
            };
            let blur = matching(blur_readings).unwrap_or(0.0);
            let hd_bloom = matching(hd_bloom_readings);
            // **The FSR 3.1 chain is two readings, and only one of them is
            // fixed.** Its six render-resolution dispatches fall with the
            // extent exactly as the scene pass and the motion-blur chain do;
            // `accumulate` and `rcas` run at presentation resolution and do
            // not. ADR-0042 counted the whole chain as fixed because there was
            // only one pair to count, and said so as a known pessimism;
            // [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)
            // is where the second pair arrived. **Both or neither**: the two
            // rings are claimed and abandoned together, so a frame with one
            // and not the other is a bug rather than a state to average over,
            // and `zip` treats it as "not measured yet" - the same answer a
            // missing chain gets.
            let upscale = matching(upscale_readings).zip(matching(upscale_presented_readings));
            // **Absent-because-it-did-not-run is zero; absent-because-it-was-not-measured
            // is a skipped frame**, and the two are told apart by whether
            // anything temporal is resolving at all. Treating an unmeasured
            // chain as free would inflate the budget by the largest single
            // term in it.
            //
            // **The setting alone is not enough to tell those apart.**
            // `reconstruction.is_temporal()` answers "did a player ask for
            // this", not "will a reading ever come back" - `self.gpu.temporal`
            // (the adapter) and `self.framebuffer.temporal_upscaler_viable()`
            // (whether the shaders actually built) are both permanent facts
            // once they turn false, and `will_upscale_temporally` in
            // `frame.rs` already gates the *claim* on all three together. This
            // has to gate the *read* on the same three, or a shader that fails
            // to build on one adapter and not another leaves this reading
            // `None` forever: `fixed` stays `None`, `Controller::record` is
            // never called again, and the render scale freezes wherever it
            // was the instant the build failed - see the `abandon()` call
            // beside the claim in `frame.rs` for the other half of this bug.
            let expects_upscale = render_profile.reconstruction.is_temporal()
                && self.gpu.temporal
                && self.framebuffer.temporal_upscaler_viable();
            let upscale = match (upscale, expects_upscale) {
                (Some(pair), _) => Some(pair),
                (None, false) => Some((0.0, 0.0)),
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
            if let (Some((upscale_scaled, upscale_presented)), Some(hd_bloom)) = (upscale, hd_bloom)
            {
                let cost = drs::Cost {
                    scalable: reading.seconds + blur + hd_bloom + upscale_scaled,
                    fixed: upscale_presented,
                };
                // The same reading, inside the same guard: a load lands in one
                // frame's timing and would otherwise drive the scale to the
                // floor and take seconds to climb back.
                if let Some(extent) = self.drs.record(cost, frame_seconds, drs_limits) {
                    // **The reserve is printed beside the costs**, because
                    // since ADR-0044 it is not a constant a reader can look up:
                    // two lines a minute apart can divide the same scalable
                    // cost by two different budgets, and without this there is
                    // nothing in the log that says so.
                    trace!(
                        "dynamic resolution: {}x{} ({:.0}% of {}x{}) - scalable {:.3} ms, \
                         fixed {:.3} ms, residual {:.3} ms{}",
                        extent.0,
                        extent.1,
                        self.drs.scale() * 100.0,
                        drs_limits.ceiling().0,
                        drs_limits.ceiling().1,
                        cost.scalable * 1000.0,
                        cost.fixed * 1000.0,
                        reserved_ms(&self.drs, drs_limits),
                        if self.drs.residual().learned().is_some() {
                            " (learned)"
                        } else {
                            " (assumed)"
                        }
                    );
                }
                // Said once per spell rather than per frame - see
                // `drs::Controller::unreachable`.
                if self.drs.unreachable() && !self.drs_unreachable_said {
                    warn!(
                        "dynamic resolution: {} fps is out of reach here - {:.3} ms of this \
                         frame is fixed cost the render scale cannot shrink and {:.3} ms is \
                         held back for work nothing times, against a {:.3} ms budget. Lower \
                         the target, or the rows that feed the fixed cost (reconstruction, \
                         msaa)",
                        drs_limits.target(),
                        cost.fixed * 1000.0,
                        reserved_ms(&self.drs, drs_limits),
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

/// What the controller is holding back this frame, in milliseconds.
///
/// A free function rather than a method on `drs::Residual`: the reserve is
/// seconds against a target period and the log wants milliseconds against a
/// target that may be off, and neither of those is the controller's business.
fn reserved_ms(drs: &drs::Controller, limits: drs::Limits) -> f32 {
    let period = limits.target().period().unwrap_or(0.0);
    drs.residual().seconds(period) * 1000.0
}
