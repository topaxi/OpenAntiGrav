//! Rows made from a list the caller holds: one `choice` row in the file stands for
//! "one row per item".
//!
//! A page whose rows are not known until run time (one switch per music source, one
//! per installed thing) declares a single template row, a `choice` with
//! `values_from = "<source>"` and no values. [`Definition::expand_rows`] replaces that
//! template with one row per [`RowItem`], in its place, before the [`super::Menu`] is
//! made. A list with no items leaves no row at all, the same answer a source with
//! nothing to choose gets from [`super::Menu::supply`].

use super::{Choice, Definition, Entry, Menu, RowMeta, ValueSource};

/// One row an expansion makes.
#[derive(Debug, Clone, PartialEq)]
pub struct RowItem {
    /// The row's key under the template's setting: its setting is `<template>.<id>`.
    pub id: String,
    /// What the row draws on the left.
    pub label: String,
    /// The line under the row while it is selected, when it has one.
    pub detail: Option<String>,
    /// The row's options.
    pub values: Vec<Choice>,
    /// The stored value it holds now.
    pub current: String,
}

impl Definition {
    /// Replaces every template row on `source` with one row per item.
    ///
    /// Returns how many rows the items made, summed over the templates.
    pub fn expand_rows(&mut self, source: ValueSource, items: &[RowItem]) -> usize {
        let mut made = 0;
        for page in &mut self.pages {
            while let Some(at) = page.entries.iter().position(|entry| {
                matches!(entry, Entry::Choice { source: from, values, .. }
                    if *from == Some(source) && values.is_empty())
            }) {
                let template = page.entries.remove(at);
                let subtitle = page.subtitles.remove(at);
                let meta = page.meta.remove(at);
                let Entry::Choice {
                    setting,
                    disabled_by,
                    pins,
                    warnings,
                    restart,
                    ..
                } = template
                else {
                    break;
                };
                for (offset, item) in items.iter().enumerate() {
                    let current = item
                        .values
                        .iter()
                        .position(|choice| choice.value == item.current)
                        .unwrap_or(0);
                    page.entries.insert(
                        at + offset,
                        Entry::Choice {
                            label: item.label.clone(),
                            setting: format!("{setting}.{}", item.id),
                            values: item.values.clone(),
                            source: None,
                            current,
                            disabled_by: disabled_by.clone(),
                            pins: pins.clone(),
                            warnings: warnings.clone(),
                            restart: restart.clone(),
                        },
                    );
                    page.subtitles.insert(
                        at + offset,
                        item.detail.clone().or_else(|| subtitle.clone()),
                    );
                    page.meta.insert(
                        at + offset,
                        RowMeta {
                            id: Some(item.id.clone()),
                            parent: meta.parent.clone(),
                        },
                    );
                }
                made += items.len();
            }
        }
        made
    }
}

impl Definition {
    /// Drops every row that opens page `id`, for a page with nothing on it to show.
    pub fn drop_submenus_to(&mut self, id: &str) {
        let Some(target) = self.pages.iter().position(|page| page.id == id) else {
            return;
        };
        for page in &mut self.pages {
            page.retain_rows(
                |entry| !matches!(entry, Entry::Submenu { target: to, .. } if *to == target),
            );
        }
    }
}

impl Menu {
    /// The settings of the rows an expansion made under `template`, in page order.
    #[must_use]
    pub fn expanded_settings(&self, template: &str) -> Vec<String> {
        let prefix = format!("{template}.");
        self.definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter_map(Entry::setting)
            .filter(|setting| setting.starts_with(&prefix))
            .map(str::to_string)
            .collect()
    }
}
