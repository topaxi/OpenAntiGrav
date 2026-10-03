//! The tunables that decide when a craft is lost and what happens next: the
//! respawn cooldown, the two rescue envelopes, and the stall.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. They sit
//! together because they are one mechanism read from three angles - `off the
//! track`, `not moving`, and `put it back` - and `oag_game`'s
//! `race/respawn.rs` and `race/field.rs` are what consume all eight.
//!
//! Moved out of `oag-game` into this crate on 2026-09-09: eight `f32`/`u32`
//! constants with no renderer, audio or input contact at all, so they belong
//! with the race rules rather than above the composition root.
//!
//! Each carries its own evidence, as it did in `race.rs`. What is authored and
//! what is this project's is stated per constant; nothing here was renamed or
//! renumbered by the move.

/// Ticks a `Reset` contact is ignored for after a respawn.
///
/// Half a second at 60 Hz. The recovery pose puts the ship on the racing line at
/// hover height, so it should be clear of any trigger immediately - this exists
/// because "should" is an assumption about shipped data, not a property of the
/// code, and the failure it guards is a race that freezes in a respawn loop with
/// no symptom.
pub const RESPAWN_COOLDOWN_TICKS: u32 = 30;

/// Consecutive respawns after which respawning stops and says so.
///
/// A ship that needs five recoveries without ever getting clear is not being
/// recovered, and a visible complaint beats an invisible freeze.
pub const RESPAWN_GIVE_UP: u32 = 5;

/// How far an opponent may drift from the sample its own driver believes it is
/// on before it counts as lost, in multiples of the track's widest half-width.
///
/// # Invented, and the reason it is not the reset volumes
///
/// A `Reset` contact is authored geometry and it is what recovers a craft that
/// falls through the floor. It does **not** recover a craft that leaves the
/// circuit sideways into open space: measured on the disc's twelve circuits, a
/// lone opponent that came off receded from the track at racing speed for the
/// rest of the race - eight thousand units in fifty seconds - and touched no
/// reset volume at any point, because there is none out there to touch. Seven
/// of the twelve never completed a lap for that reason alone.
///
/// So this is a second, invented trigger, and it is deliberately keyed on the
/// *driver's* index rather than on a global search of the sample table: it
/// costs one distance instead of four thousand per craft per tick, and it
/// measures the thing that actually went wrong, which is that the craft and the
/// driver's idea of where it is have come apart. See
/// [ADR-0006](../../docs/architecture/adr/0006-no-copyrighted-content.md) - it
/// is ours, not the original's, and nothing in the RE tree describes what the
/// original does here.
///
/// Eight half-widths is wide enough that a leap, a barrel roll off a crest or a
/// shove into a wall does not trip it, and the failure it catches overshoots it
/// by two orders of magnitude within seconds.
pub const RESCUE_HALF_WIDTHS: f32 = 8.0;

/// How long an opponent has to stay that far away before it is put back.
///
/// A second and a half at 60 Hz. **The dwell matters more than the distance**:
/// airborne over a gap is briefly indistinguishable from gone, and the two are
/// told apart by whether the craft comes back.
pub const RESCUE_TICKS: u32 = 90;

/// How far the *player* may get from the nearest spline sample before they count
/// as off the track, in multiples of the circuit's widest half-width.
///
/// # Not [`RESCUE_HALF_WIDTHS`], and the numbers differ because the measurements do
///
/// That constant measures an opponent against the sample its own driver *believes*
/// it is on, which inflates from along-track drift as well as from leaving, so it
/// has to be loose. Nobody steers the player's craft, so this measures the true
/// distance to the nearest sample - a much tighter quantity, and a much smaller
/// multiple.
///
/// **Measured before it was chosen**, one autopiloted craft alone on ten of the
/// disc's circuits at ace, 6,000 ticks each, recording the peak distance of every
/// excursion the craft *came back* from:
///
/// | circuit | widest half-width | peak returned from |
/// | --- | --- | --- |
/// | 07 | 49.1 | 33.7 (**0.69x**) |
/// | 01 | 33.2 | 24.7 (0.74x) |
/// | 16 | 57.0 | 26.0 (0.46x) |
/// | 06 | 70.3 | 24.3 (0.35x) |
/// | 14 | 73.3 | 22.8 (0.31x) |
/// | 04 | 61.6 | 20.9 (0.34x) |
/// | 03, 09, 13 | 79.0, 51.8, 70.0 | never left at all |
///
/// **No healthy craft reached three quarters of one half-width**, jumps included:
/// a circuit's racing line runs *through* its authored jump, so a craft in the air
/// over one is near the spline rather than far from it - 13, the circuit with the
/// jump, peaks at 13.7 units. Two half-widths is therefore between 2.7x and 5.8x
/// the worst healthy excursion, on every circuit measured.
///
/// The failure this exists for is in the same measurement: on `05_Track` the craft
/// passed 20 units at tick 591 and never came back, reaching **7,983 units** with
/// nothing to recover it. It took 91 ticks to go from one half-width out to four,
/// so a craft crossing this threshold is already committed to leaving rather than
/// passing through it.
///
/// **And the authored `Reset` volumes cannot be the answer**, which was assumed
/// rather than checked until this landed: `06_Track`, `14_Track` and `16_Track` -
/// including the default circuit, and including the one the failure was reported
/// on - author **no `Reset` geometry at all**, in either direction. See
/// `crates/game/tests/off_track_rescue_ground_truth.rs`.
///
/// It also sits just inside the one number in this crate that already means "too
/// far off the spline to be trusted": `visibility::OFF_TRACK_HALF_WIDTHS`, three
/// half-widths, past which the culling stops believing the craft's own section id.
/// A craft this recovers was already somewhere the partition was not drawn around.
///
/// Ours, not the original's, exactly as [`RESCUE_HALF_WIDTHS`] is.
pub const PLAYER_RESCUE_HALF_WIDTHS: f32 = 2.0;

/// How long the player has to stay that far out before they are put back.
///
/// Three quarters of a second at 60 Hz, and **half [`RESCUE_TICKS`] because the
/// measurement behind it is direct**: the opponents' dwell absorbs the noise in a
/// believed index, and there is no believed index here. What it still buys is the
/// tick or two either side of a hard landing where a craft is momentarily far from
/// the sample table and about to be near it again.
///
/// It is also the whole latency budget the player feels: a craft that leaves is
/// falling, and every tick of dwell is a tick further down before the recovery.
pub const PLAYER_RESCUE_TICKS: u32 = 45;

/// How slowly a craft has to be moving to count as stopped, in units per second.
///
/// # Invented, like [`RESCUE_HALF_WIDTHS`], and measured before it was chosen
///
/// [`RESCUE_HALF_WIDTHS`] catches a craft that has *left* the circuit. It cannot
/// catch one that is still on it and going nowhere: a craft beached against the
/// scenery is a few units from the line its driver is steering along, which is
/// exactly where a craft that is driving well also is.
///
/// **One unit per second separates the two cleanly, and the separation is not a
/// margin - it is total.** Measured over the disc's twelve circuits at all four
/// difficulties, a lone opponent driving with the throttle down, counting the
/// longest unbroken run below each of several speeds:
///
/// | cell | longest run under 1 | longest run under 5 |
/// | --- | --- | --- |
/// | novice `05_Track` | **1,215** | 3,634 |
/// | novice `07_Track` | **266** | 698 |
/// | skilled `07_Track` | **0** | 267 |
/// | every other cell | **0** | 3-11 |
///
/// The two beachings are the only cells that spend *any* consecutive time below
/// one unit per second. The five-unit column is what makes the choice: it would
/// also catch skilled `07_Track`, which is a craft crawling through a slow
/// section and recovering by itself in four and a half seconds, and rescuing that
/// one would cost a clean lap on a circuit that currently manages one. The
/// three-to-eleven-tick runs in the last row are the standing start.
///
/// The other half of the gate is that the craft is **asking** to move -
/// `ShipState::thrust` above zero - which is what tells a beached craft from one
/// held on the grid before the lights or coasting after it has finished. Measured
/// on the same runs: the throttle reads a full 100 on every tick of both real
/// beachings, so the gate holds continuously through the thing it has to catch.
pub const STALL_SPEED: f32 = 1.0;

/// How long a craft has to stay stopped before it is put back.
///
/// Two seconds at 60 Hz, and the room either side of it is wide: no healthy craft
/// in the measurement above spends a *single* consecutive tick below
/// [`STALL_SPEED`], and the shorter of the two real beachings lasts 266. So this
/// is not a fitted threshold - it is two seconds because two seconds is long
/// enough that a player watching would already call the craft stuck, and there is
/// no evidence pulling it either way.
pub const STALL_TICKS: u32 = 120;

/// Seconds a craft may go without any hover probe touching before it is reset.
///
/// **The original's, read and measured (2026-09-29).** `FUN_088418e0`, the
/// per-craft race update, calls `Ship_SetState(craft, 3)` - the same reset state
/// a `Reset` contact enters - when `*(craft+0x94)+0x284 > 4.0` and the craft is
/// not wrecked (`craft+0x860 & 0x1000` clear), at `0x08841d30`. Nothing gates
/// it on the player/AI flag, so it applies to every craft. `+0x284` is the
/// airborne clock `Ship_UpdateCraft` keeps (`0x08849e08` zeroes it on a tick any
/// probe touched, `0x08849e28` adds `dt` otherwise), which this project already
/// carries as `oag_physics::ShipState::time_airborne`.
///
/// Measured in PPSSPP on `01_Track` (the Basilico entry titled Black, reached with the dev-unlock
/// byte): a craft coasted off the lip at samples 31-42 at 20 u/s landed
/// upside down on the floor below, its probes pointing at the sky; the clock ran
/// from the tick it left the deck, read 3.99 on the tick before, and the craft
/// was relocated upright on the tick it passed 4.0. See
/// `docs/gameplay/leaving-the-track.md`. Confidence **90**.
pub const AIRBORNE_RESET_SECONDS: f32 = 4.0;
