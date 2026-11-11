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

/// The menu tree, the MODS listing and the mod campaigns a rebuild makes.
pub(crate) type Rebuilt = (
    menu::Definition,
    Vec<mods::Listing>,
    oag_game::campaign::custom::Installed,
);

/// A rebuild's inputs, owned.
pub(crate) struct RebuildJob {
    bases: Vec<std::path::PathBuf>,
    switches: oag_game::settings::Mods,
    language: Option<String>,
    title: &'static oag_title::Title,
    updated: bool,
    strings: oag_ui::language::StringTable,
}

impl RebuildJob {
    /// Reads every mod from disk and builds the tree: the part that touches files.
    pub(crate) fn run(self) -> anyhow::Result<Rebuilt> {
        oag_game::menu_tree::rebuilt(
            &self.bases,
            &self.switches,
            self.language.as_deref(),
            self.title,
            self.updated,
            &self.strings,
        )
    }
}

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

    /// The SETTINGS row: walks into the settings page of the mod the MOD row is on, so BACK
    /// lands on the MODS page with the same mod.
    pub(crate) fn open_selected_mod_settings(&mut self) {
        let (Some(shell), Stage::Menu(stage)) = (self.shell.as_ref(), &mut self.stage) else {
            return;
        };
        if let Some(page) = mods::settings_page(&stage.menu, &shell.mods) {
            stage.menu.push(&page);
        }
    }

    /// The ENABLED row moved to `on`: turns the selected mod on or off, saves, and
    /// rebuilds the menus with it. The built-in and a mod that never opened do not
    /// switch; their row holds one value.
    pub(crate) fn switch_selected_mod(&mut self, on: bool) {
        let (Some(shell), Stage::Menu(stage)) = (self.shell.as_ref(), &self.stage) else {
            return;
        };
        let Some((id, default_enabled)) = mods::selected(&stage.menu, &shell.mods)
            .map(|mod_| (mod_.id.clone(), mod_.default_enabled))
        else {
            return;
        };
        if !mods::switch(&mut self.settings.mods, &id, on, default_enabled) {
            self.resupply_mod_switch();
            return;
        }
        info!("mod {id}: turned {} on the MODS page", if on { "on" } else { "off" });
        if let Err(e) = oag_game::settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
        }
        self.rebuild_menus(Some(&id));
    }

    /// Builds the tree, listing, campaigns and tracks again for the switches in
    /// `self.settings`, and reopens the menus where they were; with `select`, that mod
    /// is on the MOD row. A campaign saved from the editor is found this way.
    pub(super) fn rebuild_menus(&mut self, select: Option<&str>) {
        let Some(job) = self.rebuild_job() else {
            info!("a --menu file has no mods to rebuild; the switch is saved");
            return;
        };
        let built = job.run().map_err(|e| format!("{e:#}"));
        self.apply_rebuilt(built, select);
    }

    /// What a rebuild needs, owned, so it can run on a worker thread. `None` for a
    /// `--menu` file, which has no mods.
    pub(super) fn rebuild_job(&self) -> Option<RebuildJob> {
        let shell = self.shell.as_ref()?;
        let rebuild = shell.rebuild?;
        Some(RebuildJob {
            bases: oag_mod::mods_search_path(),
            switches: self.settings.mods.clone(),
            language: self.settings.language.clone(),
            title: shell.title,
            updated: rebuild.updated,
            strings: shell.strings.clone(),
        })
    }

    /// Puts a finished rebuild in place and reopens the menus where they were; with `select`,
    /// that mod is on the MOD row.
    pub(super) fn apply_rebuilt(&mut self, built: Result<Rebuilt, String>, select: Option<&str>) {
        let Some(mut shell) = self.shell.clone() else {
            return;
        };
        let (definition, listing, installed) = match built {
            Ok(built) => built,
            Err(e) => {
                error!("could not rebuild the menus for the mods: {e}; the change applies from the next launch");
                return;
            }
        };
        self.custom_tracks.set_mod_files(&installed.tracks);
        shell.definition = definition;
        shell.mods = listing;
        shell.custom_campaigns = std::sync::Arc::new(installed);
        let Some(mut model) = self.reopened_menu(&shell) else {
            self.shell = Some(shell);
            return;
        };
        if let Some(id) = select {
            model.seed(mods::SELECTED, &menu::Value::Text(id.to_string()));
            mods::resupply_switch(&mut model, &shell.mods, &shell.strings);
        }
        model.settle();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu = model;
        }
        self.shell = Some(shell);
        self.stalled = true;
    }

    /// The menu model built on `shell`'s tree and opened where the open menus stand: the
    /// same pages, the cursor on the same rows. `None` when no menu is open.
    pub(crate) fn reopened_menu(&mut self, shell: &super::Shell) -> Option<menu::Menu> {
        let Stage::Menu(stage) = &self.stage else {
            return None;
        };
        let trail = stage.menu.trail();
        let skin = stage.skin;
        let mut model = self.menu_model(shell, &skin);
        if !model.restore_trail(&trail) {
            error!("the rebuilt menus lack a page the player was on: {trail:?}");
        }
        Some(model)
    }
}
