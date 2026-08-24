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

use oag_render::loading::{Quad, Wave};

use crate::font::{self, Atlas};
use crate::frontend::{Align, Draw, SCREEN};
use crate::prefetch::Progress;

/// Pulse's own two entries, re-exported for the tests and reports that name
/// them.
///
/// **Not what a loader reaches for any more.** Which entries a source's loading
/// screen reads is [`oag_title::Loading`]'s question since 2026-08-24, because
/// the third title answers it differently: Pure ships neither of these and
/// Wipeout HD ships a full-screen still instead. See [`Assets::load`].
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
    /// A circuit being read off the disc, between the menus and the grid. See
    /// [`crate::race::LoadWorker`].
    ///
    /// **The one phase that draws no counts, and the reason is the original.**
    /// The other two are this build's own waits - a transcode and a prefetch,
    /// neither of which any release has - and the figures are what make them
    /// bearable. A race load is the wait the original *also* has, and what the
    /// original puts on screen for it is a still and a word (Wipeout HD) or a
    /// wave and a tip (Pulse). A progress bar here would be this build
    /// decorating a screen the disc already authors, which is the opposite of
    /// what the other two phases needed.
    ///
    /// It draws nothing of its own, then: the circuit's name arrives through
    /// [`Progress::current`] like any other load's, and a `Progress` with no
    /// total draws no bar. See [`counted`].
    Race,
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

/// The screen's own state: the wave, which tip is up, and how far the fade is.
///
/// No GPU and no worker handle. The [`Progress`] it draws is passed in per
/// frame, so this is a value a test can drive to any state without a disc, a
/// device or a running conversion.
#[derive(Debug)]
pub struct Screen {
    wave: Wave,
    /// Whether this title authors a wave at all. See [`Assets::wave`].
    has_wave: bool,
    rng: oag_core::rng::Rng,
    tips: Vec<String>,
    /// Where the feature illustration sits in the sheet the caller built the
    /// renderer with, in sheet pixels, when the title ships one.
    ///
    /// The placement rather than the sheet: the picture is the renderer's and
    /// this is a value a test can hold. `None` draws no illustration, and the
    /// five below are the marks the screen is framed with - each `None` simply
    /// is not drawn. See [`Assets::art`].
    illustration: Option<[f32; 4]>,
    title_arrow: Option<[f32; 4]>,
    subtitle_arrow: Option<[f32; 4]>,
    rule: Option<[f32; 4]>,
    corner: Option<[f32; 4]>,
    /// The feature's own name, when its id was recovered.
    feature_title: Option<String>,
    /// The paragraph under it, which takes the tip line's place on a title that
    /// has features rather than tips.
    feature_text: Option<String>,
    /// The disc's own word for this wait, when it has one.
    caption: Option<String>,
    /// What every text scale below is multiplied by, so a disc's own face draws
    /// at the size this layout was authored for.
    ///
    /// **This layout is in the PSP's 480x272 and the font is the source's.**
    /// Drawn at the same nominal scale on every disc, Wipeout HD's heading
    /// spanned the whole frame and its tip line ran off both edges - the
    /// loading screen's own version of the bug `oag_game::menu::Skin` fixed for
    /// the menus, where HD's authored `menu_x: 800` put every label 320 pixels
    /// off a 480-wide screen.
    ///
    /// The menus fixed it by drawing in the *source's* grid. This screen's
    /// layout is not the source's - it is this build's - so the grid stays and
    /// the **face** is normalised into it instead, against
    /// [`AUTHORED_LINE_HEIGHT`].
    ///
    /// **Keyed on the face and not on the source's screen size**, which is the
    /// correction that matters and which a first attempt got wrong. The three
    /// discs in hand, from their own boot reports:
    ///
    /// | source | grid | `Default` face | line height |
    /// | --- | --- | --- | ---: |
    /// | `pulse-psp-eu.chd` | 480x272 | `pulse_text.fnt` | 13 |
    /// | `pulse-ps2-eu.chd` | 640x448 | `pulse_text.fnt` | 14 |
    /// | `hdfury-ps3-eu-dec.iso` | 1920x1080 | `helv.fnt` | 33 |
    ///
    /// The PS2's grid is 1.65 times the PSP's and its face is the **same
    /// size** - it is the same file. Dividing by the grid would have shrunk the
    /// PS2's loading screen to six tenths for no reason, which is exactly the
    /// lineage habit `oag_pulse::loading` already records from the other side:
    /// the PS2 port kept the PSP's 480-wide wave numbers on a 640-wide screen.
    font_scale: f32,
    /// Frames since the screen opened, which is what rotates the tip.
    frames: u32,
    /// Frames since the work finished, counting to [`FADE_FRAMES`].
    fade: u32,
}

impl Screen {
    /// A screen at the start of its first beat, showing whatever the source
    /// gave it.
    ///
    /// Takes the whole [`Assets`] rather than the tips alone since the title
    /// axis landed: a backdrop, a caption and whether there is a wave at all
    /// are all per-title now, and threading four arguments through every caller
    /// would put the same four together at each of them.
    #[must_use]
    pub fn new(assets: &Assets, line_height: f32) -> Self {
        Self {
            wave: Wave::new(),
            has_wave: assets.wave,
            rng: oag_core::rng::Rng::new(WAVE_SEED),
            tips: assets.tips.clone(),
            illustration: assets.art.as_ref().and_then(|art| art.illustration),
            title_arrow: assets.art.as_ref().and_then(|art| art.title_arrow),
            subtitle_arrow: assets.art.as_ref().and_then(|art| art.subtitle_arrow),
            rule: assets.art.as_ref().and_then(|art| art.rule),
            corner: assets.art.as_ref().and_then(|art| art.corner),
            feature_title: assets
                .feature
                .as_ref()
                .and_then(|feature| feature.title.clone()),
            feature_text: assets
                .feature
                .as_ref()
                .map(|feature| feature.description.clone()),
            caption: assets.caption.clone(),
            // Guarded against a zero line height, which no real face has: a
            // divide by zero here would put every glyph at `NaN` and draw
            // nothing, which is the hardest possible failure to read.
            font_scale: if line_height > 0.0 {
                AUTHORED_LINE_HEIGHT / line_height
            } else {
                1.0
            },
            frames: 0,
            fade: 0,
        }
    }

    /// A layout scale in this screen's own grid, corrected for the source's own
    /// face. See [`Self::font_scale`].
    fn text(&self, scale: f32) -> f32 {
        scale * self.font_scale
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
        // **Nothing at all on a title that authors no wave**, rather than
        // Pulse's wave over another release's screen. The strip behind this
        // pipeline is a stand-in on such a title (see [`Assets::strip`]), so
        // drawing it would be drawing a texture this build made up over a
        // picture the disc did author. See `CLAUDE.md`: draw nothing and say
        // so - the saying-so is `Assets::load`'s note.
        if !self.has_wave {
            return Vec::new();
        }
        let columns = self.wave.columns(&mut self.rng);
        self.wave.quads(&columns)
    }

    /// Whether this screen draws the procedural wave.
    #[must_use]
    pub fn has_wave(&self) -> bool {
        self.has_wave
    }

    /// The prose currently up, or `None` when the source offered none.
    ///
    /// **A title's feature description where it has one, its rotating tips
    /// otherwise**, and never both: they are the same slot on the screen and no
    /// title in hand ships both. Pulse rotates its 26 on
    /// [`TIP_FRAMES`]; a feature does not rotate, because what selects one is
    /// the original's own `Feature type` and that is unread - see
    /// [`pick_feature`].
    #[must_use]
    pub fn tip(&self) -> Option<&str> {
        if let Some(text) = self.feature_text.as_deref() {
            return Some(text);
        }
        if self.tips.is_empty() {
            return None;
        }
        let at = (self.frames / TIP_FRAMES) as usize % self.tips.len();
        self.tips.get(at).map(String::as_str)
    }

    /// The feature's own name, drawn above its prose. `None` on a title with no
    /// features, and on a feature whose title id was not recovered.
    #[must_use]
    pub fn feature_title(&self) -> Option<&str> {
        self.feature_title.as_deref()
    }

    /// Whether the caption is heading this screen, and so already carries what
    /// is loading.
    fn caption_leads(&self, progress: &Progress) -> bool {
        self.caption.is_some() && !counted(progress)
    }

    /// The line at the top of the screen.
    fn heading_text(&self, phase: Phase, progress: &Progress) -> String {
        if !self.caption_leads(progress) {
            return heading(phase, progress);
        }
        let caption = self.caption.clone().unwrap_or_default();
        match &progress.current {
            Some(current) => format!("{caption} {current}"),
            None => caption,
        }
    }

    /// Where the prose starts, which depends on what is above it.
    fn tip_top(&self) -> f32 {
        if self.illustration.is_some() {
            FEATURE_TIP_Y
        } else {
            TIP_Y
        }
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
        // **Over an illustration the text goes white and gains a dark
        // outline**, and
        // this is a legibility fix rather than a style. Every colour below was
        // chosen against the cleared near-black frame this screen used to have,
        // where the dim slate blues read as a hierarchy. Wipeout HD's still is a
        // *pale* billboard - see `oag_hd::loading::BACKDROP` - and `CURRENT`'s
        // blue-grey over its grey panel is close to invisible, which is exactly
        // the line a race load puts the circuit's name on. Tried with the
        // outline alone first: it lifts the white heading and does nothing for
        // the dim rows, because the problem there is the fill and not the edge.
        //
        // The hierarchy is what is given up, and it is the right thing to give
        // up: three shades of nearly-invisible is not a hierarchy. A title with
        // no backdrop draws exactly what it drew before - both PSP discs, and
        // any run with no title in hand.
        let over_backdrop = self.illustration.is_some();
        let border = over_backdrop.then(|| dim(OUTLINE));
        let ink = |colour: [f32; 4]| {
            if over_backdrop {
                dim(ON_BACKDROP)
            } else {
                dim(colour)
            }
        };

        // The frame is cleared either way: the backdrop is stretched over the
        // whole of it, so this is only ever seen through the fade - but the
        // fade is exactly when a hole in the list would show as whatever the
        // last stage left behind.
        out.push(Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: BACKDROP,
        });

        let (heading_x, heading_align) = if self.illustration.is_some() {
            (PANEL_X, Align::Left)
        } else {
            (SCREEN.0 / 2.0, Align::Centre)
        };
        out.push(Draw::Text {
            x: heading_x,
            y: HEADING_Y,
            scale: self.text(HEADING_SCALE),
            color: ink(HEADING),
            border,
            align: heading_align,
            // **The caption and what is loading on one line**, which is how the
            // original writes it: a running Fury race heads this screen
            // `LOADING... VINETA K`, the disc's own word followed by the
            // circuit's own name. So a title with a caption puts them together
            // and draws no separate line below; a title without one keeps this
            // build's heading and its own `current` row. A caption never
            // replaces the *counted* headings, which say something the disc's
            // single string cannot - see `heading`.
            text: self.heading_text(phase, progress),
        });

        // **The original's own arrangement**, where a title has the art for
        // it: a marked title over a rule, the illustration on the left and its
        // prose on the right, and the bar along the bottom. Read off a
        // screenshot of a running Fury race - see `docs/formats/hd-loading.md`,
        // which also says which of the labels in that shot are the game's own
        // debug overlay and are deliberately not reproduced here.
        //
        // A title with no art draws none of this and keeps the centred layout
        // below, which is what both PSP discs have always drawn.
        if self.illustration.is_some() {
            if let Some(uv) = self.title_arrow {
                out.push(Draw::Sprite {
                    rect: [
                        PANEL_X - MARKER_SIZE - 4.0,
                        HEADING_Y + 2.0,
                        MARKER_SIZE,
                        MARKER_SIZE,
                    ],
                    uv,
                    color: dim(ON_BACKDROP),
                });
            }
            if let Some(uv) = self.rule {
                for y in [RULE_TOP_Y, RULE_BOTTOM_Y] {
                    out.push(Draw::Sprite {
                        rect: [PANEL_X, y, PANEL_RIGHT - PANEL_X, RULE_HEIGHT],
                        uv,
                        color: dim(RULE_INK),
                    });
                }
            }
            for (x, y) in corners(IMAGE_BOX).into_iter().chain(corners(PROSE_BOX)) {
                if let Some(uv) = self.corner {
                    out.push(Draw::Sprite {
                        rect: [x, y, CORNER_SIZE, CORNER_SIZE],
                        uv,
                        color: dim(RULE_INK),
                    });
                }
            }
        }

        if let Some(uv) = self.illustration {
            // **Fitted, not stretched.** The aspect is kept because an
            // illustration squashed to a layout is a picture nobody authored.
            let (w, h) = (uv[2].max(1.0), uv[3].max(1.0));
            let scale = (IMAGE_BOX.2 / w).min(IMAGE_BOX.3 / h);
            let (drawn_w, drawn_h) = (w * scale, h * scale);
            out.push(Draw::Sprite {
                rect: [
                    IMAGE_BOX.0 + (IMAGE_BOX.2 - drawn_w) / 2.0,
                    IMAGE_BOX.1 + (IMAGE_BOX.3 - drawn_h) / 2.0,
                    drawn_w,
                    drawn_h,
                ],
                uv,
                color: dim([1.0, 1.0, 1.0, 1.0]),
            });
        }

        if let Some(title) = self.feature_title() {
            if let Some(uv) = self.subtitle_arrow {
                out.push(Draw::Sprite {
                    rect: [
                        PROSE_BOX.0,
                        FEATURE_TITLE_Y + 1.0,
                        MARKER_SIZE * 0.75,
                        MARKER_SIZE * 0.75,
                    ],
                    uv,
                    color: dim(ON_BACKDROP),
                });
            }
            out.push(Draw::Text {
                x: PROSE_BOX.0 + MARKER_SIZE,
                y: FEATURE_TITLE_Y,
                scale: self.text(FEATURE_TITLE_SCALE),
                color: ink(HEADING),
                border,
                align: Align::Left,
                text: title.to_string(),
            });
        }

        // **The progression bar, drawn flat.** The original has one and fills
        // it with `dot.gtf` tiled; `Draw::Sprite` has no repeat mode, so an 8x8
        // dot stretched across 320 units comes out as a blurred smear rather
        // than a pattern - tried, and it looked like a rendering fault. A flat
        // trough is the honest approximation: the shape and the place are the
        // original's and the texture is not, which is the opposite way round
        // from drawing a pattern that is not there.
        //
        // The **fill** is not drawn at all on a race load, because this build
        // has no number to fill it with - `race::load` reports no progress. So
        // the trough says "there is a bar here and nothing has told it
        // anything" rather than inventing a fraction. See
        // `docs/formats/hd-loading.md`, which carries that as an open gap.
        if self.illustration.is_some() {
            out.push(Draw::Fill {
                rect: [BAR_BOX.0, BAR_BOX.1, BAR_BOX.2, BAR_BOX.3],
                color: dim(BAR_DOTS),
            });
            let filled = if counted(progress) {
                fraction(phase, progress) * BAR_BOX.2
            } else {
                0.0
            };
            if filled > 0.0 {
                out.push(Draw::Fill {
                    rect: [BAR_BOX.0, BAR_BOX.1, filled, BAR_BOX.3],
                    color: dim(BAR_FILL),
                });
            }
        }

        if let Some(tip) = self.tip() {
            // Left-aligned in its own panel on a feature screen, centred over
            // the wave on a title without one.
            let boxed = self.illustration.is_some();
            let (x, align, width) = if boxed {
                (PROSE_BOX.0, Align::Left, PROSE_BOX.2)
            } else {
                (SCREEN.0 / 2.0, Align::Centre, TIP_WIDTH)
            };
            // A feature's paragraph is longer than a tip and has a taller panel
            // to put it in; eliding it at four lines cut every one of HD's five
            // mid-sentence.
            let lines = if boxed { FEATURE_TIP_LINES } else { TIP_LINES };
            for (row, line) in wrap(atlas, tip, self.text(TIP_SCALE), width)
                .into_iter()
                .take(lines)
                .enumerate()
            {
                out.push(Draw::Text {
                    x,
                    y: self.tip_top() + row as f32 * TIP_LINE_HEIGHT,
                    scale: self.text(TIP_SCALE),
                    color: ink(TIP_COLOUR),
                    border,
                    align,
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
                scale: self.text(COUNTS_SCALE),
                color: ink(COUNTS),
                border,
                align: Align::Left,
                text: counts(phase, progress),
            });
            out.push(Draw::Text {
                x: BAR_X + BAR_WIDTH,
                y: COUNTS_Y,
                scale: self.text(COUNTS_SCALE),
                color: ink(COUNTS),
                border,
                align: Align::Right,
                text: percentage(phase, progress),
            });
        }

        if let Some(current) = progress
            .current
            .as_ref()
            .filter(|_| !self.caption_leads(progress))
        {
            out.push(Draw::Text {
                x: BAR_X,
                y: CURRENT_Y,
                scale: self.text(CURRENT_SCALE),
                color: ink(CURRENT),
                border,
                align: Align::Left,
                text: elide(atlas, current, self.text(CURRENT_SCALE), BAR_WIDTH),
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
                scale: self.text(CURRENT_SCALE),
                color: ink(CURRENT),
                border,
                align: Align::Left,
                text: step,
            });
        }

        out
    }
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
/// The line height every scale below was measured against.
///
/// **Pulse's `pulse_text.fnt`, which is 13**, because this layout was authored
/// looking at Pulse's loading screen and every constant in it was chosen
/// against that face. A source whose own face is a different size is scaled
/// into this one rather than the layout being moved - see
/// [`Screen::font_scale`], which carries the three discs' numbers and why the
/// *face* is the right axis.
const AUTHORED_LINE_HEIGHT: f32 = 13.0;

/// Where a feature illustration goes: `x, y, width, height` in this screen's
/// own 480x272 grid.
///
/// Between the heading and the prose, and the picture is fitted inside it
/// rather than filling it - see [`Screen::draw_list`]. The original puts the
/// illustration left and the description right; this build's screen is one
/// column, so the two stack. That is this build's layout, as the rest of this
/// screen's layout is.
/// The original's own arrangement, in this screen's 480x272 grid.
///
/// Measured off a screenshot of a running Fury race as *fractions* of its
/// frame, then multiplied into this grid - so the proportions are the disc's
/// and the pixel values are this build's. The screenshot's own widget labels
/// (`FEATURE IMAGE`, `PROGRESSION BAR`, `MODE ICON`) are a debug overlay
/// printing widget names and are deliberately not reproduced; the marked title,
/// the rules, the two panels and the bar are the screen.
///
/// **Two columns rather than the original's exact two**, because our grid is
/// 480 wide where HD's is 1920 and prose in the right-hand half of a 480-wide
/// screen wraps to a column three words across. So the panels keep the
/// original's *relationship* (picture left, prose right of it, bar under both)
/// at proportions that stay readable here. That is this build's layout decision
/// and is the same kind `oag_game::menu::Skin` makes.
const PANEL_X: f32 = 47.0;

/// The right-hand edge every panel and rule stops at.
const PANEL_RIGHT: f32 = 432.0;

/// The rule under the title, and the one under the bar.
const RULE_TOP_Y: f32 = 52.0;
const RULE_BOTTOM_Y: f32 = 236.0;
const RULE_HEIGHT: f32 = 1.0;

/// What the rules and bracket marks are drawn in.
const RULE_INK: [f32; 4] = [0.75, 0.78, 0.82, 1.0];

/// The feature layout's bar trough.
///
/// Brighter than [`BAR_TROUGH`], which was picked for a thin rule on a near
/// black frame rather than for a gauge a fifth of the screen tall. See
/// [`Screen::draw_list`] for why this is a flat colour and not `dot.gtf`.
const BAR_DOTS: [f32; 4] = [0.20, 0.23, 0.28, 1.0];

/// The marker sprites' drawn size, and the bracket corners'.
const MARKER_SIZE: f32 = 7.0;
const CORNER_SIZE: f32 = 5.0;

/// Where the illustration goes: `x, y, width, height`.
const IMAGE_BOX: (f32, f32, f32, f32) = (PANEL_X, 60.0, 150.0, 84.0);

/// Where the feature's name and prose go, right of the picture.
const PROSE_BOX: (f32, f32, f32, f32) = (206.0, 60.0, PANEL_RIGHT - 206.0, 84.0);

/// Where the progression bar goes: `x, y, width, height`.
///
/// The original's runs most of the width with a gauge to its right; this build
/// draws the bar and not the gauge, because the gauge is a mode icon and which
/// icon a mode gets is unread.
/// The height is the original's proportion rather than a thin rule: its bar is
/// about a fifth of the frame tall, which is what makes it read as a gauge at a
/// glance instead of as another horizontal line.
const BAR_BOX: (f32, f32, f32, f32) = (PANEL_X, 168.0, 320.0, 42.0);

/// The four bracket marks round a box.
fn corners(bx: (f32, f32, f32, f32)) -> [(f32, f32); 4] {
    [
        (bx.0, bx.1),
        (bx.0 + bx.2 - CORNER_SIZE, bx.1),
        (bx.0, bx.1 + bx.3 - CORNER_SIZE),
        (bx.0 + bx.2 - CORNER_SIZE, bx.1 + bx.3 - CORNER_SIZE),
    ]
}

/// Where a feature's own name goes, above its prose.
const FEATURE_TITLE_Y: f32 = 63.0;

/// And how big. Between the heading and the prose, because it is a subheading.
const FEATURE_TITLE_SCALE: f32 = 1.1;

/// The outline drawn behind this screen's text when a backdrop is under it.
///
/// Near-black at four fifths, which is enough to lift the dim greys off a pale
/// picture without reading as a second colour. Not drawn at all over a cleared
/// frame; see [`Screen::draw_list`].
const OUTLINE: [f32; 4] = [0.0, 0.0, 0.0, 0.8];

/// What every line of text is drawn in when a backdrop is under it.
///
/// White, and one colour for all four rows rather than the four the cleared
/// frame uses. See [`Screen::draw_list`] for why the hierarchy is the thing
/// worth losing here.
const ON_BACKDROP: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const BACKDROP: [f32; 4] = [0.02, 0.03, 0.05, 1.0];

const HEADING_Y: f32 = 30.0;
const HEADING_SCALE: f32 = 1.4;
const HEADING: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

const TIP_Y: f32 = 74.0;

/// Where the prose starts on a title that draws an illustration above it.
///
/// The tip's own [`TIP_Y`] is where Pulse puts it, over a wave that occupies
/// the bottom of the frame; a feature screen has a picture and a subheading in
/// that space instead. See [`Screen::draw_list`].
const FEATURE_TIP_Y: f32 = 80.0;
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

/// And how many a feature's paragraph gets, in its own panel.
///
/// Six, which is what HD's longest - `FE_ABSORB_INST` - wraps to at
/// [`TIP_SCALE`] in [`PROSE_BOX`]'s width. Measured rather than guessed: five
/// cut it mid-sentence.
const FEATURE_TIP_LINES: usize = 6;
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

mod assets;
mod wording;
pub use assets::{Art, Assets, Feature, tips};
use wording::{counted, counts, fraction, heading, percentage, step_line};

#[cfg(test)]
mod tests;
