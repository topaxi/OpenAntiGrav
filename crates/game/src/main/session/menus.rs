//! Opening the menus, and what a menu event does.

use anyhow::{Context, Result};
use log::{error, info, warn};

use oag_game::render::{Renderer, VideoFormat};
use oag_game::{audio, boot, catalogue, display, marquee, menu, movie, remix, settings};
use oag_physics::SpeedClass;

use crate::frontend_stage::HeldFrame;
use crate::hints::{ESC_TO_MENU, RACE_KEYS, RACE_TITLE, SHELL_KEYS, SHELL_TITLE};
use crate::menu_stage::{Backdrop, MenuStage, menu_playhead};
use crate::stage::Stage;
use crate::window::monitor_names;

use super::Session;

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

/// A remix catalogue's tracks, in the shape `Menu::supply` wants - the same
/// `(Track, label)` -> `Choice::labelled` mapping [`Session::open_menus`]
/// applies to a booted title's own `Shell::tracks`, pulled out because the
/// RACE REMIX page needs it at two call sites and `Session::open_menus`'s
/// own list is a different, already-in-scope local of the same shape.
fn remix_track_choices(catalogue: &remix::Catalogue) -> Vec<menu::Choice> {
    catalogue
        .tracks
        .iter()
        .map(|(track, name)| menu::Choice::labelled(&track.id, name))
        .collect()
}

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
        let skin = menu::Skin::new(shell.menu_skin, shell.space, rows_face.line_height);
        model.set_visible_rows(menu::visible_rows(&skin));
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
        model.supply(menu::ValueSource::Languages, &shell.languages);
        model.supply(menu::ValueSource::RaceModes, &shell.modes);
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
        // the moment for that. See `audio::MusicDiscs`.
        let music_sources: Vec<menu::Choice> = if self.music_discs.both() {
            audio::MusicSource::ALL
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
        // Race Remix's own axis - every title this machine can currently open
        // a source for, surveyed once at `Session` construction. See the
        // field's own doc comment.
        let titles: Vec<menu::Choice> = self
            .titles
            .iter()
            .map(|candidate| menu::Choice::plain(candidate.title()))
            .collect();
        model.supply(menu::ValueSource::Titles, &titles);
        // Scoped to whichever titles `remix.track_title`/`remix.craft_title`
        // already name, the same reason the CIRCUIT row above is seeded from
        // `race.mode` rather than left empty until MODE is next touched: a
        // menu reopened on a saved pick should show that pick's own list from
        // the first frame. Both are `None` and supply nothing when nothing is
        // saved yet, or when the saved title's source is no longer on this
        // machine's search path - see `Self::remix_catalogue_for`.
        if let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.track_title) {
            model.supply(
                menu::ValueSource::RemixTracks,
                &remix_track_choices(&catalogue),
            );
        }
        if let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.craft_title) {
            model.supply(menu::ValueSource::RemixTeams, &catalogue.teams);
        }
        self.seed_menu(&mut model);
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
        // **Not a single value.** Unlike the renderer, only the MSAA half of
        // this row is baked into the scene's pipelines - `upscale::Framebuffer::resolve`
        // reads FXAA/SMAA/off fresh every frame, the same way it already
        // reads the upscaler. So every mode that shares the built scene's
        // sample count is equally "in effect": a race built at `off` can move
        // to `fxaa` or back with no restart note, and only moving to or
        // between an MSAA tier the scene was not built with earns one.
        if let Stage::Race(stage) = &self.stage {
            let built = stage.scene.anti_aliasing();
            let live_equivalent: &[display::AntiAliasing] = if built.msaa_samples() == 1 {
                &[
                    display::AntiAliasing::Off,
                    display::AntiAliasing::Fxaa,
                    display::AntiAliasing::Smaa,
                ]
            } else {
                std::slice::from_ref(&built)
            };
            let in_use: Vec<menu::Value> = live_equivalent
                .iter()
                .map(|mode| menu::Value::Text(mode.to_string()))
                .collect();
            if !model.in_effect("graphics.anti_aliasing", &in_use) {
                warn!("nothing in the menus defers graphics.anti_aliasing");
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
                frame: shell.frame.clone(),
                marquee: marquee::Timer::default(),
                // Opening the menus is not a page change: the front end's own
                // hand-off already had its moment, and starting a transition
                // here would zoom the first page in from nothing on every boot.
                change: None,
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
        self.gpu.window.set_title(SHELL_TITLE);
        Ok(())
    }

    /// Puts every setting the menus can edit onto the row that edits it.
    ///
    /// A key nothing edits is reported rather than ignored: it means a setting
    /// exists that a player has no way to change, which is a gap worth seeing in
    /// the log rather than a silent one.
    fn seed_menu(&self, model: &mut menu::Menu) {
        for (key, value) in settings::menu_seeds(&self.settings, self.anisotropy) {
            if !model.seed(key, &value) {
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
    fn race_mode(&self) -> oag_race::Mode {
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

    /// The circuits and roster whichever title `title_setting` currently
    /// names, if this machine can still open it.
    ///
    /// `None` covers three real states rather than one: the setting is empty
    /// (nothing picked yet), it names a title `Self::titles` no longer has
    /// (the disc left the search path since it was saved), or opening the
    /// title failed once it was tried - the last of which is reported, the
    /// first two are not, since an unpicked or since-removed title is not a
    /// fault. `crate::remix::catalogue` does the actual open; this only finds
    /// which source to hand it.
    fn remix_catalogue_for(&self, title_setting: &str) -> Option<remix::Catalogue> {
        let candidate = self
            .titles
            .iter()
            .find(|candidate| candidate.title() == title_setting)?;
        match remix::catalogue(&candidate.source) {
            Ok(catalogue) => Some(catalogue),
            Err(e) => {
                warn!("{}: {e:#}", candidate.source);
                None
            }
        }
    }

    /// Re-supplies the RACE REMIX page's TRACK row from whichever title
    /// `remix.track_title` currently names - the `remix.track_title`-changed
    /// counterpart to [`Self::resupply_tracks_for_mode`]. A no-op when the
    /// menus are not open or [`Self::remix_catalogue_for`] found nothing.
    pub(crate) fn resupply_remix_tracks(&mut self) {
        let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.track_title) else {
            return;
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(
                menu::ValueSource::RemixTracks,
                &remix_track_choices(&catalogue),
            );
        }
    }

    /// [`Self::resupply_remix_tracks`]' sibling for TEAM, scoped to
    /// `remix.craft_title` instead.
    pub(crate) fn resupply_remix_teams(&mut self) {
        let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.craft_title) else {
            return;
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage
                .menu
                .supply(menu::ValueSource::RemixTeams, &catalogue.teams);
        }
    }

    /// Acts on one thing the menus did.
    pub(crate) fn handle_menu(&mut self, event: &menu::MenuEvent) {
        match event {
            menu::MenuEvent::Changed { setting, value } => self.apply_setting(setting, value),
            menu::MenuEvent::Fired(menu::Action::LaunchRace) => {
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
                let Some(mut race_options) = self.race_options.take() else {
                    warn!("no disc image has been chosen yet, so there is nothing to race");
                    return;
                };
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
                match self
                    .shell
                    .as_ref()
                    .and_then(|shell| shell.team(&self.settings.race.team))
                {
                    Some(team) => race_options.team = Some(team.to_string()),
                    None => warn!(
                        "this source does not offer team {:?}, racing as {} instead",
                        self.settings.race.team,
                        race_options
                            .team
                            .as_deref()
                            .unwrap_or("this source's own default"),
                    ),
                }
                if let Some(class) = SpeedClass::from_name(&self.settings.race.class) {
                    race_options.class = class;
                }
                // The mode itself was already resolved above, before the
                // circuit - see the comment there.
                // Same shape again: an unrecognised level leaves the previous
                // one in place rather than substituting a default mid-session.
                if let Some(difficulty) =
                    oag_ai::Difficulty::from_name(&self.settings.ai.difficulty)
                {
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
                match self.launch_race() {
                    Ok(()) => println!("\n{RACE_KEYS}{ESC_TO_MENU}"),
                    // Reported rather than fatal: leaving the menus on screen
                    // lets the player pick something else, where a vanished
                    // window would just look like a crash.
                    Err(e) => error!("cannot start a race: {e:#}"),
                }
            }
            // The RACE REMIX page's own launch - see `Action::LaunchRemix`'s
            // own doc comment for why it is not a second meaning for
            // `LaunchRace` above. Builds `race::Options` the same way that
            // one does, plus the two source fields neither the CLI's
            // `--race` route nor the ordinary RACE page ever has to resolve.
            menu::MenuEvent::Fired(menu::Action::LaunchRemix) => {
                let Some(mut race_options) = self.race_options.take() else {
                    warn!("no disc image has been chosen yet, so there is nothing to race");
                    return;
                };
                if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
                    race_options.mode = mode;
                }
                if let Some(class) = SpeedClass::from_name(&self.settings.race.class) {
                    race_options.class = class;
                }
                if let Some(difficulty) =
                    oag_ai::Difficulty::from_name(&self.settings.ai.difficulty)
                {
                    race_options.difficulty = difficulty;
                }

                let track_title = self.settings.remix.track_title.clone();
                let Some(track_candidate) = self
                    .titles
                    .iter()
                    .find(|candidate| candidate.title() == track_title)
                    .cloned()
                else {
                    warn!("no track title chosen yet, so there is nothing to remix");
                    self.race_options = Some(race_options);
                    return;
                };
                race_options.source = track_candidate.source.clone();

                // An unpicked CRAFT TITLE means "the same as the track's",
                // on the same terms `race::Options::craft_source: None`
                // does - not a separate source opened redundantly. A picked
                // one this machine can no longer find (its disc left the
                // search path since it was saved) falls back the same way,
                // reported rather than silent.
                let craft_title = self.settings.remix.craft_title.clone();
                let craft_source = if craft_title.is_empty() {
                    None
                } else {
                    match self
                        .titles
                        .iter()
                        .find(|candidate| candidate.title() == craft_title)
                    {
                        Some(candidate) => Some(candidate.source.clone()),
                        None => {
                            warn!(
                                "{craft_title} is not a title this machine can currently \
                                 open; racing craft from {track_title} instead"
                            );
                            None
                        }
                    }
                };
                race_options.craft_source = craft_source.clone();

                match remix::catalogue(&track_candidate.source) {
                    Ok(catalogue) => {
                        let chosen = catalogue
                            .track(&self.settings.remix.track)
                            .or_else(|| {
                                warn!(
                                    "{track_title} does not offer {:?}; racing the first \
                                     circuit it does offer, which is what the menu shows",
                                    self.settings.remix.track
                                );
                                catalogue.tracks.first().map(|(track, _)| track)
                            })
                            .map(catalogue::Track::entry_name);
                        match chosen {
                            Some(entry) => race_options.track = Some(entry),
                            None => warn!(
                                "{track_title} offers no circuit at all; racing whatever \
                                 was already selected"
                            ),
                        }
                    }
                    Err(e) => warn!("{}: {e:#}", track_candidate.source),
                }

                let craft_catalogue_source = craft_source.unwrap_or(track_candidate.source);
                match remix::catalogue(&craft_catalogue_source) {
                    Ok(catalogue) => match catalogue.team(&self.settings.remix.team) {
                        Some(team) => race_options.team = Some(team.to_string()),
                        None => warn!(
                            "this craft title does not offer team {:?}, racing as its own \
                             default instead",
                            self.settings.remix.team
                        ),
                    },
                    Err(e) => warn!("{craft_catalogue_source}: {e:#}"),
                }

                info!(
                    "remixing {}",
                    race_options
                        .track
                        .as_deref()
                        .unwrap_or("this source's own default circuit")
                );
                self.race_options = Some(race_options);
                match self.launch_race() {
                    Ok(()) => println!("\n{RACE_KEYS}{ESC_TO_MENU}"),
                    Err(e) => error!("cannot start a remix race: {e:#}"),
                }
            }
            // QUIT always quits, parked race or not - it is a row a player
            // chose deliberately, not a fall-through.
            menu::MenuEvent::Fired(menu::Action::Quit) => {
                self.quit = true;
            }
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

    /// Swaps the menus for the race `escape` parked over them - the other half
    /// of [`Self::open_menus`] parking one. Reached only from
    /// [`Self::handle_menu`], on [`menu::MenuEvent::Closed`] while
    /// [`Self::suspended_race`] holds something.
    ///
    /// A no-op if nothing is parked, which cannot happen through
    /// `handle_menu`'s own guard but is cheap to make true unconditionally
    /// rather than only where it is currently checked.
    fn resume_race(&mut self) {
        let Some(stage) = self.suspended_race.take() else {
            return;
        };
        // Same reasoning as `Session::launch_race`: the outgoing `MenuStage`'s
        // last picture, so a later `escape` does not flash black waiting for
        // the restarted feed's first frame.
        self.held_menu_backdrop = match &mut self.stage {
            Stage::Menu(stage) => stage
                .backdrop
                .as_mut()
                .and_then(|backdrop| backdrop.held.take()),
            _ => None,
        };
        self.stage = Stage::Race(stage);
        // Never left mid-freeze by a pause that predates the trip through the
        // menus - see `Session::launch_race`, which resets this for the same
        // reason on a fresh race.
        self.paused = false;
        // The playlist's own resume: `pause_race_music` (called on the way
        // into the menus) saved the position and left `race_index` alone, so
        // this continues the same track rather than starting a new one - see
        // its doc comment.
        self.audio.start_race_music(
            &self.music_discs,
            self.settings.audio.music_source,
            &boot::default_audio_cache_dir(),
        );
        self.gpu.window.set_title(RACE_TITLE);
        println!("\n{RACE_KEYS}{ESC_TO_MENU}");
    }

    /// What escape does, which is **back one level** and not quit.
    ///
    /// One rule, four places it lands:
    ///
    /// - in the menus, it pops a page, exactly as circle does. On the root page
    ///   `Menu::back` raises `Closed`, which means "there is nothing behind the
    ///   menus" - which quits, unless a race is parked underneath, in which
    ///   case there is, and that fourth place is [`Self::resume_race`];
    /// - in a race, it hands the window back to the menus;
    /// - in the front end, in the disc chooser, or in a `--race` run that never
    ///   had menus, there is no level behind and it quits. The chooser is the
    ///   clearest case of the rule rather than an exception to it: it is the
    ///   first thing a run shows, so behind it is the desktop.
    ///
    /// **Leaving a race parks it, unless the race is over.** A race still
    /// running is kept in [`Session::suspended_race`] rather than dropped, and
    /// backing all the way out of the menus resumes it exactly where it was
    /// left - a player who backs out mid-race expects to be able to get back
    /// to it. A finished race is different: there is a results table behind it
    /// rather than a track, and nothing there to resume into, so that one is
    /// still discarded the way every race used to be.
    pub(crate) fn escape(&mut self) {
        // Collected before anything else touches `self`: `handle_menu` takes
        // `&mut self` and the events borrow the stage.
        if let Stage::Menu(stage) = &mut self.stage {
            let events = stage.menu.back();
            for event in events {
                self.handle_menu(&event);
            }
            return;
        }

        if matches!(self.stage, Stage::Race(_)) && self.shell.is_some() {
            info!("leaving the race");
            // Before `open_menus`, not after: the menu voice this resumes has
            // to be sounding by the time the menus themselves draw. See
            // `Audio::pause_race_music`.
            self.audio.pause_race_music();
            // The engine is a *held* voice - `~ENGINE` - so leaving the race
            // has to release it. Without this the menus hum.
            self.audio.stop_race_sfx();
            match self.open_menus() {
                Ok(()) => println!("\n{SHELL_KEYS}"),
                // Reported rather than fatal, and then it quits: a race whose
                // menus cannot be rebuilt has nothing left to offer, but a
                // window that vanished with no message would read as a crash.
                Err(e) => {
                    error!("cannot return to the menus: {e:#}");
                    self.quit = true;
                }
            }
            return;
        }

        self.quit = true;
    }
}
