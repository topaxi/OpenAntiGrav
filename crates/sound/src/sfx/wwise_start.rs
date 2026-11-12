//! A cue played as the Wwise event a title posts for it: every sound the
//! event's actions start, each a [`Layer`] keyed on after its own delay.
//!
//! Omega's race start (`docs/ghidra/functions/ps4-omega-eu/wwise-sound.md`,
//! "The countdown, end to end"): `Play` actions on sounds and on layer
//! containers, whose children all sound together; a relay (`0x2103`) plays
//! another event of the same bank; a `Stop` cuts what plays below its target
//! with a fade; a seek jumps a playing sound to a fraction of its length. All of
//! it is decided at load time, so the result is a fixed [`Timeline`] the
//! existing voice path plays.
//!
//! Read: which sounds, the action delays, the nodes' initial delays and volume
//! offsets (summed down the actor-mixer tree), the stop time and fade, the seek
//! fraction, and the loops: a sound's loop count ([`prop::LOOP`]) and a
//! continuous container's (`docs/formats/wwise.md`, "Loops"). A sound that
//! plays forever is a looping [`Layer`], held until the game releases the cue;
//! one that plays `n` times is its media `n` times over. A random/sequence container sounds exactly one child per post:
//! the first playlist item of a sequence, a uniform draw from `rng` for a
//! random one (a container that weights its picks is refused, none does). Not
//! read, so refused or reported: switch containers (which child plays), streamed media, a relay into another bank,
//! state and other action kinds, the busses' own volumes and every positioning
//! parameter (the start plays centred). A seek's randomised delay range plays at
//! its minimum, chosen.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, bail, ensure};
use oag_audio::Sound;
use oag_core::Rng;
use oag_formats::wwise::container::RanSeq;
use oag_formats::wwise::hirc::{self, Action, Event, Kind, Seek, Sound as WwiseSound, prop};
use oag_formats::wwise::{Bank, name_hash};

use super::banks::Loaded;
use super::layers::{Layer, Timeline};

/// What an event loads as: the voices, and what was left out.
pub(super) struct EventCue {
    pub(super) loaded: Loaded,
    pub(super) timeline: Timeline,
    /// One line per thing the event does that does not play.
    pub(super) notes: Vec<String>,
    /// Each random/sequence container reached and the child it played.
    pub(super) picks: Vec<(u32, u32)>,
}

/// A sound an action starts, before its media is cut by stops and seeks.
struct Start {
    sound: u32,
    at: f64,
    repeat: Repeat,
}

/// How many times a started sound plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Times(u32),
    Forever,
}

impl Repeat {
    const ONCE: Self = Self::Times(1);

    /// A Wwise loop count: `0` is forever.
    fn of_count(count: i64) -> Self {
        match u32::try_from(count) {
            Ok(0) | Err(_) => Self::Forever,
            Ok(n) => Self::Times(n),
        }
    }

    /// `self` played inside something that plays `outer` times.
    fn within(self, outer: Self) -> Self {
        match (self, outer) {
            (Self::Times(a), Self::Times(b)) => Self::Times(a.saturating_mul(b)),
            _ => Self::Forever,
        }
    }
}

/// The longest a counted loop is laid out as media; past it, the sound loops
/// until released. Chosen: the player's autopilot counts 200 plays of its
/// container, minutes of audio no held cue lasts.
const MAX_COUNTED_SECONDS: f64 = 30.0;

/// `frames` (of `channels` interleaved samples) played `times` over.
fn tile(frames: &[i16], times: usize) -> Vec<i16> {
    let mut out = Vec::with_capacity(frames.len() * times);
    for _ in 0..times {
        out.extend_from_slice(frames);
    }
    out
}

/// A cut that applies to every sound below `target`.
struct Cut {
    target: u32,
    at: f64,
    what: CutKind,
}

enum CutKind {
    Stop { fade: f64 },
    Seek { fraction: f64 },
}

/// The draws a load makes.
struct Picker<'a> {
    rng: &'a mut Rng,
    made: Vec<(u32, u32)>,
}

/// The objects of one bank, with every node's children.
struct Tree<'a> {
    bank: &'a Bank<'a>,
    children: BTreeMap<u32, Vec<u32>>,
}

impl<'a> Tree<'a> {
    fn new(bank: &'a Bank<'a>) -> Self {
        let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for object in bank.objects() {
            if let Some(parent) = hirc::parent(object.kind, bank.body(object)) {
                children.entry(parent).or_default().push(object.id);
            }
        }
        Self { bank, children }
    }

    fn kind_and_body(&self, id: u32) -> Option<(Kind, &'a [u8])> {
        let object = self.bank.object(id)?;
        Some((object.kind, self.bank.body(object)))
    }

    fn parent(&self, id: u32) -> Option<u32> {
        let (kind, body) = self.kind_and_body(id)?;
        hirc::parent(kind, body)
    }

    /// `id`'s initial delay in seconds.
    fn initial_delay(&self, id: u32) -> f64 {
        self.kind_and_body(id)
            .and_then(|(kind, body)| hirc::node_props(kind, body))
            .and_then(|p| p.float(prop::INITIAL_DELAY))
            .map_or(0.0, f64::from)
    }

    /// The volume offsets of `id` and every node above it, summed, in dB.
    fn volume_db(&self, id: u32) -> f64 {
        let mut total = 0.0;
        let mut at = Some(id);
        while let Some(node) = at {
            if let Some((kind, body)) = self.kind_and_body(node) {
                total += hirc::node_props(kind, body)
                    .and_then(|p| p.float(prop::VOLUME))
                    .map_or(0.0, f64::from);
            }
            at = self.parent(node);
        }
        total
    }

    /// Whether `id` is `ancestor` or sits below it.
    fn is_below(&self, id: u32, ancestor: u32) -> bool {
        let mut at = Some(id);
        while let Some(node) = at {
            if node == ancestor {
                return true;
            }
            at = self.parent(node);
        }
        false
    }

    /// The sounds playing `target` starts at `at`: the sound itself, or every
    /// child of a layer container or actor-mixer, each after its own initial
    /// delay.
    ///
    /// `repeat` is how often the containers above play what they start: a
    /// continuous container plays its pick its own loop count of times.
    fn starts(
        &self,
        target: u32,
        at: f64,
        repeat: Repeat,
        pick: &mut Picker<'_>,
        out: &mut Vec<Start>,
    ) -> anyhow::Result<()> {
        let (kind, body) = self
            .kind_and_body(target)
            .with_context(|| format!("target {target} is in no object of the bank"))?;
        let at = at + self.initial_delay(target);
        match kind {
            Kind::Sound => {
                let own = hirc::node_props(kind, body)
                    .and_then(|p| p.int(prop::LOOP))
                    .map_or(Repeat::ONCE, |n| Repeat::of_count(i64::from(n)));
                out.push(Start {
                    sound: target,
                    at,
                    repeat: own.within(repeat),
                });
            }
            Kind::LayerContainer | Kind::ActorMixer => {
                for &child in self.children.get(&target).into_iter().flatten() {
                    self.starts(child, at, repeat, pick, out)?;
                }
            }
            Kind::RanSeqContainer => {
                let container =
                    RanSeq::parse(kind, body).with_context(|| format!("container {target}"))?;
                ensure!(
                    !container.uses_weights(),
                    "container {target} weights its picks, which is not read"
                );
                ensure!(
                    !container.playlist.is_empty(),
                    "container {target} is empty"
                );
                let item = if container.is_sequence() {
                    0
                } else {
                    pick.rng.below(container.playlist.len() as u32) as usize
                };
                let child = container.playlist[item].id;
                pick.made.push((target, child));
                // A continuous container plays on through its playlist; the one
                // pick made here stands for every item it would play (chosen).
                let repeat = if container.is_continuous() {
                    Repeat::of_count(i64::from(container.loop_count)).within(repeat)
                } else {
                    repeat
                };
                self.starts(child, at, repeat, pick, out)?;
            }
            other => bail!("target {target} is a {other:?}, whose choice of child is not read"),
        }
        Ok(())
    }
}

/// Loads `event` (its full name, as the title posts it) out of the Wwise bank
/// `blob`.
///
/// # Errors
///
/// The bank does not parse, has no such event, an action starts something
/// whose choice is unread (a switch, a weighted container), or no sound it starts decodes.
pub(super) fn load(blob: &[u8], event: &str, rng: &mut Rng) -> anyhow::Result<EventCue> {
    let bank = Bank::parse(blob).map_err(|e| anyhow::anyhow!("not a Wwise bank: {e}"))?;
    let tree = Tree::new(&bank);
    let mut starts = Vec::new();
    let mut cuts = Vec::new();
    let mut notes = Vec::new();
    let mut pick = Picker {
        rng,
        made: Vec::new(),
    };
    let mut queue = vec![(name_hash(event), 0.0)];
    let mut seen = Vec::new();
    while let Some((id, base)) = queue.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some((Kind::Event, body)) = tree.kind_and_body(id) else {
            if id == name_hash(event) {
                bail!("no event {event} in the bank");
            }
            notes.push(format!("relay to event {id}: not in this bank, not played"));
            continue;
        };
        let actions = Event::parse(body).context("the event's body")?.actions;
        for action_id in actions {
            let body = tree
                .kind_and_body(action_id)
                .filter(|(k, _)| *k == Kind::Action)
                .map(|(_, b)| b)
                .with_context(|| format!("action {action_id} is not in the bank"))?;
            let action = Action::parse(body).context("an action")?;
            let props = Action::props(body).unwrap_or_default();
            let at = base + f64::from(props.int(prop::DELAY_MS).unwrap_or(0)) / 1000.0;
            match action.kind {
                Action::PLAY => {
                    tree.starts(action.target, at, Repeat::ONCE, &mut pick, &mut starts)?;
                }
                Action::PLAY_EVENT => queue.push((action.target, at)),
                Action::STOP => cuts.push(Cut {
                    target: action.target,
                    at,
                    what: CutKind::Stop {
                        fade: f64::from(props.int(prop::FADE_MS).unwrap_or(0)) / 1000.0,
                    },
                }),
                kind if Seek::is_seek(kind) => {
                    let seek = Seek::parse(body).context("a seek")?;
                    ensure!(seek.relative, "a seek to an absolute time is not read");
                    cuts.push(Cut {
                        target: action.target,
                        at,
                        what: CutKind::Seek {
                            fraction: f64::from(seek.value),
                        },
                    });
                }
                kind => notes.push(format!("action kind {kind:#06x} not played")),
            }
        }
    }

    let mut layers = Vec::new();
    for start in &starts {
        let (_, body) = tree.kind_and_body(start.sound).expect("a start is a sound");
        let sound = WwiseSound::parse(body).context("a sound")?;
        let Some(media) = bank.embedded(sound.source.media_id) else {
            notes.push(format!(
                "sound {}: media {} is not embedded in this bank, not played",
                start.sound, sound.source.media_id
            ));
            continue;
        };
        let pcm = oag_music::wem::decode(media)
            .with_context(|| format!("media {} does not decode", sound.source.media_id))?;
        let channels = usize::from(pcm.channels);
        let rate = f64::from(pcm.sample_rate);
        let mut frames: Vec<i16> = pcm.samples;
        let mut cut_at: Option<(f64, f64)> = None;
        let mut applied: Vec<&Cut> = cuts
            .iter()
            .filter(|c| tree.is_below(start.sound, c.target))
            .collect();
        applied.sort_by(|a, b| a.at.total_cmp(&b.at));
        let once = (frames.len() / channels) as f64 / rate;
        let mut looping = false;
        match start.repeat {
            Repeat::Times(1) => {}
            Repeat::Times(n) if f64::from(n) * once <= MAX_COUNTED_SECONDS => {
                frames = tile(&frames, n as usize);
            }
            Repeat::Times(_) | Repeat::Forever => looping = true,
        }
        // A loop this event also cuts plays out to its last cut as media, then
        // ends: a looping voice would repeat the cut.
        if looping && let Some(last) = applied.last() {
            let fade = match last.what {
                CutKind::Stop { fade } => fade,
                CutKind::Seek { .. } => 0.0,
            };
            let need = last.at - start.at + fade + once;
            if once > 0.0 {
                frames = tile(&frames, (need / once).ceil().max(1.0) as usize);
            }
            looping = false;
            notes.push(format!(
                "sound {}: a loop this event cuts, laid out to {need:.2} s",
                start.sound
            ));
        }
        // Cuts in time order, each on the media as the earlier ones left it.
        let mut played = 0.0f64;
        let mut timeline_start = start.at;
        for cut in applied {
            if cut.at <= start.at {
                if matches!(cut.what, CutKind::Stop { .. }) {
                    frames.clear();
                }
                continue;
            }
            let into = cut.at - timeline_start + played;
            match cut.what {
                CutKind::Seek { fraction } => {
                    let total = frames.len() / channels;
                    let keep = ((into * rate) as usize).min(total);
                    let jump = ((fraction * total as f64) as usize).min(total);
                    let mut spliced = frames[..keep * channels].to_vec();
                    spliced.extend_from_slice(&frames[jump * channels..]);
                    frames = spliced;
                    played = keep as f64 / rate;
                    timeline_start = cut.at;
                }
                CutKind::Stop { fade } => {
                    cut_at = Some((into, fade));
                    break;
                }
            }
        }
        if let Some((into, fade)) = cut_at {
            let end = (((into + fade) * rate) as usize).min(frames.len() / channels);
            frames.truncate(end * channels);
            let fade_frames = ((fade * rate) as usize).min(end);
            for f in 0..fade_frames {
                let g = (fade_frames - f) as f64 / fade_frames as f64;
                let frame = end - fade_frames + f;
                for s in &mut frames[frame * channels..(frame + 1) * channels] {
                    *s = (f64::from(*s) * g) as i16;
                }
            }
        }
        if frames.is_empty() {
            continue;
        }
        let gain = 10f64.powf(tree.volume_db(start.sound) / 20.0);
        for s in &mut frames {
            *s = (f64::from(*s) * gain).clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
        }
        let sound = Arc::new(Sound::new(frames, pcm.channels, pcm.sample_rate)?);
        layers.push(Layer {
            sound,
            looping,
            delay: start.at,
            angle: 0,
            rear_edge: false,
            bend: None,
        });
    }
    for cut in &cuts {
        if !starts.iter().any(|s| tree.is_below(s.sound, cut.target)) {
            let what = match cut.what {
                CutKind::Stop { .. } => "stop",
                CutKind::Seek { .. } => "seek",
            };
            notes.push(format!(
                "{what} on {} cuts no sound this event starts",
                cut.target
            ));
        }
    }
    ensure!(!layers.is_empty(), "{event} starts no sound that plays");
    let loaded = Loaded {
        waveforms: layers
            .iter()
            .map(|l| (Arc::clone(&l.sound), l.looping))
            .collect(),
    };
    Ok(EventCue {
        loaded,
        timeline: Timeline {
            layers,
            bends: Vec::new(),
        },
        notes,
        picks: pick.made,
    })
}
