//! Opening the menus, and what a menu event does.

use anyhow::{Context, Result};
use log::{error, info, warn};

use oag_display::display;
use oag_game::render::{Renderer, VideoFormat};
use oag_game::{boot, movie, settings};
use oag_raceplay::catalogue;
use oag_raceplay::pilots;
use oag_ui::{menu, strings};
use oag_ui_screens::marquee;

use crate::frontend_stage::HeldFrame;
use crate::hints;
use crate::menu_stage::{Backdrop, MenuStage, menu_playhead};
use crate::stage::Stage;
use crate::window::monitor_names;

use super::{Session, remix_menu};

/// Moved to `oag_game::scoreboard::speed_class_choices`, re-exported here
/// under its own name so every call site in this module (and
/// `crate::records_page`, which names this exact path in its own doc)
/// keeps working unchanged. It moved because `crate::capture::menu_page` -
/// the `--menu-page records` still - needs the identical list for the
/// RECORDS page's own per-class table, and this binary's `session` module is
/// not reachable from the library crate that lives in. See that function's
/// own doc for the full reasoning, including the `oag_title::SpeedClasses::
/// is_offered_outside_remix` confinement.
pub(crate) use oag_game::scoreboard::speed_class_choices;

/// [`speed_class_choices`]' shared tail, so the per-title row and RACE REMIX's
/// union spell a class exactly the same way.
pub(super) fn to_choices(names: Vec<&'static str>) -> Vec<menu::Choice> {
    names
        .into_iter()
        .map(|name| menu::Choice::labelled(name.to_lowercase(), name))
        .collect()
}

/// What the ordinary RACE page should actually launch on, given the stored
/// `race.class` and the title it booted - `None` meaning "leave the previous
/// class in place", the same as an empty stored value always has.
///
/// **This exists because `race.class` is one setting shared with RACE
/// REMIX.** `Session::launch_remix` reads the exact same key, and its own row
/// offers every rung [`oag_title::SpeedClasses::is_selectable`] allows -
/// `VECTOR` included, when a Pure source is mounted. So a player who settles
/// `race.class` to `"vector"` on RACE REMIX and then opens the ordinary RACE
/// page has a stored value this page's own row no longer lists (see
/// [`speed_class_choices`]'s confinement to
/// [`oag_title::SpeedClasses::is_offered_outside_remix`]). `Menu::supply`
/// only ever changes the widget's own display index, never the stored
/// setting, so without this check the row would *display* the fallback
/// (`"venom"`) while [`Session::handle_menu`]'s `LaunchRace` arm launched
/// whatever was still in storage (`"vector"`) underneath it - a menu saying
/// one thing and a race doing another, which is exactly the silent mismatch
/// [`oag_title::SpeedClasses::is_selectable`]'s own doc comment forbids.
///
/// Mirrors the CIRCUIT and TEAM fallbacks in the same handler: a stored value
/// this page does not offer races the first one the row does, with a warning,
/// rather than silently keeping the mismatch.
///
/// `title` is `None` only when [`Session::shell`] itself is - unreachable in
/// practice, since there are no menus before a disc has been chosen - and in
/// that case `stored` is carried through unclamped, the same as before this
/// check existed, because there is no row to check it against.
fn resolve_race_page_class(
    title: Option<&'static oag_title::Title>,
    stored: &str,
) -> Option<String> {
    if stored.is_empty() {
        return None;
    }
    let Some(title) = title else {
        return Some(stored.to_string());
    };
    let choices = speed_class_choices(title);
    let resolved = choices
        .iter()
        .find(|choice| choice.value.eq_ignore_ascii_case(stored))
        .or_else(|| {
            warn!(
                "this page does not offer speed class {stored:?}; racing {} instead, \
                 which is what the menu shows",
                choices
                    .first()
                    .map(|choice| choice.value.as_str())
                    .unwrap_or("this source's own default"),
            );
            choices.first()
        });
    resolved.map(|choice| choice.value.clone())
}

/// Which picture [`Session::open_menus`] should seed the new renderer's
/// planes with.
///
/// **The front end's own held frame on the boot path** (`frontend_held`,
/// only ever `Some` when the stage being replaced is [`crate::stage::Stage::Frontend`]).
/// **The menus' own last picture on the `escape` path** (`held_menu_backdrop`,
/// only consulted when `is_race`): the `MenuStage` that showed it is gone by
/// the time a race ends, so `Session::launch_race` moves it out to
/// [`Session::held_menu_backdrop`] before dropping that stage, and this reads
/// it back rather than the stage itself. `None` on every other transition,
/// and also whenever the candidate for the path taken is itself `None` - a
/// source with no backdrop at all seeds nothing on either path.
///
/// **Takes the two candidates and `is_race` rather than `&Stage`
/// itself**, which is what makes this a plain function over data instead of
/// a method that needs a live stage - and a live `Stage::Race` carries a
/// whole GPU scene, which has no business existing just to be matched on in
/// a test. See `main/tests.rs`.
pub(crate) fn backdrop_seed(
    frontend_held: Option<&HeldFrame>,
    is_race: bool,
    held_menu_backdrop: Option<&HeldFrame>,
) -> Option<HeldFrame> {
    let held = if is_race {
        held_menu_backdrop
    } else {
        frontend_held
    };
    held.map(|held| HeldFrame {
        index: held.index,
        picture: held.picture.clone(),
    })
}

#[path = "menus/variant.rs"]
mod variant;
pub(crate) use variant::{combine_variant, gated_variant_choices, variant_choices};

#[path = "menus/team.rs"]
mod team;

impl Session {
    /// Replaces the front end with the menus, seeded from the settings.
    ///
    /// The renderer moves across rather than being rebuilt - it holds the disc's
    /// font atlas and sprite sheet, already uploaded, and a menu row is text and
    /// a rectangle. A stage that is not the front end leaves the menus where
    /// they are, which is what makes this safe to call from the frame loop.
    pub(crate) fn open_menus(&mut self) -> Result<()> {
        let shell = self
            .shell
            .clone()
            .context("this run has no menus: nothing loaded a font or a sprite sheet")?;
        // Everything below this is a load, and a load is not a frame time.
        self.stalled = true;
        let mut model = menu::Menu::new(shell.definition.clone());
        // The skin decides the row pitch, and the pitch decides how many rows a
        // page shows - so the window has to be told before anything scrolls.
        // The face the rows will be drawn in, which is also the face whose
        // line height sets the pitch - so the two are chosen together or the
        // rows would be spaced for a font they are not drawn in.
        let rows_face = shell
            .menu_font
            .clone()
            .unwrap_or_else(|| shell.font.clone());
        // The source's own grid, not the PSP's: HD authors its `FEGlobals` at
        // 1920x1080 and its faces are rasterised for that screen, so the rows
        // are laid out and drawn where the disc says rather than shrunk into
        // another console's coordinates. `Space::PSP` on both PSP titles, where
        // the ratio is exactly 1.0 and nothing moves. See `menu::Skin::new`.
        let mut skin = menu::Skin::new(shell.menu_skin, shell.space, rows_face.line_height);
        // And where that face keeps its ink, for the pointer bands on the
        // selection screens' rows - the renderer below draws them with it.
        skin.set_row_ink(oag_ui::pointer::RowInk::measure(&rows_face));
        // Just the first frame's own value - `MenuStage::render` refreshes
        // this every frame off whichever page is actually current, since a
        // player can navigate to a page with a different reservation need
        // (AI PILOTS today) without this method running again. See
        // `menu::visible_rows`'s own doc for why the reservation has to
        // track the page rather than the frame's live row values.
        let reserve_note = pilots::page_reserves_axis_preview(model.page());
        model.set_visible_rows(menu::visible_rows(&skin, &shell.frame, reserve_note));
        // And which way a page steps, for the disc that draws one of them as a
        // strip: left and right along the entries rather than up and down a
        // column. Set from the same skin the drawing side reads, so what is on
        // screen and what the buttons do cannot disagree.
        model.set_strip_layout(skin.strip().is_some());
        // Supplied before seeding, because a value cannot be seeded onto a list
        // that is not there yet. The race list or the Zone one, matching
        // whichever `race.mode` is already saved - so a menu reopened on a
        // saved Zone setting shows the Zone list from the first frame, not
        // just after the row is next touched. See `Self::race_mode` and
        // `Self::resupply_tracks_for_mode`, which keeps the two in step
        // whenever MODE changes while the menus stay open.
        let tracks: Vec<menu::Choice> = shell
            .tracks_for(self.race_mode())
            .iter()
            .map(|(track, name)| menu::Choice::labelled(&track.id, name))
            .collect();
        model.supply(menu::ValueSource::Tracks, &tracks);
        model.supply(menu::ValueSource::Teams, &shell.teams);
        // Scoped to whichever team `race.team` already names, the same
        // reason the CIRCUIT row above is seeded from `race.mode`, and
        // settled against that list for the reason RACE REMIX's own rows are
        // below - `race.variant` defaults to empty, which is a real value
        // only on the titles whose first variant is the unsuffixed one.
        let race_variants =
            gated_variant_choices(shell.title, &self.settings.race.team, &shell.team_details);
        remix_menu::settle(&mut self.settings.race.variant, &race_variants);
        model.supply(menu::ValueSource::RaceVariant, &race_variants);
        // The booted title's own ladder, filtered to the rungs this build can
        // put a ship on. `None` means the title's per-team handling files have
        // not been read - Wipeout 2048 - and the caller keeps its own list
        // rather than lending that title another's measurement.
        model.supply(
            menu::ValueSource::SpeedClasses,
            &speed_class_choices(shell.title),
        );
        // RACE REMIX's own axis: the union across every title this machine can
        // currently open a source for, so a class is offered exactly when the
        // data behind it is actually here. See `Self::remix_speed_classes`.
        //
        // **Settled only when the union has something in it**, the same rule
        // the RACE REMIX rows below follow and for the same reason: an empty
        // union is not evidence that a saved pick is wrong, and settling
        // against it would clear `race.class` to the empty string. That is a
        // reachable configuration rather than a hypothetical - a machine whose
        // only playable source is Wipeout 2048, whose ladder is unread, unions
        // to nothing - and it would silently lose the player's saved class
        // while the RACE page above still displayed its own fallback.
        //
        // `supply` is unconditional because it does not touch `self.settings`:
        // it keeps the row on its current value when the list still offers it
        // and falls to the first entry for *display* otherwise.
        let remix_classes = self.remix_speed_classes();
        if !remix_classes.is_empty() {
            remix_menu::settle(&mut self.settings.race.class, &remix_classes);
        }
        model.supply(menu::ValueSource::RemixSpeedClasses, &remix_classes);
        model.supply(menu::ValueSource::Languages, &shell.languages);
        model.supply(menu::ValueSource::RaceModes, &shell.modes);
        model.supply(menu::ValueSource::KillTargets, &shell.kill_targets);
        model.supply(menu::ValueSource::Weapons, &shell.weapons);
        // Enumerated every time the menus open rather than kept from startup,
        // because a screen can be plugged in while the game is running and the
        // row should show it without a restart.
        let monitors: Vec<menu::Choice> = display::Monitor::offered(&monitor_names(
            &self.gpu.window.available_monitors().collect::<Vec<_>>(),
        ))
        .into_iter()
        .map(menu::Choice::plain)
        .collect();
        model.supply(menu::ValueSource::Monitors, &monitors);
        // Kept from startup rather than enumerated, unlike the monitors above,
        // and the difference is what a fresh answer would be worth. A screen
        // plugged in now can be used now; an adapter plugged in now cannot,
        // because the device was made at boot. Listing one would be offering a
        // row that does nothing this run.
        let renderers: Vec<menu::Choice> = display::Renderer::offered(&self.gpu.adapters)
            .into_iter()
            .map(menu::Choice::plain)
            .collect();
        model.supply(menu::ValueSource::Renderers, &renderers);
        // Empty unless this machine has both Pulse discs, which draws the row
        // unusable rather than offering a swap that cannot happen. Kept from
        // the boot survey rather than re-derived here: answering it means
        // opening every image on the search path, and a menu opening is not
        // the moment for that. See `oag_sound::MusicDiscs`.
        let music_sources: Vec<menu::Choice> = if self.music_discs.both() {
            oag_sound::MusicSource::ALL
                .iter()
                .map(|source| menu::Choice::plain(source.name()))
                .collect()
        } else {
            Vec::new()
        };
        model.supply(menu::ValueSource::MusicSources, &music_sources);
        // The stylings this source ships art for, off its own title package.
        // One row on every title but Wipeout HD, which ships two - and a row
        // with one value draws the setting as the fact it is. Empty on a source
        // whose title authors no loading screen at all, which greys it out.
        model.supply(menu::ValueSource::FrontEndStyles, &shell.front_end_styles);
        // Off, the built-ins, and the player's own files as of the last poll.
        model.supply(
            menu::ValueSource::ScreenFilters,
            &self.screen_filters.choices(),
        );
        // Race Remix's own axis - every title this machine can currently open
        // a source for, surveyed once at `Session` construction. See the
        // field's own doc comment. TRACK TITLE's own row: 2048's circuits are
        // not HD's, so no synthetic entry applies here; CRAFT TITLE's does,
        // see `Self::craft_title_choices`.
        //
        // **Settled, not merely supplied**, and outward-in: each row's stored
        // setting is brought into step with the list it is about to be given
        // before the row below it is resolved from that setting. See
        // `remix_menu::settle`, which is what keeps a fresh settings file's
        // empty `remix.*` from leaving every row showing one thing and
        // `self.settings` holding another.
        let (titles, craft_titles) = self.settle_remix_titles();
        model.supply(menu::ValueSource::Titles, &titles);
        model.supply(menu::ValueSource::CraftTitles, &craft_titles);
        // Scoped to whichever titles `remix.track_title`/`remix.craft_title`
        // now name, the same reason the CIRCUIT row above is seeded from
        // `race.mode` rather than left empty until MODE is next touched: a
        // menu reopened on a saved pick should show that pick's own list from
        // the first frame. Both are `None` and supply nothing when the named
        // title's source is no longer on this machine's search path, or will
        // not open - see `Self::remix_catalogue_for`/
        // `Self::remix_craft_catalogue`. Nothing is settled in that case
        // either: an unreadable source is not evidence that a saved pick is
        // wrong, and clearing it would lose it.
        if let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.track_title) {
            let tracks = remix_menu::remix_track_choices(&catalogue);
            remix_menu::settle(&mut self.settings.remix.track, &tracks);
            model.supply(menu::ValueSource::RemixTracks, &tracks);
        }
        if let Some(catalogue) = self.remix_craft_catalogue() {
            remix_menu::settle(&mut self.settings.remix.team, &catalogue.teams);
            model.supply(menu::ValueSource::RemixTeams, &catalogue.teams);
        }
        // Last, because it is scoped to the team the line above just settled.
        if let Some(title) = self.craft_title() {
            let variants = variant_choices(title, &self.settings.remix.team);
            remix_menu::settle(&mut self.settings.remix.variant, &variants);
            model.supply(menu::ValueSource::RemixVariant, &variants);
        }
        // Ours, and settings-backed like everything above it - except PILOT
        // and AXIS, which nothing persists, so this both supplies and seeds
        // LOW/HIGH in one call. See `super::pilot_editor`.
        self.supply_pilot_menu(&mut model);
        self.seed_menu(&mut model, shell.title, shell.platform);
        // What the row is set to comes from the settings file, above; what the
        // game is *drawing with* can only come from here, and the RENDERER row's
        // restart note is the difference between the two. Told every time the
        // menus open rather than once, because the model is new every time.
        let in_use: Vec<menu::Value> = self
            .gpu
            .in_use
            .iter()
            .map(|name| menu::Value::Text(name.clone()))
            .collect();
        if !model.in_effect("graphics.renderer", &in_use) {
            warn!("nothing in the menus defers graphics.renderer");
        }
        // What the row is set to comes from the settings file; what a race on
        // screen is actually *drawing* with can only come from its own
        // `Scene`, built once when that race started. Silent on the front end
        // and the menus, where no race exists yet to compare against - see
        // `Restart::in_effect`.
        //
        // **A single value, since [ADR-0041] split the row.** This used to
        // carry a list, because the old `anti_aliasing` row mixed MSAA - baked
        // into the scene's pipelines at `race::Scene::new` - with FXAA and SMAA,
        // which `upscale::Framebuffer::resolve_scene` reads fresh every frame;
        // every mode sharing the built scene's sample count was equally "in
        // effect". The MSAA row is now nothing but the sample count, so the
        // only thing in effect is the level the running scene was built with.
        //
        // [ADR-0041]: ../../../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md
        if let Stage::Race(stage) = &self.stage {
            let in_use = [menu::Value::Text(stage.scene.msaa().to_string())];
            if !model.in_effect("graphics.msaa", &in_use) {
                warn!("nothing in the menus defers graphics.msaa");
            }
            // Same story as MSAA above, but a single tier baked in at
            // `Race::start` rather than a range of equivalent modes.
            let in_use = [menu::Value::Text(stage.race.boost_fov_kick().to_string())];
            if !model.in_effect("graphics.boost_fov_kick", &in_use) {
                warn!("nothing in the menus defers graphics.boost_fov_kick");
            }
        }

        // **The movie planes are asked for only when there is a movie to put in
        // them.** This used to be unconditionally `None`, on the grounds that
        // wanting them would tie the menus to a stage that had played one; that
        // reasoning still holds and this does not break it. What the menus are
        // tied to is a *movie*, handed to them by whoever built the session, and
        // it is optional - the menus open with no backdrop on a source that has
        // none and on `--no-video`, and draw on black there.
        //
        // **Asked of the feed, not of a `Movie`.** The frames moved onto a decode
        // thread when the session was built, so a `Movie`'s `frames` is `None`
        // by now and testing it would build a renderer with no video pipeline and
        // draw the menus on black - with no error anywhere, and with
        // `--menu-page` still looking right, because that flag goes through the
        // headless capture path and its movie is a different one that kept its
        // frames. See `VideoFormat::of_feed`.
        //
        // **The playhead is carried over the front end's shoulder, not built
        // fresh.** See `menu_playhead`: coming out of the boot sequence there is
        // already one running, and rebuilding it at frame zero is what made the
        // picture jump back to the start of the loop the instant START was
        // pressed. The feed is only restarted when there is nothing to carry -
        // the two share an origin already, and restarting one of them is exactly
        // what would put them at odds.
        let shape = self.backdrop_shape.filter(|_| self.backdrop.is_some());
        let format = self.backdrop.as_ref().map(VideoFormat::of_feed);
        let frames = self.backdrop.as_ref().map_or(0, movie::Feed::len);
        // Cloned before the move below: the renderer takes the atlas to build
        // its own GPU-side texture from, but the stage also wants it plain,
        // to measure a value column's text against. See
        // `MenuStage::text_atlas`.
        let text_atlas = rows_face.clone();
        let mut renderer = Renderer::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.config.format,
            format,
            rows_face,
            &shell.sprites,
        )?;
        // **A fresh `Renderer` is `Space::PSP` until told otherwise**, and the
        // menus are the one stage that never told it. That was right while their
        // every coordinate was this build's own 480x272; it stopped being right
        // when `Skin` started handing back the source's. Both halves move
        // together or neither does - the `screen` uniform normalising a rect and
        // the rect itself have to be in the same grid, and being wrong in the
        // same direction is exactly what made this invisible until a 1920-wide
        // source arrived.
        renderer.set_space(skin.space());
        // The screen title's own face, when this title names a role for it -
        // mirrors `rows_face` above, one widget over. On both PSP titles,
        // whose chrome names no `Title` role, this slot instead carries the
        // `Default`-role atlas so `oag_ui_screens::campaign::footer`'s
        // `Draw::FacedText { role: "Default", .. }` can draw its own body
        // face beside the unchanged `menu`-role primary - see
        // `boot::fonts::face_atlas_slot`'s own doc. See `capture::run`'s own
        // call for why this has to happen on both the live and the headless
        // path.
        boot::fonts::install_faces(
            &mut renderer,
            &self.gpu.device,
            &self.gpu.queue,
            shell.menu_skin,
            &shell.font,
            shell.title_font.clone(),
            shell.buttons_font.clone(),
        );
        // The style's backdrop, uploaded once per menu stage the same way the
        // sprite sheet is; the model that moves is built from the same assets.
        if let Some(assets) = &shell.fury_backdrop {
            assets.install(&mut renderer, &self.gpu.device, &self.gpu.queue);
        }
        let styled = shell
            .fury_backdrop
            .as_ref()
            .and_then(|assets| assets.live());
        // **The picture moves across as well as the playhead**, and it has to,
        // because the renderer does not. A fresh `Renderer` is fresh planes:
        // zeroed, which is green rather than black, so the first menu frame
        // would draw its rows on the black fill instead and the picture would
        // only appear once the playhead reached the next decoded frame. That is
        // one to three frames of menu-on-black on every boot - the flicker on
        // the START press. Seeding the planes here closes it, and `shown` is
        // seeded to match so the draw list names the frame that is really in
        // them. **`escape` -> menus carries this too, off `held_menu_backdrop`
        // rather than a live stage** - it carries no playhead, which restarts
        // deliberately (see `menu_playhead`), but the picture that playhead
        // last showed is not lost with it.
        let frontend_held = match &self.stage {
            Stage::Frontend(stage) => stage.held_backdrop.as_ref(),
            _ => None,
        };
        let is_race = matches!(self.stage, Stage::Race(_));
        let seed = backdrop_seed(frontend_held, is_race, self.held_menu_backdrop.as_ref());
        if let Some(held) = &seed {
            renderer.upload_frame(&self.gpu.queue, &held.picture)?;
        }
        let seeded = seed.as_ref().map(|held| held.index);
        // Past the last fallible step, so a renderer that could not be built
        // leaves the front end holding its own backdrop rather than stripped of
        // one it is still drawing.
        let carried = match &mut self.stage {
            Stage::Frontend(stage) => stage.frontend.take_backdrop(),
            _ => None,
        };
        if carried.is_none()
            && let Some(feed) = self.backdrop.as_mut()
        {
            feed.restart();
        }
        // Replaced rather than plainly assigned, so the outgoing stage passes
        // through a binding instead of being dropped in place: a race that had
        // not finished is parked below rather than lost. See
        // `Session::suspended_race`.
        let outgoing = std::mem::replace(
            &mut self.stage,
            Stage::Menu(Box::new(MenuStage {
                renderer,
                menu: model,
                skin,
                text_atlas,
                default_atlas: shell.font.clone(),
                frame: shell.frame.clone(),
                nav_legend: shell.nav_legend.clone(),
                ticker: shell.ticker.clone(),
                // Free-running from zero on every open, the same choice
                // `CampaignStage::ticker_elapsed` already makes and for the
                // same reason: nothing measured suggests the original resets
                // its phase on a menu open, so there is nothing to seed this
                // from instead.
                ticker_elapsed: 0.0,
                marquee: marquee::Timer::default(),
                // Opening the menus is not a page change: the front end's own
                // hand-off already had its moment, and starting a transition
                // here would zoom the first page in from nothing on every boot.
                change: None,
                // Nothing is being typed or confirmed the instant the menus
                // open, and a prompt carried across a rebuild would be one
                // asking about a page that is no longer on screen.
                prompt: None,
                picker: None,
                campaign: None,
                styled,
                backdrop: shape.map(|shape| Backdrop {
                    player: menu_playhead(carried, frames, shape.frame_rate),
                    rect: shape.rect,
                    shown: seeded,
                    held: seed,
                }),
            })),
        );
        // **Finished is excluded deliberately.** A player leaving the results
        // table is not pausing - there is nothing there to resume back into -
        // so that race is discarded exactly as it always was. Only a race still
        // running when `escape` reached here is worth keeping.
        if let Stage::Race(stage) = outgoing
            && !stage.race.finished()
        {
            self.suspended_race = Some(stage);
        }
        self.gpu
            .window
            .set_title(&hints::shell_title(&strings::project_table(
                self.settings.language.as_deref(),
            )));
        Ok(())
    }

    /// Puts every setting the menus can edit onto the row that edits it.
    ///
    /// A key nothing edits is reported rather than ignored: it means a setting
    /// exists that a player has no way to change, which is a gap worth seeing in
    /// the log rather than a silent one.
    fn seed_menu(
        &self,
        model: &mut menu::Menu,
        title: &'static oag_title::Title,
        platform: oag_disc::Platform,
    ) {
        for (key, value) in settings::menu_seeds(&self.settings, self.anisotropy, title, platform) {
            // Rows this platform drops (`prepare.rs`) are not a gap.
            let dropped = if cfg!(target_arch = "wasm32") {
                settings::web::HIDDEN_ROWS.contains(&key) || key == "display.window_size"
            } else {
                settings::web::WEB_ONLY_ROWS.contains(&key)
            };
            if !model.seed(key, &value) && !dropped {
                warn!("nothing in the menus edits {key}");
            }
        }
    }

    /// The mode `race.mode` currently names, defaulting the way the menu's
    /// own row does when the setting names one this build does not have -
    /// see `oag_race::Mode::from_name`'s own docs. Used to pick which list
    /// the CIRCUIT row shows; a race actually being launched reads the
    /// resolved [`oag_race::Options::mode`] instead, which additionally
    /// preserves an in-flight value across an unrecognised token rather than
    /// defaulting it.
    pub(crate) fn race_mode(&self) -> oag_race::Mode {
        oag_race::Mode::from_name(&self.settings.race.mode).unwrap_or_default()
    }

    /// Re-supplies the CIRCUIT row from whichever list the current
    /// `race.mode` names, without touching any other row.
    ///
    /// **This is the fix for the menu offering all 24 circuits in Zone
    /// mode.** The row's list was supplied once in [`Self::open_menus`] and
    /// nothing told it again when MODE changed - `Menu::supply` only runs at
    /// menu-open, and a `Menu::seed` on `race.mode` changes what the row is
    /// *set to*, never what it may be set to. Called from
    /// [`Session::apply_setting`] whenever `race.mode` is the setting that
    /// changed, so this is the one other place `ValueSource::Tracks` is
    /// supplied outside a fresh menu build - and it has to pick the same list
    /// [`Self::open_menus`] would have, which is what [`Shell::tracks_for`]
    /// is for.
    ///
    /// A no-op when the menus are not open (no `Stage::Menu`) or nothing has
    /// loaded a shell yet - both real states elsewhere in this module, not
    /// oversights here.
    pub(crate) fn resupply_tracks_for_mode(&mut self) {
        let mode = self.race_mode();
        let Some(shell) = &self.shell else { return };
        let tracks: Vec<menu::Choice> = shell
            .tracks_for(mode)
            .iter()
            .map(|(track, name)| menu::Choice::labelled(&track.id, name))
            .collect();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(menu::ValueSource::Tracks, &tracks);
        }
    }

    /// Re-supplies the RACE page's own VARIANT row from the booted title's
    /// [`oag_title::RaceDefaults::team_variants`], scoped to whichever team
    /// `race.team` currently names. A no-op when the menus are not open or
    /// nothing has loaded a shell yet - see [`Self::resupply_tracks_for_mode`].
    pub(crate) fn resupply_race_variant(&mut self) {
        let Some(shell) = &self.shell else { return };
        let choices =
            gated_variant_choices(shell.title, &self.settings.race.team, &shell.team_details);
        remix_menu::settle(&mut self.settings.race.variant, &choices);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(menu::ValueSource::RaceVariant, &choices);
        }
    }

    /// Acts on one thing the menus did.
    pub(crate) fn handle_menu(&mut self, event: &menu::MenuEvent) {
        match event {
            menu::MenuEvent::Changed { setting, value } => self.apply_setting(setting, value),
            menu::MenuEvent::Fired(menu::Action::LaunchRace) => {
                // Through the race box's own screens first - Track Select,
                // then Ship Select - and only straight to the race on a title
                // that authors neither. See `session::picker`. Wipeout
                // HD/Fury authors both, in files of their own
                // (`oag_title::FrontEnd::track_select`/`team_select`), and
                // takes the same path: `Track Creation` redirects to `Team
                // Selection` by default. Zone skips the ship screen either
                // way, since it forces its own hull.
                let zone = self.race_mode() == oag_race::Mode::Zone;
                if self.open_track_picker() || (!zone && self.open_ship_picker()) {
                    return;
                }
                self.launch_from_settings();
            }
            // The RACE REMIX page's own launch - see `menu::Action::LaunchRemix`'s
            // own doc comment for why it is not a second meaning for
            // `LaunchRace` above. `Self::launch_remix` builds `race::Options`
            // the same way that one does, plus the two source fields neither
            // the CLI's `--race` route nor the ordinary RACE page ever has to
            // resolve.
            menu::MenuEvent::Fired(menu::Action::LaunchRemix) => self.launch_remix(),
            // QUIT always quits, parked race or not - it is a row a player
            // chose deliberately, not a fall-through.
            menu::MenuEvent::Fired(menu::Action::Quit) => {
                self.quit = true;
            }
            // The AI PILOTS page's own two actions - see `super::pilot_editor`.
            menu::MenuEvent::Fired(menu::Action::SavePilot) => self.save_pilot(),
            menu::MenuEvent::Fired(menu::Action::NewPilot) => self.new_pilot(),
            // Both open a modal prompt rather than doing anything: what they
            // fire is `MenuStage::prompt`, and `Session::finish_prompt` acts
            // on the answer. See `session::pilot_editor`.
            menu::MenuEvent::Fired(menu::Action::RenamePilot) => self.rename_pilot(),
            menu::MenuEvent::Fired(menu::Action::DeletePilot) => self.delete_pilot(),
            // RACE CAMPAIGN - see `session::campaign`.
            menu::MenuEvent::Fired(menu::Action::OpenCampaign) => self.open_campaign(),
            // Backing out of the root page used to always mean the same thing
            // as QUIT: there was nothing behind the menus to go back to. Now
            // there can be - a race `escape` parked rather than discarded -
            // and backing all the way out is the fourth place `escape`'s own
            // "one rule, three places it lands" doc comment lands: resuming
            // it, the same way stepping back onto a page still under it just
            // shows that page. Only when nothing is parked does this still
            // mean the desktop is behind the root page, exactly as before.
            menu::MenuEvent::Closed => {
                if self.suspended_race.is_some() {
                    self.resume_race();
                } else {
                    self.quit = true;
                }
            }
        }
    }

    // `Session::resume_race` and `Session::escape` are their own file now -
    // `session/escape.rs` - under the 1,000-line rule in
    // `scripts/check-file-size.py`. A move, with no behaviour change; see
    // that file's own doc for why it is the natural seam, not an arbitrary
    // cut.
}

impl Session {
    /// Starts the race the `race.*` settings describe - what the RACE page's
    /// START row did outright before the selection screens sat between.
    pub(crate) fn launch_from_settings(&mut self) {
        // The stored id names a *race*, and only this source can say
        // which file that is.
        //
        // **A source that does not offer the stored circuit races the
        // first one it *does* offer**, which is what the menu is already
        // showing. Keeping the previous option instead - which is what
        // this did - meant the menu displayed one circuit and the race
        // loaded another: `Menu::supply` resets the visible row to index
        // 0 when the stored id is gone, and nothing told the options.
        // The failure was then a missing archive entry naming a circuit
        // that was never on screen.
        //
        // Not a Pure bug, though Pure is where it became unmissable (no
        // circuit id is shared between the titles, so *every* stored
        // Pulse circuit misses): a settings file naming a pack circuit
        // the player no longer has takes the same path on Pulse.
        // Taken out and put back rather than reached through, so the
        // settings below can be read while it is being written. There
        // are no menus before a disc has been chosen, so the `None`
        // here is unreachable in practice and says so rather than
        // unwrapping.
        // Defensive, not load-bearing on the ordinary path: a campaign
        // launch abandoned mid-`Team Selection` (`Session::handle_picker`'s
        // `Kind::Ship` `Event::Back` arm) already clears
        // `self.campaign_cell`, but clearing it here too means a RACE-page
        // launch can never inherit a stale cell whatever path reached this
        // function.
        self.campaign_cell = None;
        self.campaign_difficulty = None;
        self.campaign_ai_skill_scale = None;
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        // The campaign writes both fields - see
        // `Session::launch_campaign_cell` - and a RACE-page launch must
        // never inherit one left over from a campaign cell that was backed
        // out of before racing. The kill target is then set from the KILLS
        // row below.
        race_options.eliminator_kill_target = None;
        race_options.laps_override = None;
        // Resolved before the circuit below, and out of its usual
        // order beneath the class and difficulty rows: which list a
        // stored circuit is checked against - the race one or the
        // Zone one - depends on it, the same way the CIRCUIT row
        // itself switches lists on this setting. An unrecognised
        // token leaves the previous mode in place rather than
        // substituting one, so a settings file from a build with a
        // mode this one does not have still races - and still checks
        // the circuit against whichever list that leftover mode
        // implies.
        if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
            race_options.mode = mode;
        }
        // The WEAPONS row's pick, for a single race only.
        race_options.weapons_override = self.settings.race.weapons_override(race_options.mode);
        // The KILLS row's pick, for an Eliminator only.
        race_options.eliminator_kill_target =
            self.settings.race.eliminator_kill_target(race_options.mode);
        let chosen = self.shell.as_ref().and_then(|shell| {
            shell
                .track(race_options.mode, &self.settings.race.track)
                .or_else(|| {
                    warn!(
                        "this source does not offer {:?}; racing the first \
                                 circuit it does offer, which is what the menu shows",
                        self.settings.race.track
                    );
                    shell
                        .tracks_for(race_options.mode)
                        .first()
                        .map(|(track, _)| track)
                })
                .map(catalogue::Track::entry_name)
        });
        match chosen {
            Some(entry) => race_options.track = Some(entry),
            None => warn!(
                "this source offers no circuit at all; racing whatever was \
                         already selected"
            ),
        }
        // The same shape as the circuit above, and needed for the same
        // reason now that the roster is what this source offers rather
        // than a fixed list: a stored team can be one only a
        // downloadable pack carries, and a boot that did not find the
        // pack drops it. `Menu::supply` resets the row silently when
        // that happens, so without this check the menu would show one
        // team and the race would attempt another - failing at the
        // archive with a message naming a team that is not on screen.
        // `team` and `title` together, in the same call: VARIANT
        // combines with whichever title's own axis this is. Shared with the
        // campaign's own launch - see `team::apply_race_team`.
        self.apply_race_team(&mut race_options);
        // Carried as a **name**, and checked against this page's own
        // row - see [`resolve_race_page_class`] for why: `race.class`
        // is one setting shared with RACE REMIX, so a class settled
        // there (`VECTOR`, offered only in remix) can sit in storage
        // while this page's row was never on it. An empty setting
        // still leaves the previous class in place, as it always did.
        if let Some(class) = resolve_race_page_class(
            self.shell.as_ref().map(|shell| shell.title),
            self.settings.race.class.trim(),
        ) {
            race_options.class = class;
        }
        // The mode itself was already resolved above, before the
        // circuit - see the comment there.
        // Same shape again: an unrecognised level leaves the previous
        // one in place rather than substituting a default mid-session.
        if let Some(difficulty) = oag_ai::Difficulty::from_name(&self.settings.ai.difficulty) {
            race_options.difficulty = difficulty;
        }
        info!(
            "loading {}",
            race_options
                .track
                .as_deref()
                .unwrap_or("this source's own default circuit")
        );
        // Back before the load, which reads it.
        self.race_options = Some(race_options);
        self.finish_launch();
    }

    /// Writes the team, variant and livery `Team Selection` last picked
    /// (`settings.race`) into `race_options`, and logs why when this source
    /// does not offer that team. A no-op with no shell (the `--race` path,
    /// which has no menus to have picked anything). See
    /// [`team::apply_race_team`].
    pub(crate) fn apply_race_team(&self, race_options: &mut oag_raceplay::Options) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        for warning in
            team::apply_race_team(shell.title, &shell.teams, &self.settings.race, race_options)
        {
            warn!("{warning}");
        }
    }

    /// [`Self::launch_race`], plus the same hand-off report on success and
    /// the same non-fatal log on failure every launcher wants - the RACE
    /// page's own [`Self::launch_from_settings`] above and a campaign cell's
    /// [`Self::launch_campaign_cell`] (`crate::main::session::campaign`)
    /// alike. Neither builds `self.race_options` here: by the time either
    /// calls this, it is already exactly what should load.
    pub(crate) fn finish_launch(&mut self) {
        match self.launch_race() {
            Ok(()) => {
                let strings = strings::project_table(self.settings.language.as_deref());
                println!(
                    "\n{}{}",
                    hints::race_keys(&strings),
                    hints::esc_to_menu(&strings)
                );
            }
            // Reported rather than fatal: leaving the menus on screen
            // lets the player pick something else, where a vanished
            // window would just look like a crash.
            Err(e) => error!("cannot start a race: {e:#}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names a call to [`speed_class_choices`] actually offers, in the
    /// order they were supplied - `menu::Choice::label` is the disc's own
    /// spelling, the same string [`to_choices`] wrote in.
    fn offered(title: &'static oag_title::Title) -> Vec<String> {
        speed_class_choices(title)
            .into_iter()
            .map(|choice| choice.label)
            .collect()
    }

    /// **The confinement, on the title that made it matter.** Wipeout Pure's
    /// own ladder carries `VECTOR`, and this build can race it - but the
    /// ordinary RACE page is not where that is offered. See
    /// `oag_title::SpeedClasses::is_offered_outside_remix` and
    /// `docs/architecture/menus.md` for why.
    #[test]
    fn a_pure_boots_race_page_offers_four_and_not_vector() {
        let names = offered(oag_pure::TITLE);

        assert_eq!(names.len(), 4);
        assert!(
            !names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(oag_title::SpeedClasses::VECTOR)),
            "VECTOR is a RACE REMIX offering, not this page's: {names:?}"
        );
    }

    /// Pulse never authored a fifth rung on its own per-team files, so this
    /// page's count is unchanged from before the confinement existed - the
    /// fix narrows Pure's page, not Pulse's.
    #[test]
    fn a_pulse_boots_race_page_still_offers_four() {
        assert_eq!(offered(oag_pulse::TITLE).len(), 4);
    }

    /// **The hazard `resolve_race_page_class` exists for.** `race.class` is
    /// shared with RACE REMIX, whose own row can settle it to `"vector"` -
    /// this page's row cannot, since the confinement above took it out. A
    /// stored `"vector"` reaching this page's own launch must not survive
    /// unclamped: it must fall back to the same first entry the row itself
    /// would show, not launch a class the row never displayed.
    #[test]
    fn a_stored_vector_on_a_pure_boot_clamps_to_the_race_pages_own_first_entry() {
        let resolved = resolve_race_page_class(Some(oag_pure::TITLE), "vector")
            .expect("a non-empty stored class always resolves to something");

        assert_eq!(resolved, "venom", "the row's own first entry, not vector");
        assert!(
            offered(oag_pure::TITLE)
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&resolved)),
            "whatever this launches must be a name the row actually shows"
        );
    }

    /// A stored class this page *does* offer resolves to itself, unchanged -
    /// the ordinary case, so the clamp above never fires when nothing is
    /// actually wrong.
    #[test]
    fn a_stored_class_the_page_already_offers_resolves_unchanged() {
        assert_eq!(
            resolve_race_page_class(Some(oag_pure::TITLE), "phantom").as_deref(),
            Some("phantom")
        );
        // Case-insensitively, the same as every other rung lookup here.
        assert_eq!(
            resolve_race_page_class(Some(oag_pure::TITLE), "PHANTOM").as_deref(),
            Some("phantom")
        );
    }

    /// An empty stored value means "leave the previous class in place" and
    /// must stay `None` - the pre-existing rule this check does not change.
    #[test]
    fn an_empty_stored_class_resolves_to_nothing() {
        assert_eq!(resolve_race_page_class(Some(oag_pure::TITLE), ""), None);
    }

    /// With no shell at all (unreachable in practice - there are no menus
    /// before a disc has been chosen) the stored value is carried through
    /// unclamped, since there is no row to check it against.
    #[test]
    fn with_no_title_the_stored_class_passes_through_unclamped() {
        assert_eq!(
            resolve_race_page_class(None, "vector").as_deref(),
            Some("vector")
        );
    }
}
