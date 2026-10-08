//! Flying a ship on a real track: the composition every layer beneath this was
//! built for.
//!
//! # Where this crate stops
//!
//! This is `oag-game`'s old `race/` module, extracted whole. It is renderer-coupled
//! on purpose (it builds the scene, the effects and the HUD readout from the same
//! tick that steps the simulation), so it sits above `oag-render`, `oag-fx`,
//! `oag-mesh`, `oag-sound`, `oag-hud`, `oag-livery` and `oag-present`, and below
//! `oag-game`. It is not a gameplay crate: `scripts/check-dependency-rules.py`
//! classifies it with the other drawing crates, and the pure simulation it drives
//! stays in `oag-gameplay`, `oag-physics`, `oag-ai` and `oag-race`.
//!
//! What came with it, because the load reads them and nothing else in the front end
//! owns them: [`catalogue`] (the circuits and teams a title offers), [`pilots`] (the
//! player-authored AI roster), [`loader_log`] (the load report's log levels), the
//! [`scoreboard`] table a finished race builds and the [`track_panel`] assets the
//! flyby shows. What it asks of the host it takes as plain data or does without:
//! opening a source is `oag-source`, the language tables are `oag_ui::language::load`,
//! and the loading screen's own `Progress` is built by the host from
//! [`LoadProgress`]. What stayed in `oag-game`: the headless capture
//! (`oag_game::race_capture`, which composites the front end's scoreboard, HUD,
//! countdown and track-panel overlays over [`Scene`]) and the overlays' own wgpu
//! passes.
//!
//! # What this module is
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

use anyhow::{Context, Result};
use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Quat, Vec3};
use oag_core::{Rng, TickClock, TickRate};
use oag_fx::exhaust::{self, Exhaust, FlareTexture};
use oag_fx::psys;
use oag_fx::sparks;
use oag_gameplay::{
    ControlScheme, GRID_SLOTS, InputSnapshot, MAX_SHIPS, Pose, Ship, World, collision_world,
    handling_for, ship_controls,
};
use oag_livery::Livery;
use oag_livery::entry::ps2_texture_set;
use oag_mesh::mesh;
use oag_mesh::mesh::{DrawCall, Model};
use oag_mesh::mesh_render;
use oag_mesh::mesh_render::Anisotropy;
use oag_physics::{CollisionWorld, Environment, Evaluated, Handling, SpeedClass};
use oag_present::perf::SceneStats;
use oag_race::recovery::{
    BENEATH_LINE, PLAYER_RESCUE_HALF_WIDTHS, PLAYER_RESCUE_TICKS, RESCUE_HALF_WIDTHS, RESCUE_TICKS,
    RESPAWN_COOLDOWN_TICKS, RESPAWN_GIVE_UP, STALL_SPEED, STALL_TICKS,
};
use oag_race::sight;
use oag_race::{Course, Mode, RaceState};
use oag_render::camera::chase::{Chase, ChaseParams, Target};
use oag_render::camera::internal::InternalParams;
use oag_render::collision as render_collision;
use oag_render::pvs::{
    ChunkSet, DrawSections, PlacementStats, SectionPadding, SwapConflicts, UNPLACED, VisibleSet,
};
use oag_render::{shield::ShipShield, track as track_render};
use oag_tables::handling;
use oag_vex::track::{AiTrack, Sample, StartPosition};
use oag_vex::vex;

mod absorb;
mod access;
pub mod adverts;
mod assets;
mod blast_models;
mod bomb_blast;
mod camera;
pub mod catalogue;
pub mod countdown;
mod craft_flash;
mod damage_fx;
mod destroy_camera;
mod drawable;
mod effects;
mod eliminator;
pub mod engine_light;
mod field;
pub mod finish_camera;
mod finished_thrust;
pub mod gantry;
mod handles;
mod hash;
mod held_buttons;
mod hit_sparks;
mod hud;
pub mod intro_camera;
mod load;
pub mod loader_log;
mod mag_floor_fx;
mod magstrip_wake;
mod missile_blast;
mod models;
mod options;
mod pads;
mod perfect_start;
pub mod pilots;
mod reconcile;
mod replay;
mod repulser_field;
mod respawn;
mod results;
mod routes;
mod scene;
pub mod scenery_fx;
pub mod scoreboard;
mod shadow;
mod sim;
mod spawn;
mod speed_plan;
mod spline;
mod stages;
mod start;
mod takeoff_line;
mod telemetry;
mod texture_sink;
mod tick;
pub mod tournament;
pub mod track_panel;
mod view;
mod visibility;
mod weapons;
mod wreck_fx;
pub(crate) use weapons::{CannonAssets, CannonDraw};
mod worker;
pub mod zone_grade;

pub use absorb::absorb_burst_for;
pub use blast_models::PlasmaBlastModels;
pub use camera::chase_params;
pub use field::{FireLaw, PadSeeking};
pub use handles::EffectHandles;
pub use held_buttons::HeldButtons;
pub use hud::hud_layout;
pub use load::ripple::{Ripples, SpanPlaces};
pub use load::{load, load_event};
pub(crate) use oag_title::Trigger;
pub use options::{CameraOverride, Campaign2048Progress, Loaded, Options, PoseRequest, Setup};
pub use replay::{Ghost, GhostCapture};
pub use respawn::RespawnCause;
pub use results::RunStats;
pub use scene::Scene;
pub use scene::behind_glass::BehindGlassModels;
pub use sim::RaceSim;
pub use spline::{Spline, circuit_length};
pub use telemetry::{Telemetry, describe};
pub use texture_sink::TextureSink;
pub use view::RaceView;
pub use visibility::TrackVisibility;
pub use worker::{LoadProgress, LoadWorker};

// `pub`, not `pub(crate)`: the selection screens' previews are built by the
// composition root's own `picker_stage`, which is the `[[bin]]` over this
// `[lib]` and draws a PS2 circuit's outline and craft through the same
// texture-set rule a race does.
// The two exhaust names are re-exported so a ground-truth test can assert the
// report line each one produces without spelling the literal a second time -
// the names are the executable's, and one copy of them is the point.
pub use assets::{FLARE_TEXTURE, NOISE_TEXTURE};
use assets::{particle_effect, unrecovered_or_absent, untextured_note};

use camera::target_of;
#[cfg(test)]
pub(crate) use drawable::Uniforms;
use drawable::draw::Lists;
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
/// deliberately **not** in `oag_mesh::mesh` - `oag-view --mesh` shows a model in its
/// own space and must keep doing so.
pub const MODEL_YAW: f32 = std::f32::consts::PI;

/// The viewport shape every camera value on the disc was authored for.
///
/// The PSP renders into 480x272 and nothing else, so an authored field of view is
/// only defined at this aspect ratio. [`Race::projection`] is where that matters.
/// Taken from [`oag_display::space::SCREEN`] rather than written again, because it is
/// the same screen: the front end lays its widgets out in it and the camera was
/// framed for it.
pub const AUTHORED_ASPECT: f32 = oag_display::space::SCREEN.0 / oag_display::space::SCREEN.1;

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
///   [`display::BoostFovKick`](oag_display::display::BoostFovKick) does.
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
/// `oag_fx::exhaust` sizes its two shared vertex buffers for
/// `exhaust::MAX_TRAILS` craft and cannot import [`MAX_SHIPS`] itself - rule 1 of
/// `docs/architecture/workspace-layout.md` runs the other way, but a render crate
/// reaching into the simulation for a constant is the kind of dependency that
/// only ever grows. This is the one place both numbers are visible, so this is
/// where they are compared.
///
/// A **compile-time** assertion, because the failure is the silent kind:
/// `exhaust::Pipeline::upload` clamps with `min`, so an undersized buffer drops
/// the last craft's ribbon with nothing in the logs.
const _: () = assert!(oag_fx::exhaust::MAX_TRAILS >= MAX_SHIPS);

/// Same reason as [`EXHAUST_SEED`]'s assertion above, for the Cannon's own
/// two quads: `oag_fx::weapon_quads` sizes its buffers for
/// [`oag_fx::weapon_quads::MAX_ROUNDS`] and cannot import
/// `oag_weapons::projectile::MAX_PROJECTILES` itself.
const _: () = assert!(oag_fx::weapon_quads::MAX_ROUNDS >= oag_weapons::projectile::MAX_PROJECTILES);

/// Seed for spark spawn parameters, kept distinct from [`SEED`] and
/// [`EXHAUST_SEED`] for the same determinism reason.
pub const SPARKS_SEED: u64 = 0x5_9a_2b_00;

/// Seed for the [`psys::Stage`]'s spawn parameters - the rocket flares and
/// the detonations. Distinct from [`SPARKS_SEED`] for the same reason that
/// one is distinct from [`SEED`].
pub const STAGE_SEED: u64 = 0x5_9a_2b_01;

/// Seed for the camera shake's phase draw. Distinct from the others for the
/// same determinism reason.
pub const SHAKE_SEED: u64 = 0x5_9a_2b_02;

/// Seed for the LeachBeam ribbon's own amplitude draws - see
/// `RaceView::leach_beam_rng`. Distinct from the others for the same
/// determinism reason.
pub const LEACH_BEAM_SEED: u64 = 0x5_9a_2b_03;

/// The archive entry a Rocket's model comes from.
///
/// **Recovered, confidence 85.** The string `Rocket_Ctor` (`0x0885cc24`) hands
/// its scene node, at `0x08a7c0e8` - see
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. Backslashes, the way
/// the loader assembles paths, which is what the name hash needs.
pub const ROCKET_MODEL_ENTRY: &str = r"Data\Weapons\Rocket.vex";

/// The archive entry a Mine's model comes from.
///
/// **Recovered, confidence 92**, off `mine.md`'s reading of `Mine_Construct`
/// (`0x08859930`), which loads the entity's model by exactly this name.
///
/// **Drawn with no drop-time effect alongside it, and that is a checked
/// negative rather than an omission.** The Next Step that wired this asked
/// for `Mine_Init` (`0x08859ac8`) and `Mine_Construct` to be read for one
/// before any was; both were, live, in the same session this constant was
/// added. `Mine_Init`'s call list is two sound cues (`MINELAUNCH`,
/// `MINERADAR`, both already in `mine.md`) plus an allocator and an init for
/// the second cue's tracked handle - no call anywhere in it resolves to
/// `Psys_Spawn_q` (`0x08915484`). So a mine is laid silently on the visual
/// side, the same as it always was; only the sound and the body model are new.
/// The live-database caveat this carries: this session's Ghidra instance
/// resolves `Mine_Init`'s and `Mine_Construct`'s own function boundaries to
/// different addresses than `mine.md` records, most likely because
/// `just apply-names` has not been replayed into it - the *content* at both
/// documented addresses still matches `mine.md`'s reading byte for byte,
/// which is what this finding rests on rather than the addresses lining up).
pub const MINE_MODEL_ENTRY: &str = r"Data\Weapons\Pulse_Mine.vex";

/// The archive entry a Bomb's model comes from.
///
/// **Recovered, confidence 92**, off `mine.md`'s "The Bomb is the same
/// weapon, one size up" section: the Bomb's own model-loading group mirrors
/// the Mine's exactly, at `Data\Weapons\Pulse_Bomb.vex`. Same no-drop-effect
/// finding as [`MINE_MODEL_ENTRY`] - the two share `Mine_Init`'s code path
/// per that section, so what is true of one's drop is true of the other's.
pub const BOMB_MODEL_ENTRY: &str = r"Data\Weapons\Pulse_Bomb.vex";

/// The archive entry a Cannon round's model comes from.
///
/// **Recovered, confidence 85.** `Cannon_Construct` (`0x088651d8`) calls
/// `Vex_LoadModel` with exactly this string (`0x08a7c85c`) for every one of
/// the sixty round instances `CannonPool_Construct` (`0x088573b0`)
/// allocates, and hangs the node it gets at `instance+0xc0` - see
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "What
/// draws a Cannon round". The entry resolves in `Data.wad` on both PSP
/// pressings (name hash `b94a2a6c`, entry 1048, 6,608 bytes).
///
/// **The name is the file's, not a description**, and it is worth saying
/// once: the mesh is called *muzzleflash* while the round it is loaded for
/// is the bolt. Each round instance also builds two GU display lists of
/// hand-written quads in its constructor, textured from
/// `Data\\Weapons\\Textures\\Cannon_bolt.mip` and
/// `Cannon_muzzle_flash.mip` (`Cannon_LoadTextures`, `0x08864b00`) - see
/// [`CANNON_BOLT_TEXTURE_ENTRY`]/[`CANNON_MUZZLE_FLASH_TEXTURE_ENTRY`],
/// which list is which, and both quads' own geometry, now recovered and
/// drawn; see `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
/// the 2026-09-17 section.
pub const CANNON_MODEL_ENTRY: &str = r"Data\Weapons\pulse_muzzleflash.vex";

/// The archive entry the Cannon round's bolt streak textures from.
///
/// **Recovered, confidence 88.** `Cannon_DrawRound` (`0x0886545c`) binds
/// `g_cannon_bolt_texture` immediately before `Cannon_BuildBoltList`'s
/// `Gu_CallList` every frame a round is alive - see
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "Which
/// display list is the bolt and which the flash". Resolves in `Data.wad` on
/// both PSP pressings (name hash `ee06a033`, entry 1057, 2,064 bytes); the
/// directory layout is one level deeper than the model's own -
/// `Data\Weapons\Textures\`, not `Data\Weapons\`.
pub const CANNON_BOLT_TEXTURE_ENTRY: &str = r"Data\Weapons\Textures\Cannon_bolt.mip";

/// The archive entry the Cannon round's muzzle flash textures from.
///
/// **Recovered, confidence 88.** `Cannon_DrawRound` binds
/// `g_cannon_muzzle_flash_texture` before `Cannon_BuildMuzzleFlashList`'s
/// `Gu_CallList`, gated on the round's own age (`+0xc8 < 0.1` seconds) - see
/// [`CANNON_BOLT_TEXTURE_ENTRY`]. Resolves in `Data.wad` on both PSP
/// pressings (name hash `1762ad77`, entry 1058, 5,136 bytes), immediately
/// after the bolt's own entry.
pub const CANNON_MUZZLE_FLASH_TEXTURE_ENTRY: &str =
    r"Data\Weapons\Textures\Cannon_muzzle_flash.mip";

/// The three archive entries the Plasma's own detonation loads: a halo and
/// two hemispheres, "the expanding shell of the blast".
///
/// **Recovered, confidence 88.** `PlasmaBlast_Construct` (`0x0885fd90`) reads
/// all three strings directly out of `.rodata` - see
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md#the-detonation-and-the-three-models-under-it`.
/// The load order is `PLASMA_BLAST_HALO_MODEL_ENTRY`, then
/// `PLASMA_BLAST_HEMISPHERE2_MODEL_ENTRY`, then
/// `PLASMA_BLAST_HEMISPHERE1_MODEL_ENTRY` (hemisphere2 before hemisphere1,
/// which is the file's own order, not a typo) - `weapons::blast_models`
/// keeps that order because it is also the order `PlasmaBlast_Update`'s own
/// per-model anim-time rate table pairs against.
pub const PLASMA_BLAST_HALO_MODEL_ENTRY: &str = r"Data\Weapons\pulse_plasma_halo1.vex";
/// See [`PLASMA_BLAST_HALO_MODEL_ENTRY`].
pub const PLASMA_BLAST_HEMISPHERE1_MODEL_ENTRY: &str = r"Data\Weapons\pulse_plasma_hemisphere1.vex";
/// See [`PLASMA_BLAST_HALO_MODEL_ENTRY`].
pub const PLASMA_BLAST_HEMISPHERE2_MODEL_ENTRY: &str = r"Data\Weapons\pulse_plasma_hemisphere2.vex";

/// Half-width of the sprite a projectile in flight is drawn as, in world units.
///
/// **Invented**, and now only a *fallback*: a rocket is drawn as
/// [`ROCKET_MODEL_ENTRY`]'s model when that model loaded, and as this billboard
/// only when it did not. Small enough to read as a bolt rather than a fireball
/// at the distance a rocket is fired from.
pub const PROJECTILE_SPRITE_HALF_SIZE: f32 = 1.5;

/// Every `Data\Psys` effect this race loads, and what triggers it.
///
/// Defined beside the triggers themselves in [`mod@effects`], and re-exported
/// here because that is where callers have always found it.
pub use effects::RACE_EFFECTS;

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

/// A race in progress: the world, the track it is on, and the camera behind it.
///
/// **Two halves and nothing else.** [`RaceSim`] decides what happens;
/// [`RaceView`] exists so it can be seen and heard. Until 2026-09-09 this was
/// one struct of ninety-seven fields in which the two were interleaved, and the
/// only thing keeping a renderer handle out of the simulation was that nobody
/// had put one there. The eighteen `impl Race` blocks are unchanged and stay
/// here: most of them read both halves - drawing an exhaust needs a ship's
/// position, and raising a cue needs the bank - and that is what `Race` is for.
/// The one that did not, [`RaceSim::state_hash`], moved.
///
/// Neither half is a crate yet. See [`sim`] for the two fields standing between
/// `RaceSim` and being one.
#[derive(Debug)]
pub struct Race {
    /// Everything that decides what happens next: the world, the track, the
    /// opponents and the rules.
    ///
    /// `pub` because `world` was, and a caller reaching a ship reaches it the
    /// same way it always did with one more hop. See [`sim`].
    pub sim: RaceSim,
    /// Everything this race carries only so it can be shown or heard.
    ///
    /// The camera, the particles, the exhaust, the sound banks and the reticle,
    /// in one place instead of interleaved with the world above. Nothing in
    /// here reaches [`RaceSim::state_hash`], and nothing above it may be reached
    /// *from* here - see [`view`] for the argument.
    view: RaceView,
    /// The recording of this race, if one was asked for, and the ghost it
    /// races against, if any. Neither reaches [`RaceSim::state_hash`] - see
    /// [`replay`].
    replay: replay::ReplayState,
}

/// How fast the kick opens, per second, as an exponential approach.
///
/// Faster than [`BOOST_FOV_CLOSE_RATE`] on purpose: the boost should arrive as a
/// shove and let go slowly. Authored, like [`oag_display::display::BoostFovKick`].
pub const BOOST_FOV_OPEN_RATE: f32 = 9.0;

/// How fast the kick closes again, per second. See [`BOOST_FOV_OPEN_RATE`].
pub const BOOST_FOV_CLOSE_RATE: f32 = 3.5;

#[cfg(test)]
mod tests;
