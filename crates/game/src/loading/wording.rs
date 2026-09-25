//! What the loading screen *says* about a wait: the heading, the counts, the
//! percentage, the bar's fraction and the step line.
//!
//! Split out of `loading.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It is a real
//! seam rather than a cut at a line number: everything here is a pure function
//! of `(Phase, &Progress)` returning a string or a number, with no state and no
//! atlas, and `Screen::draw_list` is the only caller. The layout - where those
//! strings go and what colour they are - stays in the parent.

use oag_ui::language::StringTable;

use super::{Phase, Progress};

/// Looks `id` up in `table`, substituting any `{name}` placeholders in
/// `subs` into whichever string was found - the table's own translation if
/// it has one, `literal` otherwise.
///
/// The fallback is exactly `resolve()`'s in `oag_ui::menu::definition`: a row
/// with no override keeps the text this project invented, untouched. The
/// placeholder substitution is the one thing every string here needed that
/// a menu row never did - every string composed on this screen carries at
/// least one dynamic number - so it is `String::replace` after the lookup
/// rather than `format!`, which cannot take a runtime template. A
/// translation is free to move `{done}`/`{total}`/`{verb}` anywhere in its
/// own text; only the literal names have to match.
fn resolved(table: &StringTable, id: &str, literal: &str, subs: &[(&str, &str)]) -> String {
    let mut out = table.get(id).unwrap_or(literal).to_string();
    for (name, value) in subs {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// What the current load is doing, under the entry name it is doing it to.
///
/// `None` draws nothing: a phase with no step to report has no line, rather than
/// a line saying it has nothing to report.
pub(super) fn step_line(phase: Phase, strings: &StringTable) -> Option<String> {
    let Phase::Media(step) = phase else {
        return None;
    };
    Some(match step? {
        // Named rather than silent. A cache hit is over in milliseconds, so this
        // is rarely *read* - but it is what makes the transcoding line below
        // mean something specific when it does appear.
        crate::movie::Step::Cached => resolved(
            strings,
            "OAG_LOADING_STEP_CACHED",
            "already converted, reading the cache",
            &[],
        ),
        crate::movie::Step::Decoding => {
            resolved(strings, "OAG_LOADING_STEP_DECODING", "decoding", &[])
        }
        // The counter is the anti-hang signal and the reason any of this is
        // plumbed: a number that moves once a second is the difference between
        // eighty seconds of progress and eighty seconds of nothing.
        crate::movie::Step::Transcoding {
            done,
            total: Some(total),
        } => resolved(
            strings,
            "OAG_LOADING_STEP_TRANSCODING_TOTAL",
            "transcoding - frame {done} of {total}",
            &[("done", &done.to_string()), ("total", &total.to_string())],
        ),
        crate::movie::Step::Transcoding { done, total: None } => resolved(
            strings,
            "OAG_LOADING_STEP_TRANSCODING",
            "transcoding - frame {done}",
            &[("done", &done.to_string())],
        ),
    })
}

/// What the screen calls what it is doing.
pub(super) fn heading(phase: Phase, progress: &Progress, strings: &StringTable) -> String {
    if progress.planning {
        return resolved(
            strings,
            "OAG_LOADING_READING_ARCHIVES",
            "READING THE ARCHIVES",
            &[],
        );
    }
    if !counted(progress) {
        // Nothing to count: a boot whose title names no movie at all, which is
        // the one case the media phase has no work in. See [`counted`].
        return resolved(strings, "OAG_LOADING_GENERIC", "LOADING", &[]);
    }
    if progress.finished {
        return resolved(strings, "OAG_LOADING_READY", "READY", &[]);
    }
    match phase {
        // The one wait worth its own word. A transcode of the intro is about
        // eighty seconds and a cache hit is milliseconds, and heading both
        // "loading" is how a player learns to read the word as "a moment" and
        // then sits through a minute and a half of it.
        Phase::Media(Some(crate::movie::Step::Transcoding { .. })) => resolved(
            strings,
            "OAG_LOADING_TRANSCODING_MOVIES",
            "TRANSCODING MOVIES",
            &[],
        ),
        // Deliberately not "converting" for the rest: a warm cache reads the
        // cached file and a `native-video` build decodes it, and neither is a
        // conversion. "Loading" is true of both.
        Phase::Media(_) => resolved(strings, "OAG_LOADING_LOADING_MOVIES", "LOADING MOVIES", &[]),
        Phase::Prefetch => resolved(
            strings,
            "OAG_LOADING_CONVERTING_ASSETS",
            "CONVERTING ASSETS",
            &[],
        ),
        // Unreachable in practice - a race load reports no total, so `counted`
        // has already returned "LOADING" above - and answered rather than
        // panicked, because a caller that did hand this a counted race would
        // get a true heading instead of a crash.
        Phase::Race => resolved(strings, "OAG_LOADING_GENERIC", "LOADING", &[]),
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
pub(super) fn counts(phase: Phase, progress: &Progress, strings: &StringTable) -> String {
    if progress.planning {
        return resolved(
            strings,
            "OAG_LOADING_COUNTING",
            "counting what needs converting",
            &[],
        );
    }
    // `cached` and `failed` below are always zero in the media phase - nothing
    // there is skipped for being cached, and a reel that will not load is a
    // report line rather than a failure the screen counts - so the two branches
    // differ only in the verb. The verb is baked into its own template rather
    // than substituted into a shared one: a translation may need a different
    // word order for "loaded" than for "converted", not just a different word.
    let (id, literal) = match phase {
        Phase::Media(_) => ("OAG_LOADING_COUNTS_LOADED", "{done} / {total} loaded"),
        Phase::Prefetch => ("OAG_LOADING_COUNTS_CONVERTED", "{done} / {total} converted"),
        // See the heading above: a race load counts nothing, so this is the
        // honest verb for a caller that counted one anyway.
        Phase::Race => ("OAG_LOADING_COUNTS_LOADED", "{done} / {total} loaded"),
    };
    let subs = [
        ("done", progress.done.to_string()),
        ("total", progress.total.to_string()),
    ];
    let subs: Vec<(&str, &str)> = subs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut out = resolved(strings, id, literal, &subs);
    if progress.cached > 0 {
        out.push_str(&resolved(
            strings,
            "OAG_LOADING_COUNTS_CACHED_SUFFIX",
            ", {cached} already cached",
            &[("cached", &progress.cached.to_string())],
        ));
    }
    if progress.failed > 0 {
        out.push_str(&resolved(
            strings,
            "OAG_LOADING_COUNTS_FAILED_SUFFIX",
            ", {failed} failed",
            &[("failed", &progress.failed.to_string())],
        ));
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

/// How long [`estimated_fraction`] takes to reach two thirds of the bar: four
/// seconds at 60 Hz.
///
/// **Chosen, not measured.** A race load counts nothing - `race::load` and the
/// scene build behind it report no progress - so this is a guess at the wait,
/// not a reading of it. Four seconds puts the bar at 86 % after the eight a
/// release build takes to build a Pulse or HD scene on a desktop (see
/// `docs/architecture/race-load-transition.md`), and still moving after the
/// longer wait a Steam Deck will have.
pub(super) const ESTIMATE_TICKS: f32 = 240.0;

/// A time-based fill for a race load's bar: `1 - e^(-ticks / ESTIMATE_TICKS)`,
/// which always moves and never arrives, and the whole bar once the wait is
/// over.
///
/// **A stand-in, asked for.** The original fills this bar from its own
/// loader, which this build does not reproduce; an empty trough read as a
/// stuck screen. The curve slows rather than stops so a long load never looks
/// finished early, and `finished` is the one real fact it carries.
pub(super) fn estimated_fraction(ticks: u32, finished: bool) -> f32 {
    if finished {
        return 1.0;
    }
    1.0 - (-(ticks as f32) / ESTIMATE_TICKS).exp()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn counted_progress() -> Progress {
        Progress {
            total: 1,
            finished: true,
            ..Progress::default()
        }
    }

    #[test]
    fn an_id_with_an_override_resolves_to_it() {
        let mut table = StringTable::default();
        table.merge(HashMap::from([(
            "OAG_LOADING_READY".to_string(),
            "PRET".to_string(),
        )]));
        assert_eq!(
            heading(Phase::default(), &counted_progress(), &table),
            "PRET"
        );
    }

    #[test]
    fn an_id_with_no_override_falls_back_to_the_current_literal() {
        let table = StringTable::default();
        assert_eq!(
            heading(Phase::default(), &counted_progress(), &table),
            "READY"
        );
    }

    /// A translation is free to move `{done}`/`{total}` anywhere in its own
    /// text - proved by putting them in the opposite order from the English
    /// literal.
    #[test]
    fn a_translation_may_reorder_a_templates_placeholders() {
        let mut table = StringTable::default();
        table.merge(HashMap::from([(
            "OAG_LOADING_STEP_TRANSCODING_TOTAL".to_string(),
            "{total} sur {done}".to_string(),
        )]));
        let step = step_line(
            Phase::Media(Some(crate::movie::Step::Transcoding {
                done: 3,
                total: Some(10),
            })),
            &table,
        );
        assert_eq!(step.as_deref(), Some("10 sur 3"));
    }

    #[test]
    fn a_templated_line_falls_back_to_the_literal_template_with_its_numbers_filled_in() {
        let table = StringTable::default();
        let step = step_line(
            Phase::Media(Some(crate::movie::Step::Transcoding {
                done: 3,
                total: Some(10),
            })),
            &table,
        );
        assert_eq!(step.as_deref(), Some("transcoding - frame 3 of 10"));
    }

    /// The cached/failed suffixes override independently of the main
    /// `done / total <verb>` template and of each other.
    #[test]
    fn counts_suffixes_override_independently_of_the_main_line() {
        let mut table = StringTable::default();
        table.merge(HashMap::from([(
            "OAG_LOADING_COUNTS_CACHED_SUFFIX".to_string(),
            ", {cached} deja en cache".to_string(),
        )]));
        let progress = Progress {
            done: 4,
            total: 10,
            cached: 2,
            ..Progress::default()
        };
        assert_eq!(
            counts(Phase::Prefetch, &progress, &table),
            "4 / 10 converted, 2 deja en cache"
        );
    }

    #[test]
    fn counts_falls_back_to_the_literal_when_nothing_overrides_it() {
        let table = StringTable::default();
        let progress = Progress {
            done: 4,
            total: 10,
            failed: 1,
            ..Progress::default()
        };
        assert_eq!(
            counts(Phase::Media(None), &progress, &table),
            "4 / 10 loaded, 1 failed"
        );
    }

    /// One [`StringTable`] answers every call it is handed, rather than being
    /// rebuilt per line - the same table a caller like `Screen::new` builds
    /// once and holds. A stray "build a fresh table per call" regression
    /// would still pass the tests above, since a fresh empty table falls back
    /// to the same literals; what this pins is that a *populated* table's
    /// answer is stable across repeated calls against the one instance, which
    /// is the only thing "shared" is observable proof of here.
    #[test]
    fn one_table_answers_every_call_it_is_passed_to() {
        let mut table = StringTable::default();
        table.merge(HashMap::from([(
            "OAG_LOADING_READY".to_string(),
            "PRET".to_string(),
        )]));
        for _ in 0..3 {
            assert_eq!(
                heading(Phase::default(), &counted_progress(), &table),
                "PRET"
            );
        }
    }
}
