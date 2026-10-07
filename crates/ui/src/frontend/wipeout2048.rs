//! Wipeout 2048's boot screens: the five the declared chain walks, driven off
//! each screen's **own** `<Redirect>` widgets rather than a handler per
//! screen.
//!
//! Every other title's screens in this crate have a handler apiece
//! (`updates.rs`), each spelling out which button leaves and where to. 2048's
//! five boot screens author that themselves and consistently
//! (`docs/formats/2048-frontend.md`, "The declared boot chain"): a
//! `<Redirect delay="4.0">` on the Studio Liverpool card, six button
//! redirects on the movie and on the title screen, an `AutoRedirect` the
//! movie's end arms. So one reading of the `Redirect` list serves all five,
//! and the two screens that carry no redirect at all - `Boot Connect`'s
//! network check and `Load Save Bootup`'s save check - are the two that leave
//! on a condition this build has nothing to check, and pass straight through.
//!
//! Split out of `frontend.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the way `updates.rs` and `pointer.rs` are.

use super::*;
use crate::screen::Redirect;
use oag_2048::frontend::states as w2048;

/// Every 2048 screen this build can enter and leave - the list `can_drive`
/// and `Frontend::booting` both read. See `can_drive`'s own doc for what
/// adding a name here claims.
pub(super) const STATES: &[&str] = &[
    w2048::BOOT_CONNECT,
    w2048::BOOT_STUDIO_LOGO,
    w2048::BOOT_INTRO_MOVIE,
    w2048::LOAD_SAVE_BOOTUP,
    w2048::TITLE_SCREEN,
    w2048::GAME_MODE_CHOICE,
    w2048::HOME,
    w2048::NEW_FE_SHELL,
    w2048::LAUNCH_2048,
    // `Home`'s five destinations, registered by the bare name their own
    // `redirect` attribute names - see `oag_2048::frontend::states::TEAM`'s
    // own doc for why a bare name rather than the nested path each include's
    // own standalone parse computes.
    w2048::TEAM,
    w2048::PROFILE,
    w2048::PROFILE_STATS,
    w2048::OPTIONS,
    w2048::OPTIONS_CAMERA,
    w2048::OPTIONS_AUDIO,
    w2048::OPTIONS_CONTROLS,
    w2048::OPTIONS_PILOT,
    w2048::SAVE_2048_OPTIONS,
    w2048::COMMUNITY_ADHOC_CHECK,
    w2048::EXTRAS,
    w2048::EXTRAS_MANUAL,
    w2048::EXTRAS_CREDITS,
];

/// The buttons a 2048 screen's redirects can name that the abstract layer
/// carries. `frontScreen` - a tap anywhere on the front touch panel - is the
/// sixth, and it is what a pointer click is synthesised into by the
/// composition root (`pointer::press_for_click` presses start and cross).
const BUTTONS: [Button; 5] = [
    Button::Start,
    Button::Cross,
    Button::Circle,
    Button::Triangle,
    Button::Square,
];

impl Frontend {
    /// Whether the current screen is one of [`STATES`].
    pub(super) fn in_wipeout2048(&self) -> bool {
        self.machine
            .current()
            .is_some_and(|current| STATES.contains(&current))
    }

    /// The tick handler for every screen in [`STATES`].
    pub(super) fn update_wipeout2048(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        let Some(current) = self.machine.current().map(str::to_string) else {
            return;
        };
        match current.as_str() {
            // `<UnityConBasic>`: a network check whose `MoveTo` goes to
            // `Boot Studio Logo` on success **and** on failure. This build
            // has no network to check, and the file says the answer does
            // not matter, so the screen is left the tick it is entered.
            w2048::BOOT_CONNECT => self.advance("Boot Connect's network check has one exit"),
            // `type="HDDBoot"`: the save-data check, `StartEnabled="false"`
            // on its one redirect - armed by code once the check completes.
            // There is no save to check here, so it completes at once.
            w2048::LOAD_SAVE_BOOTUP => self.advance("no save data to check"),
            w2048::BOOT_INTRO_MOVIE => self.update_intro_2048(dt, input, playhead),
            w2048::BOOT_STUDIO_LOGO | w2048::TITLE_SCREEN => {
                self.follow_authored_redirect(&current, input);
            }
            // `<ProfileController task="Auto Save">` then an unconditional
            // `<Redirect delay="0.2">` to `Home` - a save this build has
            // nothing to write, so this is a timed pass-through like the
            // boot chain's own screens above.
            w2048::SAVE_2048_OPTIONS => self.follow_authored_redirect(&current, input),
            w2048::TEAM => self.update_team(input),
            w2048::OPTIONS_CAMERA
            | w2048::OPTIONS_AUDIO
            | w2048::OPTIONS_CONTROLS
            | w2048::OPTIONS_PILOT => self.update_options_panel(input, &current),
            _ => self.update_touch(input),
        }
    }

    /// Fires whichever of the screen's own redirects the tick satisfies: a
    /// `delay` that has elapsed, or a button that was pressed.
    ///
    /// **The target is the file's, the mechanism is the chain's.** A
    /// redirect whose `goto` is the next step of the boot chain goes through
    /// [`Self::advance`], which is what rebuilds the player for a step that
    /// plays a movie; any other target is fired as the file names it. On
    /// the five boot screens both readings agree - every authored `goto` is
    /// the next chain step except `TitleScreen`'s `GameModeChoice`, which is
    /// where the chain ends and the touch front end begins.
    fn follow_authored_redirect(&mut self, current: &str, input: &mut Input) {
        let Some(screen) = self.screens.by_name(current) else {
            return;
        };
        let elapsed = self.on_screen_for;
        let timed = screen.redirects.iter().find(|redirect| {
            redirect
                .delay
                .is_some_and(|delay| elapsed >= f64::from(delay))
        });
        let pressed = BUTTONS
            .into_iter()
            .find(|button| input.is_pressed(*button))
            .and_then(|button| {
                screen
                    .redirect_for(button)
                    .map(|redirect| (button, redirect))
            });
        let (why, redirect) = match (pressed, timed) {
            (Some((button, redirect)), _) => {
                input.consume_press(button);
                (format!("{button:?} pressed on {current}"), redirect)
            }
            (None, Some(redirect)) => (
                format!(
                    "{current}'s own {:.1}s delay ran out",
                    redirect.delay.unwrap_or_default()
                ),
                redirect,
            ),
            (None, None) => return,
        };
        self.leave_for(redirect.clone(), &why);
    }

    /// `Boot Intro Movie`: `data/Videos/intro.mp4`, to its end or until any
    /// of the six authored button redirects.
    ///
    /// **A movie whose length is unknown is left at once and says so.**
    /// The step's plan has `frames == 0` when `movie::open` could not read
    /// the container - which is every run until `oag-video` demuxes MP4 -
    /// and running Pulse's 260-frame stand-in there would invent a duration
    /// for a film this build has not measured. A movie with no picture is
    /// skipped on every platform, so the note names the gap and the screen goes.
    fn update_intro_2048(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        let plan = self.movie_of(w2048::BOOT_INTRO_MOVIE);
        if plan.frames == 0 {
            self.advance("Boot Intro Movie has no length this build can read, so no picture");
            return;
        }
        for button in BUTTONS {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.advance("the intro was skipped");
                return;
            }
        }
        self.advance_movie(dt, playhead);
        if self.player.is_finished() {
            // `AutoRedirect`, `StartEnabled="false"`: armed by the movie's end.
            self.advance("the intro ended, firing its AutoRedirect");
        }
    }

    /// Leaves the current screen for a redirect's target, by the chain when
    /// the target is the next step and by name otherwise.
    fn leave_for(&mut self, redirect: Redirect, why: &str) {
        let Some(goto) = redirect.goto else {
            self.notes
                .push(format!("{why}, but the redirect names no goto"));
            return;
        };
        if self.next_step().is_some_and(|step| step.state == goto) {
            self.advance(why);
            return;
        }
        self.notes.push(format!("{why}, firing {goto}"));
        self.on_screen_for = 0.0;
        self.machine.fire(&goto);
    }

    /// The draw list for every screen in [`STATES`].
    ///
    /// Each boot screen is its own widgets, through the same
    /// `draw_screen_at` every title's PRESS START screen uses. What is added
    /// is the ground they sit on: `Boot Studio Logo` and `TitleScreen` carry
    /// a `<BootFlowCanvas>` widget, engine chrome whose triangle-grid art
    /// has no located asset (`2048-frontend.md`), and on the Vita3K captures
    /// it reads as near-white. White is drawn under those two screens and
    /// **that colour is chosen, not measured** - it is the capture's ground
    /// tone, not a number the disc states. `Boot Connect` authors its own
    /// white `Image` and needs nothing added.
    pub(super) fn draw_wipeout2048(&self) -> Vec<Draw> {
        let Some(current) = self.machine.current() else {
            return Vec::new();
        };
        let (width, height) = self.space.size;
        let white = Draw::Fill {
            rect: [0.0, 0.0, width, height],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        match current {
            w2048::BOOT_INTRO_MOVIE => {
                let plan = self.movie_of(current);
                let mut out = self.draw_screen_at(current, self.on_screen_for);
                if plan.has_picture {
                    out.push(Draw::Video {
                        rect: pillarbox_in(self.space, plan.aspect),
                        frame: self.player.frame(),
                        position: self.player.position(),
                        source: Video::Intro,
                    });
                }
                self.insert_movie_counter(&mut out, plan.has_picture);
                out
            }
            w2048::BOOT_STUDIO_LOGO | w2048::TITLE_SCREEN => {
                let mut out = self.draw_screen_at(current, self.on_screen_for);
                out.insert(1, white);
                out
            }
            w2048::GAME_MODE_CHOICE
            | w2048::HOME
            | w2048::PROFILE
            | w2048::PROFILE_STATS
            | w2048::OPTIONS
            | w2048::COMMUNITY_ADHOC_CHECK
            | w2048::EXTRAS
            | w2048::EXTRAS_MANUAL
            | w2048::EXTRAS_CREDITS => {
                let mut out = self.draw_screen_at(current, self.on_screen_for);
                out.insert(1, white);
                self.draw_touch(current, &mut out);
                out
            }
            w2048::TEAM => {
                let mut out = self.draw_screen_at(current, self.on_screen_for);
                out.insert(1, white);
                self.draw_team(&mut out);
                out
            }
            w2048::OPTIONS_CAMERA
            | w2048::OPTIONS_AUDIO
            | w2048::OPTIONS_CONTROLS
            | w2048::OPTIONS_PILOT => {
                let mut out = vec![white];
                self.draw_options_panel(current, &mut out);
                out
            }
            w2048::NEW_FE_SHELL => {
                let mut out = self.draw_screen_at(current, self.on_screen_for);
                out.insert(1, white);
                self.draw_campaign_map(&mut out);
                self.draw_event_card(&mut out);
                out
            }
            _ => self.draw_screen_at(current, self.on_screen_for),
        }
    }
}
