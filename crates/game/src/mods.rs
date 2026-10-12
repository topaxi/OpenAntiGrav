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
    /// Each candidate the search left out, with the reason.
    pub failed: Vec<oag_mod::Failed>,
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
    /// One entry per mod for the MODS page: the built-in first, then what loaded, then
    /// what was left out with its error.
    pub listing: Vec<Listing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One mod as the MODS page shows it. The page and the load report are built from the
/// same facts in the same call, so they cannot name different things.
pub struct Listing {
    /// The mod's id; for a candidate that never opened, its file name.
    pub id: String,
    /// The name a player reads; the file name for a candidate that never opened.
    pub name: String,
    /// The mod's release, empty for a candidate that never opened.
    pub version: String,
    /// Who made it.
    pub authors: Vec<String>,
    /// `None` for a candidate that never opened.
    pub license: Option<oag_mod::License>,
    /// The ids of the menu rows it added.
    pub rows: Vec<String>,
    /// Whether this is the built-in mod.
    pub builtin: bool,
    /// Why it is not in the game, when it is not.
    pub error: Option<String>,
}

/// The string id and English text of each label and word the MODS page draws.
pub mod words {
    /// `ID`, `VERSION`, `AUTHORS`, `LICENSE` and `ADDS`.
    pub const ID: (&str, &str) = ("OAG_MODS_ID", "ID");
    /// See [`ID`].
    pub const VERSION: (&str, &str) = ("OAG_MODS_VERSION", "VERSION");
    /// See [`ID`].
    pub const AUTHORS: (&str, &str) = ("OAG_MODS_AUTHORS", "AUTHORS");
    /// See [`ID`].
    pub const LICENSE: (&str, &str) = ("OAG_MODS_LICENSE", "LICENSE");
    /// See [`ID`].
    pub const ADDS: (&str, &str) = ("OAG_MODS_ADDS", "ADDS");
    /// A mod with no licence, visibly marked.
    pub const NO_LICENSE: (&str, &str) = ("OAG_MODS_NO_LICENSE", "NO LICENSE STATED");
    /// A licence kept in a file of the mod; `%s` is the file.
    pub const LICENSE_FILE: (&str, &str) = ("OAG_MODS_LICENSE_FILE", "SEE %s");
    /// A mod that added no row.
    pub const NOTHING: (&str, &str) = ("OAG_MODS_NOTHING", "NOTHING");
    /// The built-in mod's mark after its name.
    pub const BUILT_IN: (&str, &str) = ("OAG_MODS_BUILT_IN", "BUILT IN");
}

/// Longest value the page draws before it cuts with `...`: the front end's wide face leaves
/// about this much between the longest label (`AUTHORS`) and the right margin on a PSP-shaped
/// screen.
const VALUE_CHARS: usize = 22;

fn fit(text: &str) -> String {
    if text.chars().count() <= VALUE_CHARS {
        return text.to_string();
    }
    let keep: String = text.chars().take(VALUE_CHARS - 3).collect();
    format!("{keep}...")
}

/// The page the mods are listed on, and the setting its one row holds.
pub const PAGE: &str = "mods";
/// The MODS page's row. Never persisted: it only says which mod the detail lines are about.
pub const SELECTED: &str = "mods.selected";

/// The choices for `ValueSource::Mods`.
#[must_use]
pub fn choices(listing: &[Listing], strings: &StringTable) -> Vec<oag_ui::menu::Choice> {
    listing.iter().map(|mod_| mod_.choice(strings)).collect()
}

/// The detail lines for the mod the MODS page's row is on; `None` off that page.
#[must_use]
pub fn table_for(
    model: &oag_ui::menu::Menu,
    listing: &[Listing],
    strings: &StringTable,
) -> Option<Vec<(String, String)>> {
    if model.page().id != PAGE {
        return None;
    }
    let held = model
        .page()
        .entries
        .iter()
        .find(|entry| entry.setting() == Some(SELECTED))
        .and_then(oag_ui::menu::Entry::chosen)?;
    let oag_ui::menu::Value::Text(id) = held else {
        return None;
    };
    listing
        .iter()
        .find(|mod_| mod_.id == id)
        .map(|mod_| mod_.table(strings))
}

impl Listing {
    fn loaded(
        module: &oag_mod::Mod,
        rows: Vec<String>,
        builtin: bool,
        error: Option<String>,
    ) -> Self {
        let m = &module.manifest;
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.to_string(),
            authors: m.authors.clone(),
            license: Some(m.license.clone()),
            rows,
            builtin,
            error,
        }
    }

    fn failed(failed: &oag_mod::Failed) -> Self {
        Self {
            id: failed.file_name(),
            name: failed.file_name(),
            version: String::new(),
            authors: Vec::new(),
            license: None,
            rows: Vec::new(),
            builtin: false,
            error: Some(failed.cause().to_string()),
        }
    }

    /// The MODS page's choice for this mod: dimmed, with the error as the reason, when it
    /// is not in the game.
    #[must_use]
    pub fn choice(&self, _strings: &StringTable) -> oag_ui::menu::Choice {
        let choice = oag_ui::menu::Choice::labelled(self.id.clone(), fit(&self.name));
        match &self.error {
            Some(error) => choice.unusable(error.clone()),
            None => choice,
        }
    }

    /// The detail lines under the MODS page's row: label and value, in reading order.
    #[must_use]
    pub fn table(&self, strings: &StringTable) -> Vec<(String, String)> {
        let say = |(id, english): (&str, &str)| strings.get(id).unwrap_or(english).to_string();
        // A mod that is not in the game has no detail lines: its error is in the note slot
        // under the rows, where the same line would land.
        let (Some(license), None) = (&self.license, &self.error) else {
            return Vec::new();
        };
        let license = match license {
            oag_mod::License::Spdx(expression) => expression.clone(),
            oag_mod::License::File(path) => say(words::LICENSE_FILE).replace("%s", path),
            oag_mod::License::None => say(words::NO_LICENSE),
        };
        let version = if self.builtin {
            format!("{} {}", self.version, say(words::BUILT_IN))
        } else {
            self.version.clone()
        };
        let adds = if self.rows.is_empty() {
            say(words::NOTHING)
        } else {
            self.rows.join(", ")
        };
        let lines = vec![
            (say(words::ID), fit(&self.id)),
            (say(words::VERSION), fit(&version)),
            (say(words::AUTHORS), fit(&self.authors.join(", "))),
            (say(words::LICENSE), fit(&license)),
            (say(words::ADDS), fit(&adds)),
        ];
        lines
    }
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
            failed: found.failed,
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
        let mut listed = Vec::new();
        loop {
            match self.try_build(vanilla, language, target, &left_out) {
                Ok(mut built) => {
                    built.report.extend(problems);
                    built.listing.extend(listed);
                    built
                        .listing
                        .extend(self.failed.iter().map(Listing::failed));
                    return Ok(built);
                }
                Err(error) => self.leave_out(&error, &mut left_out, &mut problems, &mut listed)?,
            }
        }
    }

    fn leave_out(
        &self,
        error: &Error,
        left_out: &mut Vec<String>,
        problems: &mut Vec<String>,
        listed: &mut Vec<Listing>,
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
        listed.push(Listing::loaded(
            module,
            Vec::new(),
            false,
            Some(format!("left out: {error}")),
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
        let mut listed = Vec::new();
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
                    built.listing.extend(listed);
                    built
                        .listing
                        .extend(self.failed.iter().map(Listing::failed));
                    return Ok(built);
                }
                Err(error) => self.leave_out(&error, &mut left_out, &mut problems, &mut listed)?,
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
        let listing = all
            .iter()
            .map(|module| {
                let rows = added
                    .iter()
                    .find(|a| a.mod_id == module.manifest.id)
                    .map_or_else(Vec::new, |a| a.rows.clone());
                Listing::loaded(
                    module,
                    rows,
                    module.origin() == oag_mod::Origin::Builtin,
                    None,
                )
            })
            .collect();
        Ok(Built {
            definition,
            added,
            strings: table.clone(),
            report,
            listing,
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
        lines.extend(self.failed.iter().map(|p| format!("mod problem: {p}")));
        lines
    }
}
