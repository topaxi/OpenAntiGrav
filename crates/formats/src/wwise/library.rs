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
use super::hirc::{self, Action, Event, Kind, MusicTrack, SetState, Sound, Source, StreamType};
use super::music::{Clip, RanSeq, Segment, Switch, track_clips, track_type};

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

/// One song reached from a state: a music random/sequence container's
/// segments, and each segment's tracks with what they play.
#[derive(Debug, Clone, PartialEq)]
pub struct SongChain {
    /// The music switch the state was found in.
    pub switch: u32,
    pub ranseq: u32,
    pub segments: Vec<SegmentChain>,
}

/// One segment of a [`SongChain`].
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentChain {
    pub segment: u32,
    pub duration: f64,
    pub tracks: Vec<TrackChain>,
}

/// One track of a [`SegmentChain`].
#[derive(Debug, Clone, PartialEq)]
pub struct TrackChain {
    pub track: u32,
    /// `0` for a normal track.
    pub track_type: Option<u8>,
    pub clips: Vec<Clip>,
    pub media: Vec<MediaRef>,
}

/// Why a walk from a state stopped, by the id it stopped at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkError {
    /// An id no bank of the library defines.
    Missing(u32),
    /// An object of the wrong kind at this id.
    WrongKind(u32),
    /// A music object that its reader refused.
    Unread(u32, super::music::MusicError),
    /// A switch whose state values select nothing, at this switch id.
    NoSelection(u32),
    /// A switch's target is neither a switch nor a container.
    Dead(u32),
}

impl Library<'_> {
    fn first(&self, id: u32) -> Option<(Kind, &[u8])> {
        let &(b, o) = self.objects.get(&id)?.first()?;
        let bank = &self.banks[b];
        let object = &bank.objects()[o];
        Some((object.kind, bank.body(object)))
    }

    /// The `SetState` actions of an event, in action order, or `None` when no
    /// bank has the event.
    #[must_use]
    pub fn set_states(&self, event: u32) -> Option<Vec<SetState>> {
        let mut found = false;
        let mut out = Vec::new();
        for &(b, o) in self.objects.get(&event)? {
            let bank = &self.banks[b];
            let object = &bank.objects()[o];
            if object.kind != Kind::Event {
                continue;
            }
            found = true;
            for id in Event::parse(bank.body(object))?.actions {
                if let Some((Kind::Action, body)) = self.first(id)
                    && let Some(set) = SetState::parse(body)
                {
                    out.push(set);
                }
            }
        }
        found.then_some(out)
    }

    /// Walks from a music object down through its switches, with each group
    /// at the state `states` gives it (`none` for a group not listed, then
    /// key `0`, the tree's "any"), to the song a random/sequence container
    /// plays.
    ///
    /// # Errors
    ///
    /// Where the walk stopped: an unresolved id, a refused reader, a switch
    /// that selects nothing.
    pub fn walk_music(
        &self,
        start: u32,
        states: &[(u32, u32)],
        none: u32,
    ) -> Result<SongChain, WalkError> {
        let mut at = start;
        for _ in 0..16 {
            let (kind, body) = self.first(at).ok_or(WalkError::Missing(at))?;
            match kind {
                Kind::MusicRanSeq => return self.song(start, at),
                Kind::MusicSwitch => {
                    let switch = Switch::parse(kind, body).map_err(|e| WalkError::Unread(at, e))?;
                    let value = |group: u32| {
                        states
                            .iter()
                            .find(|(g, _)| *g == group)
                            .map_or(none, |&(_, s)| s)
                    };
                    let wanted: Vec<u32> = switch.groups.iter().map(|&(g, _)| value(g)).collect();
                    let target = switch
                        .select(&wanted)
                        .or_else(|| switch.select(&vec![0; wanted.len()]))
                        .filter(|&t| t != 0)
                        .ok_or(WalkError::NoSelection(at))?;
                    at = target;
                }
                _ => return Err(WalkError::Dead(at)),
            }
        }
        Err(WalkError::Dead(at))
    }

    /// Walks one container to its segments, tracks and media.
    ///
    /// # Errors
    ///
    /// The id the walk stopped at.
    pub fn song(&self, switch: u32, ranseq: u32) -> Result<SongChain, WalkError> {
        let (kind, body) = self.first(ranseq).ok_or(WalkError::Missing(ranseq))?;
        let parsed = RanSeq::parse(kind, body).map_err(|e| WalkError::Unread(ranseq, e))?;
        let mut segments = Vec::new();
        for id in parsed.segments() {
            let (kind, body) = self.first(id).ok_or(WalkError::Missing(id))?;
            let segment = Segment::parse(kind, body).map_err(|e| WalkError::Unread(id, e))?;
            let mut tracks = Vec::new();
            for &track in &segment.node.children {
                let (kind, body) = self.first(track).ok_or(WalkError::Missing(track))?;
                if kind != Kind::MusicTrack {
                    return Err(WalkError::WrongKind(track));
                }
                let sources = MusicTrack::parse(body).ok_or(WalkError::WrongKind(track))?;
                let clips = track_clips(body).map_err(|e| WalkError::Unread(track, e))?;
                tracks.push(TrackChain {
                    track,
                    track_type: track_type(body),
                    clips,
                    media: sources
                        .sources
                        .into_iter()
                        .map(|source| MediaRef {
                            source,
                            embedded_in: self.embedded.get(&source.media_id).copied(),
                        })
                        .collect(),
                });
            }
            segments.push(SegmentChain {
                segment: id,
                duration: segment.duration,
                tracks,
            });
        }
        Ok(SongChain {
            switch,
            ranseq,
            segments,
        })
    }
}
