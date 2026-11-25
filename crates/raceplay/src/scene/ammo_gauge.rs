//! Writes the Detonator cannon gauge into the player's hull's glow table. The law is
//! [`crate::ammo_gauge`]'s.

use super::*;

impl super::Scene {
    /// Gives the player's hull this frame's `StandardColour` and `StandardBullets`.
    pub(super) fn write_ammo_gauge(&self, race: &Race, queue: &Uploads<'_>) {
        let Some(shown) = race.ammo_gauge() else {
            return;
        };
        if let Some(hull) = self.ships.get(race.core.player_slot()) {
            hull.write_program_glow(
                queue,
                oag_mesh::mesh::PROGRAM_BULLET_GAUGE,
                shown.colour,
                shown.fill,
            );
        }
    }
}
