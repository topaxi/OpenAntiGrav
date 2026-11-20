use super::*;
use oag_ui::prompt::sub_char;

const TABLE: &Prompts = &Prompts {
    scope: oag_title::prompts::Scope::AnyFace,
    glyphs: &[
        ('Q', Prompt::Cross),
        ('W', Prompt::Circle),
        ('L', Prompt::L),
    ],
};

fn no_keys(_: Button) -> Vec<&'static str> {
    Vec::new()
}

#[test]
fn every_glyph_a_family_can_ask_for_is_in_the_art() {
    for family in PromptFamily::ALL {
        for prompt in [
            Prompt::Cross,
            Prompt::Circle,
            Prompt::Square,
            Prompt::Triangle,
            Prompt::L,
            Prompt::R,
            Prompt::Up,
            Prompt::Down,
            Prompt::Left,
            Prompt::Right,
        ] {
            let name = glyph_name(prompt, family, Some("ENTER"));
            assert!(art_index(&name).is_some(), "{family:?} {prompt:?} {name}");
        }
    }
    for key in oag_input::Bindings::default().to_pairs().keys() {
        let name = glyph_name(Prompt::Cross, PromptFamily::Keyboard, Some(key.as_str()));
        assert!(art_index(&name).is_some(), "{key} -> {name}");
    }
}

#[test]
fn nintendos_south_button_is_b_and_xboxs_is_a() {
    let xbox = glyph_name(Prompt::Cross, PromptFamily::Xbox, None);
    let nintendo = glyph_name(Prompt::Cross, PromptFamily::Nintendo, None);
    assert_eq!(xbox, "xbox-a");
    assert_eq!(nintendo, "xbox-b");
    assert_eq!(
        glyph_name(Prompt::Circle, PromptFamily::Nintendo, None),
        "xbox-a"
    );
}

#[test]
fn the_disc_family_substitutes_nothing() {
    assert!(substitution(TABLE, None, false, &no_keys).is_none());
}

#[test]
fn the_xbox_family_turns_the_cross_stand_in_into_the_a_cell() {
    let out = substitution(TABLE, Some(PromptFamily::Xbox), false, &no_keys);
    let a = art_index("xbox-a").unwrap();
    assert_eq!(
        out.apply("PRESS Q", false),
        format!("PRESS {}", sub_char(0, a))
    );
}

#[test]
fn the_keyboard_family_shows_the_bound_key() {
    let enter = |button: Button| {
        if button == Button::Cross {
            vec!["ENTER"]
        } else {
            vec!["Z"]
        }
    };
    let out = substitution(TABLE, Some(PromptFamily::Keyboard), false, &enter);
    let cross = art_index("keyboard-enter").unwrap();
    let circle = art_index("keyboard-z").unwrap();
    assert_eq!(
        out.apply("Q W", false),
        format!("{} {}", sub_char(0, cross), sub_char(1, circle))
    );
}

#[test]
fn every_title_that_read_its_glyphs_swaps_all_six() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
    ] {
        let prompts = title.prompts;
        let out = substitution(prompts, Some(PromptFamily::Xbox), false, &no_keys);
        let in_buttons_face = prompts.scope == oag_title::prompts::Scope::ButtonsFace;
        assert_eq!(prompts.glyphs.len(), 6, "{}", title.name);
        for &(stand_in, _) in prompts.glyphs {
            let text = stand_in.to_string();
            assert_ne!(
                out.apply(&text, in_buttons_face).as_ref(),
                text,
                "{}: {stand_in:?} is not swapped",
                title.name
            );
        }
    }
}

const FACES: &Prompts = &Prompts {
    scope: oag_title::prompts::Scope::AnyFace,
    glyphs: &[('W', Prompt::Circle), ('T', Prompt::Triangle)],
};

#[test]
fn the_disc_glyphs_exchange_the_circle_and_the_triangle_for_a_chosen_back() {
    let out = substitution(FACES, None, true, &no_keys);
    assert_eq!(out.apply("W Back T Help", false), "T Back W Help");
}

#[test]
fn a_family_draws_the_other_face_button_when_exchanged() {
    let out = substitution(FACES, Some(PromptFamily::Xbox), true, &no_keys);
    let (b, y) = (art_index("xbox-b").unwrap(), art_index("xbox-y").unwrap());
    // The disc's circle is drawn as Y, its triangle as B.
    assert_eq!(
        out.apply("W T", false),
        format!("{} {}", sub_char(0, y), sub_char(1, b))
    );
}

#[test]
fn the_keyboard_shows_the_key_of_the_exchanged_button() {
    let keys = |button: Button| match button {
        Button::Circle => vec!["Z"],
        Button::Triangle => vec!["V"],
        _ => Vec::new(),
    };
    let out = substitution(FACES, Some(PromptFamily::Keyboard), true, &keys);
    let (z, v) = (
        art_index("keyboard-z").unwrap(),
        art_index("keyboard-v").unwrap(),
    );
    assert_eq!(
        out.apply("W T", false),
        format!("{} {}", sub_char(0, v), sub_char(1, z))
    );
}

/// Which pressing and choice exchange what, for every title (`oag_title::Title::menu_buttons`).
#[test]
fn the_choice_resolves_on_every_title_and_pressing() {
    use oag_disc::Platform;
    use oag_title::prompts::{MenuBack, MenuButtons};
    let help = Prompt::Triangle;
    let ps2 = oag_pulse::TITLE.menu_buttons(Platform::Ps2, MenuBack::Original, Prompt::Circle);
    assert_eq!(
        ps2.back,
        Prompt::Triangle,
        "Pulse's PS2 backs on the triangle"
    );
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        for platform in [Platform::Psp, Platform::Ps2, Platform::Ps3, Platform::Ps4] {
            let original = title
                .front_end
                .map_or(Prompt::Circle, |f| f.back_button(platform));
            let authored = if original == Prompt::Triangle {
                Prompt::Circle
            } else {
                help
            };
            let kept = title.menu_buttons(platform, MenuBack::Original, authored);
            assert_eq!(
                kept,
                MenuButtons::resolve(original, MenuBack::Original, authored)
            );
            assert_eq!((kept.back, kept.help), (original, authored));
            assert!(!kept.glyph_swap, "{} {platform:?}", title.name);
            let circle = title.menu_buttons(platform, MenuBack::Circle, authored);
            assert_eq!(circle.back, Prompt::Circle, "{} {platform:?}", title.name);
            assert_eq!(circle.help, Prompt::Triangle, "{} {platform:?}", title.name);
            let triangle = title.menu_buttons(platform, MenuBack::Triangle, authored);
            assert_eq!(
                triangle.back,
                Prompt::Triangle,
                "{} {platform:?}",
                title.name
            );
            assert_eq!(triangle.help, Prompt::Circle, "{} {platform:?}", title.name);
        }
    }
}

/// The buttons a screen is handed (`crate::prompts::roles`), for every title and pressing under
/// every choice: Pulse's PS2 backs on the triangle and helps on the circle, every other title and
/// pressing the other way round; a choice moves Back and puts the button it displaced where Back
/// was (`docs/ui/back-and-help.md`).
#[test]
fn the_roles_a_screen_is_handed_on_every_title_pressing_and_choice() {
    use oag_disc::Platform;
    use oag_title::prompts::MenuBack;
    let (circle, triangle) = (Button::Circle, Button::Triangle);
    let roles = |title: &oag_title::Title, platform, choice, help| {
        let roles = super::roles(title, platform, choice, help);
        (roles.back, roles.help)
    };
    for (choice, ps2, other) in [
        (MenuBack::Original, (triangle, circle), (circle, triangle)),
        (MenuBack::Circle, (circle, triangle), (circle, triangle)),
        (MenuBack::Triangle, (triangle, circle), (triangle, circle)),
    ] {
        assert_eq!(
            roles(oag_pulse::TITLE, Platform::Ps2, choice, circle),
            ps2,
            "Pulse PS2 {choice:?}"
        );
        for title in [
            oag_pulse::TITLE,
            oag_pure::TITLE,
            oag_hd::TITLE,
            oag_omega::TITLE,
            oag_2048::TITLE,
        ] {
            for platform in [Platform::Psp, Platform::Ps3, Platform::Ps4, Platform::Vita] {
                assert_eq!(
                    roles(title, platform, choice, triangle),
                    other,
                    "{} {platform:?} {choice:?}",
                    title.name
                );
            }
        }
        // Pure's PS2 pressing is not Pulse's: only Pulse binds `backward` to the triangle.
        assert_eq!(
            roles(oag_pure::TITLE, Platform::Ps2, choice, triangle),
            other,
            "Pure PS2 {choice:?}"
        );
    }
    // A Help the disc authors on the square is not a face button Back displaces.
    assert_eq!(
        roles(
            oag_pulse::TITLE,
            Platform::Ps2,
            MenuBack::Circle,
            Button::Square
        ),
        (circle, Button::Square)
    );
}
