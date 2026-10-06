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
    assert!(substitution(TABLE, None, &no_keys).is_none());
}

#[test]
fn the_xbox_family_turns_the_cross_stand_in_into_the_a_cell() {
    let out = substitution(TABLE, Some(PromptFamily::Xbox), &no_keys);
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
    let out = substitution(TABLE, Some(PromptFamily::Keyboard), &enter);
    let cross = art_index("keyboard-enter").unwrap();
    let circle = art_index("keyboard-z").unwrap();
    assert_eq!(
        out.apply("Q W", false),
        format!("{} {}", sub_char(0, cross), sub_char(1, circle))
    );
}
