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

use crate::language::{Language, StringTable};
use crate::screen::{Screen, Screens, argb_to_rgba, parse_argb};
use crate::state_machine::{Event, StateMachine};
use oag_core::buttons::{Button, Input};

mod placement;
mod player;
pub use placement::Placed;
pub use player::{FRAME_RATE, PS2_DISPLAY_ASPECT, Player};

use oag_display::space::{SCREEN, Space, pillarbox_in};
use oag_hd::frontend::states as hd_states;
/// 2048's boot screens, touch grids and campaign map: `wipeout2048.rs`
/// drives the five screens its declared chain walks off their own
/// redirects, `touch.rs` the two icon grids after them, `campaign_map.rs`
/// the map the grids lead to. See each module's own docs.
mod campaign_map;
mod event_card;
mod faces;
mod options2048;
mod team;
mod touch;
mod wipeout2048;
pub use campaign_map::{
    CampaignMap, EarnedTier, EventIcon, HEX_FILLED, HEX_OUTLINE, HEX_SELECT, MapEvent,
    ProgressState,
};
pub use event_card::{
    CARD_TEXTURES, CardCraft, CardRestriction, CardState, CardTabs, CardTrophy, CardWeapons,
    EventCard,
};
pub(crate) use faces::{font_line_height, lighten};
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
pub use touch::{EXTRA_TILES, ExtraTile, Launch, TouchState, cursor_ring};

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
        // Wipeout HD's only movie step. The other seven screens of its declared
        // chain are **not** here and that is the point of the filter: a
        // connection check, three dialogs, an EULA and a save warning are
        // behaviour this build does not have, so `boot::load_shell` steps over
        // each one and says which on the report rather than stalling the boot
        // on a screen nothing drives.
        hd_states::STUDIO_LOGO,
    ]
    .contains(&state)
        // Wipeout 2048's five boot screens and the grids after them - one
        // list, read by `Frontend::booting`'s registration too.
        || wipeout2048::STATES.contains(&state)
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
            frame_rate: FRAME_RATE,
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
    /// disagree. See `crate::hud`.
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
/// `crate::capture` already carries a warning about for `--menu-page`, so the
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

mod draw;
mod pointer;
mod rows;
mod updates;

pub use draw::Draw;

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
    placements: Vec<(String, Placed)>,
    player: Player,
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
    /// Where the picker's font has its ink, for the pointer's row bands -
    /// see [`crate::pointer::RowInk`]. `None` until [`Self::set_row_ink`],
    /// which every test fixture in this crate leaves it at.
    row_ink: Option<crate::pointer::RowInk>,
    /// The looping backdrop `FE Screen` plays under `Show Logo`, and where it
    /// goes on screen. `None` on a source that has no backdrop, under
    /// `--no-video`, and when its plane geometry does not match the intro's -
    /// see [`Frontend::set_backdrop`], which is the only thing that sets it.
    backdrop: Option<(Player, [f32; 4])>,
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
    /// The title's own measured menu colours, for the language picker's
    /// selected-row ink. `None` on every fixture in this file's own tests and
    /// until [`Self::set_menu_skin`] is called - the picker falls back to its
    /// old row-colour-and-highlight behaviour in that case, unchanged.
    ///
    /// **Deliberately not threaded through [`Self::new`]/[`Self::booting`]'s
    /// signature**, the same way [`Self::set_space`]/[`Self::set_backdrop`]
    /// are not: `crate::race::hud`, `crate::race::load` and every test
    /// fixture here construct a `Frontend` without one, and a constructor
    /// change would touch all of them for a value only the picker reads. See
    /// `docs/formats/pure-status.md`'s `Language Selection has no highlight
    /// band at all` finding.
    menu_skin: Option<&'static oag_title::MenuSkin>,
    /// Where Wipeout 2048's touch grids are - see [`TouchState`]. Inert on
    /// every other title, which never enters a state that reads it.
    touch: TouchState,
    /// Wipeout 2048's campaign map - see [`CampaignMap`]. Empty on every
    /// other title.
    campaign: CampaignMap,
    /// How much larger or smaller each font role's face is than the
    /// `Default` one this build draws every screen widget with, by the
    /// role's name in lower case - see [`Self::set_face_scales`]. Empty
    /// draws every role at the widget's own `scale`, which is what every
    /// title did before this existed.
    face_scales: Vec<(String, f32)>,
    /// The `Default` face's own line height, in grid units, for the one
    /// widget attribute that needs it: `vertalign="middle"`. `None` until
    /// [`Self::set_face_scales`], which anchors every text at its pen.
    default_line_height: Option<f32>,
    /// The `Default` face's own line height, when the language picker's row
    /// pitch is to be one line of it - see
    /// [`Self::set_picker_line_height`]. `None` keeps the table.
    picker_line_height: Option<f32>,
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
        placements: Vec<(String, Placed)>,
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
                            frame_rate: FRAME_RATE,
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
        placements: Vec<(String, Placed)>,
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
        machine.register_all(wipeout2048::STATES.iter().copied());
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
            row_ink: None,
            player: Player::new(frames, false, first.frame_rate),
            first,
            backdrop: None,
            // Off until `--overlay` asks: a leg with no picture is skipped, so
            // there is no blank wait for the counter to explain.
            overlay: false,
            hold: Hold::None,
            held_for: 0.0,
            dev_pub_redirect: Some(states::DEV_PUB_REDIRECT.to_string()),
            acted: Vec::new(),
            notes: Vec::new(),
            finished: false,
            steps,
            backdrop_parent,
            on_screen_for: 0.0,
            menu_skin: None,
            touch: TouchState::default(),
            campaign: CampaignMap::default(),
            face_scales: Vec::new(),
            default_line_height: None,
            picker_line_height: None,
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
    /// **Running out of chain opens the menus**, which is the same hand-off
    /// [`Self::update_show_logo`] and [`Self::update_title_screen`] make, stated
    /// once for the titles that reach the end without passing through either.
    ///
    /// Neither PSP title ever gets here: Pulse's last step is `Show Logo` and
    /// Pure's is `Title Screen`, and both are left by their own handler on a
    /// START press rather than by advancing past themselves. Wipeout HD is the
    /// title that does. Six of its eight declared steps are dialogs this build
    /// has no behaviour for, so `boot::load_shell` filters them out and the
    /// chain it walks is two screens long - the picker and `Studio Logo`. When
    /// that movie ends there is nothing ahead of it, and until this arm existed
    /// the boot simply stopped there with no note saying why: 400 ticks of START
    /// and CROSS left the state at `Studio Logo` and read exactly like a hang.
    ///
    /// **What this is not.** It is not a claim about where HD's own front end
    /// goes. `Update Announcement` - the last screen of the declared chain -
    /// carries no redirect at all in the `skin.xml` this build loads, so the
    /// disc's own answer to "and then?" is not in the file. HD's front end
    /// proper is `Top FE Screen`, `type="FEMain"`, and its `Main Menu` lives in
    /// `MainMenu_Definition.xml`, one of the 21 `LoadXML` includes nothing here
    /// follows yet. So this fires [`states::LAUNCH_GAME`] - *this build's* menu
    /// tree, `assets/ui/menu.toml` - and says so in a note, exactly as the two
    /// PSP titles' documented divergences do. See `docs/formats/hd-frontend.md`.
    fn advance(&mut self, why: &str) {
        let Some(step) = self.next_step().cloned() else {
            let at = self.machine.current().unwrap_or_default().to_string();
            self.notes.push(format!(
                "{why}, and the boot chain ends on {at:?}; opening this build's \
                 own menus at {}",
                states::LAUNCH_GAME
            ));
            self.machine.fire(states::LAUNCH_GAME);
            return;
        };
        self.notes.push(format!("{why}, firing {}", step.state));
        if step.movie.frames > 0 {
            self.player = Player::new(step.movie.frames, false, step.movie.frame_rate);
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

    /// Tells the language picker where the face its rows are drawn in keeps
    /// its ink, so a pointer's row bands sit on the glyphs rather than on the
    /// pen. Measured off the atlas the renderer draws `Draw::Text` with.
    pub fn set_row_ink(&mut self, ink: Option<crate::pointer::RowInk>) {
        self.row_ink = ink;
    }

    /// Gives the sequence the title's own measured menu colours.
    ///
    /// Optional and title-agnostic on purpose: a source with no measured
    /// `selected` colour, or one whose `selected` pulses rather than swaps
    /// (`selected_pulse_period_secs` is `Some`, Pulse's own shape - see
    /// `oag_title::MenuSkin::selected_pulse_period_secs`), leaves the picker
    /// drawing exactly as it did before this existed. Only a title with a
    /// *static* measured `selected` (Pure's, confidence 65) changes anything.
    pub fn set_menu_skin(&mut self, skin: &'static oag_title::MenuSkin) {
        self.menu_skin = Some(skin);
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
            Player::new(frames, true, frame_rate),
            pillarbox_in(self.space, aspect),
        ));
    }

    /// The backdrop's playhead, for tests and for whoever is pumping its feed.
    #[must_use]
    pub fn backdrop(&self) -> Option<&Player> {
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
    /// advancing it, and keep asking the one `crate::movie::Feed` for frames
    /// at its position. Nothing restarts the feed on this path - the two share
    /// an origin already, and a restart is what would put them back at odds.
    ///
    /// Called once, when the menus open. The sequence draws no backdrop
    /// afterwards, which is correct: it is over.
    pub fn take_backdrop(&mut self) -> Option<Player> {
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
    pub fn player(&self) -> &Player {
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
            || self.machine.is(hd_states::STUDIO_LOGO)
            || self
                .machine
                .is(oag_2048::frontend::states::BOOT_INTRO_MOVIE)
    }

    /// Steps the sequence by `dt` seconds.
    ///
    /// `movie_playhead` is how far the movie's **own sound** has got, in
    /// seconds, and `None` - the ordinary case - means it has none to pace
    /// against. See [`Self::advance_movie`] for what it does with it, and
    /// `oag_sound::Audio::movie_playhead` for every reason it is `None`.
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
        self.campaign.tick(dt, self.space.size);

        if self.leg_has_no_picture() {
            input.inject_press(Button::Start);
        }

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
        } else if self.machine.is(pure_states::FMV_INTRO) || self.machine.is(hd_states::STUDIO_LOGO)
        {
            self.update_plain_movie(dt, input, movie_playhead);
        } else if self.machine.is(pure_states::TITLE_SCREEN) {
            self.update_title_screen(input);
        } else if self.in_wipeout2048() {
            self.update_wipeout2048(dt, input, movie_playhead);
        }

        let events = self.machine.apply();
        for event in &events {
            if let Event::Enter(name) = event
                && (name == states::LAUNCH_GAME || name == oag_2048::frontend::states::LAUNCH_2048)
            {
                self.finished = true;
            }
        }
        events
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

    /// HD's own `Language Selection` never presents to a player - the caller's
    /// escape hatch for the one case [`Self::preselect_language`] cannot cover:
    /// a run with no settings language yet, on the one title this is measured
    /// for.
    ///
    /// Four RPCS3 boots (English matched a shipped plugin, Polish matched
    /// none; each repeated with and without a save already on the profile)
    /// all left `Language Selection` for `PreFMVConnect` within 2-296 ms of
    /// entering it through the screen's own unconditional
    /// `LanguageAutoRedirect` - see
    /// `docs/formats/hd-frontend.md#is-the-language-picker-ever-shown`. A
    /// fresh run choosing nothing yet must not be the one case this build
    /// waits at a screen the original never does, so this defaults straight
    /// to `"English"`, which is what HD's own bootstrap plugin load resolved
    /// to in both matched and unmatched conditions before the screen was even
    /// entered.
    ///
    /// A no-op, returning `false`, on every title whose
    /// [`oag_title::Looks::skips_language_picker`] is not read (all but HD),
    /// and on an HD source
    /// that does not offer `"English"` (unmeasured elsewhere - not invented
    /// here, the same refusal [`Self::preselect_language`] already makes for
    /// a name this source does not carry).
    pub fn skip_never_shown_picker(&mut self, title: &oag_title::Title) -> bool {
        title.looks.skips_language_picker.applies_everywhere() && self.preselect_language("English")
    }
}

#[cfg(test)]
mod tests;
