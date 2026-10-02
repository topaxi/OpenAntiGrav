//! What the window does with the request Wipeout 2048's own front end
//! leaves it - see `oag_ui::frontend::Launch`.
//!
//! Every other title's boot ends on `Launch Game`, which opens this build's
//! menus at their root and stops. 2048's ends on the disc's own
//! `Launch 2048`, and it carries where the player was going: a campaign
//! event tapped on the map, or one of this build's two own tiles (RACE BOX,
//! REMIX - `oag_ui::frontend::touch`'s module docs). The menus are opened
//! first either way, so a race that fails to load has the same place to
//! fall back to as one started from the menus themselves.

use log::{debug, warn};

use oag_ui::frontend::Launch;

use crate::stage::Stage;

use super::Session;
use super::menus::combine_variant;

impl Session {
    /// Acts on the front end's request, with the menus already open.
    pub(crate) fn follow_launch(&mut self, launch: Option<Launch>) {
        match launch {
            None => {}
            Some(Launch::Event(name)) => {
                self.sync_race_team();
                debug!("Launch 2048: starting campaign event {name:?}");
                self.pending_event = Some(name);
                if let Err(e) = self.launch_race() {
                    log::error!("cannot start the campaign event: {e:#}");
                }
            }
            // Walked into rather than jumped to, so BACK from either page
            // lands on the menu root the way it does from the root's own
            // row - `oag_ui::menu::Menu::push`.
            Some(Launch::RaceBox) => self.push_page("race"),
            Some(Launch::Remix) => self.push_page("remix"),
        }
    }

    fn push_page(&mut self, id: &str) {
        debug!("Launch 2048: opening this build's {id} page");
        if let Stage::Menu(stage) = &mut self.stage
            && !stage.menu.push(id)
        {
            log::warn!("assets/ui/menu.toml has no page {id:?}; staying on the root");
        }
    }

    /// Recombines `race_options.team`/`hull_variant` from
    /// `settings.race.team`/`variant` before a direct campaign launch.
    ///
    /// **The gap this closes**: [`Self::follow_launch`]'s `Launch::Event` arm
    /// calls [`Self::launch_race`] straight off `self.race_options`, the same
    /// field the RACE page's own [`super::menus`] `LAUNCH RACE` row rebuilds
    /// from `settings.race.team`/`variant` (through [`combine_variant`])
    /// every time *that* row fires. 2048's own campaign path never visits
    /// the RACE page - a map tap goes straight from `Team` to `Launch 2048` -
    /// so without this, a craft picked on `Team` (`Session::frame`'s own
    /// `team_choice()` read, which lands in `settings.race` just before this
    /// runs) would sit in `settings` while `race_options.team` kept
    /// whichever team the source booted with. `oag_2048::campaign::craft`'s
    /// own restriction check reads `race_options.team`, so this has to run
    /// first for that check to see the player's real choice.
    fn sync_race_team(&mut self) {
        let Some(mut race_options) = self.race_options.take() else {
            return;
        };
        let resolved = self.shell.as_ref().and_then(|shell| {
            shell
                .team(&self.settings.race.team)
                .map(|team| (shell.title, team.to_string()))
        });
        match resolved {
            Some((title, team)) => {
                let (combined, hull_variant, warning) =
                    combine_variant(title, &team, &self.settings.race.variant);
                if let Some(warning) = warning {
                    warn!("{warning}");
                }
                race_options.team = Some(combined);
                race_options.hull_variant = hull_variant.map(str::to_string);
            }
            None => warn!(
                "this source does not offer team {:?}, racing as {} instead",
                self.settings.race.team,
                race_options
                    .team
                    .as_deref()
                    .unwrap_or("this source's own default"),
            ),
        }
        self.race_options = Some(race_options);
    }
}
