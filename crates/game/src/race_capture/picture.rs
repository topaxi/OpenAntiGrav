//! `--picture`: the picture controls of a shot drawn again (`docs/formats/shot-png.md`).

use oag_raceplay::Race;
use oag_raceplay::photo::FixedPicture;

use crate::shot_meta::Context;

/// Draws `race` with the controls `context` names, as photo mode's panel held them: the same
/// pass, over the finished frame, with the speed blur's ring where the title has one.
pub(super) fn apply(race: &mut Race, context: &Context) {
    if let Some(picture) = &context.picture {
        race.set_picture(FixedPicture::new(
            picture.effects(),
            Some(oag_source::title::borrowed_zoom_ring()),
        ));
    }
}
