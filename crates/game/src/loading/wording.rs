//! What the loading screen *says* about a wait: the heading, the counts, the
//! percentage, the bar's fraction and the step line.
//!
//! Split out of `loading.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It is a real
//! seam rather than a cut at a line number: everything here is a pure function
//! of `(Phase, &Progress)` returning a string or a number, with no state and no
//! atlas, and `Screen::draw_list` is the only caller. The layout - where those
//! strings go and what colour they are - stays in the parent.

use super::{Phase, Progress};

/// What the current load is doing, under the entry name it is doing it to.
///
/// `None` draws nothing: a phase with no step to report has no line, rather than
/// a line saying it has nothing to report.
pub(super) fn step_line(phase: Phase) -> Option<String> {
    let Phase::Media(step) = phase else {
        return None;
    };
    Some(match step? {
        // Named rather than silent. A cache hit is over in milliseconds, so this
        // is rarely *read* - but it is what makes the transcoding line below
        // mean something specific when it does appear.
        crate::movie::Step::Cached => "already converted, reading the cache".to_string(),
        crate::movie::Step::Decoding => "decoding".to_string(),
        // The counter is the anti-hang signal and the reason any of this is
        // plumbed: a number that moves once a second is the difference between
        // eighty seconds of progress and eighty seconds of nothing.
        crate::movie::Step::Transcoding {
            done,
            total: Some(total),
        } => format!("transcoding - frame {done} of {total}"),
        crate::movie::Step::Transcoding { done, total: None } => {
            format!("transcoding - frame {done}")
        }
    })
}

/// What the screen calls what it is doing.
pub(super) fn heading(phase: Phase, progress: &Progress) -> String {
    if progress.planning {
        return "READING THE ARCHIVES".to_string();
    }
    if !counted(progress) {
        // Nothing to count: a boot whose title names no movie at all, which is
        // the one case the media phase has no work in. See [`counted`].
        return "LOADING".to_string();
    }
    if progress.finished {
        return "READY".to_string();
    }
    match phase {
        // The one wait worth its own word. A transcode of the intro is about
        // eighty seconds and a cache hit is milliseconds, and heading both
        // "loading" is how a player learns to read the word as "a moment" and
        // then sits through a minute and a half of it.
        Phase::Media(Some(crate::movie::Step::Transcoding { .. })) => {
            "TRANSCODING MOVIES".to_string()
        }
        // Deliberately not "converting" for the rest: a warm cache reads the
        // cached file and a `native-video` build decodes it, and neither is a
        // conversion. "Loading" is true of both.
        Phase::Media(_) => "LOADING MOVIES".to_string(),
        Phase::Prefetch => "CONVERTING ASSETS".to_string(),
        // Unreachable in practice - a race load reports no total, so `counted`
        // has already returned "LOADING" above - and answered rather than
        // panicked, because a caller that did hand this a counted race would
        // get a true heading instead of a crash.
        Phase::Race => "LOADING".to_string(),
    }
}

/// Whether there is a conversion being counted, and so whether the bar, the
/// done/total pair and the percentage have anything to say.
///
/// **`false` is now the rare case, and it is the honest one.** Both of the
/// screen's waits count what they are doing - the media phase from
/// [`crate::boot::MediaPlan::loads`], the prefetch worker from its own planning
/// pass - so a total of zero means there is genuinely nothing to count: a title
/// whose chain names no movie, on a run with no `--prefetch`. Drawing the row
/// anyway gave `0 / 0` beside a full bar reading `100%`, which describes nothing
/// and looks like a bug in the counter rather than an absence of one.
///
/// The three are one row and go together. An earlier build hid all three on
/// every ordinary boot, because the media phase reported no counts at all and so
/// looked like that empty case; the bar is back because the counts behind it are
/// real, not because the gate was loosened.
///
/// `planning` counts as counted: a worker that has not finished its walk has a
/// total of zero and is certainly converting something.
pub(super) fn counted(progress: &Progress) -> bool {
    progress.planning || progress.total > 0
}

/// How much of one load's own slice the read-or-decode part takes when a
/// transcode follows it.
///
/// A fifth, and the number is a guess about *proportions* rather than a
/// measurement - it cannot be measured, because the two are not the same work.
/// What it has to get right is the ordering: opening the container and demuxing
/// it is a small fraction of what `ffmpeg` then spends on the pictures (2.6 s of
/// intro against 55 s of transcode, measured on the PS2's `INTRO512.PSS`), so
/// the head has to be small enough that the frame counter drives nearly all of
/// the slice.
const LOADING_SHARE: f32 = 0.2;

/// How full the bar is, from `0.0` to `1.0`.
///
/// **Each load owns one slice of the bar and fills its own slice**, rather than
/// the bar stepping once per load. With five loads and a transcode in the third,
/// the bar does not sit at 40% for a minute: it crosses from 40% to 60% as the
/// frame counter runs, which is the difference between a bar that is watched and
/// a bar that is assumed broken.
///
/// The step's share of its own slice:
///
/// | Step | Share | Why |
/// | --- | --- | --- |
/// | nothing reported | `0.0` | the slice has not started |
/// | [`Cached`](crate::movie::Step::Cached) | `1.0` | the hit *is* the whole of that load's work |
/// | [`Decoding`](crate::movie::Step::Decoding) | [`LOADING_SHARE`] | held, because a decode reports no progress and may still fall back to a transcode |
/// | [`Transcoding`](crate::movie::Step::Transcoding) with a total | `LOADING_SHARE` upward | the frame counter drives the rest |
/// | `Transcoding` with no total | [`LOADING_SHARE`] | held: no denominator, so no fraction to invent |
///
/// **Monotonic by construction, which is the property that matters.** `Cached`
/// takes exactly the whole slice, so the bar is already where `done + 1` will
/// put it a millisecond later; `Decoding` holds at the head that `Transcoding`
/// then counts up from. A bar that went backwards would be worse than one that
/// only stepped.
///
/// The sound loads report no step and so hold their slices at the boundary -
/// [`crate::at3`] has its own cache and does not report through
/// [`crate::movie::Watch`]. They are normally the fast ones; on a cold audio
/// cache they are two slices where the bar stands still, which is a real gap and
/// not a hidden one.
pub(super) fn fraction(phase: Phase, progress: &Progress) -> f32 {
    let base = progress.fraction().clamp(0.0, 1.0);
    if progress.total == 0 {
        return base;
    }
    let Phase::Media(Some(step)) = phase else {
        return base;
    };
    let within = match step {
        crate::movie::Step::Cached => 1.0,
        crate::movie::Step::Decoding | crate::movie::Step::Transcoding { total: None, .. } => {
            LOADING_SHARE
        }
        // A total of zero is not a conversion of nothing, it is a total nobody
        // could state - treated as the no-total case rather than divided by.
        crate::movie::Step::Transcoding { total: Some(0), .. } => LOADING_SHARE,
        crate::movie::Step::Transcoding {
            done,
            total: Some(total),
        } => {
            let encoded = (done as f32 / total as f32).clamp(0.0, 1.0);
            LOADING_SHARE + (1.0 - LOADING_SHARE) * encoded
        }
    };
    (base + within / progress.total as f32).clamp(0.0, 1.0)
}

/// The done/total pair, and the two counts that only matter when they are not
/// zero.
///
/// `cached` is worth showing because it is most of the number on any run after
/// the first, and a screen reporting `0 of 4` on a disc with 115 assets looks
/// broken without it. `failed` is worth showing for the opposite reason: it is
/// normally zero, and a run where it is not should say so while it is still on
/// screen rather than only in the summary line at the end.
pub(super) fn counts(phase: Phase, progress: &Progress) -> String {
    if progress.planning {
        return "counting what needs converting".to_string();
    }
    // `cached` and `failed` below are always zero in the media phase - nothing
    // there is skipped for being cached, and a reel that will not load is a
    // report line rather than a failure the screen counts - so the two branches
    // differ only in the verb.
    let verb = match phase {
        Phase::Media(_) => "loaded",
        Phase::Prefetch => "converted",
        // See the heading above: a race load counts nothing, so this is the
        // honest verb for a caller that counted one anyway.
        Phase::Race => "loaded",
    };
    let mut out = format!("{} / {} {verb}", progress.done, progress.total);
    if progress.cached > 0 {
        out.push_str(&format!(", {} already cached", progress.cached));
    }
    if progress.failed > 0 {
        out.push_str(&format!(", {} failed", progress.failed));
    }
    out
}

/// The bar's own fill as a whole percentage.
///
/// **The same [`fraction`] the bar is drawn from, deliberately.** They sit on one
/// row saying the same thing, and a figure that disagreed with the width beside
/// it would make both look wrong - which is exactly what reading the plain
/// `done / total` here would do now that the bar sub-fills its current slice.
///
/// Blank while planning: [`Progress::fraction`] is `1.0` before anything has
/// been counted, which is the honest answer to "how much of nothing is left"
/// and reads as `100%` on a screen that has only just opened.
pub(super) fn percentage(phase: Phase, progress: &Progress) -> String {
    if progress.planning {
        return String::new();
    }
    format!("{}%", (fraction(phase, progress) * 100.0).round())
}
