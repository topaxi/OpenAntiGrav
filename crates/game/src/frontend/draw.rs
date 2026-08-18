//! What the boot sequence draws, one screen at a time.
//!
//! Split out of [`super`] rather than designed apart from it: the module was
//! 1,873 lines against the tree's 1,000-line rule, and the seam that fell out
//! cleanly is the one this project already leans on everywhere else - a
//! sequence that decides *what state it is in* against a renderer-free
//! description of *what that state looks like*. Everything here is `&self` and
//! returns [`Draw`]s; nothing advances a state, reads a button or touches the
//! player.
//!
//! It is an `impl` block on [`Frontend`] and not a free function taking one,
//! because every method here already reached into private fields and turning
//! forty of those into accessors would have been a rewrite rather than a move.
//! `use super::*` reaches them unchanged.

use super::*;

impl Frontend {
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

        // The plain-movie screens: Pure's `FMV Intro` and Wipeout HD's
        // `Studio Logo`, which [`Self::update_plain_movie`] already drives
        // together and which draw together for the same reason - each is its
        // screen's own widgets with the movie over them and nothing else.
        //
        // **`draw_screen` first is what makes sharing this safe.** Pure's screen
        // declares no widgets at all, so on that title the call contributes
        // nothing and the branch is the video alone; HD's `Studio Logo` declares
        // a `ScreenClear` and its skip redirects, none of which draw. A branch
        // that had hard-coded "no widgets" would have been right for one and
        // wrong for the next screen of this shape.
        if let Some(state) = [pure_states::FMV_INTRO, hd_states::STUDIO_LOGO]
            .into_iter()
            .find(|state| self.machine.is(state))
        {
            // `self.player` was rebuilt for this movie's own frame count in
            // `Self::confirm_language`, and `Video::Intro` is reused rather than
            // a third `Video` variant added - see that reassignment's own
            // comment for why reuse is safe here: the first movie's player is
            // idle by the time either state is reachable at all.
            let plan = self.movie_of(state);
            let mut out = self.draw_screen(state);
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
            // This leg needs the counter more than `LogoFMV` does, not less: it
            // is the only movie either title's boot draws here, so without
            // `ffmpeg` the whole leg is a blank screen for as long as the movie
            // would have run, with nothing to say it is progressing.
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
