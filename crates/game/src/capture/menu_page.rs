//! [`menu_page`] on its own: `--menu-page`'s picture, split out of
//! `capture.rs` once that file passed the 1,000-line rule -
//! `scripts/check-file-size.py`, which is the rule as a gate.

use anyhow::{Context, Result};
use oag_mesh::mesh_render::Anisotropy;

/// A draw's clip: its index in the flattened list, plus `(left, right)` in
/// screen space - what [`menu_page`] returns for the footer ticker and
/// `capture::run`'s own call site hands `Renderer::render` unchanged. Named
/// so the return type below reads rather than counting parentheses.
type TickerClip = Option<(usize, f32, f32)>;

/// What `--menu-picker-seconds` defaults to when absent: past every
/// `LeftLayer transition` the race box's two selection screens author
/// (measured at up to `0.5`s, `docs/ui/selection-screens.md`), so the
/// default capture reads exactly as it did before that flag existed.
const SETTLED_SECONDS: f32 = 1.0;

/// Draws one page of our own menus, with the source's own lists supplied.
///
/// The lists matter even for a still: a circuit row with nothing in it and one
/// showing this disc's twenty-four circuits are different pictures, and the
/// point of the flag is to look at the real one.
///
/// Returns the draw list alongside the footer ticker's own clip, when one is
/// on screen: `(index in the returned list, left, right)` in screen space -
/// what `capture::run`'s own call site hands `Renderer::render` so the
/// ticker's text is cut at its viewport's edges instead of overdrawing the
/// nav legend and running off the frame. Found by equality against the
/// ticker's own draw after the page's layers are flattened, the same
/// `position`-after-`flatten` idiom `MenuStage::render`'s own call site
/// uses for the Race Campaign's grid/cell screens - not a hand-tracked
/// index, which `flatten` would have been free to invalidate. `None` when
/// this page draws no ticker at all: no layout, no honest tip to show, or a
/// mid-transition capture, which skips the whole footer the way
/// `MenuStage::render`'s own gate does.
#[allow(
    clippy::too_many_arguments,
    reason = "every one of these is a separate thing the page needs, and a struct \
              for one call site would name the grouping without clarifying it"
)]
pub(super) fn menu_page(
    settings: &crate::settings::Settings,
    anisotropy: Anisotropy,
    // Which title's own `[render_profiles.<key>]` the RENDER SCALE / UPSCALER
    // / SHARPNESS / ANTI-ALIASING / MOTION BLUR rows read - see
    // `crate::settings::menu_seeds` - and whose front end decides which RACE
    // page rows exist at all. There is exactly one title open in a capture,
    // the same as in a live session, so this is never a choice.
    title: &'static oag_title::Title,
    // The other half of that same row's key - see `crate::settings::profile_key`.
    platform: oag_disc::Platform,
    page: &str,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    race_setup: &crate::boot::RaceSetup,
    languages: &[oag_ui::language::Language],
    strings: &oag_ui::language::StringTable,
    music_discs: &oag_sound::MusicDiscs,
    // The RECORDS page's own store, read the same read-only way
    // `crate::race_capture::CaptureOptions::previous_best` is - see the call site in
    // `capture.rs`. Taken rather than loaded in here so this function stays
    // testable on a hand-built `Store` with no real `<config dir>` involved.
    records: &crate::records::Store,
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
    // The disc image path, `capture::Options::race`'s own `source` - `None`
    // on the `--menu-page` leg with no race hand-off behind it at all.
    // `prompt_draws`' only use is `tag-entry`/`tag-entry-typed`, which reads
    // Pulse's own `TagInput` screens live off it - every other prompt kind
    // ignores this.
    source: Option<&str>,
    // The front-end root's own `Confirm`/`Back` legend - `None` on a source
    // whose root authors no `NavigationController` this build reads. See
    // `crate::main::menu_stage::MenuStage::render`'s own call site for the
    // identical draw this mirrors.
    nav_legend: Option<&oag_ui_screens::campaign::footer::NavigationLegend>,
    // The `Default`-role face `nav_legend`'s own word half draws through.
    default_measure: &dyn Fn(&str) -> f32,
    // The footer's own scrolling tip ticker layout - `None` on a source
    // whose front-end root authors no `TextInfoIsAlwaysLast` viewport
    // (every title but Pulse today). See
    // `crate::main::menu_stage::MenuStage::render`'s own call site for the
    // identical draw this mirrors, at a frozen `elapsed` of `0.0` since a
    // still has no clock of its own to animate the scroll with.
    ticker: Option<&oag_ui_screens::campaign::footer::TickerLayout>,
    // This source's own honest tip rotation, off `records` - see
    // `crate::records::ticker_tips`'s own doc.
    ticker_tips: &[String],
) -> Result<(Vec<oag_ui::frontend::Draw>, TickerClip)> {
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
    race_sources::supply(&mut model, title, tracks, teams, race_setup, strings);
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
            oag_sound::MusicSource::ALL
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
    for (key, value) in crate::settings::menu_seeds(settings, anisotropy, title, platform) {
        model.seed(key, &value);
    }
    // The AI PILOTS page's own three sources. Read off the player's real
    // pilot directory rather than invented, for the reason the circuits above
    // are read off the disc: a `PILOT` row showing nothing and one showing
    // this machine's five are different pictures, and looking at the real one
    // is what the flag is for. A machine with no directory yet still gets the
    // four built-ins, which is `Roster::built_in`'s job and not a fallback
    // here.
    let roster =
        oag_raceplay::pilots::load().unwrap_or_else(|_| oag_raceplay::pilots::Roster::built_in());
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
        &oag_raceplay::pilots::AXES
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
        .map(|entry| oag_raceplay::pilots::AXES[0].1(&entry.pilot));
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
    let reserve_note = oag_raceplay::pilots::page_reserves_axis_preview(model.page());
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
    // draws this line through too; see `oag_raceplay::pilots::axis_preview_for`'s
    // own doc.
    let axis_preview = oag_raceplay::pilots::axis_preview_for(&model, Some(strings));
    // The legend's own draw list - empty on a source with none. **`Back`
    // never shows here**: `model.open(page)` above replaces the stack
    // outright rather than walking to it (`Menu::open`'s own doc - "a page
    // reached this way was not reached through anything"), so
    // `model.depth()` is always `1` regardless of which page was named.
    // See `crate::main::menu_stage::MenuStage::render`'s own call site,
    // which this mirrors, for the live session's own gate.
    let nav_legend_draws: Vec<oag_ui::frontend::Draw> =
        nav_legend.map_or_else(Vec::new, |legend| {
            legend.draw_gated(
                &oag_ui_screens::picker::FaceScales::default(),
                default_measure,
                model.depth() > 1,
            )
        });
    // The footer's own scrolling tip ticker, frozen at `elapsed = 0.0` - a
    // still has no clock of its own to animate the scroll with, so this
    // shows whichever tip the rotation starts on rather than one mid-scroll.
    // `measure`, not `default_measure`: the ticker's own `font="small"`
    // routes through no named role (`oag_ui_screens::campaign::footer::face_role`
    // answers `None` for it), so it draws through the same primary atlas the
    // rows do - see `crate::main::menu_stage::MenuStage::render`'s own
    // identical choice for its live ticker.
    let ticker_draw: Option<oag_ui::frontend::Draw> = ticker.and_then(|layout| {
        oag_ui_screens::campaign::footer::ticker_draw(
            layout,
            0.0,
            ticker_tips,
            &oag_ui_screens::picker::FaceScales::default(),
            measure,
        )
    });
    let ticker_draws: Vec<oag_ui::frontend::Draw> = ticker_draw.clone().into_iter().collect();
    // The ticker's own clip window, `(left, right)` in screen space - see
    // this function's own doc. Resolved against `list` after every draw is
    // in it, the same `position`-by-equality idiom `MenuStage::render`'s own
    // call site uses, rather than a hand-tracked index a later `extend`
    // could silently invalidate.
    let ticker_clip = |list: &[oag_ui::frontend::Draw]| {
        let draw = ticker_draw.as_ref()?;
        let index = list.iter().position(|d| d == draw)?;
        let [x, _, width, _] = ticker?.viewport;
        Some((index, x, x + width))
    };

    // A modal prompt over the page, when one was asked for.
    //
    // **The only way to look at one headlessly.** `--menu-page` runs no clock
    // and calls no `Menu::update`, so a prompt - which exists precisely
    // because a row was activated - can never appear here by itself. Without
    // this flag the on-screen keyboard's layout is reviewable only by playing
    // the game on a machine with a display, and this project's own rule is to
    // judge a screen by looking at it. The models are the live ones
    // (`oag_ui_screens::prompt`) and the labels come from the same lookup the live
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
            list.push(oag_ui_screens::prompt::axis_preview_draw(
                &model, skin, text,
            ));
        }
        list.extend(records_draws(&model, skin, title, tracks, records));
        list.extend(nav_legend_draws);
        list.extend(ticker_draws);
        let clip = ticker_clip(&list);
        list.extend(prompt_draws(kind, &name, strings, skin, source)?);
        return Ok((list, clip));
    }
    let Some(phase) = phase else {
        let mut list = layers.flatten();
        if let Some(text) = &axis_preview {
            list.push(oag_ui_screens::prompt::axis_preview_draw(
                &model, skin, text,
            ));
        }
        list.extend(records_draws(&model, skin, title, tracks, records));
        list.extend(nav_legend_draws);
        list.extend(ticker_draws);
        let clip = ticker_clip(&list);
        return Ok((list, clip));
    };
    // The same arithmetic the live stage runs, through the same easing, so what
    // this draws is a frame of the real transition rather than a picture of one.
    // No axis-preview line and no records table here, matching
    // `MenuStage::render`'s own gate: the row list is a zoomed, mid-tween
    // picture at this point, and both are positioned against the still one.
    let shape = oag_ui::menu::Transition::default();
    let mut tween = oag_ui::anim::Tween::new(1.0);
    tween.advance(phase.clamp(0.0, 1.0));
    let t = tween.eased();
    Ok((
        layers
            .zoomed(shape.origin, shape.in_scale + (1.0 - shape.in_scale) * t, t)
            .flatten(),
        None,
    ))
}

/// The RECORDS page's own live per-class table, as draws - empty off any
/// page but RECORDS, or while `model`'s own MODE/TRACK rows have not
/// resolved a track this capture's `tracks` list carries.
///
/// `crate::scoreboard::records_table` is the one function the live session
/// draws this same table through - see its own doc for why the track lookup
/// here (this capture's own boot-survey `tracks`, with no distinct Zone
/// list) can differ from `Shell::tracks_for(mode)` without the two pages
/// ever disagreeing about anything they both can answer. `oag_ui_screens::prompt::
/// record_row_draw` is the same drawing half `crate::menu_stage::MenuStage::
/// render` calls for the live session, so a row's position here matches a
/// live one for the same `model` state (`Menu::visible_rows`, `Menu::scroll`).
fn records_draws(
    model: &oag_ui::menu::Menu,
    skin: &oag_ui::menu::Skin,
    title: &'static oag_title::Title,
    tracks: &[oag_raceplay::catalogue::Track],
    records: &crate::records::Store,
) -> Vec<oag_ui::frontend::Draw> {
    let Some(rows) = crate::scoreboard::records_table(model, title, records, |_mode, track_id| {
        tracks
            .iter()
            .find(|track| track.id == track_id)
            .map(oag_raceplay::catalogue::Track::entry_name)
    }) else {
        return Vec::new();
    };
    rows.iter()
        .enumerate()
        .flat_map(|(index, (label, value))| {
            oag_ui_screens::prompt::record_row_draw(model, skin, index, label, value)
        })
        .collect()
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
/// If `kind` is not one of the prompts this flag knows, or - `tag-entry`/
/// `tag-entry-typed` only - if `source` is `None` or its disc does not carry
/// Pulse's own `TagInput` screens intact. Every other kind ignores `source`
/// entirely.
fn prompt_draws(
    kind: &str,
    name: &str,
    strings: &oag_ui::language::StringTable,
    skin: &oag_ui::menu::Skin,
    source: Option<&str>,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
    let say_of = |id: &str, english: &str| strings.get(id).unwrap_or(english).replace("%s", name);
    match kind {
        "tag-entry" | "tag-entry-typed" => {
            let source = source.context("tag-entry needs a --race source open")?;
            let mut archives = oag_pulse::open(source).context("this source is not Pulse's own")?;
            let skin_raw = archives
                .read_name(oag_pulse::names::FRONTEND_ROOT)
                .context("no front-end root on this disc")?;
            let skin_xml = oag_tables::fexml::text(&skin_raw).context("front-end root text")?;
            let globals = oag_ui::screen::Screens::from_xml(&skin_xml).globals;
            let entry_raw = archives
                .read_hash(oag_pulse::hashes::TAG_INPUT_SCREENS)
                .context("no TagInput screens entry on this disc")?;
            let xml = oag_tables::fexml::text(&entry_raw).context("TagInput entry text")?;
            let geometry = oag_ui_screens::tag_entry::geometry(&xml, &globals, "Name")
                .context("the Name TagInput was not found in it")?;
            let alphabet: String = oag_pulse::tag_input::ALPHABET
                .chars()
                .filter(|&c| oag_ui_screens::prompt::accepts(c))
                .collect();
            let mut tag_entry = oag_ui_screens::tag_entry::TagEntry::new(
                oag_ui_screens::tag_entry::Labels {
                    title: say("OAG_PILOT_RENAME_TITLE", "RENAME PILOT"),
                    confirm: say("OAG_KEYBOARD_ACCEPT", "OK"),
                    hint: say(
                        "OAG_TAG_ENTRY_HINT",
                        "LEFT/RIGHT CELL   UP/DOWN GLYPH   CROSS/START ACCEPT   CIRCLE CANCEL",
                    ),
                },
                geometry,
                &alphabet,
                name,
            );
            if kind == "tag-entry-typed" {
                // A few glyph changes past what `name` opened on, so a
                // before/after pair actually differs - the same reason
                // `rename-note` exists beside `rename` above.
                for edit in [
                    oag_ui_screens::prompt::Edit::Type('z'),
                    oag_ui_screens::prompt::Edit::Type('9'),
                    oag_ui_screens::prompt::Edit::Delete,
                    oag_ui_screens::prompt::Edit::Type('-'),
                ] {
                    tag_entry.edit(edit);
                }
            }
            Ok(tag_entry.draw(skin))
        }
        "rename" | "rename-note" => {
            let mut keyboard = oag_ui_screens::prompt::Keyboard::new(
                oag_ui_screens::prompt::Labels {
                    title: say("OAG_PILOT_RENAME_TITLE", "RENAME PILOT"),
                    delete: say("OAG_KEYBOARD_DELETE", "DEL"),
                    accept: say("OAG_KEYBOARD_ACCEPT", "OK"),
                    hint: say(
                        "OAG_KEYBOARD_HINT",
                        "CROSS TYPE   SQUARE DELETE   START ACCEPT   CIRCLE CANCEL",
                    ),
                },
                name,
                oag_raceplay::pilots::MAX_NAME,
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
            Ok(
                oag_ui_screens::prompt::Confirm::new(oag_ui_screens::prompt::ConfirmLabels {
                    title: say("OAG_PILOT_DELETE_TITLE", "DELETE PILOT"),
                    message,
                    yes: say("OAG_PILOT_DELETE_YES", "DELETE"),
                    no: say("OAG_PILOT_DELETE_NO", "KEEP"),
                })
                .draw(skin),
            )
        }
        // The CONTROLS page's key-capture prompt - `oag_ui_screens::prompt::
        // message_draw` is the same function `MenuStage::render` calls in
        // the binary, off `Session::awaiting_binding`, so this and a live
        // capture cannot draw two different pictures for the same state.
        // See `docs/architecture/menus.md`'s Rebinding section.
        "binding" => Ok(oag_ui_screens::prompt::message_draw(
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
    /// Where it goes, in the screen's grid - [`oag_ui_screens::picker::Layout::preview`].
    pub rect: [f32; 4],
    pub kind: oag_ui_screens::picker::Kind,
    /// The selected circuit's own `<Mode3D><Model>` pose, when its
    /// `screen.xml` authors one - see
    /// [`oag_game::preview::mode3d_view_projection`]. `None` falls back to
    /// [`oag_game::preview::orbit_for`], same as the live screen.
    pub mode3d: Option<oag_ui::screen::Model>,
}

/// Which selection screen a `--menu-page` name asks for, if either.
#[must_use]
pub(super) fn picker_kind(page: &str) -> Option<oag_ui_screens::picker::Kind> {
    match page {
        "track-select" | "track_select" => Some(oag_ui_screens::picker::Kind::Track),
        "ship-select" | "ship_select" => Some(oag_ui_screens::picker::Kind::Ship),
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
    kind: oag_ui_screens::picker::Kind,
    circuit_names: &oag_ui::language::CircuitNames,
    layout: &oag_ui_screens::picker::Layout,
    settings: &crate::settings::Settings,
    title: &'static oag_title::Title,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    strings: &oag_ui::language::StringTable,
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    sprites: &oag_hud::sprite::Sheet,
    measure: &dyn Fn(&str) -> f32,
    // The selected circuit's lap length, measured by the caller off the
    // disc - `None` draws the dash an unmeasured one draws live.
    distance: Option<f32>,
    // `--menu-picker-seconds` - `None` draws the screen settled, past every
    // `LeftLayer transition` it authors. See [`SETTLED_SECONDS`].
    seconds: Option<f32>,
) -> (Vec<oag_ui::frontend::Draw>, Option<PreviewRequest>) {
    use oag_ui_screens::picker::{Details, Entry, Kind, Picker};
    let mut grid_columns = 0;
    let (entries, previews): (Vec<Entry>, Vec<String>) = match kind {
        Kind::Track => {
            let (entries, previews, columns) = track_entries::track_entries(
                title,
                settings,
                circuit_names,
                strings,
                tracks,
                layout,
                distance,
            );
            grid_columns = columns;
            (entries, previews)
        }
        Kind::Ship => {
            let gates = crate::unlock::gates_variants(title);
            let saved = if gates {
                crate::records::load()
            } else {
                crate::records::Store::default()
            };
            teams
                .iter()
                .map(|team| {
                    // The same axis rule the live screen applies - see
                    // `session::picker::open_ship_picker`: the team's own skins
                    // where it declares any, the title's variants otherwise.
                    let variants: Vec<(String, String)> = if !team.skins.is_empty() {
                        std::iter::once((
                            String::new(),
                            strings
                                .get_or_id(oag_raceplay::catalogue::BASELINE_SKIN)
                                .to_string(),
                        ))
                        .chain(
                            team.skins
                                .iter()
                                .filter(|skin| {
                                    !gates
                                        || crate::unlock::loyalty_unlocked(
                                            &skin.unlock,
                                            &saved,
                                            title.name,
                                        )
                                })
                                .map(|skin| {
                                    (skin.name.clone(), strings.get_or_id(&skin.name).to_string())
                                }),
                        )
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
                    let stats = team.variant_stats(variants.iter().map(|(id, _)| id.as_str()));
                    (
                        Entry {
                            id: team.id.clone(),
                            label: team.label(strings).to_string(),
                            details: Details::Ship {
                                loyalty: gates.then(|| saved.loyalty_total(title.name, &team.id)),
                                rating: team.rating.map(|rating| oag_ui_screens::picker::Rating {
                                    speed: rating.speed,
                                    thrust: rating.thrust,
                                    handling: rating.handling,
                                    shield: rating.shield,
                                }),
                                variants,
                                stats,
                            },
                        },
                        format!(r"{}\ship_FE.vex", team.location),
                    )
                })
                .unzip()
        }
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
            Some(settings.race.opening_variant(title)),
        ),
    };
    let mut picker = Picker::new(kind, entries, Some(selected), variant);
    if layout.hd_track.is_some() {
        picker = picker.with_rows(grid_columns);
    }
    picker.tick(seconds.unwrap_or(SETTLED_SECONDS));
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
        // Filled in by the caller, which already has `picker_stills`'s own
        // slideshow read - see `capture.rs`.
        mode3d: None,
    });
    let layers = oag_ui_screens::picker::draw_list(
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

/// The source a capture's race options name, opened with its packs, for the
/// selection screens' own reads.
pub(super) fn open_for_previews(race: &oag_raceplay::Options) -> Result<oag_assets::Archives> {
    let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
        &race.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    for problem in problems {
        log::warn!("{problem}");
    }
    Ok(oag_source::title::open_source(&race.source, packs, pure_packs)?.archives)
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
    race: &oag_raceplay::Options,
    request: &PreviewRequest,
    anisotropy: Anisotropy,
) {
    let built = open_for_previews(race).and_then(|mut archives| {
        let mut model = crate::preview::model(&mut archives, &request.entry)?;
        // The chosen paint, the same swap the live screen and a race make;
        // a skin that will not read leaves the hull's own and says so.
        if let Some(entry) = &request.skin {
            for line in crate::preview::paint(&mut archives, entry, &request.entry, &mut model) {
                log::debug!("preview {}: {line}", request.entry);
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
        Ok(mut preview) => preview.draw_auto(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            request.mode3d.as_ref(),
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
    kind: oag_ui_screens::picker::Kind,
    settings: &crate::settings::Settings,
    track: Option<&oag_raceplay::catalogue::Track>,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    archives: Option<&mut oag_assets::Archives>,
    screens: &oag_ui::screen::Screens,
    strings: &oag_ui::language::StringTable,
    sprites: &mut oag_hud::sprite::Sheet,
    // `--menu-picker-seconds` - `None` draws the first card, fully arrived
    // (`SETTLED_SECONDS`). `Some` also advances which card is stacked up, so
    // a low value shows both effects the screen's own arrival authors: the
    // fade in and the first card sliding under a second.
    seconds: Option<f32>,
) -> (Vec<oag_ui::frontend::Draw>, Option<oag_ui::screen::Model>) {
    // The entry the *picker* selects, which falls back to the first when the
    // setting names none this source offers - `Picker::new`'s own rule. A
    // second lookup that stopped at `None` instead would leave a capture of
    // another title's disc showing a highlighted row with no picture beside
    // it, which is a disagreement with the live screen and not a finding.
    let source = match kind {
        oag_ui_screens::picker::Kind::Track => track.or(tracks.first()).map(|track| {
            (
                track.location.clone(),
                oag_race::Mode::from_name(&settings.race.mode) == Some(oag_race::Mode::Zone),
            )
        }),
        oag_ui_screens::picker::Kind::Ship => teams
            .iter()
            .find(|team| team.id == settings.race.team)
            .or(teams.first())
            .map(|team| (team.location.clone(), false)),
    };
    let (Some(archives), Some((location, zone))) = (archives, source) else {
        return (Vec::new(), None);
    };
    let globals: Vec<(&str, &str)> = screens
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let mut report = Vec::new();
    let (stills, mode3d) = match crate::preview::slideshow(
        archives,
        &location,
        zone,
        &globals,
        strings,
        &mut report,
    ) {
        Ok((show, blobs)) => {
            *sprites = sprites.extended(&blobs, &mut report);
            let alpha = (seconds.unwrap_or(SETTLED_SECONDS) / crate::preview::CARD_FADE_SECONDS)
                .clamp(0.0, 1.0);
            let draws = show
                .draws(seconds.unwrap_or(0.0), &|src| sprites.get(src))
                .into_iter()
                .map(|draw| crate::preview::fade_draw(draw, alpha))
                .collect();
            (draws, show.model.clone())
        }
        Err(error) => {
            log::warn!("{error:#} - {location} shows no stills");
            (Vec::new(), None)
        }
    };
    oag_raceplay::loader_log::lines(report);
    (stills, mode3d)
}

// A plain `mod tests;`, not `#[path]`ed: this file has no `#[path]` of its
// own (it is a normal child of `capture.rs`'s `mod menu_page;`), so this
// resolves to `menu_page/tests.rs` beside it, the same move
// `crates/physics/src/airbrake.rs` made when a `#[cfg(test)] mod` crossed
// its own 200-line cap. This module was still under that cap on its own,
// but adding `records_draws`'s own test and its `layers_text_y` fixture put
// the *file* - code plus inline tests together - one line over
// `scripts/check-file-size.py`'s separate 1,000-line cap on the file
// itself, and moving the tests out is the same fix either cap asks for.
mod race_sources;
mod track_entries;

#[cfg(test)]
mod tests;
