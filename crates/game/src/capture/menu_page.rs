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
    // Which modal prompt to draw over the page, if any - `--menu-prompt`. See
    // [`prompt_draws`], and the block that calls it for why a prompt cannot
    // otherwise appear on this path at all.
    prompt: Option<&str>,
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
    // The AI PILOTS page's own three sources. Read off the player's real
    // pilot directory rather than invented, for the reason the circuits above
    // are read off the disc: a `PILOT` row showing nothing and one showing
    // this machine's five are different pictures, and looking at the real one
    // is what the flag is for. A machine with no directory yet still gets the
    // four built-ins, which is `Roster::built_in`'s job and not a fallback
    // here.
    let roster = crate::pilots::load().unwrap_or_else(|_| crate::pilots::Roster::built_in());
    model.supply(
        crate::menu::ValueSource::Pilots,
        &roster
            .entries()
            .iter()
            .map(|entry| {
                let label = if entry.from_file {
                    entry.name.clone()
                } else {
                    format!("{} (built-in)", entry.name)
                };
                crate::menu::Choice::labelled(&entry.name, label)
            })
            .collect::<Vec<_>>(),
    );
    model.supply(
        crate::menu::ValueSource::PilotAxes,
        &crate::pilots::AXES
            .iter()
            .map(|(name, _)| crate::menu::Choice::plain(*name))
            .collect::<Vec<_>>(),
    );
    // `LOW`/`HIGH` are resupplied off whichever pilot and axis the other two
    // rows land on, which needs a live session; a still shows the first
    // pilot's first axis, which is what opening the page shows too.
    let bounds = roster
        .entries()
        .first()
        .map(|entry| crate::pilots::AXES[0].1(&entry.pilot));
    for (source, value) in [
        (
            crate::menu::ValueSource::PilotAxisLow,
            bounds.map(|b| b.low),
        ),
        (
            crate::menu::ValueSource::PilotAxisHigh,
            bounds.map(|b| b.high),
        ),
    ] {
        model.supply(
            source,
            &value
                .map(|number| vec![crate::menu::Choice::plain(format!("{number}"))])
                .unwrap_or_default(),
        );
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
    // A modal prompt over the page, when one was asked for.
    //
    // **The only way to look at one headlessly.** `--menu-page` runs no clock
    // and calls no `Menu::update`, so a prompt - which exists precisely
    // because a row was activated - can never appear here by itself. Without
    // this flag the on-screen keyboard's layout is reviewable only by playing
    // the game on a machine with a display, and this project's own rule is to
    // judge a screen by looking at it. The models are the live ones
    // (`crate::prompt`) and the labels come from the same table
    // `session::pilot_editor` resolves, so what this draws is what a player
    // sees rather than a mock-up of it.
    if let Some(kind) = prompt {
        let name = roster
            .entries()
            .first()
            .map_or_else(|| "winston".to_string(), |entry| entry.name.clone());
        let mut list = layers.flatten();
        list.extend(prompt_draws(kind, &name, strings, skin)?);
        return Ok(list);
    }
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

/// One prompt's draws, for `--menu-prompt`.
///
/// **Every label goes through `strings` here too**, which is the half of this
/// that is not merely convenience: the live path resolves them in
/// `session::pilot_editor`, so a capture that spelled its own English would be
/// the one place a translation could silently fail to show. `typed` is the
/// keyboard's seed and `name` the pilot the confirm is about - both taken from
/// the real roster by the caller.
///
/// # Errors
///
/// If `kind` is not one of the prompts this flag knows.
fn prompt_draws(
    kind: &str,
    name: &str,
    strings: &crate::language::StringTable,
    skin: &crate::menu::Skin,
) -> Result<Vec<crate::frontend::Draw>> {
    let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
    let say_of = |id: &str, english: &str| strings.get(id).unwrap_or(english).replace("%s", name);
    match kind {
        "rename" | "rename-note" => {
            let mut keyboard = crate::prompt::Keyboard::new(
                crate::prompt::Labels {
                    title: say("OAG_PILOT_RENAME_TITLE", "RENAME PILOT"),
                    delete: say("OAG_KEYBOARD_DELETE", "DEL"),
                    accept: say("OAG_KEYBOARD_ACCEPT", "OK"),
                    hint: say(
                        "OAG_KEYBOARD_HINT",
                        "CROSS TYPE   SQUARE DELETE   START ACCEPT   CIRCLE CANCEL",
                    ),
                },
                name,
                crate::pilots::MAX_NAME,
            );
            if kind == "rename-note" {
                // The note is the live path's own, set every keystroke from
                // what the text *means*; a still cannot type, so this is the
                // one place it has to be asked for.
                keyboard.set_note(Some(say_of(
                    "OAG_PILOT_RENAME_SHADOWS",
                    "%s IS BUILT IN: A FILE OF THAT NAME REPLACES IT",
                )));
            }
            Ok(keyboard.draw(skin))
        }
        // The longest message either prompt draws, which is exactly why it is
        // worth a flag of its own: the built-in-restore sentence is the whole
        // reason delete asks, and whether it fits is a question for a
        // screenshot rather than for arithmetic.
        "delete" | "delete-built-in" => {
            let message = if kind == "delete-built-in" {
                say_of(
                    "OAG_PILOT_DELETE_BUILT_IN",
                    "%s IS BUILT IN AND YOUR FILE REPLACES IT. \
                     DELETING THE FILE RESTORES THE BUILT-IN INSTEAD OF REMOVING %s.",
                )
            } else {
                say_of("OAG_PILOT_DELETE_ASK", "DELETE %s? THIS CANNOT BE UNDONE.")
            };
            Ok(crate::prompt::Confirm::new(crate::prompt::ConfirmLabels {
                title: say("OAG_PILOT_DELETE_TITLE", "DELETE PILOT"),
                message,
                yes: say("OAG_PILOT_DELETE_YES", "DELETE"),
                no: say("OAG_PILOT_DELETE_NO", "KEEP"),
            })
            .draw(skin))
        }
        other => anyhow::bail!(
            "no prompt named {other:?}; this build has rename, rename-note, delete, delete-built-in"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every other closed vocabulary in this crate is checked at load -
    /// `menu::Action::parse` against `Action::all()`, `ValueSource::parse`,
    /// `Definition::check`. This one is a runtime string match, so it gets the
    /// equivalent here rather than being the one that can only be found by
    /// running the flag and reading the error.
    #[test]
    fn every_prompt_the_flag_names_draws_something_and_an_unknown_one_errors() {
        let skin =
            crate::menu::Skin::new(oag_pulse::FRONT_END.menu, crate::frontend::Space::PSP, 22.0);
        let strings = crate::language::StringTable::default();
        for kind in ["rename", "rename-note", "delete", "delete-built-in"] {
            let list = prompt_draws(kind, "winston", &strings, &skin)
                .unwrap_or_else(|e| panic!("{kind:?} is named by the flag's own help: {e:#}"));
            assert!(!list.is_empty(), "{kind:?} drew nothing");
            // The pilot's name reaches every one of them, which is the whole
            // reason the substitution exists - a confirm that asked about `%s`
            // would be worse than one that asked about nothing.
            assert!(
                list.iter().any(
                    |draw| matches!(draw, crate::frontend::Draw::Text { text, .. }
                        if text.contains("winston"))
                ),
                "{kind:?} does not name the pilot"
            );
            assert!(
                !list.iter().any(
                    |draw| matches!(draw, crate::frontend::Draw::Text { text, .. }
                        if text.contains("%s"))
                ),
                "{kind:?} left a %s unsubstituted"
            );
        }
        let error = prompt_draws("qwerty", "winston", &strings, &skin)
            .expect_err("an unknown prompt has to be an error, not an empty picture");
        assert!(format!("{error:#}").contains("qwerty"));
    }
}
