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
    // `crate::settings::menu_seeds` - and whose front end decides which RACE
    // page rows exist at all. There is exactly one title open in a capture,
    // the same as in a live session, so this is never a choice.
    title: &'static oag_title::Title,
    page: &str,
    tracks: &[crate::catalogue::Track],
    teams: &[crate::catalogue::Team],
    languages: &[oag_ui::language::Language],
    strings: &oag_ui::language::StringTable,
    music_discs: &crate::audio::MusicDiscs,
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    // The width of a string in the face the entries are drawn in, which a
    // horizontal strip needs and a column does not. The same face `skin`'s line
    // height came off, for the same reason: a strip laid out with the wrong
    // widths overlaps its own entries.
    measure: &dyn Fn(&str) -> f32,
    // The disc's own frame around the page - its clear colour and its rules.
    // Read here rather than left out, because a captured page that is missing
    // the frame the window draws is exactly the divergence this module exists
    // to prevent.
    frame: &oag_ui::menu::Frame,
    phase: Option<f32>,
    // Which modal prompt to draw over the page, if any - `--menu-prompt`. See
    // [`prompt_draws`], and the block that calls it for why a prompt cannot
    // otherwise appear on this path at all.
    prompt: Option<&str>,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let mut definition = oag_ui::menu::Definition::parse(oag_ui::menu::BUILT_IN, strings)
        .context("parsing the built-in menu definition")?;
    // The same two trims `crate::prepare` makes before a live menu opens, so
    // a captured RACE page has the rows a player's has.
    definition.drop_unavailable_race_variant(title);
    definition.drop_rows_picked_on_screen(title);
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

    let mut model = oag_ui::menu::Menu::new(definition);
    model.supply(
        oag_ui::menu::ValueSource::Tracks,
        &tracks
            .iter()
            .map(|track| oag_ui::menu::Choice::labelled(&track.id, strings.get_or_id(&track.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::Teams,
        &teams
            .iter()
            .map(|team| oag_ui::menu::Choice::labelled(&team.id, strings.get_or_id(&team.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::RaceModes,
        &oag_ui::menu::mode_choices(strings),
    );
    model.supply(
        oag_ui::menu::ValueSource::Languages,
        &languages
            .iter()
            .map(|language| oag_ui::menu::Choice::labelled(&language.name, &language.native_name))
            .collect::<Vec<_>>(),
    );
    // No window here, so no screens to enumerate: the list is `default` plus
    // whatever the settings already name. That is enough for the row to draw
    // the player's own value, which is all `--menu-page` is for, and it does
    // not invent a monitor this machine may not have.
    model.supply(
        oag_ui::menu::ValueSource::Monitors,
        &oag_display::display::Monitor::offered(
            &settings
                .display
                .monitor
                .name()
                .map(ToString::to_string)
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(oag_ui::menu::Choice::plain)
        .collect::<Vec<_>>(),
    );
    // The same treatment, and for a closer reason than it looks: enumerating
    // adapters here would work, but with no surface to be compatible with it
    // would list ones the window's own row would have dropped. A page drawn to
    // show a player their settings should not offer a wider choice than the
    // page they can actually use.
    model.supply(
        oag_ui::menu::ValueSource::Renderers,
        &oag_display::display::Renderer::offered(
            &settings
                .graphics
                .renderer
                .name()
                .map(ToString::to_string)
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(oag_ui::menu::Choice::plain)
        .collect::<Vec<_>>(),
    );
    // The same list the live menus offer, this machine's own `shaders/`
    // directory included - a still of the GRAPHICS page should show the row a
    // player would actually see.
    model.supply(
        oag_ui::menu::ValueSource::ScreenFilters,
        &crate::screen::Catalogue::load(crate::screen::Catalogue::directory()).choices(),
    );
    // Straight off the boot survey, not off the settings file, and that is
    // the difference from the two rows above: what a player may choose here
    // depends on which discs they own rather than on what they last picked, so
    // a still drawn from the file alone would show a row that is always
    // usable. Empty draws it unusable, which is what most machines would see.
    model.supply(
        oag_ui::menu::ValueSource::MusicSources,
        &if music_discs.both() {
            crate::audio::MusicSource::ALL
                .iter()
                .map(|source| oag_ui::menu::Choice::plain(source.name()))
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
    for (key, value) in crate::settings::menu_seeds(settings, anisotropy, title.name) {
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
        oag_ui::menu::ValueSource::Pilots,
        &roster
            .entries()
            .iter()
            .map(|entry| {
                let label = if entry.from_file {
                    entry.name.clone()
                } else {
                    format!("{} (built-in)", entry.name)
                };
                oag_ui::menu::Choice::labelled(&entry.name, label)
            })
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::PilotAxes,
        &crate::pilots::AXES
            .iter()
            .map(|(name, _)| oag_ui::menu::Choice::plain(*name))
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
            oag_ui::menu::ValueSource::PilotAxisLow,
            bounds.map(|b| b.low),
        ),
        (
            oag_ui::menu::ValueSource::PilotAxisHigh,
            bounds.map(|b| b.high),
        ),
    ] {
        model.supply(
            source,
            &value
                .map(|number| vec![oag_ui::menu::Choice::plain(format!("{number}"))])
                .unwrap_or_default(),
        );
    }
    model.open(page);
    // A still shows the page at rest, not the tick it arrived on - see
    // `Menu::settle` for what the arrival tick would otherwise draw.
    model.settle();
    // The same window the live menus use, so a captured page scrolls where a
    // played one does rather than where a default happened to put it - the
    // AI PILOTS page's own reservation included, off the page just opened
    // rather than a live `AXIS` value. See `pilots::page_reserves_axis_preview`.
    let reserve_note = crate::pilots::page_reserves_axis_preview(model.page());
    model.set_visible_rows(oag_ui::menu::visible_rows(skin, frame, reserve_note));
    // The file's own table, not the built-in default: a settings file that
    // rebound a key should show that key here too. See
    // `crate::settings::Controls::live_bindings`.
    let bindings = settings.controls.live_bindings();
    let layers = oag_ui::menu::draw_list(
        &model,
        skin,
        &|button| bindings.names_for(button),
        measure,
        backdrop,
        frame,
        // `--menu-page` builds this `Menu` from scratch with no `Session`
        // behind it, so there is never a parked race to show through - see
        // `oag_ui::menu::draw_list`'s own doc for the parameter.
        false,
    );
    // What the `AXIS` row currently means, read live off `model`'s own rows -
    // `None` off any page but AI PILOTS. The one function the live session
    // draws this line through too; see `crate::pilots::axis_preview_for`'s
    // own doc.
    let axis_preview = crate::pilots::axis_preview_for(&model, Some(strings));

    // A modal prompt over the page, when one was asked for.
    //
    // **The only way to look at one headlessly.** `--menu-page` runs no clock
    // and calls no `Menu::update`, so a prompt - which exists precisely
    // because a row was activated - can never appear here by itself. Without
    // this flag the on-screen keyboard's layout is reviewable only by playing
    // the game on a machine with a display, and this project's own rule is to
    // judge a screen by looking at it. The models are the live ones
    // (`oag_ui::prompt`) and the labels come from the same lookup the live
    // path resolves them with (`session::pilot_editor` for rename/delete,
    // `crate::rebind::prompt` for `binding`), so what this draws is what a
    // player sees rather than a mock-up of it.
    if let Some(kind) = prompt {
        // `binding` names a button, not a pilot: the CONTROLS page's own
        // key-capture prompt names whichever button a `binding` row's
        // confirm press would be capturing for, off the page this capture
        // already opened - the first one on it, if there is one, since a
        // still has no selected row to prefer over another. `CIRCLE` covers
        // a page with none, which the built-in CONTROLS page never is but a
        // `--menu` override in principle could be.
        let name = if kind == "binding" {
            model
                .page()
                .entries
                .iter()
                .find_map(|entry| match entry {
                    oag_ui::menu::Entry::Binding { button, .. } => {
                        Some(button.to_string().to_ascii_uppercase())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "CIRCLE".to_string())
        } else {
            roster
                .entries()
                .first()
                .map_or_else(|| "winston".to_string(), |entry| entry.name.clone())
        };
        let mut list = layers.flatten();
        if let Some(text) = &axis_preview {
            list.push(oag_ui::prompt::axis_preview_draw(&model, skin, text));
        }
        list.extend(prompt_draws(kind, &name, strings, skin)?);
        return Ok(list);
    }
    let Some(phase) = phase else {
        let mut list = layers.flatten();
        if let Some(text) = &axis_preview {
            list.push(oag_ui::prompt::axis_preview_draw(&model, skin, text));
        }
        return Ok(list);
    };
    // The same arithmetic the live stage runs, through the same easing, so what
    // this draws is a frame of the real transition rather than a picture of one.
    // No axis-preview line here, matching `MenuStage::render`'s own gate:
    // the row list is a zoomed, mid-tween picture at this point, and the
    // preview's position is computed against the still one.
    let shape = oag_ui::menu::Transition::default();
    let mut tween = oag_ui::anim::Tween::new(1.0);
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
/// `session::pilot_editor` for the pilot-editor prompts and in
/// `crate::rebind::prompt` for `binding` (called from `session::draw`, which
/// has no pure lookup of its own to duplicate), so a capture that spelled its
/// own English would be the one place a translation could silently fail to
/// show. `name` is the pilot a rename or delete confirm is about, or the
/// button a `binding` capture is waiting on - both taken from the real data
/// by the caller rather than invented here.
///
/// # Errors
///
/// If `kind` is not one of the prompts this flag knows.
fn prompt_draws(
    kind: &str,
    name: &str,
    strings: &oag_ui::language::StringTable,
    skin: &oag_ui::menu::Skin,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
    let say_of = |id: &str, english: &str| strings.get(id).unwrap_or(english).replace("%s", name);
    match kind {
        "rename" | "rename-note" => {
            let mut keyboard = oag_ui::prompt::Keyboard::new(
                oag_ui::prompt::Labels {
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
            Ok(oag_ui::prompt::Confirm::new(oag_ui::prompt::ConfirmLabels {
                title: say("OAG_PILOT_DELETE_TITLE", "DELETE PILOT"),
                message,
                yes: say("OAG_PILOT_DELETE_YES", "DELETE"),
                no: say("OAG_PILOT_DELETE_NO", "KEEP"),
            })
            .draw(skin))
        }
        // The CONTROLS page's key-capture prompt - `oag_ui::prompt::
        // message_draw` is the same function `MenuStage::render` calls in
        // the binary, off `Session::awaiting_binding`, so this and a live
        // capture cannot draw two different pictures for the same state.
        // See `docs/architecture/menus.md`'s Rebinding section.
        "binding" => Ok(oag_ui::prompt::message_draw(
            skin,
            &say_of(
                "OAG_BINDING_CAPTURE_PROMPT",
                "PRESS A KEY FOR %s - ESCAPE CANCELS",
            ),
        )),
        other => anyhow::bail!(
            "no prompt named {other:?}; this build has rename, rename-note, delete, \
             delete-built-in, binding"
        ),
    }
}

/// What `--menu-page track-select`/`ship-select` still owes the frame once
/// its draw list is rendered: the preview mesh, which is a 3D pass rather
/// than a draw - see [`crate::preview`].
pub(super) struct PreviewRequest {
    /// The mesh's archive entry name.
    pub entry: String,
    /// The skin `.dat` to paint it in, when the settings name one the team
    /// declares.
    pub skin: Option<String>,
    /// Where it goes, in the screen's grid - [`oag_ui::picker::Layout::preview`].
    pub rect: [f32; 4],
    pub kind: oag_ui::picker::Kind,
}

/// Which selection screen a `--menu-page` name asks for, if either.
#[must_use]
pub(super) fn picker_kind(page: &str) -> Option<oag_ui::picker::Kind> {
    match page {
        "track-select" | "track_select" => Some(oag_ui::picker::Kind::Track),
        "ship-select" | "ship_select" => Some(oag_ui::picker::Kind::Ship),
        _ => None,
    }
}

/// Draws one of the race box's selection screens on the settings' own
/// selection, the same way the live session opens it - see
/// `session::picker` for the flow this is the still of.
///
/// Every list is the source's: the circuits and teams off its definition,
/// the labels off its string table, the ratings off each team's own `<FE>`
/// element. The livery axis is the title's own variant table, which is
/// empty on Pulse and draws no row there.
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument menu_page makes: each is a separate thing the page needs"
)]
pub(super) fn picker_page(
    kind: oag_ui::picker::Kind,
    layout: &oag_ui::picker::Layout,
    settings: &crate::settings::Settings,
    title: &'static oag_title::Title,
    tracks: &[crate::catalogue::Track],
    teams: &[crate::catalogue::Team],
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    sprites: &crate::sprite::Sheet,
    measure: &dyn Fn(&str) -> f32,
    // The selected circuit's lap length, measured by the caller off the
    // disc - `None` draws the dash an unmeasured one draws live.
    distance: Option<f32>,
) -> (Vec<oag_ui::frontend::Draw>, Option<PreviewRequest>) {
    use oag_ui::picker::{Details, Entry, Kind, Picker};
    let (entries, previews): (Vec<Entry>, Vec<String>) = match kind {
        Kind::Track => tracks
            .iter()
            .map(|track| {
                (
                    Entry {
                        id: track.id.clone(),
                        label: strings.get_or_id(&track.id).to_string(),
                        details: Details::Track {
                            // A capture keeps no records store; the
                            // distance is the caller's, for the selected
                            // circuit only.
                            info: [
                                if track.id == settings.race.track {
                                    distance.map_or_else(|| "-".to_string(), |d| format!("{d:.0}"))
                                } else {
                                    "-".to_string()
                                },
                                "-".into(),
                                "-".into(),
                            ],
                        },
                    },
                    format!(
                        r"{}\FE\{}.vex",
                        track.location,
                        if track.reversed { "reverse" } else { "forward" }
                    ),
                )
            })
            .unzip(),
        Kind::Ship => teams
            .iter()
            .map(|team| {
                // The same axis rule the live screen applies - see
                // `session::picker::open_ship_picker`: the team's own skins
                // where it declares any, the title's variants otherwise.
                let variants: Vec<(String, String)> =
                    if !team.skins.is_empty() {
                        std::iter::once((
                            String::new(),
                            strings
                                .get_or_id(crate::catalogue::BASELINE_SKIN)
                                .to_string(),
                        ))
                        .chain(team.skins.iter().map(|skin| {
                            (skin.name.clone(), strings.get_or_id(&skin.name).to_string())
                        }))
                        .collect()
                    } else if let Some(team_variants) = title.race.team_variants_for(&team.id) {
                        team_variants
                            .variants
                            .iter()
                            .map(|variant| (variant.suffix.to_string(), variant.label.to_string()))
                            .collect()
                    } else {
                        title
                            .race
                            .hull_variants
                            .into_iter()
                            .flatten()
                            .map(|variant| (variant.stem.to_string(), variant.label.to_string()))
                            .collect()
                    };
                (
                    Entry {
                        id: team.id.clone(),
                        label: team.label(strings).to_string(),
                        details: Details::Ship {
                            rating: team.rating.map(|rating| oag_ui::picker::Rating {
                                speed: rating.speed,
                                thrust: rating.thrust,
                                handling: rating.handling,
                                shield: rating.shield,
                            }),
                            variants,
                        },
                    },
                    format!(r"{}\ship_FE.vex", team.location),
                )
            })
            .unzip(),
    };
    let skinned = teams
        .iter()
        .find(|team| team.id == settings.race.team)
        .is_some_and(|team| !team.skins.is_empty());
    let (selected, variant) = match kind {
        Kind::Track => (settings.race.track.as_str(), None),
        Kind::Ship if skinned => (
            settings.race.team.as_str(),
            Some(settings.race.skin.as_str()),
        ),
        Kind::Ship => (
            settings.race.team.as_str(),
            Some(settings.race.variant.as_str()),
        ),
    };
    let picker = Picker::new(kind, entries, Some(selected), variant);
    let skin_entry = match kind {
        Kind::Ship => teams
            .get(picker.index())
            .and_then(|team| team.skin(&settings.race.skin))
            .map(|skin| skin.location.clone()),
        Kind::Track => None,
    };
    let request = previews.get(picker.index()).map(|entry| PreviewRequest {
        entry: entry.clone(),
        skin: skin_entry,
        rect: layout.preview,
        kind,
    });
    let layers = oag_ui::picker::draw_list(
        &picker,
        layout,
        skin,
        frame,
        backdrop,
        false,
        &|src| sprites.get(src),
        measure,
    );
    (layers.flatten(), request)
}

/// Which Race Campaign screen a `--menu-page` name asks for, if either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CampaignKind {
    Grid,
    Cell,
}

#[must_use]
pub(super) fn campaign_kind(page: &str) -> Option<CampaignKind> {
    match page {
        "grid-select" | "grid_select" => Some(CampaignKind::Grid),
        "cell-select" | "cell_select" => Some(CampaignKind::Cell),
        _ => None,
    }
}

/// Draws `Grid Selection` on its first tier, or `Cell Selection` on that
/// tier's own cells - the same shape [`picker_page`] draws the race box's
/// two screens in, minus a preview mesh: neither campaign screen authors
/// one. See `oag_game::campaign` for the read this is a still of.
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument menu_page and picker_page both make: each is a separate fact \
              the page needs"
)]
pub(super) fn campaign_page(
    kind: CampaignKind,
    archives: &mut oag_assets::Archives,
    strings: &oag_ui::language::StringTable,
    faces: oag_ui::picker::FaceScales,
    grid: [f32; 2],
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    // `&mut`, extended in place with the hex textures neither screen's XML
    // shares with `Skin.xml` - the same `*sprites = sprites.extended(...)`
    // idiom `picker_stills` already uses, so the sheet `Renderer::new` builds
    // from further down `run` is the one these two textures actually landed
    // on.
    sprites: &mut crate::sprite::Sheet,
    // The front-end root's own `FEGlobals` - see `crate::campaign::load`'s
    // own doc for why a still needs this too, not only the live session.
    fallback_globals: &[(&str, &str)],
    title: &'static oag_title::Title,
    // The `Confirm`/`Back` fit-to-gap shrink's own text-width function.
    measure: &dyn Fn(&str) -> f32,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let campaign = crate::campaign::load(
        archives,
        strings,
        faces,
        grid,
        sprites,
        fallback_globals,
        title,
    )
    .context("this source has no Race Campaign to show")?;
    // No per-session tip rotation, the same gap `circuit_names` below has -
    // see `crate::campaign::static_footer_overlay`. Read before
    // `campaign.sprites` moves out below.
    let footer_overlay = crate::campaign::static_footer_overlay(&campaign, &faces, measure);
    *sprites = campaign.sprites;
    let is_hd = title.name == oag_hd::TITLE.name;
    // **HD's own `Track Line` fold is not built here.** `CircuitNames::choose`
    // needs every copy of the track-name table across the source's own
    // archives, which this still has no ready list of - see
    // `oag_ui::campaign::hd::hd_track_line`'s own doc for the plain fallback
    // this leaves a captured HD `Cell Selection` with (the raw track id, the
    // same gap `picker_page`'s own RACE-page capture already has). The live
    // session's `crate::main::session::campaign::open_campaign` does carry
    // one (`Shell::circuit_names`) and is the path that matters for a
    // player.
    let circuit_names = oag_ui::language::CircuitNames::default();
    let layers = if is_hd {
        match kind {
            CampaignKind::Grid => {
                let model = oag_ui::campaign::GridSelection::new(
                    campaign
                        .grids
                        .iter()
                        .map(oag_ui::campaign::GridSummary::from_grid)
                        .collect(),
                );
                oag_ui::campaign::hd::hd_grid_draw_list(
                    &model,
                    &campaign.grid_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                )
            }
            CampaignKind::Cell => {
                // `crate::campaign::load` already refuses an empty grid
                // list (`"no grid in ... parsed"`), so `first()` is `None`
                // only if that changes; the fallback below is a zeroed
                // summary rather than a panic, on the same "a still draws
                // something honest rather than crashing" terms the rest of
                // this module follows.
                let grid = campaign.grids.first();
                let cells = grid.map(|grid| grid.cells.clone()).unwrap_or_default();
                let model = oag_ui::campaign::CellSelection::new(cells);
                let grid_summary = grid.map_or(
                    oag_ui::campaign::GridSummary {
                        name: String::new(),
                        cell_count: 0,
                        max_points: 0,
                        required_points: 0,
                        gold_medals: 0,
                        points_earned: 0,
                        locked: false,
                    },
                    oag_ui::campaign::GridSummary::from_grid,
                );
                oag_ui::campaign::hd::hd_cell_draw_list(
                    &model,
                    &campaign.cell_layout,
                    skin,
                    frame,
                    strings,
                    &circuit_names,
                    0,
                    campaign.grids.len().max(1),
                    &grid_summary,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                )
            }
        }
    } else {
        match kind {
            CampaignKind::Grid => {
                let model = oag_ui::campaign::GridSelection::new(
                    campaign
                        .grids
                        .iter()
                        .map(oag_ui::campaign::GridSummary::from_grid)
                        .collect(),
                );
                oag_ui::campaign::grid_draw_list(
                    &model,
                    &campaign.grid_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
            CampaignKind::Cell => {
                // The first grid `Definition.xml` lists - `--menu-page` has
                // no way to name a tier, and a still needs something to
                // show.
                let cells = campaign
                    .grids
                    .first()
                    .map(|grid| grid.cells.clone())
                    .unwrap_or_default();
                let model = oag_ui::campaign::CellSelection::new(cells);
                oag_ui::campaign::cell_draw_list(
                    &model,
                    &campaign.cell_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
        }
    };
    Ok(layers.flatten())
}

/// The source a capture's race options name, opened with its packs, for the
/// selection screens' own reads.
pub(super) fn open_for_previews(race: &crate::race::Options) -> Result<oag_assets::Archives> {
    let (packs, pure_packs, problems) =
        crate::dlc::packs_from_defaults(&race.dlc, &crate::boot::default_dlc_cache_dir());
    for problem in problems {
        log::warn!("{problem}");
    }
    Ok(crate::title::open_source(&race.source, packs, pure_packs)?.archives)
}

/// The selection screen's preview mesh, over the finished draw list - the
/// same pass `MenuStage::render` runs live, at the screen's first tick.
///
/// A mesh that will not read or build is logged and draws nothing, per the
/// rule for an asset that will not play; the rest of the capture stands.
#[allow(
    clippy::too_many_arguments,
    reason = "one call site, each a separate fact of the frame"
)]
pub(super) fn draw_preview(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    viewport: (f32, f32, f32, f32),
    target_size: (u32, u32),
    space: oag_display::space::Space,
    race: &crate::race::Options,
    request: &PreviewRequest,
    anisotropy: Anisotropy,
) {
    let built = open_for_previews(race).and_then(|mut archives| {
        let mut model = crate::preview::model(&mut archives, &request.entry)?;
        // The chosen paint, the same swap the live screen and a race make;
        // a skin that will not read leaves the hull's own and says so.
        if let Some(entry) = &request.skin {
            match archives
                .read_name(entry)
                .map_err(anyhow::Error::from)
                .and_then(|blob| oag_texture::ship_skin::parse(&blob).map_err(anyhow::Error::from))
            {
                Ok(paint) => {
                    oag_render::mesh::ship_skin::apply(&mut model, &paint);
                }
                Err(error) => {
                    log::warn!("{entry}: {error:#} - the preview keeps the hull's own paint");
                }
            }
        }
        crate::preview::Preview::new(
            device,
            queue,
            wgpu::TextureFormat::Rgba8Unorm,
            anisotropy,
            model,
        )
    });
    match built {
        Ok(mut preview) => preview.draw(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            request.rect,
            crate::preview::orbit_for(request.kind, 0.0),
            0.0,
        ),
        Err(error) => log::warn!("{}: {error:#} - the preview draws nothing", request.entry),
    }
}

/// The selected entry's stills, on the front end's sheet extended the way the
/// live screen extends it, at the moment the screen opens: the first card
/// alone.
///
/// **A team's as well as a circuit's**, or a capture of Pure's `Team
/// Selection` would show an empty panel where the live screen shows the
/// craft. `screens` supplies the `FEGlobals` table the per-entity
/// `screen.xml` tints its stills through. See `oag_game::preview::slideshow`.
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument picker_page makes: each is a separate thing the still needs \
              - which screen, the settings, the two rosters, the disc, the globals and the sheet"
)]
pub(super) fn picker_stills(
    kind: oag_ui::picker::Kind,
    settings: &crate::settings::Settings,
    track: Option<&crate::catalogue::Track>,
    tracks: &[crate::catalogue::Track],
    teams: &[crate::catalogue::Team],
    archives: Option<&mut oag_assets::Archives>,
    screens: &oag_ui::screen::Screens,
    sprites: &mut crate::sprite::Sheet,
) -> Vec<oag_ui::frontend::Draw> {
    // The entry the *picker* selects, which falls back to the first when the
    // setting names none this source offers - `Picker::new`'s own rule. A
    // second lookup that stopped at `None` instead would leave a capture of
    // another title's disc showing a highlighted row with no picture beside
    // it, which is a disagreement with the live screen and not a finding.
    let source = match kind {
        oag_ui::picker::Kind::Track => track.or(tracks.first()).map(|track| {
            (
                track.location.clone(),
                oag_race::Mode::from_name(&settings.race.mode) == Some(oag_race::Mode::Zone),
            )
        }),
        oag_ui::picker::Kind::Ship => teams
            .iter()
            .find(|team| team.id == settings.race.team)
            .or(teams.first())
            .map(|team| (team.location.clone(), false)),
    };
    let (Some(archives), Some((location, zone))) = (archives, source) else {
        return Vec::new();
    };
    let globals: Vec<(&str, &str)> = screens
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let mut report = Vec::new();
    let stills = match crate::preview::slideshow(archives, &location, zone, &globals, &mut report) {
        Ok((show, blobs)) => {
            *sprites = sprites.extended(&blobs, &mut report);
            show.draws(0.0, &|src| sprites.get(src))
        }
        Err(error) => {
            log::info!("{error:#} - {location} shows no stills");
            Vec::new()
        }
    };
    for line in report {
        log::info!("{line}");
    }
    stills
}

/// The Fury backdrop's frame for a captured page, on a source that has one:
/// the same model the live stage ticks, run `anim_seconds` in at sixty frames
/// a second, sized to the viewport and tinted for the page - the root is the
/// disc's `Main Menu`. One frame, so the trail the live picture accumulates
/// is not here; the fresh particles are.
pub(super) fn fury_picture(
    assets: Option<&crate::boot::fury::FuryAssets>,
    page: &str,
    viewport: (f32, f32, f32, f32),
    anim_seconds: Option<f32>,
    fury_path: Option<usize>,
) -> Option<oag_ui::menu::Picture> {
    let assets = assets?;
    let mut model = oag_ui::backdrop::Fury::new(
        assets.settings.clone(),
        assets.first,
        crate::boot::fury::SEED,
    )?;
    if let Some(path) = fury_path {
        model.force_path(path)?;
    }
    let frames =
        (anim_seconds.unwrap_or(0.0).max(0.0) * oag_ui::backdrop::FRAMES_PER_SECOND) as u32;
    for _ in 0..frames {
        model.tick();
    }
    let (_, _, w, h) = viewport;
    Some(oag_ui::menu::Picture::from(model.frame(
        h,
        w / h,
        assets.tints.for_root(page == "main"),
    )))
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
        let skin = oag_ui::menu::Skin::new(
            oag_pulse::FRONT_END.menu.unwrap(),
            oag_display::space::Space::PSP,
            22.0,
        );
        let strings = oag_ui::language::StringTable::default();
        for kind in [
            "rename",
            "rename-note",
            "delete",
            "delete-built-in",
            "binding",
        ] {
            let list = prompt_draws(kind, "winston", &strings, &skin)
                .unwrap_or_else(|e| panic!("{kind:?} is named by the flag's own help: {e:#}"));
            assert!(!list.is_empty(), "{kind:?} drew nothing");
            // `name` reaches every one of them, which is the whole reason the
            // substitution exists - a prompt that asked about `%s` would be
            // worse than one that asked about nothing. The test passes
            // `"winston"` for `binding` too, standing in for a button name
            // here the same way it stands in for a pilot's elsewhere: this
            // checks the substitution mechanism, not what `menu_page`'s own
            // caller derives it from.
            assert!(
                list.iter().any(
                    |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. }
                        if text.contains("winston"))
                ),
                "{kind:?} does not substitute name"
            );
            assert!(
                !list.iter().any(
                    |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. }
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
