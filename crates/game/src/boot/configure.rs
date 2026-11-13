//! What the boot hands a freshly built `Frontend` beyond its screens.

use oag_ui::frontend::{ExtraTile, Frontend, MapEvent, TeamRating, Tier};

/// Sets the skin, the face scales (only where they were measured - see
/// `load_shell`'s own gate), the picker's row pitch and the 2048 tables.
pub(super) fn apply(
    frontend: &mut Frontend,
    menu_skin: &'static oag_title::MenuSkin,
    (face_scales, line_height, picker_from_loaded_faces): (&[(String, f32)], f32, bool),
    (campaign_events, extra_tiles, tiers, team_ratings): (
        Vec<MapEvent>,
        Vec<ExtraTile>,
        Vec<Tier>,
        Vec<TeamRating>,
    ),
) {
    frontend.set_menu_skin(menu_skin);
    if !face_scales.is_empty() {
        frontend.set_face_scales(face_scales.to_vec(), line_height);
    }
    if picker_from_loaded_faces {
        frontend.set_picker_line_height(line_height);
    }
    if !campaign_events.is_empty() {
        frontend.set_campaign(campaign_events);
    }
    if !extra_tiles.is_empty() {
        frontend.set_extra_tiles(extra_tiles);
    }
    frontend.set_tiers(tiers);
    frontend.set_team_ratings(team_ratings);
}
