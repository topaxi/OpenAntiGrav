//! The grid a source places its widgets in, and how a movie is fitted to it.
//!
//! Split out of `frontend.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What holds it
//! together is one question - **what are these coordinates in, and what is that
//! shown as?** - which is exactly the question that stopped having one answer
//! when a second console arrived, and then a third.
//!
//! Three spaces now: the PSP's 480x272, the PS2's 640x448 shown as ~16:9, and
//! Wipeout HD's 1920x1080. [`SCREEN`] survives all three as a constant because
//! its remaining readers are the ones for which the PSP's pixels are right
//! whatever disc is mounted - this project's own loading screen, the HUD bounds
//! check, and `oag_race::AUTHORED_ASPECT`.

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
/// **640x448** grid, and that grid is shown at the PSP's own **480/272**, near
/// enough the 16:9 its own menu offers. Deriving the display aspect from the
/// grid would squeeze the whole front end (640/448 = 1.429 against 1.765), and
/// using the display aspect as the grid would put every widget in the wrong
/// place. Three sites need one or the other and picking the wrong one is silent
/// on the PSP:
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
/// 640x448 is PAL's own frame.
///
/// # Evidence for the PS2 display aspect
///
/// **The grid is shown at 480/272, not at 4:3, and this used to say the
/// opposite.** The same two factors that place the widgets also *size* them: an
/// `<Image>` naming the same texture and the same source rectangle on both
/// discs is drawn 224x24 on the PSP and 299x40 on the PS2, 168x26 against
/// 224x43 in `Arcade_HUD.xml`, and eighteen more pairs in that one file. A
/// rectangle stretched by two different factors is only undistorted when the
/// destination frame is shown at the source's shape, and the arithmetic gives
/// that shape exactly: `A x (1.3333 x 448) / (1.6471 x 640) = 1` at
/// `A = 1.7647 = 480/272`. 16:9, which the disc's own `Aspect Ratio` menu
/// offers, is `(16/9)/(480/272) = 1.0074` of that.
///
/// The executable does the same stretch itself where the data did not:
/// `FUN_001e9370` multiplies a rectangle's `x` by 1.3333334 and its `y` by
/// 1.6470588, and `Loading_Show` is one of its callers. Confidence **95**; see
/// [aspect-ratio](../../../docs/ps2/aspect-ratio.md) for the measurements, for
/// what the disc's own 4:3/16:9 option actually does (it moves the camera and
/// nothing else) and for the intro film, which is anamorphic to this same frame
/// despite declaring 4:3.
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
    /// PAL's 640x448, shown as the PSP's own 480/272 - anamorphic, and 0.74%
    /// off the 16:9 the disc's menu calls it.
    pub const PS2: Self = Self {
        size: (640.0, 448.0),
        display_aspect: SCREEN.0 / SCREEN.1,
    };
    /// Wipeout HD's 1920x1080 square pixels, shown as itself.
    ///
    /// **Sixteen times the PSP's grid, and this type is what stops that being a
    /// disaster.** `oag_hd::frontend::MENU_SKIN` carries `menu_x: 800`, which in
    /// a 480-wide space is 320 pixels off the right-hand edge; its own doc
    /// comment recorded the hazard and left it, on the grounds that the type
    /// should change when something finally drew these. Something does now, and
    /// the type did not have to change after all - `Space` was already the
    /// mechanism, having been built for the PS2's grid.
    ///
    /// Authored, confidence 90, on three independent sites in HD's own
    /// `skin.xml`: a `<Movie>` at `Width="1920" height="1080"`, a
    /// `<BackgroundAnim>` whose origin is `(960, 540)`, and a legal line at
    /// `y="972"`. Square, unlike the PS2's: 1920x1080 is already 16:9 as a pixel
    /// grid, so the two numbers agree where the PS2's disagree by 7%. What is
    /// *not* pinned is whether a safe-zone transform insets this before it is
    /// presented - `obeySafeZone` and `ignoreSafeZoneFullscreen` are real
    /// per-widget attributes on this disc, and nothing here applies them.
    pub const HD: Self = Self {
        size: (1920.0, 1080.0),
        display_aspect: 16.0 / 9.0,
    };

    /// What a texture's own pixel size means in this grid, per axis.
    ///
    /// **Only for the one case where the data gives no size**: an `<Image>` with
    /// no `width`/`height`, of which `Show Logo`'s `pulse_logo.mip` is the only
    /// one in the PS2 front-end XML measured - `Skin.xml`,
    /// `Additional_Definition.xml`, `InGame_Definition.xml`,
    /// `Controls_Definition_PS2.xml` and `Arcade_HUD.xml`, whose other
    /// size-less `src`s are movies, `LoadXML`s and `.vex` models. Everything
    /// else carries a size the source already scaled.
    ///
    /// A texture's dimensions are PSP pixels - `pulse_logo.mip` is the same
    /// 512x128 file on both discs - so drawing one at its texel size in a
    /// 640x448 grid shrinks it against everything around it: the boot logo
    /// would span 62% of the screen where the PSP's spans 80%, at 3/4 of the
    /// shape. These are the factors the PS2 build applies to PSP-grid numbers
    /// itself (`FUN_001e9370`, 1.3333334 on `x` and 1.6470588 on `y`), and
    /// applying them here puts the logo exactly where the PSP has it.
    ///
    /// **Confidence 70, and a reimplementation choice rather than a reading.**
    /// That the port scales PSP-grid *coordinates* this way is measured; that it
    /// scales a *texture's own size* this way is inference, and the original's
    /// default-size path has not been read. See
    /// [aspect-ratio](../../../docs/ps2/aspect-ratio.md).
    #[must_use]
    pub fn texture_scale(self) -> (f32, f32) {
        if self.size == Self::PS2.size {
            (self.size.0 / SCREEN.0, self.size.1 / SCREEN.1)
        } else {
            (1.0, 1.0)
        }
    }

    /// The space a source authors in, from what its archives say it is.
    #[must_use]
    pub fn of(platform: oag_disc::Platform) -> Self {
        match platform {
            oag_disc::Platform::Ps2 => Self::PS2,
            oag_disc::Platform::Ps3 => Self::HD,
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
/// quad this always drew. The PS2's containers report the frame they were cut
/// to fill rather than their decoded size, so they fill it too; a movie that
/// genuinely disagreed with its screen is what the boxing is here for. See
/// `crate::movie::Movie::display_aspect`.
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
/// the PSP and disagree on the PS2 by 24%, so a rect computed from the grid
/// would box a picture that fills the screen. See [`Space`].
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
