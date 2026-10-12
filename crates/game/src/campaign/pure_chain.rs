//! Wipeout Pure's SINGLE PLAYER chain as a state machine: which screen the
//! player is on, what it lists, and what a choice does.
//!
//! The order of the screens is the disc's (`oag_tables::pure_flow`, read off the
//! screens' own redirects); what each lists is the campaign data
//! (`oag_game::campaign::pure_list`) against the profile's medals. The last
//! screen before `Team Selection` names an event, and that event is the same
//! [`oag_tables::race_campaign::Cell`] a Pulse grid is made of, so it launches
//! and records through the campaign's own path.
//!
//! **Free Play** is the one mode that is not an event: the disc names it and
//! its results screen ("Free play session results"), and no medal law reaches
//! it. It races the chosen circuit and class as a plain race, with no cell and
//! so no record (chosen, not measured).

use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::{self, Event, EventKind, Progress};
use oag_tables::pure_flow::{Flow, Step};
use oag_tables::race_campaign::Medal;
use oag_ui_screens::pure_chain::{Row, Status};

use super::pure_list::{Names, PureCampaign, requirement};

/// What `Single Player`'s mode names do. Literal disc idstrings: the ground
/// truth holds the five authored modes to this table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeKind {
    /// `Arcade`: a single race.
    Arcade,
    /// `Tournament`.
    Tournament,
    /// `Time Trial`.
    TimeTrial,
    /// `Zone`.
    Zone,
    /// `Free Play`.
    FreePlay,
}

impl ModeKind {
    /// The mode a `Single Player` entry names.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        match name {
            "Arcade" => Some(Self::Arcade),
            "Tournament" => Some(Self::Tournament),
            "Time Trial" => Some(Self::TimeTrial),
            "Zone" => Some(Self::Zone),
            "Free Play" => Some(Self::FreePlay),
            _ => None,
        }
    }

    /// The kind of event this mode's last screen picks. Free Play picks a
    /// circuit off the same list a single race does, for its gate alone.
    fn event_kind(self) -> EventKind {
        match self {
            Self::Arcade | Self::FreePlay => EventKind::Race,
            Self::Tournament => EventKind::Tournament,
            Self::TimeTrial => EventKind::TimeTrial,
            Self::Zone => EventKind::Zone,
        }
    }
}

/// What a confirmed row on the last pre-race screen starts.
#[derive(Debug, Clone, PartialEq)]
pub enum Start {
    /// A campaign event: its cell launches, judges and records.
    Event(Event),
    /// A plain race on a circuit, in a class, that records nothing.
    FreePlay {
        /// The `PI_Track` name.
        track: String,
        /// The class.
        class: PureClass,
    },
}

/// What a confirm did.
#[derive(Debug, Clone, PartialEq)]
pub enum Chosen {
    /// The chain is on its next screen.
    Advanced,
    /// The row is locked (or names nothing): nothing happened.
    Refused,
    /// The chain is done; `Team Selection` follows.
    Start(Start),
}

/// One screen's worth of rows, already worded.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    /// The screen's title, from the string table under the screen's authored name.
    pub title: String,
    /// What has been chosen so far, in the player's words.
    pub context: String,
    /// The rows.
    pub rows: Vec<Row>,
}

/// What the profile holds, as the rows read it.
#[expect(
    missing_debug_implementations,
    reason = "holds a lookup closure, which has no Debug"
)]
pub struct Profile<'a> {
    /// The best medal a named event holds.
    pub medal_of: &'a dyn Fn(&str) -> Option<Medal>,
    /// `--unlock-all`: every gate open, every class offered.
    pub unlock_all: bool,
    /// The classes a fresh profile offers (`Campaign::classes_at_start`).
    pub classes_at_start: Option<&'a [&'a str]>,
}

/// The chain, between `Single Player` and `Team Selection`.
#[derive(Debug, Clone)]
pub struct Chain {
    campaign: PureCampaign,
    flow: Flow,
    at: Step,
    trail: Vec<Step>,
    mode: usize,
    class: PureClass,
    league: usize,
}

impl Chain {
    /// A chain on `Single Player`, with `class` as the class it will offer
    /// first.
    #[must_use]
    pub fn new(campaign: PureCampaign, flow: Flow, class: PureClass) -> Self {
        Self {
            campaign,
            flow,
            at: Step::Mode,
            trail: Vec::new(),
            mode: 0,
            class,
            league: 0,
        }
    }

    /// The modes `Single Player` lists, by their disc idstrings.
    #[must_use]
    pub fn modes(&self) -> &[String] {
        &self.flow.modes
    }

    /// The screen the player is on.
    #[must_use]
    pub fn at(&self) -> Step {
        self.at
    }

    /// The class last chosen.
    #[must_use]
    pub fn class(&self) -> PureClass {
        self.class
    }

    /// The row tag of the class last chosen, on `Class Selection`.
    #[must_use]
    pub fn class_tag(&self) -> Option<usize> {
        self.flow
            .classes
            .iter()
            .position(|name| name == self.class.title())
    }

    /// The campaign data behind the rows.
    #[must_use]
    pub fn campaign(&self) -> &PureCampaign {
        &self.campaign
    }

    fn mode_kind(&self) -> Option<ModeKind> {
        self.flow.modes.get(self.mode).and_then(|m| ModeKind::of(m))
    }

    /// The classes the profile may choose: every class under `--unlock-all` or a
    /// title that gates none; else those a fresh profile offers, and each later
    /// class once the one before it holds a medal in any tournament.
    ///
    /// **The opening rule for the later three is chosen, not measured.** The
    /// original hides them in code (`Unlock Flash Class`, `Unlock Rapier
    /// Class`, `Unlock Phantom Class` are its reward names, `0x08a7c820`..);
    /// the trigger was not found. It follows the shape of the disc's own
    /// `Tournament="Alpha" class="asSelected" medal="Any"` rows.
    #[must_use]
    pub fn classes_offered(&self, profile: &Profile<'_>) -> Vec<PureClass> {
        let Some(open) = profile.classes_at_start.filter(|_| !profile.unlock_all) else {
            return PureClass::ALL.to_vec();
        };
        let mut offered: Vec<PureClass> = Vec::new();
        for class in PureClass::ALL {
            let starts = open.contains(&class.title());
            let earned = offered.last().is_some_and(|before| {
                self.campaign.definition.tournaments.iter().any(|t| {
                    (profile.medal_of)(&pure_campaign::tournament_event_name(
                        &t.name,
                        before.title(),
                    ))
                    .is_some()
                })
            });
            if starts || earned {
                offered.push(class);
            }
        }
        offered
    }

    fn progress<'a>(&self, profile: &'a Profile<'a>) -> Progress<'a> {
        Progress {
            class: self.class.title(),
            medal_count: self.campaign.medal_count(profile.medal_of),
            medal_of: profile.medal_of,
        }
    }

    fn events_of(&self, kind: EventKind) -> Vec<Event> {
        self.campaign
            .events(self.class)
            .into_iter()
            .filter(|event| event.kind == kind)
            .collect()
    }

    /// The row for an event, locked or open or medalled.
    fn event_row(
        &self,
        label: String,
        tag: usize,
        event: &Event,
        profile: &Profile<'_>,
        names: &Names<'_>,
        show_medal: bool,
    ) -> Row {
        let progress = self.progress(profile);
        let open = profile.unlock_all || pure_campaign::all_met(&event.unlocks, &progress);
        let held = (profile.medal_of)(&event.name).filter(|_| show_medal);
        match (open, held) {
            (_, Some(medal)) => Row::event(
                label,
                tag,
                Status::Medal(medal),
                names.words.medal(medal).to_string(),
            ),
            (true, None) => Row::event(label, tag, Status::Open, String::new()),
            (false, None) => {
                let mut row = Row::event(label, tag, Status::Locked, names.words.locked.clone());
                row.requires = requirement(&event.unlocks, &progress, names);
                row
            }
        }
    }

    /// The screen the player is on, as rows.
    #[must_use]
    pub fn view(&self, profile: &Profile<'_>, names: &Names<'_>) -> View {
        let say = names.name_of;
        let mode_name = self
            .flow
            .modes
            .get(self.mode)
            .map_or(String::new(), |m| say(m));
        let class_name = say(self.class.title());
        let mut context = Vec::new();
        let rows = match self.at {
            Step::Mode => self
                .flow
                .modes
                .iter()
                .enumerate()
                .map(|(tag, mode)| Row::event(say(mode), tag, Status::Open, String::new()))
                .collect(),
            Step::Class => {
                context.push(mode_name);
                let offered = self.classes_offered(profile);
                self.flow
                    .classes
                    .iter()
                    .enumerate()
                    .filter(|(_, name)| offered.iter().any(|c| c.title() == name.as_str()))
                    .map(|(tag, name)| Row::event(say(name), tag, Status::Open, String::new()))
                    .collect()
            }
            Step::League => {
                context.extend([mode_name, class_name]);
                let progress = self.progress(profile);
                self.campaign
                    .definition
                    .leagues
                    .iter()
                    .enumerate()
                    .map(|(tag, league)| {
                        let open = profile.unlock_all
                            || pure_campaign::all_met(&league.unlocks, &progress);
                        let mut row = Row::event(
                            say(&league.name),
                            tag,
                            if open { Status::Open } else { Status::Locked },
                            if open {
                                String::new()
                            } else {
                                names.words.locked.clone()
                            },
                        );
                        if !open {
                            row.requires = requirement(&league.unlocks, &progress, names);
                        }
                        row
                    })
                    .collect()
            }
            Step::Tournament => {
                context.extend([mode_name, class_name]);
                self.events_of(EventKind::Tournament)
                    .iter()
                    .enumerate()
                    .map(|(tag, event)| {
                        self.event_row(say(&event.subject), tag, event, profile, names, true)
                    })
                    .collect()
            }
            Step::Track => {
                let league = self.campaign.definition.leagues.get(self.league);
                context.extend([mode_name, class_name]);
                if let Some(league) = league {
                    context.push(say(&league.name));
                }
                let kind = self
                    .mode_kind()
                    .map_or(EventKind::Race, ModeKind::event_kind);
                let events = self.events_of(kind);
                league
                    .map(|league| league.tracks.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .filter_map(|(tag, track)| {
                        // A Free Play row has no medal of its own; the race
                        // event only carries the circuit's gate.
                        let event = events.iter().find(|e| &e.subject == track)?;
                        let show = self.mode_kind() != Some(ModeKind::FreePlay);
                        Some(self.event_row(say(track), tag, event, profile, names, show))
                    })
                    .collect()
            }
            Step::ZoneTrack => {
                context.push(mode_name);
                pure_campaign::zone_events(&self.campaign.definition, &self.campaign.stats)
                    .iter()
                    .enumerate()
                    .map(|(tag, event)| {
                        self.event_row(say(&event.subject), tag, event, profile, names, true)
                    })
                    .collect()
            }
            Step::Team | Step::Launch => Vec::new(),
        };
        View {
            title: say(self.at.screen()),
            context: context.join("  -  "),
            rows,
        }
    }

    /// Chooses row `tag` on the current screen.
    pub fn choose(&mut self, tag: usize, profile: &Profile<'_>) -> Chosen {
        match self.at {
            Step::Mode => {
                if self.flow.modes.get(tag).is_none() {
                    return Chosen::Refused;
                }
                self.mode = tag;
                self.advance()
            }
            Step::Class => {
                let Some(class) = self
                    .flow
                    .classes
                    .get(tag)
                    .and_then(|name| PureClass::ALL.into_iter().find(|c| c.title() == name))
                else {
                    return Chosen::Refused;
                };
                if !self.classes_offered(profile).contains(&class) {
                    return Chosen::Refused;
                }
                self.class = class;
                self.advance()
            }
            Step::League => {
                let progress = self.progress(profile);
                let Some(league) = self.campaign.definition.leagues.get(tag) else {
                    return Chosen::Refused;
                };
                if !(profile.unlock_all || pure_campaign::all_met(&league.unlocks, &progress)) {
                    return Chosen::Refused;
                }
                self.league = tag;
                self.advance()
            }
            Step::Tournament | Step::Track | Step::ZoneTrack => self.choose_event(tag, profile),
            Step::Team | Step::Launch => Chosen::Refused,
        }
    }

    fn choose_event(&mut self, tag: usize, profile: &Profile<'_>) -> Chosen {
        let progress = self.progress(profile);
        let kind = self
            .mode_kind()
            .map_or(EventKind::Race, ModeKind::event_kind);
        let event = match self.at {
            Step::Tournament => self.events_of(EventKind::Tournament).get(tag).cloned(),
            Step::ZoneTrack => {
                pure_campaign::zone_events(&self.campaign.definition, &self.campaign.stats)
                    .get(tag)
                    .cloned()
            }
            _ => {
                let track = self
                    .campaign
                    .definition
                    .leagues
                    .get(self.league)
                    .and_then(|league| league.tracks.get(tag))
                    .cloned();
                track.and_then(|track| {
                    self.events_of(kind)
                        .into_iter()
                        .find(|e| e.subject == track)
                })
            }
        };
        let Some(event) = event else {
            return Chosen::Refused;
        };
        if !(profile.unlock_all || pure_campaign::all_met(&event.unlocks, &progress)) {
            return Chosen::Refused;
        }
        if self.mode_kind() == Some(ModeKind::FreePlay) {
            return Chosen::Start(Start::FreePlay {
                track: event.subject,
                class: self.class,
            });
        }
        Chosen::Start(Start::Event(event))
    }

    fn advance(&mut self) -> Chosen {
        let mode = self.flow.modes.get(self.mode).cloned().unwrap_or_default();
        match self.flow.next(self.at, &mode) {
            Ok(Some(next)) => {
                self.trail.push(self.at);
                self.at = next;
                Chosen::Advanced
            }
            _ => Chosen::Refused,
        }
    }

    /// Steps back one screen; `false` when the chain is on its first and
    /// should close.
    pub fn back(&mut self) -> bool {
        match self.trail.pop() {
            Some(previous) => {
                self.at = previous;
                true
            }
            None => false,
        }
    }

    /// Puts the chain on the screen that picked `event` (a race ended, or the
    /// team screen was backed out of), with the cursor to be put on it by the
    /// caller.
    pub fn return_to(&mut self, event: &Event) {
        let kind = match event.kind {
            EventKind::Tournament => Step::Tournament,
            EventKind::Zone => Step::ZoneTrack,
            EventKind::Race | EventKind::TimeTrial => Step::Track,
        };
        if let Some(class) = event.class {
            self.class = class;
        }
        if kind == Step::Track
            && let Some(index) = self
                .campaign
                .definition
                .leagues
                .iter()
                .position(|league| league.tracks.contains(&event.subject))
        {
            self.league = index;
        }
        let want = match event.kind {
            EventKind::Tournament => "Tournament",
            EventKind::Zone => "Zone",
            EventKind::Race => "Arcade",
            EventKind::TimeTrial => "Time Trial",
        };
        if let Some(mode) = self.flow.modes.iter().position(|m| m == want) {
            self.mode = mode;
        }
        // Rebuild the trail by walking the route up to `kind`.
        self.at = Step::Mode;
        self.trail.clear();
        if let Some(mode) = self.flow.modes.get(self.mode).cloned()
            && let Ok(route) = self.flow.route(&mode)
        {
            for step in route {
                if step == kind {
                    self.trail.push(self.at);
                    self.at = step;
                    break;
                }
                self.trail.push(self.at);
                self.at = step;
            }
        }
    }

    /// The row tag on the current screen that `event` is, for the cursor.
    #[must_use]
    pub fn tag_of(&self, event: &Event) -> Option<usize> {
        match self.at {
            Step::Tournament => self
                .events_of(EventKind::Tournament)
                .iter()
                .position(|e| e.name == event.name),
            Step::ZoneTrack => {
                pure_campaign::zone_events(&self.campaign.definition, &self.campaign.stats)
                    .iter()
                    .position(|e| e.name == event.name)
            }
            Step::Track => self
                .campaign
                .definition
                .leagues
                .get(self.league)
                .and_then(|league| league.tracks.iter().position(|t| *t == event.subject)),
            _ => None,
        }
    }
}

/// Chooses row `tag` on the screen the chain is on, against what `records`
/// hold.
pub fn choose(
    chain: &mut Chain,
    tag: usize,
    title: &oag_title::Title,
    records: &crate::records::Store,
) -> Chosen {
    let medal_of = crate::unlock::campaign_medal_of(records, title.name);
    chain.choose(
        tag,
        &Profile {
            medal_of: &medal_of,
            unlock_all: crate::unlock::unlock_all(),
            classes_at_start: title.campaign.classes_at_start,
        },
    )
}

/// The screen the chain is on as the player sees it: the disc's names
/// upper-cased as the menus' own rows are, the profile's medals read from
/// `records`.
#[must_use]
pub fn present(
    chain: &Chain,
    strings: &oag_ui::language::StringTable,
    title: &oag_title::Title,
    records: &crate::records::Store,
) -> View {
    let say = |name: &str| strings.get(name).unwrap_or(name).to_uppercase();
    let words = super::pure_list::wording(strings);
    let medal_of = crate::unlock::campaign_medal_of(records, title.name);
    let profile = Profile {
        medal_of: &medal_of,
        unlock_all: crate::unlock::unlock_all(),
        classes_at_start: title.campaign.classes_at_start,
    };
    chain.view(
        &profile,
        &Names {
            name_of: &say,
            words: &words,
        },
    )
}

#[cfg(test)]
mod tests;
