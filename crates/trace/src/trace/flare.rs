//! The exhaust flare's eight columns and the struct they read into.
//!
//! Split out of `trace.rs` when the sideshift timer columns pushed it past its
//! size ratchet (`scripts/check-file-size.py`): this group is self-contained -
//! its own columns, its own struct, its own one method - and moving it costs
//! nothing a caller can see, since [`super::Trace`] re-exports both names.

/// The eight exhaust-flare columns, written by `scripts/psp-trace.py --flare`:
/// all present or all absent, for the same reason as `CAMERA_COLUMNS`.
///
/// Each is one `f32` read straight out of the flare object the capture walks to
/// as `craft+0x1c4` -> `+0x78`, at the offsets `scripts/psp_trace_fields.py`'s
/// `FLARE_FIELDS` lists. Recording the offset each column came from matters more
/// than usual here, because four of the eight have names that could plausibly
/// belong to a different field:
///
/// | Column | Flare offset | What it is |
/// | --- | --- | --- |
/// | `boost_timer` | `+0xb8` | seconds left on the pad boost; `ExhaustFlare_OnSpeedupPad` stores `0.8` here |
/// | `plume_timer` | `+0x88` | seconds since the `<Team>boost.vex` plume was **revealed** - not the boost timer |
/// | `intensity` | `+0xbc` | the `0..=1` engine ramp the size and layer alphas read |
/// | `half_size` | `+0xc4` | flare quad half-extent, world units, already flickered |
/// | `engine_on` | `+0x94` | see below - an integer field, not a float |
/// | `flare_speed_kmh` | `+0x8c` | craft speed in km/h |
/// | `speed_ramp` | `+0x90` | `((flare_speed_kmh - 100) / 500)` clamped to `0..=1` |
/// | `boost_accum` | `+0x60` | the throttle-charge accumulator |
///
/// **`engine_on` is an integer read through a float lens, and is compared as a
/// predicate rather than by value.** The capture `struct.unpack("<f", ..)`s all
/// eight, but `+0x94` holds a small integer, so the column comes out as a
/// denormal: every non-zero row of `data/traces/pad0-boost.csv` reads
/// `3.601337e-43`, which is the `f32` whose bit pattern is `257`. The two
/// distinct values across that whole capture are exactly `0` and that denormal.
/// Stored here as written rather than normalised - this crate converts nothing
/// on the way in - so anything reading it must ask whether it is zero, never
/// what it equals. See [`Flare::engine_on_is_set`].
pub const FLARE_COLUMNS: [&str; 8] = [
    "boost_timer",
    "plume_timer",
    "intensity",
    "half_size",
    "engine_on",
    "flare_speed_kmh",
    "speed_ramp",
    "boost_accum",
];

/// The exhaust flare's eight fields for one tick.
///
/// One struct rather than eight `Option<f32>` on [`super::Frame`], because the
/// group is all-or-nothing: a capture either ran `--flare` or it did not, and
/// eight independently-optional fields would make "half a flare" representable
/// when it is not a state any file can be in. The angular and camera groups get
/// the same guarantee from `Vec3` being indivisible; this one has to spell it
/// out.
///
/// Field order matches [`FLARE_COLUMNS`], which matches the capture's own header
/// order, so a row written from this reads back into the same struct.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Flare {
    /// Seconds left on the pad boost, `flare+0xb8`.
    pub boost_timer: f32,
    /// Seconds since the plume was revealed, `flare+0x88`. **Not** the boost
    /// timer: the reveal is edge-triggered, so this restarts at a reveal and
    /// then runs its own span regardless of how long the boost lasts.
    pub plume_timer: f32,
    /// The `0..=1` engine intensity ramp, `flare+0xbc`.
    pub intensity: f32,
    /// Flare quad half-extent in world units, `flare+0xc4`, flicker included.
    pub half_size: f32,
    /// Whether the engine counts as lit, `flare+0x94`.
    ///
    /// An integer field the capture reads as a float - see [`FLARE_COLUMNS`].
    /// Read it through [`Self::engine_on_is_set`] rather than comparing it.
    pub engine_on: f32,
    /// Craft speed in km/h, `flare+0x8c`.
    pub speed_kmh: f32,
    /// The clamped speed ramp, `flare+0x90`.
    pub speed_ramp: f32,
    /// The throttle-charge accumulator, `flare+0x60`.
    pub boost_accumulator: f32,
}

impl Flare {
    /// Whether the engine-on field is set, which is the only question its raw
    /// value can answer - see [`FLARE_COLUMNS`] for why it is not a `bool` here
    /// and not comparable as a number.
    #[must_use]
    pub fn engine_on_is_set(&self) -> bool {
        self.engine_on != 0.0
    }
}
