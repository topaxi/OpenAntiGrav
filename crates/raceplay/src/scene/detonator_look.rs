//! Writes each laid Detonator mine's `MineColour` and `MineBrightness` into its pool slots'
//! glow tables. The values and the rules are [`crate::detonator_look`]'s.

use super::*;
use crate::detonator_look::{MinePart, mine_constants};

impl super::Scene {
    /// Gives every laid mine's football and spikes this frame's constants: the stage's colour
    /// and the music's envelope for a live mine, the dark literals for a dark one.
    ///
    /// `levels` are the sixteen band levels of the frame (`oag_audio::spectrum`), `seconds` the
    /// race clock. A race with no stage colours (no table, or a stage that authors none) leaves
    /// the entries at the material's authored defaults.
    pub(super) fn write_detonator_looks(
        &self,
        race: &Race,
        queue: &Uploads<'_>,
        levels: &[f32],
        seconds: f32,
    ) {
        let Some(stage) = self
            .zone_grade
            .as_ref()
            .and_then(crate::zone_grade::ZoneGrade::detonator_colours)
        else {
            return;
        };
        let (ordinary, laid) = race.detonator_mine_looks();
        if laid.is_empty() {
            return;
        }
        let parts = &self.bomb_blast.detonator;
        let envelopes = crate::drawable::lock(&parts.envelopes).advance(&laid, levels, seconds);
        for (index, (mine, envelope)) in laid.iter().zip(&envelopes).enumerate() {
            for (pool, part) in [
                (&self.mines, MinePart::Football),
                (&parts.mine_spikes, MinePart::Spikes),
            ] {
                let Some(drawable) = pool.get(ordinary + index) else {
                    continue;
                };
                let constants = mine_constants(mine.live, part, &stage, envelope);
                drawable.write_program_glow(
                    queue,
                    oag_mesh::mesh::PROGRAM_MINE_RIM,
                    constants.colour,
                    constants.brightness,
                );
            }
        }
    }
}
