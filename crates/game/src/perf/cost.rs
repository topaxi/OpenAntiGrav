//! What one frame cost, from the two clocks that can be asked about it.
//!
//! [`GpuCost`] is **device time**: timestamps written on the GPU around the
//! passes this build brackets, from [`oag_render::timing::PassTimer`].
//! [`CpuCost`] is **wall-clock time on the frame thread**: how long
//! `Session::frame` itself ran, and how much of that went into the two
//! swapchain calls that are allowed to block.
//!
//! # They are two axes, and only one of them adds up
//!
//! The CPU records a frame while the GPU is still finishing the one before
//! it, so a device reading and a wall-clock reading of the same frame overlap
//! by an amount nothing here can measure. Adding a GPU row to a CPU row would
//! be adding two spans that ran at the same time.
//!
//! What *does* add up is [`CpuCost`]'s own three rows. `CPU`, `PRESENT` and
//! `SLEEP` partition the interval between one redraw and the next - the very
//! interval `perf::Meter` reports as the frame time - so they sum to it by
//! construction and each one names a different thing to fix:
//!
//! - **`CPU`** is work on this thread: the ticks, the stage update, building
//!   the frame's draw lists, encoding, and mapping last frame's timestamp
//!   readback.
//! - **`PRESENT`** is `Surface::get_current_texture` plus `Queue::present`.
//!   Under [`super::Vsync::On`] the wait for the refresh lands here, which is
//!   what makes a blocked loop tell itself apart from a busy one.
//! - **`SLEEP`** is everything outside `frame()`: the frame limiter's
//!   `ControlFlow::WaitUntil` above all, which is idle time and not cost at
//!   all.
//!
//! # Why `OTHER` is still its own row
//!
//! [`GpuCost::residual_ms`] - `frame - (the timed passes)` - overlaps all
//! three of the rows above and is not replaced by them. It is the reading
//! [`crate::drs::Residual`] learns from, so the overlay showing exactly what
//! the controller is fed is the point of it, and the three rows below it are
//! what say **which** of sleep, CPU work and untimed GPU passes a large one is
//! made of. See `crates/game/src/drs/residual.rs`, which spells out the same
//! contamination from the controller's side.

/// What the GPU spent on the passes this build actually times, in seconds.
///
/// **The frame-time panel cannot answer this on its own.** `Meter` measures
/// the interval between loop iterations, which under `Vsync::On` is the
/// refresh and under a frame limit is the limit - so a frame with headroom
/// and a frame with none read the same *there*. These are GPU timestamps
/// around specific work, from [`oag_render::timing::PassTimer`]. Read
/// together - see [`GpuCost::rows`] and the `OTHER` row it appends - the two
/// say what the frame-time panel alone cannot: how much of a frame is
/// accounted for and how much is not.
///
/// Four numbers rather than one because they answer different questions and a
/// render scale moves them in different directions: lowering it makes the
/// scene pass, the motion-blur chain and the HD bloom chain cheaper and gives
/// the temporal resolve *more* to reconstruct, and the upscale chain runs
/// after the other three rather than inside them. Choosing a render scale
/// without all four is choosing on part of the cost.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuCost {
    /// The race's scene pass, or `None` before its first reading has come back.
    pub scene: Option<f32>,
    /// The motion-blur chain, or `None` before its first reading has come
    /// back. That includes every frame `[render_profiles.<title>]
    /// motion_blur` is `off`, since `oag_render::post::motion_blur::MotionBlur::render`
    /// then encodes nothing and the claimed slot is given back unwritten. See
    /// [ADR-0042](../../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md).
    pub blur: Option<f32>,
    /// Wipeout HD/Fury's read bloom chain, or `None` before its first reading
    /// has come back. `None` forever on Pulse, Pure, or an HD circuit with no
    /// `HDR and Bloom` block - see `race::Scene::has_hd_bloom` and
    /// [ADR-0043](../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md).
    pub bloom: Option<f32>,
    /// FSR 3.1's six render-resolution dispatches, or `None` when no temporal
    /// upscaler has run - which is every frame on every other rung of the
    /// ladder.
    ///
    /// **Six and not eight since
    /// [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)**:
    /// the chain is timed in two halves, and this is the one that falls when
    /// the render extent does.
    pub upscale: Option<f32>,
    /// FSR 3.1's `accumulate` and `rcas`, at presentation resolution, on the
    /// same terms as [`GpuCost::upscale`].
    ///
    /// The half of the chain a lower render scale does not make cheaper, which
    /// is why it is the only part in [`crate::drs::Cost::fixed`].
    pub upscale_presented: Option<f32>,
}

impl GpuCost {
    /// One row per reading that has come back, each labelled and in
    /// pipeline order, followed by `OTHER` - what [`Self::residual_ms`]
    /// found. Empty when nothing has been measured yet.
    ///
    /// **One row per number rather than one crammed line**, since the panel
    /// this feeds is a dedicated one - see `draw_list`'s own top-left panel,
    /// separate from the frame-time panel at top-right precisely so a reader
    /// is not holding five numbers in one line. A reading arrives a frame or
    /// more after the frame it describes, so the first few frames of a run
    /// legitimately have nothing to show for a field - a row for it would
    /// read as "free" rather than as "not measured yet", which is why each
    /// row is conditional on its own field rather than the whole panel being
    /// all-or-nothing.
    #[must_use]
    pub fn rows(self, frame_ms: f32) -> Vec<String> {
        let mut rows = Vec::new();
        if let Some(scene) = self.scene {
            rows.push(format!("SCENE {:.2} MS", scene * 1000.0));
        }
        if let Some(bloom) = self.bloom {
            rows.push(format!("BLOOM {:.2} MS", bloom * 1000.0));
        }
        if let Some(blur) = self.blur {
            rows.push(format!("BLUR {:.2} MS", blur * 1000.0));
        }
        if let Some(upscale) = self.upscale {
            rows.push(format!("FSR3 REN {:.2} MS", upscale * 1000.0));
        }
        if let Some(upscale) = self.upscale_presented {
            rows.push(format!("FSR3 OUT {:.2} MS", upscale * 1000.0));
        }
        if let Some(residual) = self.residual_ms(frame_ms) {
            rows.push(format!("OTHER {residual:.1} MS"));
        }
        rows
    }

    /// What `frame_ms` does not account for, or `None` when nothing has been
    /// measured yet.
    ///
    /// **The whole point of carrying `frame_ms` in rather than reading a
    /// target period.** A target is what a player asked for; `frame_ms` -
    /// `Stats::mean_ms`, the same wall-clock reading the panel's own top row
    /// already carries - is what the machine is actually doing, timed passes
    /// and untimed ones both. The difference is real cost sitting outside
    /// every `PassTimer` this build has - the shadow map, the PSP bloom, FXAA,
    /// SMAA, FSR 1, the HUD, the menus, the composite, this overlay itself,
    /// the driver's own overhead and the frame loop's own CPU-side work - plus
    /// **any time the loop spent asleep**, which is not cost at all. See
    /// [`CpuCost`], whose three rows say which of those a large residual is.
    ///
    /// The MSAA resolve is deliberately *not* in that list, though this doc
    /// used to say it was: it is a store-op resolve on the scene pass's own
    /// colour attachment (`race::scene::frame`), so it runs inside the pass
    /// the `SCENE` timestamps bracket and is already counted there.
    ///
    /// Can be negative in principle - `frame_ms` is a rolling mean and the GPU
    /// readings are a frame or more old, so a moment where the mean has fallen
    /// faster than the GPU cost has is not impossible - and is passed through
    /// rather than clamped, because a small negative number says "these two
    /// signals are close and slightly out of phase" where zero would claim
    /// nothing is missing.
    #[must_use]
    pub fn residual_ms(self, frame_ms: f32) -> Option<f32> {
        if self.scene.is_none()
            && self.blur.is_none()
            && self.bloom.is_none()
            && self.upscale.is_none()
            && self.upscale_presented.is_none()
        {
            return None;
        }
        let timed_ms = (self.scene.unwrap_or(0.0)
            + self.blur.unwrap_or(0.0)
            + self.bloom.unwrap_or(0.0)
            + self.upscale.unwrap_or(0.0)
            + self.upscale_presented.unwrap_or(0.0))
            * 1000.0;
        Some(frame_ms - timed_ms)
    }
}

/// Where a frame's **wall clock** went, in seconds, on the frame thread.
///
/// The half of the panel that needs no device feature at all: two
/// `Instant`s, so an adapter with no timestamp support - which has no
/// [`GpuCost`] rows whatsoever - still gets a breakdown of its frame.
///
/// Both fields are means over `perf::Meter` windows of their own, fed one
/// reading a frame by the composition root, and both are `None` until a frame
/// has been measured. A frame that carried a load is dropped from them for
/// the same reason it is dropped from the frame-time meter: a track build is
/// not a frame time.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CpuCost {
    /// How long `Session::frame` itself took, start to end - the ticks, the
    /// stage update, the draw, and the two swapchain calls [`Self::present`]
    /// measures separately.
    ///
    /// **Not "how busy the CPU was".** It is elapsed time on the frame
    /// thread, so a blocking wait inside the frame is in it, which is exactly
    /// why `present` is subtracted back out for the `CPU` row.
    pub frame: Option<f32>,
    /// `Surface::get_current_texture` plus `Queue::present`, the two calls in
    /// a frame that are allowed to block on the display.
    ///
    /// Its own number because a loop waiting for a refresh and a loop doing
    /// too much work look identical in [`Self::frame`] alone, and they want
    /// opposite fixes.
    pub present: Option<f32>,
}

impl CpuCost {
    /// `CPU`, `PRESENT` and `SLEEP`, which partition `frame_ms` between them.
    ///
    /// Empty until [`Self::frame`] has a reading: a `SLEEP` row derived from
    /// no measurement would read as the whole frame being idle.
    #[must_use]
    pub fn rows(self, frame_ms: f32) -> Vec<String> {
        let Some(frame) = self.frame else {
            return Vec::new();
        };
        let frame_body_ms = frame * 1000.0;
        let present_ms = self.present.map(|present| present * 1000.0);
        let mut rows = vec![format!(
            "CPU {:.2} MS",
            frame_body_ms - present_ms.unwrap_or(0.0)
        )];
        if let Some(present_ms) = present_ms {
            rows.push(format!("PRESENT {present_ms:.2} MS"));
        }
        rows.push(format!("SLEEP {:.1} MS", frame_ms - frame_body_ms));
        rows
    }

    /// What the loop spent outside `Session::frame` - the frame limiter's own
    /// wait, and the event loop around it - or `None` before a frame has been
    /// measured.
    ///
    /// Passed through unclamped for the same reason
    /// [`GpuCost::residual_ms`] is: the two means are one frame out of phase,
    /// so a small negative reading is the two signals disagreeing by a frame
    /// rather than a loop that slept for less than no time.
    #[must_use]
    pub fn sleep_ms(self, frame_ms: f32) -> Option<f32> {
        self.frame.map(|frame| frame_ms - frame * 1000.0)
    }
}

#[cfg(test)]
mod tests;
