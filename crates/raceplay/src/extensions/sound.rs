//! `play-mod-sound` on the simulation side: it queues `ModSoundEvent`s beside the
//! cues for the playing side to drain, and remembers which loops a module started so a
//! fault can stop them. Plain data only; nothing here knows a mixer exists, and nothing
//! is hashed.

use oag_ext_abi::SoundMode;
use oag_gameplay::cue::{ModSoundEvent, ModSoundMode};

use crate::sim::RaceSim;

/// The loops a module started and has not stopped: module, sound, slot.
pub(super) type Loops = Vec<(usize, u16, u8)>;

/// Events kept for a drain that has not come.
const MAX_QUEUED: usize = 1024;

fn event(sim: &RaceSim, module: usize, index: u16, slot: u8, mode: ModSoundMode) -> ModSoundEvent {
    ModSoundEvent {
        mod_id: sim.extensions.on.slots[module].mod_id.clone(),
        index,
        slot,
        mode,
    }
}

/// Applies one `play-mod-sound` of module `module`.
pub(super) fn apply(sim: &mut RaceSim, module: usize, slot: u8, index: u16, mode: SoundMode) {
    let mode = match mode {
        SoundMode::Once => ModSoundMode::Once,
        SoundMode::Loop => ModSoundMode::Loop,
        SoundMode::Stop => ModSoundMode::Stop,
    };
    let key = (module, index, slot);
    let loops = &mut sim.extensions.on.sound_loops;
    match mode {
        ModSoundMode::Once => {}
        ModSoundMode::Loop if loops.contains(&key) => return,
        ModSoundMode::Loop => loops.push(key),
        ModSoundMode::Stop => loops.retain(|held| *held != key),
    }
    // A host that never drains (a server) keeps a bounded queue; loop bookkeeping above
    // is unaffected, only a one-shot is dropped.
    if sim.mod_sounds.len() >= MAX_QUEUED && mode == ModSoundMode::Once {
        return;
    }
    let queued = event(sim, module, index, slot, mode);
    sim.mod_sounds.push(queued);
}

/// A module is off for the race: its loops stop.
pub(super) fn release(sim: &mut RaceSim, module: usize) {
    let (mine, rest): (Loops, Loops) = std::mem::take(&mut sim.extensions.on.sound_loops)
        .into_iter()
        .partition(|(by, ..)| *by == module);
    sim.extensions.on.sound_loops = rest;
    for (_, index, slot) in mine {
        let stop = event(sim, module, index, slot, ModSoundMode::Stop);
        sim.mod_sounds.push(stop);
    }
}
