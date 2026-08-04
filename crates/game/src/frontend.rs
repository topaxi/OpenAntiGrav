//! The boot sequence: the intro movie, then the language picker.
//!
//! This is the part worth testing, so it holds no GPU handles, opens no files
//! and reads no clock. It takes a delta and an input snapshot and emits a draw
//! list. `main.rs` supplies the first two and rasterises the third.
//!
//! # What it reproduces
//!
//! There are **two** movie legs here, and only one of them is on the disc's own
//! boot path.
//!
//! [`Leg::LogoFmv`] is the one that is, and it is the default. `LogoFMV` is a
//! screen in the disc's front-end XML: a black `Image`, a `Movie` widget with
//! `src="Data\Movies\Intro"`, `autostart`, `repeat="false"` and `autoredirect`,
//! an `AutoRedirect` to its own `Show Logo` child, and five more redirects -
//! start, circle, square, triangle and cross - all going to
//! `LogoFMVRedirectScreen`. So the movie plays straight through and any of five
//! buttons skips it. **There are no frame holds on this leg**, because the
//! widget has no frame counters: it plays a 1200-frame movie to its end.
//!
//! [`Leg::DevPubReel`] is `Intro Screen->IntroMovie1` at `0x088d7e1c`, whose
//! `OnEnter` caches two transition targets by name, `"DevPubRedirect"` and
//! `"Intro Screen->IntroMovie1"`. That state paces playback against the movie's
//! own frame counter: pause at 144 and at 231, set the finish flag at 260, each
//! pause released after two seconds, and the finish flag fires
//! `DevPubRedirect`. START skips by firing that same redirect rather than by
//! stopping the player, so the teardown is a consequence of the transition. The
//! 260 is not arbitrary and it is not ours: three movies in `Data.wad` are
//! exactly 260 frames long and are static at exactly 144 and 231.
//!
//! **The disc's own boot never enters that state.** Run under PPSSPP from a cold
//! boot with `0x088d7d80` armed from reset, `IntroMovie1`'s `OnEnter` does not
//! fire in ten minutes; the only movies opened are `Data\Movies\Intro.PMF` and
//! `Data\Movies\Backdrop.PMF`. So the reel leg is real, evidenced code whose
//! trigger is simply unknown, and it is reachable here through `--reel` rather
//! than at boot. See `docs/ghidra/functions/psp-pulse-usa/frontend-video.md` and
//! `docs/architecture/frontend-boot.md`.
//!
//! # Where it knowingly differs
//!
//! The front-end XML runs `Language Selection` **first**, and its
//! `LanguageAutoRedirect` goes on to `LogoFMV`. This build boots into `LogoFMV`
//! and lands on the picker, because that is the order asked for; the XML's own
//! order is recorded in [`Frontend::language_auto_redirect`] and reported at
//! startup so the difference is visible rather than buried.
//!
//! `LogoFMV`'s own exits also differ. On the disc the movie finishing fires
//! `AutoRedirect` to `LogoFMV->Show Logo` - the Pulse logo and PRESS START - and
//! a button fires `LogoFMVRedirectScreen`, which goes to the same place. Here
//! both go to the picker, because the picker is what this build has next.
//!
//! And [`states::LAUNCH_GAME`] goes straight into a race. The original has a main
//! menu in between - the root XML's `LoadXML` list pulls in
//! `MainMenu_Definition.xml` - and none of it is built, so a state whose whole
//! content was the words LAUNCH GAME is used as the way in instead. This module
//! does not know that: it fires the transition the original's own literal names
//! and stops, and what happens next is the composition root's business.

use crate::input::{Input, button};
use crate::language::{Language, StringTable};
use crate::screen::{Screen, Screens, argb_to_rgba};
use crate::state_machine::{Event, StateMachine};

/// Frame counts at which the dev/pub reel state acts, from `0x088d7e1c`.
pub const PAUSE_FRAMES: [usize; 2] = [144, 231];
/// The frame at which the dev/pub reel state sets its finish flag.
pub const FINISH_FRAME: usize = 260;
/// How long a pause is held, in seconds.
pub const HOLD_SECONDS: f64 = 2.0;

/// State names this build drives. Every one is a literal from the original -
/// from its executable, or from the front-end XML on the disc.
pub mod states {
    /// The screen that plays `Data\Movies\Intro.PMF`, named by the front-end
    /// XML. This is the leg the disc's own boot runs.
    pub const LOGO_FMV: &str = "LogoFMV";
    /// The dev/pub reel's parent state.
    pub const INTRO: &str = "Intro Screen";
    /// The dev/pub reel state, spelled the way the original's own string
    /// literal is. Not on the disc's boot path; see the module docs.
    pub const INTRO_MOVIE: &str = "Intro Screen->IntroMovie1";
    /// The one-shot redirect the reel state fires when it finishes or is
    /// skipped.
    pub const DEV_PUB_REDIRECT: &str = "DevPubRedirect";
    /// The language picker.
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    /// Where picking a language goes, and where a race starts.
    pub const LAUNCH_GAME: &str = "Launch Game";
}

/// Which movie leg the sequence boots into.
///
/// See the module docs: one of these is on the disc's boot path and the other
/// is a state whose trigger nobody has found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Leg {
    /// `LogoFMV`, playing `Data\Movies\Intro.PMF` straight through.
    #[default]
    LogoFmv,
    /// `Intro Screen->IntroMovie1`, with the frame-counted holds at 144, 231
    /// and 260. Reached through `--reel`.
    DevPubReel,
}

impl Leg {
    /// The state this leg starts in.
    #[must_use]
    pub fn state(self) -> &'static str {
        match self {
            Self::LogoFmv => states::LOGO_FMV,
            Self::DevPubReel => states::INTRO_MOVIE,
        }
    }
}

/// Horizontal alignment, as the XML spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// `align="left"`, the default.
    Left,
    /// `align="centre"`.
    Centre,
    /// `align="right"`.
    Right,
}

impl Align {
    /// Reads the XML's own spelling, tolerating both `centre` and `center`.
    ///
    /// Public because the HUD layouts spell alignment the same way this screen's
    /// widgets do, and one reading of the attribute is better than two that can
    /// disagree. See [`crate::hud`].
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "centre" | "center" => Self::Centre,
            "right" => Self::Right,
            _ => Self::Left,
        }
    }
}

/// One thing to draw, in the PSP's 480x272 screen space.
#[derive(Debug, Clone, PartialEq)]
pub enum Draw {
    /// A solid rectangle: `[x, y, width, height]` and RGBA.
    Fill {
        /// Rectangle.
        rect: [f32; 4],
        /// Colour.
        color: [f32; 4],
    },
    /// The movie's current frame, stretched to `rect`.
    Video {
        /// Rectangle.
        rect: [f32; 4],
        /// Which frame to show.
        frame: usize,
    },
    /// One of the front end's own images, from the sprite sheet.
    Sprite {
        /// Where it goes on the 480x272 screen: `[x, y, width, height]`.
        rect: [f32; 4],
        /// Where it is in the sheet, in sheet pixels: `[x, y, width, height]`.
        uv: [f32; 4],
        /// Modulating colour. White leaves the texture alone.
        color: [f32; 4],
    },
    /// A line of text with its baseline-less top-left at `x, y`.
    Text {
        /// Left or anchor edge, depending on `align`.
        x: f32,
        /// Top edge.
        y: f32,
        /// Scale multiplier applied to the glyph cell.
        scale: f32,
        /// Colour of the glyph body.
        color: [f32; 4],
        /// Colour of the glyph's **baked outline**, when the font has one.
        ///
        /// `None` means "the same as the body", which is what every caller wanted
        /// before the two HUD fonts turned up: those two bake an outline into their
        /// atlas and distinguish it only by grey level, so drawing them without a
        /// separate border colour fills the whole silhouette and a digit becomes a
        /// box. The three menu fonts and the built-in glyphs carry a constant mask,
        /// so this changes nothing for them whatever it is set to. See
        /// [`crate::font::Atlas::luma`].
        border: Option<[f32; 4]>,
        /// Alignment about `x`.
        align: Align,
        /// The text.
        text: String,
    },
}

/// The PSP's screen, which the XML's coordinates are in.
pub const SCREEN: (f32, f32) = (480.0, 272.0);

/// Fits `aspect` inside `screen` without distorting it, centred on whichever
/// axis is left over.
///
/// A `.PMF`'s `aspect` is its own decoded size, which is [`SCREEN`]'s own
/// ratio, so this returns `[0, 0, screen.0, screen.1]` unchanged - the video
/// quad this always drew. A PS2 `.PSS`'s `aspect` is not: `INTRO512.PSS`
/// decodes to a square 512x512 but its own display aspect is 4:3, narrower
/// than `SCREEN`'s ~16:9, so this pillarboxes it rather than stretching it
/// wide. See `crate::movie::Movie::display_aspect`.
pub fn pillarbox(screen: (f32, f32), aspect: (u32, u32)) -> [f32; 4] {
    let (screen_w, screen_h) = screen;
    let content = aspect.0 as f32 / aspect.1 as f32;
    let frame = screen_w / screen_h;

    if content >= frame {
        let height = screen_w / content;
        [0.0, (screen_h - height) / 2.0, screen_w, height]
    } else {
        let width = screen_h * content;
        [(screen_w - width) / 2.0, 0.0, width, screen_h]
    }
}

/// How the intro state is getting on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    /// Playing.
    None,
    /// Paused, with the hold's start time stamped.
    Held {
        /// Whether releasing this hold also fires the redirect.
        ///
        /// The original stamps the time for all three frame counters and sets a
        /// separate finish flag at 260. The flag fires the redirect, so the last
        /// card is held for its two seconds like the other two.
        finish: bool,
    },
}

/// The boot sequence.
#[derive(Debug)]
pub struct Frontend {
    machine: StateMachine,
    screens: Screens,
    strings: StringTable,
    languages: Vec<Language>,
    selected: usize,
    /// The picked language, once one is picked.
    chosen: Option<String>,
    /// Set by [`Frontend::preselect_language`]: take the highlighted language
    /// the frame the picker is entered instead of waiting for a press.
    auto_confirm: bool,
    /// Where each referenced image sits in the sprite sheet.
    ///
    /// The sheet's pixels live in the renderer; the model only needs to know
    /// each image's size and offset, which is what turns an `Image` widget into
    /// a rectangle. A `Vec` keeps the order the screens declared.
    placements: Vec<(String, crate::sprite::Placed)>,
    player: crate::movie::Player,
    /// The movie's own display aspect ratio, as `(width, height)`. A `.PMF` is
    /// square-pixelled, so this is its decoded size and the video quad fills
    /// [`SCREEN`] exactly, as it always did; a PS2 `.PSS` is not, and this is
    /// what keeps its picture from being stretched into the wrong aspect. See
    /// `crate::movie::Movie::display_aspect`.
    video_aspect: (u32, u32),
    /// Whether a picture is available at all.
    has_picture: bool,
    /// Whether to draw the frame counter over the intro.
    overlay: bool,
    hold: Hold,
    held_for: f64,
    /// Cached transition target, cleared once fired. The original keeps this on
    /// the state and zeroes it after firing, so a second START does nothing.
    dev_pub_redirect: Option<String>,
    /// Frame counts already acted on, so a pause cannot re-trigger.
    acted: Vec<usize>,
    /// Lines worth telling the user about, in order.
    notes: Vec<String>,
    finished: bool,
}

impl Frontend {
    /// Builds the boot sequence on the disc's own movie leg.
    ///
    /// `movie_frames` is how many frames the movie has. Zero is legal and means
    /// there is no movie at all, which still plays out a sequence.
    #[must_use]
    pub fn new(
        screens: Screens,
        strings: StringTable,
        languages: Vec<Language>,
        placements: Vec<(String, crate::sprite::Placed)>,
        movie_frames: usize,
        has_picture: bool,
    ) -> Self {
        Self::booting(
            Leg::default(),
            screens,
            strings,
            languages,
            placements,
            movie_frames,
            crate::movie::FRAME_RATE,
            (SCREEN.0 as u32, SCREEN.1 as u32),
            has_picture,
        )
    }

    /// Builds the boot sequence on a named [`Leg`].
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn booting(
        leg: Leg,
        screens: Screens,
        strings: StringTable,
        languages: Vec<Language>,
        placements: Vec<(String, crate::sprite::Placed)>,
        movie_frames: usize,
        movie_frame_rate: (u64, u64),
        video_aspect: (u32, u32),
        has_picture: bool,
    ) -> Self {
        let mut machine = StateMachine::new();
        machine.register_all([
            states::LOGO_FMV,
            states::INTRO,
            states::INTRO_MOVIE,
            states::DEV_PUB_REDIRECT,
            states::LANGUAGE_SELECTION,
            states::LAUNCH_GAME,
        ]);
        // Every screen the XML declares becomes a state, so a transition name
        // recovered from the data resolves without being listed here.
        let paths: Vec<String> = screens.screens.iter().map(|s| s.path.clone()).collect();
        machine.register_all(paths.iter().map(String::as_str));

        // With no movie at all - a source with nothing this build can read a
        // picture out of - the sequence still has to run and end. FINISH_FRAME
        // is the reel's own length, which is as good a stand-in as any and is
        // bounded.
        let frames = if movie_frames == 0 {
            FINISH_FRAME
        } else {
            movie_frames
        };

        let mut frontend = Self {
            machine,
            screens,
            strings,
            languages,
            selected: 0,
            chosen: None,
            auto_confirm: false,
            placements,
            player: crate::movie::Player::new(frames, false, movie_frame_rate),
            video_aspect,
            has_picture,
            // Without a picture the movie is a black screen for as long as it
            // runs - forty seconds for the disc's own intro - so the counter is
            // the only sign it is running. With one it is clutter, and the
            // pacing can be read off the picture instead.
            overlay: !has_picture,
            hold: Hold::None,
            held_for: 0.0,
            dev_pub_redirect: Some(states::DEV_PUB_REDIRECT.to_string()),
            acted: Vec::new(),
            notes: Vec::new(),
            finished: false,
        };
        frontend.machine.transition_to(leg.state());
        frontend
    }

    /// Draws or hides the intro's frame counter.
    pub fn set_overlay(&mut self, on: bool) {
        self.overlay = on;
    }

    /// The state machine, for tests and tracing.
    #[must_use]
    pub fn machine(&self) -> &StateMachine {
        &self.machine
    }

    /// The front-end model this was built from.
    #[must_use]
    pub fn screens(&self) -> &Screens {
        &self.screens
    }

    /// The languages the disc offers, in plugin order.
    #[must_use]
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    /// Which language is highlighted.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The language that was picked, once one is.
    #[must_use]
    pub fn chosen(&self) -> Option<&str> {
        self.chosen.as_deref()
    }

    /// Whether the sequence has reached `Launch Game`.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Where the XML's own `LanguageAutoRedirect` goes.
    ///
    /// `LogoFMV` on a real disc, which is the evidence that the original runs
    /// the picker before the named intro movie rather than after it.
    #[must_use]
    pub fn language_auto_redirect(&self) -> Option<&str> {
        self.screens
            .language_selection()?
            .redirect_named("LanguageAutoRedirect")?
            .goto
            .as_deref()
    }

    /// Notes accumulated since the last call, and clears them.
    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }

    /// The movie player, for tests.
    #[must_use]
    pub fn player(&self) -> &crate::movie::Player {
        &self.player
    }

    /// Steps the sequence by `dt` seconds.
    ///
    /// Returns the transitions that happened, in order, so a caller can log them
    /// or a test can assert on them.
    pub fn update(&mut self, dt: f64, input: &mut Input) -> Vec<Event> {
        if self.machine.is(states::LOGO_FMV) {
            self.update_logo_fmv(dt, input);
        } else if self.machine.is(states::INTRO_MOVIE) {
            self.update_intro(dt, input);
        } else if self.machine.is(states::DEV_PUB_REDIRECT) {
            // A redirect state's whole job is to leave. The front-end XML does
            // the same thing with `LogoFMVRedirectScreen`.
            self.machine.fire(states::LANGUAGE_SELECTION);
        } else if self.machine.is(states::LANGUAGE_SELECTION) {
            self.update_language_selection(input);
        }

        let events = self.machine.apply();
        for event in &events {
            if let Event::Enter(name) = event
                && name == states::LAUNCH_GAME
            {
                self.finished = true;
            }
        }
        events
    }

    /// The disc's own movie leg: play `Data\Movies\Intro.PMF` to its end, or
    /// until a button skips it.
    ///
    /// No frame counters and no holds. The `Movie` widget carries none: it is
    /// `autostart`, `repeat="false"`, `autoredirect`, and the screen's five
    /// skip redirects - start, circle, square, triangle, cross - all go to
    /// `LogoFMVRedirectScreen`. Only the buttons the abstract layer carries are
    /// read here, which is start and cross.
    fn update_logo_fmv(&mut self, dt: f64, input: &mut Input) {
        for button in [button::START, button::CROSS] {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.notes.push(format!(
                    "the movie was skipped, firing {}",
                    states::LANGUAGE_SELECTION
                ));
                self.machine.fire(states::LANGUAGE_SELECTION);
                return;
            }
        }

        self.player.update(dt);
        if self.player.is_finished() {
            // The disc's `AutoRedirect` goes to `LogoFMV->Show Logo` here. This
            // build has the picker next instead; see the module docs.
            self.notes.push(format!(
                "the movie ended, firing {}",
                states::LANGUAGE_SELECTION
            ));
            self.machine.fire(states::LANGUAGE_SELECTION);
        }
    }

    fn update_intro(&mut self, dt: f64, input: &mut Input) {
        // The skip. The player never reads the pad; the state does, and it fires
        // the cached redirect rather than stopping playback.
        if input.is_pressed(button::START)
            && let Some(target) = self.dev_pub_redirect.take()
        {
            self.notes
                .push(format!("START skipped the intro, firing {target}"));
            self.machine.fire(&target);
            input.consume_press(button::START);
            return;
        }

        if let Hold::Held { finish } = self.hold {
            self.held_for += dt;
            if self.held_for >= HOLD_SECONDS {
                self.hold = Hold::None;
                self.held_for = 0.0;
                self.player.resume();
                if finish && let Some(target) = self.dev_pub_redirect.take() {
                    self.notes.push(format!("intro finished, firing {target}"));
                    self.machine.fire(&target);
                }
            }
            return;
        }

        self.player.update(dt);
        let produced = self.player.frames_produced();

        if produced >= FINISH_FRAME && !self.acted.contains(&FINISH_FRAME) {
            self.acted.push(FINISH_FRAME);
            self.begin_hold(true);
            self.notes.push(format!(
                "intro reached frame {FINISH_FRAME}, holding {HOLD_SECONDS}s then finishing"
            ));
            return;
        }

        for &frame in &PAUSE_FRAMES {
            if produced >= frame && !self.acted.contains(&frame) {
                self.acted.push(frame);
                self.begin_hold(false);
                self.notes
                    .push(format!("intro paused at frame {frame} for {HOLD_SECONDS}s"));
                return;
            }
        }

        // A reel shorter than 260 frames still has to end.
        if self.player.is_finished()
            && let Some(target) = self.dev_pub_redirect.take()
        {
            self.notes.push("intro movie ended".to_string());
            self.machine.fire(&target);
        }
    }

    fn begin_hold(&mut self, finish: bool) {
        self.hold = Hold::Held { finish };
        self.held_for = 0.0;
        self.player.pause();
    }

    fn update_language_selection(&mut self, input: &mut Input) {
        if self.languages.is_empty() {
            return;
        }
        // A language already chosen on a previous run leaves the picker the
        // frame it is entered. The state is still entered rather than skipped
        // in the machine, so the transition sequence a test asserts on is the
        // same one; what changes is that nobody has to press anything.
        if std::mem::take(&mut self.auto_confirm) {
            self.confirm_language();
            return;
        }

        let count = self.languages.len();
        if input.is_pressed(button::DOWN) {
            input.consume_press(button::DOWN);
            self.selected = (self.selected + 1) % count;
        } else if input.is_pressed(button::UP) {
            input.consume_press(button::UP);
            self.selected = (self.selected + count - 1) % count;
        } else if input.is_pressed(button::CROSS) {
            input.consume_press(button::CROSS);
            self.confirm_language();
        }
    }

    /// Takes the highlighted language and leaves for `Launch Game`.
    fn confirm_language(&mut self) {
        let language = self.languages[self.selected].clone();
        let disc_goto = self.language_auto_redirect().map(str::to_string);
        self.chosen = Some(language.name.clone());
        self.notes.push(format!(
            "language {} ({}) selected, firing {}",
            language.name,
            language.native_name,
            states::LAUNCH_GAME
        ));
        if let Some(goto) = disc_goto {
            self.notes.push(format!(
                "note: the disc's own LanguageAutoRedirect goes to {goto}, not {}",
                states::LAUNCH_GAME
            ));
        }
        self.machine.fire(states::LAUNCH_GAME);
    }

    /// Chooses `name` without showing the picker, if this source offers it.
    ///
    /// Returns whether the name matched. `false` is the case that matters: a
    /// settings file naming a language this disc does not carry - a EU save
    /// opened against the USA release, which ships English only - must fall back
    /// to asking rather than silently racing in the wrong language or refusing
    /// to boot. The caller reports it; nothing here fails.
    ///
    /// Matched on the XML's English name (`French`), which is what
    /// [`Self::chosen`] returns and so what a settings file will have in it.
    /// Case-insensitively, because a hand-edited config is a hand-edited config.
    pub fn preselect_language(&mut self, name: &str) -> bool {
        let Some(at) = self
            .languages
            .iter()
            .position(|language| language.name.eq_ignore_ascii_case(name))
        else {
            return false;
        };
        self.selected = at;
        self.auto_confirm = true;
        true
    }

    /// What to draw this frame.
    #[must_use]
    pub fn draw_list(&self) -> Vec<Draw> {
        let (width, height) = SCREEN;
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, width, height],
            color: [0.0, 0.0, 0.0, 1.0],
        }];

        if self.machine.is_in(states::INTRO) || self.machine.is(states::LOGO_FMV) {
            if self.has_picture {
                out.push(Draw::Video {
                    rect: pillarbox(SCREEN, self.video_aspect),
                    frame: self.player.frame(),
                });
            }
            if self.overlay {
                out.push(Draw::Text {
                    x: 8.0,
                    y: 250.0,
                    scale: 1.0,
                    color: [1.0, 1.0, 1.0, 0.5],
                    border: None,
                    align: Align::Left,
                    text: format!(
                        "INTRO FRAME {} / {}{}",
                        self.player.frames_produced(),
                        self.player.frames(),
                        if self.has_picture {
                            ""
                        } else {
                            " (NO PICTURE)"
                        }
                    ),
                });
            }
            return out;
        }

        if self.machine.is(states::LANGUAGE_SELECTION) {
            self.draw_language_selection(&mut out);
            return out;
        }

        if self.machine.is(states::LAUNCH_GAME) {
            out.push(Draw::Text {
                x: width / 2.0,
                y: height / 2.0 - 4.0,
                scale: 2.0,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Centre,
                text: states::LAUNCH_GAME.to_ascii_uppercase(),
            });
        }
        out
    }

    /// A named screen's own widgets, drawn from the XML alone.
    ///
    /// This is what the disc's data describes and nothing else: no state, no
    /// input, no animation. `--screen` renders through it, which is how the
    /// image path is checked without moving any screen into the boot order.
    #[must_use]
    pub fn draw_screen(&self, name: &str) -> Vec<Draw> {
        let (width, height) = SCREEN;
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, width, height],
            color: [0.0, 0.0, 0.0, 1.0],
        }];

        let Some(screen) = self
            .screens
            .screens
            .iter()
            .find(|s| s.name == name || s.path == name)
        else {
            return out;
        };

        self.draw_backdrops(screen, &mut out);
        for text in &screen.texts {
            let Some(id) = text.idstring.as_deref().or(text.string.as_deref()) else {
                continue;
            };
            out.push(Draw::Text {
                x: text.x,
                y: text.y,
                scale: text.scale,
                color: argb_to_rgba(text.color),
                border: None,
                align: Align::parse(&text.align),
                text: self.strings.get_or_id(id).to_string(),
            });
        }
        out
    }

    /// A screen's solid backdrops and its images, in that order.
    fn draw_backdrops(&self, screen: &Screen, out: &mut Vec<Draw>) {
        for &fill in &screen.fills {
            out.push(Draw::Fill {
                rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
                color: argb_to_rgba(fill),
            });
        }

        for image in &screen.images {
            let Some(placed) = self
                .placements
                .iter()
                .find(|(src, _)| *src == image.src)
                .map(|(_, placed)| *placed)
            else {
                continue;
            };

            // An `Image` with no `x` is centred rather than pinned to the left
            // edge. `Show Logo`'s logo is a 512-wide texture on a 480-wide
            // screen, so the two readings differ by 16 pixels, and the texture
            // itself settles which: its art is transparent out to column 23 on
            // the left and from 488 on the right, so it is authored centred
            // inside its own 512. Centred on screen the art lands at 7..471 with
            // even margins; pinned at `x = 0` it would run to 487 and lose its
            // right edge. Confidence 85 - measured from the one image there is,
            // not read out of the widget's layout code.
            // See `docs/architecture/frontend-boot.md`.
            let w = image.width.unwrap_or(placed.width as f32);
            let h = image.height.unwrap_or(placed.height as f32);
            let x = if image.x == 0.0 {
                (SCREEN.0 - w) / 2.0
            } else {
                image.x
            };

            out.push(Draw::Sprite {
                rect: [x, image.y, w, h],
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                color: argb_to_rgba(image.color),
            });
        }
    }

    fn draw_language_selection(&self, out: &mut Vec<Draw>) {
        let Some(screen) = self.screens.language_selection() else {
            return;
        };

        self.draw_backdrops(screen, out);

        // The picker's own `Text` widgets, at their own coordinates, in their own
        // colours, with their strings from the string table.
        for text in &screen.texts {
            let Some(id) = text.idstring.as_deref().or(text.string.as_deref()) else {
                continue;
            };
            let body = self.strings.get_or_id(id);
            if body.is_empty() {
                continue;
            }
            let scale = text.scale.max(0.5);
            out.push(Draw::Text {
                x: text.x,
                // Nudged into the viewport. The XML's coordinates are absolute
                // within the real screen's widget tree, and we do not apply
                // parent offsets, so a title authored near the top edge lands
                // slightly above it and draws with its top row cut off. Clamping
                // is a viewer's correction, in the same spirit as `lighten`
                // below, not a claim about the game's layout.
                y: text.y.max(1.0 * scale),
                scale,
                // The XML's title colour is black on a black backdrop, so the
                // colour is lifted rather than silently producing an empty
                // screen.
                //
                // **Why it is black is a hypothesis, not a finding**, and it is
                // worth being explicit now that the menu backdrop exists and
                // could be drawn here. The guess is that the real screen sits on
                // a lit background; the evidence is against reaching for it,
                // because only three screens in this XML carry a `Movie` widget
                // and `Language Selection` is not one of them - and the picker
                // runs *before* `LogoFMV`, so the backdrop being loaded at that
                // point is not established either. Settling it means finding
                // what the picker's parent draws, not assuming it is the thing
                // we now happen to have. So the lift stays and the backdrop is
                // not drawn here. See `docs/architecture/menus.md`.
                color: lighten(argb_to_rgba(text.color)),
                border: None,
                align: Align::parse(&text.align),
                text: body.to_string(),
            });
        }

        // `DisplayLanguages` plus `Menu`: one row per language the disc offers,
        // at the `Menu` widget's own position, scale, colour and alignment.
        // There is no per-row height in the XML - `DisplayLanguages` only
        // populates the list `Menu` draws - so row spacing is inferred from
        // the widget's own font's line height, the same quantity a line of
        // that font would step by anywhere else it is used.
        let menu = screen.menu.as_ref();
        let menu_x = menu.map_or(0.0, |m| m.x);
        let menu_y = menu.map_or(0.0, |m| m.y);
        let scale = menu.map_or(1.0, |m| m.scale).max(0.5);
        let align = Align::parse(menu.map_or("left", |m| m.align.as_str()));
        let color = argb_to_rgba(menu.map_or(0xffff_ffff, |m| m.color));
        let row = font_line_height(menu.map_or("Default", |m| m.font.as_str())) * scale;

        for (index, language) in self.languages.iter().enumerate() {
            let y = menu_y + index as f32 * row;
            let selected = index == self.selected;
            if selected {
                out.push(Draw::Fill {
                    rect: [menu_x - 6.0, y - 4.0, 220.0, row - 4.0],
                    color: [0.37, 0.86, 0.96, 0.35],
                });
            }
            out.push(Draw::Text {
                x: menu_x,
                y,
                scale,
                color: if selected {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    color
                },
                border: None,
                align,
                text: language.native_name.clone(),
            });
        }
    }

    /// The screen the picker is defined by, for reporting.
    #[must_use]
    pub fn language_screen(&self) -> Option<&Screen> {
        self.screens.language_selection()
    }
}

/// The line height of a font id, in the PSP's own pixel units.
///
/// Matches the `.fnt` files named in `oag_assets::pulse::names::fonts`:
/// `Default` is `pulse_text.fnt` (13px), `Menu` is `Pulse_20.fnt` (22px),
/// `Title`, `Small`, `InGame` and `Stats` are `Pulse_14.fnt` (17px), `HUD` is
/// `PulseHud.fnt` (25px) and `HUDSmall` is `small.fnt` (10px). The XML is
/// inconsistent about case (`font="menu"` and `font="Menu"` both appear), so
/// this matches case-insensitively.
fn font_line_height(font: &str) -> f32 {
    match font.to_ascii_lowercase().as_str() {
        "menu" => 22.0,
        "title" | "small" | "ingame" | "stats" => 17.0,
        "hud" => 25.0,
        "hudsmall" => 10.0,
        // "Default", and anything this build does not otherwise recognise.
        _ => 13.0,
    }
}

/// Raises a colour to something visible on black, keeping its hue.
///
/// The picker's title is `0xFF000000` in the XML because the real screen has a
/// lit background behind it. Drawing black on black would look like a bug in
/// this code rather than a missing background, so near-black is lifted.
fn lighten(rgba: [f32; 4]) -> [f32; 4] {
    let luma = 0.299 * rgba[0] + 0.587 * rgba[1] + 0.114 * rgba[2];
    if luma < 0.15 {
        [0.85, 0.9, 0.95, rgba[3]]
    } else {
        rgba
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f64 = 1001.0 / 30_000.0;

    const XML: &str = r#"
<Screen>
  <Screen type="Language Selection" name="Language Selection">
    <Text name="LanguageText"><Values idstring="Language Selection" font="Title" x="21" y="0"></Values></Text>
    <DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
    <Menu name="Language"><Values align="left" font="Default" x="50" scale="1.0" y="46" color="0xFF33A6B9"></Values></Menu>
    <Redirect name="LanguageAutoRedirect">
      <Values backward="none"></Values>
      <Default goto="LogoFMV"></Default>
    </Redirect>
  </Screen>
  <Screen name="LogoFMV">
    <Movie><Values src="Data\Movies\Intro" autostart="true" autoredirect="true"></Values></Movie>
  </Screen>
</Screen>
"#;

    fn languages() -> Vec<Language> {
        [
            ("PI008", "French", "Français"),
            ("PI009", "German", "Deutsch"),
            ("PI012", "English", "English"),
        ]
        .into_iter()
        .map(|(plugin, name, native)| Language {
            plugin: plugin.into(),
            name: name.into(),
            native_name: native.into(),
            entries: None,
        })
        .collect()
    }

    /// The default leg: `LogoFMV`, playing the movie the disc plays.
    fn frontend(frames: usize) -> Frontend {
        Frontend::new(
            Screens::from_xml(XML),
            StringTable::default(),
            languages(),
            Vec::new(),
            frames,
            false,
        )
    }

    /// The `--reel` leg: `Intro Screen->IntroMovie1`, with the frame holds.
    fn reel(frames: usize) -> Frontend {
        Frontend::booting(
            Leg::DevPubReel,
            Screens::from_xml(XML),
            StringTable::default(),
            languages(),
            Vec::new(),
            frames,
            crate::movie::FRAME_RATE,
            (SCREEN.0 as u32, SCREEN.1 as u32),
            false,
        )
    }

    /// Runs until `predicate` holds or the step budget runs out.
    fn run_until(
        frontend: &mut Frontend,
        input: &mut Input,
        steps: usize,
        predicate: impl Fn(&Frontend) -> bool,
    ) -> bool {
        for _ in 0..steps {
            if predicate(frontend) {
                return true;
            }
            input.begin_frame(0);
            frontend.update(FRAME, input);
        }
        predicate(frontend)
    }

    #[test]
    fn boots_into_the_screen_the_disc_boots_into() {
        let frontend = frontend(300);
        assert_eq!(frontend.machine().current(), Some(states::LOGO_FMV));
        assert!(
            !frontend.machine().is_in(states::INTRO),
            "the dev/pub reel state is not on the boot path"
        );
    }

    #[test]
    fn the_logo_fmv_leg_plays_straight_through_with_no_holds() {
        let mut frontend = frontend(300);
        let mut input = Input::new();

        // Past both of the reel state's pause frames, and still playing: those
        // counters belong to the other leg.
        for _ in 0..250 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input);
        }
        assert!(!frontend.player().is_paused(), "LogoFMV has no frame holds");
        assert_eq!(frontend.player().frames_produced(), 251);
        assert!(frontend.machine().is(states::LOGO_FMV));

        assert!(
            run_until(&mut frontend, &mut input, 400, |f| f
                .machine()
                .is(states::LANGUAGE_SELECTION)),
            "the movie ending must go on to the picker"
        );
        assert_eq!(
            frontend.machine().history(),
            [states::LOGO_FMV, states::LANGUAGE_SELECTION],
            "and not through DevPubRedirect, which is the other leg's"
        );
    }

    #[test]
    fn either_skip_button_leaves_the_logo_fmv_leg() {
        for skip in [button::START, button::CROSS] {
            let mut frontend = frontend(300);
            let mut input = Input::new();
            input.begin_frame(1 << skip);
            frontend.update(FRAME, &mut input);
            assert!(
                frontend.machine().is(states::LANGUAGE_SELECTION),
                "button {skip} must skip the movie"
            );
        }
    }

    #[test]
    fn pauses_at_144_then_231_then_finishes_at_260() {
        let mut frontend = reel(300);
        let mut input = Input::new();

        // Frame 144 is reached after 143 steps from frame 1.
        for _ in 0..143 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input);
        }
        assert_eq!(frontend.player().frames_produced(), 144);
        assert!(frontend.player().is_paused(), "must pause at 144");

        // Two seconds is 59.94 frames, so 59 is not yet up and 61 is.
        for _ in 0..59 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input);
        }
        assert!(frontend.player().is_paused(), "two seconds is not up yet");
        assert_eq!(
            frontend.player().frames_produced(),
            144,
            "a hold must not advance the movie"
        );
        for _ in 0..2 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input);
        }
        assert!(!frontend.player().is_paused(), "the hold must release");

        assert!(
            run_until(&mut frontend, &mut input, 2000, |f| f
                .machine()
                .is(states::LANGUAGE_SELECTION)),
            "the intro must end at the language picker"
        );

        // Both pauses and the finish must have happened, in order.
        let history = frontend.machine().history();
        assert_eq!(
            history,
            [
                states::INTRO,
                states::INTRO_MOVIE,
                states::DEV_PUB_REDIRECT,
                states::LANGUAGE_SELECTION,
            ]
        );
    }

    #[test]
    fn start_skips_the_intro_by_firing_the_redirect() {
        let mut frontend = reel(300);
        let mut input = Input::new();

        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        assert_eq!(
            frontend.machine().current(),
            Some(states::DEV_PUB_REDIRECT),
            "skipping goes through the redirect, not straight to the picker"
        );
        // The player is not stopped: teardown is a consequence of the transition.
        assert!(!frontend.player().is_finished());

        input.begin_frame(0);
        frontend.update(FRAME, &mut input);
        assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
    }

    #[test]
    fn the_redirect_target_is_cleared_so_it_cannot_fire_twice() {
        let mut frontend = reel(300);
        let mut input = Input::new();

        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        assert!(
            run_until(&mut frontend, &mut input, 2000, |f| f
                .machine()
                .is(states::LANGUAGE_SELECTION)),
            "must arrive at the picker"
        );

        // The intro's own frame counters would fire the redirect again if the
        // target were not cleared, which would bounce out of the picker.
        let entries = frontend
            .machine()
            .history()
            .iter()
            .filter(|name| *name == states::DEV_PUB_REDIRECT)
            .count();
        assert_eq!(entries, 1, "{:?}", frontend.machine().history());
    }

    #[test]
    fn a_reel_shorter_than_260_frames_still_ends() {
        let mut frontend = reel(30);
        let mut input = Input::new();
        assert!(
            run_until(&mut frontend, &mut input, 500, |f| f
                .machine()
                .is(states::LANGUAGE_SELECTION)),
            "a short movie must not stall the boot"
        );
    }

    #[test]
    fn no_picture_still_plays_the_sequence() {
        let mut frontend = Frontend::new(
            Screens::from_xml(XML),
            StringTable::default(),
            languages(),
            Vec::new(),
            0,
            false,
        );
        let mut input = Input::new();
        assert!(run_until(&mut frontend, &mut input, 2000, |f| f
            .machine()
            .is(states::LANGUAGE_SELECTION)));
    }

    #[test]
    fn the_picker_moves_and_wraps_both_ways() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input);
        assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

        assert_eq!(frontend.selected(), 0);
        input.begin_frame(1 << button::DOWN);
        frontend.update(FRAME, &mut input);
        assert_eq!(frontend.selected(), 1);

        input.begin_frame(0);
        input.begin_frame(1 << button::UP);
        frontend.update(FRAME, &mut input);
        assert_eq!(frontend.selected(), 0);

        input.begin_frame(0);
        input.begin_frame(1 << button::UP);
        frontend.update(FRAME, &mut input);
        assert_eq!(
            frontend.selected(),
            2,
            "up from the first wraps to the last"
        );
    }

    #[test]
    fn picking_a_language_launches_the_game() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input);

        input.begin_frame(1 << button::DOWN);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        input.begin_frame(1 << button::CROSS);
        let events = frontend.update(FRAME, &mut input);

        assert_eq!(frontend.chosen(), Some("German"));
        assert!(frontend.is_finished());
        assert!(events.contains(&Event::Enter(states::LAUNCH_GAME.into())));
    }

    #[test]
    fn circle_does_not_select() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input);

        input.begin_frame(1 << button::CIRCLE);
        frontend.update(FRAME, &mut input);
        assert_eq!(
            frontend.chosen(),
            None,
            "cancel is circle, activate is cross"
        );
    }

    #[test]
    fn the_discs_own_redirect_is_reported_not_followed() {
        let frontend = frontend(300);
        assert_eq!(frontend.language_auto_redirect(), Some("LogoFMV"));
    }

    #[test]
    fn the_picker_draws_one_row_per_language() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input);

        let draws = frontend.draw_list();
        let rows: Vec<&String> = draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert!(rows.iter().any(|t| *t == "Français"), "{rows:?}");
        assert!(rows.iter().any(|t| *t == "Deutsch"), "{rows:?}");
        assert!(rows.iter().any(|t| *t == "English"), "{rows:?}");
    }

    #[test]
    fn the_intro_draws_no_video_quad_without_a_picture() {
        let frontend = frontend(300);
        let draws = frontend.draw_list();
        assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
    }

    #[test]
    fn the_intro_draws_a_video_quad_when_there_is_one() {
        let frontend = Frontend::new(
            Screens::from_xml(XML),
            StringTable::default(),
            languages(),
            Vec::new(),
            300,
            true,
        );
        let draws = frontend.draw_list();
        assert!(draws.iter().any(|d| matches!(d, Draw::Video { .. })));
    }

    #[test]
    fn near_black_text_is_lifted_off_a_black_screen() {
        assert_ne!(lighten([0.0, 0.0, 0.0, 1.0]), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(lighten([1.0, 0.5, 0.2, 1.0]), [1.0, 0.5, 0.2, 1.0]);
    }

    #[test]
    fn an_empty_language_list_does_not_panic() {
        let mut frontend = Frontend::new(
            Screens::from_xml(XML),
            StringTable::default(),
            Vec::new(),
            Vec::new(),
            300,
            false,
        );
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input);
        input.begin_frame(1 << button::CROSS);
        frontend.update(FRAME, &mut input);
        assert_eq!(frontend.chosen(), None);
        let _ = frontend.draw_list();
    }
}
