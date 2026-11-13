//! The attract demo in the windowed session: an untouched main menu starts one, any input ends it
//! (`docs/ui/attract-demo.md`).
//!
//! The flow itself is `oag_game::demo`; this is the part that needs a stage. A demo is launched
//! through the ordinary race launch (`Session::launch_race`) on a copy of the options, so the
//! player's settings and `race_options` are never touched, and handed over at
//! `Session::advance_race_build` with every slot flown by the AI (`Race::fly_as_demo`). Its race
//! stage never saves a result, a ghost or a recording, and is never parked behind the menus.

use log::{debug, info, warn};

use oag_game::demo::{self, Leave, Next, Run};
use oag_game::records;
use oag_ui::pointer::Pointer;

use crate::stage::Stage;

use super::Session;

/// What the session keeps of the attract demo between ticks.
#[derive(Default)]
pub(crate) struct State {
    attract: demo::Attract,
    /// `--demo` / `--wallpaper`: the run is a chain of demos. `Some(true)` is a wallpaper.
    pub(crate) cli: Option<bool>,
    /// The demo being loaded, until its race stage is live.
    pub(crate) loading: Option<Run>,
    /// The player's `race_options`, set aside while a demo's stand in their place.
    saved: Option<oag_raceplay::Options>,
    /// The last demo's pick, so the next differs.
    previous: Option<demo::Pick>,
    /// How many demos this run has started.
    started: u32,
    /// The seed the chain's picks come from, drawn once.
    seed: Option<u64>,
    /// A key went down since the last tick: any key, bound or not.
    pub(crate) key: bool,
}

impl Session {
    /// Whether the race on screen is a demo.
    pub(crate) fn in_demo(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage) if stage.demo.is_some())
    }

    /// Whether the demo on screen, or loading, ignores input: a wallpaper.
    pub(crate) fn demo_is_wallpaper(&self) -> bool {
        match &self.stage {
            Stage::Race(stage) => stage.demo.as_ref().is_some_and(Run::wallpaper),
            _ => self.demo.loading.as_ref().is_some_and(Run::wallpaper),
        }
    }

    /// Whether the main menu is untouched territory: the title's main page with nothing over it
    /// and nothing parked behind it.
    fn attract_eligible(&self) -> bool {
        let Stage::Menu(stage) = &self.stage else {
            return false;
        };
        stage.menu.depth() == 1
            && !stage.on_pause_page()
            && stage.prompt.is_none()
            && stage.picker.is_none()
            && stage.campaign.is_none()
            && stage.pure_chain.is_none()
            && stage.join.is_none()
            && stage.lan.is_none()
            && stage.layout.is_none()
            && stage.assign.is_none()
            && self.suspended_race.is_none()
            && self.awaiting_binding.is_none()
            && self.net.is_none()
            && self.split_lobby.is_none()
            && self.viewer.is_none()
            && self.online.is_none()
    }

    /// One tick of the demo's flow, before the stage's own tick. Returns whether the stage
    /// changed, so the caller stops stepping a tick that belongs to the old one.
    pub(crate) fn tick_demo(&mut self, pointer: &Pointer) -> bool {
        let Some(title) = self.shell.as_ref().map(|shell| shell.title) else {
            return false;
        };
        let key = std::mem::take(&mut self.demo.key);
        // A button press, any key, the mouse or a finger. **Not `pad_spoke`**, which also reads a
        // stick off centre: 2 s of full stick left the original's demo running (measured).
        let input = key || self.controls.buttons().pressed_mask() != 0 || !pointer.is_idle();
        match &self.stage {
            Stage::Menu(_) => {
                let eligible = self.attract_eligible();
                let idle = self.demo_timing(title).idle;
                let launch = self.demo.attract.tick(idle, eligible, input);
                // A `--demo` run opens the menus once and launches from them.
                let cli_launch = eligible && self.demo.cli.is_some() && self.demo.loading.is_none();
                if (launch || cli_launch) && self.launch_demo() {
                    return true;
                }
            }
            Stage::Loading(_) => {
                if let Some(run) = self.demo.loading.as_mut() {
                    run.hear(input);
                }
            }
            Stage::Race(stage) if stage.demo.is_some() => {
                let finished = stage.race.finished();
                let length = self.demo_timing(title).length;
                let Stage::Race(stage) = &mut self.stage else {
                    return false;
                };
                let Some(run) = stage.demo.as_mut() else {
                    return false;
                };
                run.hear(input);
                if let Some(leave) = run.tick(length, finished) {
                    self.end_demo(leave);
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// The title's demo timing, with its authored `DemoConfig.xml` over it.
    fn demo_timing(&self, title: &'static oag_title::Title) -> demo::Timing {
        let config = self.shell.as_ref().and_then(|shell| shell.demo_config.as_ref());
        demo::Timing::of(&title.demo, config)
    }

    /// Escape in a demo: a wallpaper quits, any other demo is left like any other input.
    pub(crate) fn escape_demo(&mut self) {
        if self.demo_is_wallpaper() {
            self.quit = true;
        } else {
            self.end_demo(Leave::Input);
        }
    }

    /// Starts a demo race. Returns whether the loading screen is up.
    pub(crate) fn launch_demo(&mut self) -> bool {
        let Some((options, title)) = self.demo_options() else {
            // Nothing to pick from, or no disc: a `--demo` run has nothing to chain.
            self.demo.cli = None;
            self.restore_race_options();
            return false;
        };
        let player = self.race_options.replace(options);
        // The first demo of a chain keeps the player's own; later ones replace a demo's.
        if self.demo.saved.is_none() {
            self.demo.saved = player;
        }
        self.campaign_cell = None;
        self.campaign_custom = None;
        self.campaign_difficulty = None;
        self.campaign_ai_skill_scale = None;
        self.demo.loading = Some(Run::new(self.demo.cli == Some(true)));
        info!("{}: starting an attract demo", title.name);
        match self.launch_race() {
            Ok(()) => {
                self.demo.started += 1;
                true
            }
            Err(error) => {
                warn!("cannot start an attract demo: {error:#}");
                self.demo.loading = None;
                self.demo.cli = None;
                self.restore_race_options();
                false
            }
        }
    }

    /// The options of the next demo: a Single Race on a circuit and team the player can already
    /// pick, drawn by the title's selection rule.
    fn demo_options(&mut self) -> Option<(oag_raceplay::Options, &'static oag_title::Title)> {
        let gate = self.landed_menu_source().map(|(_, gate)| gate);
        let shell = self.shell.as_ref()?;
        let title = shell.title;
        let mode = oag_race::Mode::SingleRace;
        let campaign_title = records::campaign_title(title, shell.platform);
        let tracks: Vec<_> = shell
            .tracks_for(mode)
            .iter()
            .map(|(track, _)| track)
            .filter(|track| {
                demo::unlocks_ignored(&title.demo)
                    || match &gate {
                        Some(gate) => gate.offers(track, &self.records, &campaign_title),
                        None => track.unlock_grid.is_none(),
                    }
            })
            .collect();
        // A title that gates its teams on the campaign (Pure) offers only the player's own pick,
        // because reading what is unlocked would open an archive on this thread.
        let teams: Vec<&str> = if title.campaign.list_entry.is_some() {
            shell
                .teams
                .iter()
                .map(|choice| choice.value.as_str())
                .filter(|team| *team == self.settings.race.team)
                .collect()
        } else {
            let all: Vec<(&str, bool)> = shell
                .teams
                .iter()
                .map(|choice| {
                    let marked = shell
                        .team_details
                        .iter()
                        .any(|team| team.id == choice.value && team.multiplayer);
                    (choice.value.as_str(), marked)
                })
                .collect();
            demo::offered_teams(&title.demo, &all)
        };
        let seed = *self.demo.seed.get_or_insert_with(|| oag_game::race_seed::fresh());
        let mut pick = demo::pick(
            &title.demo,
            demo::chain_seed(seed, self.demo.started),
            tracks.len(),
            teams.len(),
            self.demo.previous,
        )?;
        // A title that authors its circuit cycle (`DemoConfig.xml`) flies that, in order.
        let circuits: Vec<(&str, &str)> = tracks
            .iter()
            .map(|track| (track.id.as_str(), track.location.as_str()))
            .collect();
        let config = shell.demo_config.as_ref();
        if let Some(track) = demo::authored_circuit(config, &circuits, self.demo.started) {
            pick.track = track;
        }
        let mut options = self.race_options.clone()?;
        options.mode = mode;
        options.track = Some(tracks[pick.track].entry_name());
        let race = oag_game::settings::Race {
            team: teams[pick.team].to_string(),
            variant: String::new(),
            skin: String::new(),
            ..self.settings.race.clone()
        };
        for warning in super::menus::team::apply_race_team(title, &shell.teams, &race, &mut options)
        {
            warn!("{warning}");
        }
        options.laps_override = None;
        options.weapons_override = None;
        options.eliminator_kill_target = None;
        options.eliminator_score_target = None;
        options.seat_picks = Vec::new();
        debug!(
            "attract demo {}: {} as {}",
            self.demo.started + 1,
            tracks[pick.track].id,
            teams[pick.team]
        );
        self.demo.previous = Some(pick);
        Some((options, title))
    }

    /// Puts the player's `race_options` back where a demo's stood.
    pub(crate) fn restore_race_options(&mut self) {
        if let Some(saved) = self.demo.saved.take() {
            self.race_options = Some(saved);
        }
    }

    /// A demo is over: on to the next, or back to the main menu.
    fn end_demo(&mut self, leave: Leave) {
        let Some(title) = self.shell.as_ref().map(|shell| shell.title) else {
            return;
        };
        let chain = self.demo.cli.is_some();
        let next = match leave {
            Leave::Input => Next::Menu,
            Leave::Over => demo::after_over(&title.demo, chain),
        };
        if leave == Leave::Input {
            // The person at the machine has taken it back: no chain starts again behind them.
            self.demo.cli = None;
        }
        debug!("attract demo over ({leave:?}), next: {next:?}");
        if next == Next::Demo && self.launch_demo() {
            return;
        }
        self.audio.pause_race_music();
        self.audio.stop_race_sfx();
        self.restore_race_options();
        if let Err(error) = self.open_menus() {
            log::error!("cannot return to the menus: {error:#}");
            self.quit = true;
        }
    }
}

impl State {
    /// The state of a run started with `--demo` (`wallpaper`: `--wallpaper` too), or of a plain one.
    pub(crate) fn with_cli(cli: Option<bool>) -> Self {
        Self {
            cli,
            ..Self::default()
        }
    }
}
