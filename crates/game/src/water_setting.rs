//! The `graphics.water` choice: which values a machine offers, and which law a race draws.
//!
//! A borrowed law (`hd`, `2048`, `omega`) reads the lender's own textures, so
//! it is offered only while that title's disc image is present, and never on
//! the lender itself, where it would be `original` twice
//! (`docs/rendering/water.md`, "The setting").

use oag_display::display::Water;

/// Whether the water row offers `value` on `title`: its law is drawn yet
/// (`oag_mesh::mesh::rcs::water_law_is_drawn`), its lender is available, and `enhanced` only
/// where the title lists the surfaces it is drawn on.
#[must_use]
pub fn offered(value: Water, title: &oag_title::Title, present: &dyn Fn(&str) -> bool) -> bool {
    oag_mesh::mesh::rcs::water_law_is_drawn(value)
        && lender_available(value, title, present)
        && (value != Water::Enhanced || title.water.offers_enhanced())
}

/// Whether the row changes the picture on `title` at all: its own `original` differs from `off`,
/// or a value other than those two is offered. Where nothing does, the row is hidden.
#[must_use]
pub fn row_changes_picture(title: &oag_title::Title, present: &dyn Fn(&str) -> bool) -> bool {
    title.water.original_differs
        || Water::ALL
            .into_iter()
            .filter(|value| !matches!(value, Water::Off | Water::Original))
            .any(|value| offered(value, title, present))
}

/// Whether the lender of `value`'s law is on this machine and is not `title` itself.
/// `present` says whether a title's disc image (by [`oag_title::Title::slug`]) is here.
#[must_use]
pub fn lender_available(
    value: Water,
    title: &oag_title::Title,
    present: &dyn Fn(&str) -> bool,
) -> bool {
    match oag_source::title::water_lender(value) {
        None => true,
        Some(lender) => lender.slug() != title.slug() && present(&lender.slug()),
    }
}

/// The law a race draws: the player's `chosen` value, or `original` when it is
/// not offered here (a disc that was present when it was saved is gone, or it
/// names the title itself).
#[must_use]
pub fn resolve(chosen: Water, title: &oag_title::Title, present: &dyn Fn(&str) -> bool) -> Water {
    if offered(chosen, title, present) {
        return chosen;
    }
    log::warn!(
        "water: {chosen} is not offered on {} here (its disc image is absent or it is this title); drawing original",
        title.name
    );
    Water::Original
}

/// The law a race off `source` draws, for a route that has no menu shell to ask: the title and
/// platform come from opening the source, the choice from that title's profile.
/// `original` when the source will not open (the load reports that itself).
#[must_use]
pub fn for_source(settings: &crate::settings::Settings, source: &str) -> Water {
    let Ok(opened) = oag_source::title::open_source(source, Vec::new(), Vec::new()) else {
        return Water::Original;
    };
    let title = opened.title;
    let chosen = settings
        .render_profiles
        .get(&crate::settings::profile_key(
            title,
            opened.archives.layout.platform,
        ))
        .map_or(Water::Original, |profile| profile.water);
    resolve(chosen, title, &crate::menu_tree::disc_probe())
}

/// Trims the WATER row of `definition` to what [`offered`] allows on `title`, and drops the
/// row where [`row_changes_picture`] says nothing in it would show.
pub fn trim_row(
    definition: &mut oag_ui::menu::Definition,
    title: &oag_title::Title,
    present: &dyn Fn(&str) -> bool,
) {
    definition.drop_unavailable_water(
        &|value| {
            value
                .parse::<Water>()
                .is_ok_and(|water| offered(water, title, present))
        },
        row_changes_picture(title, present),
    );
}
