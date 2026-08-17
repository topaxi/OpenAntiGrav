//! Flying a ship on a real track: the composition every layer beneath this was
//! built for.
//!
//! Nothing here is a new subsystem. The track spline, the collision soup, the
//! handling parameters, the force law, the keyboard mapping, the mesh pipeline and
//! the chase camera all already exist and are all already tested; this module is
//! the wiring, and it is deliberately thin so that a surprise in the picture is a
//! surprise in one of those layers rather than in here.
//!
//! ```text
//! Data.wad -> track.vex          -> WO Track spline -> spawn pose, ribbon
//!                                -> collision nodes -> oag_physics::CollisionWorld
//!          -> Ship.vex                              -> the drawn model
//!          -> handlingstats.xml                     -> Handling, ChaseParams
//! ```
//!
//! # What is drawn
//!
//! The track's **art meshes** - the map, textured, as a player sees it. 144,351
//! triangles on `16_Track`.
//!
//! `--ribbon` swaps in the **driveable ribbon** instead, a flat band over the
//! spline, and that is a **development view** rather than a style: it is the
//! geometry the simulation actually spawns on and queries, so a picture of
//! ship-plus-ribbon shows directly whether the ship is where the physics thinks it
//! is, where the art meshes are prettier and prove less.
//!
//! **The default used to be the other way round**, and that was right while M4's
//! whole job was verifying a force law against a captured trace. It stopped being
//! right once there was a game to look at: a player launching `just play` was
//! shown a debug view of a spline.
//!
//! # Three things are not recovered, and none is papered over here
//!
//! - ~~**There is no inertia tensor in the ship data**~~, and there is not - but there
//!   is one in the *code*, and it has been read. [`box_inertia`] is now
//!   [`oag_physics::forces::ship_inertia`], the code-literal box `(12, 8, 12)` at a
//!   mass of `0.9` that `Body_SetBoxInertia`'s single call site passes, giving
//!   `(15.6, 21.6, 15.6)` for every craft in the game. The two earlier positions this
//!   module held - "a derived tensor is an invented constant, worse than a visible
//!   oddity", then "build one from `<Misc>`" - are both superseded by a reading, and
//!   the `<Misc>` derivation was `4.4x` out on roll. See [`box_inertia`].
//! - **The magnetic hold is not implemented**, so an inverted ship falls off. That
//!   is expected and documented in `docs/physics/README.md`.
//! - **The camera's `fov` unit is measured as degrees** - vertical, at the
//!   authored 480x272 aspect - by rendering a captured pose beside the
//!   original's own frame and sweeping: the RMSE minimum lands on the authored
//!   value, bounded to about two degrees. One ship, one view, so the load
//!   report still prints the value read; see
//!   `docs/ghidra/functions/psp-pulse-usa/camera.md` and
//!   `docs/tools/frame-compare.md` for the measurement.
//!
//! Nothing in this module tunes, scales or corrects a physics value. The four
//! pre-scaled handling fields are scaled exactly once, inside
//! [`oag_gameplay::handling_for`], and grid-slot assignment - which
//! `oag_gameplay::spawn` records as an open question - is not guessed at: one ship
//! starts on the racing line at the beginning of the first path.
//!
//! Two conventions *are* reconciled here, both at the boundary and both scored:
//! [`camera::chase_pos_length`], because the disc's camera offset is signed the
//! other way from the renderer's, and [`MODEL_YAW`], because a `.vex` ship is
//! authored nose along `+Z` while the simulation's forward is `-Z`. Each is one
//! expression with its evidence written next to it.
//!
//! # Where things are
//!
//! This file is the root: the constants, [`Telemetry`], the [`Race`] struct
//! itself, and the re-exports that keep every path a caller already uses. The
//! work is in the modules beside it, and each one is a move out of this file
//! rather than a rewrite - the split landed on 2026-08-16 under the 1,000-line
//! rule in `scripts/check-file-size.py`, with `just test` reporting the same
//! 1,896 passed / 223 skipped either side of it.
//!
//! Loading a race: `options` (what is asked for and what comes back), `load`
//! (the read itself), `assets`, `hud`, `spawn`.
//!
//! Running one: `start`, `tick`, `field` (the racing line, the standings and
//! what one craft knows of another), `pads`, `weapons`, `respawn`, `results`
//! (the finish condition and the table it leaves), `effects`, `hash`,
//! `telemetry`, `access`, `spline`, `camera`.
//!
//! Drawing one: `scene` and its `frame` child, `drawable`, `visibility`,
//! `models`, `capture` (the headless path), `held_buttons` (the input a capture
//! drives).

use std::path::PathBuf;

use crate::language::roles;
use crate::livery::{self, Livery};
use anyhow::{Context, Result};
use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Quat, Vec3};
use oag_core::{Rng, TickClock, TickRate};
use oag_formats::track::{AiTrack, Sample, StartPosition};
use oag_formats::vex;
use oag_formats::{collision, handling};
use oag_gameplay::{
    ControlScheme, GRID_SLOTS, InputSnapshot, MAX_SHIPS, Pose, Ship, World, collision_world,
    handling_for, ship_controls, to_format_class,
};
use oag_physics::{CollisionWorld, Environment, Evaluated, Handling, SpeedClass};
use oag_pulse::race::ships;
use oag_race::{Course, Mode, RaceState};
use oag_render::camera::chase::{Chase, ChaseParams, Target};
use oag_render::camera::internal::InternalParams;
use oag_render::collision as render_collision;
use oag_render::exhaust::{self, Exhaust, FlareTexture};
use oag_render::mesh::{DrawCall, Model};
use oag_render::mesh_render::Anisotropy;
use oag_render::psys;
use oag_render::pvs::{
    DrawSections, PlacementStats, SectionPadding, SwapConflicts, UNPLACED, VisibleSet,
};
use oag_render::sparks;
use oag_render::{mesh, mesh_render, track as track_render};

mod access;
mod assets;
mod camera;
mod capture;
mod drawable;
mod effects;
mod field;
mod hash;
mod held_buttons;
mod hud;
mod load;
mod models;
mod options;
mod pads;
mod respawn;
mod results;
mod scene;
mod spawn;
mod spline;
mod start;
mod telemetry;
mod tick;
mod visibility;
mod weapons;

pub use assets::{boost_entry_name, ship_entry_name};
pub use camera::chase_params;
pub use capture::{CaptureOptions, Presented, capture, describe};
pub use held_buttons::HeldButtons;
pub use hud::hud_layout;
pub use load::load;
pub use options::{CameraOverride, Loaded, Options, PoseRequest, Setup};
pub use scene::Scene;
pub use spline::Spline;
pub use visibility::{SceneStats, TrackVisibility};

pub(crate) use assets::ps2_texture_set;
use assets::{
    FLARE_TEXTURE, NOISE_TEXTURE, mip_texture, particle_effect, unrecovered_or_absent,
    untextured_note,
};

use camera::target_of;
#[cfg(test)]
pub(crate) use drawable::Uniforms;
use drawable::{Drawable, model_matrix_of};
use effects::exhaust_seed;
#[cfg(test)]
pub(crate) use held_buttons::key_for_button;
use hud::load_hud;
use spawn::{grid_poses, spawn_pose, start_position_of};
use spline::{ai_order, racing_line};
use visibility::{CAMERA_SEARCH_SAMPLES, OFF_TRACK_HALF_WIDTHS, visible};

/// The track a race is flown on unless another is named.
///
/// **`16_Track`, and this is a measurement rather than a preference.** The reference
/// scenario every PPSSPP capture uses is Talon's Junction White (see
/// `docs/reverse-engineering/ppsspp-debugger.md`), and the directory that holds it was
/// assumed to be `01_Track` until the recording was checked against the geometry
/// instead of against the name:
///
/// - `Data\Plugins\PI001\Definition.xml` lists `<PI_Track name="16_Track">` **first**,
///   at `soundregister="1"`, with `location="Data\Environments\16_Track"`.
///   `01_Track` is `soundregister="18"`.
/// - Decisively, the capture's own 200 recorded positions were cast against every
///   track on the disc. `16_Track` is the only one that finds geometry under **200 of
///   200** of them, and the mean height it finds is **4.002** against the
///   `4.125 - 0.147 = 3.978` this crate's own spring predicts for a resting ship.
///   Every other track and every sign convention comes out at a few dozen units off
///   or misses outright.
///
/// So a trace comparison run against `01_Track` seeded the original's Talon's Junction
/// position into a different track's collision soup, which is why `grounded` read `0.0`
/// from the first tick while the recording read `1.0` on all 200. That was read as a
/// force-law failure and it was a track-selection one.
///
/// Note the reciprocal fact recorded with it: the same 200 positions sit a fairly
/// constant **~21 units** from `16_Track`'s AI racing line, so `spline_distance` and
/// "on the driveable surface" are not the same measurement for this recording.
pub use oag_pulse::race::DEFAULT_TRACK;

/// The default team: [`oag_pulse::race::DEFAULT_TEAM`], and the reason it is
/// Assegai rather than Feisar is on the constant itself.
pub use oag_pulse::race::DEFAULT_TEAM;

/// Turns the ship model's own facing into the body's.
///
/// **A `.vex` ship is authored nose along `+Z`**, and
/// [`oag_physics::Body::forward`] is `-Z`, so a model drawn straight from the body's
/// orientation faces backwards: the chase camera sits behind the body, which is the
/// model's *nose* side, and draws the ship head-on. A half turn about the model's up
/// axis reconciles them.
///
/// Measured rather than assumed, on the default team's `Ship.vex`. Sliced along `z`,
/// the hull is 5.3 units across at the `-Z` end with the full height of the model and
/// 1,175 of its 1,334 vertices, and tapers to 0.9 across at the `+Z` end. The wide,
/// detailed, full-height end of a racing ship is its engine block and the narrow
/// tapering end is its nose; there is no reading of those numbers in which `+Z` is
/// the tail.
///
/// Confidence **85**. The measurement admits one reading and is reproducible from the
/// asset, but it is an inference from the hull's shape: nothing in the executable has
/// been read that states the convention, and only one team's model was measured.
/// Applied here, at the boundary between a gameplay body and a drawn model, and
/// deliberately **not** in `oag_render::mesh` - `oag-view --mesh` shows a model in its
/// own space and must keep doing so.
pub const MODEL_YAW: f32 = std::f32::consts::PI;

/// The viewport shape every camera value on the disc was authored for.
///
/// The PSP renders into 480x272 and nothing else, so an authored field of view is
/// only defined at this aspect ratio. [`Race::projection`] is where that matters.
/// Taken from [`crate::frontend::SCREEN`] rather than written again, because it is
/// the same screen: the front end lays its widgets out in it and the camera was
/// framed for it.
pub const AUTHORED_ASPECT: f32 = crate::frontend::SCREEN.0 / crate::frontend::SCREEN.1;

/// Degrees of extra field of view per unit/s of **forward** speed.
///
/// A recovered constant, not a taste decision, and it is the whole of a
/// behaviour this project was missing until 2026-08-09: the original's field
/// widens as the craft goes faster. It is a code literal at `0x08a7b6a0`, read
/// at the top of `FUN_088455ec` as
/// `craft[0x790] = dot(fwd, vel) * 0.075 + craft[0x7c]`, and `craft+0x790` is
/// added to **both** tripod fovs every frame in `Ship_UpdateCameraRigs`' tail.
///
/// Verified on the running game rather than only read: against
/// `g_camera_fov_degrees` (`0x08b34310`), `fov = 60 + 0.075 * dot(fwd, vel)`
/// reproduces to a worst residual of **0.0007 degrees over 19 consecutive
/// samples** spanning 122-146 units/s - float32 precision. `craft+0x7c` is
/// exactly zero on a clean run. Confidence 94; see
/// `docs/rendering/projection-vs-the-original.md`.
///
/// Three things about the shape of it, each of which was a wrong guess first:
///
/// - **Additive degrees, not a tangent multiplier.** It does not compose the way
///   [`display::BoostFovKick`](crate::display::BoostFovKick) does.
/// - **Driven by `dot(fwd, vel)`, not by speed.** The two are the same number
///   whenever the craft goes where it points, and a live sample at *negative*
///   forward velocity drove the fov to `54.26` - **below** the authored 60,
///   which no speed magnitude can do. That is what settles it.
/// - **Nothing to do with boosting.** It is present with no pad involved;
///   `BoostFovKick` remains this project's own invention and is composed
///   separately.
const SPEED_FOV_GAIN_DEG: f32 = 0.075;

/// Guard rails on the composed field of view, in degrees.
///
/// **Not recovered - the original clamps nothing here**, and at any speed the
/// game can actually reach the term lands far inside these. They exist because
/// a fov at or past `0` or `180` degrees makes `tan(fov/2)` degenerate and the
/// projection matrix `NaN`, and a renderer should not produce a broken frame
/// for a craft that has been teleported or handed an absurd velocity by a test.
const FOV_GUARD_DEG: (f32, f32) = (1.0, 179.0);

/// The world's generator seed.
///
/// Fixed and arbitrary: nothing in the recovered force law draws from the
/// generator, so this exists only so the seed is written down in one place for
/// whenever something does. See `crates/core/src/rng.rs`.
pub const SEED: u64 = 1;

/// Seed for the player's exhaust flicker, kept distinct from [`SEED`].
///
/// The exhaust draws two numbers per tick and the simulation must not see them:
/// sharing a generator would let the picture change the physics, which is what
/// `docs/architecture/determinism.md` forbids. A separate seed also means the two
/// streams cannot be mistaken for each other when reading a capture.
///
/// **Slot 0 keeps exactly this value**, unshifted, so every capture and every
/// pinned exhaust number taken before the field had flares of its own still
/// reproduces - see [`exhaust_seed`].
pub const EXHAUST_SEED: u64 = 0xe8_a5_71_00;

/// The exhaust's per-frame budgets have to cover the whole grid.
///
/// `oag_render::exhaust` sizes its two shared vertex buffers for
/// `exhaust::MAX_TRAILS` craft and cannot import [`MAX_SHIPS`] itself - rule 1 of
/// `docs/architecture/workspace-layout.md` runs the other way, but a render crate
/// reaching into the simulation for a constant is the kind of dependency that
/// only ever grows. This is the one place both numbers are visible, so this is
/// where they are compared.
///
/// A **compile-time** assertion, because the failure is the silent kind:
/// `exhaust::Pipeline::upload` clamps with `min`, so an undersized buffer drops
/// the last craft's ribbon with nothing in the logs.
const _: () = assert!(oag_render::exhaust::MAX_TRAILS >= MAX_SHIPS);

/// Seed for spark spawn parameters, kept distinct from [`SEED`] and
/// [`EXHAUST_SEED`] for the same determinism reason.
pub const SPARKS_SEED: u64 = 0x5_9a_2b_00;

/// Seed for the [`psys::Stage`]'s spawn parameters - the rocket flares and
/// the detonations. Distinct from [`SPARKS_SEED`] for the same reason that
/// one is distinct from [`SEED`].
pub const STAGE_SEED: u64 = 0x5_9a_2b_01;

/// The archive entry a Rocket's model comes from.
///
/// **Recovered, confidence 85.** The string `Rocket_Ctor` (`0x0885cc24`) hands
/// its scene node, at `0x08a7c0e8` - see
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. Backslashes, the way
/// the loader assembles paths, which is what the name hash needs.
pub const ROCKET_MODEL_ENTRY: &str = r"Data\Weapons\Rocket.vex";

/// Half-width of the sprite a projectile in flight is drawn as, in world units.
///
/// **Invented**, and now only a *fallback*: a rocket is drawn as
/// [`ROCKET_MODEL_ENTRY`]'s model when that model loaded, and as this billboard
/// only when it did not. Small enough to read as a bolt rather than a fireball
/// at the distance a rocket is fired from.
pub const PROJECTILE_SPRITE_HALF_SIZE: f32 = 1.5;

/// The effect the original attaches to every rocket at launch.
///
/// **Recovered, confidence 72.** `Rocket_Init` (`0x0885cdb8`) spawns it through
/// `Psys_Spawn_q` with the tag `ROFL`; the string is at `0x08a7c100`. Two
/// emitters, both [`oag_formats::pob::flags::LOOPING`], so it runs for as long
/// as the rocket does rather than for its authored 100 ticks - see
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
///
/// **There is no separate trail effect.** The Shuriken authors a `_HEAD` and a
/// `_TRAIL`; the rocket has only this, so the one emitter draws both the glow
/// at the nose and the streak behind it.
pub const ROCKET_FLARE_EFFECT: &str = "WO_ROCKET_FLARE";

/// The explosion a rocket that hits **track geometry** plays.
///
/// **Recovered, confidence 72.** Both of `Rocket_Update`'s (`0x0885d2a8`)
/// detonating branches spawn it, tag `ROD2`, string `0x08a7c110`. Four
/// emitters: a root glow, `fat_streaks`, a `SMOKERING` and `Fire_Emitter`.
pub const TRACK_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO_TRACK";

/// The explosion a rocket that hits a **craft** plays.
///
/// **Recovered, confidence 72.** `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`),
/// reached only from the craft-hit path `Rocket_HitCraft_q` (`0x0886ebdc`), tag
/// `ROEX`, string `0x08a7ca74`. Seven emitters, the largest tree on the disc:
/// a root that hangs a `SMOKEMUSHROOM` off every particle, 32 pieces of
/// `DEBRIS` a tick, a `SMOKERING`, a `GLOW`, and a second per-particle pair of
/// `FIREMUSHROOM` glows.
///
/// That the two explosions are separately authored is confirmed rather than
/// inferred from the names, which is why this engine plays two files rather
/// than one file at two sizes.
pub const CRAFT_BLAST_EFFECT: &str = "WO_ROCKET_EXPLO";

/// How far below a struck craft's centre its blast is drawn, in world units.
///
/// **Recovered, confidence 78.** `Rocket_HitCraft_q` (`0x0886ebdc`) builds the
/// explosion's position from the struck craft's own position with `y - 2.5`, not
/// from the rocket's impact point.
pub const CRAFT_BLAST_DROP: f32 = 2.5;

/// The engine flare the **PS2** port authors as a particle effect.
///
/// Two looping emitters, and there is no PSP counterpart - the PSP release
/// authors no `Data\Psys` engine flare at all, which is why
/// [`oag_render::exhaust`] draws one procedurally from the `Engine Flare`
/// locator and the behaviour measured off the PSP. Where the source *does*
/// ship one, playing it beats approximating it, so a PS2-sourced race gets
/// the asset and the procedural flare quad steps aside - see
/// [`Race::engine_flare_effect`].
pub const ENGINE_FLARE_EFFECT: &str = "WO_SHIP_ENGINEFLARE";

/// Every `Data\Psys` effect this race loads, and what triggers it.
///
/// **The list is the trigger set, not the asset set.** There are 35 effects on
/// the PSP disc and 41 on the PS2 one; what decides whether one appears here is
/// whether the *executable's* reason for playing it has been recovered, because
/// an effect with no recovered trigger would just be this engine guessing when
/// to fire it. `crates/game/tests/psys_inventory_ground_truth.rs` holds every
/// effect on both discs against this list and fails if one is neither played
/// nor explicitly recorded as having no recovered trigger.
///
/// **It is a superset across sources, not a per-disc list.** An entry absent
/// from the mounted archives is reported by the loader and skipped, so naming
/// a PS2-only effect here costs a PSP race one report line and nothing else.
pub const RACE_EFFECTS: [&str; 5] = [
    sparks::DAMAGE_EFFECT,
    ROCKET_FLARE_EFFECT,
    TRACK_BLAST_EFFECT,
    CRAFT_BLAST_EFFECT,
    ENGINE_FLARE_EFFECT,
];

/// How much faster a craft assumes a Turbo will make it, when deciding whether
/// it can afford to fire one.
///
/// **Ours, and a rule of thumb rather than a measurement.** Nothing derives the
/// speed a boost settles at - it depends on the class, the craft's own thrust
/// and where the boost is spent - and the number only has to be roughly right,
/// because it is used to ask whether the *corner* ahead is a corner. Observed
/// on `16_Track`: 126 into a Turbo comes out at about 270. Erring high is the
/// safe direction, since it makes a craft keep the pickup rather than spend it
/// into a wall. See [`Race::spend_opponent_pickup`].
const TURBO_SPEED_RATIO: f32 = 2.2;

/// How much clear road a craft wants in front before it spends a Turbo. Ours.
///
/// A boost more than doubles the speed, so a craft that fires one at a rival
/// two hull lengths ahead arrives in its gearbox.
const TURBO_CLEARANCE: f32 = 60.0;

/// How close along the track two craft have to be to count as alongside rather
/// than ahead or behind. Ours - about two hull lengths.
const ALONGSIDE_GAP: f32 = 16.0;

/// And how close across. Ours - about three hull widths, so a craft on the far
/// side of a wide corridor is not "alongside" in any sense worth acting on.
const ALONGSIDE_WIDTH: f32 = 12.0;

/// Placing a ship on a track lives in [`oag_gameplay::spawn`], which is where a
/// harness that is not the composition root can reach it: `oag-trace drive` puts
/// a ship on a start line exactly the way a race does and may not depend on this
/// crate. Re-exported so every call site here, and both ground-truth tests, read
/// as they did.
pub use oag_gameplay::spawn::{box_inertia, spawn_height};

/// One tick's worth of what the simulation did, for a log or an overlay.
///
/// Everything here is read out of the state *after* a step; nothing in it is an
/// input to the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// Where the ship is.
    pub position: Vec3,
    /// How fast it is going, in world units per second.
    pub speed: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`.
    pub grounded: f32,
    /// Distance to the nearest spline sample.
    pub spline_distance: f32,
    /// Height above that sample along its own up axis. Negative is below the
    /// surface line.
    pub height_above_spline: f32,
    /// What the craft is carrying, if anything.
    ///
    /// Here so `--race --screenshot`'s own log answers "did the pad grant
    /// anything" without a debugger. It is the question every weapon capture
    /// starts with, and a HUD icon in a screenshot is a poor way to ask it.
    pub pickup: Option<oag_formats::weapons::Weapon>,
    /// How many projectiles are in the air.
    pub projectiles: usize,
}

/// A race in progress: the world, the track it is on, and the camera behind it.
#[derive(Debug)]
pub struct Race {
    /// The simulation state.
    pub world: World,
    collision: CollisionWorld,
    spline: Spline,
    /// The authored racing line, as the opponents' drivers want it.
    ///
    /// Built once from [`Self::spline`] and **index-parallel to [`Self::ai_order`]**,
    /// so a driver's place on the line is also its place in that table and no
    /// craft pays for two searches. Track data rather than world state, which is
    /// why it is here and the drivers themselves are on the ships. See
    /// [`racing_line`] and `docs/gameplay/ai.md`.
    racing_line: oag_ai::Line,
    /// Which spline sample each racing-line index is, in lap order.
    ///
    /// The identity permutation on nine of the disc's twelve circuits. See
    /// [`ai_order`], and [`Self::ai_sample`] for the lookup itself.
    ai_order: Vec<u32>,
    /// What the opponents' drivers are flown with. One set for the whole field:
    /// per-craft variation is the skill work, and this is the basic driver.
    ai_tuning: oag_ai::Tuning,
    /// Which pilot each grid slot is flying.
    ///
    /// Not world state: it is read-only for the whole race and drawn from the
    /// race seed, so it reproduces without being carried. Slot 0 is the
    /// player's and is never read.
    ai_pilots: [oag_ai::Pilot; oag_gameplay::MAX_SHIPS],
    /// The lap counter's ring, or `None` on a track whose chain does not close.
    course: Option<Course>,
    /// Zone mode's three numbers, off the disc. `None` outside Zone mode, and on
    /// a source whose `handlingstats.xml` carries no `<Global><Zone/>`.
    zone: Option<oag_formats::handling::Zone>,
    /// The external block the chase camera is currently flying, which is
    /// whichever of [`Self::chase_far`] / [`Self::chase_close`]
    /// [`Self::camera_view`] names. Kept as its own field rather than looked up
    /// per use, so every consumer reads one thing and cannot pick the wrong
    /// block.
    chase_params: ChaseParams,
    /// `<ExternalCameraFar>`, kept so a view change can swap it back in.
    chase_far: ChaseParams,
    /// `<ExternalCameraClose>`.
    chase_close: ChaseParams,
    /// `<InternalCamera>`: the cockpit view, which has no spring and so no state
    /// of its own beside these five numbers.
    internal_params: InternalParams,
    /// Which of the three perspectives is live. **Render-only state**, on `Race`
    /// rather than in `World` for the same reason [`Self::exhaust`] is - and here
    /// it is load-bearing rather than tidy: cycling the view mid-race must not
    /// move a determinism hash or a replay by a single bit. See
    /// [`crate::display::CameraView`] and
    /// `tests::cycling_the_camera_changes_no_simulation_state`.
    view: crate::display::CameraView,
    camera: Chase,
    /// Render from this pose instead of [`Self::camera`]. See [`CameraOverride`].
    camera_override: Option<CameraOverride>,
    dt: f32,
    /// Ticks left before a `Reset` contact can respawn each craft again.
    ///
    /// See [`RESPAWN_COOLDOWN_TICKS`].
    ///
    /// **Per slot, and all four of these are, which is not tidiness.** One
    /// shared set of counters would let an opponent stuck in a corner exhaust
    /// [`RESPAWN_GIVE_UP`] and switch off the *player's* recovery, and a
    /// cooldown armed by one craft would strand another that fell off in the
    /// same half second. Indexed by ship slot, like [`Self::exhaust`].
    respawn_cooldown: [u32; oag_gameplay::MAX_SHIPS],
    /// How many respawns each craft has had back to back, for
    /// [`RESPAWN_GIVE_UP`].
    respawns_in_a_row: [u32; oag_gameplay::MAX_SHIPS],
    /// Set once respawning has given up on a craft, so the complaint is printed
    /// once.
    respawn_disabled: [bool; oag_gameplay::MAX_SHIPS],
    /// How many times each craft has been respawned this race, for tests and
    /// for the load report.
    respawns: [u32; oag_gameplay::MAX_SHIPS],
    /// How many consecutive ticks each opponent has spent away from the sample
    /// its driver believes it is on. See [`RESCUE_TICKS`].
    ///
    /// Slot 0's entry is never written: the player is recovered by the authored
    /// reset volumes and by their own hands, and teleporting a craft somebody is
    /// flying is a much bigger decision than teleporting one nobody can see.
    lost_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// How many consecutive ticks each opponent has spent stopped while asking to
    /// move. See [`STALL_TICKS`].
    ///
    /// A **second** dwell rather than a widening of [`Self::lost_ticks`], because
    /// the two measure different failures and share only their response: one
    /// craft has left the circuit, the other is still on it and going nowhere.
    /// Slot 0's entry is never written, for the reason [`Self::lost_ticks`] gives.
    stalled_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// [`RESCUE_HALF_WIDTHS`] in track units, resolved once against this
    /// circuit's widest half-width rather than folded over the sample table
    /// every tick.
    rescue_distance: f32,
    /// Each craft's exhaust animation state, advanced on the simulation tick.
    ///
    /// **One per racer, slot 0 the player's**, because the original runs
    /// `Exhaust_Update` per craft: a flare, a ribbon and a boost plume belong to
    /// the craft that is burning, not to the race. Indexed by ship slot, so
    /// `exhaust[n]` and `world.ships[n]` are the same craft; entries past
    /// `world.ship_count` are simply never advanced.
    ///
    /// Here rather than in `World` for the same reason [`Chase`] is: it is
    /// render-only state, so it must not enter a snapshot a replay or a
    /// determinism hash reads. `physics/src/probe.rs` destructures `ShipState`
    /// exhaustively on purpose, and adding a visual field there would move the
    /// pinned hashes for no reason.
    exhaust: [Exhaust; MAX_SHIPS],
    /// The flicker's generators, deliberately **not** `world.rng`.
    ///
    /// The exhaust draws two random numbers per tick. Taking them from the
    /// simulation's generator would make the picture change what the simulation
    /// does next - the determinism rules exist to prevent exactly that. Seeded, so
    /// a capture at tick *n* is still reproducible.
    ///
    /// **One stream per craft** rather than one shared stream drawn from eight
    /// times a tick, for two reasons: a shared stream makes slot 0's flicker
    /// depend on how many opponents the mode fields, which would break every
    /// capture taken against a single-craft race, and it couples eight flares
    /// that should be independent. See [`exhaust_seed`].
    exhaust_rng: [Rng; MAX_SHIPS],
    /// The `engine_flare` locator in model space, when the ship model has one.
    ///
    /// One value for the whole field, because every craft wears the player's hull
    /// today - see `Scene::new`. It becomes one per craft when per-team models
    /// land, at which point the locator moves with the model rather than with the
    /// race.
    nozzles: Vec<Option<Vec3>>,
    /// Collision sparks' particle pool, advanced on the simulation tick.
    ///
    /// Here rather than in `World`, for the same reason [`Self::exhaust`] is -
    /// see `oag_render::psys`'s module doc comment.
    sparks: psys::System,
    /// Every `.pob` this race loaded - see [`Setup::effects`].
    effects: psys::Library,
    /// The multi-instance pool everything *except* the hull-mounted sparks
    /// plays in: the rockets' flares and their detonations today, and
    /// whatever gets a recovered trigger next.
    ///
    /// [`Self::sparks`] stays its own [`psys::System`] because it is one
    /// permanently hull-mounted emitter re-ignited on a cooldown rather than
    /// an effect that comes and goes - see [`Self::sparks_anchor`].
    stage: psys::Stage,
    /// The [`ENGINE_FLARE_EFFECT`] instance riding each craft's nozzle, on a
    /// source that authors one. All `None` on a PSP-sourced race.
    engine_flare: [Option<psys::Playing>; MAX_SHIPS],
    /// The flare instance riding each live projectile, indexed by its
    /// [`oag_gameplay::projectile`] slot.
    ///
    /// The slot **is** the identity: it is fixed for a projectile's whole
    /// life and reused the moment the projectile is gone, which is exactly
    /// when the flare should be a fresh one.
    projectile_flare: [Option<psys::Playing>; oag_gameplay::projectile::MAX_PROJECTILES],
    /// The stage's generator, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    stage_rng: Rng,
    /// How many bursts the *trigger* has fired, whether or not an effect
    /// was loaded to play them.
    ///
    /// Counted here rather than read off the pool because the two answer
    /// different questions: the pool knows how many times it was ignited,
    /// this knows how many times the contact rule said to - and only the
    /// second is [`Race`]'s to get right.
    sparks_ignitions: u32,
    /// The sparks' generator, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    sparks_rng: Rng,
    /// Seconds remaining before another spark burst is allowed.
    ///
    /// Counts down every tick regardless of contact, and gates a burst
    /// alongside `oag_physics::wall::WallResponse::impact` - see
    /// `oag_render::sparks::COLLISION_COOLDOWN`'s doc comment for why this is
    /// a cooldown timer and not a one-shot edge latch: `impact` stays `true`
    /// for every tick of a sustained scrape, and spawning a burst on every
    /// one of those ticks is the per-frame-instead-of-per-impact bug
    /// `oag_physics::wall::STUN_PER_CONTACT`'s doc comment already records
    /// costing a session of play-testing, but the recovered original itself
    /// re-fires periodically rather than going silent for the rest of the
    /// scrape.
    sparks_cooldown: f32,
    /// The `Ship Collision Fx` locators in model space - see
    /// [`Setup::collision_fx`].
    collision_fx: Vec<Vec3>,
    /// The per-class grounded-gravity scale - see [`Setup::class_gravity_scale`].
    class_gravity_scale: f32,
    /// The track's speed-pad trigger volumes - see [`Setup::speedup_pads`].
    speedup_pads: Vec<oag_formats::pads::PadVolume>,
    /// Distance from the ship to each pad, one entry per pad, in track units.
    ///
    /// The original's `pad+0x1d0`, reimplemented as a broadphase rather than as
    /// the latch `docs/formats/track.md` used to guess it was: `Pad_SweptTest`
    /// (`0x0888686c`) subtracts how far the craft moved from each entry every
    /// tick and only runs the real containment test on entries that reach zero,
    /// then stores the freshly measured distance back. A ship 900 units from a pad
    /// moving 2 units a tick is skipped for 450 ticks for the cost of one
    /// subtraction.
    ///
    /// **One row per racer**, as the original keeps: a broadphase cache is a
    /// statement about where *a* craft is, and eight craft sharing one row would
    /// skip pads for each other.
    pad_distance: [Vec<f32>; MAX_SHIPS],
    /// Which pad the ship was inside last tick, the original's `craft+0x1d0`.
    ///
    /// `None` outside every pad. Only a *change* of value counts as entering a new
    /// pad, which is what gates the Zone score - standing still on one pad does
    /// not pay repeatedly. One per racer.
    pad_current: [Option<usize>; MAX_SHIPS],
    /// The track's weapon-pad trigger volumes - see [`Setup::weapon_pads`].
    ///
    /// **Empty unless the mode arms them.** A weapons-off race in the original
    /// does not skip the trigger, it zeroes the list's own count
    /// (`World_CollectNodeLists`), and emptying this reproduces that at the same
    /// layer rather than adding a mode test to every tick.
    weapon_pads: Vec<oag_formats::pads::PadVolume>,
    /// Distance from the ship to each weapon pad. The speed pads'
    /// [`Self::pad_distance`], one class over, and the same broadphase.
    weapon_pad_distance: [Vec<f32>; MAX_SHIPS],
    /// Which weapon pad the ship was inside last tick.
    ///
    /// The edge this changes on is what grants a pickup, exactly as
    /// [`Self::pad_current`]'s edge is what pays the Zone score. Two pads
    /// overlapping on one tick is one entry. One per racer.
    weapon_pad_current: [Option<usize>; MAX_SHIPS],
    /// Seconds each weapon pad has left before it can be triggered again.
    ///
    /// The original's `pad+0x1a0`, stamped by `WeaponPads_TestCraft` with
    /// `<WeaponPad refresh_time>` and counted back down by
    /// `WeaponPad_UpdateRefreshTimer` (`0x0892c034`) - see
    /// [`oag_formats::handling::WeaponPad`]. Per pad rather than per craft,
    /// which is what makes it a property of the track rather than of the racer.
    ///
    /// **Deliberately outside the determinism hash**, unlike
    /// `ShipState::turbo_timer`. It is genuine simulation state and a replay of
    /// a single race would need it; it is not hashed because the hash covers
    /// `ShipState` and the tick, and widening that is a change to the gate
    /// rather than a change to this feature. Recorded here so the gap is a known
    /// one - see `docs/gameplay/pickups.md`.
    weapon_pad_refresh_left: Vec<f32>,
    /// How long a stamped weapon pad stays inert - see
    /// [`Setup::weapon_pad_refresh`].
    weapon_pad_refresh: f32,
    /// This race's weapon table - see [`Setup::weapons`].
    weapons: Option<oag_formats::weapons::WeaponStats>,
    /// The speed class, which indexes the pickup odds - see [`Setup::class`].
    class: SpeedClass,
    /// How far the boost's field-of-view kick has opened, `0.0` to `1.0`.
    ///
    /// Render-only state, on `Race` rather than in `World` for the same reason
    /// [`Self::exhaust`] is. See [`crate::display::BoostFovKick`], which is
    /// where the "authored, not recovered" argument for the whole effect
    /// lives.
    boost_kick: f32,
    /// How strong the kick is, `0` off. `[graphics] boost_fov_kick`.
    boost_fov_kick: crate::display::BoostFovKick,
    /// How far each airbrake flap has swung, left then right, on `0..=100`.
    ///
    /// The airbrake's own scale rather than an angle, because that is the
    /// scale `up_speed`/`down_speed` ramp on - see the tick that advances it.
    /// [`Self::airbrake_flaps`] is what turns it into radians.
    ///
    /// **Render-only state**, on `Race` rather than in `World` for the same
    /// reason [`Self::boost_kick`] is - and here the separation is the
    /// original's own, not this crate's convenience. `<AirbrakeGraphics>`
    /// carries `up_speed` and `down_speed` *beside* `<Airbrake>`'s `gain` and
    /// `falloff`, which is the game saying outright that the flap the pilot
    /// sees and the airbrake the force law applies ramp at different rates.
    /// Putting these two floats in `ShipState` would move the determinism
    /// hashes every time somebody adjusted an animation.
    flaps: [f32; 2],
    /// The authored deflection and the two rates, from `<AirbrakeGraphics>`.
    ///
    /// `amount` is already radians here: `oag_gameplay::airbrake_graphics_for`
    /// applies the loader's own degrees-to-radians scale, recovered at
    /// confidence 92 - see `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    flap_graphics: oag_gameplay::AirbrakeGraphics,
    /// Which control scheme maps the snapshot. `[controls] scheme`.
    ///
    /// On `Race` and not on the input layer because the schemes differ in which
    /// *gesture* a sideshift takes, and the gesture is read by the simulation
    /// out of `ShipControls` - so this is what decides which of
    /// `ship_controls`' two field groups gets filled. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    scheme: ControlScheme,
    /// Where the ship was at the end of last tick, for the swept test.
    ///
    /// `None` on the first tick, which is the original's own "no previous
    /// position" case in `Pads_TestCraft` and takes the single-point path. One
    /// per racer.
    pad_previous_position: [Option<Vec3>; MAX_SHIPS],
    /// The burst's emitter anchor, **model space**: the `Ship Collision Fx`
    /// locator nearest the last impact, or the contact point itself mapped
    /// into model space when the model authors no locators. Transformed
    /// through the ship's current matrix every tick, so the burst rides the
    /// hull - rotation included - the way the original's scene-graph node
    /// does.
    sparks_anchor: Option<Vec3>,
    /// Whether slot 0 is being driven by its own [`oag_ai::Driver`] instead of
    /// by the input snapshot. See [`Self::set_autopilot`].
    autopilot: bool,
    /// The results table, taken on the tick the race reached its finish
    /// condition, and `None` before that.
    ///
    /// **A snapshot rather than a live query**, and that is the whole point: "the
    /// race is over" has to mean one fixed table. The field is still moving when
    /// the player crosses - `oag_race::places` ranks whoever has not finished by
    /// distance covered, which is the honest answer at that instant - and a board
    /// recomputed a second later would show a different one.
    ///
    /// On `Race` rather than in `World`: it is derived from state the world
    /// already holds, so hashing it would hash the same facts twice, and a
    /// replay reproduces it by reaching the same tick. See [`Self::results`].
    results: Option<crate::scoreboard::Board>,
}

/// How fast the kick opens, per second, as an exponential approach.
///
/// Faster than [`BOOST_FOV_CLOSE_RATE`] on purpose: the boost should arrive as a
/// shove and let go slowly. Authored, like [`crate::display::BoostFovKick`].
pub const BOOST_FOV_OPEN_RATE: f32 = 9.0;

/// How fast the kick closes again, per second. See [`BOOST_FOV_OPEN_RATE`].
pub const BOOST_FOV_CLOSE_RATE: f32 = 3.5;

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

#[cfg(test)]
mod tests;
