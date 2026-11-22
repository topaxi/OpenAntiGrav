//! Writes each laid Detonator mine's `MineColour` and `MineBrightness` into its pool slots'
//! glow tables. The values and the rules are [`crate::detonator_look`]'s.

use super::*;
use crate::detonator_look::{MinePart, bomb_constants, mine_constants};

impl super::Scene {
    /// Gives every laid mine's football and spikes this frame's constants: the stage's colour
    /// and the music's envelope for a live mine, the dark literals for a dark one.
    ///
    /// The band levels are the frame's raw sixteen ([`Scene::set_band_levels`]), `seconds` the
    /// race clock. A race with no stage colours (no table, or a stage that authors none) leaves
    /// the entries at the material's authored defaults.
    pub(super) fn write_detonator_looks(
        &self,
        race: &Race,
        queue: &Uploads<'_>,
        seconds: f32,
        bombs: usize,
    ) {
        let Some(stage) = self
            .zone_grade
            .as_ref()
            .and_then(crate::zone_grade::ZoneGrade::detonator_colours)
        else {
            return;
        };
        let parts = &self.bomb_blast.detonator;
        // The flash's writer is unread: a bomb at rest (`detonator_look::bomb_constants`).
        let bomb = bomb_constants(&stage, 0.0);
        for (pool, tint, brightness) in [
            (&self.bombs, bomb.inner, bomb.body_brightness),
            (
                &parts.bomb_equalisers,
                bomb.outer,
                bomb.equaliser_brightness,
            ),
        ] {
            for drawable in pool.iter().take(bombs) {
                drawable.write_program_glow(
                    queue,
                    oag_mesh::mesh::PROGRAM_BOMB_TINT,
                    tint,
                    brightness,
                );
            }
        }
        let (ordinary, laid) = race.detonator_mine_looks();
        if laid.is_empty() {
            return;
        }
        let envelopes = crate::drawable::lock(&parts.envelopes).advance(
            &laid,
            &self.band_levels.get(),
            seconds,
        );
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
