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
//!
//! [`Draw`] itself moved here on 2026-08-26, off the same argument one step
//! further: the type and the code that produces it belong together, and
//! `frontend.rs` was at its own ceiling in `scripts/check-file-size.py`.
//! `super` re-exports it, so every `crate::frontend::Draw` in the tree is
//! unchanged.

use super::*;

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
    /// A solid rectangle whose **top-right corner is cut back diagonally**.
    ///
    /// The cut runs the full height of `rect`: the top edge is `chamfer`
    /// shorter than the bottom one, and the right edge is the diagonal joining
    /// them. `chamfer` of `0.0` is exactly [`Self::Fill`], and a `chamfer`
    /// wider than `rect`'s own width collapses the top edge to nothing rather
    /// than inverting it.
    ///
    /// **A separate variant rather than a field on [`Self::Fill`]**, for the
    /// reason [`Self::RotatedSprite`] is separate: fills are constructed in
    /// dozens of places across the HUD, the front end and the menus, and every
    /// one of them would carry a `0.0` for the single widget that needs this.
    ///
    /// # It draws a chamfer, not a whole tab
    ///
    /// HD's main-menu tab is **not** one of these. Its corner is a 45-degree
    /// cut followed by a *flat landing* before the vertical right edge - a
    /// pentagon, `---\\___|` - and a single chamfered rectangle spanning the
    /// tab's own height would run the diagonal all the way to the corner
    /// instead. That shape was built, shown next to the capture, and reverted
    /// on 2026-09-01 for exactly that reason. What draws the real thing is
    /// this applied to a *band* the height of the cut alone, narrowed by the
    /// landing, with an ordinary [`Self::Fill`] below it - see
    /// [`crate::menu::strip::draw`] and
    /// [hd-frontend.md](../../../docs/formats/hd-frontend.md).
    ChamferedFill {
        /// Rectangle, before the corner is cut: `[x, y, width, height]`.
        rect: [f32; 4],
        /// How far the top edge falls short of the bottom one, in screen units.
        chamfer: f32,
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
        /// whole movie - a headless capture reading a [`crate::movie::FrameStore`] - asks for.
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
    /// A sprite spun about its own centre.
    ///
    /// **A separate variant rather than a `rotation` on [`Self::Sprite`]**: the
    /// front end constructs sprites in a dozen places and none of them turns
    /// one, so a field there would be `0.0` at every site but this weapon's.
    ///
    /// The one thing that needs it is the lock-on reticle, whose four corner
    /// brackets are four instances of a single corner-bracket model at four
    /// quarter turns - the original writes an angle per instance to
    /// `widget+0xac`. See
    /// [`lock-sight.md`](../../../docs/ghidra/functions/psp-pulse-usa/lock-sight.md).
    RotatedSprite {
        /// Where it goes: `[x, y, width, height]`, `x`/`y` being the top-left of
        /// the *unrotated* rectangle.
        rect: [f32; 4],
        /// Where it is in the sheet, in sheet pixels.
        uv: [f32; 4],
        /// Modulating colour.
        color: [f32; 4],
        /// Clockwise turn about the rectangle's centre, in radians.
        rotation: f32,
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
        /// box. The three menu fonts and the built-in glyphs carry a constant mask, so
        /// this changes nothing for them whatever it is set to. See [`crate::font::Atlas::luma`].
        border: Option<[f32; 4]>,
        /// Alignment about `x`.
        align: Align,
        /// The text.
        text: String,
        /// Wrap width from `widthlimited="true"`; `None` is one line, however wide.
        wrap_width: Option<f32>,
    },
}

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
            // `draw_screen_at`, not `draw_screen`, and `self.on_screen_for` as
            // its clock: `BOOT_PRESS_START` carries `pulse="true"` and
            // `delay="1"`, so on the disc it fades in a second late and throbs
            // rather than appearing at once and standing still. `--screen` and
            // every test still go through the public `draw_screen`, which
            // freezes any pulsing widget at its own ceiling (the same static
            // colour this screen drew before this existed) - only the live
            // boot order animates. See [`pulse_alpha`] and
            // `docs/architecture/frontend-boot.md`.
            let mut out = self.draw_screen_at(states::SHOW_LOGO, self.on_screen_for);
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
                wrap_width: None,
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
        for fill in &parent.fills {
            out.push(Draw::Fill {
                rect: self.fill_rect(fill),
                color: argb_to_rgba(fill.color),
            });
        }
    }

    /// A [`crate::screen::Fill`]'s own rect, falling its `width`/`height`
    /// back to the whole screen when the widget omits them - the reading
    /// every colour-only `Image` measured without one actually wants. See
    /// [`crate::screen::Fill::width`].
    fn fill_rect(&self, fill: &crate::screen::Fill) -> [f32; 4] {
        [
            fill.x,
            fill.y,
            fill.width.unwrap_or(self.space.size.0),
            fill.height.unwrap_or(self.space.size.1),
        ]
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
                wrap_width: None,
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
            wrap_width: None,
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
            wrap_width: None,
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
    /// input, no wall-clock animation. `--screen` renders through it, which
    /// is how the image path is checked without moving any screen into the
    /// boot order - including a `pulse="true"` widget like `BOOT_PRESS_START`,
    /// which this draws at its own authored colour (the pulse's ceiling, see
    /// [`pulse_alpha`]) rather than mid-throb. The live boot order animates it
    /// instead, through [`Self::draw_screen_at`].
    #[must_use]
    pub fn draw_screen(&self, name: &str) -> Vec<Draw> {
        self.draw_screen_at(name, f64::INFINITY)
    }

    /// [`Self::draw_screen`], with `elapsed` seconds of wall clock since the
    /// screen appeared - the one piece of state that lets a `pulse="true"`
    /// widget's alpha move. `f64::INFINITY` (what the public method passes)
    /// reads as "settled": every pulsing widget's alpha lands on the ceiling
    /// [`pulse_alpha`] converges to, which is its own authored colour, so
    /// nothing here changes for a caller that never had a clock to give.
    fn draw_screen_at(&self, name: &str, elapsed: f64) -> Vec<Draw> {
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
            let mut color = argb_to_rgba(text.color);
            color[3] *= pulse_alpha(text, elapsed);
            out.push(Draw::Text {
                x: text.x,
                y: text.y,
                scale: text.scale,
                color,
                border: None,
                align: Align::parse(&text.align),
                text: self.strings.get_or_id(id).to_string(),
                wrap_width: text.wrap_width,
            });
        }
        out
    }

    /// A screen's solid backdrops and its images, in that order.
    fn draw_backdrops(&self, screen: &Screen, out: &mut Vec<Draw>) {
        for fill in &screen.fills {
            out.push(Draw::Fill {
                rect: self.fill_rect(fill),
                color: argb_to_rgba(fill.color),
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

            // An `Image` with no `x` is left-pinned, not centred - settled
            // against a live capture of the real disc, not just the texture.
            // `Show Logo`'s logo is a 512-wide texture on a 480-wide screen,
            // so the two readings differ by 16 pixels, and the texture's own
            // transparency (out to column 23 on the left, from 488 on the
            // right) only says the art is authored centred *inside its own
            // 512* - it does not say where that 512 lands on the 480-wide
            // screen, which is the actual question. A real `PPSSPPSDL` frame
            // of `Show Logo` (2026-09-02, `pulse-psp-usa.chd`, via the
            // websocket debugger) settles it: the frame is black up to
            // screen column 22 and the first non-zero pixel is column 23 -
            // an exact match to the texture's own column-23 transparency
            // edge, with no 16px shift. Centred would have put that edge at
            // screen column 7. Confidence 90 - one clean capture, right at
            // the boundary the two hypotheses disagree on; see
            // `docs/architecture/frontend-boot.md`'s `Image layout` section.
            // A texture's own size is in PSP pixels whichever disc it came off,
            // so on the PS2's larger grid the fallback is scaled the way that
            // build scales every other PSP-grid number. See
            // `Space::texture_scale`.
            let (scale_x, scale_y) = self.space.texture_scale();
            let w = image.width.unwrap_or(placed.width as f32 * scale_x);
            let h = image.height.unwrap_or(placed.height as f32 * scale_y);
            let x = image.x;

            // `U`/`V`/`TxtrWidth`/`TxtrHeight` name a sub-rect of `src`'s own
            // texture, in that texture's own pixels - not the whole thing,
            // which is what every image drew before this existed. Several
            // widgets sharing one texture (`ArrowSelect`, and three widgets on
            // Pure's `Title Screen`) need their own patch of it rather than
            // all drawing its top-left corner. Absent means the whole placed
            // texture, exactly as it did before. See [`crate::screen::Image::u`].
            let uv = [
                placed.x as f32 + image.u.unwrap_or(0.0),
                placed.y as f32 + image.v.unwrap_or(0.0),
                image.texture_width.unwrap_or(placed.width as f32),
                image.texture_height.unwrap_or(placed.height as f32),
            ];

            out.push(Draw::Sprite {
                rect: [x, image.y, w, h],
                uv,
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
                wrap_width: text.wrap_width,
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
                wrap_width: None,
            });
        }
    }

    /// The screen the picker is defined by, for reporting.
    #[must_use]
    pub fn language_screen(&self) -> Option<&Screen> {
        self.screens.language_selection()
    }
}

/// The throb's period, in seconds, for a [`Text`] widget whose `pulse` is set.
///
/// Measured off a real capture of `BOOT_PRESS_START` on `Show Logo`
/// (`pulse-psp-eu.chd` under PPSSPPSDL, two independent 20s captures,
/// confidence 75 - a screen-pixel luminance proxy, not a live memory read).
/// See `docs/architecture/frontend-boot.md`'s `BOOT_PRESS_START does not
/// pulse` section. No other `pulse="true"` widget has been captured, so this
/// is applied to any future one too rather than left unimplemented, on the
/// same "measure one, extrapolate rather than invent a second value" basis
/// the rest of this crate uses for shared constants.
pub(super) const PULSE_PERIOD: f32 = 1.10;

/// The throb's dim floor, as a fraction of the widget's own authored alpha.
///
/// The same capture put the dim phase at roughly 42% of the bright phase's
/// luminance - never fully faded to black - and the bright phase at roughly
/// the widget's own static (currently: only) rendered alpha, so 1.0 is the
/// ceiling this multiplies against rather than a second measured number.
pub(super) const PULSE_FLOOR: f32 = 0.42;

/// A `pulse`/`delay` widget's alpha multiplier, `elapsed` seconds after its
/// screen appeared. `1.0` for a non-pulsing widget or an infinite `elapsed`
/// (`draw_screen`'s "settled" default) - both read as "just use the authored
/// colour". Otherwise: `0.0` before `delay` has elapsed (on hardware the
/// widget is not drawn at all yet), then a sine throb between [`PULSE_FLOOR`]
/// and `1.0` with period [`PULSE_PERIOD`], itself ramped in linearly over its
/// first cycle so the widget's first appearance is a fade rather than a pop
/// at the dim floor.
///
/// **The ramp's own shape and duration are chosen, not measured.** The
/// capture only covers the repeating throb, not the fade-in `delay="1"`
/// implies; using one throb period as the fade-in's length reuses the one
/// timescale that *is* measured rather than inventing an unrelated second
/// one, and a plain sine stands in for the throb's own shape, which the
/// capture shows is not quite sinusoidal (it holds near each extreme rather
/// than smoothly reversing - a detail not modelled here). See
/// `docs/architecture/frontend-boot.md`'s `BOOT_PRESS_START now pulses`
/// section for the capture this is built on.
fn pulse_alpha(text: &Text, elapsed: f64) -> f32 {
    if !text.pulse || !elapsed.is_finite() {
        return 1.0;
    }
    let since_delay = elapsed - f64::from(text.delay);
    if since_delay < 0.0 {
        return 0.0;
    }
    let cycles = (since_delay / f64::from(PULSE_PERIOD)) as f32;
    let wave = (cycles.fract() * std::f32::consts::TAU).sin();
    let settled = PULSE_FLOOR + (1.0 - PULSE_FLOOR) * (wave + 1.0) / 2.0;
    settled * cycles.min(1.0)
}

impl Draw {
    /// The colour this draw is modulated by, when it has one.
    ///
    /// `None` for [`Self::Video`] alone: a movie frame has no alpha channel to
    /// fade and is never part of a page.
    ///
    /// **It exists so a new variant is one edit rather than three.** The menu's
    /// zoom and fade helpers each used to spell the variant list out, so adding
    /// [`Self::RotatedSprite`] meant remembering both - and forgetting one is a
    /// widget that silently does not fade with the page it is on.
    pub fn colour_mut(&mut self) -> Option<&mut [f32; 4]> {
        match self {
            Self::Text { color, .. }
            | Self::Fill { color, .. }
            | Self::ChamferedFill { color, .. }
            | Self::Sprite { color, .. }
            | Self::RotatedSprite { color, .. } => Some(color),
            Self::Video { .. } => None,
        }
    }

    /// Scales this draw about a point and multiplies its alpha.
    ///
    /// The whole of the menu's page-change effect - see
    /// [`crate::menu::Layers::zoomed`], which calls it once per body draw.
    /// **It lives here rather than beside its one caller** for the reason
    /// [`Self::colour_mut`] does: it spells the variant list out, so a new
    /// variant that is not added here silently does not move with the page it
    /// is on. Next to the type, that is one file to check.
    ///
    /// No variant needed adding for any of it: `Text`, `Fill` and `Sprite`
    /// already carry a position, a size or a scale, and a colour whose fourth
    /// channel is the alpha.
    pub fn zoom(&mut self, origin: (f32, f32), scale: f32, alpha: f32) {
        let about = |value: f32, from: f32| from + (value - from) * scale;
        // The chamfer before the match, so the two fill variants stay one arm.
        // It scales with its own quad: a fixed-size cut left on a shrinking
        // corner would eat more of the tab the smaller the tab got.
        if let Self::ChamferedFill { chamfer, .. } = self {
            *chamfer *= scale;
        }
        match self {
            Self::Text {
                x,
                y,
                scale: size,
                color,
                ..
            } => {
                *x = about(*x, origin.0);
                *y = about(*y, origin.1);
                *size *= scale;
                color[3] *= alpha;
            }
            Self::Fill { rect, color }
            | Self::ChamferedFill { rect, color, .. }
            | Self::Sprite { rect, color, .. }
            | Self::RotatedSprite { rect, color, .. } => {
                rect[0] = about(rect[0], origin.0);
                rect[1] = about(rect[1], origin.1);
                rect[2] *= scale;
                rect[3] *= scale;
                color[3] *= alpha;
            }
            // A movie has no alpha channel to fade and is never part of a page.
            Self::Video { .. } => {}
        }
    }
}
