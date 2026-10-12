//! Mods in the running game: finding them, building the menu tree from the
//! title's own main menu plus their rows, and saying what each one did.
//!
//! The format and the reader are `oag_mod`; the merge is
//! `oag_ui::menu::Definition::build`. This is the composition root's half: where
//! to look, which title and language the tree is built for, and what to do with a
//! mod that does not fit - leave it out and name it, never stop the game.
//! ADR-0066.

use std::path::PathBuf;

use anyhow::{Context, Result};
use oag_ui::language::StringTable;
use oag_ui::menu::{Added, Definition, Error, For, ModMenu};
use oag_ui::strings;

#[derive(Debug)]
/// The mods this run has: the built-in extras and what the search found.
pub struct Mods {
    extras: oag_mod::Mod,
    player: Vec<oag_mod::Mod>,
    /// One line for each candidate or row left out, with the reason.
    pub problems: Vec<String>,
}

#[derive(Debug)]
/// A tree built for one title and language.
pub struct Built {
    /// The tree.
    pub definition: Definition,
    /// What each mod added, extras first.
    pub added: Vec<Added>,
    /// The strings the tree's labels were resolved against.
    pub strings: StringTable,
    /// Lines for the load report: every mod, its version, licence and rows.
    pub report: Vec<String>,
}

fn game_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

impl Mods {
    /// The extras and whatever lies on the default search path.
    ///
    /// # Errors
    ///
    /// The built-in extras will not open, which is a build defect.
    pub fn load() -> Result<Self> {
        Self::load_from(&oag_mod::mods_search_path())
    }

    /// The extras and whatever lies in `bases`.
    ///
    /// # Errors
    ///
    /// As [`Self::load`].
    pub fn load_from(bases: &[PathBuf]) -> Result<Self> {
        let extras = oag_mod::extras::builtin().context("opening the built-in oag-extras")?;
        let found = oag_mod::load(bases, &game_version());
        Ok(Self {
            extras,
            player: found.mods,
            problems: found.problems,
        })
    }

    /// Only the built-in extras: what a test or a tool without a search path asks for.
    ///
    /// # Errors
    ///
    /// As [`Self::load`].
    pub fn extras_only() -> Result<Self> {
        Self::load_from(&[])
    }

    /// Whether any mod beside the extras is loaded.
    #[must_use]
    pub fn has_player_mods(&self) -> bool {
        !self.player.is_empty()
    }

    fn strings(
        &self,
        module: &oag_mod::Mod,
        table: &mut StringTable,
        language: &str,
    ) -> Result<()> {
        if let Some(text) = module
            .strings(language)
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            strings::overlay_text(table, &text)
                .map_err(|e| anyhow::anyhow!("{}: strings/{language}.toml: {e}", module.label()))?;
        }
        Ok(())
    }

    /// Builds the tree: `vanilla` trimmed to `target`'s own main menu when given,
    /// then the extras and each player mod. A player mod whose rows do not fit
    /// is left out and named in [`Self::problems`]; the extras failing is an error.
    ///
    /// # Errors
    ///
    /// The vanilla tree or the extras are wrong, which is a build defect.
    pub fn build(
        &self,
        vanilla: &str,
        language: Option<&str>,
        target: Option<For<'_>>,
    ) -> Result<Built> {
        let mut left_out: Vec<String> = Vec::new();
        let mut problems = Vec::new();
        loop {
            match self.try_build(vanilla, language, target, &left_out) {
                Ok(mut built) => {
                    built.report.extend(problems);
                    return Ok(built);
                }
                Err(error) => self.leave_out(&error, &mut left_out, &mut problems)?,
            }
        }
    }

    fn leave_out(
        &self,
        error: &Error,
        left_out: &mut Vec<String>,
        problems: &mut Vec<String>,
    ) -> Result<()> {
        let named = match error {
            Error::Mod { mod_id, .. } => self
                .player
                .iter()
                .find(|module| &module.manifest.id == mod_id),
            _ => None,
        };
        let Some(module) = named else {
            return Err(anyhow::anyhow!("{error}"));
        };
        problems.push(format!(
            "mod problem: {}: left out: {error}",
            module.label()
        ));
        left_out.push(module.manifest.id.clone());
        Ok(())
    }

    /// [`Self::build`] against a strings table the caller already has (a capture's,
    /// disc text and all), with each mod's English strings laid over it.
    ///
    /// # Errors
    ///
    /// As [`Self::build`].
    pub fn build_with_strings(
        &self,
        vanilla: &str,
        strings: &StringTable,
        target: Option<For<'_>>,
    ) -> Result<Built> {
        let mut left_out: Vec<String> = Vec::new();
        let mut problems = Vec::new();
        loop {
            let mut table = strings.clone();
            let mut english = strings::project_table(None);
            let tried = self
                .layer_strings(&mut table, "english", &left_out)
                .and_then(|()| self.layer_strings(&mut english, "english", &left_out))
                .and_then(|()| self.assemble(vanilla, &table, &english, target, &left_out));
            match tried {
                Ok(mut built) => {
                    built.report.extend(problems);
                    return Ok(built);
                }
                Err(error) => self.leave_out(&error, &mut left_out, &mut problems)?,
            }
        }
    }

    fn layer_strings(
        &self,
        table: &mut StringTable,
        tongue: &str,
        left_out: &[String],
    ) -> Result<(), Error> {
        for module in self.active(left_out) {
            self.strings(module, table, tongue)
                .map_err(|e| Error::Mod {
                    mod_id: module.manifest.id.clone(),
                    key: "strings".to_string(),
                    problem: e.to_string(),
                })?;
        }
        Ok(())
    }

    fn active<'a>(&'a self, left_out: &'a [String]) -> impl Iterator<Item = &'a oag_mod::Mod> {
        std::iter::once(&self.extras).chain(
            self.player
                .iter()
                .filter(|m| !left_out.contains(&m.manifest.id)),
        )
    }

    fn try_build(
        &self,
        vanilla: &str,
        language: Option<&str>,
        target: Option<For<'_>>,
        left_out: &[String],
    ) -> Result<Built, Error> {
        let tongue = language.unwrap_or("English");
        let mut table = strings::project_table(language);
        let mut english = strings::project_table(None);
        self.layer_strings(&mut table, tongue, left_out)?;
        self.layer_strings(&mut english, "english", left_out)?;
        self.assemble(vanilla, &table, &english, target, left_out)
    }

    fn assemble(
        &self,
        vanilla: &str,
        table: &StringTable,
        english: &StringTable,
        target: Option<For<'_>>,
        left_out: &[String],
    ) -> Result<Built, Error> {
        let all: Vec<&oag_mod::Mod> = self.active(left_out).collect();
        let mut menus = Vec::new();
        for module in &all {
            let text = module
                .menu()
                .map_err(|e| Error::Mod {
                    mod_id: module.manifest.id.clone(),
                    key: "menu.toml".to_string(),
                    problem: e.to_string(),
                })?
                .unwrap_or_default();
            menus.push((module.manifest.id.clone(), text));
        }
        let mods: Vec<ModMenu<'_>> = menus
            .iter()
            .filter(|(_, text)| !text.is_empty())
            .map(|(id, text)| ModMenu { id, text })
            .collect();
        let (definition, added) = Definition::build(vanilla, table, english, target, &mods)?;
        let report = self.report(&all, &added);
        Ok(Built {
            definition,
            added,
            strings: table.clone(),
            report,
        })
    }

    fn report(&self, all: &[&oag_mod::Mod], added: &[Added]) -> Vec<String> {
        let mut lines = Vec::new();
        for module in all {
            let did = added.iter().find(|a| a.mod_id == module.manifest.id);
            let rows = did.map_or_else(String::new, |a| {
                let mut text = format!(
                    "rows: {}",
                    if a.rows.is_empty() {
                        "none".to_string()
                    } else {
                        a.rows.join(", ")
                    }
                );
                if !a.skipped.is_empty() {
                    text.push_str(&format!(
                        "; skipped (already there): {}",
                        a.skipped.join(", ")
                    ));
                }
                if !a.moved_to_top.is_empty() {
                    text.push_str(&format!(
                        "; parent not on this menu, placed at the top: {}",
                        a.moved_to_top.join(", ")
                    ));
                }
                text
            });
            lines.push(format!("mod {} - {rows}", module.describe()));
        }
        lines.extend(self.problems.iter().map(|p| format!("mod problem: {p}")));
        lines
    }
}
