//! Reloading the front end after a LANGUAGE row pick, live.
//!
//! Split out of `session/menus.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` - a move, with no behaviour change.

use log::error;

use oag_game::prompts::substitution;
use oag_game::{boot, movie};
use oag_raceplay::pilots;
use oag_ui::menu;

use crate::race_stage::endrace_touch::EndRace;
use crate::stage::Stage;

use super::{Session, Shell};

impl Session {
    /// Reloads every disc-derived, language-scoped piece of the front end
    /// after a LANGUAGE row pick, instead of leaving the switch to sit until
    /// the next boot - see `main::session::apply::Session::apply_setting`'s
    /// `"language"` arm, which is this function's one caller.
    ///
    /// **Re-runs `boot::load_shell`, the exact call a fresh boot makes**,
    /// rather than re-deriving which fields the new language touches by
    /// hand - a hand-picked subset is how `race::hud::load_hud`'s own `None`
    /// (never the player's language) went unnoticed for as long as it did.
    /// `load_shell` is measured at 0.05 s on the EU disc (see its own doc),
    /// so paying it again from a keypress costs about what a first boot
    /// already pays before its intro even starts. The result is folded into
    /// [`Shell::from_boot`] - the same function [`crate::prepare::Pending::windowed`]
    /// calls - so a live switch and a fresh boot cannot read the same
    /// `boot::Shell` two different ways.
    ///
    /// **Builds the menu tree again** ([`oag_game::menu_tree::rebuilt`], the call a MODS
    /// page switch makes) when the run has mods to read, so every row's label, a mod's
    /// `strings/<language>.toml` and the MODS page's listing resolve against the new
    /// language. The menu model is rebuilt on that tree and reopened on the pages and rows
    /// the player stood on ([`Session::reopened_menu`]), so a player standing on LANGUAGE
    /// is still standing there afterwards. A `--menu` file has no mods: its tree is kept as
    /// built and only the supplied lists are re-`supply`d, the same mechanism
    /// [`super::menus::Session::resupply_tracks_for_mode`] uses for a MODE change.
    ///
    /// The extras' own rows follow `oag_ui::strings::project_table`, a project-owned
    /// translation layer separate from the disc's, which has English and French only.
    ///
    /// A parked race's own `hud::Assets` is not touched: it already resolved
    /// its strings and fonts when that race loaded, and reaching into a
    /// suspended `RaceStage` to swap them is out of scope here - resuming it
    /// still shows whatever language was live when it started, and a fresh
    /// LAUNCH RACE picks up the new one through `Session::launch_race`.
    ///
    /// A no-op when nothing has booted yet (`self.race_options` is `None`,
    /// which only the disc chooser reaches) or the reload itself fails -
    /// reported rather than losing the front end the player is standing in.
    pub(crate) fn resupply_language(&mut self) {
        let Some(race_options) = self.race_options.as_ref() else {
            return;
        };
        let Some((definition, mods, custom_campaigns, rebuild)) = self.shell.as_ref().map(|shell| {
            (
                shell.definition.clone(),
                shell.mods.clone(),
                shell.custom_campaigns.clone(),
                shell.rebuild,
            )
        })
        else {
            return;
        };
        let options = boot::Options {
            source: race_options.source.clone(),
            dlc: race_options.dlc.clone(),
            // The default leg, whatever the run actually booted on: this
            // reload never touches a movie, so the only thing `leg` decides
            // for `load_shell` - which `BootStep`s `walked` names - is
            // discarded output here, exactly like `_archives` below.
            leg: oag_ui::frontend::Leg::default(),
            language: self.settings.language.clone(),
            movie: None,
            cache: oag_source::cache::default_cache_dir(),
            audio_cache: oag_source::cache::default_audio_cache_dir(),
            extent: movie::Extent::Whole,
            no_video: true,
            refresh_video: false,
            prefer_av1_cache: false,
        };
        let (boot_shell, _archives, title) = match boot::load_shell(&options) {
            Ok(loaded) => loaded,
            Err(e) => {
                error!("could not reload the language: {e:#}");
                return;
            }
        };
        oag_raceplay::loader_log::lines(&boot_shell.report);
        // The tree is built again for the new language, as a boot builds it, so every
        // mod's rows and the MODS page read it; a `--menu` file has no mods to rebuild
        // and keeps its tree.
        let rebuilt = rebuild.and_then(|rebuild| {
            oag_game::menu_tree::rebuilt(
                &oag_mod::mods_search_path(),
                &self.settings.mods.disabled,
                self.settings.language.as_deref(),
                title,
                rebuild.updated,
                &boot_shell.strings,
            )
            .map_err(|e| error!("could not rebuild the menus for the language: {e:#}"))
            .ok()
        });
        let rebuilt_tree = rebuilt.is_some();
        let mut shell = Shell::from_boot(title, definition, &boot_shell);
        shell.mods = mods;
        shell.custom_campaigns = custom_campaigns;
        shell.rebuild = rebuild;
        if let Some((definition, listing, installed)) = rebuilt {
            self.custom_tracks.set_mod_files(&installed.tracks);
            shell.definition = definition;
            shell.mods = listing;
            shell.custom_campaigns = std::sync::Arc::new(installed);
        }
        // Decoded once at boot and language-independent, so it rides across.
        shell.menu_sfx = self.shell.as_ref().and_then(|old| old.menu_sfx.clone());
        let mode = self.race_mode();
        let reopened = if rebuilt_tree {
            self.reopened_menu(&shell)
        } else {
            None
        };
        let device = &self.gpu.device;
        let queue = &self.gpu.queue;
        if let Stage::Menu(stage) = &mut self.stage {
            if let Some(mut model) = reopened {
                model.settle();
                stage.menu = model;
            }
            let tracks: Vec<menu::Choice> = shell
                .tracks_for(mode)
                .iter()
                .map(|(track, name)| menu::Choice::labelled(&track.id, name))
                .collect();
            stage.menu.supply(menu::ValueSource::Tracks, &tracks);
            stage.menu.supply(menu::ValueSource::Teams, &shell.teams);
            stage
                .menu
                .supply(menu::ValueSource::RaceModes, &shell.modes);
            stage
                .menu
                .supply(menu::ValueSource::Languages, &shell.languages);
            stage
                .menu
                .supply(menu::ValueSource::FrontEndStyles, &shell.front_end_styles);
            // The footer chrome and the row-drawing setup - mirrors
            // `Session::open_menus`' own build of these from `shell`, since a
            // `MenuStage` keeps its own copies rather than reading `self.shell`
            // fresh every frame.
            stage.nav_legend = shell.nav_legend.clone();
            stage.ticker = shell.ticker.clone();
            stage.frame = shell.frame.clone();
            stage.default_atlas = shell.font.clone();
            let rows_face = shell
                .menu_font
                .clone()
                .unwrap_or_else(|| shell.font.clone());
            let mut skin = menu::Skin::new(shell.menu_skin, shell.space, rows_face.line_height);
            skin.set_row_ink(oag_ui::pointer::RowInk::measure(&rows_face));
            let reserve_note = pilots::page_reserves_axis_preview(stage.menu.page())
                || stage.menu.reserves_unavailable_note();
            stage
                .menu
                .set_visible_rows(menu::visible_rows(&skin, &shell.frame, reserve_note));
            stage.menu.set_strip_layout(skin.strip().is_some());
            stage
                .menu
                .set_reveal_attached(shell.title.main_menu.attached == oag_title::Attached::Revealed);
            stage.text_atlas = rows_face;
            stage.skin = skin;
            boot::fonts::install_faces(
                &mut stage.renderer,
                device,
                queue,
                shell.menu_skin,
                &shell.font,
                shell.title_font.clone(),
                shell.buttons_font.clone(),
            );
            self.stalled = true;
        }
        self.shell = Some(shell);
        self.reload_loading_assets();
    }
}

impl Session {
    /// Sets this frame's prompt substitution on the stage's renderer.
    ///
    /// Every frame rather than on change: a stage built since the last frame
    /// has a fresh renderer, and the substitution is a handful of pairs.
    pub(super) fn refresh_prompts(&mut self) {
        let Some(title) = self.shell.as_ref().map(|shell| shell.title) else {
            return;
        };
        let style = self.settings.controls.prompt_style_value();
        let style = self.prompt.style_override.unwrap_or(style);
        let family = self.controls.prompt_family(style);
        if self.prompt.family_logged != Some(family) {
            self.prompt.family_logged = Some(family);
            log::debug!(
                "button prompts: {} (style {}, last used {:?})",
                family.map_or("the disc's own", |family| family.name()),
                style.name(),
                self.controls.prompt_used(),
            );
        }
        let controls = &self.controls;
        let substitution =
            substitution(title.prompts, family, &|button| controls.bound_keys(button));
        match &mut self.stage {
            Stage::Frontend(stage) => stage.renderer.set_prompt_substitution(substitution),
            Stage::Menu(stage) => stage.renderer.set_prompt_substitution(substitution),
            Stage::Race(stage) => {
                if let Some(EndRace::Disc(runtime)) = &mut stage.endrace {
                    runtime.set_prompt_substitution(substitution);
                }
            }
            _ => {}
        }
    }
}
