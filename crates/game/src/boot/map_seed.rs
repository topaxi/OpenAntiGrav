//! The 2048 campaign map as a title's menus open it: Omega's chooser hands the
//! player to the same `oag_ui::frontend::Frontend` map Vita 2048 boots into,
//! built from the data the boot already read.

use oag_ui::frontend::{EarnedTier, Frontend, MapEvent, Placed};
use oag_ui::language::{Language, StringTable};
use oag_ui::screen::Screens;

use super::Shell;
use crate::records;

/// Everything a hosted map is built from, kept past boot because the map is
/// built when the player chooses it, not when the game starts.
#[derive(Clone, Debug)]
pub struct MapSeed {
    screens: Screens,
    strings: StringTable,
    languages: Vec<Language>,
    placements: Vec<(String, Placed)>,
    events: Vec<MapEvent>,
    face_scales: Vec<(String, f32)>,
    line_height: f32,
    space: oag_display::space::Space,
    screen: &'static str,
}

impl MapSeed {
    /// `None` on a title that hosts no map, or whose map did not load.
    #[must_use]
    pub fn of(shell: &Shell) -> Option<Self> {
        let touch = shell
            .title
            .front_end?
            .touch
            .filter(|_| shell.title.front_end.is_some_and(|fe| fe.menu.is_some()))?;
        let screens = shell.map_screens.clone()?;
        if shell.campaign_events.is_empty() {
            return None;
        }
        Some(Self {
            screens,
            strings: shell.strings.clone(),
            languages: shell.languages.clone(),
            placements: shell.sprites.entries().to_vec(),
            events: shell.campaign_events.clone(),
            face_scales: shell.face_scales.clone(),
            line_height: shell.font.line_height,
            space: shell.space,
            screen: touch.canvas_screen,
        })
    }

    /// The map, opened on the event last started and folded with the profile's
    /// medals, as `Session::finish_loading` does for 2048's own.
    #[must_use]
    pub fn open(
        &self,
        records: &records::Store,
        campaign_title: &str,
        source: &str,
        craft: String,
    ) -> Frontend {
        let mut frontend = Frontend::hosted_map(
            self.screens.clone(),
            self.strings.clone(),
            self.languages.clone(),
            self.placements.clone(),
            self.screen,
        );
        if !self.face_scales.is_empty() {
            frontend.set_face_scales(self.face_scales.clone(), self.line_height);
        }
        frontend.set_campaign(self.events.clone());
        frontend.set_space(self.space);
        frontend.refresh_campaign_progress(|name| {
            records
                .campaign_medal(campaign_title, name)
                .and_then(|row| row.best_medal)
                .map(|medal| match medal {
                    records::Medal::Gold => EarnedTier::Elite,
                    records::Medal::Silver | records::Medal::Bronze => EarnedTier::Pass,
                })
        });
        if let Some(name) = records.campaign_cursor(campaign_title).grid {
            frontend.select_campaign_event(&name);
        }
        super::reveals2048::refresh(&mut frontend, source, records, campaign_title);
        frontend.seed_craft(craft);
        frontend
    }
}
