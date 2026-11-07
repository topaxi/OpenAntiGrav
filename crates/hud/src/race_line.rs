//! The Eliminator's race line: a bar along the bottom of the screen with one marker per
//! craft at its place round the lap, and a count of the craft in front of and behind the
//! player at either end.
//!
//! HD / Fury's `elimination_hud.xml` composes it from `HUD_Elim_line.xml`: the
//! `EliminatorPositions` bar, two end panels and their arrows (unnamed images), `ShipLinePos0`
//! to `ShipLinePos7` and the `ShipsBehind` / `ShipsInFront` labels. The executable's
//! `Hud_UpdateRaceLine` (`0x00089fa8`, called from `Hud_Update` when `hud+0x44 & 0x4000`) places
//! the markers; `docs/ui/hud.md` ("HD's Eliminator race line") has the decompile and the frames.
//!
//! - **Marker.** Each craft sits at `bar_x + fraction * bar_width`, `fraction` being its
//!   distance into the lap measured from the grid's start offset, wrapped into `[0, 1)`: a
//!   craft still behind the line on the grid reads just under 1, so the whole pack starts at
//!   the bar's right end. The player is `ShipLinePos7` (the red one); the others take
//!   `ShipLinePos0` upwards in craft order.
//! - **Counts.** `ShipsInFront` is the number of other craft whose marker is at or right of the
//!   player's, `ShipsBehind` is every other craft left over.
//!
//! - **Half turn.** A craft facing back down the track (`craft+0x6ad8`) has its marker turned by
//!   pi, `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.
//!
//! The layout is the gate: only the Eliminator roots author these widgets, and a readout without
//! a race line draws none of them.

use crate::{Layout, Sprite};

/// Where each craft is round the lap, as the fraction the bar is read at.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceLine {
    /// The player's own marker.
    pub player: f32,
    /// The player faces back down the track.
    pub player_turned: bool,
    /// The other craft's markers, in craft order.
    pub others: Vec<f32>,
    /// Which of [`Self::others`] face back down the track.
    pub others_turned: Vec<bool>,
}

impl RaceLine {
    /// The other craft whose marker is at or past the player's: `ShipsInFront`.
    #[must_use]
    pub fn in_front(&self) -> usize {
        self.others
            .iter()
            .filter(|&&other| other >= self.player)
            .count()
    }

    /// The other craft behind the player: `ShipsBehind`.
    #[must_use]
    pub fn behind(&self) -> usize {
        self.others.len() - self.in_front()
    }
}

/// The widget the bar is, and the first marker the group's own pieces end at.
const BAR: &str = "EliminatorPositions";
const MARKER_PREFIX: &str = "ShipLinePos";
/// The player's marker, `ShipLinePos7`: the layout's last slot.
const PLAYER_SLOT: usize = 7;

/// What the race line draws: the bar with its end panels, then the markers placed.
///
/// The end panels and their arrows are images with no `name`, so they cannot be picked by one:
/// they are the images the layout authors between the bar and its first marker.
#[must_use]
pub fn sprites(layout: &Layout, line: &RaceLine) -> Vec<Sprite> {
    let Some(bar) = layout.sprites.iter().position(|s| s.name == BAR) else {
        return Vec::new();
    };
    let first_marker = format!("{MARKER_PREFIX}0");
    let Some(markers) = layout.sprites.iter().position(|s| s.name == first_marker) else {
        return Vec::new();
    };
    if markers < bar {
        return Vec::new();
    }
    let bar_rect = layout.sprites[bar].rect;
    let mut drawn: Vec<Sprite> = layout.sprites[bar..markers].to_vec();
    let place = |slot: usize, fraction: f32, turned: bool| -> Option<Sprite> {
        let mut marker = layout.sprite(&format!("{MARKER_PREFIX}{slot}"))?.clone();
        let centre = bar_rect[0] + fraction * bar_rect[2];
        marker.rect[0] = centre - marker.rect[2] / 2.0;
        if turned {
            let [u, v, width, height] = marker.uv;
            marker.uv = [u + width, v + height, -width, -height];
        }
        Some(marker)
    };
    drawn.extend(
        line.others
            .iter()
            .take(PLAYER_SLOT)
            .enumerate()
            .filter_map(|(slot, &fraction)| {
                let turned = line.others_turned.get(slot).copied().unwrap_or(false);
                place(slot, fraction, turned)
            }),
    );
    drawn.extend(place(PLAYER_SLOT, line.player, line.player_turned));
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Readout;
    use crate::draw::draw_list;
    use crate::tests::{context, strings};
    use oag_ui::frontend::Draw;

    /// `HUD_Elim_line.xml` cut down to its shape: the bar, one end panel with an arrow (both
    /// unnamed), two markers and the two counts, on the atlas the sample sheet holds.
    fn fixture() -> String {
        let atlas = oag_pulse::hud::ATLAS;
        format!(
            r#"<Screen><Screen name="HUD">
<Image name="EliminatorPositions" transition="0">
  <Values x="266" y="1030" width="1000" height="6" U="0" V="0" TxtrWidth="8" TxtrHeight="6" src="{atlas}"/>
  <Image transition="0"><Values x="40" y="969" width="226" height="100" U="0" V="0" TxtrWidth="16" TxtrHeight="16" src="{atlas}"/></Image>
  <Text name="ShipsBehind" transition="0"><Values string="" x="100" y="1019" scale="1" font="HUDSmall" align="centre" vertalign="middle"/></Text>
  <Text name="ShipsInFront" transition="0"><Values string="" x="1820" y="1019" scale="1" font="HUDSmall" align="centre" vertalign="middle"/></Text>
  <Image name="ShipLinePos0"><Values centred="true" x="0" y="1012" width="32" height="36" U="0" V="0" TxtrWidth="16" TxtrHeight="18" src="{atlas}"/></Image>
  <Image name="ShipLinePos7"><Values centred="true" x="0" y="1009" width="50" height="42" U="0" V="0" TxtrWidth="33" TxtrHeight="28" src="{atlas}"/></Image>
</Image>
</Screen></Screen>"#
        )
    }

    fn line() -> RaceLine {
        RaceLine {
            player: 0.5,
            others: vec![0.75],
            ..RaceLine::default()
        }
    }

    #[test]
    fn a_craft_facing_back_has_its_marker_turned_by_half_a_turn() {
        let layout = Layout::from_xml(&fixture());
        let upright = sprites(&layout, &line());
        let turned = sprites(
            &layout,
            &RaceLine {
                player_turned: true,
                others_turned: vec![true],
                ..line()
            },
        );
        for (upright, turned) in upright.iter().zip(&turned).skip(2) {
            let [u, v, width, height] = upright.uv;
            assert_eq!(turned.uv, [u + width, v + height, -width, -height]);
            assert_eq!(turned.rect, upright.rect);
        }
        assert_eq!(upright[..2], turned[..2], "the bar and panels stay");
    }

    #[test]
    fn a_tie_counts_as_in_front_and_the_rest_are_behind() {
        let tied = RaceLine {
            player: 0.5,
            others: vec![0.5, 0.25, 0.75],
            ..RaceLine::default()
        };
        assert_eq!((tied.in_front(), tied.behind()), (2, 1));
    }

    #[test]
    fn markers_sit_at_their_fraction_of_the_bar_and_the_panels_come_with_it() {
        let layout = Layout::from_xml(&fixture());
        let drawn = sprites(&layout, &line());
        // The bar and its panel (unnamed), then the opponent and the player.
        assert_eq!(drawn.len(), 4, "{drawn:?}");
        let opponent = &drawn[2];
        let player = &drawn[3];
        assert_eq!(opponent.name, "ShipLinePos0");
        assert_eq!(player.name, "ShipLinePos7");
        // Centre = bar x + fraction * bar width, so the left edge is half a width short of it.
        assert_eq!(
            opponent.rect[0] + opponent.rect[2] / 2.0,
            266.0 + 0.75 * 1000.0
        );
        assert_eq!(player.rect[0] + player.rect[2] / 2.0, 266.0 + 0.5 * 1000.0);
        assert!(sprites(&Layout::from_xml("<Screen/>"), &line()).is_empty());
    }

    /// The readout's race line reaches the screen: the pieces as sprites and the counts as the
    /// layout's own labels. Dropping the hook in `draw::list` or the two `text_for` arms empties
    /// one half each.
    #[test]
    fn the_race_line_reaches_the_draw_list() {
        let layout = Layout::from_xml(&fixture());
        let strings = strings();
        let cx = context(&layout, &strings);
        let none = draw_list(&cx, &Readout::blank());
        let some = draw_list(
            &cx,
            &Readout {
                race_line: Some(line()),
                ..Readout::blank()
            },
        );
        assert_eq!(some.sprites.len() - none.sprites.len(), 4);
        let texts: Vec<_> = some
            .small_text
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["0", "1"], "behind then in front: {texts:?}");
        assert!(
            none.small_text
                .iter()
                .all(|draw| !matches!(draw, Draw::Text { text, .. } if !text.is_empty()))
        );
    }
}
