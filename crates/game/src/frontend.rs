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
//! **Pure diverges in the same shape and at the same point.** Its chain ends on
//! [`pure_states::TITLE_SCREEN`] - the WipEout Pure logo and PRESS START - and
//! that screen's own `TitleRedirect` goes to `Profile Manager`, a screen this
//! build does not have any more than it has Pulse's four Memory Stick ones. So
//! START there fires [`states::LAUNCH_GAME`] too, and the two titles reach the
//! menus by the same edge from their own last boot screen. See
//! [`Frontend::update_title_screen`] and `docs/architecture/pure-boot.md`.
//!
//! And [`states::LAUNCH_GAME`] goes straight into a race. The original has a main
//! menu in between - the root XML's `LoadXML` list pulls in
//! `MainMenu_Definition.xml` - and none of it is built, so a state whose whole
//! content was the words LAUNCH GAME is used as the way in instead. This module
//! does not know that: it fires the transition the original's own literal names
//! and stops, and what happens next is the composition root's business.

use crate::input::{Input, button};
use crate::language::{Language, StringTable};
use crate::screen::{Screen, Screens, argb_to_rgba, parse_argb};
use crate::state_machine::{Event, StateMachine};

/// The reel's frame counts and every state name: `oag_pulse::frontend`.
///
/// Both are literals off Pulse's executable and its front-end XML, so they moved
/// to the title package under [ADR-0022]. The state machine that drives them, and
/// every divergence this build makes from the disc's own sequence, stay here - see
/// the module docs.
///
/// [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md
pub use oag_pulse::frontend::{FINISH_FRAME, HOLD_SECONDS, PAUSE_FRAMES, states};
use oag_pure::frontend::states as pure_states;

/// How long `Developer Publisher Screen` takes, as the reel's own length.
///
/// **Derived rather than timed.** The screen plays the dev/pub reel and holds at
/// two of its frames, so its duration is the video's 260 frames at 29.97 fps plus
/// two [`HOLD_SECONDS`] pauses: 12.68 s. A cold boot measured 11-12.5 s at 0.5 s
/// sampling, which rules out the 8.68 s the reel would take without the holds.
///
/// Kept as a documented figure rather than a timer the code reads: nothing counts
/// seconds on this screen any more, the reel running out is what leaves it. See
/// `docs/architecture/pure-boot.md`.
pub const DEVELOPER_PUBLISHER_SECONDS: f64 =
    260.0 / (30000.0 / 1001.0) + 2.0 * oag_pulse::frontend::HOLD_SECONDS;

/// Which leg the sequence boots into.
///
/// **A command-line intent, not a title fact.** It stays in this crate for that
/// reason: which screen either leg starts on is the title's own business and
/// lives in [`oag_title::BootProfile`], while whether `--reel` was given is the
/// composition root's. This enum used to answer both questions, mapping an intent
/// onto *Pulse's* screen names for every source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Leg {
    /// The title's own boot sequence, whatever it opens on.
    #[default]
    LogoFmv,
    /// The off-path dev/pub reel state, for a title that has one. Reached
    /// through `--reel`, and refused by name on a title that does not.
    DevPubReel,
}

/// Whether this build has any behaviour for a screen at all.
///
/// A title's chain names every screen the disc walks, including ones nothing here
/// draws or advances yet - Pure's `Developer Publisher Screen` and
/// `MemoryStickWarning` are both in its chain and neither is implemented. The
/// mechanism steps over them rather than stalling the boot on a screen with no
/// way out, and this is the list it steps by.
///
/// **Adding a state here is a claim that it can be entered and left.** A screen
/// listed but not driven is a hang; a screen driven but not listed is skipped.
#[must_use]
pub fn can_drive(state: &str) -> bool {
    [
        states::LOGO_FMV,
        states::INTRO,
        states::INTRO_MOVIE,
        states::DEV_PUB_REDIRECT,
        states::LOGO_FMV_REDIRECT,
        states::SHOW_LOGO,
        states::LANGUAGE_SELECTION,
        states::LAUNCH_GAME,
        pure_states::DEVELOPER_PUBLISHER,
        pure_states::MEMORY_STICK_WARNING,
        pure_states::FMV_INTRO,
        pure_states::TITLE_SCREEN,
    ]
    .contains(&state)
}

/// One screen the boot walks, and the movie it plays there.
#[derive(Debug, Clone)]
pub struct Step {
    pub state: &'static str,
    pub movie: MoviePlan,
}

/// What `boot::load` resolved about the sequence before the front end existed.
///
/// Every part of this was a `screens.by_name(...)` probe in this file or in
/// `boot.rs` - four of them, three asking "is this Pure?" in different words.
/// They are resolved once now, from the title's own table, and handed in. See
/// [ADR-0023](../../../docs/architecture/adr/0023-boot-sequence-as-title-data.md).
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Every screen this boot walks, in order, already filtered to the ones this
    /// pressing carries and this build can drive.
    ///
    /// **A whole chain rather than a start and one next step.** An earlier shape
    /// here carried exactly `start` and `after_language`, which was enough for
    /// Pulse's three screens and structurally could not express Pure's five: a
    /// single "where does the picker go" field cannot walk the two screens
    /// between Pure's picker and its movie however they are implemented, so the
    /// order came out wrong no matter what was drawn.
    pub steps: Vec<Step>,
    /// The screen whose fills the boot's own screens sit on, if they need one.
    ///
    /// Pure's picker, developer/publisher cards and storage warning are all
    /// children of `Intro Screen` and carry no fill of their own; the real ones
    /// are on that parent's white. `None` for a title whose screens carry their
    /// own backgrounds.
    pub backdrop_parent: Option<&'static str>,
}

impl Sequence {
    /// The screen the boot opens on.
    ///
    /// # Panics
    ///
    /// If there are no steps at all, which `boot::load` cannot produce: a title's
    /// chain is a non-empty compile-time constant and the boot screen itself is
    /// always drivable.
    #[must_use]
    pub fn start(&self) -> &'static str {
        self.steps
            .first()
            .expect("a boot sequence has at least one screen")
            .state
    }
}

/// One boot movie, as the sequence needs to know it.
///
/// The four facts travel together because they are **per movie**, and that is
/// the whole reason this type exists rather than four more arguments: a boot can
/// play more than one movie, they need not be the same shape, and they do not
/// decode or fail together. A single shared `video_aspect` drew the second movie
/// at the first one's aspect - right only by coincidence on the PSP, where both
/// happen to be 480x272, and wrong the moment either changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoviePlan {
    /// How many frames it has. Zero means there is no movie at all, which still
    /// plays a sequence out.
    pub frames: usize,
    pub frame_rate: (u64, u64),
    /// Its own display aspect, as `(width, height)`. A `.PMF` is
    /// square-pixelled, so this is its decoded size and the video quad fills
    /// [`SCREEN`] exactly; a PS2 `.PSS` is not, and this is what keeps its
    /// picture from being stretched. See `crate::movie::Movie::display_aspect`.
    pub aspect: (u32, u32),
    /// Whether there is a picture to draw. False under `--no-video`, with no
    /// `ffmpeg`, and on a source that has no such movie.
    pub has_picture: bool,
}

impl MoviePlan {
    /// A leg with no movie: nothing to draw, nothing to time against.
    #[must_use]
    pub fn none(aspect: (u32, u32)) -> Self {
        Self {
            frames: 0,
            frame_rate: crate::movie::FRAME_RATE,
            aspect,
            has_picture: false,
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

/// The PSP's screen, in the pixels our own layouts are written in.
///
/// **Still a constant, and deliberately.** ADR-0022's stage 4 asked whether this
/// should become per-source, and the answer that came out of doing it is no: a
/// source that authors somewhere else says so through [`Space`], which already
/// carries a grid *and* the display aspect that grid is shown as - two numbers
/// that disagree by 7% on the PS2 and that one `(f32, f32)` cannot hold. Making
/// this per-source would have produced a second, weaker `Space`.
///
/// What was per-source and hardcoded here got routed through [`Space`] instead:
/// the menu backdrop's rect in `crate::main` and `crate::capture`, and the
/// no-movie aspect fallback in `crate::boot`. See [`pillarbox_in`].
///
/// The remaining readers are console facts under
/// [ADR-0004](../../../docs/architecture/adr/0004-asset-pipeline.md) rather than
/// title ones, which is why none of them moved to a title package either:
///
/// - `crate::loading`, whose layout this project authored, in these pixels;
/// - `oag_race::AUTHORED_ASPECT`, where the original's authored field of view is
///   only defined at the PSP's aspect;
/// - [`crate::hud::inside_screen`], which is a **PSP-only** test helper and says
///   so - the PS2's `Arcade_HUD.xml` authors the same layout in a 640x448 grid,
///   reaching `y=435`, so checking it against these numbers would fail on nearly
///   every widget rather than on a parser bug. **It is not a flat scaling**, and
///   the exceptions matter to anyone attempting the sweep; the measurement is on
///   [`crate::hud::inside_screen`] itself.
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
/// The file's own extremes are `x=613 y=415` on the PS2 against `x=460 y=252`
/// on the PSP, and both ratios land on 640/480 and 448/272 to better than a
/// tenth of a percent: 613/460 = 1.3326 against 1.3333, 415/252 = 1.6468
/// against 1.6471. Confidence **95** - the arithmetic is unambiguous and
/// 640x448 is PAL's own frame. What is not independently confirmed is the
/// *display* aspect, which is taken from `crate::display::Aspect::Ps2`'s
/// existing 4:3 rather than measured here.
///
/// # What the grid does not mean
///
/// **The PS2 layout is not the PSP's scaled.** This paragraph used to say it
/// was, "by exactly the resolution ratio", at confidence 95, on the strength of
/// the extremes above and three samples. Measured coordinate by coordinate, 30
/// of the 43 coordinates the two files share land within a pixel of the ratio
/// and **13 do not**: ten widgets the PS2 pressing re-placed by hand, and three
/// `<Animation><Key>` values that are a *travel* rather than a position and are
/// byte-identical across the consoles. Extremes cannot see an exception in the
/// middle - the same reading, of `Arcade_HUD.xml`, was wrong the same way. The
/// measurement is `crates/game/tests/frontend_grid_ground_truth.rs`, and
/// [`crate::hud::inside_screen`] is where the mixed meaning of `x` matters.
///
/// One trap that measurement pins: the PRESS START text is at `y=220` on the
/// **USA** PSP pressing and `y=230` on the EU one, and the PS2's 362 scales
/// from 220. Since the only PS2 disc here is a EU one, the natural EU-to-EU
/// comparison makes that coordinate look 4% off for a reason that has nothing
/// to do with the console.
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
    /// The first boot movie, as the sequence needs to know it.
    ///
    /// Every *other* screen's movie is looked up from [`Self::steps`] by
    /// [`Self::movie_of`]; this one is kept because the overlay's default and the
    /// no-picture player length are decided from it before any step is entered.
    first: MoviePlan,
    /// The grid this source's XML places widgets in, and what it is shown as.
    ///
    /// [`Space::PSP`] unless a caller says otherwise, because every screen this
    /// file's own tests parse is PSP XML. `boot::load` sets it from the
    /// archives' platform - see [`Space`] for why the PS2 needs it and what
    /// goes wrong silently without it.
    space: Space,
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
    /// The chain this boot walks, resolved from the title's own table. See
    /// [`Sequence::steps`].
    steps: Vec<Step>,
    /// The screen whose fills this boot's screens sit on, if they need one.
    backdrop_parent: Option<&'static str>,
    /// How long the current screen has been on, for the ones that leave on a
    /// timer rather than on a press. See [`Frontend::update_timed`].
    on_screen_for: f64,
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
        let screen = (SCREEN.0 as u32, SCREEN.1 as u32);
        // Pulse's own chain, which is what every in-file test here parses.
        Self::booting(
            Sequence {
                steps: vec![
                    Step {
                        state: states::LOGO_FMV,
                        movie: MoviePlan {
                            frames: movie_frames,
                            frame_rate: crate::movie::FRAME_RATE,
                            aspect: screen,
                            has_picture,
                        },
                    },
                    Step {
                        state: states::LANGUAGE_SELECTION,
                        movie: MoviePlan::none(screen),
                    },
                    Step {
                        state: states::SHOW_LOGO,
                        movie: MoviePlan::none(screen),
                    },
                ],
                backdrop_parent: None,
            },
            screens,
            strings,
            languages,
            placements,
        )
    }

    /// Builds the boot sequence a title's own table resolved to.
    #[must_use]
    pub fn booting(
        sequence: Sequence,
        screens: Screens,
        strings: StringTable,
        languages: Vec<Language>,
        placements: Vec<(String, crate::sprite::Placed)>,
    ) -> Self {
        let start = sequence.start();
        let first = sequence.steps[0].movie;
        let Sequence {
            steps,
            backdrop_parent,
        } = sequence;
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
            // Every state this build can drive, whichever title is opened -
            // harmless on a source that never fires into one, and `can_drive` is
            // the same list stated once for the mechanism that skips the rest.
            pure_states::DEVELOPER_PUBLISHER,
            pure_states::MEMORY_STICK_WARNING,
            pure_states::TITLE_SCREEN,
            pure_states::FMV_INTRO,
        ]);
        // Every screen the XML declares becomes a state, so a transition name
        // recovered from the data resolves without being listed here.
        let paths: Vec<String> = screens.screens.iter().map(|s| s.path.clone()).collect();
        machine.register_all(paths.iter().map(String::as_str));

        // With no movie this build can read a picture out of, a leg that plays
        // one still has to run and end: `FINISH_FRAME` is Pulse's own reel
        // length, a bounded stand-in.
        //
        // **A leg that plays no movie by design gets nothing to run instead.**
        // Borrowing Pulse's 260 there would give a picker-first boot a player
        // counting down a length belonging to another title's reel. Nothing reads
        // it - `confirm_language` rebuilds the player for the second movie before
        // any movie state is entered - but a number that is never read is still a
        // number no one can explain later.
        let frames = match (first.frames, first.has_picture) {
            (0, false) if start == states::LANGUAGE_SELECTION => 0,
            (0, _) => FINISH_FRAME,
            (frames, _) => frames,
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
            player: crate::movie::Player::new(frames, false, first.frame_rate),
            first,
            backdrop: None,
            // Without a picture the movie is a black screen for as long as it
            // runs - forty seconds for the disc's own intro - so the counter is
            // the only sign it is running. With one it is clutter, and the
            // pacing can be read off the picture instead.
            overlay: !first.has_picture,
            hold: Hold::None,
            held_for: 0.0,
            dev_pub_redirect: Some(states::DEV_PUB_REDIRECT.to_string()),
            acted: Vec::new(),
            notes: Vec::new(),
            finished: false,
            steps,
            backdrop_parent,
            on_screen_for: 0.0,
        };
        frontend.machine.transition_to(start);
        frontend
    }

    /// The movie a named screen plays, or a silent, pictureless one.
    fn movie_of(&self, state: &str) -> MoviePlan {
        self.steps
            .iter()
            .find(|step| step.state == state)
            .map_or(MoviePlan::none(self.first.aspect), |step| step.movie)
    }

    /// The step after whichever screen is current, if the chain has one.
    fn next_step(&self) -> Option<&Step> {
        let current = self.machine.current()?;
        let at = self.steps.iter().position(|step| step.state == current)?;
        self.steps.get(at + 1)
    }

    /// Leaves the current screen for the next one in the chain.
    ///
    /// **The one place the sequence moves forward**, so the order comes from the
    /// title's own table at every step rather than from a `fire` spelled out in
    /// each `update_*`. Those spellings are what made Pure's order wrong: each
    /// one was individually defensible and together they described Pulse's chain.
    ///
    /// The player is rebuilt for the next screen's own movie when it has one,
    /// which is safe because the previous screen's is idle by then - see
    /// [`Self::is_playing_movie`].
    ///
    /// Nothing happens at the end of the chain. On Pure that is `Title Screen`,
    /// which the chain does not advance *past* - it is left by
    /// [`Self::update_title_screen`] on a START press, the same way Pulse's own
    /// last step `Show Logo` is, and for the same reason: both hand off to the
    /// menus rather than to another boot screen.
    fn advance(&mut self, why: &str) {
        let Some(step) = self.next_step().cloned() else {
            return;
        };
        self.notes.push(format!("{why}, firing {}", step.state));
        if step.movie.frames > 0 {
            self.player =
                crate::movie::Player::new(step.movie.frames, false, step.movie.frame_rate);
        }
        self.on_screen_for = 0.0;
        self.machine.fire(step.state);
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

    /// Whether the sequence is on one of its movie screens.
    ///
    /// What a caller holding the movie's sound asks to know when to stop it,
    /// and the reason it lives here is that it has to be true of **every** leg
    /// and of every way out of them. The movie ends on its own on one path and
    /// is cut short by START on four others - all five leave the state, and
    /// none of them finishes the player - so "is the player finished" is the
    /// wrong question and would leave the intro's sound playing under the
    /// language picker.
    ///
    /// [`pure_states::FMV_INTRO`] is here for the same reason the two Pulse
    /// states are, and leaving it out was a real bug rather than an omission of
    /// coverage: `main.rs` stops the movie's sound on every tick this returns
    /// `false`, so Pure's second boot movie was torn down on the first tick of
    /// its own state and then paced by `dt` instead of by its own audio, every
    /// run.
    #[must_use]
    pub fn is_playing_movie(&self) -> bool {
        self.machine.is(states::LOGO_FMV)
            || self.machine.is(states::INTRO_MOVIE)
            || self.machine.is(pure_states::DEVELOPER_PUBLISHER)
            || self.machine.is(pure_states::FMV_INTRO)
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

        self.on_screen_for += dt;

        if self.machine.is(states::LOGO_FMV) {
            self.update_logo_fmv(dt, input, movie_playhead);
        } else if self.machine.is(states::INTRO_MOVIE) {
            self.update_intro(dt, input, movie_playhead);
        } else if self.machine.is(states::DEV_PUB_REDIRECT)
            || self.machine.is(states::LOGO_FMV_REDIRECT)
        {
            // A redirect state's whole job is to leave. Both of these are real
            // screens with nothing in them but one `forward="none"` redirect, and
            // neither is a chain step of its own: the boot movie fires into one
            // and it leaves for whatever followed that movie. So the target is the
            // step after the *boot* step - which is where both are only ever
            // entered from, on either leg.
            if let Some(after_boot) = self.steps.get(1).map(|step| step.state) {
                self.machine.fire(after_boot);
            }
        } else if self.machine.is(pure_states::DEVELOPER_PUBLISHER) {
            self.update_developer_publisher(dt, input, movie_playhead);
        } else if self.machine.is(pure_states::MEMORY_STICK_WARNING) {
            self.update_memory_stick_warning(input);
        } else if self.machine.is(states::LANGUAGE_SELECTION) {
            self.update_language_selection(input);
        } else if self.machine.is(states::SHOW_LOGO) {
            self.update_show_logo(input);
        } else if self.machine.is(pure_states::FMV_INTRO) {
            self.update_fmv_intro(dt, input, movie_playhead);
        } else if self.machine.is(pure_states::TITLE_SCREEN) {
            self.update_title_screen(input);
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
            self.advance("the movie ended");
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

    /// The screen this source's boot lands on after the language picker.
    ///
    /// Read out of the title's own chain rather than probed for. This used to be a
    /// three-branch probe over `Show Logo`, then `FMV Intro`, then `Title Screen`,
    /// which was a title guess dressed as a capability check: it happened to give
    /// the right answer for the two titles that existed, and would have gone on
    /// happening to until it did not.
    #[must_use]
    pub fn language_confirm_target(&self) -> &'static str {
        self.steps
            .iter()
            .position(|step| step.state == states::LANGUAGE_SELECTION)
            .and_then(|at| self.steps.get(at + 1))
            .map_or(states::SHOW_LOGO, |step| step.state)
    }

    /// Every screen in the chain that plays a movie, in order.
    ///
    /// What a caller holding those movies' frames and sound needs, so it can hand
    /// each over on the tick its screen is entered. **Not "the boot screen and the
    /// one after the picker"**: on Pure neither movie is on the boot step - the
    /// reel plays on the developer/publisher screen and the FMV two steps later -
    /// so any rule phrased in terms of a particular screen gets one of the two
    /// titles wrong.
    #[must_use]
    pub fn movie_states(&self) -> Vec<&'static str> {
        self.steps
            .iter()
            .filter(|step| step.movie.frames > 0)
            .map(|step| step.state)
            .collect()
    }

    /// Takes the highlighted language and leaves for the next screen in the chain.
    fn confirm_language(&mut self) {
        let language = self.languages[self.selected].clone();
        let disc_goto = self.language_auto_redirect().map(str::to_string);
        let target = self.language_confirm_target();
        self.chosen = Some(language.name.clone());
        if let Some(goto) = disc_goto
            && goto != target
        {
            self.notes.push(format!(
                "note: the disc's own LanguageAutoRedirect goes to {goto}, not {target}"
            ));
        }
        let selected = format!(
            "language {} ({}) selected",
            language.name, language.native_name
        );
        self.advance(&selected);
    }

    /// `Developer Publisher Screen`: the dev/pub reel, with its frame holds.
    ///
    /// **This is the same reel state Pulse's `Intro Screen->IntroMovie1` is**, and
    /// finding that out corrected a wrong reading recorded here for a while. The
    /// screen declares no widgets, and this build concluded its two cards were
    /// therefore drawn by engine code - a false dichotomy, since a Pure child
    /// screen inherits its parent's widgets, which is exactly how the picker gets
    /// its background. It plays `IntroMovie1` off `Intro Screen`, and the "two
    /// teletyped phases" are frames 144 and 231 of that video, matched
    /// pixel-for-pixel against the decoded reel.
    ///
    /// The holds are Pure's own, not borrowed: `144`, `231` and `260` are three
    /// `li` immediates in Pure's `BOOT.BIN`, the only such site in the binary, each
    /// starting an identical pause-and-reload block. [`HOLD_SECONDS`] is still
    /// Pulse's measurement - Pure loads its hold duration from a global rather
    /// than an immediate, so the *value* has not been read out of Pure.
    ///
    /// See `docs/architecture/pure-boot.md`.
    fn update_developer_publisher(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        for button in [button::START, button::CROSS] {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.advance("the reel was skipped");
                return;
            }
        }

        // Before the hold, so the audio clock can discount it - see
        // `Self::update_intro`, which does this for the same reel.
        self.advance_movie(dt, playhead);

        if let Hold::Held { finish } = self.hold {
            self.held_for += dt;
            if self.held_for >= HOLD_SECONDS {
                self.hold = Hold::None;
                self.held_for = 0.0;
                self.player.resume();
                if finish {
                    self.advance("the reel finished");
                }
            }
            return;
        }

        let produced = self.player.frames_produced();
        if produced >= FINISH_FRAME && !self.acted.contains(&FINISH_FRAME) {
            self.acted.push(FINISH_FRAME);
            self.begin_hold(true);
            return;
        }
        for &frame in &PAUSE_FRAMES {
            if produced >= frame && !self.acted.contains(&frame) {
                self.acted.push(frame);
                self.begin_hold(false);
                self.notes.push(format!(
                    "the reel paused at frame {frame} for {HOLD_SECONDS}s"
                ));
                return;
            }
        }

        // A reel shorter than its own finish frame still has to end.
        if self.player.is_finished() {
            self.advance("the reel ended");
        }
    }

    /// `MemoryStickWarning`: the storage disclaimer, waiting on cross.
    ///
    /// **Cross and only cross, and no timeout.** `MemoryStickRedirect` carries
    /// `StartEnabled="true"` and the screen reads "PRESS X TO CONTINUE"; a cold
    /// boot sat on it indefinitely until cross was sent, which is how the two
    /// screens before it were found at all.
    ///
    /// Its own text is deliberately not the disc's - see
    /// [`pure_states::MEMORY_STICK_WARNING`] and [`Self::draw_storage_warning`].
    fn update_memory_stick_warning(&mut self, input: &mut Input) {
        if input.is_pressed(button::CROSS) {
            input.consume_press(button::CROSS);
            self.advance("the storage warning was acknowledged");
        }
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

    /// `Title Screen`: Pure's logo, PRESS START, and the hand-off to the menus.
    ///
    /// **The button is evidenced; the destination is this build's.** The screen
    /// declares a `Text` widget whose `idstring` is the literal `PRESS START`
    /// (`Data\Plugins\PI001\GUI\Skin.xml`, both pressings), drawn beside a
    /// blinking `_` in the same face, and a PPSSPP session watched the real
    /// firmware sit on it until START - see [`pure_states::TITLE_SCREEN`], which
    /// carries that observation and its confidence of 90. So reading START here
    /// is a measurement.
    ///
    /// Where it *goes* is not. `TitleRedirect` carries no `forward` attribute at
    /// all - unlike `Show Logo`'s explicit `forward="start"` - and its
    /// `Default goto` names `Profile Manager`, a screen this build does not
    /// have. Firing [`states::LAUNCH_GAME`] instead is a **deliberate
    /// divergence**, the same one [`Self::update_show_logo`] makes on Pulse and
    /// for the same reason: `Launch Game` is where this project's own menu tree
    /// (`assets/ui/menu.toml`, see `docs/architecture/menus.md`) is opened, and
    /// the alternative is a title that boots and then cannot be left. Recorded
    /// in `docs/architecture/pure-boot.md` beside Pulse's.
    ///
    /// Note this leaves the screen's *own* unnamed redirect unfired, exactly as
    /// `Show Logo`'s is: a redirect with no button and no name is something the
    /// original's code fires, and what fires it has not been found on either
    /// title. Guessing a hidden timeout here would be inventing a duration.
    fn update_title_screen(&mut self, input: &mut Input) {
        if input.is_pressed(button::START) {
            input.consume_press(button::START);
            self.notes.push(format!(
                "START pressed on {}, firing {}",
                pure_states::TITLE_SCREEN,
                states::LAUNCH_GAME
            ));
            self.machine.fire(states::LAUNCH_GAME);
        }
    }

    /// `FMV Intro`: Pure's second boot movie, on the way to
    /// [`pure_states::TITLE_SCREEN`].
    ///
    /// START and CROSS both skip it, matching [`pure_states::FMV_INTRO`]'s own
    /// doc comment - the runtime-observed button. The screen's own
    /// `FMVRedirect` has no frame counter or delay of its own to hold against,
    /// so unlike [`Self::update_logo_fmv`] there is no *held* pause here - the
    /// only other way out is the movie running out on its own, the same
    /// `player.is_finished()` check `Self::update_logo_fmv` ends on.
    fn update_fmv_intro(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        for button in [button::START, button::CROSS] {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.advance("the video was skipped");
                return;
            }
        }

        self.advance_movie(dt, playhead);
        if self.player.is_finished() {
            self.advance("the video ended");
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
            if self.first.has_picture {
                out.push(Draw::Video {
                    rect: pillarbox_in(self.space, self.first.aspect),
                    frame: self.player.frame(),
                    // The same number as `frame` here, the intro being played
                    // once through rather than looped, and carried anyway so a
                    // consumer never has to know which movie it is holding.
                    position: self.player.position(),
                    source: Video::Intro,
                });
            }
            self.insert_movie_counter(&mut out, self.first.has_picture);
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

        if self.machine.is(pure_states::FMV_INTRO) {
            // The screen itself carries no widgets - see
            // `pure_states::FMV_INTRO`'s own doc comment - so this is the
            // video and nothing else, the same shape `LogoFMV`'s own draw
            // has above. `self.player` was rebuilt for this movie's own
            // frame count in `Self::confirm_language`, and `Video::Intro` is
            // reused rather than a third `Video` variant added - see that
            // reassignment's own comment for why reuse is safe here: the
            // first movie's player is idle by the time this state is
            // reachable at all.
            let plan = self.movie_of(pure_states::FMV_INTRO);
            let mut out = self.draw_screen(pure_states::FMV_INTRO);
            if plan.has_picture {
                out.push(Draw::Video {
                    // **This movie's own aspect, not the first one's.** The two
                    // need not be the same shape, and a shared field drew this
                    // one pillarboxed for the other - correct only while both
                    // happen to be 480x272, which is exactly the kind of
                    // coincidence that survives review and then breaks.
                    rect: pillarbox_in(self.space, plan.aspect),
                    frame: self.player.frame(),
                    position: self.player.position(),
                    source: Video::Intro,
                });
            }
            // This leg needs the counter more than `LogoFMV` does, not less: on
            // Pure it is the only movie the boot draws, so without `ffmpeg` the
            // whole leg is a blank screen for as long as the movie would have
            // run, with nothing to say it is progressing.
            self.insert_movie_counter(&mut out, plan.has_picture);
            self.insert_backdrop(&mut out);
            return out;
        }

        if self.machine.is(pure_states::DEVELOPER_PUBLISHER) {
            // The dev/pub reel, played off the parent's `IntroMovie1` widget -
            // the cards are frames of this video, not text this build draws. See
            // [`Self::update_developer_publisher`].
            let plan = self.movie_of(pure_states::DEVELOPER_PUBLISHER);
            self.insert_backdrop_parent_fills(&mut out);
            if plan.has_picture {
                out.push(Draw::Video {
                    rect: pillarbox_in(self.space, plan.aspect),
                    frame: self.player.frame(),
                    position: self.player.position(),
                    source: Video::Intro,
                });
            }
            self.insert_movie_counter(&mut out, plan.has_picture);
            self.insert_backdrop(&mut out);
            return out;
        }

        if self.machine.is(pure_states::MEMORY_STICK_WARNING) {
            // **Not `draw_screen`, deliberately.** This screen's widgets *are* in
            // the XML, and drawing them would put the disc's Memory-Stick-removal
            // strings on screen - the one thing this build has decided not to say.
            // Its geometry, colours and rules are reproduced from those widgets in
            // `draw_storage_warning`, with wording that is true here.
            self.insert_backdrop_parent_fills(&mut out);
            self.draw_storage_warning(&mut out);
            self.insert_backdrop(&mut out);
            return out;
        }

        if self.machine.is(pure_states::TITLE_SCREEN) {
            // Pure's own counterpart to `Show Logo` above, and drawn the same
            // way: its widgets and nothing else. See
            // [`pure_states::TITLE_SCREEN`] for what advancing past it would
            // need that is not yet evidenced.
            let mut out = self.draw_screen(pure_states::TITLE_SCREEN);
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

    /// The fills of the screen this boot's own screens sit on, if it names one.
    ///
    /// Pure's picker, developer/publisher cards and storage warning are all
    /// children of `Intro Screen` and declare no background of their own; the real
    /// ones are on that parent's white, and without this they would draw on the
    /// black this build clears to. Which screen - if any - is the title's own
    /// table's answer, not a probe.
    fn insert_backdrop_parent_fills(&self, out: &mut Vec<Draw>) {
        let Some(parent) = self
            .backdrop_parent
            .and_then(|name| self.screens.by_name(name))
        else {
            return;
        };
        for &fill in &parent.fills {
            out.push(Draw::Fill {
                rect: [0.0, 0.0, self.space.size.0, self.space.size.1],
                color: argb_to_rgba(fill),
            });
        }
    }

    /// This build's own storage warning, at the disc's own geometry.
    ///
    /// **The wording is deliberately not the disc's.** Its six `MSInfo`/`MSWarning`
    /// strings are about a Memory Stick Duo being physically removed mid-write;
    /// this is a reimplementation on hardware where storage is assumed present, so
    /// the screen keeps its structure, its colours, its rules and its cross gate,
    /// and says what is actually true here instead. A product decision, recorded
    /// in [`pure_states::MEMORY_STICK_WARNING`] so it is not "fixed" back to the
    /// disc's strings by someone reading the XML.
    ///
    /// Everything geometric *is* the disc's: the two rules at y=10 and y=240, the
    /// body at x=15 from y=30, the prompt at y=245, and the
    /// `MSWarningColour1`/`MSWarningColour2`/`MSWarningScale` globals the screen's
    /// own widgets reference.
    fn draw_storage_warning(&self, out: &mut Vec<Draw>) {
        let global = |name: &str, fallback: u32| {
            self.screens
                .globals
                .get(name)
                .and_then(|value| parse_argb(value))
                .unwrap_or(fallback)
        };
        let heading = argb_to_rgba(global("MSWarningColour1", 0xFF00_AEEF));
        let body = argb_to_rgba(global("MSWarningColour2", 0xFF00_AEEF));
        let scale = self
            .screens
            .globals
            .get("MSWarningScale")
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(1.0);

        for y in [10.0, 240.0] {
            out.push(Draw::Fill {
                rect: [0.0, y, self.space.size.0, 1.0],
                color: heading,
            });
        }
        for (index, line) in [
            "THIS GAME SAVES AUTOMATICALLY.",
            "PROGRESS IS WRITTEN WHEN A RACE ENDS AND",
            "WHEN A SETTING CHANGES.",
        ]
        .iter()
        .enumerate()
        {
            out.push(Draw::Text {
                x: 15.0,
                y: 30.0 + index as f32 * 15.0,
                scale,
                color: body,
                border: None,
                align: Align::Left,
                text: (*line).to_string(),
            });
        }
        out.push(Draw::Text {
            x: 15.0,
            y: 245.0,
            scale,
            color: heading,
            border: None,
            align: Align::Left,
            text: "PRESS X TO CONTINUE".to_string(),
        });
    }

    /// The frame counter `--overlay` puts over a movie leg.
    ///
    /// Whether the movie on this leg has a picture is the caller's to say: the
    /// sequence has more than one movie and they do not decode or fail together,
    /// so a shared flag would report the wrong one. The counter exists for the
    /// case where there is no picture at all - it is the only thing on screen
    /// then, and the only sign the leg is running rather than hung.
    fn insert_movie_counter(&self, out: &mut Vec<Draw>, has_picture: bool) {
        if !self.overlay {
            return;
        }
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
                if has_picture { "" } else { " (NO PICTURE)" }
            ),
        });
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
    ///
    /// # One video per list, and the renderer is what says so
    ///
    /// The single exception to "every screen gets it": a screen already drawing
    /// a movie of its own does not also get the loop. This is not a stylistic
    /// choice. The front end is drawn by one renderer with one set of I420
    /// planes and one video draw ([`crate::render::Renderer::render`] keeps a
    /// single `video_at`), so a list carrying two `Draw::Video`s would upload
    /// one movie's frame and draw it at the *other* movie's rect - and the two
    /// halves disagree about which, because `sync_video` takes the first video
    /// in the list while the renderer takes the last.
    ///
    /// The two are alternatives by construction anyway: a foreground movie and
    /// a background loop are never both wanted, and the loop keeps running
    /// underneath unseen exactly as it does through `LogoFMV`. Unreachable on
    /// today's discs - neither Pure pressing carries `Data\Movies\Backdrop.PMF`,
    /// so `self.backdrop` is `None` wherever a foreground movie draws - which is
    /// why this is asserted by a test rather than observed in a screenshot.
    fn insert_backdrop(&self, out: &mut Vec<Draw>) {
        if out.iter().any(|draw| matches!(draw, Draw::Video { .. })) {
            return;
        }
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

        // Pure only: `Language Selection` is a child of `Intro Screen`, which
        // owns the white background `Language Selection` itself carries no
        // `Image` or fill of its own - confirmed by driving `pure-psp-usa.chd`
        // under PPSSPP (2026-08-10, same session `pure_states::TITLE_SCREEN`
        // was evidenced in): the real picker sits on white, not the black this
        // build's own default canvas fill leaves it on without this.
        //
        // **Only where the title's own table names a parent to inherit from.**
        // Which screen that is - if any - is a per-title fact, so it comes from
        // [`Sequence::picker_backdrop_parent`] rather than from a probe. It used
        // to be scoped by "does this source have Pure's `Title Screen`", which
        // asked about one screen in order to conclude something about a different
        // one; Pulse gets `None` and is untouched, as it was, but now because its
        // own table says so rather than because a probe missed.
        //
        // Whether Pulse's picker inherits its parent's backdrop the same way is
        // still unconfirmed, and its `Language Selection` is authored under
        // `Top FE Screen->FE Screen` rather than under the movie screen, so "draw
        // the parent" would not even be the same parent there.
        //
        // **Fills only, not the parent's images.** `Intro Screen` also owns
        // `profileFrame` (`StartEnabled="false"`) and `ArrowSelect`, neither
        // of which belongs on the picker - this build has no model of
        // `StartEnabled` at all (`Image` carries no such field), so drawing
        // the parent's images unconditionally would put a 512x512 profile
        // card over the language list. The one thing actually evidenced is
        // the background colour.
        self.insert_backdrop_parent_fills(out);

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
/// Matches the `.fnt` files Pulse's language plugins resolve each role to:
/// `Default` is `pulse_text.fnt` (13px), `Menu` is `Pulse_20.fnt` (22px),
/// `Title`, `Small`, `InGame` and `Stats` are `Pulse_14.fnt` (17px), `HUD` is
/// `PulseHud.fnt` (25px) and `HUDSmall` is `small.fnt` (10px). The XML is
/// inconsistent about case (`font="menu"` and `font="Menu"` both appear), so
/// this matches case-insensitively.
///
/// # This is Pulse's table, and it is wrong on Pure
///
/// **A known, bounded gap, recorded rather than papered over.** These numbers
/// are the line heights of *Pulse's* faces, and a role does not resolve to the
/// same file on both discs - see [`crate::language::roles`]. Measured on
/// `pure-psp-eu.chd`, Pure's `Default` is `FX300ANG.fnt` at **15px** against
/// the 13 here, and its `Title` is the same file rather than a 17px one. So a
/// Pure front-end screen with more than one line of text spaces those lines by
/// up to a couple of pixels wrong.
///
/// The fix is not another table: it is reading the height back off the atlas the
/// way `boot::load_menu_font`'s caller already does for menu rows
/// (`menu::Skin::new(skin, rows_face.line_height)`), which needs each role's
/// `.fnt` actually loaded rather than only the `Default` one. That is real work
/// and it is not what the boot path is blocked on, so it is named here and left.
/// Nothing about it is title-specific once done - it deletes this function.
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
mod tests;
