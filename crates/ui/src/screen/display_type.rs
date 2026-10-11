//! A `Text` widget's authored `DisplayType`.

/// A `Text` widget's authored `DisplayType`, which Pulse's `Text` parser
/// (`FUN_088cdb34`) reads into two bits of the widget's flag byte at `+0x173`.
///
/// **Recorded as authored, and nothing acts on it - on purpose.** The
/// original's `Text` class has no type-in: `SCROLLING` sets both bits and
/// `TELETYPE` sets only `0x20`, and the one place the update reads the pair
/// (`FUN_088cd860`) calls `FUN_088cdb2c`, which is an empty function.
/// Every `Text` draws its whole string on its first frame, `TELETYPE` ones
/// included, confirmed live on PPSSPP. See
/// `docs/ghidra/functions/psp-pulse-usa/text-display-type.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DisplayType {
    /// `DisplayType="NORMAL"`, or no attribute.
    #[default]
    Normal,
    /// `DisplayType="SCROLLING"`.
    Scrolling,
    /// `DisplayType="TELETYPE"`.
    Teletype,
}

impl DisplayType {
    /// The enum for an authored value, compared case-insensitively as the
    /// original's `strcasecmp` does. An unknown value is [`Self::Normal`],
    /// which is what the original's parser leaves the flags at.
    #[must_use]
    pub fn from_authored(value: &str) -> Self {
        if value.eq_ignore_ascii_case("TELETYPE") {
            Self::Teletype
        } else if value.eq_ignore_ascii_case("SCROLLING") {
            Self::Scrolling
        } else {
            Self::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DisplayType;
    use crate::screen::Screens;

    #[test]
    fn the_authored_spellings_map_case_insensitively() {
        assert_eq!(
            DisplayType::from_authored("TELETYPE"),
            DisplayType::Teletype
        );
        assert_eq!(
            DisplayType::from_authored("teletype"),
            DisplayType::Teletype
        );
        assert_eq!(
            DisplayType::from_authored("Scrolling"),
            DisplayType::Scrolling
        );
        assert_eq!(DisplayType::from_authored("NORMAL"), DisplayType::Normal);
        assert_eq!(DisplayType::from_authored("nonsense"), DisplayType::Normal);
    }

    #[test]
    fn a_text_carries_its_display_type_from_attribute_or_values() {
        let screens = Screens::from_xml(
            r#"<Screen name="S">
  <Text name="a"><Values idstring="A" DisplayType="TELETYPE"></Values></Text>
  <Text name="b" DisplayType="SCROLLING"><Values idstring="B"></Values></Text>
  <Text name="c"><Values idstring="C"></Values></Text>
</Screen>"#,
        );
        let texts = &screens.by_name("S").expect("screen S").texts;
        let kinds: Vec<_> = texts.iter().map(|t| t.display_type).collect();
        assert_eq!(
            kinds,
            [
                DisplayType::Teletype,
                DisplayType::Scrolling,
                DisplayType::Normal
            ]
        );
    }
}
