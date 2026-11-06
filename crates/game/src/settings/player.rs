//! The `[player]` section: the name every title shares and the tag two of them use.
//!
//! Kept automatically when the first-run prompt is accepted (modern saves: no save slot). One
//! name serves every title; whether a title asks for it, and for a tag, is `Title::profile`.

use serde::{Deserialize, Serialize};

/// Longest name any title's own entry holds (`oag_title::profile::Profile::NAME_AND_TAG`,
/// `length="10"`). The shared name is cut to the smallest limit so every title can show it.
pub const MAX_NAME: usize = 10;

/// Longest tag (`length="3"`).
pub const MAX_TAG: usize = 3;

/// What the player is called, and the short tag Pulse and Pure draw beside it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Empty until the first-run prompt is answered.
    #[serde(default)]
    pub name: String,
    /// Empty until a title that uses a tag has asked for one.
    #[serde(default)]
    pub tag: String,
}

/// Whether `c` may be part of a shared name or tag.
///
/// The intersection of what every consumer can show: Pulse's `<TagInput>` alphabet
/// (`oag_pulse::tag_input::ALPHABET`), the on-screen grid (`oag_ui_screens::prompt`) and the
/// online lobby's name keys, case aside. Letters, digits and the hyphen: the grid's underscore
/// is not in Pulse's alphabet, and its space and punctuation are not in the lobby's.
#[must_use]
pub fn allowed(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// `text` as a shared name: disallowed characters dropped, cut to [`MAX_NAME`], trimmed.
#[must_use]
pub fn clean_name(text: &str) -> String {
    text.chars()
        .filter(|&c| allowed(c))
        .take(MAX_NAME)
        .collect()
}

/// `text` as a tag: letters and digits only, upper case, cut to [`MAX_TAG`].
#[must_use]
pub fn clean_tag(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .take(MAX_TAG)
        .collect()
}

impl Player {
    /// A copy with both fields cleaned, so a hand-edited file cannot hold what a title's font
    /// cannot draw.
    #[must_use]
    pub fn cleaned(&self) -> Self {
        Self {
            name: clean_name(&self.name),
            tag: clean_tag(&self.tag),
        }
    }

    /// Stores `text` as the name. An answer with nothing drawable in it leaves the stored name
    /// alone and returns `false`.
    pub fn set_name(&mut self, text: &str) -> bool {
        let name = clean_name(text);
        let keep = !name.is_empty();
        if keep {
            self.name = name;
        }
        keep
    }

    /// As [`Self::set_name`], for the tag.
    pub fn set_tag(&mut self, text: &str) -> bool {
        let tag = clean_tag(text);
        let keep = !tag.is_empty();
        if keep {
            self.tag = tag;
        }
        keep
    }

    /// The name, or `None` while none is stored. `multiplayer-tags` and the lobby read this.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        (!self.name.is_empty()).then_some(self.name.as_str())
    }

    /// The tag, or `None` while none is stored.
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        (!self.tag.is_empty()).then_some(self.tag.as_str())
    }
}

/// What a launch should ask for before the front end is usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Nothing.
    Nothing,
    /// A name, then a tag if the title uses one.
    Name,
    /// The name is stored; this title uses a tag and none is stored.
    Tag,
}

/// What to ask, given what is stored.
///
/// `interactive` is false for every scripted, captured or original-comparison launch (an
/// allowlist computed in `main::cli`), `asked` is true once the player has answered or declined
/// this session.
#[must_use]
pub fn what_to_ask(
    profile: &oag_title::profile::Profile,
    stored: &Player,
    interactive: bool,
    asked: bool,
) -> Ask {
    if !interactive || asked {
        return Ask::Nothing;
    }
    if profile.asks_for_name() && stored.name().is_none() {
        Ask::Name
    } else if profile.asks_for_tag() && stored.tag().is_none() {
        Ask::Tag
    } else {
        Ask::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_title::profile::Profile;

    fn stored(name: &str, tag: &str) -> Player {
        Player {
            name: name.into(),
            tag: tag.into(),
        }
    }

    #[test]
    fn every_allowed_character_is_in_pulses_alphabet_and_the_lobbys_keys() {
        for c in (0..=127u8).map(char::from).filter(|&c| allowed(c)) {
            assert!(
                oag_pulse::tag_input::ALPHABET.contains(c),
                "{c:?} not in Pulse's alphabet"
            );
            let folded = c.to_ascii_lowercase();
            assert!(
                "abcdefghijklmnopqrstuvwxyz0123456789-_".contains(folded),
                "{c:?} not a lobby key"
            );
        }
    }

    #[test]
    fn a_name_is_cleaned_and_cut_to_the_shortest_title_limit() {
        assert_eq!(clean_name("Zo e_Strange!!!Long"), "ZoeStrange");
        assert_eq!(clean_name("a-b"), "a-b");
    }

    #[test]
    fn a_tag_is_upper_case_letters_and_digits() {
        assert_eq!(clean_tag("a-bcd"), "ABC");
    }

    #[test]
    fn a_prompt_is_asked_for_when_a_title_wants_a_name_and_none_is_stored() {
        assert_eq!(
            what_to_ask(&Profile::NAME_AND_TAG, &Player::default(), true, false),
            Ask::Name
        );
        assert_eq!(
            what_to_ask(&Profile::NAME_ONLY, &Player::default(), true, false),
            Ask::Name
        );
    }

    #[test]
    fn a_title_with_no_name_is_never_asked() {
        assert_eq!(
            what_to_ask(&Profile::NEITHER, &Player::default(), true, false),
            Ask::Nothing
        );
    }

    #[test]
    fn a_stored_name_is_not_asked_for_again() {
        assert_eq!(
            what_to_ask(&Profile::NAME_ONLY, &stored("zoe", ""), true, false),
            Ask::Nothing
        );
    }

    #[test]
    fn a_tag_is_asked_for_only_where_the_title_has_one() {
        let named = stored("zoe", "");
        assert_eq!(
            what_to_ask(&Profile::NAME_AND_TAG, &named, true, false),
            Ask::Tag
        );
        assert_eq!(
            what_to_ask(&Profile::NAME_ONLY, &named, true, false),
            Ask::Nothing
        );
        assert_eq!(
            what_to_ask(&Profile::NAME_AND_TAG, &stored("zoe", "ZOE"), true, false),
            Ask::Nothing
        );
    }

    #[test]
    fn automation_and_a_declined_prompt_ask_nothing() {
        assert_eq!(
            what_to_ask(&Profile::NAME_AND_TAG, &Player::default(), false, false),
            Ask::Nothing
        );
        assert_eq!(
            what_to_ask(&Profile::NAME_AND_TAG, &Player::default(), true, true),
            Ask::Nothing
        );
    }

    #[test]
    fn an_empty_answer_never_blanks_a_stored_name_or_tag() {
        let mut player = stored("Zoe", "ZOE");
        assert!(!player.set_name(""));
        assert!(!player.set_name("!!!"));
        assert!(!player.set_tag(""));
        assert_eq!(player, stored("Zoe", "ZOE"));
        assert!(player.set_name("kim"));
        assert!(player.set_tag("kim"));
        assert_eq!(player, stored("kim", "KIM"));
    }

    #[test]
    fn a_stored_name_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("oag-player-name-{}", std::process::id()));
        let file = dir.join("settings.toml");
        let mut first = super::super::load_from(&file).expect("a fresh profile loads");
        assert_eq!(first.player.name(), None);
        first.player = stored("Zoe-7", "ZOE");
        std::fs::write(&file, super::super::render(&first).expect("renders")).expect("writes");
        let second = super::super::load_from(&file).expect("reloads");
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(second.player, stored("Zoe-7", "ZOE"));
    }
}
