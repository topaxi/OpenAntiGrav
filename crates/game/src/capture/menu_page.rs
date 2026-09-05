//! [`menu_page`] on its own: `--menu-page`'s picture, split out of
//! `capture.rs` once that file passed the 1,000-line rule -
//! `scripts/check-file-size.py`, which is the rule as a gate.

use anyhow::{Context, Result};
use oag_render::mesh_render::Anisotropy;

/// Draws one page of our own menus, with the source's own lists supplied.
///
/// The lists matter even for a still: a circuit row with nothing in it and one
/// showing this disc's twenty-four circuits are different pictures, and the
/// point of the flag is to look at the real one.
#[allow(
    clippy::too_many_arguments,
    reason = "every one of these is a separate thing the page needs, and a struct \
              for one call site would name the grouping without clarifying it"
)]
pub(super) fn menu_page(
    settings: &crate::settings::Settings,
    anisotropy: Anisotropy,
    // Which title's own `[render_profiles.<title>]` the RENDER SCALE / UPSCALER
    // / SHARPNESS / ANTI-ALIASING / MOTION BLUR rows read - see
    // `crate::settings::menu_seeds`. There is exactly one title open in a
    // capture, the same as in a live session, so this is never a choice.
    title: &str,
    page: &str,
    tracks: &[crate::catalogue::Track],
    teams: &[crate::catalogue::Team],
    languages: &[crate::language::Language],
    strings: &crate::language::StringTable,
    music_discs: &crate::audio::MusicDiscs,
    backdrop: Option<crate::menu::Backdrop>,
    skin: &crate::menu::Skin,
    // The width of a string in the face the entries are drawn in, which a
    // horizontal strip needs and a column does not. The same face `skin`'s line
    // height came off, for the same reason: a strip laid out with the wrong
    // widths overlaps its own entries.
    measure: &dyn Fn(&str) -> f32,
    // The disc's own frame around the page - its clear colour and its rules.
    // Read here rather than left out, because a captured page that is missing
    // the frame the window draws is exactly the divergence this module exists
    // to prevent.
    frame: &crate::menu::Frame,
    phase: Option<f32>,
) -> Result<Vec<crate::frontend::Draw>> {
    let definition = crate::menu::Definition::parse(crate::menu::BUILT_IN, strings)
        .context("parsing the built-in menu definition")?;
    if definition.page(page).is_none() {
        anyhow::bail!(
            "no menu page named {page:?}; this definition has {}",
            definition
                .pages
                .iter()
                .map(|p| format!("{:?}", p.id))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut model = crate::menu::Menu::new(definition);
    model.supply(
        crate::menu::ValueSource::Tracks,
        &tracks
            .iter()
            .map(|track| crate::menu::Choice::labelled(&track.id, strings.get_or_id(&track.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        crate::menu::ValueSource::Teams,
        &teams
            .iter()
            .map(|team| crate::menu::Choice::labelled(&team.id, strings.get_or_id(&team.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        crate::menu::ValueSource::RaceModes,
        &crate::menu::mode_choices(strings),
    );
    model.supply(
        crate::menu::ValueSource::Languages,
        &languages
            .iter()
            .map(|language| crate::menu::Choice::labelled(&language.name, &language.native_name))
            .collect::<Vec<_>>(),
    );
    // No window here, so no screens to enumerate: the list is `default` plus
    // whatever the settings already name. That is enough for the row to draw
    // the player's own value, which is all `--menu-page` is for, and it does
    // not invent a monitor this machine may not have.
    model.supply(
        crate::menu::ValueSource::Monitors,
        &crate::display::Monitor::offered(
            &settings
                .display
                .monitor
                .name()
                .map(ToString::to_string)
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(crate::menu::Choice::plain)
        .collect::<Vec<_>>(),
    );
    // The same treatment, and for a closer reason than it looks: enumerating
    // adapters here would work, but with no surface to be compatible with it
    // would list ones the window's own row would have dropped. A page drawn to
    // show a player their settings should not offer a wider choice than the
    // page they can actually use.
    model.supply(
        crate::menu::ValueSource::Renderers,
        &crate::display::Renderer::offered(
            &settings
                .graphics
                .renderer
                .name()
                .map(ToString::to_string)
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(crate::menu::Choice::plain)
        .collect::<Vec<_>>(),
    );
    // Straight off the boot survey, not off the settings file, and that is
    // the difference from the two rows above: what a player may choose here
    // depends on which discs they own rather than on what they last picked, so
    // a still drawn from the file alone would show a row that is always
    // usable. Empty draws it unusable, which is what most machines would see.
    model.supply(
        crate::menu::ValueSource::MusicSources,
        &if music_discs.both() {
            crate::audio::MusicSource::ALL
                .iter()
                .map(|source| crate::menu::Choice::plain(source.name()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        },
    );
    // Seeded after supplying, and from the same list the live menus use, so
    // what the flag draws is what a player would see rather than whatever each
    // row's list happened to start on.
    //
    // No `Menu::in_effect` to go with it, deliberately: the RENDERER row's
    // restart note is the gap between the settings file and a *running* game,
    // and there is no running game here - this path makes a device of its own to
    // draw one frame with. Supplying the settings value would draw a note that
    // is silent by construction; supplying this capture's adapter would say a
    // player had changed something they have not touched.
    for (key, value) in crate::settings::menu_seeds(settings, anisotropy, title) {
        model.seed(key, &value);
    }
    model.open(page);
    // The same window the live menus use, so a captured page scrolls where a
    // played one does rather than where a default happened to put it.
    model.set_visible_rows(crate::menu::visible_rows(skin));
    // The file's own table, not the built-in default: a settings file that
    // rebound a key should show that key here too. See
    // `crate::settings::Controls::live_bindings`.
    let bindings = settings.controls.live_bindings();
    let layers = crate::menu::draw_list(
        &model,
        skin,
        &|button| bindings.names_for(button),
        measure,
        backdrop,
        frame,
    );
    let Some(phase) = phase else {
        return Ok(layers.flatten());
    };
    // The same arithmetic the live stage runs, through the same easing, so what
    // this draws is a frame of the real transition rather than a picture of one.
    let shape = crate::menu::Transition::default();
    let mut tween = crate::anim::Tween::new(1.0);
    tween.advance(phase.clamp(0.0, 1.0));
    let t = tween.eased();
    Ok(layers
        .zoomed(shape.origin, shape.in_scale + (1.0 - shape.in_scale) * t, t)
        .flatten())
}
