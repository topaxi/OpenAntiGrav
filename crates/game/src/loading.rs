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

use crate::prefetch::Progress;
use oag_display::space::SCREEN;
use oag_ui::font::{self, Atlas};
use oag_ui::frontend::{Align, Draw};
use oag_ui::language::StringTable;

/// Pulse's own two entries, re-exported for the tests and reports that name
/// them.
///
/// **Not what a loader reaches for.** Which entries a source's loading screen
/// reads is [`oag_title::Loading`]'s question, because titles answer it differently: Pure ships neither of these and
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
    /// [`oag_raceplay::LoadWorker`].
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
    /// The box of the illustration's texels that show against the screen's ground, in sheet
    /// pixels: what the panel fit measures, since the placement is padded with ground. See
    /// [`layout`].
    illustration_ink: Option<[f32; 4]>,
    title_arrow: Option<[f32; 4]>,
    subtitle_arrow: Option<[f32; 4]>,
    rule: Option<[f32; 4]>,
    corner: Option<[f32; 4]>,
    /// The bar's dot, tiled across it, and the bullet before each label.
    dot: Option<[f32; 4]>,
    square: Option<[f32; 4]>,
    /// The four colours this source tints the screen with, when it names them.
    ///
    /// `None` keeps the palette this build chose against a near-black frame,
    /// which is what both PSP titles get. See [`Assets::palette`].
    palette: Option<assets::Palette>,
    /// The circuit's name a track supplied for the caption line, instead of the one the load
    /// reports. See [`track`].
    circuit: Option<String>,
    /// The slot of the feature on show in the title's own table, which is the number the
    /// original's `Feature type == N` line reports.
    feature_slot: Option<u8>,
    /// The feature's own name, when its id was recovered.
    feature_title: Option<String>,
    /// The heading over the paragraph, drawn in the row the name shares.
    feature_heading: Option<String>,
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
    /// loading screen's own version of the bug `oag_ui::menu::Skin` fixed for
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
    /// Whether the mode icon's clip has a picture to show: set by whoever
    /// feeds the clip, once its first frame is in the renderer's planes.
    mode_icon: bool,
    /// The bar's stage-driven fill, on a title whose loader drives it. `None`
    /// keeps the time estimate. See [`fill::Fill`].
    fill: Option<fill::Fill>,
    /// This title's loading layout file, when it names one that loaded. See [`layout`].
    layouts: Option<oag_hud::loading_layout::LoadingLayouts>,
    /// The strings the layout's labels name, resolved. See [`Assets::layout_strings`].
    layout_strings: Vec<(String, String)>,
    /// `oag_race::Mode::name` of the race the screen covers, which picks the layout.
    mode_name: Option<&'static str>,
    /// The player's Safe Area as a scale about the screen's centre.
    safe_area: f32,
    /// The surface's width over height, which picks among a file's layouts.
    surface_aspect: f32,
    /// This project's own strings, layered over nothing: this screen runs
    /// before any disc's language plugin is read - `heading`'s "READING THE
    /// ARCHIVES" case fires while that read is still in flight - so there is
    /// no disc-merged table to build this from, only [`oag_ui::strings::project_table`].
    /// Built once, here, rather than per line: the same reason
    /// `prepare::definition` builds its own copy once rather than at every
    /// `resolve()` call.
    strings: StringTable,
}

impl Screen {
    /// A screen at the start of its first beat, showing whatever the source
    /// gave it.
    ///
    /// Takes the whole [`Assets`] rather than the tips alone since the title
    /// axis landed: a backdrop, a caption and whether there is a wave at all
    /// are all per-title now, and threading four arguments through every caller
    /// would put the same four together at each of them.
    ///
    /// `language` names which of this project's own string files to overlay -
    /// see the `strings` field's own doc - and is `Option<&str>` rather than a
    /// [`StringTable`] so a caller with no disc open yet (every caller of this
    /// constructor) does not have to build one itself; `None` falls back to
    /// English, [`oag_ui::strings::project_table`]'s own fallback.
    #[must_use]
    pub fn new(assets: &Assets, line_height: f32, draw: u64, language: Option<&str>) -> Self {
        Self::for_mode(assets, line_height, draw, language, None)
    }

    /// The same, covering a race in `mode` - the source executable's own mode
    /// id - which narrows the feature draw to that mode's deck.
    ///
    /// **One feature, drawn per screen from the mode's own deck**, as the
    /// original's `LoadingScreen_Construct` does; see [`feature::pick`] and
    /// `docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md`. `draw` is a seed
    /// the caller varies per screen, not a clock read.
    #[must_use]
    pub fn for_mode(
        assets: &Assets,
        line_height: f32,
        draw: u64,
        language: Option<&str>,
        mode: Option<u32>,
    ) -> Self {
        let shown = feature::pick(assets, mode, draw);
        Self {
            wave: Wave::new(),
            has_wave: assets.wave,
            rng: oag_core::rng::Rng::new(WAVE_SEED),
            tips: assets.tips.clone(),
            illustration: shown
                .and_then(|at| {
                    assets
                        .art
                        .as_ref()
                        .and_then(|art| art.illustrations.get(at))
                })
                .copied(),
            illustration_ink: shown.and_then(|at| {
                let art = assets.art.as_ref()?;
                let ground = assets
                    .palette
                    .map_or(BACKDROP, |palette| palette.background);
                art.sheet.ink_within(*art.illustrations.get(at)?, |texel| {
                    shows_against(texel, ground)
                })
            }),
            title_arrow: assets.art.as_ref().and_then(|art| art.title_arrow),
            subtitle_arrow: assets.art.as_ref().and_then(|art| art.subtitle_arrow),
            rule: assets.art.as_ref().and_then(|art| art.rule),
            corner: assets.art.as_ref().and_then(|art| art.corner),
            dot: assets.art.as_ref().and_then(|art| art.dot),
            square: assets.art.as_ref().and_then(|art| art.square),
            palette: assets.palette,
            circuit: None,
            feature_slot: shown
                .and_then(|at| assets.features.get(at))
                .map(|feature| feature.slot),
            feature_title: shown
                .and_then(|at| assets.features.get(at))
                .and_then(|feature| feature.title.clone()),
            feature_heading: shown
                .and_then(|at| assets.features.get(at))
                .and_then(|feature| feature.heading.clone()),
            feature_text: shown
                .and_then(|at| assets.features.get(at))
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
            mode_icon: false,
            fill: assets.progression.map(fill::Fill::new),
            layouts: assets.layout.clone(),
            layout_strings: assets.layout_strings.clone(),
            mode_name: mode.and_then(feature::mode_name),
            safe_area: 1.0,
            surface_aspect: 16.0 / 9.0,
            strings: oag_ui::strings::project_table(language),
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
        if let Some(fill) = self.fill.as_mut() {
            fill.step();
        }
        if finished {
            self.wave.finish();
            self.fade = self.fade.saturating_add(1);
        } else {
            self.wave.advance();
        }
    }

    /// Frames since the screen opened: the playhead of the mode icon's clip,
    /// which runs at the screen's own 60 Hz.
    #[must_use]
    pub fn playhead(&self) -> u64 {
        u64::from(self.frames)
    }

    /// Tells the bar which stage the race load has reached, for a title whose
    /// bar the loader drives. Ignored on one that has no such bar.
    pub fn set_load_stage(&mut self, stage: u8) {
        if let Some(fill) = self.fill.as_mut() {
            fill.reach(stage);
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
    /// [`TIP_FRAMES`]; a feature does not rotate: one is drawn per screen from
    /// the mode's deck - see [`feature::pick`].
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

    /// The title's own slot of the feature on show: what the original's constructor logs as
    /// `Feature type == N` for the same feature. `None` on a title with no features.
    #[must_use]
    pub fn feature_slot(&self) -> Option<u8> {
        self.feature_slot
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
            return heading(phase, progress, &self.strings);
        }
        let caption = self.caption.clone().unwrap_or_default();
        match self.circuit.as_ref().or(progress.current.as_ref()) {
            Some(current) => format!("{caption} {current}"),
            None => caption,
        }
    }

    /// The whole screen except the wave, in the PSP's 480x272 screen space.
    ///
    /// The wave is not in here because it is not a [`Draw`]: it has its own
    /// pipeline and its own additive blend, and is drawn over this list in a
    /// second pass. See [`oag_render::loading::Pipeline`].
    ///
    /// **A title with art for its own arrangement is drawn from its layout file**
    /// ([`layout`]): Wipeout HD's marked title over a rule, its illustration and prose, its
    /// bar and mode icon, none of which is placed here. What this function lays out is the
    /// centred screen both PSP discs have always drawn, which is this build's own.
    #[must_use]
    pub fn draw_list(&self, phase: Phase, progress: &Progress, atlas: &Atlas) -> Vec<Draw> {
        if self.is_laid_out() {
            return self.laid_out_list(phase, progress, atlas);
        }
        let mut out = Vec::new();
        let fade = self.opacity();
        let dim = |colour: [f32; 4]| [colour[0], colour[1], colour[2], colour[3] * fade];
        // **The disc's own tint where this source names one.** The four globals the original's
        // loading screen resolves are read out of its own `FEGlobals`, so a screen drawn here is
        // the colour that disc draws it. A title that names none keeps what it always drew.
        let tint = self.palette;
        let ink = |colour: [f32; 4]| match tint {
            Some(palette) => dim(palette.ink),
            None => dim(colour),
        };

        // The frame is cleared either way, so a hole in the list never shows as whatever the
        // last stage left behind.
        out.push(Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: tint.map_or(BACKDROP, |palette| dim(palette.background)),
        });

        out.push(Draw::Text {
            x: SCREEN.0 / 2.0,
            y: HEADING_Y,
            scale: self.text(HEADING_SCALE),
            color: ink(HEADING),
            border: None,
            align: Align::Centre,
            // **The caption and what is loading on one line**, which is how the original writes
            // it: a title with a caption puts them together and draws no separate line below; a
            // title without one keeps this build's heading and its own `current` row. A caption
            // never replaces the *counted* headings, which say something the disc's single
            // string cannot - see `heading`.
            text: self.heading_text(phase, progress),
            wrap_width: None,
        });

        if let Some(tip) = self.tip() {
            for (row, line) in wrap(atlas, tip, self.text(TIP_SCALE), TIP_WIDTH)
                .into_iter()
                .take(TIP_LINES)
                .enumerate()
            {
                out.push(Draw::Text {
                    x: SCREEN.0 / 2.0,
                    y: TIP_Y + row as f32 * TIP_LINE_HEIGHT,
                    scale: self.text(TIP_SCALE),
                    color: ink(TIP_COLOUR),
                    border: None,
                    align: Align::Centre,
                    text: line,
                    wrap_width: None,
                });
            }
        }

        // The bar and the counts say the same thing twice on purpose: the bar is what is read at
        // a glance across ten minutes, and the figures are what distinguish "slow" from "stuck"
        // when the bar has not visibly moved. All of it together, or none of it: see [`counted`].
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
                border: None,
                align: Align::Left,
                text: counts(phase, progress, &self.strings),
                wrap_width: None,
            });
            out.push(Draw::Text {
                x: BAR_X + BAR_WIDTH,
                y: COUNTS_Y,
                scale: self.text(COUNTS_SCALE),
                color: ink(COUNTS),
                border: None,
                align: Align::Right,
                text: percentage(phase, progress),
                wrap_width: None,
            });
        }

        if let Some(current) = progress
            .current
            .as_ref()
            .filter(|_| !self.caption_leads(progress))
        {
            out.push(Draw::Text {
                x: BAR_X,
                y: COUNTS_Y + CENTRED_ROW_PITCH,
                scale: self.text(CURRENT_SCALE),
                color: ink(CURRENT),
                border: None,
                align: Align::Left,
                text: elide(atlas, current, self.text(CURRENT_SCALE), BAR_WIDTH),
                wrap_width: None,
            });
        }

        // A line of its own rather than a suffix on the one above, because `elide` keeps a
        // string's *tail*: appended, the state would survive and the entry name would be eaten
        // from the left, which is the wrong half to lose.
        if let Some(step) = step_line(phase, &self.strings) {
            out.push(Draw::Text {
                x: BAR_X,
                y: COUNTS_Y + CENTRED_ROW_PITCH * 2.0,
                scale: self.text(CURRENT_SCALE),
                color: ink(CURRENT),
                border: None,
                align: Align::Left,
                text: step,
                wrap_width: None,
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

/// The dots of an unlit bar when the title names no palette.
const BAR_DOTS: [f32; 4] = [0.20, 0.23, 0.28, 1.0];

/// The ink a laid-out screen draws in when its title names no palette.
const ON_BACKDROP: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// The most a texel's channel may differ from the ground, out of 255, and still read as the
/// ground. **Chosen, not measured.**
const GROUND_TOLERANCE: f32 = 16.0;

/// Whether a texel changes what the ground shows: translucent texels and ones that are the
/// ground's own colour do not.
fn shows_against(texel: [u8; 4], ground: [f32; 4]) -> bool {
    texel[3] != 0
        && texel.iter().zip(ground).take(3).any(|(&channel, ground)| {
            (f32::from(channel) - ground * 255.0).abs() > GROUND_TOLERANCE
        })
}

const BACKDROP: [f32; 4] = [0.02, 0.03, 0.05, 1.0];

const HEADING_Y: f32 = 30.0;
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

const CURRENT_SCALE: f32 = 0.8;
const CURRENT: [f32; 4] = [0.5, 0.58, 0.66, 1.0];

/// How far apart the counted rows sit on the centred layout.
///
/// The gap `CURRENT_Y` was defined at, kept as the pitch now that both
/// layouts space their three rows the same way from their own first one.
const CENTRED_ROW_PITCH: f32 = 16.0;

mod assets;
mod feature;
pub mod icon;
mod layout;
pub mod track;
mod wording;
pub use assets::{Art, Assets, Feature, Labels, ModeIcon, tips};
pub use feature::executable_mode;
pub use track::TrackLoading;
use wording::{counted, counts, estimated_fraction, fraction, heading, percentage, step_line};

#[cfg(test)]
mod deck_tests;
mod fill;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod track_tests;
