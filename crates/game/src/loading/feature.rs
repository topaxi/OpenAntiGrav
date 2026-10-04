//! Which feature a race's loading screen shows, and the two words over it.
//!
//! Its own file because `loading.rs` is near the size ceiling, and because the
//! pick is a law read off Wipeout HD's constructor while the drawing is this
//! build's layout.

use super::{Align, Assets, Draw, IMAGE_BOX, MARKER_SIZE, PROSE_BOX};

/// Where a feature's name and its heading go: one row, over the picture and
/// over the paragraph.
pub(super) const TITLE_Y: f32 = 63.0;

/// And how big.
pub(super) const TITLE_SCALE: f32 = 1.1;

/// Which of `assets.features` this screen shows, as a position in that list.
///
/// **A draw from the mode's own deck.** The original reduces `rand()` modulo a
/// range its mode picks, and the set that range covers is the title's
/// [`oag_title::loading::Deck`]; `draw` is a seed rather than a clock read, so
/// a capture of this screen is reproducible. A title with no deck, or a deck
/// none of whose features resolved, draws from everything it has - a screen
/// with no feature would be a worse answer than a wider draw.
///
/// `mode` is the source executable's own mode id, `None` where no race is being
/// covered.
pub(super) fn pick(assets: &Assets, mode: Option<u32>, draw: u64) -> Option<usize> {
    let wanted = assets.deck.map(|deck| deck.indices(mode));
    let mut candidates: Vec<usize> = assets
        .features
        .iter()
        .enumerate()
        .filter(|(_, feature)| wanted.is_none_or(|deck| deck.contains(&feature.slot)))
        .map(|(at, _)| at)
        .collect();
    if candidates.is_empty() {
        candidates = (0..assets.features.len()).collect();
    }
    let draw = oag_core::rng::Rng::new(draw).next_u32() as usize;
    (!candidates.is_empty()).then(|| candidates[draw % candidates.len()])
}

/// The two rows over the picture and over the paragraph, each behind its marker.
///
/// **The original's arrangement**: `PILOT ASSIST` sits over the illustration
/// and `DESCRIPTION` over the prose, in the same row. Until 2026-10-04 this
/// build drew the name over the prose alone and no heading, because the
/// heading's id was unread.
pub(super) struct Rows<'a> {
    pub arrow: Option<[f32; 4]>,
    pub arrow_color: [f32; 4],
    pub color: [f32; 4],
    pub border: Option<[f32; 4]>,
    pub scale: f32,
    pub title: Option<&'a str>,
    pub heading: Option<&'a str>,
}

impl Rows<'_> {
    pub(super) fn draw(&self, out: &mut Vec<Draw>) {
        let columns = [(IMAGE_BOX.0, self.title), (PROSE_BOX.0, self.heading)];
        for (x, text) in columns {
            let Some(text) = text else { continue };
            if let Some(uv) = self.arrow {
                out.push(Draw::Sprite {
                    rect: [x, TITLE_Y + 1.0, MARKER_SIZE * 0.75, MARKER_SIZE * 0.75],
                    uv,
                    color: self.arrow_color,
                });
            }
            out.push(Draw::Text {
                x: x + MARKER_SIZE,
                y: TITLE_Y,
                scale: self.scale,
                color: self.color,
                border: self.border,
                align: Align::Left,
                text: text.to_string(),
                wrap_width: None,
            });
        }
    }
}

/// The source executable's own mode id for a mode this build races, where it
/// is named.
///
/// Only the ones `docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md` names:
/// `3` is `SPArcade` (a single race), `4` `SPTournament`, `5` `SPTimeTrial` and
/// `8` `SPElimination`. **Speed Lap, Zone and Head2Head are `None`**, because the ids that
/// would be theirs are among the seven unnamed ones - `6`, `0xd` and `0xe`
/// carry decks of their own and which of them is Zone is not established. They
/// draw from the default deck, which is chosen, not measured.
#[must_use]
pub fn executable_mode(mode: oag_race::Mode) -> Option<u32> {
    use oag_race::Mode;
    match mode {
        Mode::SingleRace => Some(3),
        Mode::Tournament => Some(4),
        Mode::TimeTrial => Some(5),
        Mode::Eliminator => Some(8),
        Mode::SpeedLap | Mode::Zone | Mode::Head2Head => None,
    }
}
