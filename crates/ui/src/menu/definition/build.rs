//! The menu tree for one title: the vanilla top level it shows, plus the rows
//! mods attach.
//!
//! `assets/ui/menu.toml`'s `main` page holds the rows an original menu can have
//! (campaign, racebox, options, records), each with an `id`. [`Definition::build`]
//! keeps the ones the title's own main menu opens, in the disc's order
//! ([`oag_title::MainMenu`]), then merges each mod's `menu.toml`: a row names
//! where it goes with `parent` (a top-level row's id, or `top`) and, optionally,
//! `after` a sibling. The project's own rows (REMIX, RECORDS, QUIT, and RACEBOX or
//! OPTIONS where the original has none) arrive the same way, from the built-in
//! `oag-extras`. ADR-0066 has the model.

use std::collections::HashMap;

use serde::Deserialize;

use super::{Definition, Error, RowMeta, StringTable, raw, resolve, resolve_subtitle};

/// The reserved `parent`: the top level of the main page.
pub const TOP: &str = "top";

/// A mod's `menu.toml`, with the id of the mod it came from.
#[derive(Debug, Clone, Copy)]
pub struct ModMenu<'a> {
    /// The mod's `id`, for the messages.
    pub id: &'a str,
    /// The file's text.
    pub text: &'a str,
}

/// The title a tree is built for.
#[derive(Debug, Clone, Copy)]
pub struct For<'a> {
    /// The title.
    pub title: &'a oag_title::Title,
    /// Whether the title's update is mounted (see [`oag_title::MainMenu::updated_rows`]).
    pub updated: bool,
}

/// What one mod did to the tree, for the load report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Added {
    /// The mod's `id`.
    pub mod_id: String,
    /// The ids of the rows it added, in file order.
    pub rows: Vec<String>,
    /// Rows left out because the tree already had a row with that id
    /// (`only_if_missing`).
    pub skipped: Vec<String>,
    /// Rows whose parent is not on this title's main menu, placed at the top level.
    pub moved_to_top: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModFile {
    #[serde(rename = "toml-schema")]
    toml_schema: Option<oag_schema::Header>,
    #[serde(default, rename = "entry")]
    entries: Vec<ModEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModEntry {
    id: String,
    parent: String,
    after: Option<String>,
    #[serde(default)]
    only_if_missing: bool,
    kind: String,
    label: String,
    target: Option<String>,
    action: Option<String>,
    string_id: Option<String>,
    subtitle: Option<String>,
    subtitle_string_id: Option<String>,
}

/// The schema kind a mod's menu file declares.
pub const MOD_SCHEMA_KIND: &str = "mod-menu";

fn fault(mod_id: &str, key: impl Into<String>, problem: impl Into<String>) -> Error {
    Error::Mod {
        mod_id: mod_id.to_string(),
        key: key.into(),
        problem: problem.into(),
    }
}

impl Definition {
    /// This project's tree with the built-in extras and no title: every row the
    /// extras place resolves, none of a title's trimming applies. What a test or
    /// a tool that has no disc asks for.
    ///
    /// # Errors
    ///
    /// As [`Self::build`].
    pub fn built_in(strings: &StringTable) -> Result<Self, Error> {
        let extras = oag_mod::extras::builtin()
            .map_err(|e| fault(oag_mod::extras::ID, "read", e.to_string()))?;
        let text = extras
            .menu()
            .map_err(|e| fault(oag_mod::extras::ID, "menu.toml", e.to_string()))?
            .unwrap_or_default();
        let mods = [ModMenu {
            id: oag_mod::extras::ID,
            text: &text,
        }];
        Self::build(
            super::super::BUILT_IN,
            strings,
            &crate::strings::project_table(None),
            None,
            &mods,
        )
        .map(|(tree, _)| tree)
    }

    /// Builds the tree: `vanilla` parsed, trimmed to `title`'s own main menu when
    /// one is given, then every mod's rows merged in order.
    ///
    /// `strings` resolves labels in the player's language; `english` is the base
    /// every mod `string_id` must exist in.
    ///
    /// # Errors
    ///
    /// Any [`Error`]: a mod row with an unknown parent, `after`, page, action or
    /// string id names the mod and the key.
    pub fn build(
        vanilla: &str,
        strings: &StringTable,
        english: &StringTable,
        title: Option<For<'_>>,
        mods: &[ModMenu<'_>],
    ) -> Result<(Self, Vec<Added>), Error> {
        let mut tree = Self::parse_vanilla(vanilla, strings)?;
        let root = tree.root;
        let mut known: Vec<String> = tree.pages[root]
            .meta
            .iter()
            .filter_map(|meta| meta.id.clone())
            .collect();
        if let Some(target) = title {
            tree.keep_title_rows(target)?;
        }

        let index: HashMap<String, usize> = tree
            .pages
            .iter()
            .enumerate()
            .map(|(i, page)| (page.id.clone(), i))
            .collect();
        let mut owners: HashMap<String, String> = tree.pages[root]
            .meta
            .iter()
            .filter_map(|meta| meta.id.clone())
            .map(|id| (id, "the vanilla menu".to_string()))
            .collect();

        let mut added = Vec::new();
        for module in mods {
            let file: ModFile = toml::from_str(module.text)
                .map_err(|e| fault(module.id, "menu.toml", e.to_string()))?;
            oag_schema::check(file.toml_schema.as_ref(), MOD_SCHEMA_KIND, 1)
                .map_err(|e| fault(module.id, "menu.toml", e.to_string()))?;
            // Every id this file names is known to the others, so a row attached to
            // another mod's row that this title lacks still falls to the top level.
            known.extend(file.entries.iter().map(|entry| entry.id.clone()));
            added.push((module.id, file));
        }

        let mut report = Vec::new();
        for (mod_id, file) in added {
            let mut done = Added {
                mod_id: mod_id.to_string(),
                ..Added::default()
            };
            for entry in &file.entries {
                tree.attach(
                    mod_id,
                    entry,
                    &index,
                    strings,
                    english,
                    &known,
                    &mut owners,
                    &mut done,
                )?;
            }
            report.push(done);
        }
        tree.check()?;
        Ok((tree, report))
    }

    /// Keeps the main-page rows the title's own menu opens, in the disc's order.
    fn keep_title_rows(&mut self, target: For<'_>) -> Result<(), Error> {
        let root = self.root;
        let page = &mut self.pages[root];
        let wanted: Vec<&str> = target.title.main_menu.built(target.updated).collect();
        for id in &wanted {
            if page.row_with_id(id).is_none() {
                return Err(fault(
                    target.title.name,
                    format!("main_menu builds {id:?}"),
                    "assets/ui/menu.toml's main page has no row with that id",
                ));
            }
        }
        let order: Vec<usize> = wanted
            .iter()
            .filter_map(|id| page.row_with_id(id))
            .collect();
        reorder(&mut page.entries, &order);
        reorder(&mut page.subtitles, &order);
        reorder(&mut page.meta, &order);
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one placement, every input it reads"
    )]
    fn attach(
        &mut self,
        mod_id: &str,
        row: &ModEntry,
        index: &HashMap<String, usize>,
        strings: &StringTable,
        english: &StringTable,
        known: &[String],
        owners: &mut HashMap<String, String>,
        done: &mut Added,
    ) -> Result<(), Error> {
        let key = |name: &str| format!("entry {:?} {name}", row.id);
        if let Some(owner) = owners.get(&row.id) {
            if row.only_if_missing {
                done.skipped.push(row.id.clone());
                return Ok(());
            }
            return Err(fault(
                mod_id,
                key("id"),
                format!("{:?} is already used by {owner}", row.id),
            ));
        }
        if !matches!(row.kind.as_str(), "submenu" | "action") {
            return Err(fault(
                mod_id,
                key("kind"),
                format!("{:?}: a mod row is a submenu or an action", row.kind),
            ));
        }
        for (name, id) in [
            ("string_id", &row.string_id),
            ("subtitle_string_id", &row.subtitle_string_id),
        ] {
            if let Some(id) = id
                && english.get(id).is_none()
                && strings.get(id).is_none()
            {
                return Err(fault(
                    mod_id,
                    key(name),
                    format!("{id:?} is in no strings file (the project's or this mod's)"),
                ));
            }
        }

        let raw_entry = raw::Entry {
            id: Some(row.id.clone()),
            kind: row.kind.clone(),
            label: row.label.clone(),
            target: row.target.clone(),
            action: row.action.clone(),
            setting: None,
            values: Vec::new(),
            values_from: None,
            button: None,
            disabled_by: None,
            pinned: None,
            warn_when: None,
            restart_required: None,
            string_id: row.string_id.clone(),
            subtitle: row.subtitle.clone(),
            subtitle_string_id: row.subtitle_string_id.clone(),
        };
        let context = format!("mod {mod_id} {}", key("row"));
        let entry = resolve(&raw_entry, &context, index, strings).map_err(|e| match e {
            Error::NoSuchPage { id, .. } => fault(mod_id, key("target"), format!("no page {id:?}")),
            Error::NoSuchAction { name, known, .. } => fault(
                mod_id,
                key("action"),
                format!("{name:?} is not an action ({known})"),
            ),
            other => fault(mod_id, key("row"), other.to_string()),
        })?;
        let subtitle = resolve_subtitle(&raw_entry, strings);

        let root = self.root;
        let page = &mut self.pages[root];
        let mut parent = None;
        if row.parent != TOP {
            match page.row_with_id(&row.parent) {
                Some(at) if page.is_attached(at) => {
                    return Err(fault(
                        mod_id,
                        key("parent"),
                        format!(
                            "{:?} is itself attached; a row attaches to a top-level row",
                            row.parent
                        ),
                    ));
                }
                Some(_) => parent = Some(row.parent.clone()),
                None if known.contains(&row.parent) => done.moved_to_top.push(row.id.clone()),
                None => {
                    return Err(fault(
                        mod_id,
                        key("parent"),
                        format!(
                            "{:?} is no row id (use {TOP:?} for the top level)",
                            row.parent
                        ),
                    ));
                }
            }
        }
        let mut at = match &parent {
            Some(id) => {
                let head = page.row_with_id(id).unwrap_or_default();
                page.family(head)
                    .last()
                    .map_or(page.entries.len(), |last| last + 1)
            }
            None => page.entries.len(),
        };
        if let Some(sibling) = &row.after {
            match page.row_with_id(sibling) {
                Some(found) => {
                    let siblings: Vec<usize> = match &parent {
                        Some(id) => page
                            .family(page.row_with_id(id).unwrap_or_default())
                            .collect(),
                        None => page.top_level().collect(),
                    };
                    if !siblings.contains(&found) {
                        return Err(fault(
                            mod_id,
                            key("after"),
                            format!("{sibling:?} is not a sibling under {:?}", row.parent),
                        ));
                    }
                    at = match &parent {
                        Some(_) => found + 1,
                        None => page.family(found).last().map_or(found + 1, |last| last + 1),
                    };
                }
                None if known.contains(sibling) => {}
                None => {
                    return Err(fault(
                        mod_id,
                        key("after"),
                        format!("{sibling:?} is no row id"),
                    ));
                }
            }
        }
        page.entries.insert(at, entry);
        page.subtitles.insert(at, subtitle);
        page.meta.insert(
            at,
            RowMeta {
                id: Some(row.id.clone()),
                parent,
            },
        );
        owners.insert(row.id.clone(), format!("mod {mod_id}"));
        done.rows.push(row.id.clone());
        Ok(())
    }
}

/// Keeps `rows[i]` for each `i` in `order`, in that order, and drops the rest.
fn reorder<T>(rows: &mut Vec<T>, order: &[usize]) {
    let mut taken: Vec<Option<T>> = rows.drain(..).map(Some).collect();
    rows.extend(order.iter().filter_map(|&i| taken.get_mut(i)?.take()));
}
