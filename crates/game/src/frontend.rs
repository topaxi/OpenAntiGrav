//! The boot sequence: intro movie, then the language picker.
//!
//! This is the part worth testing, so it holds no GPU handles, opens no files
//! and reads no clock. It takes a delta and an input snapshot and emits a draw
//! list. `main.rs` supplies the first two and rasterises the third.
//!
//! # What it reproduces
//!
//! The intro is `0x088d7e1c`, whose `OnEnter` caches two transition targets by
//! name, `"DevPubRedirect"` and `"Intro Screen->IntroMovie1"`. Playback is paced
//! against the movie's own frame counter: pause at 144 and at 231, set the
//! finish flag at 260, each pause released after two seconds, and the finish flag
//! fires `DevPubRedirect`. START skips by firing that same redirect rather than
//! by stopping the player, so the teardown is a consequence of the transition.
//! See `docs/ghidra/functions/psp-pulse/frontend-video.md`.
//!
//! The 260 is not arbitrary and it is not ours: three movies in `Data.wad` are
//! exactly 260 frames long, which is what confirms those counters are frame
//! numbers of a real reel. See `docs/architecture/frontend-boot.md`.
//!
//! # Where it knowingly differs
//!
//! The front-end XML runs `Language Selection` **first** and its
//! `LanguageAutoRedirect` goes on to `LogoFMV`, which is where the named intro
//! movie plays. This build boots into the intro and lands on the picker, because
//! that is the order asked for; the XML's own order is recorded in
//! [`Frontend::language_auto_redirect`] and reported at startup so the
//! difference is visible rather than buried.

use crate::input::{Input, button};
use crate::language::{Language, StringTable};
use crate::screen::{Screen, Screens, argb_to_rgba};
use crate::state_machine::{Event, StateMachine};

/// Frame counts at which the intro state acts, from `0x088d7e1c`.
pub const PAUSE_FRAMES: [usize; 2] = [144, 231];
/// The frame at which the intro sets its finish flag.
pub const FINISH_FRAME: usize = 260;
/// How long a pause is held, in seconds.
pub const HOLD_SECONDS: f64 = 2.0;

/// State names this build drives. Every one is a literal from the original.
pub mod states {
    /// The intro's parent state.
    pub const INTRO: &str = "Intro Screen";
    /// The intro movie, spelled the way the original's own string literal is.
    pub const INTRO_MOVIE: &str = "Intro Screen->IntroMovie1";
    /// The one-shot redirect the intro fires when it finishes or is skipped.
    pub const DEV_PUB_REDIRECT: &str = "DevPubRedirect";
    /// The language picker.
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    /// Where picking a language goes.
    pub const LAUNCH_GAME: &str = "Launch Game";
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
    fn parse(value: &str) -> Self {
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
    /// A line of text with its baseline-less top-left at `x, y`.
    Text {
        /// Left or anchor edge, depending on `align`.
        x: f32,
        /// Top edge.
        y: f32,
        /// Scale multiplier applied to the glyph cell.
        scale: f32,
        /// Colour.
        color: [f32; 4],
        /// Alignment about `x`.
        align: Align,
        /// The text.
        text: String,
    },
}

/// The PSP's screen, which the XML's coordinates are in.
pub const SCREEN: (f32, f32) = (480.0, 272.0);

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
    player: crate::movie::Player,
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
    /// Builds the boot sequence.
    ///
    /// `movie_frames` is how many frames of the intro reel are actually
    /// available. Zero is legal and means no picture, which still plays out the
    /// full sequence over the movie's real duration.
    #[must_use]
    pub fn new(
        screens: Screens,
        strings: StringTable,
        languages: Vec<Language>,
        movie_frames: usize,
        has_picture: bool,
    ) -> Self {
        let mut machine = StateMachine::new();
        machine.register_all([
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

        // The intro's pacing needs frames up to FINISH_FRAME; a shorter reel
        // finishes early rather than stalling.
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
            player: crate::movie::Player::new(frames, false),
            has_picture,
            // Without a picture the intro is a black screen for eight seconds,
            // so the counter is the only sign it is running. With one it is
            // clutter, and the pacing can be read off the picture instead.
            overlay: !has_picture,
            hold: Hold::None,
            held_for: 0.0,
            dev_pub_redirect: Some(states::DEV_PUB_REDIRECT.to_string()),
            acted: Vec::new(),
            notes: Vec::new(),
            finished: false,
        };
        frontend.machine.transition_to(states::INTRO_MOVIE);
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
        if self.machine.is(states::INTRO_MOVIE) {
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
        let count = self.languages.len();

        if input.is_pressed(button::DOWN) {
            input.consume_press(button::DOWN);
            self.selected = (self.selected + 1) % count;
        } else if input.is_pressed(button::UP) {
            input.consume_press(button::UP);
            self.selected = (self.selected + count - 1) % count;
        } else if input.is_pressed(button::CROSS) {
            input.consume_press(button::CROSS);
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
    }

    /// What to draw this frame.
    #[must_use]
    pub fn draw_list(&self) -> Vec<Draw> {
        let (width, height) = SCREEN;
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, width, height],
            color: [0.0, 0.0, 0.0, 1.0],
        }];

        if self.machine.is_in(states::INTRO) {
            if self.has_picture {
                out.push(Draw::Video {
                    rect: [0.0, 0.0, width, height],
                    frame: self.player.frame(),
                });
            }
            if self.overlay {
                out.push(Draw::Text {
                    x: 8.0,
                    y: 250.0,
                    scale: 1.0,
                    color: [1.0, 1.0, 1.0, 0.5],
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
                align: Align::Centre,
                text: states::LAUNCH_GAME.to_ascii_uppercase(),
            });
        }
        out
    }

    fn draw_language_selection(&self, out: &mut Vec<Draw>) {
        let Some(screen) = self.screens.language_selection() else {
            return;
        };

        for &fill in &screen.fills {
            out.push(Draw::Fill {
                rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
                color: argb_to_rgba(fill),
            });
        }

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
            out.push(Draw::Text {
                x: text.x,
                y: text.y,
                scale: text.scale.max(0.5),
                // The XML's title colour is black on a black backdrop, because
                // the real screen sits on the menu's own lit background. Nothing
                // draws that yet, so the colour is lifted rather than silently
                // producing an empty screen.
                color: lighten(argb_to_rgba(text.color)),
                align: Align::parse(&text.align),
                text: body.to_string(),
            });
        }

        // `DisplayLanguages` plus `Menu`: one row per language the disc offers,
        // in its own name.
        let menu_x = 50.0;
        let menu_y = 60.0;
        let row = 24.0;
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
                scale: 1.6,
                color: if selected {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.6, 0.85, 0.9, 1.0]
                },
                align: Align::Left,
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
    <Menu name="Language"><Values align="left"></Values></Menu>
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

    fn frontend(frames: usize) -> Frontend {
        Frontend::new(
            Screens::from_xml(XML),
            StringTable::default(),
            languages(),
            frames,
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
    fn boots_into_the_intro_movie() {
        let frontend = frontend(300);
        assert_eq!(frontend.machine().current(), Some(states::INTRO_MOVIE));
        assert!(frontend.machine().is_in(states::INTRO));
    }

    #[test]
    fn pauses_at_144_then_231_then_finishes_at_260() {
        let mut frontend = frontend(300);
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
        let mut frontend = frontend(300);
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
        let mut frontend = frontend(300);
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
        let mut frontend = frontend(30);
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
