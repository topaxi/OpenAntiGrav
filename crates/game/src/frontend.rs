//! The boot sequence: the intro movie, the language picker, then PRESS START.
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
//! an `AutoRedirect` to `Show Logo`, and five more redirects -
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
//! `LogoFMV`'s own exits also differ, though less than they used to. On the disc
//! the movie finishing fires `AutoRedirect` to `Show Logo` - the Pulse logo and
//! PRESS START - and any of start, circle, square, triangle or cross fires
//! [`states::LOGO_FMV_REDIRECT`], which goes to the same place. Both hops are
//! modelled here and both still land on the picker rather than on
//! [`states::SHOW_LOGO`], because the picker is what this build has next; only
//! start and cross are read, because those are the buttons the abstract layer
//! carries.
//!
//! [`states::SHOW_LOGO`] itself **is** on the boot path now, one state later than
//! the disc puts it: `LogoFMV` -> picker -> `Show Logo` -> `Launch Game`. That
//! keeps the disc's own relative order of the three screens this build has -
//! `Show Logo` after both the movie and the picker, and immediately before the
//! menus - which is as close as the picker/movie swap above allows. What the
//! disc has between `Show Logo` and its main menu is `RemoveMemoryStickWarning`,
//! `NameSetup2FromBoot`, `TagSetup2FromBoot` and `CreateFromBoot`, four screens
//! that exist to serve Memory Stick mechanics this build deliberately does not
//! have; they are skipped rather than unimplemented. See `HANDOVER.md`.
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
    /// The one-shot redirect `LogoFMV`'s five skip buttons fire.
    ///
    /// A real screen in the front-end XML, and it holds nothing but a single
    /// `Redirect` with `forward="none"` - so its whole job is to leave, the
    /// same shape as [`DEV_PUB_REDIRECT`]. On the disc it goes to
    /// [`SHOW_LOGO`], which is where `LogoFMV`'s own `AutoRedirect` goes too;
    /// here it goes to [`LANGUAGE_SELECTION`], because the picker is what this
    /// build has next. See the module docs.
    pub const LOGO_FMV_REDIRECT: &str = "LogoFMVRedirectScreen";
    /// The Pulse logo and PRESS START.
    ///
    /// **Spelled bare rather than as a path, and that is evidence rather than
    /// convenience.** In the XML this screen is nested three deep, at
    /// `Top FE Screen->FE Screen->Show Logo`, but the running machine's own
    /// current-state buffer reads `"Show Logo"`: at `0x08d0a820+0x18c` it was
    /// caught holding `"Show Logo\0election\0"`, the shorter name overwriting
    /// `"Language Selection"` in place. Every `goto` that reaches it spells it
    /// bare as well. Confidence **95**, runtime-observed - see
    /// `docs/ghidra/functions/psp-pulse-usa/main-loop.md`. What the buffer does
    /// with a name that genuinely needs its parents is not established, so
    /// nothing here concludes that the original's states are flat.
    pub const SHOW_LOGO: &str = "Show Logo";
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

/// Which of the front end's two movies a [`Draw::Video`] wants a frame of.
///
/// The sequence plays both, one after the other, and they are **different files
/// with their own plane geometry** - so a caller that reads a frame out of the
/// wrong one gets a picture rather than an error. That is precisely the failure
/// [`crate::capture`] already carries a warning about for `--menu-page`, so the
/// draw says which movie it means instead of leaving it to be inferred from the
/// current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Video {
    /// `Data\Movies\Intro.PMF`, played once through by `LogoFMV`.
    Intro,
    /// `Data\Movies\Backdrop.PMF`, looped by `FE Screen` under
    /// [`states::SHOW_LOGO`].
    Backdrop,
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
    /// A movie's current frame, stretched to `rect`.
    Video {
        /// Rectangle.
        rect: [f32; 4],
        /// Which frame of the movie to show, counting from zero.
        ///
        /// Wraps on a movie that loops, so this is what a caller holding the
        /// whole movie - a headless capture reading a
        /// [`crate::movie::FrameStore`] - asks for.
        frame: usize,
        /// How far playback has got, counting every loop.
        ///
        /// The same number [`crate::movie::Player::position`] reports, and the
        /// **only** one a [`crate::movie::Feed`] can be asked with: a feed
        /// decodes forward forever, so on the second time round a 270-frame
        /// loop it holds positions 270..274 while `frame` has gone back to 0.
        /// Asking it with `frame` takes nothing from that point on and the
        /// picture freezes - which is exactly what the menu backdrop used to do
        /// nine seconds into the boot sequence.
        position: u64,
        /// Which movie the frame belongs to.
        source: Video,
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
///
/// Still a constant, and still the PSP's: it is what our own layouts
/// (`crate::loading`), the HUD's bounds checks and `oag_race`'s authored aspect
/// are written against, and every front-end test embeds PSP XML. A source that
/// authors somewhere else says so through [`Space`] instead of redefining this.
pub const SCREEN: (f32, f32) = (480.0, 272.0);

/// The coordinate space a source's front-end XML places widgets in, and what
/// that space is displayed as.
///
/// # Why these are two numbers and not one
///
/// On the PSP they are the same ratio and nothing notices. On the PS2 they are
/// not, because its pixels are not square: `Skin.xml` places widgets in a
/// **640x448** grid, and a PAL 640x448 frame is shown as **4:3**. Deriving the
/// display aspect from the grid would stretch the whole front end 7% wide
/// (640/448 = 1.429 against 4:3 = 1.333), and using the display aspect as the
/// grid would put every widget in the wrong place. Three sites need one or the
/// other and picking the wrong one is silent on the PSP:
///
/// - the renderer's `screen` uniform, which maps a draw's rect onto the
///   viewport, wants the **grid**;
/// - `crate::render::letterbox`, which fits that grid into a window without
///   distorting it, wants the **display aspect**;
/// - [`pillarbox`] wants **both** - it returns a rect in grid coordinates, but
///   the decision of whether a picture is wider or narrower than the screen is
///   a question about display aspects. This is the one that stays wrong after
///   the other two are fixed, and it is what `INTRO512.PSS` runs into.
///
/// # Evidence for the PS2 grid
///
/// The PS2 `Skin.xml`'s layout is the PSP's, scaled by exactly the resolution
/// ratio. `BOOT_PRESS_START` sits at `x=613 y=362` against the PSP's `460`/`230`
/// on the same disc region, the `Show Logo` logo at `y=119` against `72`, and
/// the extremes over the whole file are `x=613 y=415` against `x=460 y=252`.
/// Every one of those matches 640/480 and 448/272 to better than a tenth of a
/// percent: 613/460 = 1.3326 against 1.3333, and 415/252 = 1.6468 against
/// 1.6471. Confidence **95** - the arithmetic is unambiguous and 640x448 is
/// PAL's own frame; what is not independently confirmed is the *display*
/// aspect, which is taken from `crate::display::Aspect::Ps2`'s existing 4:3
/// rather than measured here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    /// The grid widget coordinates are in.
    pub size: (f32, f32),
    /// What that grid is shown as. **Not** `size.0 / size.1`.
    pub display_aspect: f32,
}

impl Space {
    /// 480x272 square pixels, shown as itself.
    pub const PSP: Self = Self {
        size: SCREEN,
        display_aspect: SCREEN.0 / SCREEN.1,
    };
    /// PAL's 640x448, shown as 4:3.
    pub const PS2: Self = Self {
        size: (640.0, 448.0),
        display_aspect: 4.0 / 3.0,
    };

    /// The space a source authors in, from what its archives say it is.
    #[must_use]
    pub fn of(platform: oag_disc::Platform) -> Self {
        match platform {
            oag_disc::Platform::Ps2 => Self::PS2,
            // A source we could not identify is read as a PSP one, which is
            // what every other unidentified-source path here already does.
            _ => Self::PSP,
        }
    }
}

impl Default for Space {
    fn default() -> Self {
        Self::PSP
    }
}

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
    pillarbox_in(
        Space {
            size: screen,
            display_aspect: screen.0 / screen.1,
        },
        aspect,
    )
}

/// [`pillarbox`], told what the grid is *shown* as rather than assuming square
/// pixels.
///
/// The returned rect is in `space.size`'s coordinates, but which way a picture
/// has to be boxed is decided against `space.display_aspect`. The two agree on
/// the PSP and disagree on the PS2 by 7%, which is a `.PSS` pillarboxed inside
/// a screen that was already its own shape. See [`Space`].
#[must_use]
pub fn pillarbox_in(space: Space, aspect: (u32, u32)) -> [f32; 4] {
    let (screen_w, screen_h) = space.size;
    let content = aspect.0 as f32 / aspect.1 as f32;
    let frame = space.display_aspect;

    // A `w` by `h` slice of a grid that shows as `frame` is displayed at
    // `frame * (w / screen_w) / (h / screen_h)`. Setting that equal to
    // `content` and solving for the axis being given up is what these two are;
    // both reduce to the old `screen_w / content` and `screen_h * content` when
    // the pixels are square, which is why the PSP never noticed.
    if content >= frame {
        let height = frame * screen_h / content;
        [0.0, (screen_h - height) / 2.0, screen_w, height]
    } else {
        let width = content * screen_w / frame;
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
    /// The grid this source's XML places widgets in, and what it is shown as.
    ///
    /// [`Space::PSP`] unless a caller says otherwise, because every screen this
    /// file's own tests parse is PSP XML. `boot::load` sets it from the
    /// archives' platform - see [`Space`] for why the PS2 needs it and what
    /// goes wrong silently without it.
    space: Space,
    /// Whether a picture is available at all.
    has_picture: bool,
    /// The looping backdrop `FE Screen` plays under `Show Logo`, and where it
    /// goes on screen. `None` on a source that has no backdrop, under
    /// `--no-video`, and when its plane geometry does not match the intro's -
    /// see [`Frontend::set_backdrop`], which is the only thing that sets it.
    backdrop: Option<(crate::movie::Player, [f32; 4])>,
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
            states::LOGO_FMV_REDIRECT,
            states::SHOW_LOGO,
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
            space: Space::PSP,
            player: crate::movie::Player::new(frames, false, movie_frame_rate),
            video_aspect,
            has_picture,
            backdrop: None,
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

    /// Tells the sequence which grid its XML places widgets in.
    ///
    /// **Call this before [`Self::set_backdrop`].** The backdrop's rect is
    /// computed once, from the space in force at the time, so setting the space
    /// afterwards leaves a PSP-shaped rect on a PS2 screen.
    pub fn set_space(&mut self, space: Space) {
        self.space = space;
    }

    /// The grid this sequence's draw rects are in.
    ///
    /// Whatever draws them has to be told: the renderer maps a rect onto the
    /// viewport through it, and a renderer still assuming 480x272 would put a
    /// PS2 screen's widgets at a third again their proper size. See [`Space`].
    #[must_use]
    pub fn space(&self) -> Space {
        self.space
    }

    /// Gives the sequence the looping backdrop `Show Logo` sits on.
    ///
    /// **`Show Logo` is a child of `FE Screen`, and `FE Screen` is what owns
    /// `Data\Movies\Backdrop`** - so on hardware the Pulse logo and PRESS START
    /// are drawn over the moving menu backdrop, not over black. That nesting is
    /// read off `Skin.xml` (confidence 98), the boot capture confirms
    /// `Backdrop.PMF` opening during boot rather than when the menus open, and a
    /// player who has run the original confirms the picture. Before all three
    /// agreed this was left as a documented gap; it is not a gap now.
    ///
    /// Optional because it is optional on the data: a source with no backdrop,
    /// `--no-video` and a missing `ffmpeg` all end with this never being called,
    /// and then `Show Logo` draws on black exactly as it did.
    ///
    /// `repeat` is `true`, which is the whole difference between this movie and
    /// the intro, and it is the widget's own attribute rather than a choice
    /// here. The playhead runs from the moment the sequence starts rather than
    /// from the moment `Show Logo` is entered: the movie is `autostart` on a
    /// screen the boot reaches long before this build draws it, and the capture
    /// showing it open ~15 s in - while the state is still `LogoFMV` - is what
    /// says it is already running by then rather than starting fresh.
    pub fn set_backdrop(&mut self, frames: usize, frame_rate: (u64, u64), aspect: (u32, u32)) {
        if frames == 0 {
            return;
        }
        self.backdrop = Some((
            crate::movie::Player::new(frames, true, frame_rate),
            pillarbox_in(self.space, aspect),
        ));
    }

    /// The backdrop's playhead, for tests and for whoever is pumping its feed.
    #[must_use]
    pub fn backdrop(&self) -> Option<&crate::movie::Player> {
        self.backdrop.as_ref().map(|(player, _)| player)
    }

    /// Hands the backdrop's playhead on, leaving the sequence without one.
    ///
    /// **`Data\Movies\Backdrop` is one continuous playback for as long as the
    /// front end is up.** `Show Logo` and the menus that follow it are two
    /// screens in front of the *same* looping movie, not two playbacks of it -
    /// so when the boot sequence ends, the player moves rather than being
    /// rebuilt at frame zero. Rebuilding it is what made the picture jump back
    /// to the start of the loop the moment START was pressed, which a player
    /// who has run the original reported as wrong.
    ///
    /// Whoever takes it also takes the obligation the sequence had: keep
    /// advancing it, and keep asking the one [`crate::movie::Feed`] for frames
    /// at its position. Nothing restarts the feed on this path - the two share
    /// an origin already, and a restart is what would put them back at odds.
    ///
    /// Called once, when the menus open. The sequence draws no backdrop
    /// afterwards, which is correct: it is over.
    pub fn take_backdrop(&mut self) -> Option<crate::movie::Player> {
        self.backdrop.take().map(|(player, _)| player)
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

    /// Whether the sequence is on one of its two movie screens.
    ///
    /// What a caller holding the movie's sound asks to know when to stop it,
    /// and the reason it lives here is that it has to be true of **both** legs
    /// and of every way out of them. The movie ends on its own on one path and
    /// is cut short by START on four others - all five leave the state, and
    /// none of them finishes the player - so "is the player finished" is the
    /// wrong question and would leave the intro's sound playing under the
    /// language picker.
    #[must_use]
    pub fn is_playing_movie(&self) -> bool {
        self.machine.is(states::LOGO_FMV) || self.machine.is(states::INTRO_MOVIE)
    }

    /// Steps the sequence by `dt` seconds.
    ///
    /// `movie_playhead` is how far the movie's **own sound** has got, in
    /// seconds, and `None` - the ordinary case - means it has none to pace
    /// against. See [`Self::advance_movie`] for what it does with it, and
    /// [`crate::audio::Audio::movie_playhead`] for every reason it is `None`.
    /// It is a parameter rather than something set beforehand because there are
    /// two tick loops - the window's and the headless capture's - and an
    /// argument makes forgetting one a compile error instead of a movie that
    /// silently reverts to the wrong clock on one path.
    ///
    /// Returns the transitions that happened, in order, so a caller can log them
    /// or a test can assert on them.
    pub fn update(
        &mut self,
        dt: f64,
        input: &mut Input,
        movie_playhead: Option<f64>,
    ) -> Vec<Event> {
        // Advanced whatever state the sequence is in, and deliberately: the
        // widget is `autostart` on a screen whose movie the boot opens well
        // before this build draws it, so by the time `Show Logo` is reached the
        // loop is already somewhere in the middle of itself rather than at frame
        // zero. It repeats, so there is no end to run off.
        //
        // **Always the tick clock, never `movie_playhead`.** `Backdrop.PMF` has
        // no audio stream and its widget is `sound="false"`, so there is no
        // playhead that belongs to it - and the one being passed in belongs to
        // a different movie entirely, the intro, which is playing over the top
        // of this loop for the whole boot sequence. Pacing the backdrop against
        // the intro's sound would tie the menus' loop to a movie that has
        // ended. See ADR-0019.
        if let Some((player, _)) = &mut self.backdrop {
            player.update(dt);
        }

        if self.machine.is(states::LOGO_FMV) {
            self.update_logo_fmv(dt, input, movie_playhead);
        } else if self.machine.is(states::INTRO_MOVIE) {
            self.update_intro(dt, input, movie_playhead);
        } else if self.machine.is(states::DEV_PUB_REDIRECT)
            || self.machine.is(states::LOGO_FMV_REDIRECT)
        {
            // A redirect state's whole job is to leave. Both of these are real
            // screens with nothing in them but one `forward="none"` redirect.
            self.machine.fire(states::LANGUAGE_SELECTION);
        } else if self.machine.is(states::LANGUAGE_SELECTION) {
            self.update_language_selection(input);
        } else if self.machine.is(states::SHOW_LOGO) {
            self.update_show_logo(input);
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
    fn update_logo_fmv(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        for button in [button::START, button::CROSS] {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.notes.push(format!(
                    "the movie was skipped, firing {}",
                    states::LOGO_FMV_REDIRECT
                ));
                self.machine.fire(states::LOGO_FMV_REDIRECT);
                return;
            }
        }

        self.advance_movie(dt, playhead);
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

    /// Advances the movie's playhead on whichever clock ADR-0019 says is its.
    ///
    /// The **only** place the intro's picture is advanced, so the choice is
    /// made once: audio when the movie has sound that is actually sounding,
    /// the fixed timestep otherwise. Two callers reach it, which is exactly why
    /// it is a method rather than the same three lines in each of them - the
    /// two movie legs must not end up on different clocks.
    fn advance_movie(&mut self, dt: f64, playhead: Option<f64>) {
        match playhead {
            Some(seconds) => self.player.follow(seconds),
            None => self.player.update(dt),
        }
    }

    fn update_intro(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
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

        // **Before the hold, not after it, and that is what keeps this leg's
        // sound in step with its picture.** This leg holds the picture for two
        // seconds at frames 144, 231 and 260, and a sound card cannot be asked
        // to hold with it - so the player has to be told the hold is happening
        // while it happens, or those two seconds of audio would be skipped over
        // in one step the moment it resumed. It is paused throughout, so on the
        // tick clock this call does nothing at all and the leg's timing is
        // unchanged; on the audio clock it is what lets `Player::follow`
        // discount the hold. See `movie::Player::follow`.
        self.advance_movie(dt, playhead);

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

    /// Takes the highlighted language and leaves for [`states::SHOW_LOGO`].
    fn confirm_language(&mut self) {
        let language = self.languages[self.selected].clone();
        let disc_goto = self.language_auto_redirect().map(str::to_string);
        self.chosen = Some(language.name.clone());
        self.notes.push(format!(
            "language {} ({}) selected, firing {}",
            language.name,
            language.native_name,
            states::SHOW_LOGO
        ));
        if let Some(goto) = disc_goto {
            self.notes.push(format!(
                "note: the disc's own LanguageAutoRedirect goes to {goto}, not {}",
                states::SHOW_LOGO
            ));
        }
        self.machine.fire(states::SHOW_LOGO);
    }

    /// `Show Logo`: the Pulse logo, PRESS START, and nothing that moves.
    ///
    /// **START and only START, and no timeout at all.** The screen's own XML is
    /// the whole of the evidence and it is unambiguous: `Show Logo` carries
    /// exactly two `Redirect` widgets, one `forward="start"` and one with no
    /// `forward` attribute, both `goto="RemoveMemoryStickWarning"`. There is no
    /// frame counter, no `delay` on either redirect and no timer anywhere on the
    /// screen, so nothing here invents a duration. Note this is a **narrower**
    /// button set than [`states::LOGO_FMV`]'s, which lists all five of start,
    /// circle, square, triangle and cross - the screen that says "Press START
    /// button" means it. Confidence **95**: read straight off
    /// `Data\Plugins\PI001\GUI\Skin.xml` on `pulse-psp-usa.chd`, and the EU
    /// disc's copy has the same two redirects.
    ///
    /// The unnamed redirect is the one thing left open. It has no button and no
    /// name, which is the shape `Language Selection` gives to
    /// `LanguageAutoRedirect` - a redirect the code fires rather than the pad -
    /// so something in the original can advance this screen without a press.
    /// **What that something is has not been found**, and a guess would be a
    /// hidden timeout, so it is left unfired here rather than modelled.
    fn update_show_logo(&mut self, input: &mut Input) {
        if input.is_pressed(button::START) {
            input.consume_press(button::START);
            self.notes.push(format!(
                "START pressed on {}, firing {}",
                states::SHOW_LOGO,
                states::LAUNCH_GAME
            ));
            self.machine.fire(states::LAUNCH_GAME);
        }
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
        let (width, height) = self.space.size;
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, width, height],
            color: [0.0, 0.0, 0.0, 1.0],
        }];

        if self.machine.is_in(states::INTRO) || self.machine.is(states::LOGO_FMV) {
            if self.has_picture {
                out.push(Draw::Video {
                    rect: pillarbox_in(self.space, self.video_aspect),
                    frame: self.player.frame(),
                    // The same number as `frame` here, the intro being played
                    // once through rather than looped, and carried anyway so a
                    // consumer never has to know which movie it is holding.
                    position: self.player.position(),
                    source: Video::Intro,
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
            self.insert_backdrop(&mut out);
            self.draw_language_selection(&mut out);
            return out;
        }

        if self.machine.is(states::SHOW_LOGO) {
            // Its widgets and nothing else, which is what `--screen "Show Logo"`
            // has been drawing all along - so the state reaching the boot order
            // changes when it is drawn, not what is drawn.
            //
            // Over the moving menu backdrop, which is the screen's parent's
            // movie: `Show Logo` is a child of `FE Screen` and `FE Screen` is
            // what owns `Data\Movies\Backdrop`. Inserted at index 1 - after the
            // black fill the screen is cleared with, under the logo and the
            // text - because the draw list is drawn in its own order and this
            // belongs at the bottom of it.
            //
            // One thing the screen has that this still does not reproduce, and
            // it is recorded rather than approximated: `BOOT_PRESS_START`
            // carries `pulse="true"` and `delay="1"`, so on the disc it fades in
            // a second late and throbs instead of appearing at once and standing
            // still. See `docs/architecture/frontend-boot.md`.
            let mut out = self.draw_screen(states::SHOW_LOGO);
            self.insert_backdrop(&mut out);
            return out;
        }

        if self.machine.is(states::LAUNCH_GAME) {
            // The same loop, for the same reason, and this one is the frame a
            // player actually stares at: the composition root draws
            // `Launch Game` **once and then stalls** building the menus (see
            // `Session::frame`), so this is on screen for the whole of that
            // load rather than for a frame. Without the backdrop under it the
            // START press read as `Show Logo` -> a few hundred milliseconds of
            // black -> the menus, which is most of the flicker the transition
            // was reported for.
            self.insert_backdrop(&mut out);
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

    /// Puts the `FE Screen` backdrop at the bottom of a screen's draw list.
    ///
    /// At index 1, after the black fill a screen is cleared with and under
    /// everything else, because the list is drawn in its own order.
    ///
    /// **Every screen between the intro and the menus gets it, not only
    /// `Show Logo`.** The movie belongs to `FE Screen`, is `autostart` and
    /// `repeat`, and the boot opens it long before any of these screens draw;
    /// on the disc its children are drawn over a loop that never stops. Leaving
    /// it off a screen therefore does not mean "the loop is not running there",
    /// it means one screen punches a black hole in a continuous picture - which
    /// is what both reported transition defects turned out to be made of.
    /// `Language Selection` is this build's own screen rather than one of the
    /// disc's, so putting it on the same backdrop is a consistency judgement;
    /// `Launch Game` is a continuity requirement. See
    /// [menus.md](../../../docs/architecture/menus.md).
    fn insert_backdrop(&self, out: &mut Vec<Draw>) {
        let Some((player, rect)) = &self.backdrop else {
            return;
        };
        out.insert(
            1,
            Draw::Video {
                rect: *rect,
                frame: player.frame(),
                position: player.position(),
                source: Video::Backdrop,
            },
        );
    }

    /// A named screen's own widgets, drawn from the XML alone.
    ///
    /// This is what the disc's data describes and nothing else: no state, no
    /// input, no animation. `--screen` renders through it, which is how the
    /// image path is checked without moving any screen into the boot order.
    #[must_use]
    pub fn draw_screen(&self, name: &str) -> Vec<Draw> {
        let (width, height) = self.space.size;
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
                rect: [0.0, 0.0, self.space.size.0, self.space.size.1],
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
                (self.space.size.0 - w) / 2.0
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
/// Matches the `.fnt` files named in `oag_pulse::names::fonts`:
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

    /// The PS2 grid is the PSP's, scaled by the resolution ratio.
    ///
    /// Not a round-number check: `Skin.xml`'s own extremes are `x=613 y=415` on
    /// the PS2 against `x=460 y=252` on the PSP, and both ratios have to land on
    /// 640/480 and 448/272 for 640x448 to be the grid it authors in. They do, to
    /// under a tenth of a percent. See [`Space`].
    #[test]
    fn the_ps2_grid_is_the_psp_layout_scaled_by_the_resolution() {
        let psp = Space::PSP.size;
        let ps2 = Space::PS2.size;
        assert!((613.0 / 460.0 - ps2.0 / psp.0).abs() < 0.002, "x extreme");
        assert!((415.0 / 252.0 - ps2.1 / psp.1).abs() < 0.002, "y extreme");
        // **220, the USA PSP's value, not the EU's 230.** The EU PSP disc moved
        // this widget down 10px on its own; the PS2 EU disc did not follow it,
        // and 362/220 lands on the resolution ratio while 362/230 misses it by
        // 4%. So the PS2 layout was scaled from the USA-era artwork rather than
        // from the EU PSP release - a small dating fact this test happens to
        // pin, and the reason a naive EU-to-EU comparison looks wrong.
        assert!(
            (362.0 / 220.0 - ps2.1 / psp.1).abs() < 0.005,
            "PRESS START y"
        );
        assert!((119.0 / 72.0 - ps2.1 / psp.1).abs() < 0.01, "the logo's y");
    }

    /// The grid and the display aspect are two numbers, and on the PS2 they
    /// disagree.
    #[test]
    fn a_ps2_grid_is_not_its_own_display_aspect() {
        assert!((Space::PSP.display_aspect - Space::PSP.size.0 / Space::PSP.size.1).abs() < 1e-6);
        let as_grid = Space::PS2.size.0 / Space::PS2.size.1;
        assert!(
            (as_grid - Space::PS2.display_aspect).abs() > 0.09,
            "640/448 is {as_grid}, and taking it for the display aspect is the 7% stretch"
        );
    }

    /// A picture already the screen's shape fills it, on either disc.
    ///
    /// The PS2 half is the case that was wrong and stayed wrong after the
    /// renderer was fixed: `INTRO512.PSS` declares 4:3, the PS2 screen *is* 4:3,
    /// and the old square-pixel comparison boxed it inside itself anyway.
    #[test]
    fn a_picture_of_the_screens_own_shape_is_not_boxed() {
        assert_eq!(
            pillarbox_in(Space::PSP, (480, 272)),
            [0.0, 0.0, 480.0, 272.0]
        );
        let ps2 = pillarbox_in(Space::PS2, (4, 3));
        assert!(
            (ps2[2] - 640.0).abs() < 0.01 && (ps2[3] - 448.0).abs() < 0.01,
            "a 4:3 cut fills a 4:3 screen: {ps2:?}"
        );
        assert!(ps2[0].abs() < 0.01 && ps2[1].abs() < 0.01, "{ps2:?}");
    }

    /// A wider picture is letterboxed, and the bars are in grid units.
    #[test]
    fn a_wider_picture_is_letterboxed_within_the_grid() {
        let ps2 = pillarbox_in(Space::PS2, (16, 9));
        assert!((ps2[2] - 640.0).abs() < 0.01, "full width: {ps2:?}");
        // 4:3 shown, 16:9 wanted, so the height gives up 3/4 of itself.
        let wanted = (4.0 / 3.0) * 448.0 / (16.0 / 9.0);
        assert!((ps2[3] - wanted).abs() < 0.01, "{ps2:?} against {wanted}");
        assert!((ps2[1] - (448.0 - wanted) / 2.0).abs() < 0.01, "centred");
    }

    /// The old two-argument form is the square-pixel case and is unchanged.
    #[test]
    fn the_square_pixel_form_still_agrees_with_itself() {
        for aspect in [(4, 3), (16, 9), (512, 512), (480, 272)] {
            assert_eq!(
                pillarbox(SCREEN, aspect),
                pillarbox_in(Space::PSP, aspect),
                "{aspect:?}"
            );
        }
    }

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
    <Redirect name="AutoRedirect"><Values backward="none" forward="none"></Values><Default goto="Show Logo"></Default></Redirect>
    <Redirect><Values backward="none" forward="start"></Values><Default goto="LogoFMVRedirectScreen"></Default></Redirect>
  </Screen>
  <Screen name="LogoFMVRedirectScreen">
    <Redirect><Values backward="none" forward="none"></Values><Default goto="Show Logo"></Default></Redirect>
  </Screen>
  <Screen type="FEMain" name="Top FE Screen">
    <Screen name="FE Screen">
      <Screen name="Show Logo">
        <Image transition="0"><Values y="72" AutoLoad="true" src="Data\FE\Images\pulse_logo.mip"></Values></Image>
        <Text delay="1" transition="0"><Values align="right" idstring="BOOT_PRESS_START" font="Menu" pulse="true" x="460" y="220" color="0x7FFFFFFF"></Values></Text>
        <Redirect><Values backward="none"></Values><Default goto="RemoveMemoryStickWarning"></Default></Redirect>
        <Redirect><Values backward="none" forward="start"></Values><Default goto="RemoveMemoryStickWarning"></Default></Redirect>
      </Screen>
    </Screen>
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
            frontend.update(FRAME, input, None);
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
            frontend.update(FRAME, &mut input, None);
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
            frontend.update(FRAME, &mut input, None);
            // A skip goes through the redirect screen the XML sends it to, the
            // way the reel leg goes through `DevPubRedirect`. The movie *ending*
            // does not: its `AutoRedirect` is a different exit.
            assert_eq!(
                frontend.machine().current(),
                Some(states::LOGO_FMV_REDIRECT),
                "button {skip} must skip the movie through the redirect screen"
            );

            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
            assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
            assert_eq!(
                frontend.machine().history(),
                [
                    states::LOGO_FMV,
                    states::LOGO_FMV_REDIRECT,
                    states::LANGUAGE_SELECTION,
                ]
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
            frontend.update(FRAME, &mut input, None);
        }
        assert_eq!(frontend.player().frames_produced(), 144);
        assert!(frontend.player().is_paused(), "must pause at 144");

        // Two seconds is 59.94 frames, so 59 is not yet up and 61 is.
        for _ in 0..59 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        }
        assert!(frontend.player().is_paused(), "two seconds is not up yet");
        assert_eq!(
            frontend.player().frames_produced(),
            144,
            "a hold must not advance the movie"
        );
        for _ in 0..2 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
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
        frontend.update(FRAME, &mut input, None);
        assert_eq!(
            frontend.machine().current(),
            Some(states::DEV_PUB_REDIRECT),
            "skipping goes through the redirect, not straight to the picker"
        );
        // The player is not stopped: teardown is a consequence of the transition.
        assert!(!frontend.player().is_finished());

        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
        assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
    }

    #[test]
    fn the_redirect_target_is_cleared_so_it_cannot_fire_twice() {
        let mut frontend = reel(300);
        let mut input = Input::new();

        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input, None);
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
        frontend.update(FRAME, &mut input, None);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
        assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

        assert_eq!(frontend.selected(), 0);
        input.begin_frame(1 << button::DOWN);
        frontend.update(FRAME, &mut input, None);
        assert_eq!(frontend.selected(), 1);

        input.begin_frame(0);
        input.begin_frame(1 << button::UP);
        frontend.update(FRAME, &mut input, None);
        assert_eq!(frontend.selected(), 0);

        input.begin_frame(0);
        input.begin_frame(1 << button::UP);
        frontend.update(FRAME, &mut input, None);
        assert_eq!(
            frontend.selected(),
            2,
            "up from the first wraps to the last"
        );
    }

    /// Walks from the picker to `Show Logo`, leaving the machine there.
    fn pick_a_language(frontend: &mut Frontend, input: &mut Input) {
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, input, None);
        input.begin_frame(0);
        frontend.update(FRAME, input, None);

        input.begin_frame(1 << button::DOWN);
        frontend.update(FRAME, input, None);
        input.begin_frame(0);
        input.begin_frame(1 << button::CROSS);
        frontend.update(FRAME, input, None);
    }

    #[test]
    fn picking_a_language_shows_the_logo_rather_than_launching() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);

        assert_eq!(frontend.chosen(), Some("German"));
        assert!(
            frontend.machine().is(states::SHOW_LOGO),
            "the picker's own exit is PRESS START, not the menus"
        );
        assert!(
            !frontend.is_finished(),
            "the front end is not done until Show Logo is pressed through"
        );
    }

    #[test]
    fn start_on_show_logo_launches_the_game() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);

        input.begin_frame(0);
        input.begin_frame(1 << button::START);
        let events = frontend.update(FRAME, &mut input, None);

        assert!(frontend.is_finished());
        assert!(events.contains(&Event::Enter(states::LAUNCH_GAME.into())));
        assert_eq!(
            frontend.machine().history(),
            [
                states::LOGO_FMV,
                states::LOGO_FMV_REDIRECT,
                states::LANGUAGE_SELECTION,
                states::SHOW_LOGO,
                states::LAUNCH_GAME,
            ]
        );
    }

    #[test]
    fn show_logo_waits_and_ignores_every_button_but_start() {
        // The screen's XML has one `forward="start"` redirect and no timer, so
        // neither time passing nor the buttons `LogoFMV` itself accepts may move
        // it on. Cross is the one that matters: it skips the movie, and it is
        // also the button that was just pressed to pick a language.
        for held in [button::CROSS, button::CIRCLE, button::DOWN] {
            let mut frontend = frontend(300);
            let mut input = Input::new();
            pick_a_language(&mut frontend, &mut input);

            for _ in 0..600 {
                input.begin_frame(0);
                frontend.update(FRAME, &mut input, None);
                input.begin_frame(1 << held);
                frontend.update(FRAME, &mut input, None);
            }
            assert!(
                frontend.machine().is(states::SHOW_LOGO),
                "button {held} must not leave Show Logo"
            );
            assert!(!frontend.is_finished());
        }
    }

    #[test]
    fn show_logo_draws_the_backdrop_under_its_own_widgets() {
        let mut frontend = frontend(300);
        // 270 frames at the PSP's own rate, which is what `Backdrop.PMF` is.
        frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);
        assert!(frontend.machine().is(states::SHOW_LOGO));

        let draws = frontend.draw_list();
        // Under the widgets and over the clear, because the list is painted in
        // its own order: black fill, backdrop, then whatever the screen has.
        assert!(matches!(draws[0], Draw::Fill { .. }));
        let Draw::Video { rect, source, .. } = draws[1] else {
            panic!("Show Logo must draw the backdrop second: {draws:?}");
        };
        assert_eq!(
            source,
            Video::Backdrop,
            "the intro is over by here; naming the wrong movie draws a picture rather than failing"
        );
        assert_eq!(
            rect,
            [0.0, 0.0, SCREEN.0, SCREEN.1],
            "a .PMF fills the screen"
        );
        assert!(
            draws[2..].iter().any(|d| matches!(d, Draw::Text { .. })),
            "and PRESS START on top of it: {draws:?}"
        );
    }

    #[test]
    fn the_backdrop_is_already_running_by_the_time_show_logo_is_reached() {
        // It is `autostart` on a screen the boot opens long before this build
        // draws it, so it must not be sitting at frame zero when it appears.
        let mut frontend = frontend(300);
        frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
        let mut input = Input::new();

        for _ in 0..200 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        }
        let running = frontend.backdrop().expect("a backdrop was set").frame();
        assert!(running > 0, "the playhead must advance during LogoFMV");

        // And it loops rather than ending, so it is still there after its own
        // length has gone by twice over.
        for _ in 0..600 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        }
        let player = frontend.backdrop().expect("a backdrop was set");
        assert!(!player.is_finished(), "repeat=true has no end to run off");
        assert!(player.frame() < 270, "and it wraps inside its own length");
    }

    /// **The backdrop is tick-clocked, whatever the intro's sound is doing.**
    ///
    /// ADR-0019's rule is per movie, and the two movies here are not the same
    /// movie: the intro has a track and `Backdrop.PMF` has none - it is
    /// video-only *and* its `FE Screen` widget is `sound="false"`. The playhead
    /// passed into `update` belongs to the intro, which plays over the top of
    /// this loop for the whole boot sequence, so a backdrop that read it would
    /// be paced by a movie that ends - and would stop dead when it did.
    ///
    /// Asserted as an equality against the same run with no playhead at all,
    /// rather than as a range, because the property is that the value is
    /// *ignored* rather than that its effect is small. An absurd playhead is
    /// used for the same reason: 600 seconds is 65 times round this loop, so a
    /// backdrop that read it could not accidentally agree.
    /// **The intro is on the audio clock, and this is the only test that can
    /// tell.** In every headless run the mixer is advanced exactly
    /// `sample_rate / 60` frames a tick, so the audio playhead and the tick
    /// clock are numerically identical and every capture measurement agrees
    /// with both. Drop the playhead from `update_logo_fmv` and nothing else in
    /// the suite notices.
    ///
    /// So the clocks are made to disagree: `dt` is zero and the sound is five
    /// seconds in. A tick-clocked player has not moved; an audio-clocked one is
    /// on frame 149.
    #[test]
    fn the_movie_is_paced_by_its_sound_rather_than_by_the_tick() {
        // Both legs, because they are two different methods reaching the same
        // `advance_movie` and either could be the one that loses the argument.
        for (state, mut frontend) in [
            (states::LOGO_FMV, frontend(1200)),
            (states::INTRO_MOVIE, reel(1200)),
        ] {
            assert!(frontend.machine().is(state), "the leg under test");

            let mut input = Input::new();
            input.begin_frame(0);
            frontend.update(0.0, &mut input, None);
            assert_eq!(
                frontend.player().frame(),
                0,
                "{state}: no time and no sound is no movement"
            );

            input.begin_frame(0);
            frontend.update(0.0, &mut input, Some(5.0));
            assert_eq!(
                frontend.player().frame(),
                (5.0 * 30_000.0 / 1001.0) as usize,
                "{state}: the sound is what moved the picture, not the tick"
            );
        }
    }

    #[test]
    fn the_backdrop_ignores_the_movies_audio_clock() {
        let run = |playhead: Option<f64>| {
            let mut frontend = frontend(300);
            frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
            let mut input = Input::new();
            for tick in 0..400 {
                input.begin_frame(0);
                // Growing, the way a real playhead does, rather than one value
                // repeated - a backdrop reading a constant would look stuck
                // rather than wrong.
                frontend.update(FRAME, &mut input, playhead.map(|s| s * f64::from(tick)));
            }
            let player = frontend.backdrop().expect("a backdrop was set");
            (player.position(), player.frame())
        };

        let ticked = run(None);
        assert_eq!(ticked.0, 400, "the tick clock is the one it is on");
        assert_eq!(run(Some(1.5)), ticked, "an audio clock must change nothing");
        assert_eq!(run(Some(0.0)), ticked, "and neither must a stalled one");
    }

    #[test]
    fn the_backdrops_draw_carries_a_position_that_outgrows_its_own_loop() {
        // What a `movie::Feed` is asked with. It decodes forward forever, so on
        // the second time round it holds positions 270..274 while `frame` has
        // gone back to 0 - and a draw carrying only `frame` takes nothing from
        // it from that moment on, which froze the picture nine seconds in.
        let mut frontend = frontend(300);
        frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);
        assert!(frontend.machine().is(states::SHOW_LOGO));

        for _ in 0..300 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        }

        let draws = frontend.draw_list();
        let Draw::Video {
            frame,
            position,
            source: Video::Backdrop,
            ..
        } = draws[1]
        else {
            panic!("Show Logo must draw the backdrop second: {draws:?}");
        };
        assert!(position >= 270, "the loop has been round once by here");
        assert_eq!(
            frame,
            (position % 270) as usize,
            "the frame is the position wrapped, and the draw carries both"
        );
        assert_eq!(
            position,
            frontend.backdrop().expect("a backdrop was set").position(),
            "and it is the playhead's own position, not a second count of it"
        );
    }

    /// Every screen between the intro and the menus draws the one loop.
    ///
    /// `Launch Game` is the one that matters: the composition root draws it
    /// **once and then stalls** building the menus, so it is on screen for the
    /// whole of that load. Without the backdrop it was a few hundred
    /// milliseconds of black between `Show Logo` and the menus - the flicker on
    /// the START press, and the half of it no amount of feed bookkeeping could
    /// have fixed, because a screen that emits no video draw has nothing to
    /// take a frame for.
    #[test]
    fn the_screens_after_the_intro_all_sit_on_the_backdrop() {
        for state in [
            states::LANGUAGE_SELECTION,
            states::SHOW_LOGO,
            states::LAUNCH_GAME,
        ] {
            let mut frontend = frontend(300);
            frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
            let mut input = Input::new();
            if state == states::LANGUAGE_SELECTION {
                // START skips the movie, and the redirect that follows it
                // spends one update on its way to the picker.
                input.begin_frame(1 << button::START);
                frontend.update(FRAME, &mut input, None);
                input.begin_frame(0);
                frontend.update(FRAME, &mut input, None);
            } else {
                pick_a_language(&mut frontend, &mut input);
                if state == states::LAUNCH_GAME {
                    // `Show Logo` leaves on START and on nothing else.
                    input.begin_frame(0);
                    input.begin_frame(1 << button::START);
                    frontend.update(FRAME, &mut input, None);
                }
            }
            assert!(
                frontend.machine().is(state),
                "meant to be on {state}, got {:?}",
                frontend.machine().current()
            );

            let draws = frontend.draw_list();
            assert!(
                matches!(draws[0], Draw::Fill { .. }),
                "{state} still clears to black first: {draws:?}"
            );
            assert!(
                matches!(
                    draws[1],
                    Draw::Video {
                        source: Video::Backdrop,
                        ..
                    }
                ),
                "{state} must draw the backdrop under its own widgets: {draws:?}"
            );
        }
    }

    #[test]
    fn the_backdrops_playhead_is_handed_on_rather_than_left_behind() {
        // `Show Logo` and the menus are two screens in front of one playback.
        // Whoever opens the menus takes this player as it stands; anything that
        // rebuilt it would put the picture back to the start of the loop.
        let mut frontend = frontend(300);
        frontend.set_backdrop(270, crate::movie::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);
        for _ in 0..500 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        }

        let running = frontend.backdrop().expect("a backdrop was set").position();
        assert!(running > 0, "the playhead must have got somewhere");

        let handed = frontend.take_backdrop().expect("a backdrop was set");
        assert_eq!(
            handed.position(),
            running,
            "the menus continue the playback rather than starting one"
        );
        assert!(!handed.is_finished(), "and it still loops");
        assert!(
            frontend.backdrop().is_none(),
            "the sequence is over; two playheads on one feed is the bug this fixes"
        );
        // And with it gone the sequence stops asking for a picture, rather than
        // drawing one nobody is advancing.
        let draws = frontend.draw_list();
        assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
    }

    #[test]
    fn no_backdrop_leaves_show_logo_on_black() {
        // A source without one, `--no-video`, and a backdrop whose planes are
        // not the intro's all land here, and none of them may emit a video draw
        // the renderer has no feed for.
        let mut frontend = frontend(300);
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);

        let draws = frontend.draw_list();
        assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
        assert!(matches!(draws[0], Draw::Fill { .. }));
    }

    #[test]
    fn show_logo_draws_the_pulse_logo_and_its_press_start_line() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);
        assert!(frontend.machine().is(states::SHOW_LOGO));

        let draws = frontend.draw_list();
        // The `Text` widget's own coordinates, colour and alignment, from the
        // USA disc's `Skin.xml`. `y` is 220 there and 230 on the EU disc, which
        // drops the `BOOT_LEGAL` line above it.
        let text = draws
            .iter()
            .find_map(|d| match d {
                Draw::Text {
                    x,
                    y,
                    align,
                    color,
                    text,
                    ..
                } => Some((*x, *y, *align, *color, text.clone())),
                _ => None,
            })
            .expect("Show Logo draws its text widget");
        assert_eq!(text.0, 460.0);
        assert_eq!(text.1, 220.0);
        assert_eq!(text.2, Align::Right);
        assert_eq!(text.3, argb_to_rgba(0x7fff_ffff));
        assert_eq!(
            text.4, "BOOT_PRESS_START",
            "with no string table loaded the id stands in for its own string"
        );

        // The logo itself needs a sprite sheet, which these tests do not build.
        // What must hold either way is that the screen's `Image` was found and
        // nothing else crept in: a black fill, then the text.
        assert!(matches!(draws[0], Draw::Fill { .. }));
        assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
    }

    #[test]
    fn circle_does_not_select() {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << button::START);
        frontend.update(FRAME, &mut input, None);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);

        input.begin_frame(1 << button::CIRCLE);
        frontend.update(FRAME, &mut input, None);
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
        frontend.update(FRAME, &mut input, None);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);

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
        frontend.update(FRAME, &mut input, None);
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
        input.begin_frame(1 << button::CROSS);
        frontend.update(FRAME, &mut input, None);
        assert_eq!(frontend.chosen(), None);
        let _ = frontend.draw_list();
    }
}
