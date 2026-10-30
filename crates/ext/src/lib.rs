//! The Rust guest SDK for OpenAntiGrav's simulation mods: write an [`Extension`],
//! name it in [`export!`], build for `wasm32-unknown-unknown`.
//!
//! ```ignore
//! #![cfg_attr(not(test), no_std)]
//! use oag_ext::{Extension, Observation, Reply};
//!
//! oag_ext::blob! {
//!     /// One countdown per grid slot.
//!     pub struct Curses { pub countdown: [u16; 8] }
//! }
//!
//! struct WallPenalty;
//!
//! impl Extension for WallPenalty {
//!     type State = Curses;
//!     fn tick(obs: &Observation<'_>, state: &mut Curses, reply: &mut Reply<'_>) {
//!         // ...
//!     }
//! }
//!
//! oag_ext::export!(WallPenalty);
//! ```
//!
//! Everything here is a typed view over ABI 1's I/O buffer, laid out once in
//! `oag-ext-abi` for the host and this crate alike: an [`Observation`] reads it and the
//! [`Track`] the host writes beside it, a
//! [`Reply`] writes commands that name a slot only through a [`Craft`] the observation
//! handed out, and the state is a [`blob!`] struct. The exports (`oag_abi`, `oag_io`,
//! `oag_track`, `oag_state_len`, `tick`) and the panic handler exist only when built for wasm32; the
//! same `tick` runs natively under the `test` feature's [`test::Race`].
#![cfg_attr(not(any(test, feature = "test")), no_std)]

mod blob;
mod reply;
#[cfg(any(test, feature = "test"))]
pub mod test;
mod track;

pub use blob::Blob;
pub use oag_ext_abi::track::{Kind, TRACK_LEN, prop_key};
pub use oag_ext_abi::{
    ABI_WORD, Command, Cue, Effect, Event, IO_LEN, MAX_CRAFTS, MAX_STATE_BYTES, MAX_THRUST_FACTOR,
    Pickup, Rgba, Slot, Weapon, WeatherKind,
};
pub use reply::{Refused, Reply};
pub use track::{Entity, Track};

use oag_ext_abi::Sections;

/// A simulation mod: one call per race tick.
///
/// Two rules the buffer hides:
///
/// - **The instance is fresh every tick.** Nothing in the module's own memory survives
///   from one call to the next; only [`Extension::State`] does, which the host owns,
///   hashes, snapshots and rolls back with the race.
/// - **Events are one tick late.** What happened during tick `n` (a wall contact, a
///   fire press) reaches the call of tick `n + 1`; the commands a call sends act on the
///   tick it is called for.
pub trait Extension {
    /// What survives between ticks: a [`blob!`] struct. Its length is the module's
    /// `oag_state_len` and must equal the manifest's `state_bytes`.
    type State: Blob;

    /// One tick: read `obs`, update `state`, send commands through `reply`.
    fn tick(obs: &Observation<'_>, state: &mut Self::State, reply: &mut Reply<'_>);
}

/// One grid slot that holds a craft this tick. Only an [`Observation`] hands one out,
/// so a command taking a `&Craft` cannot name an empty slot. A view: each accessor
/// reads its field from the buffer, so a craft costs nothing until it is asked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Craft<'a> {
    slot: Slot,
    record: &'a [u8; oag_ext_abi::CRAFT_LEN],
}

impl Craft<'_> {
    fn read(&self) -> oag_ext_abi::Craft {
        oag_ext_abi::Craft::decode(self.record)
    }

    /// Its grid slot.
    #[must_use]
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// A person flies it (else the AI).
    #[must_use]
    pub fn is_person(&self) -> bool {
        self.read().person
    }

    /// A person flying it holds the fire button this tick.
    #[must_use]
    pub fn fire_held(&self) -> bool {
        self.read().fire_held
    }

    /// Racing: past the start, not wrecked, not respawning.
    #[must_use]
    pub fn is_racing(&self) -> bool {
        self.read().racing
    }

    /// The craft's Shield is running: a weapon hit this tick is absorbed.
    #[must_use]
    pub fn is_shielded(&self) -> bool {
        self.read().shielded
    }

    /// The lap being driven, 1-based, saturating at 255.
    #[must_use]
    pub fn lap(&self) -> u8 {
        self.read().lap
    }

    /// The laps the race ends after; 0 when it has no target. The craft is on its final
    /// lap when `laps_target() > 0 && lap() == laps_target()`.
    #[must_use]
    pub fn laps_target(&self) -> u8 {
        self.read().laps_target
    }

    /// The energy pool.
    #[must_use]
    pub fn shield(&self) -> f32 {
        self.read().shield
    }

    /// The pool's maximum: the bound of a [`Reply::shield_delta`].
    #[must_use]
    pub fn shield_max(&self) -> f32 {
        self.read().shield_max
    }

    /// The body's speed, km/h.
    #[must_use]
    pub fn speed_kmh(&self) -> f32 {
        self.read().speed_kmh
    }

    /// What it holds.
    #[must_use]
    pub fn pickup(&self) -> Pickup {
        self.read().pickup
    }
}

/// One tick's observation: the tick, the crafts on the grid, the events since the last
/// call. Nothing is decoded until asked for.
#[derive(Debug)]
pub struct Observation<'a> {
    head: &'a [u8],
    records: &'a [u8],
    active: u32,
    track: &'a [u8],
}

impl<'a> Observation<'a> {
    fn read(head: &'a [u8], records: &'a [u8], track: &'a [u8]) -> Self {
        Self {
            head,
            records,
            active: Sections::active(head),
            track,
        }
    }

    /// The race's track: its pads, its triggers and their properties. The same every
    /// tick of a race.
    #[must_use]
    pub fn track(&self) -> Track<'a> {
        Track::new(self.track)
    }

    /// The world tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        Sections::tick(self.head)
    }

    /// The craft in `slot`, if the slot holds one.
    #[must_use]
    pub fn craft(&self, slot: Slot) -> Option<Craft<'a>> {
        if self.active & slot.bit() == 0 {
            return None;
        }
        let record = Sections::craft_record(self.records, slot)?;
        Some(Craft { slot, record })
    }

    /// Every slot holding a craft, in slot order.
    pub fn crafts(&self) -> impl Iterator<Item = Craft<'a>> + '_ {
        Slot::all().filter_map(|slot| self.craft(slot))
    }

    /// What happened since the last call, in engine order. A kind this SDK does not
    /// know is skipped.
    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        (0..Sections::event_count(self.head))
            .filter_map(|i| Sections::event_record(self.head, i).and_then(Event::decode))
    }
}

/// One call of `E`'s tick over a whole I/O buffer: what the `tick` export runs. Returns
/// the export's status: 0, or 1 when the host's state length is not `E::State`'s
/// (the host faults the module on it).
#[doc(hidden)]
pub fn run<E: Extension>(io: &mut [u8; IO_LEN], track: &[u8]) -> u32 {
    let sections = Sections::split(io);
    let bytes = <E::State as Blob>::BYTES;
    if Sections::state_len(sections.head) != bytes {
        return 1;
    }
    let Some(blob) = sections.state.get_mut(..bytes) else {
        return 1;
    };
    let observation = Observation::read(sections.head, sections.crafts, track);
    let mut state = E::State::from_bytes(blob);
    let mut reply = Reply::new(sections.reply, observation.track().speed_pads());
    E::tick(&observation, &mut state, &mut reply);
    state.to_bytes(blob);
    reply.finish();
    0
}

/// The I/O buffer and the exports' bodies on wasm32.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[allow(unsafe_code)]
pub mod wasm {
    use super::{Extension, IO_LEN, TRACK_LEN};

    static mut IO: [u8; IO_LEN] = [0; IO_LEN];
    static mut TRACK: [u8; TRACK_LEN] = [0; TRACK_LEN];

    /// `oag_io`: the buffer's address in linear memory.
    #[must_use]
    pub fn io_address() -> u32 {
        core::ptr::addr_of!(IO) as u32
    }

    /// `oag_track`: the track region's address in linear memory.
    #[must_use]
    pub fn track_address() -> u32 {
        core::ptr::addr_of!(TRACK) as u32
    }

    /// `tick`.
    pub fn tick<E: Extension>() -> u32 {
        // SAFETY: wasm32 has one thread and these are the only references to `IO` and
        // `TRACK` made during a call; the host writes both only between calls.
        let io = unsafe { &mut *core::ptr::addr_of_mut!(IO) };
        let track = unsafe { &*core::ptr::addr_of!(TRACK) };
        super::run::<E>(io, track)
    }
}

/// Exports `E` as the module's extension: `oag_abi`, `oag_io`, `oag_track`, `oag_state_len`,
/// `tick` and the panic handler (`unreachable`, which the host turns into a fault),
/// all only on wasm32.
#[macro_export]
macro_rules! export {
    ($extension:ty) => {
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn oag_abi() -> u32 {
            $crate::ABI_WORD
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn oag_io() -> u32 {
            $crate::wasm::io_address()
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn oag_track() -> u32 {
            $crate::wasm::track_address()
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn oag_state_len() -> u32 {
            <<$extension as $crate::Extension>::State as $crate::Blob>::BYTES as u32
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn tick() -> u32 {
            $crate::wasm::tick::<$extension>()
        }

        #[cfg(all(target_arch = "wasm32", not(test)))]
        #[panic_handler]
        fn panic(_: &::core::panic::PanicInfo<'_>) -> ! {
            ::core::arch::wasm32::unreachable()
        }
    };
}

#[cfg(test)]
mod tests;
