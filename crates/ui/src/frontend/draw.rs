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

use super::rows::LanguageRows;
use super::*;
use oag_display::space::pillarbox_in;

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
    /// A solid rectangle whose **top corners are cut back diagonally**.
    ///
    /// Each cut runs the full height of `rect`: the top edge is
    /// `chamfer[0]` shorter than the bottom one at the left and `chamfer[1]`
    /// shorter at the right, and each side edge is the diagonal joining the
    /// two lengths. `[0.0, 0.0]` is exactly [`Self::Fill`], and cuts wider
    /// than `rect`'s own width together collapse the top edge to nothing
    /// rather than inverting it.
    ///
    /// **Two cuts since 2026-09-14**, because the shape HD's `Block` draws
    /// its top band with has one at each end on the Fury style
    /// (`Block_DrawTopBand`, `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`)
    /// and only the right one on the HD style; a one-sided variant could
    /// draw the second style and not the first.
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
    /// this applied to a *band* ten units tall, the executable's
    /// `Block_DrawTopBand`, with an ordinary [`Self::Fill`] body below it -
    /// [`crate::menu::block::draw`], and
    /// [hd-frontend.md](../../../docs/formats/hd-frontend.md).
    ChamferedFill {
        /// Rectangle, before the corners are cut: `[x, y, width, height]`.
        rect: [f32; 4],
        /// How far the top edge falls short of the bottom one at the left
        /// and at the right, in screen units.
        chamfer: [f32; 2],
        /// Colour.
        color: [f32; 4],
    },
    /// A solid rectangle whose colour runs from `left` at its left edge to
    /// `right` at its right edge, interpolated across.
    ///
    /// What a `Color1`..`Color4` `<Image>` authors - see
    /// [`crate::screen::Fill::gradient`]. Every one measured is a horizontal
    /// rule that fades in from nothing and back out again, drawn as two
    /// mirrored halves, which is why this carries two colours and not four:
    /// the top and bottom of an edge never differ on any shipped widget, so
    /// a vertical component would be an axis nothing authored.
    ///
    /// **A separate variant rather than a second colour on [`Self::Fill`]**,
    /// for the reason [`Self::ChamferedFill`] is: fills are constructed in
    /// dozens of places and every one would carry a duplicate colour for the
    /// handful of rules that fade.
    GradientFill {
        /// Rectangle.
        rect: [f32; 4],
        /// Colour at the left edge.
        left: [f32; 4],
        /// Colour at the right edge.
        right: [f32; 4],
    },
    /// A movie's current frame, stretched to `rect`.
    Video {
        /// Rectangle.
        rect: [f32; 4],
        /// Which frame of the movie to show, counting from zero.
        ///
        /// Wraps on a movie that loops, so this is what a caller holding the
        /// whole movie - a headless capture reading a `crate::movie::FrameStore` - asks for.
        frame: usize,
        /// How far playback has got, counting every loop.
        ///
        /// The same number [`crate::frontend::Player::position`] reports, and the
        /// **only** one a `crate::movie::Feed` can be asked with: a feed
        /// decodes forward forever, so on the second time round a 270-frame
        /// loop it holds positions 270..274 while `frame` has gone back to 0.
        /// Asking it with `frame` takes nothing from that point on and the
        /// picture freezes - which is exactly what the menu backdrop used to do
        /// nine seconds into the boot sequence.
        position: u64,
        /// Which movie the frame belongs to.
        source: Video,
    },
    /// The Fury menu backdrop's point-cloud pass, composited here in the
    /// list's order the way [`Self::Video`] is.
    ///
    /// The frame says which cloud, where the camera is and every constant the
    /// sprites are drawn with - see [`crate::backdrop::Frame`]. Like a movie
    /// frame it covers the whole screen, has no alpha to fade, and is never
    /// part of a page: it is what the page sits on. Boxed, for the same
    /// reason [`crate::menu::Picture`] boxes it: a `Draw` is otherwise a few
    /// words and this is two matrices and nine constants.
    FuryBackdrop(Box<crate::backdrop::Frame>),
    /// The HD-style `BackgroundAnim` scene, drawn and filtered into a grey
    /// drawing, composited here in the list's order. Like
    /// [`Self::FuryBackdrop`] it is what the page sits on, not part of it -
    /// see [`crate::scene_backdrop::Frame`].
    SceneBackdrop(Box<crate::scene_backdrop::Frame>),
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
    /// A sprite **repeated** across its rectangle rather than stretched over
    /// it.
    ///
    /// What an `Image` widget whose `TxtrWidth`/`TxtrHeight` exceed the
    /// texture's own size asks for: the selection screens' `Infohexgrid` is
    /// a 32x16 `hex_bg.mip` drawn as `TxtrWidth="340" TxtrHeight="120"` in a
    /// 170x60 rect - ten and a half tiles across, seven and a half down, at
    /// half size - and sampled off a sheet that way it reads the neighbours
    /// in the sheet instead of the tile again. A separate variant rather
    /// than a field on [`Self::Sprite`] for the reason [`Self::RotatedSprite`]
    /// is: one widget needs it and the rest would carry a `[1.0, 1.0]`.
    TiledSprite {
        /// Where it goes.
        rect: [f32; 4],
        /// The **one tile** in the sheet, in sheet pixels.
        uv: [f32; 4],
        /// How many tiles fit across and down `rect`. Fractional is fine: the
        /// last tile is cut where the rectangle ends.
        repeat: [f32; 2],
        /// Modulating colour.
        color: [f32; 4],
    },
    /// A `<Mode3D><Model>` widget's quad, composited the way the **model's own
    /// `pass_mask`** asks rather than the way every other sprite is.
    ///
    /// **A separate variant rather than a `blend` on [`Self::Sprite`] and
    /// [`Self::RotatedSprite`]**, for the reason those two and
    /// [`Self::ChamferedFill`] already give: sprites are constructed in dozens
    /// of places and every one of them would carry an "ordinary alpha blend"
    /// for the handful of widgets whose art declares something else. It
    /// subsumes the rotation too, so the sight brackets need one variant and
    /// not two crossed with each other.
    ///
    /// **What declares it is the file.** Pulse's three lock-on sight models -
    /// `missile_sight_outer.vex`, `missile_sight_inner.vex` and
    /// `leachbeam_sight.vex` - all carry `pass_mask 0x120e`, whose `0x0200` bit
    /// is [`oag_vex::vex::BlendClass::Additive`], and their embedded
    /// textures put the bracket in the colour channels over black with alpha
    /// pinned at 250/255. Drawn with the ordinary alpha blend every other
    /// sprite uses, each piece of the reticle sat on an opaque black tile; the
    /// blend the model asks for is what makes the black field vanish. Pure's
    /// icon models reach this the same way, off their own reading. See
    /// [`crate::frontend::Placed::blend`] and
    /// [hud.md](../../../docs/ui/hud.md).
    BlendedSprite {
        /// Where it goes: `[x, y, width, height]`, `x`/`y` being the top-left of
        /// the *unrotated* rectangle.
        rect: [f32; 4],
        /// Where it is in the sheet, in sheet pixels.
        uv: [f32; 4],
        /// Modulating colour.
        color: [f32; 4],
        /// Clockwise turn about the rectangle's centre, in radians. Zero for
        /// the pickup icons, which do not turn.
        rotation: f32,
        /// The class the model's own batch declared, or `None` for a model
        /// whose batches are opaque - which composites exactly as
        /// [`Self::Sprite`] does.
        blend: Option<oag_vex::vex::BlendClass>,
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
    /// [`Self::Text`], drawn in a **named font role** rather than whatever
    /// face the frame is already bound to.
    ///
    /// **A separate variant rather than an `Option<&str>` field on
    /// [`Self::Text`]**, for the reason [`Self::ChamferedFill`] and the three
    /// sprite variants already give: `Text` is constructed at two dozen call
    /// sites across the HUD, the loading screen, the picker, the prompts and
    /// the scoreboard, and every one of them would carry a `None` for the one
    /// widget that needs this - the menu's own screen title, when the title
    /// package names a role for it. See [`oag_title::MenuSkin::title_font`].
    ///
    /// The renderer resolves `role` against whichever atlas it loaded for
    /// that role, falling back to the frame's own bound face when it loaded
    /// none - never to a *different* title's answer, and never silently to
    /// `Default` either, both of which would be a face this variant did not
    /// ask for.
    FacedText {
        /// The role, as the language plugin's `<Font><Values name=...>`
        /// spells it - `"Title"` for Wipeout HD's chrome title.
        role: &'static str,
        /// Left or anchor edge, depending on `align`.
        x: f32,
        /// Top edge.
        y: f32,
        /// Scale multiplier applied to the glyph cell.
        scale: f32,
        /// Colour of the glyph body.
        color: [f32; 4],
        /// Colour of the glyph's baked outline, when the font has one - see
        /// [`Self::Text`]'s own `border` field.
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

        if self.in_wipeout2048() {
            return self.draw_wipeout2048();
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
            // Pure's own counterpart to `Show Logo` above: its widgets and
            // nothing else, and `draw_screen_at`/`self.on_screen_for` for
            // the same reason - its thirteen `<Animation>`-wrapped frame-line/
            // bracket/patch widgets wipe in over their own key timeline
            // (`docs/ghidra/functions/psp-pure-eu/title-screen.md`) and only
            // the live boot order has a clock to give them. `self.advance`
            // resets `on_screen_for` to `0.0` on every transition, this one
            // included, so the reveal starts clean. See
            // [`pure_states::TITLE_SCREEN`] for what advancing past it would
            // need that is not yet evidenced.
            let mut out = self.draw_screen_at(pure_states::TITLE_SCREEN, self.on_screen_for);
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

    /// The frame counter `--overlay` puts over a movie leg.
    ///
    /// Whether the movie on this leg has a picture is the caller's to say: the
    /// sequence has more than one movie and they do not decode or fail together,
    /// so a shared flag would report the wrong one. The counter exists for the
    /// case where there is no picture at all - it is the only thing on screen
    /// then, and the only sign the leg is running rather than hung.
    pub(super) fn insert_movie_counter(&self, out: &mut Vec<Draw>, has_picture: bool) {
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
    /// planes and one video draw (`crate::render::Renderer::render` keeps a
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
    /// widget's alpha move, or a `<Animation><Key>`-wrapped widget wipe in.
    /// `f64::INFINITY` (what the public method passes) reads as "settled":
    /// every pulsing widget's alpha lands on the ceiling [`pulse_alpha`]
    /// converges to and every reveal lands on its own final key, so nothing
    /// here changes for a caller that never had a clock to give.
    ///
    /// `pub`, not `pub(super)`, since `--screen-seconds` needs it from
    /// `oag_game::capture` to show a mid-reveal still without moving the
    /// screen into the live boot order - the same reason `--menu-anim-phase`/
    /// `--menu-picker-seconds` exist for the menu pages this does not reach.
    #[must_use]
    pub fn draw_screen_at(&self, name: &str, elapsed: f64) -> Vec<Draw> {
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

        self.draw_backdrops_at(screen, elapsed, &mut out);
        for text in &screen.texts {
            let Some(id) = text.idstring.as_deref().or(text.string.as_deref()) else {
                continue;
            };
            let mut color = argb_to_rgba(text.color);
            color[3] *= pulse_alpha(text, elapsed);
            let scale = text.scale * self.face_scale(&text.font);
            // `vertalign="middle"`: half a line up from the authored `y`,
            // which needs the face's height and so only moves once
            // `set_face_scales` has said what it is.
            let y = match (text.middle, self.default_line_height) {
                (true, Some(line)) => text.y - 0.5 * line * scale,
                _ => text.y,
            };
            out.push(Draw::Text {
                x: text.x,
                y,
                scale,
                color,
                border: None,
                align: Align::parse(&text.align),
                text: self.strings.get_or_id(id).to_string(),
                wrap_width: text.wrap_width,
            });
        }
        out
    }

    /// A screen's solid backdrops and its images, in that order, settled -
    /// see [`Self::draw_backdrops_at`] for the one that can reveal.
    fn draw_backdrops(&self, screen: &Screen, out: &mut Vec<Draw>) {
        self.draw_backdrops_at(screen, f64::INFINITY, out);
    }

    /// [`Self::draw_backdrops`], with `elapsed` seconds since the screen
    /// appeared - the same clock [`pulse_alpha`] reads, so a widget wrapped
    /// in an `<Animation>` can wipe in alongside a `pulse="true"` text
    /// throbbing. `f64::INFINITY` reads as fully revealed: every wrapped
    /// widget's own `TextureWidth` interpolation lands on its final key
    /// (`0.0`, added to the authored width unchanged), matching this
    /// function's own settled behaviour before `reveal` existed.
    fn draw_backdrops_at(&self, screen: &Screen, elapsed: f64, out: &mut Vec<Draw>) {
        for fill in &screen.fills {
            let mut rect = self.fill_rect(fill);
            rect[2] += reveal_delta(&fill.reveal, elapsed);
            out.push(Draw::Fill {
                rect,
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
            // `Centred="true"` names the middle, not the corner - see
            // `Image::centred`. Resolved here because `w`/`h` may be the
            // texture's own, which the parser never had.
            let (x, y) = if image.centred {
                (image.x - w * 0.5, image.y - h * 0.5)
            } else {
                (image.x, image.y)
            };

            // `U`/`V`/`TxtrWidth`/`TxtrHeight` name a sub-rect of `src`'s own
            // texture, in that texture's own pixels - not the whole thing,
            // which is what every image drew before this existed. Several
            // widgets sharing one texture (`ArrowSelect`, and three widgets on
            // Pure's `Title Screen`) need their own patch of it rather than
            // all drawing its top-left corner. Absent means the whole placed
            // texture, exactly as it did before. See [`crate::screen::Image::u`].
            let mut uv = [
                placed.x as f32 + image.u.unwrap_or(0.0),
                placed.y as f32 + image.v.unwrap_or(0.0),
                image.texture_width.unwrap_or(placed.width as f32),
                image.texture_height.unwrap_or(placed.height as f32),
            ];

            // A wrapped `<Animation>`'s own `TextureWidth` shrinks both the
            // sampled and the rendered width by the same amount, rather than
            // stretching the full texture into a narrower box - a live
            // capture of `Title Screen`'s own barcode patch mid-reveal shows
            // a crisp, undistorted partial pattern growing from the left
            // (`docs/ghidra/functions/psp-pure-eu/title-screen.md`'s own
            // reveal section), not a squeezed one, which is what pins this
            // over the equally-decompiled alternative of leaving the sample
            // full and shrinking only the render rect.
            let delta = reveal_delta(&image.reveal, elapsed);
            let w = w + delta;
            uv[2] += delta;

            let mut color = argb_to_rgba(image.color);
            color[3] *= fade_alpha(image.transition, elapsed);

            out.push(Draw::Sprite {
                rect: [x, y, w, h],
                uv,
                color,
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
            // A button-glyph prompt draws in the `Buttons` face where the title
            // reads its picker from the loaded faces - see
            // `oag_title::BootProfile::picker_from_loaded_faces`.
            let button_glyph = self.picker_line_height.is_some()
                && matches!(
                    text.name.as_deref(),
                    Some("ControlTextConfirmButton" | "ControlTextBackButton")
                );
            let draw = Draw::Text {
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
            };
            out.push(match draw {
                Draw::Text {
                    x,
                    y,
                    scale,
                    color,
                    align,
                    text,
                    ..
                } if button_glyph => Draw::FacedText {
                    role: crate::language::roles::BUTTONS,
                    x,
                    y,
                    scale,
                    color,
                    border: None,
                    align,
                    text,
                    wrap_width: None,
                },
                other => other,
            });
        }

        // `DisplayLanguages` plus `Menu`: one row per language the disc offers,
        // at the `Menu` widget's own position, scale, colour and alignment.
        // There is no per-row height in the XML - `DisplayLanguages` only
        // populates the list `Menu` draws - so row spacing is inferred from
        // the widget's own font's line height, the same quantity a line of
        // that font would step by anywhere else it is used.
        let menu = screen.menu.as_ref();
        let LanguageRows {
            x: menu_x,
            y: menu_y,
            scale,
            pitch: row,
            align,
        } = self.language_rows(menu);
        // **The title's own measured unselected ink wins over the widget's
        // `color`**, where it measured one. Pure's `<Menu>` says
        // `color="FEGlobals->TextColor"`, and `TextColor` is `0xFF11ACD0` -
        // which the Main Menu capture behind `MenuSkin::selected`
        // (`0xFF16AED1`) shows is the *selected* row's ink, not an
        // unselected one; its unselected rows sample `0xFF88D6E8`, lighter.
        // Taking the widget's attribute literally paints both rows the same
        // colour and the list loses its selection cue entirely. Until the
        // style skin was read, `TextColor` resolved to the sampled
        // `0xFF88D6E8` and the two agreed by accident.
        let color = self
            .menu_skin
            .and_then(|skin| skin.text)
            .or_else(|| menu.map(|m| m.color))
            .map_or([1.0, 1.0, 1.0, 1.0], argb_to_rgba);
        // **A title with a measured selected ink marks the row by ink, as its
        // own menus do, and draws no bar.** `selected_ink` is the one rule
        // `menu::Skin::selected` also reads, so Pulse's picker pulses with the
        // main menu's period and Pure's holds its static ink. The old bar was
        // this viewer's own affordance (no capture of either title's picker
        // shows one; Pure's does not, see `docs/formats/pure-status.md`).
        let ink_skin = self.menu_skin.filter(|skin| skin.selected.is_some());
        let band = self.language_band(&LanguageRows {
            x: menu_x,
            y: menu_y,
            scale,
            pitch: row,
            align,
        });
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a pulse clock is seconds; f32 holds it"
        )]
        let elapsed = if self.on_screen_for.is_finite() {
            self.on_screen_for as f32
        } else {
            0.0
        };

        for (index, language) in self.languages.iter().enumerate() {
            let y = menu_y + index as f32 * row;
            let selected = index == self.selected;
            let row_color = match (selected, ink_skin) {
                (true, Some(skin)) => crate::menu::selected_ink(skin, color, elapsed),
                _ => color,
            };
            // Without a measured selected ink (HD, Omega, a bare fixture) a
            // translucent band is the only cue, **chosen, not measured**, and
            // it is the rect the pointer hit-tests: [`Self::language_band`].
            if selected && ink_skin.is_none() {
                out.push(Draw::Fill {
                    rect: [band.left, band.top + index as f32 * row, band.width, row],
                    color: [0.37, 0.86, 0.96, 0.35],
                });
            }
            out.push(Draw::Text {
                x: menu_x,
                y,
                scale,
                color: row_color,
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

mod storage_warning;
mod transform;
pub(super) mod widget_alpha;
use widget_alpha::{fade_alpha, pulse_alpha, reveal_delta};
