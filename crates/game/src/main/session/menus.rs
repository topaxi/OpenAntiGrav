//! Opening the menus, and what a menu event does.

use anyhow::{Context, Result};

use oag_game::render::{Renderer, VideoFormat};
use oag_game::{audio, catalogue, display, marquee, menu, movie, settings};
use oag_physics::SpeedClass;

use crate::frontend_stage::HeldFrame;
use crate::hints::{ESC_TO_MENU, RACE_KEYS, SHELL_KEYS, SHELL_TITLE};
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
        // Supplied before seeding, because a value cannot be seeded onto a list
        // that is not there yet.
        let tracks: Vec<menu::Choice> = shell
            .tracks
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
            eprintln!("note: nothing in the menus defers graphics.renderer");
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
                eprintln!("note: nothing in the menus defers graphics.anti_aliasing");
            }
            // Same story as MSAA above, but a single tier baked in at
            // `Race::start` rather than a range of equivalent modes.
            let in_use = [menu::Value::Text(stage.race.boost_fov_kick().to_string())];
            if !model.in_effect("graphics.boost_fov_kick", &in_use) {
                eprintln!("note: nothing in the menus defers graphics.boost_fov_kick");
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
        self.stage = Stage::Menu(Box::new(MenuStage {
            renderer,
            menu: model,
            skin,
            text_atlas,
            marquee: marquee::Timer::default(),
            // Opening the menus is not a page change: the front end's own
            // hand-off already had its moment, and starting a transition here
            // would zoom the first page in from nothing on every boot.
            change: None,
            backdrop: shape.map(|shape| Backdrop {
                player: menu_playhead(carried, frames, shape.frame_rate),
                rect: shape.rect,
                shown: seeded,
                held: seed,
            }),
        }));
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
                eprintln!("note: nothing in the menus edits {key}");
            }
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
                    eprintln!("no disc image has been chosen yet, so there is nothing to race");
                    return;
                };
                let chosen = self.shell.as_ref().and_then(|shell| {
                    shell
                        .track(&self.settings.race.track)
                        .or_else(|| {
                            eprintln!(
                                "this source does not offer {:?}; racing the first \
                                 circuit it does offer, which is what the menu shows",
                                self.settings.race.track
                            );
                            shell.tracks.first().map(|(track, _)| track)
                        })
                        .map(catalogue::Track::entry_name)
                });
                match chosen {
                    Some(entry) => race_options.track = Some(entry),
                    None => eprintln!(
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
                    None => eprintln!(
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
                // Same shape as the speed class above: an unrecognised token
                // leaves the previous mode in place rather than substituting
                // one, so a settings file from a build with a mode this one
                // does not have still races.
                if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
                    race_options.mode = mode;
                }
                // Same shape again: an unrecognised level leaves the previous
                // one in place rather than substituting a default mid-session.
                if let Some(difficulty) =
                    oag_ai::Difficulty::from_name(&self.settings.ai.difficulty)
                {
                    race_options.difficulty = difficulty;
                }
                println!(
                    "\nloading {}",
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
                    Err(e) => eprintln!("cannot start a race: {e:#}"),
                }
            }
            // Backing out of the root page means the same thing as choosing
            // QUIT: there is nothing behind the menus to go back to.
            menu::MenuEvent::Fired(menu::Action::Quit) | menu::MenuEvent::Closed => {
                self.quit = true;
            }
        }
    }

    /// What escape does, which is **back one level** and not quit.
    ///
    /// One rule, three places it lands:
    ///
    /// - in the menus, it pops a page, exactly as circle does. On the root page
    ///   `Menu::back` raises `Closed`, which already means "there is nothing
    ///   behind the menus", so the last one still quits;
    /// - in a race, it hands the window back to the menus;
    /// - in the front end, in the disc chooser, or in a `--race` run that never
    ///   had menus, there is no level behind and it quits. The chooser is the
    ///   clearest case of the rule rather than an exception to it: it is the
    ///   first thing a run shows, so behind it is the desktop.
    ///
    /// **Leaving a race discards it.** There is no pause and no resume - the
    /// `World` is dropped and re-entering the race loads a fresh one - and
    /// building that is a milestone of its own, not something to half-do here.
    /// A player who backs out of a race expects to lose it; one who backs out
    /// and finds a *stale* race would not.
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
            println!("\nleaving the race");
            // Before `open_menus`, not after: the menu voice this resumes has
            // to be sounding by the time the menus themselves draw. See
            // `Audio::pause_race_music`.
            self.audio.pause_race_music();
            match self.open_menus() {
                Ok(()) => println!("\n{SHELL_KEYS}"),
                // Reported rather than fatal, and then it quits: a race whose
                // menus cannot be rebuilt has nothing left to offer, but a
                // window that vanished with no message would read as a crash.
                Err(e) => {
                    eprintln!("cannot return to the menus: {e:#}");
                    self.quit = true;
                }
            }
            return;
        }

        self.quit = true;
    }
}
