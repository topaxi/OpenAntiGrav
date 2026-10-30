//! Events between mods, and from a track's triggers (`hooks.md`, "Events between mods").
//!
//! A module's `emit{channel, slot, value}` and a craft entering a trigger whose
//! `oag_emit` names a channel both end as a [`Mail`] in the race's queue, delivered to
//! **every** module as `Event::Mod` at the next call, sorted so that no module's id, and
//! so no module's place in the order, changes what any module sees. The queue is race
//! state: hashed with the modules' state (and so not at all in a race with no module).
//!
//! A trigger needs no module to raise its channel: [`Beacons`] tests the track's emitting
//! triggers whether or not a module runs, and [`Extensions::fired`] keeps when each
//! channel last fired for the view's animations (`oag_on`). That log is an output, never
//! hashed.

use std::sync::Arc;

use oag_core::math::Vec3;
use oag_trackfile::props::Trigger;
use oag_vex::pads::PadVolume;

/// The channels a mod declares in `oag-mod.toml`'s `[channels]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declared {
    /// Channels its modules may `emit` on.
    pub publishes: Vec<String>,
    /// Channels it hears: shown on the mods page, warned about when nobody publishes one.
    pub listens: Vec<String>,
}

/// A trigger box as the engine tests it: armed and unexpanded, the author's.
pub(super) fn box_of(trigger: &Trigger) -> PadVolume {
    PadVolume {
        to_world: trigger.to_world,
        min: trigger.min,
        max: trigger.max,
        disabled: 0.0,
    }
}

/// The track's triggers that send a channel: tested every tick a craft steps, with or
/// without a module, so an animation can start on a track no mod is on.
#[derive(Debug, Clone, Default)]
pub(super) struct Beacons {
    boxes: Arc<[PadVolume]>,
    channels: Arc<[u32]>,
    /// One bit per beacon a slot is inside now.
    inside: [u64; oag_gameplay::MAX_SHIPS],
}

impl Beacons {
    pub(super) fn new(triggers: &[Trigger]) -> Self {
        let sending: Vec<(PadVolume, u32)> = triggers
            .iter()
            .filter_map(|t| {
                t.emits
                    .as_deref()
                    .map(|name| (box_of(t), oag_ext_abi::channel_key(name)))
            })
            .collect();
        Self {
            boxes: sending.iter().map(|(b, _)| *b).collect(),
            channels: sending.iter().map(|(_, c)| *c).collect(),
            inside: [0; oag_gameplay::MAX_SHIPS],
        }
    }

    /// The channels `slot` raised this tick by entering a beacon its sweep reaches now and
    /// did not reach before.
    pub(super) fn entered(&mut self, slot: usize, sweep: &[Vec3]) -> Vec<u32> {
        if self.boxes.is_empty() {
            return Vec::new();
        }
        let now = self
            .boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| sweep.iter().any(|p| b.contains(p.to_array())))
            .fold(0_u64, |mask, (i, _)| mask | 1 << i);
        let fresh = now & !self.inside[slot];
        self.inside[slot] = now;
        (0..self.channels.len())
            .filter(|i| fresh & (1 << i) != 0)
            .map(|i| self.channels[i])
            .collect()
    }

    /// Where each slot is among the beacons: what decides the next entry, so state.
    #[cfg(feature = "extensions")]
    pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        for mask in self.inside {
            hasher.write_u64(mask);
        }
    }
}

/// One event in the queue.
#[cfg(feature = "extensions")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Mail {
    pub slot: u8,
    /// The sender's key: `channel_key` of its mod id, or `TRACK_SENDER`.
    pub from: u32,
    pub channel: u32,
    pub value: f32,
}

#[cfg(feature = "extensions")]
impl Mail {
    /// The order every module sees them in, and the order they are hashed in.
    pub(super) fn sort_key(&self) -> (u32, u8, u32, u32) {
        (self.channel, self.slot, self.from, self.value.to_bits())
    }

    pub(super) fn event(self) -> Option<oag_ext_abi::Event> {
        Some(oag_ext_abi::Event::Mod {
            slot: oag_ext_abi::Slot::new(usize::from(self.slot))?,
            from: self.from,
            channel: self.channel,
            value: self.value,
        })
    }
}

/// The queue ready for delivery or hashing: sorted, and held to
/// [`oag_ext_abi::MAX_MOD_EVENTS`] (the later in sorted order are dropped).
#[cfg(feature = "extensions")]
pub(super) fn sorted(mail: &[Mail]) -> Vec<Mail> {
    let mut out = mail.to_vec();
    out.sort_by_key(Mail::sort_key);
    out.truncate(oag_ext_abi::MAX_MOD_EVENTS);
    out
}

#[cfg(feature = "extensions")]
pub(super) fn hash_mail(mail: &[Mail], hasher: &mut oag_core::hash::StateHasher) {
    let mail = sorted(mail);
    hasher.write_u32(mail.len() as u32);
    for m in mail {
        hasher.write_u8(m.slot);
        hasher.write_u32(m.from);
        hasher.write_u32(m.channel);
        hasher.write_f32(m.value);
    }
}

/// What a module's `emit` is checked against: the channels its mod publishes.
#[cfg(feature = "extensions")]
pub(super) fn publishes(declared: &Declared) -> Vec<(u32, String)> {
    declared
        .publishes
        .iter()
        .map(|name| (oag_ext_abi::channel_key(name), name.clone()))
        .collect()
}

/// The most (channel, value) pairs [`Extensions::fired`] keeps.
const FIRED_LIMIT: usize = 256;

impl super::Extensions {
    /// When each channel last fired with each value, as `(channel key, value, tick)`:
    /// mods' emits and triggers' alike. An output for the view's `oag_on` animations;
    /// never hashed.
    #[must_use]
    pub fn fired(&self) -> &[(u32, f32, u64)] {
        &self.fired
    }

    /// The tick `channel` last fired at: with exactly `value` when one is named, with
    /// any value otherwise.
    #[must_use]
    pub fn last_fired(&self, channel: u32, value: Option<f32>) -> Option<u64> {
        self.fired
            .iter()
            .filter(|(c, v, _)| *c == channel && value.is_none_or(|want| *v == want))
            .map(|(_, _, tick)| *tick)
            .max()
    }

    /// Notes that `channel` fired with `value` at `tick`. The log holds at most
    /// [`FIRED_LIMIT`] (channel, value) pairs; a module that emits a new value every tick
    /// pushes out the oldest, which no animation can still be waiting on.
    pub(super) fn note_fired(&mut self, channel: u32, value: f32, tick: u64) {
        log::debug!("channel {channel:#010x} = {value} fired at tick {tick}");
        if let Some(entry) = self
            .fired
            .iter_mut()
            .find(|(c, v, _)| *c == channel && *v == value)
        {
            entry.2 = tick;
        } else if self.fired.len() < FIRED_LIMIT {
            self.fired.push((channel, value, tick));
        } else if let Some(oldest) = self.fired.iter_mut().min_by_key(|(_, _, t)| *t) {
            *oldest = (channel, value, tick);
        }
    }
}
