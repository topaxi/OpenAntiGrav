//! What the output callback saw, counted for a thread that can afford to say
//! it.
//!
//! Every field here is bumped from the audio callback, where formatting a
//! message would itself be a dropout, and read from the frame loop. The point
//! is to tell three failures apart that all sound the same from a chair:
//!
//! 1. **A dropped buffer** - the mixer lock was still held when the callback's
//!    waiting budget ran out. See [ADR-0031](../../../../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md).
//! 2. **A jump** - the callback rendered normally and the samples it produced
//!    still step discontinuously from the previous buffer's last frame. That is
//!    a click in *our own mix*, and it means a voice started or ended on a
//!    non-zero sample rather than that anything was late.
//! 3. **A late callback** - more wall-clock time passed between two calls than
//!    the buffer between them was worth. That is the device path or the
//!    scheduler, not this crate, and it is the one nothing here can fix.
//!
//! `starved` and `clipped` come from [`Mixer`](crate::Mixer) and are reported
//! alongside because a refused voice and a saturating mix are two more things
//! that sound like a fault and are neither of the three.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};

use log::warn;

/// Frames between two reports, at 60 Hz about two seconds - short enough that
/// a fault happening "every few seconds" lands in a window of its own.
const REPORT_EVERY: u32 = 120;

/// How far two adjacent samples may step before it counts as a click.
///
/// A quarter of full scale. A cue that starts or ends on a loud sample steps
/// by roughly its own amplitude, and nothing legitimate in a 44.1 kHz stream
/// moves that far in one frame - the highest frequency it can carry moves by
/// less than that between samples unless it is near full scale itself.
const JUMP: f32 = 0.25;

/// The largest step a window may hold without being reported at all.
///
/// Well under [`JUMP`], so a mix that is merely *nearly* clicking still shows
/// up. A busy race genuinely moves this far between samples on its own - the
/// engine bed's own median step is 0.055 of full scale and its 99.9th
/// percentile is 0.287, measured on `pulse-psp-eu.chd` - so this is a floor for
/// "worth a line", not a fault threshold.
const QUIET_STEP: f32 = 0.40;

/// Counters shared between the output callback and the frame loop.
#[derive(Debug, Default)]
pub struct Health {
    buffers: AtomicU64,
    /// Frames in the last buffer the device asked for - the period, which is
    /// what every deadline here is measured against.
    buffer_frames: AtomicU64,
    dropped: AtomicU64,
    jumps: AtomicU64,
    /// The largest step seen, in thousandths of full scale, and the frame
    /// inside its buffer that it happened at - which says whether the seam
    /// between two buffers is where the mix breaks or whether it is a voice
    /// ending somewhere in the middle of one.
    worst_jump: AtomicU64,
    worst_jump_at: AtomicU64,
    late: AtomicU64,
    worst_late_us: AtomicU64,

    since_report: AtomicU32,
    said_dropped: AtomicU64,
    said_jumps: AtomicU64,
    said_late: AtomicU64,
    said_starved: AtomicU64,
    said_clipped: AtomicU64,
}

impl Health {
    /// One buffer of `frames` went out.
    pub(crate) fn buffer(&self, frames: usize) {
        self.buffers.fetch_add(1, Relaxed);
        self.buffer_frames.store(frames as u64, Relaxed);
    }

    /// Frames in the last buffer the device asked for.
    ///
    /// Never requested: `Output::open` takes `default_output_config` verbatim,
    /// so this is whatever the device or the sound server chose. It is the
    /// number every other counter here is relative to - a late callback means
    /// late against this many frames' playing time.
    #[must_use]
    pub fn buffer_frames(&self) -> u64 {
        self.buffer_frames.load(Relaxed)
    }

    /// The mixer lock was still held when the budget ran out.
    pub(crate) fn dropped(&self) {
        self.dropped.fetch_add(1, Relaxed);
    }

    /// Scans a rendered buffer for steps, including the seam with the last one.
    ///
    /// **The whole buffer, not only its first frame.** This checked the seam
    /// alone until 2026-08-31 and reported zero through a race that was audibly
    /// thumping: a voice ends wherever its own playhead runs out, which with a
    /// 1,920-frame buffer is at the seam roughly one time in two thousand. A
    /// blind spot that size reads exactly like a clean mix.
    ///
    /// `tail` is the previous buffer's last frame. Called only for a buffer
    /// that was actually rendered and did not follow a drop: the ramp either
    /// side of a gap is a deliberate discontinuity and counting it would bury
    /// the accidental ones.
    pub(crate) fn scan(&self, stereo: &[f32], tail: [f32; 2]) {
        let mut previous = tail;
        let mut worst = 0.0f32;
        let mut found = 0u64;
        let mut at = 0usize;
        for (frame, pair) in stereo.as_chunks::<2>().0.iter().enumerate() {
            let step = (pair[0] - previous[0])
                .abs()
                .max((pair[1] - previous[1]).abs());
            previous = [pair[0], pair[1]];
            if step > JUMP {
                found += 1;
            }
            // Tracked whether or not it crossed the threshold, because the
            // useful reading when the count is zero is *how close* it came - a
            // report that only ever says "nothing over a quarter" cannot tell a
            // clean mix from one stepping by a fifth of full scale sixty times
            // a second.
            if step > worst {
                worst = step;
                at = frame;
            }
        }
        self.jumps.fetch_add(found, Relaxed);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a step is bounded by the mixer's own clamp at full scale"
        )]
        let milli = (worst * 1000.0) as u64;
        if self.worst_jump.fetch_max(milli, Relaxed) < milli {
            self.worst_jump_at.store(at as u64, Relaxed);
        }
    }

    /// More time passed since the previous callback than the buffer between
    /// them was worth, by `over` microseconds.
    pub(crate) fn late(&self, over_us: u64) {
        self.late.fetch_add(1, Relaxed);
        self.worst_late_us.fetch_max(over_us, Relaxed);
    }

    /// Buffers the callback gave up on rather than render.
    #[must_use]
    pub fn dropped_buffers(&self) -> u64 {
        self.dropped.load(Relaxed)
    }

    /// Buffers that stepped discontinuously from the one before.
    #[must_use]
    pub fn jumps(&self) -> u64 {
        self.jumps.load(Relaxed)
    }

    /// Callbacks that arrived later than the previous buffer's playing time.
    #[must_use]
    pub fn late_callbacks(&self) -> u64 {
        self.late.load(Relaxed)
    }

    /// Whether enough frames have passed to say anything again.
    ///
    /// Throttled in frames rather than by a clock because nothing in this crate
    /// reads one.
    pub(crate) fn due(&self) -> bool {
        if self.since_report.fetch_add(1, Relaxed) < REPORT_EVERY {
            return false;
        }
        self.since_report.store(0, Relaxed);
        true
    }

    /// Logs what changed since the last report, or nothing if nothing did.
    ///
    /// One line covering all five, because the useful reading is which of them
    /// moved *together*: late callbacks with no jumps is the device path, jumps
    /// with no late callbacks is our own mix, and drops with neither is the
    /// mixer lock.
    pub(crate) fn report(&self, starved: u64, clipped: u64) {
        let dropped = self.dropped.load(Relaxed);
        let jumps = self.jumps.load(Relaxed);
        let late = self.late.load(Relaxed);

        let d_dropped = dropped - self.said_dropped.swap(dropped, Relaxed);
        let d_jumps = jumps - self.said_jumps.swap(jumps, Relaxed);
        let d_late = late - self.said_late.swap(late, Relaxed);
        let d_starved = starved.saturating_sub(self.said_starved.swap(starved, Relaxed));
        let d_clipped = clipped.saturating_sub(self.said_clipped.swap(clipped, Relaxed));

        let worst_jump = self.worst_jump.swap(0, Relaxed) as f32 / 1000.0;
        // Reported on the worst step alone as well as on the counters, because
        // a mix that is stepping by a fifth of full scale is audible and counts
        // as none of the five.
        if d_dropped == 0
            && d_jumps == 0
            && d_late == 0
            && d_starved == 0
            && d_clipped == 0
            && worst_jump < QUIET_STEP
        {
            return;
        }

        let worst_late = self.worst_late_us.swap(0, Relaxed) as f32 / 1000.0;
        let jump_at = self.worst_jump_at.load(Relaxed);
        warn!(
            "audio: {d_dropped} dropped, {d_jumps} jump(s) in the mix \
             (worst {worst_jump:.3} at frame {jump_at}), \
             {d_late} late callback(s) (worst {worst_late:.1} ms over), \
             {d_starved} voice(s) refused, {d_clipped} sample(s) clipped \
             - of {} buffer(s) of {} frame(s) so far",
            self.buffers.load(Relaxed),
            self.buffer_frames.load(Relaxed)
        );
    }
}
