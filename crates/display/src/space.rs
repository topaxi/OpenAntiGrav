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
/// the menu backdrop's rect in `oag_game::main` and `oag_game::capture`, and the
/// no-movie aspect fallback in `oag_game::boot`. See [`pillarbox_in`].
///
/// The remaining readers are console facts under
/// [ADR-0004](../../../docs/architecture/adr/0004-asset-pipeline.md) rather than
/// title ones, which is why none of them moved to a title package either:
///
/// - `oag_game::loading`, whose layout this project authored, in these pixels;
/// - `oag_race::AUTHORED_ASPECT`, where the original's authored field of view is
///   only defined at the PSP's aspect;
/// - `oag_game::hud::inside_screen`'s own tests, which still exercise the PSP's
///   480x272 by name even though the function itself takes a `screen`
///   parameter and checks the PS2's 640x448 grid just as well - see its doc
///   comment for the coordinate-by-coordinate measurement of why the raw XML
///   not being a flat scaling does not stop that PS2 sweep from working.
pub const SCREEN: (f32, f32) = (480.0, 272.0);

/// The aspect ratio the PS2 engine builds its 3D projection at: `640 / 448`.
///
/// `Camera_SubmitScene` (`0x0013e280`) loads this literal (`0x3fb6db6e` at
/// `0x0013e604`) and passes it to the perspective build, widening it by `4/3`
/// only when the `Aspect Ratio` option reads `16:9` - see
/// `docs/ps2/aspect-ratio.md`. It is the PS2's own frame, 640 by 448, and not
/// the PSP's 480 by 272. Read live off a running PCSX2, 2026-10-01: the race
/// camera's matrix has `m11 / m00 = 1.42857` exactly and a vertical field of
/// `60.00000` degrees against the craft's `<ExternalCameraClose fov="60">`.
///
/// **Not the option's `16:9` widen**: that multiply is not implemented here, and
/// the option's out-of-box value is unmeasured, so this is the `4:3` setting's
/// number (the one the savestate flew).
pub const PS2_CAMERA_ASPECT: f32 = 640.0 / 448.0;

/// The viewport shape an authored `fov` is defined at, for the executable that
/// runs the race.
///
/// The PSP's own 480/272 for every platform but the PS2, whose engine builds the
/// projection at [`PS2_CAMERA_ASPECT`]. **Only the PS2 is measured**; every
/// other source keeps the PSP's shape, which is the project's long-standing
/// choice rather than a reading of those titles' own projection (HD's and
/// 2048's are not read here).
///
/// This is the aspect `oag_game::race`'s `fit_vertical_fov` holds the horizontal
/// field at, so it only changes what a window *narrower* than the PSP's draws:
/// at or above it the authored vertical field is kept whichever constant is
/// used.
#[must_use]
pub fn camera_authored_aspect(platform: oag_disc::Platform) -> f32 {
    match platform {
        oag_disc::Platform::Ps2 => PS2_CAMERA_ASPECT,
        _ => SCREEN.0 / SCREEN.1,
    }
}

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
/// `oag_game::hud::inside_screen` is where the mixed meaning of `x` matters.
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
    /// Wipeout 2048's 960x544 square pixels, shown as itself - the Vita's
    /// own panel.
    ///
    /// **Measured, confidence 95.** Every HUD widget in a 960x544 frame of
    /// the running original (Vita3K, 2026-09-16,
    /// `docs/reverse-engineering/vita3k-capture.md`) sits at its authored
    /// rectangle read as Vita pixels: `LapTxt` at `(15, 10)`, `EnergyBgFrame`
    /// at `(15.5, 360)` 71x134, `ThrustBarBG` at `(674, 479)` 271x50,
    /// `EnergyText` centred on `(51, 509)`. The same idiom the reticle's
    /// `(-480, 272)` placeholder already implied
    /// (`oag_2048::hud::ART`), now read off the screen rather than inferred
    /// from a negated centre. Before this constant existed a Vita source fell
    /// through to [`Self::PSP`], which drew the top-left labels at twice their
    /// size and put the whole right-hand column of widgets off the screen.
    pub const VITA: Self = Self {
        size: (960.0, 544.0),
        display_aspect: 960.0 / 544.0,
    };

    /// Wipeout: Omega Collection's 1920x1080 square pixels, shown as itself.
    ///
    /// Its front end is HD's `PI001` plugin carried forward, and its `skin.xml`
    /// authors the same grid: `Studio Logo`'s `<Movie>` states `Width="1920"
    /// height="1080"` and the whole of `mainmenu_definition.xml` reads against
    /// it (`oag_omega::frontend::MENU_SKIN::space`). A PS4 source used to fall
    /// through [`Space::of`] to [`Self::PSP`], so HD-scale coordinates were
    /// drawn on a 480x272 grid and text came out about four times too large.
    ///
    /// **Grid measured, confidence 85**, on three sites in Omega's own
    /// `skin.xml` (`omega_font_scale_ground_truth`): the legal line centred at
    /// `x="960" y="972"`, and two `line.gtf` rules at `x="160"
    /// width="1600"`, symmetric about 960, at `y="110"` and `y="975"`. A
    /// separate constant rather than a reuse of [`Self::HD`] because the grid
    /// is Omega's own reading; it happens to agree with HD's. The panel
    /// a PS4 presents this on is **chosen, not measured**: 16:9 is assumed
    /// from the grid alone.
    pub const OMEGA: Self = Self {
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

    /// Grid units per texel of the source's own font atlases.
    ///
    /// `1.0` for every source but Omega, whose `helv`, `helvb` and
    /// `PS_BUTTONS` are HD's faces at twice the pixel size (median glyph
    /// width, height and advance ratio 2.0, all 242 shared glyphs of `helv`
    /// and `helvb` within 1.7-2.4) under `skin.xml` numbers that are HD's own.
    /// **0.5 is an inference, confidence 60**: the ratio is measured off the
    /// disc, that the PS4 build draws them at half size is not - nothing of
    /// the PS4 executable's text path has been read.
    #[must_use]
    pub fn font_texel_scale(platform: oag_disc::Platform) -> f32 {
        // Keyed on the platform: `OMEGA` and `HD` are the same numbers.
        if platform == oag_disc::Platform::Ps4 {
            0.5
        } else {
            1.0
        }
    }

    /// The space a source authors in, from what its archives say it is.
    #[must_use]
    pub fn of(platform: oag_disc::Platform) -> Self {
        match platform {
            oag_disc::Platform::Ps2 => Self::PS2,
            oag_disc::Platform::Ps3 => Self::HD,
            oag_disc::Platform::Vita => Self::VITA,
            oag_disc::Platform::Ps4 => Self::OMEGA,
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
/// `oag_game::movie::Movie::display_aspect`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ps4_source_authors_in_1080p_not_the_psp_grid() {
        let space = Space::of(oag_disc::Platform::Ps4);
        assert_eq!(space, Space::OMEGA);
        assert_ne!(space, Space::PSP);
        assert_eq!(space.size, (1920.0, 1080.0));
    }

    #[test]
    fn only_omega_reads_its_font_texels_at_half_size() {
        use oag_disc::Platform;
        assert_eq!(Space::font_texel_scale(Platform::Ps4), 0.5);
        for other in [Platform::Psp, Platform::Ps2, Platform::Ps3, Platform::Vita] {
            assert_eq!(Space::font_texel_scale(other), 1.0);
        }
    }

    /// The literal `Camera_SubmitScene` loads at `0x0013e604`, bit for bit.
    #[test]
    fn the_ps2_camera_aspect_is_the_literal_the_engine_loads() {
        assert_eq!(PS2_CAMERA_ASPECT.to_bits(), 0x3fb6_db6e);
        assert_eq!(
            camera_authored_aspect(oag_disc::Platform::Ps2).to_bits(),
            0x3fb6_db6e
        );
    }

    /// Every platform but the PS2 keeps the shape this project has always
    /// fitted the field of view to - the same expression `AUTHORED_ASPECT` is,
    /// so the PSP's projection cannot move.
    #[test]
    fn every_other_platform_keeps_the_psps_authored_aspect() {
        use oag_disc::Platform;
        for platform in [Platform::Psp, Platform::Ps3, Platform::Vita, Platform::Ps4] {
            assert_eq!(
                camera_authored_aspect(platform).to_bits(),
                (SCREEN.0 / SCREEN.1).to_bits()
            );
        }
    }
}
