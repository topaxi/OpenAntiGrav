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
//! fraction. Not read, so refused or reported: random/sequence and switch
//! containers (which child plays), streamed media, a relay into another bank,
//! state and other action kinds, the busses' own volumes and every positioning
//! parameter (the start plays centred). A seek's randomised delay range plays at
//! its minimum, chosen.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, bail, ensure};
use oag_audio::Sound;
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
}

/// A sound an action starts, before its media is cut by stops and seeks.
struct Start {
    sound: u32,
    at: f64,
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
    fn starts(&self, target: u32, at: f64, out: &mut Vec<Start>) -> anyhow::Result<()> {
        let (kind, _) = self
            .kind_and_body(target)
            .with_context(|| format!("target {target} is in no object of the bank"))?;
        let at = at + self.initial_delay(target);
        match kind {
            Kind::Sound => out.push(Start { sound: target, at }),
            Kind::LayerContainer | Kind::ActorMixer => {
                for &child in self.children.get(&target).into_iter().flatten() {
                    self.starts(child, at, out)?;
                }
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
/// whose choice is unread, or no sound it starts decodes.
pub(super) fn load(blob: &[u8], event: &str) -> anyhow::Result<EventCue> {
    let bank = Bank::parse(blob).map_err(|e| anyhow::anyhow!("not a Wwise bank: {e}"))?;
    let tree = Tree::new(&bank);
    let mut starts = Vec::new();
    let mut cuts = Vec::new();
    let mut notes = Vec::new();
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
                Action::PLAY => tree.starts(action.target, at, &mut starts)?,
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
            looping: false,
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
            .map(|l| (Arc::clone(&l.sound), false))
            .collect(),
    };
    Ok(EventCue {
        loaded,
        timeline: Timeline {
            layers,
            bends: Vec::new(),
        },
        notes,
    })
}
