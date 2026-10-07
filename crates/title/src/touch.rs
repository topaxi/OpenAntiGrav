//! A touch-driven front end: an icon grid over a persistent scene.
//!
//! # Why this is not a second `MenuSkin`
//!
//! [`crate::menu::MenuSkin`] describes a vocabulary three titles share and
//! disagree on the numbers of: a `FEGlobals` block driving a scrolling text
//! list or a `<HorizMenu>` strip. Wipeout 2048 ships neither. Its front end -
//! read in full in `docs/formats/2048-frontend.md` - authors no `<FEGlobals>`
//! block anywhere (grepped across all 25 `NEWGUI` files plus the patch's 32,
//! zero hits for `MenuXOffset`/`MenuScale`/`TitleXOffset`/`TitleColor`/etc.)
//! and no `<Menu>` or `<HorizMenu>` widget either. What it authors instead is
//! two touch-icon grids (`GameModeChoice`, `Home`) over a persistent
//! `<FE3DCanvas>` scene. Filling `MenuSkin`'s mandatory fields with numbers
//! this title does not state would be exactly the "plausible-looking
//! stand-in" `CLAUDE.md`'s "never invent what the assets already author"
//! section forbids - a menu shape (a scrolling list, a horizontal strip) 2048
//! never draws.
//!
//! So this is a second, independent type rather than a variant of
//! `MenuSkin`: [`crate::FrontEnd::menu`] and [`crate::FrontEnd::touch`] are
//! both `Option`, and a title fills whichever vocabulary its own front end
//! actually authors - never both, on the evidence read so far, though
//! nothing here forbids a hybrid the next title might turn out to need. See
//! [ADR-0054] for the decision and why `menu` moved from required to
//! optional to make room for this rather than 2048 being forced into it.
//!
//! # A single corpus, on purpose
//!
//! [ADR-0022] licenses an axis once a second corpus is measured - the bar
//! [`crate::menu::MenuSkin`] itself cleared by two disagreeing `Skin.xml`
//! reads. This type does not clear that bar: Wipeout 2048 is the only title
//! in this project's lineage that ships a touch-icon front end at all, so
//! every field below is shaped by one disc's reading. It is still a type
//! and not a pile of constants in `oag-2048` because [`crate::FrontEnd`]
//! already has to hold *some* answer for "how does this title lay its
//! screens out", the same argument that put [`crate::boot::BootProfile`] on
//! `Title` rather than selected separately - and because `Option` costs
//! nothing on the titles that never fill it. A second touch-driven title
//! would test whether this shape generalises; none exists yet, so nothing
//! here claims it does.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md

/// One `<TouchButton>` widget, exactly as its own `<Values>` element authors
/// it.
///
/// **Authored**, confidence 92 the same cap `2048-frontend.md` puts on every
/// claim read straight out of the disc's own XML. Read directly off
/// `data/plugins/frontend/NEWGUI/Definition.xml` in `PSP2/data.psarc`
/// (`data/extracted/vita/PCSF00007/base`), 2026-09-20 - not transcribed from
/// the docs page's own prose, which quotes the shared `140x140` size and the
/// button names but not their individual `x`/`y`; those were re-read from the
/// archive for this type rather than guessed from the size alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchButton {
    /// The widget's own `idstring` - a font/string-table id, not a free
    /// label. Present on every button in both [`TouchFrontEnd::game_mode_choice`]
    /// and [`TouchFrontEnd::home`], unlike the widget's `name` attribute,
    /// which two of `Home`'s five buttons (`FE_PROFILE`, `FE_OPT_PLUS`) omit -
    /// `idstring` is the one identifier every button actually carries.
    pub id: &'static str,
    /// Left edge, in the front end's own 960x544 space - the same space
    /// `2048-hud.md` measured for the in-race HUD.
    pub x: f32,
    /// Top edge, same space as [`Self::x`].
    pub y: f32,
    /// Icon width. `140.0` on every button this type carries - both grids
    /// share one icon size, which is why [`TouchFrontEnd::game_mode_choice`]
    /// and [`TouchFrontEnd::home`] do not need a separate size field.
    pub width: f32,
    /// Icon height. `140.0`, see [`Self::width`].
    pub height: f32,
}

/// One title's touch-driven front end. See the module docs for why this is
/// not a second [`crate::menu::MenuSkin`].
///
/// Held by [`crate::FrontEnd::touch`], on the same footing `menu` is: opening
/// one title's archives while reading another's touch layout is a state that
/// cannot be constructed, for the same reason [`crate::boot::BootProfile`]
/// hangs off `Title` rather than being selected apart from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchFrontEnd {
    /// `NEWGUI/Definition.xml`'s `GameModeChoice` screen, in the file's own
    /// order. A title's own table, so a count is the title's: 2048's is the
    /// v1.04 patch's copy, **six** buttons in two rows of three (the extra two,
    /// `FE_RC_HD` and `FE_RC_FURY`, are what a Vita3K capture shows). The base
    /// package's copy authors four in one row, and `2048-frontend.md`'s
    /// "The Game Mode grid has six tiles" section read that copy: the patch's
    /// own file is what resolves it. DLC gating of the two, if the original has
    /// it, is not read.
    pub game_mode_choice: &'static [TouchButton],
    /// `NEWGUI/Definition.xml`'s `Home` screen, five buttons: team,
    /// community, profile, options, extras, in the file's own order.
    pub home: &'static [TouchButton],
    /// The screen `Team_Definition.xml` nests the ship preview inside:
    /// `<Screen name="team" type="team2048">`, itself under a `teamshell`
    /// wrapper.
    pub team_screen: &'static str,
    /// That screen's own `<Model name="ShipModel">` origin, `(OriginX,
    /// OriginY)` in the same 960x544 space [`TouchButton`]'s coordinates
    /// share. Authored: `<Values OriginX="1400" OriginY="264" ...>`.
    ///
    /// **Real evidence 2048 does preview a mesh on this screen**, the same
    /// class of fact [`crate::FrontEnd::preview_meshes`] exists to carry for
    /// Pulse and Pure - carried here rather than as a `bool` on `FrontEnd`
    /// because `preview_meshes` is defined against [`crate::FrontEnd::race_box`]'s
    /// picker, which 2048 does not fill (its team/track pickers are this
    /// touch idiom, not the `oag_ui_screens::picker::Layout` dialect `race_box`
    /// names), so that field would be inert either way. See
    /// `docs/formats/2048-frontend.md`'s "Team selection is 3D too".
    pub team_model_origin: (f32, f32),
    /// The screen `<FE3DCanvas>` lives inside: `newFEshell`, nested inside its
    /// own `<TouchScroll name="FeShellTouchScroll">`.
    ///
    /// **A screen name, deliberately not a table of coordinates - the same
    /// choice [`crate::FrontEnd::menu_frame`]'s own doc comment makes and for
    /// the same reason.** `NEWGUI/Definition.xml`'s `<FE3DCanvas>` carries
    /// ~50 `<CanvasLabel>` hotspots, and `2048-frontend.md` measured that all
    /// 69 of this file's own labels cluster in `x` 0-196, `y` 0-87 of the
    /// canvas - but that bounding box is `crates/tools/examples/
    /// campaign_map_preview.rs` computed over the label set, not a number the
    /// disc states anywhere. Putting it in this type would be exactly the
    /// hand-transcribed-table failure `CLAUDE.md`'s "never invent" section
    /// names: a derived measurement dressed as authored data, and one that
    /// stops being true the moment a label moves. Naming the screen instead
    /// lets a caller walk the real `<CanvasLabel>` list and play the data.
    pub canvas_screen: &'static str,
    /// The atlas `<CanvasLabel>` UV coordinates index into:
    /// `Data\FE\NewImages\canvasTexture.gxt`.
    ///
    /// **Recovered, not authored on the widget.** No `<FE3DCanvas>` or
    /// `<CanvasLabel>` tag carries a `src` naming this file anywhere in the
    /// 25 `NEWGUI` documents - the association comes from
    /// `campaign_map_preview.rs` cropping this exact texture at each label's
    /// `u`/`v` and landing on the label's own glyph (the `Trophy-2048-*`
    /// family's shared anchor hits the trophy-cup art), which
    /// `2048-frontend.md` records as confirmed rather than inferred. The
    /// entry itself is real and present in the base package, verified this
    /// session (`cargo run -p oag-assets --example psarc_list -- ...
    /// canvasTexture` -> `data/FE/NewImages/canvasTexture.gxt`).
    pub canvas_texture: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2048's own shape, as a fixture: this crate must not know any title's
    /// data, so the sanity check below cannot reach for `oag-2048`.
    const TWO_ZERO_FOUR_EIGHT_SHAPED: TouchFrontEnd = TouchFrontEnd {
        game_mode_choice: &[
            TouchButton {
                id: "FE_SP_CAMPAIGN",
                x: 167.0,
                y: 190.0,
                width: 140.0,
                height: 140.0,
            },
            TouchButton {
                id: "FE_MP_CAMPAIGN",
                x: 329.0,
                y: 190.0,
                width: 140.0,
                height: 140.0,
            },
            TouchButton {
                id: "FE_ADHOC",
                x: 491.0,
                y: 190.0,
                width: 140.0,
                height: 140.0,
            },
            TouchButton {
                id: "FE_CROSSPLAY",
                x: 653.0,
                y: 190.0,
                width: 140.0,
                height: 140.0,
            },
        ],
        home: &[TouchButton {
            id: "ER_TEAM",
            x: 86.0,
            y: 190.0,
            width: 140.0,
            height: 140.0,
        }],
        team_screen: "team",
        team_model_origin: (1400.0, 264.0),
        canvas_screen: "newFEshell",
        canvas_texture: r"Data\FE\NewImages\canvasTexture.gxt",
    };

    /// Every button on `GameModeChoice` shares one icon size, which is what
    /// lets [`TouchFrontEnd`] carry it per-button rather than once for the
    /// whole grid - a title whose two grids disagreed on size would still be
    /// representable.
    #[test]
    fn game_mode_choice_buttons_share_one_icon_size() {
        for button in TWO_ZERO_FOUR_EIGHT_SHAPED.game_mode_choice {
            assert_eq!(button.width, 140.0);
            assert_eq!(button.height, 140.0);
        }
    }

    /// The four `GameModeChoice` buttons run left to right in one row -
    /// ascending `x`, one shared `y` - which is what "a row" in the module
    /// docs and `2048-frontend.md` both mean.
    #[test]
    fn game_mode_choice_runs_in_one_row() {
        let xs: Vec<f32> = TWO_ZERO_FOUR_EIGHT_SHAPED
            .game_mode_choice
            .iter()
            .map(|button| button.x)
            .collect();
        let mut sorted = xs.clone();
        sorted.sort_by(f32::total_cmp);
        assert_eq!(xs, sorted, "the file's own order is already left to right");
        let ys: Vec<f32> = TWO_ZERO_FOUR_EIGHT_SHAPED
            .game_mode_choice
            .iter()
            .map(|button| button.y)
            .collect();
        assert!(ys.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
