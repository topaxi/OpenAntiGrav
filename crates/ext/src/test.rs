//! Running an [`Extension`] natively: `cargo test` in the mod's own crate, no wasm
//! toolchain. The observation goes through `oag-ext-abi`'s writer, the host's, and the
//! reply comes back through its decoder, so what a test sees is what the host would.

use std::marker::PhantomData;

use oag_ext_abi::{Command, Event, IO_LEN, MAX_CRAFTS, Slot};

/// A craft record as the host writes it, to put on a test grid.
pub use oag_ext_abi::Craft as CraftRecord;

use crate::{Blob, Extension};

/// A track as a test puts it under a race: each entity's `(prop_key, value)` pairs,
/// every box the identity placement and empty. What the host writes for a track file
/// with those properties.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackRecord {
    /// The track's own properties.
    pub props: Vec<(u32, f32)>,
    /// One list per speed pad.
    pub speed_pads: Vec<Vec<(u32, f32)>>,
    /// One list per weapon pad.
    pub weapon_pads: Vec<Vec<(u32, f32)>>,
    /// One list per trigger.
    pub triggers: Vec<Vec<(u32, f32)>>,
}

impl TrackRecord {
    /// The region the host would write.
    ///
    /// # Panics
    ///
    /// If the track is over a cap of `oag_ext_abi::track`: the host would refuse it.
    #[must_use]
    pub fn region(&self) -> Vec<u8> {
        use oag_ext_abi::track::{Entity, IDENTITY, TRACK_LEN, write};
        fn entities(lists: &[Vec<(u32, f32)>]) -> Vec<Entity<'_>> {
            lists
                .iter()
                .map(|props| Entity {
                    to_world: IDENTITY,
                    min: [0.0; 3],
                    max: [0.0; 3],
                    props,
                })
                .collect()
        }
        let mut region = vec![0; TRACK_LEN];
        let used = write(
            &mut region,
            &self.props,
            &entities(&self.speed_pads),
            &entities(&self.weapon_pads),
            &entities(&self.triggers),
        )
        .unwrap_or_else(|e| panic!("the host would refuse this track: {e}"));
        region.truncate(used);
        region
    }
}

/// A race as one extension sees it: the crafts on the grid, the events queued for the
/// next call, and the state carried between calls.
#[derive(Debug)]
pub struct Race<E: Extension> {
    tick: u64,
    crafts: [Option<CraftRecord>; MAX_CRAFTS],
    events: Vec<Event>,
    state: Vec<u8>,
    track: Vec<u8>,
    extension: PhantomData<E>,
}

impl<E: Extension> Default for Race<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Extension> Race<E> {
    /// An empty grid at tick 0, the state all zeros.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tick: 0,
            crafts: [None; MAX_CRAFTS],
            events: Vec::new(),
            state: vec![0; E::State::BYTES],
            track: Vec::new(),
            extension: PhantomData,
        }
    }

    /// Puts `craft` in `slot`.
    #[must_use]
    pub fn with_craft(mut self, slot: Slot, craft: CraftRecord) -> Self {
        self.crafts[slot.index()] = Some(craft);
        self
    }

    /// Races on `track`.
    #[must_use]
    pub fn with_track(mut self, track: &TrackRecord) -> Self {
        self.track = track.region();
        self
    }

    /// The craft in `slot`, to change between ticks; `None` empties the slot.
    pub fn craft_mut(&mut self, slot: Slot) -> &mut Option<CraftRecord> {
        &mut self.crafts[slot.index()]
    }

    /// Queues `event` for the next call.
    pub fn event(&mut self, event: Event) -> &mut Self {
        self.events.push(event);
        self
    }

    /// The state as the last call left it.
    #[must_use]
    pub fn state(&self) -> E::State {
        E::State::from_bytes(&self.state)
    }

    /// Runs one call: the queued events, the grid, the carried state. Returns the
    /// commands as the host decodes them.
    ///
    /// # Panics
    ///
    /// If the tick's status is not 0 or a command is malformed: the host would fault
    /// the module on either.
    pub fn tick(&mut self) -> Vec<Command> {
        let mut io = Box::new([0_u8; IO_LEN]);
        let active = self
            .crafts
            .iter()
            .enumerate()
            .filter(|(_, craft)| craft.is_some())
            .fold(0_u32, |mask, (slot, _)| mask | 1 << slot);
        let records: Vec<CraftRecord> = self.crafts.iter().map(|c| c.unwrap_or_default()).collect();
        oag_ext_abi::write_observation(
            &mut io,
            self.tick,
            active,
            &self.events,
            &records,
            &self.state,
        );
        let status = crate::run::<E>(&mut io, &self.track);
        assert_eq!(status, 0, "the tick returned {status}");
        let at = oag_ext_abi::STATE_AT;
        let len = self.state.len();
        self.state.copy_from_slice(&io[at..at + len]);
        let count = oag_ext_abi::reply_count(&io).expect("a reply within MAX_COMMANDS");
        let commands = (0..count)
            .map(|i| {
                Command::decode_in(&oag_ext_abi::command_record(&io, i), oag_ext_abi::hud::pool(&io))
                    .unwrap_or_else(|e| panic!("command {i}: {e}"))
            })
            .collect();
        self.events.clear();
        self.tick += 1;
        commands
    }
}
