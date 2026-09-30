//! HD/Fury's per-lap history: `Lap1Image`-`Lap4Image` and the
//! `Lap{n}Text`/`Lap{n}Time` pair each one carries.
//!
//! Split out of [`super::draw`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The widget
//! names are a numbered family the same way `ZonePlusN` is, so this is the one
//! place that reads the shape rather than eight arms scattered through
//! [`super::draw::text_for`].

use super::draw::{Context, sprite_draw};
use super::{Draw, Precision, Readout, format_lap_time};

/// Which half of a per-lap history row `Lap{n}Text`/`Lap{n}Time` is: the lap's
/// own number, or its time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LapSplitField {
    /// `Lap{n}Text` - which lap this row is, `n`.
    Number,
    /// `Lap{n}Time` - how long that lap took.
    Time,
}

/// Whether `name` is one of HD/Fury's per-lap history widgets, and which one.
///
/// **A numbered family, like `zone_plus`'s.** `HUD_lap_times.xml` authors
/// eight widgets off one four-row pattern - `Lap1Text`/`Lap1Time` through
/// `Lap4Text`/`Lap4Time`, both halves of each row `string=""` in the layout -
/// rather than eight unrelated names, so this reads the shape once instead of
/// eight arms. The index returned is 0-based, matching
/// [`oag_race::RaceState::lap_splits`]'s own indexing.
fn lap_split_row(name: &str) -> Option<(usize, LapSplitField)> {
    let rest = name.strip_prefix("Lap")?;
    let (digits, field) = if let Some(digits) = rest.strip_suffix("Text") {
        (digits, LapSplitField::Number)
    } else {
        (rest.strip_suffix("Time")?, LapSplitField::Time)
    };
    let lap: usize = digits.parse().ok()?;
    lap.checked_sub(1).map(|index| (index, field))
}

/// The text for one half of a per-lap history row, or `None` when that lap has
/// not been completed - the row draws nothing rather than a zero.
///
/// `index` is only used for [`LapSplitField::Number`]: the row's own number is
/// its position, not anything read off the split itself, and is drawn only once
/// the lap it names has actually finished - a `Lap3Text` reading `3` before lap
/// 3 exists would be claiming a row for a lap that has not happened.
fn lap_split_text(split: Option<u32>, field: LapSplitField, index: usize) -> Option<String> {
    split.map(|ticks| match field {
        LapSplitField::Number => (index + 1).to_string(),
        // A completed lap's time, same as `BestTime`: hundredths, because the
        // clock this reads has stopped rather than still running.
        LapSplitField::Time => format_lap_time(u64::from(ticks), Precision::Hundredths),
    })
}

/// [`super::draw::text_for`]'s whole answer for a per-lap history widget:
/// `Some(None)` when `name` is one of the family but that lap has not been
/// completed, `None` when `name` is not in the family at all - the shape a
/// caller chains with `.unwrap_or_else` into its own fallback.
pub(super) fn lap_split_row_text(name: &str, readout: &Readout) -> Option<Option<String>> {
    lap_split_row(name).map(|(index, field)| {
        lap_split_text(
            readout.lap_splits.get(index).copied().flatten(),
            field,
            index,
        )
    })
}

/// The per-lap history's row backgrounds, `Lap1Image` through `Lap4Image`.
///
/// Not part of [`oag_title::HudArt::always_on`]: unlike that list's widgets,
/// which are up for every frame of a mode that authors them, these four are up
/// only once the lap they belong to has actually been driven - see
/// [`super::draw::draw_list`]'s own use of this and [`lap_split_text`]. The
/// order matches [`oag_race::MAX_RECORDED_LAPS`]'s own indexing, so
/// `LAP_SPLIT_IMAGES[i]` is lap `i + 1`'s row. The compile-time assertion below
/// is what keeps the two from drifting apart if that constant ever changes.
pub(super) const LAP_SPLIT_IMAGES: [&str; 4] = ["Lap1Image", "Lap2Image", "Lap3Image", "Lap4Image"];

const _: () = assert!(LAP_SPLIT_IMAGES.len() == oag_race::MAX_RECORDED_LAPS);

/// The per-lap history's row backgrounds that are up this frame.
///
/// HD's own layout authors all four `Lap1Image`-`Lap4Image` unconditionally at
/// fixed positions, so unlike [`super::draw::pickup_sprites`] there is no art
/// to pick between - only whether each row's lap has actually been driven yet.
/// See `docs/formats/hd-hud.md`.
pub(super) fn lap_split_sprites(cx: &Context<'_>, readout: &Readout) -> Vec<Draw> {
    let mut out = Vec::new();
    for (index, name) in LAP_SPLIT_IMAGES.into_iter().enumerate() {
        if readout.lap_splits.get(index).copied().flatten().is_none() {
            continue;
        }
        if let Some(sprite) = cx.layout.sprite(name) {
            out.extend(sprite_draw(sprite, cx.sheet));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::draw::draw_list;
    use super::*;

    #[test]
    fn the_text_widgets_parse_to_a_zero_based_index() {
        assert_eq!(lap_split_row("Lap1Text"), Some((0, LapSplitField::Number)));
        assert_eq!(lap_split_row("Lap4Time"), Some((3, LapSplitField::Time)));
        assert_eq!(lap_split_row("Lap2Text"), Some((1, LapSplitField::Number)));
    }

    /// Everything that must not be swept up by the family match: the running
    /// lap counter, its separator, the container `<Text>` and the row
    /// backgrounds themselves (`Image`s, not `Text`s, and never reach this
    /// function at all in practice, but a name collision would be silent).
    #[test]
    fn names_outside_the_family_are_not_matched() {
        for name in ["Lap", "LapOf", "LapTimesParent", "CurrentTime", "Lap1Image"] {
            assert_eq!(lap_split_row(name), None, "{name} should not match");
        }
    }

    /// `Lap0Text` names no lap: [`oag_race::RaceState::lap`] starts at `1`, so
    /// there is no zeroth row for the checked subtraction to underflow into.
    #[test]
    fn lap_zero_does_not_underflow() {
        assert_eq!(lap_split_row("Lap0Text"), None);
    }

    #[test]
    fn a_completed_lap_shows_its_number_and_its_time() {
        assert_eq!(
            lap_split_text(Some(60), LapSplitField::Number, 0),
            Some("1".to_string())
        );
        assert_eq!(
            lap_split_text(Some(60), LapSplitField::Time, 0),
            Some("0.01.00".to_string())
        );
    }

    /// The row for a lap not yet driven shows nothing, not a zero - the same
    /// rule `text_for`'s own module docs state for the widget allow-list as a
    /// whole.
    #[test]
    fn an_unset_lap_shows_neither_half() {
        assert_eq!(lap_split_text(None, LapSplitField::Number, 2), None);
        assert_eq!(lap_split_text(None, LapSplitField::Time, 2), None);
    }

    /// End to end, off a fragment shaped exactly like the disc's own
    /// `HUD_lap_times.xml` - two rows, both `string=""` in the layout. Lap 1
    /// has a recorded split; lap 2 does not.
    #[test]
    fn a_completed_row_draws_and_an_unset_one_does_not() {
        const FRAGMENT: &str = r#"
<Text name="LapTimesParent" OffsetX="40" OffsetY="793">
  <Image name="Lap2Image" OffsetX="30" OffsetY="85">
    <Values x="0" y="0" width="221" height="44" U="542" V="187" TxtrWidth="228" TxtrHeight="44" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
    <Text name="Lap2Text"><Values string="" font="HUDSmall" align="centre" scale="1.0" x="35" y="10"></Values></Text>
    <Text name="Lap2Time"><Values string="" font="HUDSmall" align="left" scale="1.0" x="80" y="10"></Values></Text>
  </Image>
  <Image name="Lap1Image" OffsetX="30" OffsetY="125">
    <Values x="0" y="0" width="221" height="44" U="542" V="187" TxtrWidth="228" TxtrHeight="44" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
    <Text name="Lap1Text"><Values string="" font="HUDSmall" align="centre" scale="1.0" x="35" y="10"></Values></Text>
    <Text name="Lap1Time"><Values string="" font="HUDSmall" align="left" scale="1.0" x="80" y="10"></Values></Text>
  </Image>
</Text>
"#;
        let layout = crate::hud::Layout::from_xml(FRAGMENT);
        let strings = oag_ui::language::StringTable::default();
        let sheet = crate::sprite::Sheet::placed_at(&[(
            oag_hd::hud::TEXTURES[0],
            crate::sprite::Placed {
                x: 0,
                y: 0,
                width: 256,
                height: 256,
                quad_extent: None,
                blend: None,
            },
        )]);
        let cx = Context {
            layout: &layout,
            strings: &strings,
            sheet: &sheet,
            art: oag_hd::hud::ART,
            hud_line_height: 25.0,
            small_line_height: 10.0,
            default_line_height: 10.0,
            default_border: layout.default_border(),
        };
        let mut readout = Readout::blank();
        // A minute-and-nothing lap, so its formatted time is unambiguous.
        readout.lap_splits[0] = Some(60);
        let frame = draw_list(&cx, &readout);

        // The gate on the sprite side: lap 1's row background drew, lap 2's -
        // authored identically but for no recorded split - did not.
        assert_eq!(
            frame.sprites.len(),
            1,
            "expected exactly lap 1's row: {frame:?}"
        );

        let small_text: Vec<&str> = frame
            .small_text
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            small_text,
            ["1", "0.01.00"],
            "lap 1's own row should show its number and its time, lap 2's neither"
        );
    }
}
