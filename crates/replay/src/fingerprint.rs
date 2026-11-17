//! [`Replay::fingerprint`]: one number for the run a recording is of, equal on every machine
//! that recorded the same run.
//!
//! An online race's replay is the server-confirmed run, kept on each client. Two clients of one
//! race (and the server's journal) hold different files: a header with this machine's team, build
//! and finish place, a ghost, and a pose track of the build that recorded it. What they share is
//! the race's parameters, every seat's input stream, the changes made between ticks and the state
//! hash at every interval. The fingerprint covers exactly that, so equal fingerprints mean the
//! local file is the real run, and a different one means this client missed a stretch or
//! desynchronised.
//!
//! Hashed through [`oag_core::StateHasher`] over the stored words, never the file's bytes: the
//! compression and the chunk layout are the format's to change.

use oag_core::StateHasher;

use crate::Replay;
use crate::codec::words;

impl Replay {
    /// The fingerprint of the run (see the module doc), over the first `ticks` ticks of it, or
    /// all of them when the recording is shorter. Two clients that stopped confirming at
    /// different ticks compare over the shorter run.
    #[must_use]
    pub fn fingerprint_through(&self, ticks: u64) -> u64 {
        let ticks = ticks.min(self.ticks());
        let header = &self.header;
        let mut hasher = StateHasher::new();
        for text in [&header.title, &header.track, &header.mode, &header.class] {
            hasher.write_u32(u32::try_from(text.len()).unwrap_or(u32::MAX));
            hasher.write(text.as_bytes());
        }
        hasher.write_u64(header.seed);
        hasher.write_u32(header.tick_rate);
        hasher.write_u32(header.hash_interval);
        for slot in &header.slots {
            hasher.write_u8(*slot);
        }
        for (key, value) in &header.options {
            hasher.write(key.as_bytes());
            hasher.write_u8(0);
            hasher.write(value.as_bytes());
            hasher.write_u8(0);
        }
        for module in &header.mods {
            hasher.write(module.id.as_bytes());
            hasher.write(module.sha256.as_bytes());
            hasher.write_u32(module.abi);
            hasher.write_u32(module.revision);
        }
        hasher.write_u64(self.initial_hash);
        hasher.write_u64(ticks);
        let interval = u64::from(header.hash_interval.max(1));
        let hashes = usize::try_from(ticks / interval).unwrap_or(usize::MAX);
        for hash in self.hashes.iter().take(hashes) {
            hasher.write_u64(*hash);
        }
        let keep = usize::try_from(ticks).unwrap_or(usize::MAX);
        for stream in &self.inputs {
            for snapshot in stream.iter().take(keep) {
                for word in words(snapshot) {
                    hasher.write_u32(word);
                }
            }
        }
        for event in self.events.iter().filter(|event| event.tick < ticks) {
            hasher.write_u64(event.tick);
            hasher.write_u8(event.seat);
            hasher.write_u8(event.kind);
            hasher.write_u32(event.value);
        }
        hasher.finish()
    }

    /// [`Self::fingerprint_through`] over the whole recording.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint_through(self.ticks())
    }
}

#[cfg(test)]
mod tests {
    use oag_gameplay::InputSnapshot;

    use crate::{Header, Recorder};

    fn run(ticks: u64, nudge_at: Option<u64>) -> crate::Replay {
        let mut header = Header::new("pulse", "track", "single_race", "venom", "team", 7);
        header.slots = vec![0, 1];
        let mut recorder = Recorder::new(header, 99);
        for tick in 0..ticks {
            let mut inputs = oag_gameplay::PlayerInputs::none();
            let stick_x = if nudge_at == Some(tick) { 0.5 } else { 0.25 };
            inputs.set(
                0,
                InputSnapshot {
                    stick_x,
                    ..InputSnapshot::default()
                },
            );
            inputs.set(1, InputSnapshot::default());
            recorder.record(&inputs, || tick);
        }
        recorder.replay(None)
    }

    /// Per-machine fields are outside it; one changed input, a different run or a different
    /// stretch is inside it.
    #[test]
    fn the_fingerprint_ignores_this_machine_and_sees_one_changed_input() {
        let base = run(400, None);
        let mut mine = base.clone();
        mine.header.team = "someone else's team".into();
        mine.header.build = "another build".into();
        mine.header.ticks = 12;
        mine.header.finish = Some(crate::header::Finish { place: 3, ticks: 9 });
        assert_eq!(mine.fingerprint(), base.fingerprint());
        assert_ne!(run(400, Some(123)).fingerprint(), base.fingerprint());
        let mut other_seed = base.clone();
        other_seed.header.seed += 1;
        assert_ne!(other_seed.fingerprint(), base.fingerprint());
        assert_ne!(run(399, None).fingerprint(), base.fingerprint());
        assert_eq!(
            run(399, None).fingerprint(),
            base.fingerprint_through(399),
            "a client that confirmed fewer ticks compares over the common prefix"
        );
    }
}
