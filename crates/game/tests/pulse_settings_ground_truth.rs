//! Pulse's Racebox settings screen and the faces its front end draws in,
//! against the disc they are read off.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! What it pins, each of which a plausible edit would silently lose:
//!
//! - `Single Player`'s numbers - the label column at 50, the value column at
//!   250, the first row's text at 35 and a pitch of 23, and the twelve rule
//!   `<Image>`s - read by `oag_ui::menu::SettingsLayout` rather than carried;
//! - the step arrows' two sheet rectangles, which `oag_title::MenuSettings`
//!   carries and `InGame_Definition.xml` authors as plain `<Image>`s;
//! - the `Title` and `small` roles are one `.fnt`, 17 px, so a settings row
//!   drawn in the title's slot is the face the screen names;
//! - the renderer's two glyph slots: `Default` in the face slot and `Title`
//!   in the third.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_ui::frontend::{self, Draw};
use oag_ui::menu::{Definition, Menu, Skin};

fn images() -> Vec<(&'static str, PathBuf)> {
    [
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd"),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd"),
    ]
    .into_iter()
    .filter_map(|(label, name)| oag_testdata::image(name).map(|path| (label, path)))
    .collect()
}

fn shell(image: &Path) -> (boot::Shell, oag_assets::Archives) {
    let options = boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-pulse-settings-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, archives, _title) = boot::load_shell(&options).expect("Pulse's front end boots");
    (shell, archives)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_race_page_reads_the_racebox_settings_screen_off_both_pressings() {
    let images = images();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !images.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pulse PSP image is present"
    );
    for (label, path) in images {
        let (shell, mut archives) = shell(&path);
        let layout = shell
            .frame
            .settings
            .as_ref()
            .unwrap_or_else(|| panic!("{label}: no settings layout was read"));
        assert_eq!(
            (layout.label_x, layout.value_x, layout.first_y, layout.pitch),
            (50.0, 250.0, 35.0, 23.0),
            "{label}: the Single Player screen's own columns and pitch"
        );
        assert_eq!(
            layout.rules.len(),
            12,
            "{label}: six rules, two halves each"
        );
        for rule in &layout.rules {
            let Draw::GradientFill { rect, .. } = rule else {
                panic!("{label}: a rule that does not fade: {rule:?}");
            };
            assert_eq!((rect[2], rect[3]), (190.0, 1.0), "{label}: half a rule");
        }
        assert!(
            layout.arrows.is_some(),
            "{label}: the sheet carries the arrows"
        );

        // The two arrow rectangles are the disc's: `InGame_Definition.xml`
        // draws the same texels as plain images.
        let raw = archives
            .read_name(r"Data\Plugins\PI001\GUI\InGame_Definition.xml")
            .expect("InGame_Definition.xml is on the disc");
        let xml = oag_tables::fexml::text(&raw).expect("it is text");
        for [u, v, w, h] in layout.spec.arrow {
            let wanted = format!(r#"width="{w}" height="{h}" U="{u}" V="{v}""#);
            assert!(
                xml.contains(&wanted),
                "{label}: the pause bar authors no image at {wanted}"
            );
        }

        // Drawn: a page of the built-in tree named in the skin's list picks the
        // rules and the title's face up, and nothing else does.
        let skin = Skin::new(shell.menu_skin, shell.space, 22.0);
        let mut menu = Menu::new(
            Definition::parse(oag_ui::menu::BUILT_IN, &shell.strings).expect("the tree parses"),
        );
        menu.open("race");
        menu.settle();
        let list = oag_ui::menu::draw_list(
            &menu,
            &skin,
            &|_| Vec::new(),
            &|_| 0.0,
            None,
            &shell.frame,
            false,
        )
        .flatten();
        let rules = list
            .iter()
            .filter(|d| matches!(d, Draw::GradientFill { .. }))
            .count();
        assert!(
            rules >= 12,
            "{label}: the race page draws the rules ({rules})"
        );
        assert!(
            list.iter()
                .any(|d| matches!(d, Draw::FacedText { role: "Title", x, .. } if *x == 50.0)),
            "{label}: the labels draw in the Title face at the label column"
        );
        menu.open("options");
        menu.settle();
        let plain = oag_ui::menu::draw_list(
            &menu,
            &skin,
            &|_| Vec::new(),
            &|_| 0.0,
            None,
            &shell.frame,
            false,
        )
        .flatten();
        assert!(
            !plain.iter().any(|d| matches!(d, Draw::GradientFill { .. })),
            "{label}: a page outside the skin's list keeps its plain rows"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_title_and_small_roles_are_one_seventeen_pixel_face_in_the_third_slot() {
    for (label, path) in images() {
        let (shell, _) = shell(&path);
        let title = shell
            .title_font
            .as_ref()
            .unwrap_or_else(|| panic!("{label}: the Title role did not load"));
        assert!(
            (title.line_height - 17.0).abs() < f32::EPSILON,
            "{label}: Pulse_14.fnt is 17 px, got {}",
            title.line_height
        );
        assert!(
            (shell.font.line_height - 13.0).abs() < f32::EPSILON,
            "{label}: the Default face is 13 px"
        );
        let (face, face_role) =
            boot::fonts::face_atlas_slot(shell.menu_skin, &shell.font, shell.title_font.clone());
        assert_eq!(face_role, Some("Default"), "{label}");
        assert!(face.is_some_and(|a| (a.line_height - 13.0).abs() < f32::EPSILON));
        let (third, role) = boot::fonts::third_atlas_slot(
            shell.menu_skin,
            shell.title_font.clone(),
            shell.buttons_font.clone(),
        );
        assert_eq!(
            role, "Title",
            "{label}: Pulse has no button font to put here"
        );
        assert!(third.is_some_and(|a| (a.line_height - 17.0).abs() < f32::EPSILON));
    }
}
