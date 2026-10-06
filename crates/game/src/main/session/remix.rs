//! Race Remix's own CRAFT TITLE/TRACK TITLE/TEAM rows, and its launch.
//!
//! Split out of [`super::menus`] under this project's 1,000-line ceiling
//! (`scripts/check-file-size.py`) - a move, with no behaviour change, except
//! for the CRAFT TITLE/TEAM/VARIANT guest-roster fold this file introduced
//! alongside the split. [`super::menus::variant_choices`]/
//! [`super::menus::combine_variant`] stay there: they are shared with the
//! ordinary RACE page's own VARIANT row, not Race Remix's alone.

use log::{error, info, warn};

use oag_game::remix;
use oag_raceplay::catalogue;
use oag_ui::{menu, strings};

use crate::hints;
use crate::stage::Stage;

use super::Session;
use super::menus::{combine_variant, variant_choices};

/// Brings a stored `remix.*` setting into step with the list its row is
/// about to be given, by **`Menu::supply`'s own rule**: keep the stored value
/// when the new list still offers it, take the first one otherwise, and hold
/// nothing at all when the list is empty.
///
/// **Without this the page is dead on a fresh settings file.**
/// `settings::Remix` defaults every field to the empty string, `Menu::seed`
/// can only move a row to a value the row's own list carries, and `supply`
/// resets an unmatched row to index 0 - so the row showed the first title
/// while `self.settings` still held `""`, `Session::remix_catalogue_for`
/// answered `None` for a title of that name, TRACK/TEAM/VARIANT were never
/// supplied at all, and START reported "no track title chosen yet" and did
/// nothing. Applying `supply`'s rule to the setting too is what makes the row
/// and the setting agree by construction rather than by a second seed.
///
/// The same staleness the ordinary RACE page's own CIRCUIT row documents -
/// see `Session::resupply_tracks_for_mode` - reached one row further here,
/// because a remix's whole page is scoped to a title the settings file may
/// name before the row it feeds has any list at all.
pub(super) fn settle(stored: &mut String, offered: &[menu::Choice]) {
    if !offered.iter().any(|choice| choice.value == *stored) {
        *stored = offered
            .first()
            .map(|choice| choice.value.clone())
            .unwrap_or_default();
    }
}

/// A remix catalogue's tracks, in the shape `Menu::supply` wants - the same
/// `(Track, label)` -> `Choice::labelled` mapping [`Session::open_menus`]
/// applies to a booted title's own `Shell::tracks`, pulled out because the
/// RACE REMIX page needs it at two call sites and `Session::open_menus`'s
/// own list is a different, already-in-scope local of the same shape.
pub(super) fn remix_track_choices(catalogue: &remix::Catalogue) -> Vec<menu::Choice> {
    catalogue
        .tracks
        .iter()
        .map(|(track, name)| menu::Choice::labelled(&track.id, name))
        .collect()
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
    for reshipped in titles
        .iter()
        .filter_map(|c| c.playable()?.race.guest_roster)
        .map(|guest| guest.reships)
    {
        let offered = choices.iter().any(|choice| choice.value == reshipped);
        if !offered {
            choices.push(menu::Choice::plain(reshipped));
        }
    }
    choices
}

/// Which of `titles` backs `craft_title` - the matching one if it is there,
/// or Wipeout 2048's own package when "Wipeout HD" is named and no real HD
/// source is among `titles`.
///
/// **Race Remix only, and confined to the CRAFT TITLE row.** 2048 reships
/// Wipeout HD/Fury's own twelve-team roster verbatim under a tree of its own
/// (`oag_title::RaceDefaults::guest_roster`) - a real disc still wins when
/// this machine has one, matching `oag_assets::Archives::open_with_packs`'s
/// own precedent that a more canonical source always wins a collision.
/// TRACK TITLE's own resolver, [`Session::remix_catalogue_for`], does not
/// call this: 2048's own circuits are not HD's, so no fallback applies
/// there.
///
/// A free function for the same testability reason [`craft_title_choices`]
/// is.
fn resolve_craft_backing<'a>(
    titles: &'a [oag_game::launcher::Candidate],
    craft_title: &str,
) -> Option<&'a oag_game::launcher::Candidate> {
    titles
        .iter()
        .find(|candidate| candidate.title() == craft_title)
        .or_else(|| {
            titles.iter().find(|candidate| {
                candidate
                    .playable()
                    .and_then(|title| title.race.guest_roster)
                    .is_some_and(|guest| guest.reships == craft_title)
            })
        })
}

impl Session {
    /// Opens `title_setting`'s own catalogue, if this machine can currently
    /// open a source for it - the shared resolver for TRACK TITLE's own two
    /// call sites. CRAFT TITLE uses [`Self::remix_craft_catalogue`] instead:
    /// it can resolve to a different real title than the one named on
    /// screen, which TRACK TITLE never does.
    pub(super) fn remix_catalogue_for(&self, title_setting: &str) -> Option<remix::Catalogue> {
        let candidate = self
            .titles
            .iter()
            .find(|candidate| candidate.title() == title_setting)?;
        match remix::catalogue(&candidate.source) {
            Ok(catalogue) => Some(catalogue),
            Err(e) => {
                warn!("{}: {e:#}", candidate.source);
                None
            }
        }
    }

    /// Re-supplies the RACE REMIX page's TRACK row from whichever title
    /// `remix.track_title` currently names - the `remix.track_title`-changed
    /// counterpart to [`Self::resupply_tracks_for_mode`]. A no-op when the
    /// menus are not open or [`Self::remix_catalogue_for`] found nothing.
    pub(crate) fn resupply_remix_tracks(&mut self) {
        let Some(catalogue) = self.remix_catalogue_for(&self.settings.remix.track_title) else {
            return;
        };
        let tracks = remix_track_choices(&catalogue);
        settle(&mut self.settings.remix.track, &tracks);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(menu::ValueSource::RemixTracks, &tracks);
        }
    }

    /// [`Self::resupply_remix_tracks`]' sibling for TEAM, scoped to
    /// `remix.craft_title` instead - [`Self::remix_craft_catalogue`], not
    /// [`Self::remix_catalogue_for`], since CRAFT TITLE is the row that can
    /// resolve to a different real title than the one named on screen.
    pub(crate) fn resupply_remix_teams(&mut self) {
        let Some(catalogue) = self.remix_craft_catalogue() else {
            return;
        };
        settle(&mut self.settings.remix.team, &catalogue.teams);
        if let Stage::Menu(stage) = &mut self.stage {
            stage
                .menu
                .supply(menu::ValueSource::RemixTeams, &catalogue.teams);
        }
    }

    /// CRAFT TITLE's own offered list - [`craft_title_choices`]'s own doc
    /// comment for the rule.
    pub(super) fn craft_title_choices(&self) -> Vec<menu::Choice> {
        craft_title_choices(&self.titles)
    }

    /// Settles TRACK TITLE and CRAFT TITLE against the titles this machine
    /// can actually open, and hands back the two lists so the caller does not
    /// build them twice - [`settle`]'s own rule, at the outermost level.
    ///
    /// **Run before anything scoped under them.** A remix's page is a chain -
    /// a title decides its catalogue and the catalogue decides the
    /// TRACK/TEAM under it - so settling inward-out would settle a row
    /// against the list the title it no longer names would have given.
    ///
    /// No archive is opened here: both rows are answered from the survey
    /// [`Session::titles`] already holds, which is why this is the one step
    /// that is free to run on every menu-open.
    pub(super) fn settle_remix_titles(&mut self) -> (Vec<menu::Choice>, Vec<menu::Choice>) {
        let titles: Vec<menu::Choice> = self
            .titles
            .iter()
            .map(|candidate| menu::Choice::plain(candidate.title()))
            .collect();
        settle(&mut self.settings.remix.track_title, &titles);
        let craft_titles = self.craft_title_choices();
        settle(&mut self.settings.remix.craft_title, &craft_titles);
        (titles, craft_titles)
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
    /// against the file that authored it, so `oag_physics::SpeedClass` never
    /// had to grow a fifth variant. `is_selectable` says `true` for it and
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
    pub(super) fn remix_speed_classes(&self) -> Vec<menu::Choice> {
        let ladders = self
            .titles
            .iter()
            .filter_map(|candidate| match &candidate.state {
                oag_game::launcher::State::Playable(title) => title.race.speed_classes,
                oag_game::launcher::State::Unavailable(_) => None,
            });
        let union = oag_title::SpeedClasses::union(ladders);
        super::menus::to_choices(
            union
                .into_iter()
                .filter(|name| oag_title::SpeedClasses::is_selectable(name))
                .collect(),
        )
    }

    /// Which real candidate backs `remix.craft_title` -
    /// [`resolve_craft_backing`]'s own doc comment for the rule.
    fn craft_backing(&self) -> Option<&oag_game::launcher::Candidate> {
        resolve_craft_backing(&self.titles, &self.settings.remix.craft_title)
    }

    /// Which title `remix.craft_title` currently names, resolved through
    /// [`Self::craft_backing`] so a guest-backed pick answers with the title
    /// that will actually be opened (2048), not the one named on screen.
    pub(super) fn craft_title(&self) -> Option<&'static oag_title::Title> {
        self.craft_backing()
            .and_then(|candidate| candidate.playable())
    }

    /// Whether `id` is one of `guest`'s reused teams - its own
    /// [`oag_title::GuestRoster::variants`] list (2048's twelve HD/Fury teams),
    /// reused here rather than duplicated a third time.
    fn is_guest_team(guest: &oag_title::GuestRoster, id: &str) -> bool {
        guest.variants.teams.contains(&id)
    }

    /// CRAFT TITLE's own team list - [`Self::remix_catalogue_for`] scoped to
    /// [`Self::craft_backing`] instead of a direct survey match, and filtered
    /// so 2048's own list and "Wipeout HD"'s never show the same twelve teams
    /// twice:
    ///
    /// - 2048 picked directly: its own five native teams only - the reused
    ///   twelve now live under "Wipeout HD" instead.
    /// - "Wipeout HD" picked, backed by a real HD source: unfiltered, exactly
    ///   today's list.
    /// - "Wipeout HD" picked, backed by 2048 (no real HD source on this
    ///   machine): only the reused twelve, off 2048's own catalogue.
    /// - Every other title: unfiltered.
    pub(super) fn remix_craft_catalogue(&self) -> Option<remix::Catalogue> {
        let requested = &self.settings.remix.craft_title;
        let candidate = self.craft_backing()?;
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
                    .retain(|choice| !Self::is_guest_team(guest, &choice.value));
            } else if requested == guest.reships {
                catalogue
                    .teams
                    .retain(|choice| Self::is_guest_team(guest, &choice.value));
            }
        }
        Some(catalogue)
    }

    /// [`super::menus::Session::resupply_race_variant`]'s sibling for RACE
    /// REMIX's craft-side TEAM row, scoped to `remix.craft_title` and
    /// `remix.team` instead of the booted title.
    pub(crate) fn resupply_remix_variant(&mut self) {
        let Some(title) = self.craft_title() else {
            return;
        };
        let choices = variant_choices(title, &self.settings.remix.team);
        settle(&mut self.settings.remix.variant, &choices);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(menu::ValueSource::RemixVariant, &choices);
        }
    }

    /// The RACE REMIX page's own launch - see `menu::Action::LaunchRemix`'s
    /// own doc comment for why it is not a second meaning for `LaunchRace`.
    /// Builds `race::Options` the same way that one does, plus the two
    /// source fields neither the CLI's `--race` route nor the ordinary RACE
    /// page ever has to resolve.
    pub(crate) fn launch_remix(&mut self) {
        // Defensive, the same reason `Session::launch_from_settings` clears
        // both: RACE REMIX is not reachable from the campaign at all, but a
        // cell abandoned mid-`Team Selection` must never leak its own
        // eliminator target or lap override into an unrelated launch.
        self.campaign_cell = None;
        self.campaign_difficulty = None;
        self.campaign_ai_skill_scale = None;
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        race_options.eliminator_kill_target = None;
        race_options.laps_override = None;
        race_options.weapons_override = None;
        if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
            race_options.mode = mode;
        }
        // Carried as a **name** - see the same assignment in `menus.rs`. The
        // union this came from is built from the ladders of the titles this
        // machine can actually open, so `VECTOR` is in it exactly when Pure's
        // data is present.
        let class = self.settings.race.class.trim();
        if !class.is_empty() {
            race_options.class = class.to_string();
        }
        if let Some(difficulty) = oag_ai::Difficulty::from_name(&self.settings.ai.difficulty) {
            race_options.difficulty = difficulty;
        }

        let track_title = self.settings.remix.track_title.clone();
        let Some(track_candidate) = self
            .titles
            .iter()
            .find(|candidate| candidate.title() == track_title)
            .cloned()
        else {
            warn!("no track title chosen yet, so there is nothing to remix");
            self.race_options = Some(race_options);
            return;
        };
        race_options.source = track_candidate.source.clone();

        // An unpicked CRAFT TITLE means "the same as the track's",
        // on the same terms `race::Options::craft_source: None`
        // does - not a separate source opened redundantly. A picked
        // one this machine can no longer find (its disc left the
        // search path since it was saved) falls back the same way,
        // reported rather than silent.
        let craft_title = self.settings.remix.craft_title.clone();
        let craft_source = if craft_title.is_empty() {
            None
        } else {
            // `craft_backing` is what resolves "Wipeout HD" to 2048's
            // own package when this machine has no real HD source -
            // see its own doc comment.
            match self.craft_backing() {
                Some(candidate) => Some(candidate.source.clone()),
                None => {
                    warn!(
                        "{craft_title} is not a title this machine can currently \
                         open; racing craft from {track_title} instead"
                    );
                    None
                }
            }
        };
        race_options.craft_source = craft_source.clone();

        match remix::catalogue(&track_candidate.source) {
            Ok(catalogue) => {
                let chosen = catalogue
                    .track(&self.settings.remix.track)
                    .or_else(|| {
                        warn!(
                            "{track_title} does not offer {:?}; racing the first \
                             circuit it does offer, which is what the menu shows",
                            self.settings.remix.track
                        );
                        catalogue.tracks.first().map(|(track, _)| track)
                    })
                    .map(catalogue::Track::entry_name);
                match chosen {
                    Some(entry) => race_options.track = Some(entry),
                    None => warn!(
                        "{track_title} offers no circuit at all; racing whatever \
                         was already selected"
                    ),
                }
            }
            Err(e) => warn!("{}: {e:#}", track_candidate.source),
        }

        // The team's own title, for VARIANT to combine against -
        // whichever title actually supplied the craft: the picked
        // one, or the track's when craft follows it unpicked.
        let variant_title = if craft_source.is_some() {
            self.craft_title()
        } else {
            match &track_candidate.state {
                oag_game::launcher::State::Playable(title) => Some(*title),
                oag_game::launcher::State::Unavailable(_) => None,
            }
        };
        let craft_catalogue_source = craft_source.unwrap_or(track_candidate.source);
        match remix::catalogue(&craft_catalogue_source) {
            Ok(catalogue) => match catalogue.team(&self.settings.remix.team) {
                Some(team) => {
                    let team = team.to_string();
                    let (combined, hull_variant) = match variant_title {
                        Some(title) => {
                            let (combined, hull_variant, warning) =
                                combine_variant(title, &team, &self.settings.remix.variant);
                            if let Some(warning) = warning {
                                warn!("{warning}");
                            }
                            (combined, hull_variant)
                        }
                        // No title to check `team_variants`/`hull_variants`
                        // against - racing the bare id, which fails the same
                        // way this crate's own original bug report did if
                        // `team` is one that needs a second directory. Warned
                        // rather than silent, so a dead-end team picker names
                        // itself instead of surfacing as `race::load`'s own
                        // "no entry at ..." error.
                        None => {
                            warn!(
                                "{team}: could not resolve its own title, so no \
                                 VARIANT was applied; this fails if {team} needs one"
                            );
                            (team, None)
                        }
                    };
                    race_options.team = Some(combined);
                    race_options.hull_variant = hull_variant.map(str::to_string);
                }
                None => warn!(
                    "this craft title does not offer team {:?}, racing as its own \
                     default instead",
                    self.settings.remix.team
                ),
            },
            Err(e) => warn!("{craft_catalogue_source}: {e:#}"),
        }

        info!(
            "remixing {}",
            race_options
                .track
                .as_deref()
                .unwrap_or("this source's own default circuit")
        );
        self.race_options = Some(race_options);
        match self.launch_race() {
            Ok(()) => {
                let strings = strings::project_table(self.settings.language.as_deref());
                println!(
                    "\n{}{}",
                    hints::race_keys(&strings),
                    hints::esc_to_menu(&strings)
                );
            }
            Err(e) => error!("cannot start a remix race: {e:#}"),
        }
    }
}

#[cfg(test)]
#[path = "remix/tests.rs"]
mod tests;
