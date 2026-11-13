//! Race Remix's own CRAFT TITLE/TRACK TITLE/TEAM rows, and its launch.
//!
//! Split out of [`super::menus`] under this project's 1,000-line ceiling
//! (`scripts/check-file-size.py`) - a move, with no behaviour change, except
//! for the CRAFT TITLE/TEAM/VARIANT guest-roster fold this file introduced
//! alongside the split. [`oag_game::livery_row::variant_choices`]/
//! [`super::menus::combine_variant`] stay there: they are shared with the
//! ordinary RACE page's own VARIANT row, not Race Remix's alone.

use log::warn;

use oag_game::remix;
use oag_ui::{menu, strings};

use crate::stage::Stage;

use super::Session;

/// Brings a stored setting into step with the list its row is about to be
/// given, by **`Menu::supply`'s own rule**: keep the stored value when the new
/// list still offers it, take the first one otherwise, and hold nothing at all
/// when the list is empty. `Menu::seed` can only move a row to a value its list
/// carries and `supply` resets an unmatched row to index 0, so without this the
/// row would show one value while the setting a launch reads holds another
/// (`oag_game::remix::settle_queries` is the same rule for a mod's picks).
pub(super) fn settle(stored: &mut String, offered: &[menu::Choice]) {
    if !offered.iter().any(|choice| choice.value == *stored) {
        *stored = offered
            .first()
            .map(|choice| choice.value.clone())
            .unwrap_or_default();
    }
}

/// CRAFT TITLE's own offered list - `titles` plus a synthetic "Wipeout HD"
/// choice when 2048 is among them and a real HD source is not, so 2048's
/// reused twelve teams stay reachable even with no real disc to reach them
/// from. Absent when a real HD source already offers the row, so the two
/// never both appear.
///
/// A free function over `titles` rather than a method reading `self.titles`
/// directly so it is unit-testable against hand-built candidates - see
/// `tests` below - without a live `Session`, which needs a window.
fn craft_title_choices(titles: &[oag_game::launcher::Candidate]) -> Vec<menu::Choice> {
    let mut choices: Vec<menu::Choice> = titles
        .iter()
        .map(|candidate| menu::Choice::plain(candidate.title()))
        .collect();
    for guest in titles
        .iter()
        .filter_map(|c| c.playable()?.race.guest_roster)
    {
        let mounted = titles
            .iter()
            .any(|candidate| candidate.title() == guest.reships);
        let entry = match (mounted, guest.alongside_label) {
            (true, Some(label)) => label,
            (true, None) => continue,
            (false, _) => guest.reships,
        };
        if !choices.iter().any(|choice| choice.value == entry) {
            choices.push(menu::Choice::plain(entry));
        }
    }
    choices
}

/// Which of `titles` backs `craft_title` - the matching one if it is there,
/// or Wipeout 2048's own package when "Wipeout HD" is named and no real HD
/// source is among `titles`.
///
/// **For a craft title only** (a `craft_titles` row, `teams_of`, `variants_of`,
/// the launch's craft side). 2048 reships
/// Wipeout HD/Fury's own twelve-team roster verbatim under a tree of its own
/// (`oag_title::RaceDefaults::guest_roster`) - a real disc still wins when
/// this machine has one, matching `oag_assets::Archives::open_with_packs`'s
/// own precedent that a more canonical source always wins a collision.
/// A `tracks_of` answer does not call this: 2048's own circuits are not HD's,
/// so no fallback applies there.
///
/// A free function for the same testability reason [`craft_title_choices`]
/// is.
pub(super) fn resolve_craft_backing<'a>(
    titles: &'a [oag_game::launcher::Candidate],
    craft_title: &str,
) -> Option<&'a oag_game::launcher::Candidate> {
    let alongside = |candidate: &&oag_game::launcher::Candidate| {
        candidate
            .playable()
            .and_then(|title| title.race.guest_roster)
            .is_some_and(|guest| guest.alongside_label == Some(craft_title))
    };
    titles
        .iter()
        .find(alongside)
        .or_else(|| titles.iter().find(|c| c.title() == craft_title))
        .or_else(|| {
            titles.iter().find(|candidate| {
                candidate
                    .playable()
                    .and_then(|title| title.race.guest_roster)
                    .is_some_and(|guest| guest.reships == craft_title)
            })
        })
}

/// The list a table-form query offers, off the survey `titles` and the custom-track scan.
/// `None` when the title the query names cannot be opened here.
///
/// `teams_of` is a craft title's roster ([`craft_catalogue`]: the guest roster of ADR-0035
/// folded in, narrowed to the companion class); `variants_of` resolves its title the same way;
/// `tracks_of` on CUSTOM is the scan's files, each checked against its host disc and the
/// companion craft title.
fn query_lists(
    titles: &[oag_game::launcher::Candidate],
    custom: &oag_game::custom_tracks::Scan,
    strings: &oag_ui::language::StringTable,
    query: menu::Query,
    args: remix::QueryArgs<'_>,
) -> Option<Vec<menu::Choice>> {
    match query {
        menu::Query::TracksOf if args.title == oag_game::custom_tracks::CUSTOM => {
            let craft = args
                .companion
                .and_then(|craft| resolve_craft_backing(titles, craft))
                .and_then(oag_game::launcher::Candidate::playable);
            Some(custom_track_choices(titles, custom, strings, craft))
        }
        menu::Query::TeamsOf => {
            let title = resolve_craft_backing(titles, args.title)?.playable()?;
            let catalogue = craft_catalogue(titles, args.title, args.companion.unwrap_or(""))?;
            remix::query_choices(query, &catalogue, title, None)
        }
        menu::Query::VariantsOf => {
            let candidate = resolve_craft_backing(titles, args.title)?;
            let title = candidate.playable()?;
            let catalogue = match args.companion {
                Some(_) => remix::Catalogue {
                    tracks: Vec::new(),
                    teams: Vec::new(),
                    multiplayer_teams: Vec::new(),
                },
                None => craft_catalogue(titles, args.title, "")?,
            };
            remix::query_choices(query, &catalogue, title, args.companion)
        }
        _ => {
            let candidate = titles.iter().find(|candidate| candidate.title() == args.title)?;
            let title = candidate.playable()?;
            match remix::catalogue(&candidate.source) {
                Ok(catalogue) => remix::query_choices(query, &catalogue, title, args.companion),
                Err(e) => {
                    warn!("{}: {e:#}", candidate.source);
                    None
                }
            }
        }
    }
}

/// The custom tracks as TRACK offers them under CUSTOM: the scan's files, each unavailable
/// with the reason when its host disc is not here or `craft` cannot race it.
fn custom_track_choices(
    titles: &[oag_game::launcher::Candidate],
    custom: &oag_game::custom_tracks::Scan,
    strings: &oag_ui::language::StringTable,
    craft: Option<&'static oag_title::Title>,
) -> Vec<menu::Choice> {
    let mut choices = custom.choices(strings);
    for choice in &mut choices {
        if choice.unavailable.is_some() {
            continue;
        }
        let Some(listing) = custom.listing(&choice.value) else {
            continue;
        };
        if let Some(host) = listing.host.as_deref()
            && let Err(why) = oag_game::custom_tracks::host_disc(titles, host)
        {
            choice.unavailable = Some(why);
        } else if let Some(why) = oag_game::custom_tracks::not_offered(listing, craft) {
            choice.unavailable = Some(why);
        }
    }
    choices
}

impl Session {
    /// Re-lists the rows whose `values_from` carries an `of`, for the values the settings they
    /// read hold now, and settles the mod picks under them (`remix::settle_queries`).
    /// `changed` limits it to the queries that read that setting.
    pub(crate) fn resupply_queries(&mut self, changed: Option<&str>) {
        let strings = strings::project_table(self.settings.language.as_deref());
        let (titles, custom) = (&self.titles, &self.custom_tracks);
        let mod_settings = &mut self.settings.mod_settings;
        if let Stage::Menu(stage) = &mut self.stage {
            remix::settle_queries(&mut stage.menu, mod_settings, changed, |query, args| {
                query_lists(titles, custom, &strings, query, args)
            });
        }
    }

    /// [`Self::resupply_queries`] on a model not yet on screen.
    pub(crate) fn resolve_menu_queries(&mut self, model: &mut menu::Menu) {
        let strings = strings::project_table(self.settings.language.as_deref());
        let (titles, custom) = (&self.titles, &self.custom_tracks);
        remix::settle_queries(model, &mut self.settings.mod_settings, None, |query, args| {
            query_lists(titles, custom, &strings, query, args)
        });
    }

    /// CRAFT TITLE's own offered list - [`craft_title_choices`]'s own doc
    /// comment for the rule.
    pub(super) fn craft_title_choices(&self) -> Vec<menu::Choice> {
        craft_title_choices(&self.titles)
    }

    /// Every speed class any title this machine can currently open a source
    /// for authors, as one ladder, filtered to the rungs this build can put a
    /// ship on.
    ///
    /// # Built from what is actually here, not from a list of every title
    ///
    /// [`Self::titles`] is the survey of *readable* sources, so a machine
    /// holding only the Pulse images unions Pulse's ladder and nothing else,
    /// and a machine that also has Wipeout Pure unions Pure's five rungs in
    /// too. A class whose title is absent is not offered at all - not greyed,
    /// not shown as a hint that the disc exists elsewhere. Drawing nothing is
    /// the honest absence; an entry that cannot load is not.
    ///
    /// That is the same rule the rows above it already follow: `TRACK TITLE`
    /// offers the titles this machine can open rather than the four this
    /// project knows about.
    ///
    /// # Vector is authored, raceable, and offered here
    ///
    /// Wipeout Pure authors a fifth rung, `VECTOR`, in every one of its race
    /// teams' `handlingstats.xml` files, and this build can put a ship on it -
    /// a race carries the rung as the name the disc spells and resolves it
    /// against the file that authored it, so no four-wide table
    /// had to grow a fifth entry. `is_selectable` says `true` for it and
    /// filters nothing here: with a Pure image present the *union* genuinely
    /// contains five names and this row offers all five.
    ///
    /// **This row is not where `VECTOR` is confined to remix - it is the
    /// remix row.** `oag_title::SpeedClasses::is_offered_outside_remix` is a
    /// *different*, narrower filter the ordinary RACE page's own row applies
    /// instead (`super::menus::speed_class_choices`), so a Pure-only boot's
    /// RACE page still offers four while this one offers five. See
    /// `docs/architecture/menus.md`'s "SPEED CLASS: `VECTOR` is confined to
    /// RACE REMIX" section for why the two pages disagree.
    pub(super) fn speed_classes_union(&self) -> Vec<menu::Choice> {
        let ladders = self
            .titles
            .iter()
            .filter_map(|candidate| match &candidate.state {
                oag_game::launcher::State::Playable(title) => title.race.speed_classes,
                oag_game::launcher::State::Unavailable(_) | oag_game::launcher::State::NeedsKey => None,
            });
        let union = oag_title::SpeedClasses::union(ladders);
        super::menus::to_choices(
            union
                .into_iter()
                .filter(|name| oag_title::SpeedClasses::is_selectable(name))
                .collect(),
        )
    }

    /// RACE REMIX's AI DIFFICULTY options: the four tiers and a `none` option
    /// the row is pinned to while the chosen class has no controller
    /// (`oag_ai::coverage`), labelled with the reason.
    pub(super) fn ai_difficulties(&self) -> Vec<menu::Choice> {
        let table = strings::project_table(self.settings.language.as_deref());
        let mut choices: Vec<menu::Choice> = ["novice", "skilled", "elite", "ace"]
            .into_iter()
            .map(menu::Choice::plain)
            .collect();
        choices.push(menu::Choice::labelled(
            "none",
            table
                .get("OAG_RACE_NO_DRIVER")
                .unwrap_or("NO OPPONENTS: NO DRIVER FOR THIS CLASS")
                .to_string(),
        ));
        choices
    }

}

/// The roster a craft title offers: [`resolve_craft_backing`]'s title, with 2048's guest
/// roster folded in or out by which name was picked, and narrowed to the teams that author
/// `class` (none when `class` is empty).
pub(super) fn craft_catalogue(
    titles: &[oag_game::launcher::Candidate],
    requested: &str,
    class: &str,
) -> Option<remix::Catalogue> {
    let candidate = resolve_craft_backing(titles, requested)?;
    let mut catalogue = match remix::catalogue(&candidate.source) {
        Ok(catalogue) => catalogue,
        Err(e) => {
            warn!("{}: {e:#}", candidate.source);
            return None;
        }
    };
    if let Some(guest) = candidate
        .playable()
        .and_then(|title| title.race.guest_roster)
    {
        if candidate.title() == requested {
            catalogue
                .teams
                .retain(|choice| !is_guest_team(guest, &choice.value));
        } else if requested == guest.reships || guest.alongside_label == Some(requested) {
            catalogue
                .teams
                .retain(|choice| is_guest_team(guest, &choice.value));
        }
    }
    // **A team whose files do not author the chosen class is hidden**
    // (chosen, not measured; `oag_title::RaceDefaults::team_races_class`).
    // Pure's VECTOR and 2048's SUPERPHANTOM are on one title's ladder or
    // one roster only, and a remix can pair them with a craft title whose
    // teams author neither: the TEAM row narrows, possibly to nothing,
    // rather than offering a team that fails to load.
    if let Some(title) = candidate.playable() {
        let class = class.trim();
        if !class.is_empty() {
            catalogue
                .teams
                .retain(|team| title.race.team_races_class(&team.value, class) != Some(false));
        }
    }
    Some(catalogue)
}

/// Whether `id` is one of `guest`'s reused teams - its own
/// [`oag_title::GuestRoster::variants`] list (2048's twelve HD/Fury teams).
fn is_guest_team(guest: &oag_title::GuestRoster, id: &str) -> bool {
    guest.variants.teams.contains(&id)
}

#[cfg(test)]
#[path = "remix/tests.rs"]
mod tests;
