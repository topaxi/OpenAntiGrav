//! The MODS page's ENABLED row: turning a mod on or off, live.
//!
//! A switch saves `settings::Mods::disabled` and rebuilds what the mods make, the way
//! a boot builds it (`crate::prepare::mods_menu` and `crate::prepare::finish_definition`):
//! the menu tree with its rows, the MODS listing, the mod campaigns and RACE REMIX's
//! mod tracks. The menu model is built again on the new tree with the same
//! [`Session::menu_model`] the menus open with, and reopened where the player stood:
//! the MODS page, on the same mod, BACK leading where it led.
//!
//! Live rather than on the next launch because nothing a mod adds is held past the
//! front end: a race reads its options when it launches, a campaign when it opens. A
//! race parked under the menus (`Session::suspended_race`) keeps what it started with.

use log::{error, info};

use oag_game::mods;
use oag_ui::menu;

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Re-supplies the ENABLED row for the mod the MOD row just moved onto.
    pub(crate) fn resupply_mod_switch(&mut self) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        if let Stage::Menu(stage) = &mut self.stage {
            mods::resupply_switch(&mut stage.menu, &shell.mods, &shell.strings);
        }
    }

    /// The ENABLED row moved to `on`: turns the selected mod on or off, saves, and
    /// rebuilds the menus with it. The built-in and a mod that never opened do not
    /// switch; their row holds one value.
    pub(crate) fn switch_selected_mod(&mut self, on: bool) {
        let (Some(shell), Stage::Menu(stage)) = (self.shell.as_ref(), &self.stage) else {
            return;
        };
        let Some(id) = mods::selected(&stage.menu, &shell.mods).map(|mod_| mod_.id.clone())
        else {
            return;
        };
        if !mods::switch(&mut self.settings.mods.disabled, &id, on) {
            self.resupply_mod_switch();
            return;
        }
        info!("mod {id}: turned {} on the MODS page", if on { "on" } else { "off" });
        if let Err(e) = oag_game::settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
        }
        self.rebuild_mod_menus(&id);
    }

    /// Builds the tree, listing, campaigns and tracks again for the switches in
    /// `self.settings`, and reopens the menus where they were with `id` on the MOD row.
    fn rebuild_mod_menus(&mut self, id: &str) {
        let Some(mut shell) = self.shell.clone() else {
            return;
        };
        let Some(rebuild) = shell.rebuild else {
            info!("a --menu file has no mods to rebuild; the switch is saved");
            return;
        };
        let built = mods::Mods::load(&self.settings.mods.disabled).and_then(|loaded| {
            crate::prepare::mods_menu(
                &loaded,
                self.settings.language.as_deref(),
                shell.title,
                rebuild.updated,
            )
        });
        let (mut definition, listing, installed) = match built {
            Ok(built) => built,
            Err(e) => {
                error!("could not rebuild the menus for the mods: {e:#}; the switch applies from the next launch");
                return;
            }
        };
        crate::prepare::finish_definition(&mut definition, shell.title, &shell.strings);
        self.custom_tracks.set_mod_files(&installed.tracks);
        shell.definition = definition;
        shell.mods = listing;
        shell.custom_campaigns = std::sync::Arc::new(installed);
        let Stage::Menu(stage) = &self.stage else {
            self.shell = Some(shell);
            return;
        };
        let trail = stage.menu.trail();
        let skin = stage.skin;
        let mut model = self.menu_model(&shell, &skin);
        if !model.restore_trail(&trail) {
            error!("the rebuilt menus lack a page the player was on: {trail:?}");
        }
        model.seed(mods::SELECTED, &menu::Value::Text(id.to_string()));
        mods::resupply_switch(&mut model, &shell.mods, &shell.strings);
        model.settle();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu = model;
        }
        self.shell = Some(shell);
        self.stalled = true;
    }
}
