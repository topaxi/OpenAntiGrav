//! The front end's per-state update handlers: one function per screen the boot
//! sequence stops on.
//!
//! Split out of `frontend.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What stays
//! behind is the type, its accessors, and the `update` that dispatches into
//! these; what moved is the handling of each individual screen.

use super::*;

impl Frontend {
    /// The disc's own movie leg: play `Data\Movies\Intro.PMF` to its end, or
    /// until a button skips it.
    ///
    /// No frame counters and no holds. The `Movie` widget carries none: it is
    /// `autostart`, `repeat="false"`, `autoredirect`, and the screen's five
    /// skip redirects - start, circle, square, triangle, cross - all go to
    /// `LogoFMVRedirectScreen`. Only the buttons the abstract layer carries are
    /// read here, which is start and cross.
    pub(super) fn update_logo_fmv(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        for button in [Button::Start, Button::Cross] {
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
    pub(super) fn advance_movie(&mut self, dt: f64, playhead: Option<f64>) {
        match playhead {
            Some(seconds) => self.player.follow(seconds),
            None => self.player.update(dt),
        }
    }

    pub(super) fn update_intro(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        // The skip. The player never reads the pad; the state does, and it fires
        // the cached redirect rather than stopping playback.
        if input.is_pressed(Button::Start)
            && let Some(target) = self.dev_pub_redirect.take()
        {
            self.notes
                .push(format!("START skipped the intro, firing {target}"));
            self.machine.fire(&target);
            input.consume_press(Button::Start);
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

    /// Whether the current screen is a movie leg that has frames but no picture to
    /// show for them.
    ///
    /// Such a leg is a black screen for as long as the movie would have run (forty
    /// seconds for the disc's own intro), so it is left at once, the way a player's
    /// Start press leaves it. A leg that plays no movie by design is not this: its
    /// plan has no frames.
    pub(super) fn leg_has_no_picture(&self) -> bool {
        if !self.is_playing_movie() {
            return false;
        }
        let Some(current) = self.machine.current() else {
            return false;
        };
        let plan = self
            .steps
            .iter()
            .find(|step| step.state == current)
            .map_or(self.first, |step| step.movie);
        !plan.has_picture
    }

    pub(super) fn begin_hold(&mut self, finish: bool) {
        self.hold = Hold::Held { finish };
        self.held_for = 0.0;
        self.player.pause();
    }

    pub(super) fn update_language_selection(&mut self, input: &mut Input) {
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
        if input.is_pressed(Button::Down) {
            input.consume_press(Button::Down);
            self.selected = (self.selected + 1) % count;
        } else if input.is_pressed(Button::Up) {
            input.consume_press(Button::Up);
            self.selected = (self.selected + count - 1) % count;
        } else if input.is_pressed(Button::Cross) {
            input.consume_press(Button::Cross);
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
    pub(super) fn confirm_language(&mut self) {
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
    /// starting an identical pause-and-reload block. **[`HOLD_SECONDS`] is now
    /// confirmed for Pure too, not just imported (2026-09-23).** The global the
    /// hold logic reads is a running clock sampled to compute elapsed time, not
    /// the duration; the duration itself is a `2.0` built as a raw float
    /// immediate, identical on both pressings, and a live PPSSPP breakpoint
    /// measured the actual held span at 1.9965 s.
    ///
    /// See `docs/architecture/pure-boot.md` and
    /// `docs/ghidra/functions/psp-pure-eu/devpub-reel-hold.md`.
    pub(super) fn update_developer_publisher(
        &mut self,
        dt: f64,
        input: &mut Input,
        playhead: Option<f64>,
    ) {
        for button in [Button::Start, Button::Cross] {
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
    pub(super) fn update_memory_stick_warning(&mut self, input: &mut Input) {
        if input.is_pressed(Button::Cross) {
            input.consume_press(Button::Cross);
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
    pub(super) fn update_show_logo(&mut self, input: &mut Input) {
        if input.is_pressed(Button::Start) {
            input.consume_press(Button::Start);
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
    pub(super) fn update_title_screen(&mut self, input: &mut Input) {
        if input.is_pressed(Button::Start) {
            input.consume_press(Button::Start);
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
    /// A movie screen with nothing on it but the movie: play, allow a skip,
    /// leave when it ends.
    ///
    /// **Two titles' screens share this, and that is a reading rather than a
    /// convenience.** Pure's `FMV Intro` and Wipeout HD's `Studio Logo` each
    /// declare exactly three things and the same three: a `Movie` that is
    /// `autostart` and not `repeat`, an `AutoRedirect` armed for when it
    /// finishes, and a handful of skip-button redirects that all go to the one
    /// place the auto-redirect goes. Neither declares a frame hold, a counter or
    /// a second widget - which is what separates them from Pulse's `LogoFMV` and
    /// from the dev/pub reel, both of which need their own handlers for things
    /// this deliberately does not do.
    ///
    /// So the state names differ and the behaviour does not, and a second copy
    /// of this body under an HD name would have been two places to fix the next
    /// time a skip button changes.
    pub(super) fn update_plain_movie(&mut self, dt: f64, input: &mut Input, playhead: Option<f64>) {
        for button in [Button::Start, Button::Cross] {
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
}
