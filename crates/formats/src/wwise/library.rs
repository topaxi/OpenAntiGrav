//! Banks read together: event to media across bank boundaries.
//!
//! A `Play` action names its target by id and says the target is in its own
//! bank, and that is not always so: 1,450 of the 4,957 `Play` actions in the
//! base and patch banks (`data00`, `data05`, `data08`) name an object no shipped
//! bank defines. [`Library`] resolves what it can across whichever banks it is
//! given and reports the rest by id, so a caller draws or plays nothing for
//! them rather than something plausible.

use std::collections::{BTreeMap, BTreeSet};

use super::Bank;
use super::hirc::{self, Action, Event, Kind, MusicTrack, Sound, Source, StreamType};

/// Where an object is: which bank, which entry of its `objects()`.
type Location = (usize, usize);

/// A set of banks with the indices to walk between them.
#[derive(Debug)]
pub struct Library<'a> {
    banks: Vec<Bank<'a>>,
    objects: BTreeMap<u32, Vec<Location>>,
    children: BTreeMap<u32, Vec<Location>>,
    embedded: BTreeMap<u32, usize>,
}

/// One piece of media an event reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRef {
    pub source: Source,
    /// The bank whose `DIDX` holds the media, when one does - not always the
    /// bank that names it: 140 of 145 embedded sounds without a `DIDX` record
    /// of their own are in another bank's.
    pub embedded_in: Option<usize>,
}

impl MediaRef {
    /// Whether the media has to be read from a loose `<id>.wem`.
    #[must_use]
    pub fn is_loose(&self) -> bool {
        matches!(
            self.source.stream,
            StreamType::Streamed | StreamType::Prefetch
        )
    }
}

/// What playing an event reaches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventPlan {
    /// The `Play` targets, in action order.
    pub targets: Vec<u32>,
    /// Every distinct media the targets' sounds and tracks name.
    pub media: Vec<MediaRef>,
    /// Targets, or descendants, that are in no bank of the library.
    pub unresolved: Vec<u32>,
    /// Targets of a kind whose children this crate does not read (music
    /// segments, switches and sequences).
    pub unread: Vec<u32>,
}

impl<'a> Library<'a> {
    /// Indexes `banks`: every object by id, every node under its parent, every
    /// `DIDX` record by media id.
    #[must_use]
    pub fn new(banks: Vec<Bank<'a>>) -> Self {
        let mut objects: BTreeMap<u32, Vec<Location>> = BTreeMap::new();
        let mut children: BTreeMap<u32, Vec<Location>> = BTreeMap::new();
        let mut embedded = BTreeMap::new();
        for (b, bank) in banks.iter().enumerate() {
            for (o, object) in bank.objects().iter().enumerate() {
                objects.entry(object.id).or_default().push((b, o));
                if let Some(parent) = hirc::parent(object.kind, bank.body(object)) {
                    children.entry(parent).or_default().push((b, o));
                }
            }
            for media in bank.media() {
                embedded.entry(media.id).or_insert(b);
            }
        }
        Self {
            banks,
            objects,
            children,
            embedded,
        }
    }

    /// The banks, in the order they were given.
    #[must_use]
    pub fn banks(&self) -> &[Bank<'a>] {
        &self.banks
    }

    /// The bank whose `DIDX` has this media id.
    #[must_use]
    pub fn bank_with_media(&self, id: u32) -> Option<usize> {
        self.embedded.get(&id).copied()
    }

    /// The `Play` targets of an event, in action order, or `None` when no bank
    /// has an event with this id.
    #[must_use]
    pub fn play_targets(&self, event: u32) -> Option<Vec<u32>> {
        let locations = self.objects.get(&event)?;
        let mut found = false;
        let mut targets = Vec::new();
        for &(b, o) in locations {
            let bank = &self.banks[b];
            let object = &bank.objects()[o];
            if object.kind != Kind::Event {
                continue;
            }
            found = true;
            let Some(parsed) = Event::parse(bank.body(object)) else {
                continue;
            };
            for id in parsed.actions {
                for &(ab, ao) in self.objects.get(&id).into_iter().flatten() {
                    let action_bank = &self.banks[ab];
                    let action = &action_bank.objects()[ao];
                    if action.kind != Kind::Action {
                        continue;
                    }
                    if let Some(action) = Action::parse(action_bank.body(action))
                        && action.play.is_some()
                        && !targets.contains(&action.target)
                    {
                        targets.push(action.target);
                    }
                }
            }
        }
        found.then_some(targets)
    }

    /// What playing an event reaches, or `None` when no bank has it.
    #[must_use]
    pub fn resolve_event(&self, event: u32) -> Option<EventPlan> {
        let targets = self.play_targets(event)?;
        let mut plan = EventPlan {
            targets: targets.clone(),
            ..EventPlan::default()
        };
        let mut seen = BTreeSet::new();
        for target in targets {
            self.descend(target, &mut seen, &mut plan);
        }
        Some(plan)
    }

    fn descend(&self, id: u32, seen: &mut BTreeSet<u32>, plan: &mut EventPlan) {
        if !seen.insert(id) {
            return;
        }
        let Some(locations) = self.objects.get(&id) else {
            plan.unresolved.push(id);
            return;
        };
        for &(b, o) in locations {
            let bank = &self.banks[b];
            let object = &bank.objects()[o];
            let body = bank.body(object);
            match object.kind {
                Kind::Sound => {
                    if let Some(sound) = Sound::parse(body) {
                        self.push_media(sound.source, plan);
                    }
                }
                Kind::MusicTrack => {
                    for source in MusicTrack::parse(body).into_iter().flat_map(|t| t.sources) {
                        self.push_media(source, plan);
                    }
                }
                Kind::RanSeqContainer
                | Kind::SwitchContainer
                | Kind::ActorMixer
                | Kind::LayerContainer => {
                    let below: Vec<u32> = self
                        .children
                        .get(&id)
                        .into_iter()
                        .flatten()
                        .map(|&(cb, co)| self.banks[cb].objects()[co].id)
                        .collect();
                    for child in below {
                        self.descend(child, seen, plan);
                    }
                }
                Kind::MusicSegment | Kind::MusicSwitch | Kind::MusicRanSeq
                    if !plan.unread.contains(&id) =>
                {
                    plan.unread.push(id);
                }
                _ => {}
            }
        }
    }

    fn push_media(&self, source: Source, plan: &mut EventPlan) {
        let media = MediaRef {
            source,
            embedded_in: self.embedded.get(&source.media_id).copied(),
        };
        if !plan.media.contains(&media) {
            plan.media.push(media);
        }
    }
}
