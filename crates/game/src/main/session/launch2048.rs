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

use log::info;

use oag_ui::frontend::Launch;

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Acts on the front end's request, with the menus already open.
    pub(crate) fn follow_launch(&mut self, launch: Option<Launch>) {
        match launch {
            None => {}
            Some(Launch::Event(name)) => {
                info!("Launch 2048: starting campaign event {name:?}");
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
        info!("Launch 2048: opening this build's {id} page");
        if let Stage::Menu(stage) = &mut self.stage
            && !stage.menu.push(id)
        {
            log::warn!("assets/ui/menu.toml has no page {id:?}; staying on the root");
        }
    }
}
