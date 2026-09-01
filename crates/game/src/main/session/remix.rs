//! Race Remix's own CRAFT TITLE/TRACK TITLE/TEAM rows, and its launch.
//!
//! Split out of [`super::menus`] under this project's 1,000-line ceiling
//! (`scripts/check-file-size.py`) - a move, with no behaviour change, except
//! for the CRAFT TITLE/TEAM/VARIANT guest-roster fold this file introduced
//! alongside the split. [`super::menus::variant_choices`]/
//! [`super::menus::combine_variant`] stay there: they are shared with the
//! ordinary RACE page's own VARIANT row, not Race Remix's alone.

use log::{error, info, warn};

use oag_game::{catalogue, menu, remix};
use oag_physics::SpeedClass;

use crate::hints::{ESC_TO_MENU, RACE_KEYS};
use crate::stage::Stage;

use super::Session;
use super::menus::{combine_variant, variant_choices};

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
    let has_hd = titles.iter().any(|c| c.title() == oag_hd::TITLE.name);
    let has_2048 = titles.iter().any(|c| c.title() == oag_2048::TITLE.name);
    if !has_hd && has_2048 {
        choices.push(menu::Choice::plain(oag_hd::TITLE.name));
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
            (craft_title == oag_hd::TITLE.name)
                .then(|| titles.iter().find(|c| c.title() == oag_2048::TITLE.name))
                .flatten()
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
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(
                menu::ValueSource::RemixTracks,
                &remix_track_choices(&catalogue),
            );
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
            .and_then(|candidate| match &candidate.state {
                oag_game::launcher::State::Playable(title) => Some(*title),
                oag_game::launcher::State::Unavailable(_) => None,
            })
    }

    /// Whether `id` is one of 2048's own twelve reused HD/Fury teams -
    /// [`oag_2048::race::GUEST_TEAM_VARIANTS`]' own list, reused here rather
    /// than duplicated a third time.
    fn is_guest_team(id: &str) -> bool {
        oag_2048::race::GUEST_TEAM_VARIANTS.teams.contains(&id)
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
        if requested == oag_2048::TITLE.name {
            catalogue
                .teams
                .retain(|choice| !Self::is_guest_team(&choice.value));
        } else if requested == oag_hd::TITLE.name && candidate.title() != oag_hd::TITLE.name {
            catalogue
                .teams
                .retain(|choice| Self::is_guest_team(&choice.value));
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
        let Some(mut race_options) = self.race_options.take() else {
            warn!("no disc image has been chosen yet, so there is nothing to race");
            return;
        };
        if let Some(mode) = oag_race::Mode::from_name(&self.settings.race.mode) {
            race_options.mode = mode;
        }
        if let Some(class) = SpeedClass::from_name(&self.settings.race.class) {
            race_options.class = class;
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
                    race_options.team = Some(match variant_title {
                        Some(title) => {
                            let (combined, warning) =
                                combine_variant(title, &team, &self.settings.remix.variant);
                            if let Some(warning) = warning {
                                warn!("{warning}");
                            }
                            combined
                        }
                        // No title to check `team_variants` against - racing the
                        // bare id, which fails the same way this crate's own
                        // original bug report did if `team` is one that needs a
                        // second directory. Warned rather than silent, so a
                        // dead-end team picker names itself instead of surfacing
                        // as `race::load`'s own "no entry at ..." error.
                        None => {
                            warn!(
                                "{team}: could not resolve its own title, so no \
                                 VARIANT was applied; this fails if {team} needs one"
                            );
                            team
                        }
                    });
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
            Ok(()) => println!("\n{RACE_KEYS}{ESC_TO_MENU}"),
            Err(e) => error!("cannot start a remix race: {e:#}"),
        }
    }
}

#[cfg(test)]
#[path = "remix/tests.rs"]
mod tests;
