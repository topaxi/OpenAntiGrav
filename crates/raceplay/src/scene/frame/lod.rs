//! The per-frame `LodGroup` switch, run for every drawable that authors one.
//!
//! Its own file for the 1,000-line rule in `scripts/check-file-size.py`, the
//! seam `frame/shadow.rs` already set.

use oag_core::math::Mat4;
use oag_mesh::mesh::LodEye;

use crate::Race;

impl super::Scene {
    /// Chooses this frame's `LodGroup` child on the circuit and on every
    /// craft in play, from the one camera the frame is drawn with - see
    /// `oag_mesh::mesh::LodGroups::child_at` for the rule.
    ///
    /// **Every craft, the player's own included**: `LodGroup_SelectChild`
    /// runs per group per frame with no knowledge of whose hull it is on, so
    /// a far chase camera that puts the player past the switch distance
    /// draws the player's coarse tier too. Nothing here special-cases it.
    ///
    /// The sky, the pads and every weapon model are not switched: none
    /// authors a group on any disc measured, and a drawable nobody switches
    /// keeps its finest tier anyway.
    pub(super) fn select_lod(&self, race: &Race, eye: LodEye) {
        self.track.select_lod(Mat4::IDENTITY, eye);
        for slot in 0..usize::from(race.ship_count()) {
            if !race.ship_active(slot) {
                continue;
            }
            if let Some(drawable) = self.ships.get(slot) {
                drawable.select_lod(race.ship_model_matrix_of(slot), eye);
            }
        }
        self.select_wreck_lod(race, eye);
    }
}
