//! The loading screen shown while the boot's movies decode and `--prefetch`
//! converts the disc.
//!
//! [`crate::prefetch`] costs about ten minutes on a cold cache and the boot's
//! own reels several seconds on every start, and a window that draws nothing
//! for either is indistinguishable from a hang. This is the screen that says
//! otherwise: the original's own procedural wave from [`oag_render::loading`],
//! one of the disc's 30 loading tips, and - the part that actually does the
//! job - the counts.
//!
//! Both waits are counted and [`Phase`] says which one is being counted; they
//! never overlap, so the screen shows one set of figures throughout.
//!
//! **The numbers are the point and the wave is the decoration.** A pretty
//! animation with no figures still reads as a hang after the first minute, so
//! [`Screen::draw_list`] draws the fraction, the done/total pair and the asset
//! currently in `ffmpeg` before it draws anything else. Everything else here is
//! in service of that.
//!
//! # What is recovered and what is not
//!
//! The wave, its heartbeat and its glow strip are recovered - see
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md` and the module
//! documentation on [`oag_render::loading`]. The tips are the disc's own, read
//! out of `Data\Plugins\loading\Definition.xml` and resolved through the chosen
//! language's string table.
//!
//! **The layout is ours.** The original's `<PI_LoadingScreen>` entries carry a
//! title, a body string and a weapon render (`ImageSrc`), positioned by the
//! screen's own widgets; nothing on this screen is loading a weapon, so a
//! progress bar and a pair of counts sit where that artwork would. The per-tip
//! `ImageSrc` is deliberately not drawn: it belongs to the original's screen and
//! would be decoration on ours.

use oag_formats::fexml;
use oag_render::loading::{Quad, Wave};

use crate::font::{self, Atlas};
use crate::frontend::{Align, Draw, SCREEN};
use crate::language::StringTable;
use crate::prefetch::Progress;

/// The two entries this screen reads: `oag_pulse::loading`.
///
/// Both are `Data.wad` names off Pulse's disc, so they moved to the title package
/// under [ADR-0022]. The frame counts below did not: they are this build's own
/// pacing choices, not the original's.
///
/// [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md
pub use oag_pulse::loading::{GLOW_STRIP_ENTRY, TIPS_ENTRY};

/// Which of the screen's two waits the counts belong to.
///
/// The screen is up for both and they never overlap - `--prefetch` does not
/// start until the boot's own movies are done with the cache - so this says
/// which one is being counted rather than describing a mixture. It exists
/// because the two are honestly different work: one decodes the three reels
/// this boot needs, the other transcodes every convertible asset on the disc,
/// and a heading that called both by one word would be wrong about one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The boot's own movies, on every windowed start. See
    /// [`crate::boot::MediaWorker`].
    ///
    /// Carries what the current load is doing, because that is the difference
    /// between a wait nobody notices and eighty seconds of one - see
    /// [`crate::movie::Step`]. `None` before a load has said anything, and for
    /// the sound loads, which have their own cache and do not report.
    Media(Option<crate::movie::Step>),
    /// `--prefetch` converting the disc. See [`crate::prefetch`].
    ///
    /// No step: that worker converts by the hundred and names each asset as it
    /// starts it, so "which of them is transcoding" is answered by the count
    /// already on screen.
    Prefetch,
}

impl Default for Phase {
    /// The media phase, having said nothing yet: what every windowed boot opens
    /// on, before the first load has reported.
    fn default() -> Self {
        Self::Media(None)
    }
}

/// Frames one tip stays up, at the 60 Hz the window steps at.
///
/// Five seconds. The tips are two or three lines of prose and rotating them
/// faster than they can be read would be worse than not rotating them.
pub const TIP_FRAMES: u32 = 300;

/// Frames the finished screen stays up before the front end takes the window.
///
/// The wave is frozen for all of them - `Wave::finish` is the original's
/// `g_loading_finished`, which gates the motion while the screen fades - and
/// the tint fades to black across them. A quarter of a second: long enough that
/// the hand-off reads as a transition rather than as a frame drop, short enough
/// that nobody waits for it.
pub const FADE_FRAMES: u32 = 15;

/// Where the wave's [`Rng`](oag_core::rng::Rng) starts.
///
/// Any seed draws the same *character* of wave - the walk is re-run from
/// scratch every frame and only the impulses come from here - so this is a
/// constant rather than a clock read, which keeps a capture of this screen
/// reproducible. Same value the render crate's own tests use.
pub const WAVE_SEED: u64 = 0x10ad;

/// Everything the screen needs off the disc.
///
/// Read only when `--prefetch` asks for the screen: a default boot never opens
/// either entry, which is what keeps its half-second where it was.
#[derive(Debug)]
pub struct Assets {
    /// The disc's loading tips, in the chosen language and in XML order.
    ///
    /// Empty is an ordinary outcome - a source whose plugin will not read, or a
    /// string table that resolves none of the ids - and the screen simply draws
    /// no tip line.
    pub tips: Vec<String>,
    /// The wave's glow strip.
    pub strip: oag_render::loading::GlowStrip,
    /// Lines worth printing once, describing what was found or what was not.
    pub notes: Vec<String>,
}

impl Default for Assets {
    /// No tips and the authored stand-in strip.
    ///
    /// The same thing [`Assets::load`] falls back to when the disc's own strip
    /// will not decode, so this is a drawable screen rather than an empty
    /// husk - which matters because `--race` carries one it never draws, and a
    /// zero-sized texture would be a validation error the moment anything did.
    fn default() -> Self {
        Self {
            tips: Vec::new(),
            strip: oag_render::loading::GlowStrip::placeholder(
                oag_render::loading::STRIP_SIZE as u32,
            ),
            notes: Vec::new(),
        }
    }
}

impl Assets {
    /// Reads both entries out of `source`, degrading rather than failing.
    ///
    /// Neither is worth refusing to boot over: without the strip the wave has
    /// nothing to sample and without the tips there is no tip line, and in both
    /// cases the counts - the thing the screen exists for - are unaffected. So
    /// every failure is a note, and the caller prints it.
    ///
    /// **The strip falls back to [`oag_render::loading::GlowStrip::placeholder`]
    /// and says so.** That function's own documentation refuses to be a silent
    /// substitute, and the note is what makes it not one.
    #[must_use]
    pub fn load(source: &str, strings: &StringTable) -> Self {
        let mut notes = Vec::new();
        let mut tips = Vec::new();
        let mut strip = None;

        match oag_pulse::open(source) {
            Ok(mut archives) => {
                match archives
                    .read_name(TIPS_ENTRY)
                    .map_err(|e| e.to_string())
                    // `text` rather than `expand`: shortening is per file, and
                    // the PS2 leaves several of these plain.
                    .and_then(|blob| fexml::text(&blob).map_err(|e| e.to_string()))
                {
                    Ok(xml) => tips = self::tips(&xml, strings),
                    Err(e) => notes.push(format!("{TIPS_ENTRY}: {e}")),
                }
                match archives
                    .read_name(GLOW_STRIP_ENTRY)
                    .map_err(|e| e.to_string())
                    .and_then(|blob| {
                        oag_render::loading::GlowStrip::decode(&blob).map_err(|e| format!("{e:#}"))
                    }) {
                    Ok(decoded) => strip = Some(decoded),
                    Err(e) => notes.push(format!("{GLOW_STRIP_ENTRY}: {e}")),
                }
            }
            Err(e) => notes.push(format!("no loading screen assets from {source}: {e}")),
        }

        let strip = strip.unwrap_or_else(|| {
            notes.push(
                "the loading wave is drawing an authored stand-in strip, not the disc's own"
                    .to_string(),
            );
            oag_render::loading::GlowStrip::placeholder(oag_render::loading::STRIP_SIZE as u32)
        });
        notes.push(format!("{TIPS_ENTRY}: {} loading tip(s)", tips.len()));

        Self { tips, strip, notes }
    }
}

/// Every `<PI_LoadingScreen>`'s body string, resolved through `strings`.
///
/// The tip is the `<Text StringSrc="MSC_LOAD_...">` child; the `<Title>`'s own
/// `TitleSrc` names the weapon the original's artwork shows and is skipped with
/// it. An id the table does not carry is dropped rather than drawn as its own
/// id: `get_or_id`'s "showing the id beats showing nothing" is right for a
/// screen whose title is missing and wrong for one entry of a rotating list,
/// where the next one along is a better thing to show than `MSC_LOAD_QUAKE`.
#[must_use]
pub fn tips(xml: &str, strings: &StringTable) -> Vec<String> {
    let root = fexml::parse(xml);
    let mut out = Vec::new();
    collect_tips(&root, strings, &mut out);
    out
}

/// Walks the tree for `<PI_LoadingScreen>` at any depth.
///
/// Any depth rather than only the root's children: the file this parses puts
/// them directly under `<Screen name="Top">`, but a front-end XML nests screens
/// inside screens elsewhere and a reader that assumed one level would silently
/// find nothing on the first source that did.
fn collect_tips(node: &fexml::Node, strings: &StringTable, out: &mut Vec<String>) {
    for child in &node.children {
        if child.name.eq_ignore_ascii_case("PI_LoadingScreen") {
            if let Some(text) = child
                .children_named("Text")
                .next()
                .and_then(|text| text.attr("StringSrc"))
                .and_then(|id| strings.get(id))
            {
                out.push(text.to_string());
            }
            continue;
        }
        collect_tips(child, strings, out);
    }
}

/// The screen's own state: the wave, which tip is up, and how far the fade is.
///
/// No GPU and no worker handle. The [`Progress`] it draws is passed in per
/// frame, so this is a value a test can drive to any state without a disc, a
/// device or a running conversion.
#[derive(Debug)]
pub struct Screen {
    wave: Wave,
    rng: oag_core::rng::Rng,
    tips: Vec<String>,
    /// Frames since the screen opened, which is what rotates the tip.
    frames: u32,
    /// Frames since the work finished, counting to [`FADE_FRAMES`].
    fade: u32,
}

impl Screen {
    /// A screen at the start of its first beat, showing the first tip.
    #[must_use]
    pub fn new(tips: Vec<String>) -> Self {
        Self {
            wave: Wave::new(),
            rng: oag_core::rng::Rng::new(WAVE_SEED),
            tips,
            frames: 0,
            fade: 0,
        }
    }

    /// Steps one frame.
    ///
    /// `finished` is the worker's own flag. Once it is set the wave is frozen
    /// rather than merely stopped being drawn - see [`Wave::finish`] - and the
    /// fade starts counting.
    pub fn advance(&mut self, finished: bool) {
        self.frames = self.frames.saturating_add(1);
        if finished {
            self.wave.finish();
            self.fade = self.fade.saturating_add(1);
        } else {
            self.wave.advance();
        }
    }

    /// Whether the fade has run out and the window can be handed on.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.fade >= FADE_FRAMES
    }

    /// How opaque the screen is, `1.0` until the work finishes and `0.0` at the
    /// end of the fade.
    #[must_use]
    pub fn opacity(&self) -> f32 {
        if self.fade == 0 {
            return 1.0;
        }
        1.0 - (self.fade as f32 / FADE_FRAMES as f32).min(1.0)
    }

    /// This frame's wave, in normalised screen space.
    ///
    /// Takes `&mut self` because the walk draws from the screen's own
    /// [`Rng`](oag_core::rng::Rng) twice per column - that is the only thing
    /// making one frame differ from the last.
    pub fn quads(&mut self) -> Vec<Quad> {
        let columns = self.wave.columns(&mut self.rng);
        self.wave.quads(&columns)
    }

    /// The tip currently up, or `None` when the source offered none.
    #[must_use]
    pub fn tip(&self) -> Option<&str> {
        if self.tips.is_empty() {
            return None;
        }
        let at = (self.frames / TIP_FRAMES) as usize % self.tips.len();
        self.tips.get(at).map(String::as_str)
    }

    /// The whole screen except the wave, in the PSP's 480x272 screen space.
    ///
    /// The wave is not in here because it is not a [`Draw`]: it has its own
    /// pipeline and its own additive blend, and is drawn over this list in a
    /// second pass. See [`oag_render::loading::Pipeline`].
    #[must_use]
    pub fn draw_list(&self, phase: Phase, progress: &Progress, atlas: &Atlas) -> Vec<Draw> {
        let mut out = Vec::new();
        let fade = self.opacity();
        let dim = |colour: [f32; 4]| [colour[0], colour[1], colour[2], colour[3] * fade];

        out.push(Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: BACKDROP,
        });

        out.push(Draw::Text {
            x: SCREEN.0 / 2.0,
            y: HEADING_Y,
            scale: HEADING_SCALE,
            color: dim(HEADING),
            border: None,
            align: Align::Centre,
            text: heading(phase, progress),
        });

        if let Some(tip) = self.tip() {
            for (row, line) in wrap(atlas, tip, TIP_SCALE, TIP_WIDTH)
                .into_iter()
                .take(TIP_LINES)
                .enumerate()
            {
                out.push(Draw::Text {
                    x: SCREEN.0 / 2.0,
                    y: TIP_Y + row as f32 * TIP_LINE_HEIGHT,
                    scale: TIP_SCALE,
                    color: dim(TIP_COLOUR),
                    border: None,
                    align: Align::Centre,
                    text: line,
                });
            }
        }

        // The bar and the counts say the same thing twice on purpose: the bar is
        // what is read at a glance across ten minutes, and the figures are what
        // distinguish "slow" from "stuck" when the bar has not visibly moved.
        //
        // All of it together, or none of it: see [`counted`].
        if counted(progress) {
            let filled = fraction(phase, progress) * BAR_WIDTH;
            out.push(Draw::Fill {
                rect: [BAR_X, BAR_Y, BAR_WIDTH, BAR_HEIGHT],
                color: dim(BAR_TROUGH),
            });
            if filled > 0.0 && !progress.planning {
                out.push(Draw::Fill {
                    rect: [BAR_X, BAR_Y, filled, BAR_HEIGHT],
                    color: dim(BAR_FILL),
                });
            }

            out.push(Draw::Text {
                x: BAR_X,
                y: COUNTS_Y,
                scale: COUNTS_SCALE,
                color: dim(COUNTS),
                border: None,
                align: Align::Left,
                text: counts(phase, progress),
            });
            out.push(Draw::Text {
                x: BAR_X + BAR_WIDTH,
                y: COUNTS_Y,
                scale: COUNTS_SCALE,
                color: dim(COUNTS),
                border: None,
                align: Align::Right,
                text: percentage(phase, progress),
            });
        }

        if let Some(current) = &progress.current {
            out.push(Draw::Text {
                x: BAR_X,
                y: CURRENT_Y,
                scale: CURRENT_SCALE,
                color: dim(CURRENT),
                border: None,
                align: Align::Left,
                text: elide(atlas, current, CURRENT_SCALE, BAR_WIDTH),
            });
        }

        // A line of its own rather than a suffix on the one above, because
        // `elide` keeps a string's *tail*: appended, the state would survive and
        // the entry name would be eaten from the left, which is the wrong half
        // to lose. Two short lines are both legible at any name length.
        if let Some(step) = step_line(phase) {
            out.push(Draw::Text {
                x: BAR_X,
                y: STEP_Y,
                scale: CURRENT_SCALE,
                color: dim(CURRENT),
                border: None,
                align: Align::Left,
                text: step,
            });
        }

        out
    }
}

/// What the current load is doing, under the entry name it is doing it to.
///
/// `None` draws nothing: a phase with no step to report has no line, rather than
/// a line saying it has nothing to report.
fn step_line(phase: Phase) -> Option<String> {
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
fn heading(phase: Phase, progress: &Progress) -> String {
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
fn counted(progress: &Progress) -> bool {
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
fn fraction(phase: Phase, progress: &Progress) -> f32 {
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
fn counts(phase: Phase, progress: &Progress) -> String {
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
fn percentage(phase: Phase, progress: &Progress) -> String {
    if progress.planning {
        return String::new();
    }
    format!("{}%", (fraction(phase, progress) * 100.0).round())
}

/// Breaks `text` into lines no wider than `max_width` in screen units.
///
/// Measured through the atlas that will draw it rather than by character count,
/// because the disc's fonts are proportional - a line of `MISSILES` and a line
/// of `illillil` are the same length in characters and nothing like it on
/// screen. A single word longer than `max_width` is left over-long rather than
/// broken mid-word: it cannot be made to fit, and hyphenating it would invent
/// text.
#[must_use]
pub fn wrap(atlas: &Atlas, text: &str, scale: f32, max_width: f32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && font::measure(atlas, &candidate) * scale > max_width {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// What a shortened label is marked with.
///
/// Three periods rather than U+2026, and that is a correctness choice rather
/// than a stylistic one: [`crate::render::Renderer`] skips a character the
/// atlas has no cell for and [`font::measure`] skips it too, so a marker the
/// disc's font does not carry would disappear silently and leave a truncated
/// label reading as a whole one. The disc spells it this way itself -
/// `MSC_LOADING` is `CHARGEMENT...` - so this is both the safe glyph and the
/// game's own.
const ELISION: &str = "...";

/// Shortens `text` from the left until it fits, marking that it was shortened.
///
/// From the left because the informative half of an asset label is its tail:
/// `Data.wad Data\Movies\Intro.PMF` truncated the other way is 30 characters of
/// archive name and no answer to "which movie".
#[must_use]
pub fn elide(atlas: &Atlas, text: &str, scale: f32, max_width: f32) -> String {
    if font::measure(atlas, text) * scale <= max_width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.remove(0);
        let candidate: String = ELISION.chars().chain(chars.iter().copied()).collect();
        if font::measure(atlas, &candidate) * scale <= max_width {
            return candidate;
        }
    }
    String::new()
}

/// What the screen is drawn on.
///
/// Not pure black: the original's loading screen blits a full-screen backdrop
/// under the wave, and although this build has not recovered that image, black
/// under a cyan glow reads as a missing asset rather than as a background.
const BACKDROP: [f32; 4] = [0.02, 0.03, 0.05, 1.0];

const HEADING_Y: f32 = 40.0;
const HEADING_SCALE: f32 = 1.4;
const HEADING: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const TIP_Y: f32 = 74.0;
const TIP_SCALE: f32 = 0.9;
const TIP_LINE_HEIGHT: f32 = 14.0;
const TIP_WIDTH: f32 = 380.0;
/// How many wrapped lines of a tip are drawn, the rest being dropped.
///
/// A cap and not a measurement: the only cut measured on screen is the German
/// one, whose longest tips wrap to two lines at [`TIP_WIDTH`], and the Italian
/// and Spanish tables carry visibly longer prose for the same ids. Four leaves
/// room above the bar for roughly twice the German length. **A tip longer than
/// that loses its ending silently**, which is the trade taken rather than
/// letting the text run into the counts; a cut that needs more than four lines
/// is a reason to shrink [`TIP_SCALE`], not to raise this.
const TIP_LINES: usize = 4;
const TIP_COLOUR: [f32; 4] = [0.62, 0.72, 0.82, 1.0];

const BAR_X: f32 = 50.0;
/// High enough that the counts clear the wave.
///
/// The band is centred on `BASELINE_Y = 220` and swings about 40 reference
/// pixels either side of it at full amplitude, so anything below about 180
/// gets a cyan glow through it. The wave's own geometry is recovered and does
/// not move; this does.
const BAR_Y: f32 = 140.0;
const BAR_WIDTH: f32 = SCREEN.0 - BAR_X * 2.0;
const BAR_HEIGHT: f32 = 6.0;
const BAR_TROUGH: [f32; 4] = [0.12, 0.16, 0.2, 1.0];
const BAR_FILL: [f32; 4] = [0.35, 0.85, 0.95, 1.0];

const COUNTS_Y: f32 = 152.0;
const COUNTS_SCALE: f32 = 0.9;
const COUNTS: [f32; 4] = [0.85, 0.9, 0.95, 1.0];

const CURRENT_Y: f32 = 168.0;
const CURRENT_SCALE: f32 = 0.8;
const CURRENT: [f32; 4] = [0.5, 0.58, 0.66, 1.0];

/// One line under the entry name, at the same scale. See [`step_line`].
const STEP_Y: f32 = 180.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(done: usize, total: usize) -> Progress {
        Progress {
            planning: false,
            total,
            done,
            cached: 0,
            failed: 0,
            current: None,
            finished: false,
        }
    }

    fn text_of(list: &[Draw]) -> Vec<String> {
        list.iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// The tips come out of the XML in the order the file lists them, with the
    /// title's own id left alone.
    #[test]
    fn a_loading_definition_yields_one_tip_per_entry() {
        let xml = r#"<Screen name="Top">
            <PI_LoadingScreen name="Bomb">
              <Title TitleSrc="PRO_STATS_BOMB"></Title>
              <Text StringSrc="TIP_ONE"></Text>
            </PI_LoadingScreen>
            <PI_LoadingScreen name="Mines">
              <Text StringSrc="TIP_TWO"></Text>
            </PI_LoadingScreen>
          </Screen>"#;
        let strings = StringTable::from_xml(
            r#"<Entries>
                 <Entry ID="TIP_ONE" String="first"></Entry>
                 <Entry ID="TIP_TWO" String="second"></Entry>
                 <Entry ID="PRO_STATS_BOMB" String="BOMB"></Entry>
               </Entries>"#,
        );
        assert_eq!(tips(xml, &strings), ["first", "second"]);
    }

    /// An id the chosen language does not carry drops out rather than being
    /// drawn as itself - see [`tips`].
    #[test]
    fn an_unresolved_tip_is_dropped_rather_than_shown_as_its_id() {
        let xml = r#"<Screen><PI_LoadingScreen><Text StringSrc="MSC_LOAD_MISSING"></Text>
          </PI_LoadingScreen></Screen>"#;
        assert!(tips(xml, &StringTable::default()).is_empty());
    }

    /// The tips this project's own file nests one level down; a reader that
    /// only looked at the root's children would find them anyway, so the guard
    /// is a deeper nesting.
    #[test]
    fn tips_are_found_however_deeply_the_xml_nests_them() {
        let xml = r#"<Screen><Group><Inner>
            <PI_LoadingScreen><Text StringSrc="TIP"></Text></PI_LoadingScreen>
          </Inner></Group></Screen>"#;
        let strings = StringTable::from_xml(r#"<E><Entry ID="TIP" String="deep"></Entry></E>"#);
        assert_eq!(tips(xml, &strings), ["deep"]);
    }

    /// Every line fits, and nothing is lost or duplicated in the middle.
    #[test]
    fn wrapping_fits_every_line_and_keeps_every_word() {
        let atlas = Atlas::build();
        let text = "Actuating the left or right airbrake through a corner turns tighter \
                    without shedding speed";
        let lines = wrap(&atlas, text, 1.0, 200.0);
        assert!(lines.len() > 1, "{lines:?}");
        for line in &lines {
            assert!(
                font::measure(&atlas, line) <= 200.0,
                "{line:?} is {} wide",
                font::measure(&atlas, line)
            );
        }
        assert_eq!(
            lines.join(" "),
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        );
    }

    /// A word that cannot fit is left over-long rather than cut in half or
    /// dropped.
    #[test]
    fn a_word_wider_than_the_column_is_kept_whole() {
        let atlas = Atlas::build();
        let lines = wrap(&atlas, "antigravitational", 1.0, 10.0);
        assert_eq!(lines, ["antigravitational"]);
    }

    /// The tail survives, which is the half that names the asset.
    #[test]
    fn eliding_keeps_the_end_of_a_label() {
        let atlas = Atlas::build();
        let label = r"Data.wad Data\Movies\Intro.PMF";
        // Wide enough for the marker and the tail together: a column narrower
        // than `ELISION` plus what is worth keeping has nothing to return but
        // the marker and a fragment, which is the degenerate case rather than
        // the one worth pinning.
        let short = elide(&atlas, label, 1.0, 120.0);
        assert!(short.starts_with(ELISION), "{short:?}");
        assert!(short.ends_with("Intro.PMF"), "{short:?}");
        assert!(short.len() < label.len(), "{short:?}");
        assert!(font::measure(&atlas, &short) <= 120.0);
    }

    /// The marker has to be drawable, which is the whole reason it is three
    /// periods - see [`ELISION`]. Checked against the built-in atlas here; the
    /// disc's fonts are a superset of ASCII, and a font missing `.` could not
    /// draw a file name either.
    #[test]
    fn the_elision_marker_is_a_glyph_the_atlas_can_draw() {
        let atlas = Atlas::build();
        assert!(font::measure(&atlas, ELISION) > 0.0);
        for ch in ELISION.chars() {
            assert!(atlas.cell(ch).is_some(), "{ch:?} has no cell");
        }
    }

    #[test]
    fn a_label_that_already_fits_is_left_alone() {
        let atlas = Atlas::build();
        assert_eq!(elide(&atlas, "Data.wad", 1.0, 400.0), "Data.wad");
    }

    /// The three things the screen exists to say are all in the list.
    #[test]
    fn the_draw_list_states_the_counts_the_fraction_and_the_asset() {
        let atlas = Atlas::build();
        let screen = Screen::new(vec!["a tip".to_string()]);
        let mut state = progress(37, 115);
        state.cached = 12;
        state.current = Some("Data.wad hash:71d3c1ec".to_string());
        let drawn = text_of(&screen.draw_list(Phase::Prefetch, &state, &atlas));

        assert!(
            drawn.iter().any(|line| line.contains("37 / 115")),
            "{drawn:?}"
        );
        assert!(
            drawn.iter().any(|line| line.contains("12 already cached")),
            "{drawn:?}"
        );
        assert!(drawn.iter().any(|line| line == "32%"), "{drawn:?}");
        assert!(
            drawn.iter().any(|line| line.contains("hash:71d3c1ec")),
            "{drawn:?}"
        );
        assert!(drawn.iter().any(|line| line == "a tip"), "{drawn:?}");
    }

    /// **Nothing to count draws no counter at all.** A title whose chain names
    /// no movie, on a run with no `--prefetch`, has nothing behind this screen
    /// and every number on it would be zero. The bar, the `0 / 0` and the `100%`
    /// are one row and are all absent together; the tip and the wave are what is
    /// left.
    #[test]
    fn a_boot_with_nothing_to_convert_draws_no_bar_and_no_counts() {
        let atlas = Atlas::build();
        let screen = Screen::new(vec!["a tip".to_string()]);
        // What `Session::prefetch_progress` reports on a run with no worker,
        // and what `MediaPlan::loads` counts for a plan that names nothing.
        let state = Progress {
            finished: true,
            ..Progress::default()
        };
        let list = screen.draw_list(Phase::Media(None), &state, &atlas);
        let drawn = text_of(&list);

        assert!(
            !drawn.iter().any(|line| line.contains("loaded")),
            "no done/total pair: {drawn:?}"
        );
        assert!(
            !drawn.iter().any(|line| line.ends_with('%')),
            "no percentage: {drawn:?}"
        );
        assert!(drawn.iter().any(|line| line == "a tip"), "{drawn:?}");
        assert!(
            drawn.iter().any(|line| line == "LOADING"),
            "the heading says what is happening rather than READY: {drawn:?}"
        );
        // The backdrop fill is the only one left: no trough, no filled half.
        let fills = list
            .iter()
            .filter(|draw| matches!(draw, Draw::Fill { .. }))
            .count();
        assert_eq!(fills, 1, "only the backdrop, no bar: {list:?}");
    }

    /// **The ordinary boot draws the bar again**, because the media phase now
    /// counts its own loads: `Session::loading_progress` folds
    /// [`crate::boot::MediaWorker::progress`] in, and a plan naming both movies
    /// and a backdrop is five loads rather than nothing.
    ///
    /// The regression this pins is the one that removed it: the phase reported
    /// no counts, `counted` read that as the empty case, and the whole row went
    /// with it.
    #[test]
    fn the_media_phase_draws_the_bar_and_counts_its_own_loads() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let state = Progress {
            total: 5,
            done: 2,
            current: Some("WO_INTRO.PMF".to_string()),
            ..Progress::default()
        };
        let list = screen.draw_list(Phase::Media(None), &state, &atlas);
        let drawn = text_of(&list);

        assert!(
            drawn.iter().any(|line| line == "2 / 5 loaded"),
            "the media phase loads rather than converts: {drawn:?}"
        );
        assert!(drawn.iter().any(|line| line == "40%"), "{drawn:?}");
        assert!(
            drawn.iter().any(|line| line.contains("WO_INTRO.PMF")),
            "the reel being read is named: {drawn:?}"
        );
        assert!(
            drawn.iter().any(|line| line == "LOADING MOVIES"),
            "not CONVERTING ASSETS, which is the other phase: {drawn:?}"
        );
        // The backdrop, the trough and the filled part.
        let fills = list
            .iter()
            .filter(|draw| matches!(draw, Draw::Fill { .. }))
            .count();
        assert_eq!(fills, 3, "{list:?}");
    }

    /// Nothing counted yet is not `100%` - see [`percentage`].
    #[test]
    fn planning_reports_no_percentage_it_cannot_know() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let state = Progress {
            planning: true,
            ..Progress::default()
        };
        let drawn = text_of(&screen.draw_list(Phase::Prefetch, &state, &atlas));
        assert!(!drawn.iter().any(|line| line.ends_with('%')), "{drawn:?}");
        assert!(
            drawn.iter().any(|line| line == "READING THE ARCHIVES"),
            "{drawn:?}"
        );
    }

    /// The bar's filled half is drawn only once there is a real fraction, and
    /// it is exactly the fraction wide.
    #[test]
    fn the_bar_is_the_fraction_wide() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let fills: Vec<[f32; 4]> = screen
            .draw_list(Phase::Prefetch, &progress(1, 4), &atlas)
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        // The backdrop, the trough and the fill.
        assert_eq!(fills.len(), 3, "{fills:?}");
        assert!((fills[2][2] - BAR_WIDTH * 0.25).abs() < 0.01, "{fills:?}");
    }

    /// **A transcode says so, and says how far.** The complaint this answers is
    /// that a cache hit and an eighty-second `ffmpeg` run looked identical: same
    /// heading, same entry name, one of them over before it was read.
    #[test]
    fn a_transcode_is_headed_and_counted_apart_from_a_cache_hit() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let state = Progress {
            total: 5,
            done: 2,
            current: Some(r"Data\Movies\Intro.pss".to_string()),
            ..Progress::default()
        };

        let transcoding = text_of(&screen.draw_list(
            Phase::Media(Some(crate::movie::Step::Transcoding {
                done: 340,
                total: Some(1200),
            })),
            &state,
            &atlas,
        ));
        assert!(
            transcoding.iter().any(|l| l == "TRANSCODING MOVIES"),
            "{transcoding:?}"
        );
        assert!(
            transcoding
                .iter()
                .any(|l| l == "transcoding - frame 340 of 1200"),
            "the frame counter is the anti-hang signal: {transcoding:?}"
        );

        let cached = text_of(&screen.draw_list(
            Phase::Media(Some(crate::movie::Step::Cached)),
            &state,
            &atlas,
        ));
        assert!(cached.iter().any(|l| l == "LOADING MOVIES"), "{cached:?}");
        assert!(
            cached.iter().any(|l| l.contains("reading the cache")),
            "{cached:?}"
        );
        assert!(
            !cached.iter().any(|l| l.contains("transcoding")),
            "a cache hit must not borrow the last transcode's caption: {cached:?}"
        );

        // Nothing reported yet, and the sound loads, which never report.
        let quiet = text_of(&screen.draw_list(Phase::Media(None), &state, &atlas));
        assert!(quiet.iter().any(|l| l == "LOADING MOVIES"), "{quiet:?}");
        assert!(
            !quiet.iter().any(|l| l.contains("transcoding")
                || l.contains("decoding")
                || l.contains("cache")),
            "no step is no line, not a line saying there is no step: {quiet:?}"
        );
    }

    /// An uncapped conversion has a numerator and no denominator, and must not
    /// invent one - see [`crate::movie::Step::Transcoding`].
    #[test]
    fn an_uncapped_transcode_counts_frames_without_claiming_a_total() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let drawn = text_of(&screen.draw_list(
            Phase::Media(Some(crate::movie::Step::Transcoding {
                done: 91,
                total: None,
            })),
            &progress(1, 2),
            &atlas,
        ));
        assert!(
            drawn.iter().any(|l| l == "transcoding - frame 91"),
            "{drawn:?}"
        );
    }

    /// **The bar crosses its slice as the transcode runs**, rather than sitting
    /// still for a minute and then stepping. See [`fraction`].
    #[test]
    fn a_transcode_fills_its_own_slice_of_the_bar() {
        // Two of five loads done, so the third owns 40%..60%.
        let state = Progress {
            total: 5,
            done: 2,
            ..Progress::default()
        };
        let at = |done, total| {
            fraction(
                Phase::Media(Some(crate::movie::Step::Transcoding { done, total })),
                &state,
            )
        };

        // The head, before a frame is encoded: started, but barely.
        let started = at(0, Some(1200));
        assert!(
            (started - (0.4 + 0.2 / 5.0)).abs() < 1e-6,
            "the read-or-decode head is LOADING_SHARE of one slice: {started}"
        );
        // Half the frames is half of what is left of the slice.
        let half = at(600, Some(1200));
        assert!(
            (half - (0.4 + (0.2 + 0.8 * 0.5) / 5.0)).abs() < 1e-6,
            "{half}"
        );
        // The last frame lands exactly on the slice boundary, which is where
        // `done + 1` puts it a moment later.
        let end = at(1200, Some(1200));
        assert!((end - 0.6).abs() < 1e-6, "{end}");

        assert!(started < half && half < end, "{started} {half} {end}");
    }

    /// A slice is never left behind and never overshot, whatever a step reports.
    ///
    /// The property the whole scheme rests on: a bar that went backwards would
    /// be worse than one that only stepped, and `Cached` taking a whole slice is
    /// the case where that is easiest to get wrong.
    #[test]
    fn no_step_moves_the_bar_backwards_or_past_its_own_slice() {
        use crate::movie::Step;

        let state = Progress {
            total: 4,
            done: 1,
            ..Progress::default()
        };
        let floor = fraction(Phase::Media(None), &state);
        let ceiling = fraction(
            Phase::Media(Some(Step::Cached)),
            &Progress {
                done: 2,
                ..state.clone()
            },
        );
        assert!((floor - 0.25).abs() < 1e-6, "{floor}");

        for step in [
            Step::Cached,
            Step::Decoding,
            Step::Transcoding {
                done: 0,
                total: None,
            },
            Step::Transcoding {
                done: 999,
                total: None,
            },
            // A stated total of zero: nobody could divide by it, and it must not
            // produce a NaN width either.
            Step::Transcoding {
                done: 0,
                total: Some(0),
            },
            Step::Transcoding {
                done: 7,
                total: Some(7),
            },
            // More frames than the total claimed, which `ffmpeg` has no reason
            // to report but which must not push past the slice if it does.
            Step::Transcoding {
                done: 99,
                total: Some(7),
            },
        ] {
            let at = fraction(Phase::Media(Some(step)), &state);
            assert!(at.is_finite(), "{step:?} gave {at}");
            assert!(
                (floor..=floor + 0.25 + 1e-6).contains(&at),
                "{step:?} left the slice 0.25..0.50: {at}"
            );
            assert!(at <= ceiling + 1e-6, "{step:?} passed the next load: {at}");
        }
    }

    /// The figure and the width are one statement, so they are one number.
    #[test]
    fn the_percentage_is_the_width_the_bar_was_drawn_at() {
        let atlas = Atlas::build();
        let screen = Screen::new(Vec::new());
        let phase = Phase::Media(Some(crate::movie::Step::Transcoding {
            done: 600,
            total: Some(1200),
        }));
        let state = Progress {
            total: 5,
            done: 2,
            ..Progress::default()
        };
        let list = screen.draw_list(phase, &state, &atlas);

        let widths: Vec<f32> = list
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, .. } => Some(rect[2]),
                _ => None,
            })
            .collect();
        // The backdrop, the trough and the fill.
        assert_eq!(widths.len(), 3, "{list:?}");
        let drawn = (widths[2] / BAR_WIDTH * 100.0).round();

        let text = text_of(&list);
        assert!(
            text.iter().any(|line| line == &format!("{drawn}%")),
            "the bar is {drawn}% wide and the figures beside it say: {text:?}"
        );
    }

    /// The wave freezes when the work does, and only then - `Wave::finish` is
    /// the original's `g_loading_finished` and this is the thing that sets it.
    #[test]
    fn finishing_freezes_the_wave_and_starts_the_fade() {
        let mut screen = Screen::new(Vec::new());
        for _ in 0..5 {
            screen.advance(false);
        }
        let running = screen.wave;
        assert!(!running.is_finished());
        assert_eq!(screen.opacity(), 1.0);
        assert!(!screen.is_done());

        screen.advance(true);
        assert!(screen.wave.is_finished());
        assert_eq!(
            screen.wave.phase(),
            running.phase(),
            "a frozen wave stands still"
        );
        assert!(screen.opacity() < 1.0);

        for _ in 0..FADE_FRAMES {
            screen.advance(true);
        }
        assert!(screen.is_done());
        assert_eq!(screen.opacity(), 0.0);
    }

    /// The fade reaches every colour on the screen, not only the wave's tint.
    #[test]
    fn the_fade_dims_the_whole_list() {
        let atlas = Atlas::build();
        let mut screen = Screen::new(Vec::new());
        for _ in 0..FADE_FRAMES / 2 {
            screen.advance(true);
        }
        let list = screen.draw_list(Phase::Prefetch, &progress(1, 2), &atlas);
        let alpha = list
            .iter()
            .find_map(|draw| match draw {
                Draw::Text { color, .. } => Some(color[3]),
                _ => None,
            })
            .expect("a line of text");
        assert!(alpha > 0.0 && alpha < 1.0, "{alpha}");
    }

    /// Rotation is by time, wraps round, and a source with no tips draws none
    /// rather than dividing by zero.
    #[test]
    fn tips_rotate_and_wrap() {
        let mut screen = Screen::new(vec!["one".to_string(), "two".to_string()]);
        assert_eq!(screen.tip(), Some("one"));
        for _ in 0..TIP_FRAMES {
            screen.advance(false);
        }
        assert_eq!(screen.tip(), Some("two"));
        for _ in 0..TIP_FRAMES {
            screen.advance(false);
        }
        assert_eq!(screen.tip(), Some("one"));

        assert_eq!(Screen::new(Vec::new()).tip(), None);
    }

    /// Every column of the wave is there, three bands deep, and two frames of a
    /// running wave differ - the walk is re-drawn rather than integrated.
    #[test]
    fn the_wave_produces_a_fresh_band_every_frame() {
        let mut screen = Screen::new(Vec::new());
        for _ in 0..5 {
            screen.advance(false);
        }
        let first = screen.quads();
        let second = screen.quads();
        assert_eq!(
            first.len(),
            oag_render::loading::COLUMNS * oag_render::loading::BANDS
        );
        assert_ne!(first, second);
    }
}
