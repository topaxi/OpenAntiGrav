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
//! [`chase_pos_length`], because the disc's camera offset is signed the other way
//! from the renderer's, and [`MODEL_YAW`], because a `.vex` ship is authored nose
//! along `+Z` while the simulation's forward is `-Z`. Each is one expression with
//! its evidence written next to it.

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
use oag_input::Keyboard;
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

/// The exhaust flicker seed for one craft.
///
/// Slot 0 is [`EXHAUST_SEED`] itself and an opponent is that plus its slot, so
/// eight craft side by side on the grid do not flicker in lockstep. Adjacent
/// seeds are safe to use this way because `Rng::new` runs a SplitMix64 expansion
/// over the seed before the first draw, which is what decorrelates them; a raw
/// LCG would need a stride instead.
fn exhaust_seed(slot: usize) -> u64 {
    EXHAUST_SEED + slot as u64
}

/// Seed for spark spawn parameters, kept distinct from [`SEED`] and
/// [`EXHAUST_SEED`] for the same determinism reason.
pub const SPARKS_SEED: u64 = 0x5_9a_2b_00;

/// Seed for the [`psys::Stage`]'s spawn parameters - the rocket flares and
/// the detonations. Distinct from [`SPARKS_SEED`] for the same reason that
/// one is distinct from [`SEED`].
pub const STAGE_SEED: u64 = 0x5_9a_2b_01;

/// A pad index for [`Race::state_hash`], with a discriminant byte.
///
/// Or "not on a pad" hashes the same as "on pad zero", which is precisely the
/// edge the trigger is built on.
fn write_pad_index(hasher: &mut oag_core::hash::StateHasher, index: Option<usize>) {
    match index {
        None => hasher.write_u8(0),
        Some(index) => {
            hasher.write_u8(1);
            hasher.write_u32(index as u32);
        }
    }
}

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

/// The archive entry name of a team's `.vex` model.
///
/// Assembled the way the loader assembles it, with backslashes, which is what the
/// name hash needs.
///
/// A [`Mode::Zone`] run loads `Zone.vex` instead of `Ship.vex`. That is not a
/// livery swap of convenience: `Ship_LoadModel` (`0x08843258`) switches on the
/// same `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` expression already established
/// as the Zone selector (see `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`),
/// and only that case builds the `%s\Zone.vex` path. Every team's `Zone.vex`
/// decodes to the same 1213 vertices / 1149 triangles / 8 meshes, so the hull
/// itself is shared - only the livery painted on it still varies by team.
#[must_use]
pub fn ship_entry_name(team: &str, mode: Mode) -> String {
    let model = if mode == Mode::Zone {
        ships::ZONE_HULL
    } else {
        ships::HULL
    };
    ships::entry_name(team, model)
}

/// The boost plume that goes with [`ship_entry_name`]'s hull.
///
/// **Zone mode has its own plume file and this used to load the wrong one.**
/// `ship_entry_name` switches the hull between `Ship.vex` and `Zone.vex` on
/// mode; the plume load hardcoded `shipboost.vex`, so a Zone race drew a
/// `Zone.vex` hull with the `Ship.vex` plume. Every team ships a `Zoneboost.vex`
/// beside its `Zone.vex`, so the pairing exists in the data and we simply were
/// not using it.
///
/// Cosmetic today rather than visibly broken - Feisar's `Zoneboost.vex` decodes
/// geometrically identical to its `shipboost.vex` - but "identical on the one
/// team that was checked" is not a reason to keep loading the wrong file, and
/// the caller's missing-entry path already handles a set that does not carry
/// one.
#[must_use]
pub fn boost_entry_name(team: &str, mode: Mode) -> String {
    let model = if mode == Mode::Zone {
        ships::ZONE_BOOST
    } else {
        ships::BOOST
    };
    ships::entry_name(team, model)
}

/// Reads and decodes a model's external PS2 texture set, from the archive
/// entry directly before it.
///
/// **Only attempted when the model's own embedded textures are all
/// missing** - a PSP model already has them and this never runs for one; a
/// PS2 model's texture block is empty by design and this is what replaces
/// it. That gate matters beyond efficiency: the "entry before this one" rule
/// is a directory-position heuristic, not a name or a checked format tag, so
/// it must never have the chance to overwrite a model that already decoded
/// correctly on its own.
///
/// See [`oag_assets::Archives::read_preceding`] for the rule itself and the
/// evidence behind it.
pub(crate) fn ps2_texture_set(
    archives: &mut oag_assets::Archives,
    entry_name: &str,
) -> Option<Vec<Option<mesh::ModelTexture>>> {
    let blob = archives.read_preceding(entry_name).ok()?;
    mesh::ps2_texture_set(&blob).ok()
}

/// What to load, and which of it to draw.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Directories to look in for [downloadable content](crate::dlc), mounted
    /// behind `source`'s own archives.
    ///
    /// Independent of which release `source` is, on purpose: the original tied
    /// a pack to its own territory's disc and this does not. See
    /// `docs/formats/dlc-pack.md`.
    pub dlc: Vec<PathBuf>,
    /// Archive entry name of the track's `.vex`, or `None` for the source's own.
    ///
    /// **`None` rather than a constant, because the two titles share no circuit
    /// directory.** `Data\Environments\16_Track\track.vex` resolves on no Pure
    /// pressing, so a default baked in here - which is what this field used to
    /// carry - meant every Pure race asked for a name that hashes to nothing and
    /// failed inside the archive with a message about a missing entry rather than
    /// about a missing circuit.
    ///
    /// Resolved by [`load`] once the source is open and its title known, from
    /// [`oag_title::RaceDefaults::track`], and the choice is reported. It cannot
    /// be resolved earlier: which title a source is comes from the serial in its
    /// own filesystem, which is read by opening it.
    pub track: Option<String>,
    /// Team name, which selects both the handling stats and the model.
    pub team: String,
    /// The team ids the *opponents* may fly, in the caller's own order.
    ///
    /// The caller supplies them because they come off the player's own disc -
    /// `catalogue::all_teams` over the plugin definition and any mounted DLC
    /// pack - and `load` may not invent a team id any more than it may invent a
    /// path. **Empty means the whole grid wears the player's hull**, which is
    /// what this engine did before liveries and what a source with no readable
    /// definition still gets.
    ///
    /// Which of them ends up in which slot is [`crate::livery::teams_for_slots`],
    /// and is this project's rule rather than the original's.
    pub opponent_teams: Vec<String>,
    /// Speed class the handling parameters are read for.
    pub class: SpeedClass,
    /// How good the opponents are.
    ///
    /// Applied by degrading the measured tuning rather than by boosting a weak
    /// one, and it never reads the player - see [`oag_ai::Difficulty`].
    pub difficulty: oag_ai::Difficulty,
    /// Which mode's rules the race runs under.
    ///
    /// Also selects the HUD layout: a Zone run draws `Zone_HUD.xml`, the other
    /// two share `TimeTrial_HUD.xml`. See [`hud_layout`].
    pub mode: Mode,
    /// Draw the driveable ribbon instead of the track's art meshes.
    ///
    /// **A development view, not a style.** The ribbon is the geometry the
    /// simulation spawns on and queries, so ship-plus-ribbon shows directly
    /// whether the ship is where the physics thinks it is. Default is off, which
    /// means a race draws the map.
    pub ribbon: bool,
    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of - the same view `oag-view --collision` draws, wireframe and
    /// Cage-excluded, on top of whichever track model was chosen above.
    pub collision: bool,
    /// Whether ship and track models draw every child of an authored
    /// `LodGroup`, or only the higher-detail first one. See [`mesh::Lod`].
    pub lod: mesh::Lod,
    /// Force the rest of the grid to spawn even though `mode` says nothing
    /// races there.
    ///
    /// **A verification aid, not a menu choice - the same shape as [`pose`]
    /// and [`camera`] below.** [`Mode::has_opponents`] is `false` for every
    /// mode this crate implements, because the original races all three
    /// single-ship; this exists so `crates/game/tests/race_ground_truth.rs`'s
    /// grid-geometry check and `just play`'s own visual QA can still put
    /// eight craft on the grid without pretending a fourth mode exists.
    /// `false` follows the mode, which is what every real race does.
    ///
    /// [`pose`]: Options::pose
    /// [`camera`]: Options::camera
    pub opponents: bool,
    /// The world generator's seed, or `None` for [`SEED`].
    ///
    /// **A verification aid too**, and it exists because one already-recovered
    /// thing became untestable without it: `crates/game/tests/race_ground_truth.rs`
    /// asserts a fired Turbo's *magnitude* off the disc's own `<Engine turbo>`,
    /// which needs a pad crossing that actually draws a Turbo. That was
    /// automatic while `oag_gameplay::pickup::IMPLEMENTED` held one weapon and
    /// stopped being so the moment it held two. A seed the test can choose is
    /// what keeps that assertion pointed at the weapon it is about, rather than
    /// weakening it to "whatever the pad handed over".
    ///
    /// It changes nothing about a real race: every caller that does not set it
    /// gets [`SEED`], which is the fixed value the field replaced.
    pub seed: Option<u64>,
    /// Put the craft here instead of on its grid slot.
    ///
    /// **A capture aid, not a spawn.** The point is that two circuits, or a
    /// frame of ours and a frame of the original, can be photographed from the
    /// same place instead of by running the same number of ticks and hoping.
    /// `None` spawns normally.
    pub pose: Option<PoseRequest>,
    /// Render from this camera instead of the chase camera.
    ///
    /// The other half of the same capture aid: a ship pose replicates *what*
    /// the original showed, and only a replicated camera replicates *how it was
    /// framed*. `None` uses the chase camera as always.
    pub camera: Option<CameraOverride>,
}

/// Where [`Options::pose`] puts the craft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoseRequest {
    /// A position plus a yaw off the track's own direction there; pitch and
    /// roll come from the nearest spline sample. What `--pose X,Y,Z[,YAW]`
    /// always meant - see
    /// [`oag_gameplay::spawn::Pose::from_position_on_sample`].
    SplineAligned {
        /// Where to put the craft, world space.
        position: Vec3,
        /// Radians off the spline tangent at that point.
        yaw: f32,
    },
    /// A full pose, applied verbatim - a captured trace row's position and
    /// basis, nothing recomputed from the spline.
    Exact(Pose),
}

/// A camera pose imposed from outside, replacing the chase camera.
///
/// The one seam is [`Race::view`]: [`Race::camera_position`] derives from the
/// view matrix, so the PVS culling eye and the fog eye follow the override
/// without knowing it exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraOverride {
    /// The camera eye, world space.
    pub eye: Vec3,
    /// The camera's orientation, in the same convention as the ship body's:
    /// `-Z` is the look direction, `Y` is up. What
    /// [`oag_trace::replay::camera_orientation_of`] produces.
    pub orientation: Quat,
    /// Replace the disc's authored fov, in **vertical degrees**, with this
    /// value. `None` keeps the authored one.
    ///
    /// This was the calibration knob for settling the unit, and the unit is now
    /// settled (confidence 94). What it is *for* has changed rather than gone
    /// away: a matched-pose render still needs it, because
    /// [`oag_gameplay::spawn::Ship::place_at`] resets the body, so a posed craft
    /// has zero velocity and never gets the original's speed-dependent widen.
    /// Pass the fov computed from the captured tick's own forward velocity.
    pub fov_deg: Option<f32>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: crate::source::DEFAULT_IMAGE.to_string(),
            difficulty: oag_ai::Difficulty::default(),
            // Empty rather than the search path: a default that read the
            // filesystem would make two runs of the same test differ by what
            // the developer happens to have downloaded. The composition root
            // fills this in from `source::resolve_dlc`.
            dlc: Vec::new(),
            track: None,
            team: DEFAULT_TEAM.to_string(),
            // Empty for the same reason `dlc` is: the ids come off the
            // player's own disc, and `load` may not invent one. The
            // composition root fills it from the catalogue.
            opponent_teams: Vec::new(),
            class: SpeedClass::Venom,
            mode: Mode::default(),
            ribbon: false,
            collision: false,
            lod: mesh::Lod::Both,
            opponents: false,
            seed: None,
            pose: None,
            camera: None,
        }
    }
}

/// Everything a race needs with no GPU anywhere in sight.
///
/// Split out from [`Loaded`] so a test can build one by hand, or take one off a
/// disc, and run a whole race against it on a machine with no graphics driver.
#[derive(Debug, Clone)]
pub struct Setup {
    /// Which mode's rules the race runs under.
    pub mode: Mode,
    /// How good the opponents are. See [`Options::difficulty`].
    pub difficulty: oag_ai::Difficulty,
    /// The speed class the race is run in.
    ///
    /// Most of what the class decides is already resolved by the time a `Setup`
    /// exists - the handling block, the gravity scale and the speed-pad tunables
    /// are all per-class and all baked in above. It is carried anyway because
    /// [`Self::weapons`]' pickup odds are indexed by class *inside* a table this
    /// keeps whole, and resolving that at load would throw away the other
    /// classes' rows for no gain.
    pub class: SpeedClass,
    /// Whether to spawn the rest of the grid regardless of what `mode` says.
    ///
    /// The resolved form of [`Options::opponents`] - see its doc comment.
    /// `mode.has_opponents()` already decides this correctly for every mode
    /// this crate implements; the field exists only so a verification build
    /// can override that decision.
    pub opponents: bool,
    /// The world generator's seed, already resolved from [`Options::seed`].
    pub seed: u64,
    /// The decoded spline graph, as the file has it.
    pub ai: AiTrack,
    /// The spline resampled for locating a ship, and for placing it.
    pub spline: Spline,
    /// Zone mode's speed law and recharge, when the source carries them.
    pub zone: Option<oag_formats::handling::Zone>,
    /// The same graph walked into a closed ring, for lap counting.
    ///
    /// `None` when the primary chain does not close, which means this track gets
    /// no lap counter rather than a wrong one. The load report says so.
    pub course: Option<Course>,
    /// The track's own authored grid slot, when it has one.
    ///
    /// `None` is not a broken track: it means this source's file carries no
    /// `Start Position` node, and a ship goes on the spline instead. Every PSP
    /// track has one, so on that path this is always `Some`.
    pub start_position: Option<StartPosition>,
    /// Every collidable triangle of the track.
    pub collision: CollisionWorld,
    /// The force law's parameter set for one team in one speed class.
    pub handling: Handling,
    /// How far and how fast the airbrake flaps move, from the same document.
    ///
    /// Separate from [`Self::handling`] because it is per *team* rather than
    /// per speed class, and because no force term reads it - see
    /// `oag_gameplay::airbrake_graphics_for`.
    pub airbrake_graphics: oag_gameplay::AirbrakeGraphics,
    /// The per-class scale on grounded gravity, from `<GlobalClass><GravityMul/>`.
    ///
    /// `1.0` when the engine-wide file could not be read, which leaves gravity
    /// exactly as it was before this was decoded. On the sim half deliberately:
    /// it reaches `oag_physics::forces::Environment` every tick and a headless
    /// race must fall the same way a drawn one does.
    pub class_gravity_scale: f32,
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
    /// The same seven from `<ExternalCameraClose>`: the nearer of the two
    /// external views [`crate::display::CameraView`] cycles.
    pub chase_close: ChaseParams,
    /// The cockpit view's five values, from `<InternalCamera>`.
    pub internal: InternalParams,
    /// Each grid slot's `engine_flare` locator, in **that slot's own hull's**
    /// model space. Slot 0 is the player's.
    ///
    /// `None` means that hull carries no `Engine Flare` node, and the exhaust
    /// is then not drawn rather than guessed at. Every team whose `Ship.vex`
    /// resolves by name has **exactly one**, a direct child of `world` - see
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`. One nozzle, centred,
    /// not one per visible engine.
    ///
    /// **Per slot since liveries landed.** The eight hulls are different
    /// models, so one team's locator carried onto another's craft puts the
    /// flare inside the fuselage.
    pub nozzles: Vec<Option<Vec3>>,
    /// The `Ship Collision Fx` locators, in the ship model's own space.
    ///
    /// The original attaches up to 10 and `Ship_DispatchCollisionFx`
    /// (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) triggers the
    /// one **nearest the contact**; the spark burst then emits from that
    /// node as it rides the hull. Empty means the model authors none, and
    /// the burst falls back to anchoring at the contact point itself.
    pub collision_fx: Vec<Vec3>,
    /// Every [`RACE_EFFECTS`] entry that loaded, parsed from the disc's own
    /// `Data\Psys\*.POB`.
    ///
    /// An effect missing from here - no such archive entry, or a blob that
    /// would not parse - is **not drawn** rather than approximated: every
    /// number that shapes one lives in its file, so there is nothing left to
    /// fall back to and a stand-in would be this engine's invention rather
    /// than the game's. Headless tests that build a `Race` by hand leave the
    /// library empty, which is the same path.
    pub effects: psys::Library,
    /// The track's speedup pads, as trigger volumes.
    ///
    /// The same nodes [`Loaded::pad_model`] draws, decoded for what they *do*
    /// rather than for what they look like: an oriented box in each pad's own
    /// space, plus the world matrix that places it and gives the push its
    /// direction. Empty for a track that authors none.
    ///
    /// Nothing consumes these yet - the boost is the next milestone - but they
    /// belong to the simulation half rather than to [`Loaded`], because a
    /// headless race has to be able to trigger a pad without a GPU.
    pub speedup_pads: Vec<oag_formats::pads::PadVolume>,
    /// The track's weapon pads, as trigger volumes.
    ///
    /// Everything [`Self::speedup_pads`] says applies, one class over. Consumed
    /// by [`Race::test_weapon_pads`] in the one mode that arms them, and decoded
    /// on every mode regardless, which is what lets
    /// `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    /// them. Two independent decodes of the same nodes agreeing is worth more
    /// than either alone.
    pub weapon_pads: Vec<oag_formats::pads::PadVolume>,
    /// The weapon table for this race, out of `WeaponStats_Race.xml`.
    ///
    /// `None` when the file is absent or unreadable, which means a pad hands
    /// nothing out rather than handing out an invented weapon - the same choice
    /// [`Setup::speedup_pads`]' tunables make. See `docs/formats/weapon-stats.md`.
    pub weapons: Option<oag_formats::weapons::WeaponStats>,
    /// Seconds a weapon pad is inert for after it is crossed, for this race's
    /// speed class.
    ///
    /// `<WeaponPad refresh_time>` out of the same `<GlobalClass>` block the
    /// speed-pad tunables come from, stamped onto the pad by the original's
    /// `WeaponPads_TestCraft` - see [`oag_formats::handling::WeaponPad`], which
    /// records why it is a debounce rather than a respawn. Zero when the global
    /// file could not be read.
    pub weapon_pad_refresh: f32,
    /// Where [`Options::pose`] asked for the craft to start, already resolved
    /// against the spline. `None` uses the ordinary spawn.
    pub pose_override: Option<Pose>,
    /// [`Options::camera`], carried through to [`Race::view`]. Plain data, so
    /// it rides in the simulation half even though only the renderer reads it.
    pub camera_override: Option<CameraOverride>,
}

/// A [`Setup`] plus the geometry to draw it with.
#[derive(Debug)]
pub struct Loaded {
    /// The simulation half.
    pub setup: Setup,
    /// The HUD's layout, atlas, fonts and strings.
    pub hud: crate::hud::Assets,
    /// What to draw for the track.
    pub track_model: Model,
    /// One hull, plume, nozzle and spark-anchor set per grid slot, each off
    /// its own team's directory. Slot 0 is the player's.
    ///
    /// Per slot rather than one shared model since 2026-08-15: the eight teams
    /// the disc declares are eight *different* hulls (845 to 1,497 triangles),
    /// not one hull repainted, so a shared model was visibly one team's ship
    /// eight times over. See [`crate::livery`], which also records which half
    /// of this is recovered and which half is this project's.
    /// The model a Rocket in flight is drawn as: `Data\Weapons\Rocket.vex`.
    ///
    /// **Recovered, confidence 85.** `Rocket_Ctor` (`0x0885cc24`) builds the
    /// projectile a scene node of `.vex` class `0x3e9` from exactly this entry
    /// and keeps the handle at `+0x110`, which `Rocket_Update` drives every
    /// tick. See `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    /// **A rocket is a model, not a billboard**, which is what this engine drew
    /// before. The file is 3712 bytes, 84 vertices, 28 triangles, radius 1.34:
    /// a finned dart.
    ///
    /// `None` when the source carries no entry under that name, on the same
    /// terms as [`Self::boost_model`] - a missing rocket model falls back to the
    /// billboard rather than failing the race.
    pub rocket_model: Option<Model>,
    pub liveries: Vec<Livery>,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// The track's authored `fogCube` volumes, for [`Scene`] to sample per
    /// frame at the camera.
    pub fog_volumes: Vec<oag_formats::fog::FogVolume>,
    /// The track's `Skycube`, when it authors one.
    ///
    /// Built from the same blob as [`Self::track_model`] and indexing the same
    /// textures, but kept separate because it is drawn camera-centred and out of
    /// depth. `None` for a driveable-ribbon build, which has no art meshes at
    /// all, and for any track that authors no sky.
    pub sky_model: Option<Model>,
    /// The track's `Speedup Pad` geometry, when it authors any.
    ///
    /// Built from the same blob as [`Self::track_model`] and drawn on the same
    /// pipeline, but kept separate because pads belong to no visibility
    /// `section` and the track model's draw calls are what the PVS indexes.
    /// `None` for a driveable-ribbon build and for any track that authors none.
    pub pad_model: Option<Model>,
    /// The track's `Weapon Pad` geometry, when it authors any.
    ///
    /// Everything [`Self::pad_model`]'s note says applies here too; the two are
    /// separate because they are separate gameplay objects. **Nothing consumes
    /// a weapon pad yet** - it is drawn and no more.
    pub weapon_pad_model: Option<Model>,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` when the track declares no `section` nodes - a driveable-ribbon
    /// build has no art meshes to place, and Pure tracks do not use Pulse's
    /// class numbering at all. The first tier is then skipped.
    pub visibility: Option<TrackVisibility>,
    /// The trail ribbon's noise texture off the disc, when it decodes.
    pub noise: Option<FlareTexture>,
    /// The engine-flare texture off the disc, when it decodes.
    ///
    /// `None` falls back to [`Exhaust`]'s procedural glow, and the load report
    /// says so - it is not a silent substitution.
    pub flare: Option<FlareTexture>,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}

/// The exhaust sprite's texture, named as a literal string in the executable.
///
/// At `0x08a84c80`, loaded by `Texture_LoadEngineFlare`. Being a literal means the
/// WAD lookup is an exact `wad::hash_name` hit rather than a mined candidate, which
/// is unusual for this project and worth the note.
pub const FLARE_TEXTURE: &str = r"Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip";

/// The trail ribbon's texture, also a literal in the executable.
///
/// At `0x08a889e4`, loaded by `Texture_LoadEngineNoise` into three slots that
/// `Trail_DrawRibbon` indexes per layer. Note the directory case differs from
/// [`FLARE_TEXTURE`]'s - `engineFlare` here, `EngineFlare` there - which does not
/// matter, since the WAD hash is case-insensitive.
pub const NOISE_TEXTURE: &str = r"Data\Tex\engineFlare\Engine_noise.mip";

/// Decodes a `.mip` texture out of the archive set.
///
/// Returns the pixels and a line for the load report, or the reason it could not -
/// **as text, not as `None`**. A missing entry and a blob that does not parse are
/// different problems with different fixes (name mining versus the decoder), and a
/// silent fallback hides which one happened.
/// Reads one `Data\Psys\<name>.POB` out of the archive set and parses it
/// into a playable effect.
///
/// The note it returns goes in the loader report next to the other assets',
/// because "the sparks did not appear" and "the file was not found" are
/// otherwise indistinguishable from a screenshot - the failure mode the
/// `--give` flag exists to remove for weapons.
fn particle_effect(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<(psys::Effect, String), String> {
    let path = sparks::effect_path(name);
    let blob = archives
        .read_name(&path)
        .map_err(|e| format!("{path}: not in the archive set ({e})"))?;
    // The one thing about a `.pob` that is a property of the release rather
    // than of the file - see `psys::ColourScale`. Read off the source
    // because it cannot be read off the bytes.
    let scale = match archives.layout.platform {
        oag_assets::Platform::Ps2 => psys::ColourScale::Half,
        _ => psys::ColourScale::Full,
    };
    let effect = psys::Effect::parse(&blob, scale)
        .map_err(|e| format!("{path}: {} bytes, does not parse ({e})", blob.len()))?;
    let note = format!(
        "{name}: {} emitter(s) - {}",
        effect.emitters.len(),
        effect
            .emitters
            .iter()
            .map(|spec| spec.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok((effect, note))
}

fn mip_texture(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<(FlareTexture, String), String> {
    let blob = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let texture = oag_formats::texture::Texture::parse(&blob)
        .map_err(|e| format!("{name}: {} bytes, does not parse ({e})", blob.len()))?;
    let note = format!(
        "{name}: {}x{}, {} mip level(s)",
        texture.width, texture.height, texture.mip_levels
    );
    Ok((
        FlareTexture {
            width: u32::from(texture.width),
            height: u32::from(texture.height),
            rgba: texture.to_rgba(),
        },
        note,
    ))
}

/// Loads a track, a ship and its handling out of a disc image.
///
/// # Errors
///
/// Propagates a missing archive, a missing entry, a `.vex` that does not decode,
/// a `WO Track` payload the spline parser cannot account for, and a
/// `handlingstats.xml` with an attribute missing.
pub fn load(options: &Options) -> Result<Loaded> {
    let mut report = Vec::new();

    // Which archives this source has, rather than which archives a PSP disc has.
    // Every entry name below is the same on both releases - the PS2 build ships
    // `Data\Ships\<Team>\handlingstats.xml` and `Data\Environments\<n>_Track\...`
    // under the names the PSP uses - so the layout is the whole of the
    // difference. See `docs/formats/handling-stats.md`.
    //
    // Downloadable content is mounted behind them. A pack is not tied to the
    // release it was sold for here, so this is the same call whichever image
    // `source` names - see `docs/formats/dlc-pack.md`. **A Pure source mounts no
    // packs at all**, which `crate::title::open_source` decides rather than this
    // call site; see its own docs for why silently mounting Pulse's DLC behind a
    // different title would be worse than mounting none.
    let (packs, problems) = crate::dlc::packs(&options.dlc, &crate::boot::default_dlc_cache_dir());
    // **Opened as whichever title the source turned out to be**, the same way the
    // boot path already does it. This used to be `oag_pulse::open_with_packs`
    // outright, which refused a Pure disc by name (`WrongTitle`) - so `--race`
    // and the menus' own `Launch Game` could not reach a second title at all,
    // however much of the rest of the path was ready for one.
    let opened = crate::title::open_source(&options.source, packs)?;
    let title = opened.title;
    let mut archives = opened.archives;
    report.push(format!("racing on {}", title.name));
    report.push(archives.layout.describe());
    for pack in &archives.packs {
        report.push(format!("dlc: {}", pack.label()));
    }
    report.extend(problems.into_iter().map(|p| format!("dlc: {p}")));

    // **The circuit this source actually offers**, resolved here because this is
    // the first point the title is known - see [`Options::track`]. Reported
    // either way, so a run that took a default says which title's default it was
    // rather than leaving a reader to work it out from the path.
    let track = match &options.track {
        Some(asked) => asked.clone(),
        None => {
            report.push(format!(
                "no track named: {}'s own default, {}",
                title.name, title.race.track
            ));
            title.race.track.to_string()
        }
    };

    let spec = archives
        .locate(&track)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} is in none of this source's archives ({})",
                track,
                archives.layout.describe()
            )
        })?
        .to_string();

    let (ai, label) = track_render::load(&spec, &track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    let read = |archives: &mut oag_assets::Archives, name: &str| -> Result<Vec<u8>> {
        archives
            .read_name(name)
            .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
    };

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice. See the
    // wanted-change note in `docs/tools/oag-game.md`. On the PS2 that costs more
    // than it does on the PSP, where nothing is compressed at all: 5,861 of
    // `WADS2.WAD`'s 7,200 entries are LZSS.
    let track_blob = read(&mut archives, &track)?;
    // **This track file's own class numbering.** Read once, off its version word,
    // and consulted everywhere below that used to spell a `vex::CLASS_*`
    // constant - all of which are version 6's. A version-4 track (Pure's are)
    // uses different numbers entirely, so the constants would find nothing here
    // or, worse, find the wrong node type. An id this project has not recovered
    // for this version comes back `None`, and each site below says what it does
    // with that rather than substituting version 6's answer. See
    // `oag_formats::vex::classes`.
    let track_classes =
        oag_formats::vex::classes_of(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
    let speedup_pad_class = track_classes.speedup_pad;
    let weapon_pad_class = track_classes.weapon_pad;
    let start_position = start_position_of(&track_blob);
    match start_position {
        Some(slot) => report.push(format!(
            "Start Position: {:?} facing {:?}",
            slot.position, slot.forward
        )),
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes = collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
    let collision = collision_world(&nodes);
    report.push(format!(
        "{} collision node(s) -> {} collider(s), {} triangle(s)",
        nodes.len(),
        collision.colliders().len(),
        collision
            .colliders()
            .iter()
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum::<usize>()
    ));

    // The pads' trigger volumes, from the same blob the geometry comes from.
    // Reported with the distance from the grid to the nearest one, because that
    // is the number anyone testing a pad needs and there is nowhere else to get
    // it: a pad is a plate on the track surface with nothing to distinguish it
    // in a screenshot.
    let speedup_pads = oag_formats::vex::nodes(&track_blob)
        .map(|nodes| {
            speedup_pad_class
                .map(|class| oag_formats::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    if speedup_pads.is_empty() {
        report.push("the track authors no Speedup Pad trigger volumes".to_string());
    } else {
        let nearest = start_position.map(|slot| {
            speedup_pads
                .iter()
                .map(|pad| Vec3::from_array(slot.position).distance(Vec3::from_array(pad.centre())))
                .fold(f32::INFINITY, f32::min)
        });
        report.push(match nearest {
            Some(d) => format!(
                "{} speedup pad trigger volume(s), nearest {d:.0} units from the grid",
                speedup_pads.len()
            ),
            None => format!("{} speedup pad trigger volume(s)", speedup_pads.len()),
        });
    }
    // The pickup pads, decoded the same way and **unconditionally**, unlike the
    // mesh below: nothing triggers them yet, so this is asset data rather than
    // gameplay, and `the_weapon_pads_are_drawn_where_they_trigger` checks the
    // decode against the drawn geometry regardless of mode. The original
    // decodes them unconditionally too - `World_CollectNodeLists` always walks
    // the tree - it only suppresses what a *weapons-off* mode does with the
    // result afterward, which is what `weapon_pad_model`'s own report line
    // below says plainly rather than repeating here.
    let weapon_pads = oag_formats::vex::nodes(&track_blob)
        .map(|nodes| {
            weapon_pad_class
                .map(|class| oag_formats::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    report.push(if weapon_pads.is_empty() {
        "the track authors no Weapon Pad trigger volumes".to_string()
    } else {
        format!("{} weapon pad trigger volume(s)", weapon_pads.len())
    });

    let stats_name = handling::entry_name(&options.team);
    let stats_blob = read(&mut archives, &stats_name)?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    // **A borrowed pitch response is never silent.** Pure authors no `<pitch>` in
    // any `<Class>`, so `handling_for` substitutes
    // `oag_gameplay::handling::PITCH_STAND_IN` - another title's tuning on a ship
    // whose own is unknown. That is exactly the kind of substitution that looks
    // like a physics bug three sessions later if nobody wrote it down, so it is
    // named here with the classes it applied to.
    let unauthored: Vec<&str> = stats
        .classes
        .iter()
        .filter(|class| class.pitch.is_none())
        .map(|class| class.raw_name.as_str())
        .collect();
    if !unauthored.is_empty() {
        report.push(format!(
            "{stats_name}: {} of {} class(es) author no <pitch> ({}); flying on \
             oag_gameplay::handling::PITCH_STAND_IN, which is Pulse's block and \
             not this title's",
            unauthored.len(),
            stats.classes.len(),
            unauthored.join(", ")
        ));
    }
    // The engine-wide block, out of `Data\XML\HandlingStats.xml` rather than this
    // team's file - see `handling::GLOBAL_ENTRY`. Read on **every** mode now, not
    // only Zone: the speed-pad tunables live here too and every mode has pads.
    // Three distinct failures, reported as three distinct lines. The decoder was
    // deliberately tightened so an incomplete `<Global>` names the element,
    // attribute or class that is missing rather than returning a zero, and
    // collapsing that into one "unreadable" would throw the whole point away -
    // especially now that `<Zone>` and `<SpeedupPads>` share one result, so a
    // malformed pad block would otherwise be reported as a missing Zone law.
    let global = match read(&mut archives, handling::GLOBAL_ENTRY) {
        Err(e) => {
            report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
            None
        }
        Ok(blob) => match handling::global_from_blob(&blob) {
            Err(e) => {
                report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
                None
            }
            Ok(None) => {
                report.push(format!(
                    "{} carries no <Global> block",
                    handling::GLOBAL_ENTRY
                ));
                None
            }
            Ok(some) => some,
        },
    };
    // An absent or unreadable file means no boost and no auto-speed rather than
    // invented numbers. Said out loud, because a speed pad that quietly does
    // nothing reads as a physics bug and gets looked for in the force law.
    let pad_tunables = match global {
        Some(global) => global.speedup_pads(to_format_class(options.class)),
        None => {
            report.push("speed pads apply no boost this run".to_string());
            handling::SpeedupPads::default()
        }
    };
    // `<Special speedpad_jump>`, out of the same file and with the same fallback
    // reasoning: a zero means the boost simply does not tilt, which is a missing
    // feature rather than a broken race.
    let special = global.map(|global| global.special).unwrap_or_default();
    // Reported for the same reason the pad tunables are: a tilt that silently
    // reads zero is indistinguishable from a tilt that is not implemented, and
    // this is the only place the number's journey off the disc is observable.
    report.push(format!(
        "<Special speedpad_jump>: {} - the boost tilts {:.2} degrees toward the \
         hull's up while the pitch axis is held up",
        special.speedpad_jump,
        special.speedpad_jump.atan().to_degrees()
    ));
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once
    // and this is not the place it happens, and neither `<SpeedupPads>` nor
    // `<Special>` is one of them.
    let handling = handling_for(&stats, options.class, pad_tunables, special);
    // `<AirbrakeGraphics>` is the fifth pre-scaled field and it *is* converted
    // here, by its own function: it is per team rather than per class, and it
    // is graphics, so it deliberately never enters `Handling`.
    let airbrake_graphics = oag_gameplay::airbrake_graphics_for(&stats);
    // The per-class scale on grounded gravity, `g_class_gravity_scale`. **The
    // fallback is the identity, not zero**, unlike the pad tunables above: a
    // missing boost is a missing feature, but a zero here would leave a grounded
    // craft weightless, which is not a degraded race - it is a broken one.
    //
    // Named `airborne` in the XML and applied to the *grounded* term; see
    // `oag_formats::handling::GravityMul`, which reads the VFPU pair chain out.
    let class_gravity_scale = match global {
        Some(global) => {
            let scale = global.gravity_mul(to_format_class(options.class)).airborne;
            report.push(format!(
                "<GravityMul>: grounded gravity scaled by {scale} for the {:?} class",
                options.class
            ));
            scale
        }
        None => {
            report.push("gravity is unscaled this run".to_string());
            1.0
        }
    };
    let zone = if options.mode == Mode::Zone {
        match global {
            Some(global) => report.push(format!(
                "<Zone>: start {}, increment {} per zone, recharge {}",
                global.zone.start, global.zone.increment, global.zone.recharge
            )),
            None => report.push("this run has no auto-speed".to_string()),
        }
        global.map(|g| g.zone)
    } else {
        None
    };
    // The weapon-pad debounce, out of the same `<GlobalClass>` block as the two
    // above and with the same "absent means the feature is off" fallback. Zero
    // is a real degradation rather than a neutral value - it would let one
    // crossing grant a pickup on every tick the hull is inside the volume - so
    // the trigger treats a zero as "grant once and never again on this pad"
    // rather than trusting it; see `Race::test_weapon_pads`.
    let weapon_pad_refresh = match global {
        Some(global) => {
            global
                .weapon_pads(to_format_class(options.class))
                .refresh_time
        }
        None => 0.0,
    };
    // The weapon table. Read on every mode even though only a single race arms
    // the pads: which mode is racing is a gameplay question and this is the
    // asset half, and reading it unconditionally is what makes a broken file a
    // reported line on every run rather than one nobody sees until they pick
    // the one mode that needs it.
    //
    // Two failures, two lines, for the reason `<Global>` above gives at length.
    let weapons = match read(&mut archives, oag_formats::weapons::RACE_ENTRY) {
        Err(e) => {
            report.push(format!("{}: {e}", oag_formats::weapons::RACE_ENTRY));
            None
        }
        Ok(blob) => match oag_formats::weapons::from_blob(&blob) {
            Err(e) => {
                report.push(format!("{}: {e}", oag_formats::weapons::RACE_ENTRY));
                None
            }
            Ok(stats) => {
                report.push(format!(
                    "{}: {} weapon(s) with an absorb value, {} pickup table(s)",
                    oag_formats::weapons::RACE_ENTRY,
                    stats.absorb.len(),
                    stats.pickups.len()
                ));
                Some(stats)
            }
        },
    };
    if options.mode.weapons_enabled() && weapons.is_none() {
        // Only worth saying on a mode that would otherwise hand something out.
        report.push("weapon pads hand nothing out this run".to_string());
    }
    let far = stats.external_camera_far;
    let chase = chase_params(far);
    // The other two views the player can cycle to, read from the same document
    // and through the same conversion - so the sign convention is reconciled in
    // exactly one place for both external blocks, which is what
    // `chase_pos_length` exists for.
    let chase_close = chase_params(stats.external_camera_close);
    let internal = InternalParams {
        fov: stats.internal_camera.fov,
        headtilt: stats.internal_camera.headtilt,
        height: stats.internal_camera.height,
        length: stats.internal_camera.length,
        pitch: stats.internal_camera.pitch,
    };
    report.push(format!(
        "{stats_name}: team {:?}, {:?} class, mass {}, ride_height {}",
        stats.team, options.class, handling.physical.mass, handling.antigrav.ride_height
    ));
    // The eye offsets are reported **after** `craft_scale`, because that is
    // where the eye actually ends up, and the two differ now that the scale is a
    // rig parameter rather than a factor folded into the four offsets. Reporting
    // the authored numbers under the word "eye" would be a report that lies -
    // "eye 4 up" for a rig whose eye is 3 up.
    let eye_offsets = |chase: &ChaseParams| {
        (
            chase.pos_height * chase.craft_scale,
            chase.pos_length * chase.craft_scale,
        )
    };
    let (far_up, far_back) = eye_offsets(&chase);
    let (close_up, close_back) = eye_offsets(&chase_close);
    report.push(format!(
        "<ExternalCameraFar>: fov {} as vertical degrees (confirmed against the \
         original's own frame), pos_length {} on disc -> eye {far_up} up and \
         {far_back} back (authored offsets times the craft's {} scale)",
        chase.fov, far.pos_length, chase.craft_scale
    ));
    report.push(format!(
        "<ExternalCameraClose>: fov {}, eye {close_up} up and {close_back} back; \
         <InternalCamera>: fov {}, eye {} up and {} forward, pitch {} over {} units",
        chase_close.fov,
        internal.fov,
        internal.height,
        internal.length,
        internal.pitch,
        oag_render::camera::internal::AIM_DISTANCE,
    ));

    // One hull, plume and nozzle per grid slot, each off its own team's
    // directory - see `crate::livery`, which also records what is recovered
    // here (the paths) and what is this project's (which team flies which
    // slot). Slot 0 is the player's.
    // Which teams the field may fly. The caller's list wins, because only the
    // caller can know about mounted DLC packs - the front end passes the whole
    // catalogue. When it is empty the disc's own plugin definition is read
    // here instead, which is what makes `--race`, a capture and a test field a
    // varied grid rather than eight identical ships; an entry point that
    // forgot to pass the list is exactly how this was first found.
    let available: Vec<String> = if options.opponent_teams.is_empty() {
        let declared = archives
            .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
            .ok()
            .and_then(|blob| oag_formats::fexml::expand(&blob).ok())
            .map(|xml| crate::catalogue::teams(&xml))
            .unwrap_or_default();
        report.push(format!(
            "{}: {} team(s) for the grid, read here because the caller passed none",
            oag_pulse::names::GAME_PLUGIN_DEFINITION,
            declared.len()
        ));
        declared.into_iter().map(|team| team.id).collect()
    } else {
        options.opponent_teams.clone()
    };
    let slot_teams = livery::teams_for_slots(&options.team, &available, oag_gameplay::MAX_SHIPS);
    let liveries = livery::load(
        &mut archives,
        &slot_teams,
        options.mode,
        options.lod,
        &mut report,
    )?;
    report.push(format!(
        "grid liveries: {} - which team flies which slot is this project's, not \
         the original's (livery.rs)",
        slot_teams.join(", ")
    ));
    // The Rocket's own model, on the same terms as the boost plume: absence is
    // reported, not fatal. It is not per-team and not per-track - one entry
    // serves every rocket in the game, which is why it loads here once and the
    // scene clones it per projectile slot.
    let rocket_model = match archives.read_name(ROCKET_MODEL_ENTRY) {
        Ok(blob) => match mesh::build_with_textures(ROCKET_MODEL_ENTRY, &blob, None, options.lod) {
            Ok(model) => {
                report.push(format!(
                    "{ROCKET_MODEL_ENTRY}: {} triangle(s), radius {:.2} - a rocket in \
                     flight is this model, not a billboard",
                    model.indices.len() / 3,
                    model.radius
                ));
                Some(model)
            }
            Err(error) => {
                report.push(format!("{ROCKET_MODEL_ENTRY}: did not decode ({error})"));
                None
            }
        },
        Err(_) => {
            report.push(format!(
                "{ROCKET_MODEL_ENTRY}: absent - rockets fall back to a billboard"
            ));
            None
        }
    };

    // Shared with the sky and the pads below: all three are node classes inside
    // the same track file, and a material in any of them names a texture by its
    // ordinal among *all* of the file's `Texture` nodes, not a per-class one -
    // see `mesh::build_sky`'s doc comment. Resolved once here, against the track
    // model's own embedded textures, and reused rather than re-read from the
    // archive and re-decoded per model.
    let mut ps2_track_textures: Option<Vec<Option<mesh::ModelTexture>>> = None;
    let track_model = if options.ribbon {
        track_render::build_model(&label, &ai)
    } else {
        let mut track_model = mesh::build_with_textures(&track, &track_blob, None, options.lod)?;
        // Same PS2 signature and the same directory-position heuristic as the
        // ship above. Checked separately for tracks specifically (not just
        // assumed from the ship result): the entry directly before a track's
        // own `.vex` decodes as a texture set with the same entry count as the
        // model's `Texture` nodes on 27 of the 32 `<n>_Track`/`track_reversed`
        // pairs on the PS2 disc, off by only 1-2 slots (never more) on the
        // rest - see `docs/formats/ps2-texture.md`.
        if !track_model.textures.is_empty()
            && track_model.textures.iter().all(Option::is_none)
            && let Some(external) = ps2_texture_set(&mut archives, &track)
        {
            ps2_track_textures = Some(external.clone());
            track_model =
                mesh::build_with_textures(&track, &track_blob, Some(external), options.lod)?;
        }
        track_model
    };
    // The track's authored fog volumes. Empty for a ribbon build, and empty for
    // the four circuits that author no `fogCube` at all - both ordinary, and
    // both meaning the race renders unfogged.
    let fog_volumes = if options.ribbon {
        Vec::new()
    } else {
        let nodes = oag_formats::vex::nodes(&track_blob).unwrap_or_default();
        let volumes = oag_formats::fog::volumes(&track_blob, &nodes);
        report.push(match volumes.first() {
            None => "the track authors no fogCube; the race is unfogged".to_string(),
            Some(v) => format!(
                "fog: {} volume(s), {:.0} units across, {:.0}..{:.0} at the near end",
                volumes.len(),
                v.edge,
                v.near_end.near,
                v.near_end.far,
            ),
        });
        volumes
    };

    // The ribbon build draws an invented surface rather than the disc's art, so
    // it gets no sky either: the two belong to the same "show what shipped"
    // mode.
    let sky_model = if options.ribbon {
        None
    } else {
        let sky = mesh::build_sky(&track, &track_blob, ps2_track_textures.clone())?;
        if sky.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.skycube,
                "Skycube",
                "the sky stays black",
            ));
            None
        } else {
            report.push(format!(
                "drawing the track's sky: {} triangle(s), {} material(s)",
                sky.indices.len() / 3,
                sky.draws.len() + sky.alpha_tested_draws.len() + sky.transparent_draws.len(),
            ));
            Some(sky)
        }
    };

    // Same reasoning as the sky: a ribbon build shows an invented surface, so it
    // shows none of the disc's art meshes, pads included.
    let pad_model = if options.ribbon {
        None
    } else {
        let pads = mesh::build_pads(&track, &track_blob, ps2_track_textures.clone())?;
        if pads.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.speedup_pad,
                "Speedup Pad",
                "no pad plates are drawn",
            ));
            None
        } else {
            report.push(format!(
                "drawing the track's speedup pads: {} triangle(s), {} material(s)",
                pads.indices.len() / 3,
                pads.draws.len() + pads.alpha_tested_draws.len() + pads.transparent_draws.len(),
            ));
            Some(pads)
        }
    };
    // The pickup pads, built the same way and kept separate for the reason
    // `mesh::build_weapon_pads` gives: they are a different gameplay object and
    // a merged buffer could not show one without the other. **Nothing hands
    // anything out yet** - see the roadmap's weapons item.
    //
    // **Decoded here regardless of `options.mode`.** Whether it actually reaches
    // the screen is a `Scene::new` decision, not this one - see its own comment,
    // and `Mode::weapons_enabled`. Keeping the decode unconditional is what lets
    // `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    // the trigger volumes on every mode's own default `Options`, and it mirrors
    // the original's own order of operations: `World_CollectNodeLists` always
    // walks the tree before anything asks whether weapons are on.
    let weapon_pad_model = if options.ribbon {
        None
    } else {
        let pads = mesh::build_weapon_pads(&track, &track_blob, ps2_track_textures.clone())?;
        if pads.indices.is_empty() {
            report.push(unrecovered_or_absent(
                track_classes,
                track_classes.weapon_pad,
                "Weapon Pad",
                "no pickup plates are drawn",
            ));
            None
        } else {
            report.push(format!(
                "{} the track's weapon pads: {} triangle(s), {} material(s)",
                if options.mode.weapons_enabled() {
                    "drawing"
                } else {
                    "decoded but not drawing (weapons off in this mode)"
                },
                pads.indices.len() / 3,
                pads.draws.len() + pads.alpha_tested_draws.len() + pads.transparent_draws.len(),
            ));
            Some(pads)
        }
    };
    report.push(format!(
        "drawing the track's {}: {} triangle(s), radius {:.0}",
        if options.ribbon {
            "driveable ribbon"
        } else {
            "art meshes"
        },
        track_model.indices.len() / 3,
        track_model.radius
    ));

    for model in [
        Some(&liveries[0].hull),
        Some(&track_model),
        sky_model.as_ref(),
        pad_model.as_ref(),
        weapon_pad_model.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(line) = untextured_note(model) {
            report.push(line);
        }
    }

    // The authored PVS. Skipped for a ribbon build, whose geometry is generated
    // from the spline rather than authored, so the section boxes have nothing
    // to say about it.
    let visibility = if options.ribbon {
        None
    } else {
        TrackVisibility::build(&track_model, &track_blob, &ai)
    };
    match &visibility {
        Some(visibility) => report.push(format!(
            "{} authored visibility section(s); {} of {} draw call(s) governed by one \
             ({:.1}%), the rest always drawn; {} LOD-swap pair(s)",
            visibility.pvs.len(),
            visibility.placement.placed,
            visibility.placement.total(),
            visibility.placement.placed_fraction() * 100.0,
            visibility.swap_pairs(),
        )),
        None if options.ribbon => {}
        None => report.push(
            "no authored visibility sections: PVS culling is unavailable on this track".to_string(),
        ),
    }

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));

    let collision_model = if options.collision {
        let soup = render_collision::build_model(
            &label,
            &nodes,
            render_collision::Style::Wireframe,
            false,
        );
        report.push(format!(
            "drawing the collision soup: {} triangle(s) as outlines",
            soup.indices.len() / 3
        ));
        Some(soup)
    } else {
        None
    };

    // The player's own, and reported by `livery::load` beside the hull it was
    // read off. Sparks are slot 0's today - `oag_render::sparks` triggers off
    // the player's contacts alone - so this takes slot 0's locators rather than
    // carrying eight sets nothing reads.
    let collision_fx = liveries[0].collision_fx.clone();

    // One loop for all of them, and one report line each: adding an effect
    // is adding its name to `RACE_EFFECTS` and a trigger, never a loader.
    let mut effects = psys::Library::new();
    for name in RACE_EFFECTS {
        match particle_effect(&mut archives, name) {
            Ok((effect, note)) => {
                report.push(note);
                effects.insert(name, effect);
            }
            Err(why) => report.push(format!("{why} - {name} will not be drawn")),
        }
    }

    let noise = match mip_texture(&mut archives, NOISE_TEXTURE) {
        Ok((texture, note)) => {
            report.push(note);
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - the trail falls back to a procedural glow"));
            None
        }
    };

    let flare = match mip_texture(&mut archives, FLARE_TEXTURE) {
        Ok((texture, note)) => {
            report.push(note);
            Some(texture)
        }
        Err(why) => {
            // Reported rather than silently swapped for the placeholder. A
            // stand-in that looks plausible is how a decode failure survives
            // review; see the note on `FlareTexture::placeholder`.
            report.push(format!("{why} - the flare falls back to a procedural glow"));
            None
        }
    };

    let hud = load_hud(&mut archives, options.mode, &mut report);

    // The ring the lap counter runs on. Reported either way: "this track has no
    // lap counting" is exactly the kind of thing that otherwise gets discovered
    // as a HUD that never counts past one.
    let course = Course::from_track(&ai, start_position.as_ref().map(|s| Vec3::from(s.position)));
    match &course {
        Some(course) => report.push(format!(
            "course: {} points over {:.0} units, start line at point {}, path \
             boundaries at {:?}",
            course.len(),
            course.length(),
            course.start_index(),
            course.path_boundaries()
        )),
        None => report.push(
            "course: the spline's primary chain does not close, so this track has no lap \
             counting"
                .to_string(),
        ),
    }

    // Resolved here rather than in `Race::start` because the spline is what
    // supplies the attitude, and `load` is where the spline is.
    let pose_override = options.pose.and_then(|request| match request {
        PoseRequest::SplineAligned { position, yaw } => {
            let (_, sample, distance) = spline.nearest(position)?;
            report.push(format!(
                "pose override: {position:?} yaw {:.1} deg, attitude from the spline sample \
                 {distance:.1} units away",
                yaw.to_degrees()
            ));
            Some(Pose::from_position_on_sample(sample, position, yaw))
        }
        // Verbatim: the whole point of an exact pose is that nothing here
        // second-guesses the recorded basis against the spline.
        PoseRequest::Exact(pose) => {
            report.push(format!("pose override: exact, at {:?}", pose.position));
            Some(pose)
        }
    });

    Ok(Loaded {
        setup: Setup {
            mode: options.mode,
            difficulty: options.difficulty,
            class: options.class,
            opponents: options.opponents,
            seed: options.seed.unwrap_or(SEED),
            zone,
            ai,
            spline,
            course,
            start_position,
            collision,
            handling,
            airbrake_graphics,
            chase,
            chase_close,
            internal,
            nozzles: liveries.iter().map(|livery| livery.nozzle).collect(),
            collision_fx,
            effects,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_pad_refresh,
            class_gravity_scale,
            pose_override,
            camera_override: options.camera,
        },
        hud,
        track_model,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        fog_volumes,
        liveries,
        rocket_model,
        visibility,
        flare,
        noise,
        report,
    })
}

/// Which layout a mode draws its HUD from.
///
/// Time trial and speed lap share one: `TimeTrial_HUD.xml` carries both, which
/// is why `docs/ui/hud.md` counts five layouts for six modes and why the disc has
/// no `SpeedLap_HUD.xml`. Zone has its own.
///
/// `Arcade_HUD.xml` is the single race's, and it is the only shipped layout that
/// carries pickup widgets at all: `PickupBackground`, `SubWeapon` and one
/// `<Type>Icon` per weapon. `TimeTrial_HUD.xml` and `Zone_HUD.xml` carry none,
/// which is the disc agreeing from the presentation side with what
/// [`Mode::weapons_enabled`] reads out of the code. See
/// `docs/gameplay/pickups.md`.
///
/// The one layout this does not reach - `Elimination_HUD.xml` - is in
/// [`crate::hud::layouts`] waiting for the mode that uses it.
#[must_use]
pub const fn hud_layout(mode: Mode) -> &'static str {
    match mode {
        Mode::TimeTrial | Mode::SpeedLap => crate::hud::layouts::TIME_TRIAL,
        Mode::Zone => crate::hud::layouts::ZONE,
        Mode::SingleRace => crate::hud::layouts::ARCADE,
    }
}

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
fn load_hud(
    archives: &mut oag_assets::Archives,
    mode: Mode,
    report: &mut Vec<String>,
) -> crate::hud::Assets {
    let entry = hud_layout(mode);
    let layout = match archives
        .read_name(entry)
        .map_err(|e| e.to_string())
        // `text`, not `expand`: the PS2 ships these five layouts as plain
        // `<?xml` where the PSP shortens them, and reaching for `expand`
        // refused the PS2's outright and took the whole HUD with it.
        .and_then(|blob| oag_formats::fexml::text(&blob).map_err(|e| e.to_string()))
    {
        Ok(xml) => {
            let layout = crate::hud::Layout::from_xml(&xml);
            report.push(format!(
                "HUD {entry}: {} sprite(s), {} fill(s), {} label(s), {} model(s)",
                layout.sprites.len(),
                layout.fills.len(),
                layout.labels.len(),
                layout.models.len()
            ));
            for note in &layout.skipped {
                report.push(format!("HUD: skipped {note}"));
            }
            Some(layout)
        }
        Err(why) => {
            report.push(format!("HUD {entry} unavailable ({why}); no HUD"));
            None
        }
    };

    // One texture in the sheet, which is what `Sheet::build` is for. Going
    // through the sheet rather than binding the atlas directly means the HUD
    // shares the renderer every other screen uses, `Draw::Sprite` and all.
    let mut sheet = crate::sprite::Sheet::default();
    // **The layout names its own texture**; this used to name it instead. See
    // `crate::hud::Layout::atlas` for why that mattered - Pure's HUDs name none
    // at all, so a constant sent a Pure race looking for a Pulse file.
    match layout.as_ref().and_then(crate::hud::Layout::atlas) {
        None => report.push(format!(
            "HUD {entry} names no texture; its sprites are drawn from <Model> \
             geometry rather than an atlas"
        )),
        // `read_image`, not `read_name`: the PS2 keeps its atlas under an entry
        // its own XML's name does not hash to (see `oag_pulse::PS2_IMAGES`). That
        // substitution table is keyed on Pulse-PSP names and is inert on any other
        // source - a name it does not hold falls through to the original error -
        // so it stays here rather than becoming a per-title lookup.
        Some(atlas) => match oag_pulse::read_image(archives, atlas) {
            Ok(blob) => {
                let mut notes = Vec::new();
                let built = crate::sprite::Sheet::build(&[(atlas.to_string(), blob)], &mut notes);
                report.extend(notes);
                if built.get(atlas).is_some() {
                    report.push(format!(
                        "HUD atlas {atlas}: {}x{} sheet",
                        built.width, built.height
                    ));
                    sheet = built;
                } else {
                    report.push(format!("HUD atlas {atlas} did not decode"));
                }
            }
            Err(why) => report.push(format!("HUD atlas {atlas} unavailable ({why})")),
        },
    }

    // The HUD's captions are `idstring` keys - `IG_HUD_LAP`, `IG_HUD_BEST` - and
    // without a table `StringTable::get_or_id` falls back to the key itself, which
    // put `ig_hud_lap` on screen where `LAP` belongs. The preferred language is the
    // player's saved one; a race reached through `--race` has no settings to read,
    // so this takes the chain's default rather than threading one through.
    //
    // **Before the fonts now**, because the plugins parsed here are also what
    // name the two faces below - the same reordering `boot::load_shell` needed.
    let languages = crate::boot::load_languages(archives, report);
    let strings = crate::boot::load_strings(archives, &languages, None, report);

    // One role each, and no fallback to a second: both titles fill in both of
    // these slots, so a chain of alternatives would be an unexercised guess about
    // a disc nobody has. A source filling in neither draws in 5x7 and says so,
    // which is the honest answer to a question its data has not been asked.
    let font = hud_font(archives, &languages, roles::HUD, report);
    let small_font = hud_font(archives, &languages, roles::HUD_SMALL, report);

    crate::hud::Assets {
        layout,
        sheet,
        font,
        small_font,
        strings,
    }
}

/// Reads the `.fnt` this source's language plugins fill `role` in with.
///
/// The same shape as `crate::boot::load_font`, which reads the front end's
/// `Default` face, and now the same read *and* the same resolution: both take
/// the filename off a `<Font>` slot rather than naming one, and both go through
/// [`oag_assets::Archives::read_font`], so a PS2 source finds the glyph atlas
/// the disc keeps in the entry after the `.fnt` rather than falling back to 5x7.
/// Kept separate only so the report line says which font is being talked about.
///
/// A source whose plugins fill in no such slot draws in 5x7 and says which role
/// went unanswered - see [`crate::language::roles`] for what each disc fills in.
///
/// Both HUD fonts are pre-outlined on both Pulse pressings - six distinct greys,
/// alpha covering glyph *plus* border - so `Atlas::from_font`'s body/outline
/// split applies unchanged there; see `docs/formats/fnt.md`. Whether Pure's own
/// `HUDFont.fnt` is outlined the same way has not been measured.
fn hud_font(
    archives: &mut oag_assets::Archives,
    languages: &[crate::language::Language],
    role: &str,
    report: &mut Vec<String>,
) -> crate::font::Atlas {
    let Some(name) = languages
        .iter()
        .find_map(|language| language.font(role))
        .map(str::to_string)
    else {
        report.push(format!(
            "no language plugin names a {role:?} font on this source; drawing with 5x7"
        ));
        return crate::font::Atlas::build();
    };
    match archives.read_font(&name).map_err(|e| e.to_string()) {
        Ok(font) => {
            report.push(format!(
                "HUD font {name} (role {role:?}): {}x{} atlas, {} glyph(s), line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            crate::font::Atlas::from_font(&font)
        }
        Err(why) => {
            report.push(format!(
                "HUD font {name} (role {role:?}) unavailable ({why}); drawing with 5x7"
            ));
            crate::font::Atlas::build()
        }
    }
}

/// Says so when a model's texture slots did not all fill, and why.
///
/// **Keyed on empty slots, not on an empty list, and the difference is the whole
/// point.** A model that declares no texture slots at all wanted none - the
/// driveable ribbon `oag_render::track::build_model` generates is one, on either
/// disc - and saying "untextured" about it would blame a decode for geometry that
/// was never textured. What is worth reporting is a model that asked for `n`
/// textures and got fewer, because that is a picture missing something it was
/// authored with.
///
/// **PS2 ships are textured now; PS2 track art is the one place this still
/// fires, and it is reported rather than papered over.** A PS2 model's
/// textures are separate archive entries gathered into a nested WAD of
/// Graphics Synthesizer upload packets, and a `.vex` declares no texture
/// pixels of its own to say which. For ships, `ps2_texture_set` below finds
/// it anyway: the entry directly before a model in the archive's own
/// directory, checked independently against every team on the roster - see
/// `docs/formats/ps2-texture.md`. Track models are a separate, harder case:
/// several typically share one texture set instead of one each, and that
/// grouping is not verified against a known-correct picture the way a ship
/// is, so it is not attempted and `16_Track\track.vex` still draws
/// untextured on PS2.
///
/// The model still draws: `Drawable::draw` binds the white fallback for a draw
/// with no texture slot. So an unresolved model is a correctly-shaped
/// untextured one, and nothing here guesses at a texture set to avoid saying
/// that.
///
/// Deliberately keyed on the model rather than on the platform. A PSP model with a
/// slot that will not decode deserves the same line, and the PS2 disc carries
/// PSP-format batches too - so "which disc is this" is never the right question to
/// ask about one mesh.
/// Where a ship starts a race: the track's authored slot, or the spline.
///
/// The authored [`StartPosition`] wins whenever the track has one, because it is
/// a value recovered off the disc and `spline.start()` is an artifact of how this
/// crate resamples - sample 0 of path 0, which is wherever the exporter happened
/// to begin writing control points. On `16_Track` the two are **188.8 units
/// apart** and the authored slot is the one on the grid.
///
/// What this is **not** is pole. Every track ships one slot and a race grids
/// eight, so this places a ship on the one slot that was authored and says
/// nothing about the other seven. On `16_Track` a time trial in the original
/// starts about 138 units *ahead* of this, on the other side of the centreline -
/// which is what makes the remaining slots worth recovering rather than
/// deriving. See `docs/formats/track.md#start-position`.
///
/// The height comes off the collision geometry rather than out of the slot: see
/// [`Pose::from_start_position`] for the measurement that says the authored `y`
/// is not a ride height. `collision` is cast straight down from well above the
/// slot, and a slot over a hole in the mesh falls back to the authored value.
///
/// `None` only when a track has no authored slot *and* an empty spline, which no
/// real track is.
#[must_use]
fn spawn_pose(
    spline: &Spline,
    start_position: Option<&StartPosition>,
    collision: &CollisionWorld,
    handling: &Handling,
) -> Option<Pose> {
    let height = spawn_height(handling);
    if let Some(slot) = start_position {
        return Some(Pose::from_start_position(
            slot,
            ground_under(collision, slot),
            height,
        ));
    }
    spline
        .start()
        .map(|sample| Pose::from_sample(sample, sample.racing_line, height))
}

/// Every grid slot's pose, front to back, re-dropped onto the track under each.
///
/// `base` is slot 8 - the authored `Start Position` node, which
/// [`oag_gameplay::grid_pose`] establishes is the *back* of the grid. The other
/// seven are offsets from it, and each is dropped onto the collision surface
/// under its own footprint rather than inheriting slot 8's height: a grid is 138
/// units long and no track is flat over that, so sharing one `y` would leave the
/// front of the field buried or floating.
///
/// A slot with no surface under it keeps the offset height it was given, which is
/// the same fallback [`spawn_pose`] already takes when a track authors no slot.
fn grid_poses(base: Pose, collision: &CollisionWorld, height: f32) -> [Pose; GRID_SLOTS as usize] {
    core::array::from_fn(|index| {
        let slot = u8::try_from(index + 1).unwrap_or(GRID_SLOTS);
        let mut pose = oag_gameplay::grid_pose(base, slot);
        let up = pose.orientation * Vec3::Y;
        // `base` already carries `height` along its own up axis, so the drop has
        // to take it off before re-applying it under this slot - otherwise every
        // craft gains a ride height per slot.
        let foot = pose.position - up * height;
        let origin = foot + Vec3::Y * SPAWN_PROBE_RISE;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, SPAWN_PROBE_REACH);
        if let Some(hit) = oag_physics::Raycaster::raycast(collision, ray, None, false) {
            pose.position = Vec3::new(foot.x, hit.point.y, foot.z) + up * height;
        }
        pose
    })
}

/// How far up the track's own drop for a spawn probe starts, and how far it
/// reaches.
///
/// Generous either way on purpose: this is a one-off query at load, and a slot
/// authored a few units under an overhanging piece of track should still find the
/// floor rather than silently falling back.
const SPAWN_PROBE_RISE: f32 = 20.0;
const SPAWN_PROBE_REACH: f32 = 80.0;

/// The world `y` of the collision surface under an authored slot, if there is one.
#[must_use]
fn ground_under(collision: &CollisionWorld, slot: &StartPosition) -> Option<f32> {
    let origin = Vec3::from_array(slot.position) + Vec3::Y * SPAWN_PROBE_RISE;
    let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, SPAWN_PROBE_REACH);
    oag_physics::Raycaster::raycast(collision, ray, None, false).map(|hit| hit.point.y)
}

/// Reads a track `.vex`'s authored grid slot, if it has one.
///
/// `None` covers three cases a caller treats the same way and none of which is an
/// error: a blob that is not a `.vex` at all, one with no `Start Position` node,
/// and one whose node payload is not the 64 bytes a transform needs. A track with
/// no authored slot is a track a ship goes on the spline of.
#[must_use]
fn start_position_of(blob: &[u8]) -> Option<StartPosition> {
    let nodes = vex::nodes(blob).ok()?;
    // Version-keyed: unrecovered on version 4, and `None` there puts the ship on
    // the spline instead - which `load`'s own report line already describes.
    let class = vex::classes_of(blob).ok()?.start_position?;
    let node = nodes.iter().find(|node| node.class_id == class)?;
    oag_formats::track::start_position(blob.get(node.payload())?)
}

/// Why a class produced nothing: the file authors none, or nobody has recovered
/// its id for this format version.
///
/// **Two very different findings that look identical in a draw list**, and
/// collapsing them is how a decoding gap gets filed as authored content. A Pure
/// track really does author no `Speedup Pad` under version 6's `0x3bd` - but it
/// authors none under *any* id this project knows, because version 4's numbering
/// for that class has never been recovered, and saying "the track authors no
/// pads" would close a question that is still open. See
/// `docs/formats/pure-status.md`.
fn unrecovered_or_absent(
    classes: oag_formats::vex::classes::Classes,
    id: Option<u32>,
    what: &str,
    consequence: &str,
) -> String {
    match id {
        Some(_) => format!("the track authors no {what} geometry; {consequence}"),
        None => format!(
            "no {what} class id is recovered for .vex version {}, so this track was \
             never asked for any; {consequence}",
            classes.version
        ),
    }
}

#[must_use]
fn untextured_note(model: &Model) -> Option<String> {
    let slots = model.textures.len();
    let decoded = model.textures.iter().filter(|t| t.is_some()).count();
    if slots == 0 || decoded == slots {
        return None;
    }
    Some(format!(
        "{}: {decoded} of {slots} texture slot(s) decoded, drawing the rest \
         untextured. PS2 models keep their textures in separate archive entries, \
         found by directory position; a handful of circuits' texture sets are \
         short 1-2 slots on disc rather than unresolved by this lookup - see \
         docs/formats/ps2-texture.md",
        model.label
    ))
}

/// Converts the disc's `pos_length` into the one
/// [`ChaseParams::pos_length`] documents.
///
/// The two conventions differ by a sign, and this is the only place that is
/// reconciled.
///
/// `oag_render::camera::chase` documents `pos_length` as a **positive distance
/// behind** and computes the eye as `position - forward * pos_length`. The disc
/// stores a **signed offset along forward**: every `<ExternalCameraFar>` and
/// `<ExternalCameraClose>` block observed has it negative, and the *close* camera's
/// value is the smaller magnitude of the two, which is what "closer behind the
/// ship" looks like under that reading and is not consistent with any other.
///
/// Passing the file's value through unchanged puts the eye that far **in front** of
/// the ship, aimed at a point `lookat_length` further ahead still, so the ship is
/// behind the camera and nothing of it is drawn. `chase.rs` records that exact
/// mistake as "obvious on screen, so this one is cheap to check once there is a ship
/// to look at" - this is that check, and the negation is its answer.
///
/// Confidence **80**: the geometry admits no other reading and both external blocks
/// of every team agree, but nothing has been compared against the original running,
/// and the fix arguably belongs in `oag-render`'s documented convention rather than
/// here.
#[must_use]
fn chase_pos_length(from_disc: f32) -> f32 {
    -from_disc
}

/// One `<ExternalCamera*>` block as the renderer's [`ChaseParams`].
///
/// Both external views go through this, so [`chase_pos_length`] is applied to
/// exactly one of them never and to both of them always: a second transcription
/// of the seven fields is how the close view would end up with the far view's
/// sign convention.
///
/// # The rig carries the craft's global `0.75` scale, and does not pre-multiply it
///
/// `oag_physics::hover::TARGET_GLOBAL_SCALE` is a **world scale on craft-space
/// geometry**, not a suspension tuning knob, and the external camera rigs are one
/// of the places the original applies it. A camera reading a scale out of the
/// physics crate looks odd, which is why it is spelled out here:
///
/// - It is one global, `DAT_08ab0e1c`, set to a literal `0.75` in the craft
///   constructor (`FUN_08840c74`, the store at `0x08841000`). The **same** global
///   is read by `Ship_HoverFourCorner`'s four probe corners, by the `<Misc>` hull
///   dimensions on their way into the collider, by the initial body position, and
///   by both camera rigs in `Ship_UpdateCameraRigs` - which end with
///   `eye = shipPos + (eye - shipPos) * 0.75` and the same line for the look-at
///   point. Confidence **85**.
/// - `fov` and the two springs are **not** scaled: they are not lengths.
///
/// **It goes in [`ChaseParams::craft_scale`] rather than into the four offsets,
/// and that is a correction, not a preference.** Scaling the eye-and-look-at pair
/// about the ship and scaling the four offsets are the same thing for the aim
/// point, which is rigid, and are **not** the same thing for the eye, which is
/// sprung: the original scales *after* writing its spring state back, so the
/// state it keeps is unscaled, while a pre-multiplied offset set keeps a scaled
/// one - and the ship those two are measured from has moved in between. Over the
/// 150 ticks of `data/traces/pad0-boost.csv` that is `0.121` RMS against `0.008`.
/// See `crates/game/tests/chase_camera_ground_truth.rs`.
///
/// Three independent numbers agree that the scale belongs here at all. The
/// authored close block times `0.75` is `11.643` units from the craft; a static
/// probe of the original in that view measured `11.643`; and a 150-tick capture
/// of the original measured `11.571`-`11.674` across 40-154 units/s. The far
/// block lands on `14.562` against a measured `14.562`. Before this scale existed
/// here the eye sat `15.5` and `19.4` units out, which is the framing difference
/// `docs/ghidra/functions/psp-pulse-usa/camera.md` recorded as an unexplained
/// factor of three quarters and this closes.
///
/// **`<InternalCamera>` is deliberately *not* scaled** - see
/// `oag_render::camera::internal`. The original's internal rig reads this global
/// nowhere, and the cockpit eye was measured at `+3.000` against an authored
/// `length` of `3`. Counterintuitive next to the two external blocks, and
/// measured twice, so it stays.
///
/// `pub` so that `crates/game/tests/chase_camera_ground_truth.rs` compares the
/// **shipped** conversion against the original's own camera rather than a second
/// transcription of it - which would pass while the game rendered something
/// else.
#[must_use]
pub fn chase_params(block: handling::ExternalCamera) -> ChaseParams {
    ChaseParams {
        fov: block.fov,
        lookat_height: block.lookat_height,
        lookat_length: block.lookat_length,
        pos_height: block.pos_height,
        pos_length: chase_pos_length(block.pos_length),
        spring_horiz: block.spring_horiz,
        spring_vert: block.spring_vert,
        craft_scale: oag_physics::hover::TARGET_GLOBAL_SCALE,
    }
}

/// Placing a ship on a track lives in [`oag_gameplay::spawn`], which is where a
/// harness that is not the composition root can reach it: `oag-trace drive` puts
/// a ship on a start line exactly the way a race does and may not depend on this
/// crate. Re-exported so every call site here, and both ground-truth tests, read
/// as they did.
pub use oag_gameplay::spawn::{box_inertia, spawn_height};

/// Advances every active craft's standing, and syncs the player's lap from it.
///
/// Slot order, and slot order only: this feeds the finishing order, which is
/// simulation state. See `docs/architecture/determinism.md`.
///
/// `laps_target` comes from the player's race because it is the *mode's* number
/// rather than a craft's - all eight are running the same event.
fn advance_standings(world: &mut World, course: &Course) {
    let tick = world.tick;
    let target = world.race.laps_target;
    for slot in 0..world.ship_count as usize {
        let ship = &mut world.ships[slot];
        if !ship.active {
            continue;
        }
        let position = ship.physics.body.position;
        ship.standing.update(course, position, tick, target);
    }
    // One lap rule, not two: the player's displayed lap is the standing's.
    // `RaceState` keeps the clock, the best lap, the Zone counters and the finish
    // condition, all of which are the player's alone.
    world.race.lap = world.ships[0].standing.lap;
}

/// The authored racing line and the corridor around it, one entry per spline
/// sample.
///
/// Five things the disc carries and this reads: the sample position, the
/// `HOVER_LIFT` the loader applies to it, `racing_line` - the lateral offset the
/// artists authored, in the sample's own `lateral` axis - and the two bounds
/// `ai_bound_left` and `ai_bound_right` that fence the AI corridor in the same
/// axis. See `docs/formats/track.md`.
///
/// **The bounds are rebased onto the line.** The disc stores all three as
/// offsets from the sample's own centre; a driver only ever asks how far it may
/// stray from the line it is driving, so what it gets is the difference. Left
/// comes out at most zero and right at least zero, which the shipped data
/// guarantees - the loader clamps the bounds to straddle the racing line by at
/// least 0.1 and all 34,261 control points already satisfy it.
///
/// The corridor is what an opponent's [`oag_ai::Personality`] spends on not
/// driving the ideal line, so how far the field spreads is the track's own
/// property and narrows where the artists narrowed it. See
/// `docs/gameplay/ai.md`.
///
/// Index-parallel to [`ai_order`] on purpose - see [`Race::racing_line`].
#[must_use]
fn racing_line(spline: &Spline, order: &[u32]) -> oag_ai::Line {
    let samples: Vec<_> = order
        .iter()
        .filter_map(|&index| spline.sample(index as usize))
        .collect();
    let points = samples
        .iter()
        .map(|sample| {
            let down = Vec3::from_array(sample.down);
            let lateral = Vec3::from_array(sample.lateral);
            Vec3::from_array(sample.pos) - oag_formats::track::HOVER_LIFT * down
                + sample.racing_line * lateral
        })
        .collect();
    let corridor = samples
        .iter()
        .map(|sample| oag_ai::Frame {
            // Already unit length: checked on all 34,261 control points, and
            // the B-spline blend of four unit vectors is not, which is why
            // this renormalises rather than trusting the sample.
            lateral: Vec3::from_array(sample.lateral).normalize_or_zero(),
            left: (sample.ai_bound_left - sample.racing_line).min(0.0),
            right: (sample.ai_bound_right - sample.racing_line).max(0.0),
        })
        .collect();
    oag_ai::Line::with_corridor(points, corridor)
}

/// Which spline samples the AI line is made of, in the order a lap drives them.
///
/// **The one place the two index spaces are related**, and the reason there are
/// two. [`Spline`] is every path the track authors, concatenated in *file*
/// order, because the hover hold and the culler have to be able to locate a
/// craft wherever it physically is - including on the branch of a split that
/// this lap does not use. A driver wants the opposite: the circuit, once round,
/// with nothing in it that a lap does not drive.
///
/// On nine of the disc's twelve circuits those are the same list and this is the
/// identity permutation. On `05_Track`, `14_Track` and `07_Track` the file holds
/// three paths and the lap ring walks two; the third is the other side of a
/// split, and file order dropped a thousand samples of it into the middle of the
/// lap, pointing the wrong way. A driver reaching the end of path 0 found its
/// next samples a kilometre away, its 48-sample window could not follow, and the
/// index stuck - those three circuits were exactly the three a lone craft never
/// got a second lap out of. See `docs/gameplay/ai.md`.
///
/// Falls back to the whole table in file order when the track has no closed
/// ring. A driveable ribbon with no lap counter still wants a line, and "every
/// sample there is" is the honest answer when nothing knows which way round is
/// forward.
#[must_use]
fn ai_order(spline: &Spline, course: Option<&Course>) -> Vec<u32> {
    let every = || (0..spline.len() as u32).collect::<Vec<_>>();
    let Some(course) = course else {
        return every();
    };
    let mut order = Vec::with_capacity(spline.len());
    for path in course.path_order() {
        // The samples of one path, which the table holds contiguously. Scanning
        // rather than assuming the two crates resample identically: `Course` and
        // `Spline` each carry their own `STEPS_PER_SEGMENT` and a mapping built
        // on them being equal would break silently if either moved.
        order.extend(
            (0..spline.len() as u32).filter(|&index| spline.path_of(index as usize) == Some(path)),
        );
    }
    if order.is_empty() { every() } else { order }
}

/// The spline resampled into a flat table, for locating a ship on the track.
///
/// A `Vec` walked in order rather than any kind of spatial index: the nearest-sample
/// comparison feeds simulation state, so the order it happens in must not be able to
/// vary between runs, and ties go to the earlier sample. See
/// `docs/architecture/determinism.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    samples: Vec<Sample>,
    /// Which path each sample came from, parallel to [`Self::samples`].
    paths: Vec<u16>,
}

impl Spline {
    /// Samples per control-point interval.
    ///
    /// The same four `oag_render::track` draws the ribbon with, so the table and the
    /// picture agree about where the track is.
    pub const STEPS_PER_SEGMENT: usize = 4;

    /// Resamples every path of a decoded spline graph.
    #[must_use]
    pub fn from_track(ai: &AiTrack) -> Self {
        let mut samples = Vec::new();
        let mut paths = Vec::new();
        for (index, path) in ai.paths.iter().enumerate() {
            for segment in 0..path.points.len() {
                for step in 0..Self::STEPS_PER_SEGMENT {
                    let t = step as f32 / Self::STEPS_PER_SEGMENT as f32;
                    if let Some(sample) = path.sample(segment, t) {
                        samples.push(sample);
                        paths.push(index as u16);
                    }
                }
            }
        }
        Self { samples, paths }
    }

    /// How many samples the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the track produced no samples at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The first sample of the first path: where a ship is put.
    ///
    /// Not a grid slot. How the original assigns one is an open question that
    /// `oag_gameplay::spawn` deliberately leaves alone, so this is the start of the
    /// track and claims nothing more than that.
    #[must_use]
    pub fn start(&self) -> Option<&Sample> {
        self.samples.first()
    }

    /// The sample as [`oag_physics::maglock`] wants it: **lifted**, with its axis.
    ///
    /// Two conversions happen here and both are the format side's business rather
    /// than the simulation's:
    ///
    /// - `Sample::pos` is the *unlifted* disc value and the running game holds the
    ///   lifted one, because `AiTrack_LoadPathPoints` does `pos -= 3.0 * down` at
    ///   load. The hold subtracts that same lift back off to recover the surface
    ///   point, so it has to be handed the lifted form or it lands three units low.
    ///   Blending is linear, so lifting an interpolated sample and interpolating
    ///   lifted points are the same thing.
    /// - `down` is passed **raw**. The hold normalises it where it needs an axis
    ///   and reads it unnormalised where the original does, which is the mag
    ///   probe's direction.
    #[must_use]
    pub fn track_sample(sample: &Sample) -> oag_physics::TrackSample {
        let down = Vec3::from_array(sample.down);
        oag_physics::TrackSample {
            position: Vec3::from_array(sample.pos) - down * oag_formats::track::HOVER_LIFT,
            down,
        }
    }

    /// The nearest sample to `position`: its index, itself, and its distance.
    #[must_use]
    pub fn nearest(&self, position: Vec3) -> Option<(usize, &Sample, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, sample) in self.samples.iter().enumerate() {
            let distance = (Vec3::from_array(sample.pos) - position).length();
            // Strictly nearer, so a tie keeps the earlier sample.
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// The nearest sample to `position`, searching only `window` samples either
    /// side of `around` in table order.
    ///
    /// **A local search whose failure mode is safe.** [`Self::nearest`] walks
    /// every sample - ~3,400 on a real track - which is affordable once a tick
    /// inside the simulation and not twice more per *frame* on top. The camera
    /// is a chase spring a few units behind the craft, so it is a few samples
    /// away in this table, and a window finds it.
    ///
    /// When it does not - the window straddles a junction, or the camera really
    /// has left the track - the answer is a sample that is too far away, which
    /// the caller turns into [`UNPLACED`] and therefore into *draw everything*.
    /// A local search that misses costs a frame of culling, never a frame of
    /// missing geometry, which is why this is allowed to be approximate.
    #[must_use]
    pub fn nearest_within(
        &self,
        position: Vec3,
        around: usize,
        window: usize,
    ) -> Option<(usize, &Sample, f32)> {
        let last = self.samples.len().checked_sub(1)?;
        let from = around.saturating_sub(window);
        let to = around.saturating_add(window).min(last);
        let mut best: Option<(usize, f32)> = None;
        for index in from..=to {
            let distance = (Vec3::from_array(self.samples[index].pos) - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// Distance from `position` to the nearest sample, or `None` on an empty track.
    ///
    /// Distance to a *sample*, not to the curve: at four samples per segment the two
    /// differ by a fraction of a control-point interval, far below anything worth
    /// asserting on.
    #[must_use]
    pub fn distance_to(&self, position: Vec3) -> Option<f32> {
        self.nearest(position).map(|(_, _, distance)| distance)
    }

    /// The sample at `index`, in table order.
    #[must_use]
    pub fn sample(&self, index: usize) -> Option<&Sample> {
        self.samples.get(index)
    }

    /// Which path the sample at `index` belongs to.
    #[must_use]
    pub fn path_of(&self, index: usize) -> Option<u16> {
        self.paths.get(index).copied()
    }

    /// The widest half-width anywhere on the track, either side.
    ///
    /// A scale for "still roughly on the track" that comes from the track itself
    /// rather than from a number somebody picked.
    #[must_use]
    pub fn max_half_width(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |widest, sample| {
            widest
                .max(sample.half_width_left)
                .max(sample.half_width_right)
        })
    }
}

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

impl Race {
    /// Starts a race: one ship, on the racing line, at the start of the spline.
    ///
    /// The mass is copied into the rigid body because the force law reads
    /// `handling.physical.mass` while the integrator reads `body.mass`. They are one
    /// quantity stored twice and keeping them equal is this layer's job -
    /// `oag_physics` deliberately does not do it, so that a mismatch stays visible.
    ///
    /// The inertia comes from [`box_inertia`]; see the module documentation and that
    /// function for why a derived tensor replaced the `(1, 1, 1)` placeholder.
    #[must_use]
    pub fn start(setup: Setup) -> Self {
        let Setup {
            mode,
            difficulty,
            opponents,
            seed,
            zone,
            spline,
            course,
            start_position,
            collision,
            handling,
            chase,
            chase_close,
            internal,
            nozzles,
            collision_fx,
            effects,
            speedup_pads,
            weapon_pads,
            weapons,
            weapon_pad_refresh,
            class,
            class_gravity_scale,
            pose_override,
            camera_override,
            ..
        } = setup;

        // The mode gate, applied once at construction rather than on every tick,
        // because that is where the original applies it: `World_CollectNodeLists`
        // zeroes the trigger list's own count at track load, so a weapons-off
        // race has no weapon pads to walk rather than a walk that decides
        // nothing. Dropping the volumes here also makes it impossible for a
        // later change to reach them by accident.
        let weapon_pads = if mode.weapons_enabled() {
            weapon_pads
        } else {
            Vec::new()
        };

        let mut world = World::new(seed);
        world.race = RaceState::new(mode);
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = box_inertia();
        // The pool starts full, which is `Ship_ResetShield` (`0x0883dd24`) - the
        // only thing on the disc that sets it outright. Wall contact spends it
        // from here (`oag_physics::damage`), and Zone's perfect-zone recharge
        // adds back to it.
        oag_physics::damage::reset(&mut ship.physics, &handling.dimensions);
        // The override wins outright rather than being an offset from the grid
        // slot: it exists to put the craft at a position read off somewhere
        // else, and anything added to that would make the two disagree.
        let base = spawn_pose(&spline, start_position.as_ref(), &collision, &handling);
        if let Some(pose) = pose_override.or(base) {
            ship.place_at(pose);
        }

        // The rest of the grid. **The player keeps slot 0 of the array and slot
        // 8 of the grid**, which is not a coincidence: the array index is what
        // every rule, camera and HUD in this file means by "the ship", and slot
        // 8 is where the original puts the local player and where the authored
        // `Start Position` node is. So the seven opponents are ahead of the
        // player, which is what a Pulse grid looks like.
        //
        // Each one gets a driver here and its own `Environment`, standing and
        // exhaust as the race runs - see [`Self::step_opponents`],
        // [`Self::update_standings`] and [`Self::advance_exhausts`].
        //
        // **Gated on the mode first of all.** `mode.has_opponents()` is `false`
        // for a time trial, a speed lap and a Zone race - measured live,
        // confirmed on the running original, AI DIFFICULTY greyed to N/A the
        // same way WEAPONS is - so those three spawn the player alone, the way
        // the original does, and a single race fields the grid. `opponents` is
        // the verification override [`Setup::opponents`] documents.
        //
        // Skipped entirely when the caller overrode the pose: an override exists
        // to put *one* craft somewhere specific, and surrounding it with a grid
        // built from a different anchor would put opponents through the scenery.
        //
        // Also skipped when the track authors no `Start Position`. The whole
        // layout is offsets from that node, which is slot 8; anchored on the
        // spline fallback instead, the offsets would be measured from a pose
        // nobody authored and the seven opponents would be a guess wearing the
        // shape of a measurement.
        let mut ai_pilots = [oag_ai::Pilot::BALANCED; oag_gameplay::MAX_SHIPS];
        let order = ai_order(&spline, course.as_ref());
        let line = racing_line(&spline, &order);
        let rescue_distance = spline.max_half_width() * RESCUE_HALF_WIDTHS;
        let mut pilot_names: [String; oag_gameplay::MAX_SHIPS] = Default::default();
        // The built-ins, plus whatever the player has authored. A directory
        // that cannot be read is not a reason to refuse to race: the built-ins
        // are always there, and the error is reported rather than fatal.
        let roster = match crate::pilots::load() {
            Ok(roster) => roster,
            Err(e) => {
                eprintln!("pilots: {e:#} - racing with the built-in four");
                crate::pilots::Roster::built_in()
            }
        };
        if pose_override.is_none()
            && start_position.is_some()
            && (mode.has_opponents() || opponents)
            && let Some(base) = base
        {
            let poses = grid_poses(base, &collision, spawn_height(&handling));
            for (index, pose) in poses.iter().enumerate().take(GRID_SLOTS as usize - 1) {
                let opponent = &mut world.ships[index + 1];
                opponent.active = true;
                opponent.handling = handling;
                opponent.physics.body.mass = handling.physical.mass;
                opponent.physics.body.inertia = box_inertia();
                oag_physics::damage::reset(&mut opponent.physics, &handling.dimensions);
                // **Eight drivers, not one driver eight times.** The seed is
                // the race's own and the craft's slot, so the same race fields
                // the same eight characters on every replay and the next race
                // fields eight others. Nothing here reads a clock or the
                // world's generator - see `oag_ai::Driver::for_slot`.
                let slot = index as u32 + 1;
                opponent.driver = oag_ai::Driver::for_slot(seed, slot);
                // **Where on the line this craft actually is**, found once with
                // a search over the whole line rather than left at zero.
                //
                // `Driver::drive` searches a 48-sample *window* around the last
                // index, deliberately, so a track that passes near itself
                // cannot make a craft latch onto a stacked section. That window
                // is also why the starting value matters: a driver that begins
                // at sample zero when the grid is at sample 2,500 never finds
                // itself, and steers at the piece of circuit it thinks it is
                // on. Measured before this line existed - on eight of the
                // disc's twelve circuits the field sat between 280 and 10,862
                // units from its own racing line, and only the two whose start
                // line happens to sit near sample zero worked.
                opponent.driver.index = line.nearest(pose.position, 0, line.len()) as u32;
                // **And the character it is a variation on**, drawn from the
                // race seed through a stream of its own. Sharing the driver
                // seed's would tie a craft's pilot to its personality, so the
                // aggressive slot would always be the one that also drew a high
                // commitment - see `oag_ai::pilot_for_slot`.
                let choice = oag_ai::pilot_for_slot(seed, slot, roster.len());
                let entry = &roster.entries()[choice as usize];
                // The level scales how a pilot treats other craft and leaves
                // the line it drives alone, so a shy pilot at novice is still
                // shy rather than generically slow.
                ai_pilots[slot as usize] = difficulty.temper(&entry.pilot);
                pilot_names[slot as usize].clone_from(&entry.name);
                // **The digest, into the world snapshot.** A pilot can come out
                // of the player's own config directory, so two machines running
                // "the same race" with different files must not agree on the
                // hash - see `oag_ai::Driver::pilot`.
                opponent.driver.pilot = entry.digest;
                opponent.place_at(*pose);
            }
            world.ship_count = GRID_SLOTS;
            // **What was loaded, and who is flying what, once per race.** The
            // digests are the point of the first line: thirty-two bits cannot
            // be inverted, so when two machines disagree on a world hash this
            // is the only thing that says *which* pilot file differs. The
            // second line is for the player, who wants to know the grid they
            // are about to race. A `*` marks a pilot that came from a file.
            eprintln!("pilots loaded: {}", roster.summary());
            eprintln!("ai difficulty: {}", difficulty.name());
            let grid: Vec<String> = (1..GRID_SLOTS as usize)
                .map(|slot| format!("{slot}:{}", pilot_names[slot]))
                .collect();
            eprintln!("pilots on the grid: {}", grid.join(" "));
        } else {
            world.ship_count = 1;
        }

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        Self {
            racing_line: line,
            ai_order: order,
            // **Degraded from the measured tuning**, never boosted toward it -
            // see `oag_ai::Difficulty`. At the top level this is the
            // measurement unchanged.
            ai_tuning: difficulty.tune(&oag_ai::Tuning::default()),
            ai_pilots,
            world,
            collision,
            spline,
            course,
            // Only Zone reads these, so the other two modes carry `None` and the
            // engine keeps its ordinary throttle path.
            zone: zone.filter(|_| mode == Mode::Zone),
            chase_params: chase,
            chase_far: chase,
            chase_close,
            internal_params: internal,
            // The default rather than the settings file's value, for the reason
            // `set_boost_fov_kick` gives: `Setup` is what a *headless* race
            // needs and a display preference must not be in front of a caller
            // with no screen. `set_camera_view` is what applies the player's.
            view: crate::display::CameraView::default(),
            camera,
            camera_override,
            // ADR-0007: 60 Hz, from the clock rather than from a literal, so there
            // is one place the rate is decided.
            dt: TickClock::new(TickRate::DEFAULT).rate().dt(),
            respawn_cooldown: [0; oag_gameplay::MAX_SHIPS],
            respawns_in_a_row: [0; oag_gameplay::MAX_SHIPS],
            respawn_disabled: [false; oag_gameplay::MAX_SHIPS],
            respawns: [0; oag_gameplay::MAX_SHIPS],
            lost_ticks: [0; oag_gameplay::MAX_SHIPS],
            rescue_distance,
            // Cold, then snapped on the first tick. A race starts from a standing
            // start with no thrust, so there is nothing to snap *to* here.
            //
            // A cold `Exhaust` has an empty trail ring and `Exhaust::trail_ready`
            // gates the ribbon on a *full* one, so no craft draws a ribbon across
            // the track from the origin to its grid slot on the opening ticks.
            exhaust: [Exhaust::new(); MAX_SHIPS],
            exhaust_rng: std::array::from_fn(|slot| Rng::new(exhaust_seed(slot))),
            nozzles,
            collision_fx,
            sparks: psys::System::new(),
            effects,
            stage: psys::Stage::new(),
            engine_flare: [None; MAX_SHIPS],
            projectile_flare: [None; oag_gameplay::projectile::MAX_PROJECTILES],
            stage_rng: Rng::new(STAGE_SEED),
            sparks_ignitions: 0,
            sparks_rng: Rng::new(SPARKS_SEED),
            // No sync frame to be mid-scrape on, so the first tick's contact -
            // if any - is always read as a fresh impact.
            sparks_cooldown: 0.0,
            sparks_anchor: None,
            // Every pad starts due for a real test. `Pad_Bind` zeroes the same
            // cache at load, so the first tick measures rather than trusting a
            // distance nothing has computed yet.
            pad_distance: std::array::from_fn(|_| vec![0.0; speedup_pads.len()]),
            speedup_pads,
            // The same broadphase, and the same "due for a real test" start.
            weapon_pad_distance: std::array::from_fn(|_| vec![0.0; weapon_pads.len()]),
            // Every pad starts collectable. The original's `+0x1a0` is zero
            // until something stamps it, and nothing stamps it at load.
            weapon_pad_refresh_left: vec![0.0; weapon_pads.len()],
            weapon_pads,
            weapon_pad_current: [None; MAX_SHIPS],
            weapon_pad_refresh,
            weapons,
            class,
            class_gravity_scale,
            pad_current: [None; MAX_SHIPS],
            pad_previous_position: [None; MAX_SHIPS],
            boost_kick: 0.0,
            boost_fov_kick: crate::display::BoostFovKick::DEFAULT,
            flaps: [0.0, 0.0],
            flap_graphics: setup.airbrake_graphics,
            scheme: ControlScheme::default(),
        }
    }

    /// Chooses which control scheme maps the pilot's buttons.
    ///
    /// A setter for the same reason `set_boost_fov_kick` is one: it is a
    /// preference read from the settings file, and `Setup` is what a *headless*
    /// race needs. Unlike the kick it changes what the simulation sees, so it is
    /// set once before the first tick and not touched again - a scheme swapped
    /// mid-race would leave a half-finished gesture armed in `ShipState`.
    pub fn set_control_scheme(&mut self, scheme: ControlScheme) {
        self.scheme = scheme;
    }

    /// How far each airbrake flap has swung, left then right, in **radians**.
    ///
    /// Read by the renderer once a frame. Render-only state - see
    /// [`Self::flaps`], which holds the same thing on the airbrake's `0..=100`
    /// scale, and note the rates driving it are **not** the force law's.
    #[must_use]
    pub fn airbrake_flaps(&self) -> [f32; 2] {
        self.flaps
            .map(|level| self.flap_graphics.amount * level / 100.0)
    }

    /// Which scheme this race is being driven with.
    #[must_use]
    pub fn control_scheme(&self) -> ControlScheme {
        self.scheme
    }

    /// Sets how strong the boost's field-of-view kick is. `[graphics]
    /// boost_fov_kick`.
    ///
    /// A setter rather than a [`Setup`] field because the kick is a display
    /// choice and `Setup` is what a headless race needs; threading a graphics
    /// setting through the loader would put it in front of every caller that has
    /// no screen. [`crate::display::BoostFovKick::OFF`] leaves [`Race::projection`]
    /// bit-identical to what it returned before the effect existed.
    pub fn set_boost_fov_kick(&mut self, kick: crate::display::BoostFovKick) {
        self.boost_fov_kick = kick;
        if kick == crate::display::BoostFovKick::OFF {
            self.boost_kick = 0.0;
        }
    }

    /// What this race was actually built with, for the `BOOST FOV KICK` row's
    /// restart note - see `Self::set_boost_fov_kick`, which is called once at
    /// `Race::start` and does not track the settings file afterwards.
    #[must_use]
    pub fn boost_fov_kick(&self) -> crate::display::BoostFovKick {
        self.boost_fov_kick
    }

    /// Chooses which of the three perspectives to render from. `[graphics]
    /// camera_view`.
    ///
    /// A setter for the same reason `set_boost_fov_kick` is one - and unlike that
    /// one this is called **during** a race as well as before it, because the
    /// original binds the choice to a button. Everything it touches is
    /// render-only, so calling it mid-race cannot change what the simulation does
    /// next; that is the property
    /// `tests::cycling_the_camera_changes_no_simulation_state` pins.
    ///
    /// **The chase eye is snapped on a change between the two external views**,
    /// not sprung across. They share one eye and their anchors are metres apart,
    /// so springing would sweep the camera through the world for a second - the
    /// same reason [`Chase::snapped`] exists for a race start and a respawn.
    /// Whether the original springs or snaps between them is **not recovered**:
    /// it keeps a separate previous-eye per rig, so it does neither, and
    /// reproducing that needs a second [`Chase`] rather than a decision here.
    pub fn set_camera_view(&mut self, view: crate::display::CameraView) {
        if view == self.view {
            return;
        }
        self.view = view;
        self.chase_params = match view {
            crate::display::CameraView::Close => self.chase_close,
            // The cockpit view does not use these, but leaving the *far* block
            // installed means a cycle back out of the cockpit lands on the block
            // the next external view will want anyway.
            crate::display::CameraView::Internal | crate::display::CameraView::Far => {
                self.chase_far
            }
        };
        self.camera = Chase::snapped(target_of(self.ship()), &self.chase_params);
    }

    /// Which perspective this race is rendering from.
    #[must_use]
    pub fn camera_view(&self) -> crate::display::CameraView {
        self.view
    }

    /// Whether the player's own hull should be drawn this frame.
    ///
    /// False in the cockpit view alone. Read by [`Scene::render`], which skips
    /// the draw call rather than moving the model: see
    /// [`crate::display::CameraView::draws_own_ship`] for the evidence and its
    /// confidence.
    ///
    /// **The exhaust plume, the boost plume and the collision sparks are still
    /// drawn.** The recovered flag covers the hull and nothing else, so the
    /// alternative would be inventing three more consequences for it; and from a
    /// cockpit at the nose the nozzle is behind the eye, so the plume is normally
    /// off screen anyway rather than visibly wrong.
    #[must_use]
    pub fn draws_own_ship(&self) -> bool {
        self.view.draws_own_ship()
    }

    /// How many times a `Reset` contact has respawned the player this race.
    #[must_use]
    pub fn respawns(&self) -> u32 {
        self.respawns[0]
    }

    /// The same, for any craft on the grid.
    ///
    /// Out of bounds reads zero rather than panicking: a caller sweeping
    /// [`oag_gameplay::MAX_SHIPS`] slots on a six-craft grid is asking a fair
    /// question and the answer is "none".
    #[must_use]
    pub fn respawns_of(&self, slot: usize) -> u32 {
        self.respawns.get(slot).copied().unwrap_or(0)
    }

    /// The fixed timestep, from [`TickRate::DEFAULT`].
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// The player's ship.
    #[must_use]
    pub fn ship(&self) -> &Ship {
        &self.world.ships[0]
    }

    /// What the player's craft is carrying, if anything.
    ///
    /// A convenience over `ship().pickup.weapon`, and the accessor the HUD
    /// reads: which pickup is held is presentation-facing in a way the rest of
    /// [`Ship`] is not.
    #[must_use]
    pub fn ship_pickup(&self) -> Option<oag_formats::weapons::Weapon> {
        self.world.ships[0].pickup.weapon
    }

    /// The resampled spline, for a caller that wants to measure against it.
    #[must_use]
    pub fn spline(&self) -> &Spline {
        &self.spline
    }

    /// Replaces what the opponents are flown with, mid-race.
    ///
    /// **For measurement, and it is the only way to sweep a knob across a
    /// circuit.** The tuning is otherwise decided once at
    /// [`Self::start`] from the difficulty, which is right for a race and
    /// useless for finding out what a number is worth: a sweep that had to
    /// recompile between points could not be a test, and a tuning fitted on the
    /// one circuit that happens to be the default is how this page got a grip
    /// figure that was wrong on the other eleven.
    ///
    /// It is not a difficulty setting and nothing in the game calls it.
    pub fn set_ai_tuning(&mut self, tuning: oag_ai::Tuning) {
        self.ai_tuning = tuning;
    }

    /// The line the opponents' drivers follow, index-parallel to
    /// [`Self::ai_order`] rather than to [`Self::spline`].
    ///
    /// Read-only, and it exists so a test can ask the question that matters
    /// when an opponent misbehaves: *is the craft where its driver believes it
    /// is?* Comparing a ship's position against `point(driver.index)` separates
    /// a driver that has lost its place on the line from one that is simply
    /// driving badly, and those two have nothing in common as bugs.
    #[must_use]
    pub fn racing_line(&self) -> &oag_ai::Line {
        &self.racing_line
    }

    /// The spline sample a driver's index stands on.
    ///
    /// **`driver.index` is an index into the racing line, not into the sample
    /// table**, and on the three circuits where those differ using one as the
    /// other reads a sample from the wrong side of the track. Wraps, because the
    /// line is a closed ring and the caller that wants the *next* sample is
    /// asking a lap question rather than a table question - `ai_order[i + 1]`,
    /// never `ai_order[i] + 1`, which at the end of a path is a different place
    /// entirely.
    #[must_use]
    pub fn ai_sample(&self, ai_index: usize) -> Option<&Sample> {
        self.sample_index_of(ai_index)
            .and_then(|index| self.spline.sample(index))
    }

    /// The same lookup, as an index into [`Self::spline`].
    ///
    /// For the one caller that needs the number rather than the sample:
    /// [`Self::respawn`] takes a *sample* index, because the player reaches it
    /// from `Spline::nearest` and an opponent from its driver, and a single
    /// parameter carrying two index spaces depending on the caller is the bug
    /// this pair of accessors exists to make impossible to write.
    #[must_use]
    fn sample_index_of(&self, ai_index: usize) -> Option<usize> {
        if self.ai_order.is_empty() {
            return None;
        }
        Some(self.ai_order[ai_index % self.ai_order.len()] as usize)
    }

    /// Drives and steps every craft that is not the player's.
    ///
    /// Slot order, and only slot order: iteration here feeds simulation state, so
    /// it must be something that cannot vary between runs. See
    /// `docs/architecture/determinism.md`.
    ///
    /// Each craft gets **its own** `Environment`, built from the sample its own
    /// driver is standing on. That is what the old "they do not move" comment
    /// meant by an opponent stepped with the player's environment reading the
    /// player's track sample: the hover probes and the magstrip hold both take
    /// their surface from it, so sharing one would fly seven craft against the
    /// player's piece of track.
    ///
    /// **What an opponent does not get yet**, each deliberate and each a
    /// separate piece of work: a lap *time* of its own ([`oag_race::RaceState`]
    /// is the player's clock and only [`oag_race::Standing`] is per craft), a
    /// respawn when it falls off, a target for anything it fires, and a livery of
    /// its own. It does get pads of both classes, a standing, an `Exhaust` and a
    /// pickup it can spend - see [`Self::spend_opponent_pickup`].
    fn step_opponents(&mut self) {
        let damage_rules = oag_gameplay::damage_rules(self.world.race.mode);
        // Once for the whole grid: every craft's view needs the same ordering.
        let places = self.places();
        for slot in 1..self.world.ship_count as usize {
            if !self.world.ships[slot].active {
                continue;
            }

            // **A wrecked opponent stops driving, and is still stepped.** A single
            // race runs with `Damage` on - see `oag_gameplay::damage_rules` - so
            // an opponent grinding a wall empties its pool exactly as the player
            // does, and `oag_physics::step` will keep integrating whatever it is
            // handed. Left driving, it would be a wreck holding full throttle
            // round the circuit, which is worse than the parked hulls this
            // replaced. Released rather than skipped, because
            // `damage::advance_state` runs *inside* the step and the destroyed
            // sequence has to finish.
            //
            // What happens next is the gap: nothing reads an opponent's
            // `Eliminated`, because the explosion, the respawn and the
            // elimination bookkeeping are all unbuilt. So it coasts, settles and
            // is passed. See `oag_physics::damage::CraftState`.
            let pilot = self.ai_pilots[slot];
            let field = self.field_for(slot, &places);
            let ship = &mut self.world.ships[slot];
            let handling = ship.handling;
            let position = ship.physics.body.position;
            let controls = if ship.physics.craft_state == oag_physics::CraftState::Racing {
                ship.driver.drive(
                    &ship.physics,
                    &oag_ai::Context {
                        line: &self.racing_line,
                        tuning: &self.ai_tuning,
                        pilot: &pilot,
                        field: &field,
                    },
                )
            } else {
                oag_physics::ShipControls::default()
            };

            // The pads, measured from where the craft starts the tick, exactly as
            // slot 0's are. Both classes: a speed pad boosts an opponent and a
            // weapon pad hands it a pickup.
            let (moved, sweep) = self.pad_sweep(slot, position);
            let pad_hit = self.test_speedup_pads(slot, position, moved, &sweep);
            self.test_weapon_pads(slot, position, moved, &sweep);
            self.spend_opponent_pickup(slot, &controls, &field);

            // The driver just located itself, and the line is index-parallel to
            // `ai_order`, so this costs a lookup rather than a search.
            let index = self.world.ships[slot].driver.index as usize;
            let track_sample = self.ai_sample(index).map(Spline::track_sample);
            let track_sample_next = self
                .ai_sample(index + 1)
                .map(Spline::track_sample)
                .or(track_sample);
            let env = Environment {
                track_sample,
                track_sample_next,
                // **The same pads the player crosses.** Measured from the
                // position the tick started at and swept to where the craft is
                // now, exactly as slot 0's are - see `Race::pad_sweep`. Without
                // this an opponent is the only thing on the circuit that a speed
                // pad does not touch, and the field falls away from a player who
                // uses them.
                pad_hit,
                class_gravity_scale: self.class_gravity_scale,
                damage_rules,
                ..Environment::default()
            };
            oag_physics::step(
                &mut self.world.ships[slot].physics,
                &controls,
                &handling,
                &env,
                &self.collision,
                self.dt,
            );

            // **The same recovery the player gets**, and it is not a nicety.
            // Without it an opponent that leaves the geometry keeps going: the
            // solo benchmark measured craft receding from the track at racing
            // speed, 1,500 units per ten seconds, for the rest of the race.
            // Seven of the disc's twelve circuits never saw a completed lap for
            // this reason, and it read as a driving fault rather than as a
            // missing mechanism.
            //
            // Same `env` and same pre-step position the force law just ran with,
            // for the reason `reset_zone_touched` gives, and `index` is the
            // sample the craft was on before the step - the last place it is
            // known to have been on the track. Mapped out of the driver's index
            // space, because [`Self::respawn`] speaks the sample table's.
            let last_good = self.sample_index_of(index);
            self.respawn_cooldown[slot] = self.respawn_cooldown[slot].saturating_sub(1);
            // Unconditionally, and before the `||` could skip it: the dwell
            // counter has to see every tick or a craft banks time it never
            // spent away.
            let lost = self.lost_off_the_circuit(slot);
            if lost || self.reset_zone_touched(slot, &env, position) {
                self.respawn(slot, last_good);
            } else if self.respawn_cooldown[slot] == 0 {
                self.respawns_in_a_row[slot] = 0;
            }
        }
    }

    /// Advances every active craft's [`oag_race::Standing`].
    ///
    /// Slot order, and slot order only: this feeds the finishing order, which is
    /// simulation state. See `docs/architecture/determinism.md`.
    ///
    /// **`laps_target` comes from the player's race**, because it is the mode's
    /// number rather than a craft's - all eight are running the same event.
    fn update_standings(&mut self) {
        // A free function so the `course` and `world` borrows stay disjoint -
        // cloning a `Course` once a tick to satisfy the borrow checker would be
        // a `Vec` copy per frame for nothing.
        if let Some(course) = &self.course {
            advance_standings(&mut self.world, course);
        }
    }

    /// Advances every active craft's exhaust and lays down its trail sample.
    ///
    /// The original updates the exhaust inside the per-craft update, so all eight
    /// flares ramp, flicker and trail on their own state; this is that fan-out.
    /// A craft's ramp follows *its* thrust and *its* speed, so an opponent
    /// coasting into a corner dims while the leader on the straight does not.
    ///
    /// **Slot order, and slot 0 first.** Each craft draws from its own generator
    /// ([`exhaust_seed`]), so the order cannot change what any of them sees - but
    /// the player's stream is the one captures are pinned against, and keeping it
    /// first keeps the tick's shape the same as when the player was the only
    /// craft with a flare at all.
    ///
    /// The trail sample is one per tick, which is what `Trail_Update` does per
    /// frame - it takes no `dt` at all. The direction is the nozzle's own
    /// backwards axis, so a segment keeps the orientation the craft had when it
    /// was laid down rather than swinging with the current pose as the ship
    /// turns.
    fn advance_exhausts(&mut self) {
        for slot in 0..self.world.ship_count as usize {
            if !self.world.ships[slot].active {
                continue;
            }
            let ship = &self.world.ships[slot].physics;
            let thrust = ship.thrust;
            let speed = ship.body.linear_velocity.length();
            let back = -ship.body.forward();
            self.exhaust[slot].advance(self.dt, thrust, speed, &mut self.exhaust_rng[slot]);
            if let Some(nozzle) = self.nozzle_of(slot) {
                self.exhaust[slot].push_trail(nozzle, back);
            }
        }
    }

    /// Every craft's race position, `1`-based, in slot order.
    ///
    /// Inactive slots are placed too - they are at the start line and last,
    /// which is where a slot with no craft in it belongs. A caller that cares
    /// reads [`Self::ship_count`] first.
    #[must_use]
    pub fn places(&self) -> [u8; MAX_SHIPS] {
        let Some(course) = &self.course else {
            // No closed ring, so no positions. Everyone is first, which is the
            // honest answer for a track that cannot count a lap at all.
            return [1; MAX_SHIPS];
        };
        let standings: [oag_race::Standing; MAX_SHIPS] =
            std::array::from_fn(|slot| self.world.ships[slot].standing);
        oag_race::places(&standings, course)
    }

    /// What slot `slot` can see of the rest of the grid.
    ///
    /// **`Field::EMPTY` when the track has no closed ring.** `Standing::distance`
    /// needs a `Course` to mean anything, and a guessed along-track gap is worse
    /// than none - it would put a craft half a lap away in the mirror and have a
    /// driver defend against it. The same answer [`Race::places`] gives, for the
    /// same reason.
    ///
    /// Slot order throughout and `total_cmp` for every comparison, so nothing
    /// here can feed the simulation an order that depends on a hasher.
    ///
    /// Takes the places rather than calling [`Race::places`] itself, because the
    /// caller asks this once per craft and that would sort the standings eight
    /// times a tick to get the same answer.
    #[must_use]
    fn field_for(&self, slot: usize, places: &[u8; MAX_SHIPS]) -> oag_ai::Field {
        let Some(course) = &self.course else {
            return oag_ai::Field::EMPTY;
        };
        let mine = &self.world.ships[slot];
        if !mine.active {
            return oag_ai::Field::EMPTY;
        }
        let body = &mine.physics.body;
        let (forward, right) = (body.forward(), body.right());
        let my_distance = mine.standing.distance(course);
        let my_speed = mine.physics.body.linear_velocity.dot(forward);

        let mut field = oag_ai::Field {
            place: places[slot],
            ..oag_ai::Field::EMPTY
        };
        let (mut best_ahead, mut best_behind, mut best_alongside) =
            (f32::INFINITY, f32::INFINITY, f32::INFINITY);

        for other in 0..self.world.ship_count as usize {
            if other == slot || !self.world.ships[other].active {
                continue;
            }
            let them = &self.world.ships[other];
            let gap = them.standing.distance(course) - my_distance;
            if gap.abs() >= oag_ai::AWARENESS_RANGE {
                continue;
            }
            let to_them = them.physics.body.position - body.position;
            let range = to_them.length();
            // **The rate `|gap|` shrinks**, so it means the same thing for a
            // craft ahead and one behind - see `oag_ai::Rival::closing`.
            let their_speed = them.physics.body.linear_velocity.dot(forward);
            let approach = their_speed - my_speed;
            let closing = if gap >= 0.0 { -approach } else { approach };
            let rival = oag_ai::Rival {
                slot: other as u8,
                gap,
                offset: to_them.dot(right),
                closing,
                range,
                cos_bearing: if range > f32::EPSILON {
                    to_them.dot(forward) / range
                } else {
                    1.0
                },
            };

            // Alongside first, and exclusively: a craft level with this one is
            // not something to defend against or lift for, it is something to
            // avoid touching.
            if gap.abs() < ALONGSIDE_GAP && rival.offset.abs() < ALONGSIDE_WIDTH {
                if rival.offset.abs() < best_alongside {
                    best_alongside = rival.offset.abs();
                    field.alongside = Some(rival);
                }
            } else if gap >= 0.0 {
                if gap < best_ahead {
                    best_ahead = gap;
                    field.ahead = Some(rival);
                }
            } else if -gap < best_behind {
                best_behind = -gap;
                field.behind = Some(rival);
            }
        }
        field
    }

    /// The lap counter's ring, for a caller that wants to measure against it.
    #[must_use]
    pub fn course(&self) -> Option<&Course> {
        self.course.as_ref()
    }

    /// The player's own race position, `1`-based.
    #[must_use]
    pub fn player_place(&self) -> u8 {
        self.places()[0]
    }

    /// Craft against craft, every pair, once a tick.
    ///
    /// `oag_physics::pair::resolve` is `Body_ResolveContactPair` (`0x0884ef30`),
    /// read 2026-08-11 - the function this engine was missing, and the reason
    /// craft used to pass through each other. The response is recovered; the
    /// *shape* is not, and `pair::overlap` says so.
    ///
    /// **After every craft has been stepped**, not during. Stepping and
    /// resolving interleaved would let a craft that has already moved this tick
    /// collide with one that has not, which makes the outcome depend on slot
    /// order - and the original resolves contacts in its own pass, after
    /// integration, which
    /// `docs/ghidra/functions/ps2-pulse-eu/craft-update.md` records for the PS2
    /// twin.
    ///
    /// Pairs are visited in slot order, low index first, and each unordered pair
    /// exactly once. `pair::resolve` is symmetric in its two arguments - pinned
    /// by its own test - so the order cannot change the world; visiting in a
    /// fixed order anyway is what
    /// `docs/architecture/determinism.md` asks for, because a *sequence* of
    /// resolutions is order-dependent even when each one is not.
    fn resolve_craft_pairs(&mut self) {
        for a in 0..self.world.ship_count as usize {
            for b in (a + 1)..self.world.ship_count as usize {
                if !self.world.ships[a].active || !self.world.ships[b].active {
                    continue;
                }
                let (first, second) = self.world.ships.split_at_mut(b);
                let ship_a = &mut first[a];
                let ship_b = &mut second[0];
                let a_size = ship_a.handling.dimensions;
                let b_size = ship_b.handling.dimensions;
                oag_physics::pair::resolve(
                    &mut ship_a.physics,
                    &a_size,
                    &mut ship_b.physics,
                    &b_size,
                );
            }
        }
    }

    /// Fires an opponent's Rocket at the craft ahead, if its driver wants to.
    ///
    /// Returns whether anything left the rails, so the caller knows whether to
    /// spend the pickup.
    ///
    /// **The owner is the firing slot, and that is the line to get right.** The
    /// player's path passes `0` because the player *is* slot 0; an opponent
    /// passing `0` would put rockets in the air owned by the player, which
    /// `projectile::step` would then fly straight through the player and
    /// detonate on whoever actually fired them.
    /// `an_opponents_rocket_is_owned_by_the_slot_that_fired_it` pins it.
    fn fire_opponent_rocket(&mut self, slot: usize, field: &oag_ai::Field) -> bool {
        let context = oag_ai::Context {
            line: &self.racing_line,
            tuning: &self.ai_tuning,
            pilot: &self.ai_pilots[slot],
            field,
        };
        if self.world.ships[slot]
            .driver
            .wants_to_fire(&context)
            .is_none()
        {
            return false;
        }
        let Some(stats) = self
            .weapons
            .as_ref()
            .and_then(oag_formats::weapons::WeaponStats::rocket)
        else {
            return false;
        };
        let ship = &self.world.ships[slot];
        let shots = oag_gameplay::projectile::launch(
            &ship.physics,
            &ship.handling.dimensions,
            &stats,
            to_format_class(self.class),
        );
        let mut fired = 0;
        for (position, velocity) in shots {
            if self.world.projectiles.spawn(
                oag_formats::weapons::Weapon::Rocket,
                position,
                velocity,
                slot as u8,
            ) {
                fired += 1;
            }
        }
        fired > 0
    }

    /// What an opponent does with a pickup it is holding.
    ///
    /// # This is a policy, and it is the crudest one that is not "nothing"
    ///
    /// **Nothing about it is recovered.** The original decides this in
    /// `WeaponAIstats.xml` and whatever reads it - a sixth AI file that
    /// `AiStats_LoadAll` does not even load, so it has its own loader that has
    /// not been looked for. See
    /// `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`. Until that is read,
    /// anything here is invention, so it is kept small enough to be obviously
    /// provisional:
    ///
    /// - **Turbo is fired at once**, but only on a stretch the driver is not
    ///   braking for. A turbo spent into a corner is a turbo spent into a wall.
    ///   It arms that craft's own boost plume, the same reuse the player's Turbo
    ///   makes of the speed pad's visual.
    /// - **A Rocket is aimed**, as of 2026-08-11, at whatever
    ///   `oag_ai::Driver::wants_to_fire` picks - a craft ahead, in range, inside
    ///   a cone, with straight enough road between. It is kept rather than spent
    ///   when there is no target, no authored rocket or no free slot.
    /// - **Everything else is absorbed**, which pays energy into the pool and is
    ///   a real effect rather than a discard. It is also what a cautious human
    ///   does with a weapon they cannot aim, and the remaining nine cannot be
    ///   aimed: they need a lock, a beam or a mechanic nothing has read.
    fn spend_opponent_pickup(
        &mut self,
        slot: usize,
        controls: &oag_physics::ShipControls,
        field: &oag_ai::Field,
    ) {
        let Some(weapon) = self.world.ships[slot].pickup.weapon else {
            return;
        };
        // Looked up and copied out before the branches, so the borrow of
        // `self.weapons` ends here: the Rocket arm needs `&mut self` to put
        // anything in the air, and every one of these returns an owned value.
        let Some((simple, absorb)) = self
            .weapons
            .as_ref()
            .map(|weapons| (weapons.simple(weapon), weapons.absorb(weapon)))
        else {
            return;
        };

        if weapon == oag_formats::weapons::Weapon::Turbo {
            // Full throttle means the speed target is not asking it to slow for
            // anything it can see, which is the only "is this a straight?" this
            // engine has. **The controls the driver chose this tick**, not
            // `ShipState::thrust`: that is written by `controls::update` inside
            // the step, which has not run yet, so it still holds last tick's
            // value here.
            // **`> 0.0`, not `>= 1.0`.** `Driver::caution` scales the throttle
            // down by an arbitrary fraction when a craft is closing on one
            // ahead, so an exact comparison against full throttle would have a
            // craft silently stop using Turbo the moment anybody was within
            // forty-five units of its nose. What this asks is the question it
            // always meant: is the speed target letting it accelerate, or is it
            // lifting for a corner.
            let on_a_straight = controls.thrust > 0.0;
            let Some(simple) = simple else {
                return;
            };
            if !on_a_straight {
                return;
            }
            // **And clear over the distance the boost itself covers**, which is
            // not the distance the driver was looking at. Its braking horizon is
            // built from the speed it is doing now, so a craft that fires a
            // Turbo at 126 arrives at the next corner doing 270 having checked
            // 160 units ahead. Measured on `16_Track`: one craft in a field of
            // seven did exactly that and left the circuit, and it never touched
            // a wall on the way - see `docs/gameplay/ai.md`.
            // **And not into the back of somebody.** A separate condition from
            // the throttle above, deliberately: coupling the two through
            // `Driver::caution`'s fractional lift would make this depend on how
            // hard the craft happened to be lifting rather than on whether
            // there is anyone there.
            if let Some(ahead) = field.ahead
                && ahead.gap < TURBO_CLEARANCE
            {
                return;
            }
            let ship = &self.world.ships[slot];
            let speed = ship
                .physics
                .body
                .linear_velocity
                .dot(ship.physics.body.forward())
                .max(0.0);
            let boosted = speed * TURBO_SPEED_RATIO;
            let context = oag_ai::Context {
                line: &self.racing_line,
                tuning: &self.ai_tuning,
                pilot: &self.ai_pilots[slot],
                field,
            };
            if !ship
                .driver
                .allows_speed(&context, boosted, boosted * simple.time)
            {
                return;
            }
            self.world.ships[slot].physics.turbo_timer = simple.time;
            // And its own plume, the same reuse the player's Turbo makes of it -
            // an opponent's boost has to be visible from behind, or the field
            // gains speed with nothing on screen saying why.
            self.exhaust[slot].boost(exhaust::BOOST_SECONDS);
        } else if weapon == oag_formats::weapons::Weapon::Rocket
            && self.fire_opponent_rocket(slot, field)
        {
            // Fired. `fire_opponent_rocket` reports false when there was no
            // target, no authored rocket or no free slot, and then the pickup is
            // kept rather than spent - the same rule the player's path follows.
        } else {
            let Some(amount) = absorb else {
                return;
            };
            let dimensions = self.world.ships[slot].handling.dimensions;
            oag_physics::damage::add(&mut self.world.ships[slot].physics, &dimensions, amount);
        }
        self.world.ships[slot].pickup.weapon = None;
    }

    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// The snapshot is mapped through [`oag_gameplay::ship_controls`], which is the
    /// one place "cross is thrust" is written down.
    pub fn tick(&mut self, snapshot: &InputSnapshot) -> Evaluated {
        let controls = ship_controls(snapshot, self.scheme);

        // Before the force law, so a Turbo fired this tick boosts this tick.
        self.spend_pickup(snapshot);

        // The two spline samples the magstrip hold reads. In the original these are
        // `AiTrack_LocatePosition`'s two output records on the ship entity; here
        // they are the nearest table entry and the one after it, which is the same
        // "where am I and what is next" pair at this crate's resolution. Off a
        // magstrip nothing consumes them, so a track without one is unaffected.
        let nearest = self
            .spline
            .nearest(self.world.ships[0].physics.body.position);
        let index = nearest.map(|(index, _, _)| index);
        let track_sample = nearest.map(|(_, sample, _)| Spline::track_sample(sample));
        // The neighbour is the next entry in table order, which at the end of a
        // path is the *next path's* first sample rather than the geometric
        // successor through a junction. The original follows the junction graph;
        // this is one sample out of 34,000 per lap and is recorded rather than
        // pretended away.
        //
        // Still table order, deliberately: the player's locator works in sample
        // space, where every path the track authors is reachable. An opponent's
        // equivalent goes through [`Self::ai_sample`] instead, because a driver
        // is asking a lap question. See [`ai_order`].
        let track_sample_next = index
            .and_then(|index| self.spline.sample(index + 1))
            .map(Spline::track_sample);

        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            self.world.ships[0].segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        // Zone's auto-speed, from the zone the run has reached. Read before the
        // step, from the zone the last tick left behind, because that is the
        // order the original runs in: `Zone_Update` assigns the counter into the
        // craft and `Ship_UpdateEngine` reads it on the following craft update.
        let auto_speed = self
            .zone
            .map(|zone| oag_race::zone::thrust(zone.start, zone.increment, self.world.race.zone));
        let before = self.world.ships[0].physics.body.position;
        // Step 15's input, measured before the step because that is when the
        // original measures it: `Ship_ApplySpeedupPad` runs inside the same craft
        // update as the other fourteen terms, all of them against the position the
        // tick started at, and the integrator moves the body afterwards.
        let (moved, sweep) = self.pad_sweep(0, before);
        let pad_hit = self.test_speedup_pads(0, before, moved, &sweep);
        // After the speed pad, sharing its sweep. The two are independent - a
        // track can author a weapon pad on top of a speed pad and both fire -
        // and this one returns nothing, because a pickup is an event rather than
        // a per-tick force.
        self.test_weapon_pads(0, before, moved, &sweep);
        let ship = &mut self.world.ships[0];
        let env = Environment {
            track_sample,
            track_sample_next,
            auto_speed,
            pad_hit,
            class_gravity_scale: self.class_gravity_scale,
            // The mode's own `Weapons`/`Damage` defaults, which decide both what
            // a wall costs the energy pool and whether it recovers - a time trial
            // and a speed lap run with both off, so their pool floors at 20 and
            // regenerates, exactly as the disc's manual text describes. See
            // `oag_gameplay::damage_rules`.
            damage_rules: oag_gameplay::damage_rules(self.world.race.mode),
            ..Environment::default()
        };
        let evaluated = oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            &self.collision,
            self.dt,
        );

        // **After the craft moved and before the race rules.** A rocket fired
        // this tick was spawned from the pose the tick *started* at, in
        // `spend_pickup`, so flying it here gives it a full tick of travel from
        // where the muzzle was rather than half a tick from wherever the step
        // ended up - which is the same argument that puts the Turbo's boost on
        // the tick it was fired. Before the rules, so a craft blown up by a
        // blast this tick is blown up before the lap counter reads it.
        let rocket = self.weapons.as_ref().and_then(|w| w.rocket());
        let damage_rules = oag_gameplay::damage_rules(self.world.race.mode);
        let impacts = oag_gameplay::projectile::step(
            &mut self.world,
            self.dt,
            &self.collision,
            rocket.as_ref(),
            damage_rules,
        );
        // After `projectile::step`, so a flare rides where its rocket
        // actually ended the tick rather than a tick behind it.
        self.advance_projectile_flares();
        for impact in impacts.iter().flatten() {
            self.ignite_blast(impact.point, impact.struck.map(usize::from));
        }
        // After the craft have moved, so a flare sits on this tick's nozzle
        // rather than the last one's.
        self.advance_engine_flares();
        self.stage.advance(self.dt, &mut self.stage_rng);

        self.respawn_cooldown[0] = self.respawn_cooldown[0].saturating_sub(1);
        if self.reset_zone_touched(0, &env, before) {
            // `index` is where the ship was *before* this tick moved it, which is
            // as close to "last known good" as this loop can cheaply get.
            self.respawn(0, index);
        } else if self.respawn_cooldown[0] == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.respawns_in_a_row[0] = 0;
        }

        self.step_opponents();
        self.resolve_craft_pairs();

        self.world.tick += 1;

        // The race rules run last of the simulation, on the position the step
        // produced and the tick it produced it on. Running them before the step
        // would test last tick's position against this tick's clock, which is a
        // whole tick of error on a quantity whose job is to be exact at one
        // instant. A track with no closed ring simply has no lap counter; the
        // load report already said so.
        // The craft finished blowing up this tick, so the race is over. Checked
        // before the rules run rather than after, because a wrecked craft's
        // position should not go on counting laps - and checked every tick
        // because `RaceState::eliminate` is the thing that makes it idempotent.
        //
        // Only Zone can reach this: a time trial and a speed lap race with the
        // original's `Damage` option off, which floors their pool at 20. See
        // `oag_gameplay::damage_rules`.
        if self.world.ships[0].physics.craft_state == oag_physics::CraftState::Eliminated
            && self.world.race.eliminate()
        {
            // The original plays `_BLOWUP`, hides the HUD and swings the camera
            // into its mode 5 on the way here. **None of that is built** - there
            // is no explosion, no HUD hide and no camera mode - so the race
            // simply stops. See `oag_physics::damage::CraftState`.
        }

        if let Some(course) = &self.course {
            let position = self.world.ships[0].physics.body.position;
            // The same flag the collision sparks fire on, so "the HUD says that
            // zone was not clean" and "sparks came off the hull" cannot disagree.
            let contact = evaluated.wall.impact;
            let outcome =
                self.world
                    .race
                    .update(course, position, self.world.tick, self.dt, contact);

            // A zone survived without touching anything pays shield back, clamped
            // to the ship's own pool. `Ship_SetShield` (`0x0883e6f4`) does the
            // same clamp against the stat block's maximum, so a full ship gains
            // nothing and the bar cannot overfill.
            if outcome.perfect_zone
                && let Some(zone) = self.zone
            {
                let ship = &mut self.world.ships[0];
                let max = ship.handling.dimensions.shield;
                ship.physics.shield = (ship.physics.shield + zone.recharge).min(max);
            }

            if outcome.lap_completed {
                self.grant_free_turbo();
            }
        }

        // Every craft's standing, the player's included, from the position it
        // holds now. **After the player's `RaceState`**, so the two see the same
        // tick, and slot 0's lap is then taken from the standing rather than
        // counted a second time - see `Ship::standing`. Outside the borrow above
        // rather than inside it because this walks the whole ship array.
        self.update_standings();

        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);

        // Advanced here, on the fixed tick, and not in the frame loop. That is
        // what makes the headless `capture` path - which calls only `tick` -
        // produce the same flare at the same tick count as the window does, and
        // it is the same reason the chase camera is advanced from here.
        self.advance_exhausts();

        // Open while the boost's own timer runs, close once it has expired, both
        // as an exponential approach so neither edge is a step. Driven from
        // `pad_timer` rather than from the exhaust, because the exhaust's boost
        // is a `max` that other things may one day also arm and this must follow
        // the pad specifically.
        if self.boost_fov_kick != crate::display::BoostFovKick::OFF {
            let boosting = self.world.ships[0].physics.pad_timer > 0.0;
            let (target, rate) = if boosting {
                (1.0, BOOST_FOV_OPEN_RATE)
            } else {
                (0.0, BOOST_FOV_CLOSE_RATE)
            };
            self.boost_kick += (target - self.boost_kick) * rate * self.dt;
            // Snapped, so a closed kick is *exactly* zero and `projection` takes
            // its bit-identical path rather than an `atan(tan(x))` round trip
            // that never quite settles.
            if !boosting && self.boost_kick < 1.0e-3 {
                self.boost_kick = 0.0;
            }
        }

        // The flaps, on the fixed tick beside the camera and the exhaust, and
        // for the same reason: a headless capture calls only `tick`, so an
        // animation advanced in the frame loop would be at a different angle in
        // a screenshot than in a window at the same tick count.
        //
        // Two rates, not one. `up_speed` and `down_speed` are authored beside
        // each other in `<AirbrakeGraphics>` and are *not* the force law's
        // `gain`/`falloff`, so a flap deploys and returns at its own pace while
        // the airbrake it depicts ramps at another. The state it chases is the
        // ramped `airbrake_left`/`airbrake_right` on `0..=100` rather than the
        // raw input, so a flap follows the airbrake the ship actually has.
        //
        // **The rates are on the airbrake's own `0..=100` scale**, per second,
        // which is why `self.flaps` holds a level rather than an angle. Not
        // read out of the binary - no consumer of `<AirbrakeGraphics>` was
        // located - but not a coin flip either: every team authors
        // `up_speed = 500` against a `down_speed` of 80 to 120, and on this
        // reading that is a flap that snaps out in 0.2 s and folds back over
        // about a second, which is what an airbrake does. Read as *fractions*
        // of full deflection per second instead, 500 would be full travel in
        // two milliseconds and the parameter would not be worth authoring.
        // It is also the convention `<Airbrake gain/falloff>` already uses on
        // the same scale, one element away.
        let ship = &self.world.ships[0].physics;
        for (flap, level) in self
            .flaps
            .iter_mut()
            .zip([ship.airbrake_left, ship.airbrake_right])
        {
            let target = level.clamp(0.0, 100.0);
            let (rate, rising) = if target > *flap {
                (self.flap_graphics.up_speed, true)
            } else {
                (self.flap_graphics.down_speed, false)
            };
            let step = rate * self.dt;
            *flap = if rising {
                (*flap + step).min(target)
            } else {
                (*flap - step).max(target)
            };
        }

        // Cooldown-gated, not edge-triggered - see `Self::sparks_cooldown`'s
        // doc comment for why a sustained scrape must re-fire periodically
        // rather than spawn once and go silent.
        self.sparks_cooldown = (self.sparks_cooldown - self.dt).max(0.0);
        // The trigger rule runs whether or not the disc's own effect
        // loaded - see `Setup::effects`. Only the pool needs it.
        let effect = self.effects.get(sparks::DAMAGE_EFFECT).cloned();
        let can_fire = evaluated.wall.impact && self.sparks_cooldown <= 0.0;
        let model_matrix = self.ship_model_matrix();
        if can_fire && let Some(contact) = evaluated.wall.resolved {
            // `contact.point` is deliberately the *penetrating* hull sample
            // point, not the wall surface - see its doc comment on
            // `oag_physics::wall::WallContact`, which matches the original's
            // own `Collision_AddContact` exactly. Correct for physics, wrong
            // for a visual: that point sits up to `depth` units inside the
            // opaque wall, so a spark quad centred there is depth-tested away
            // by the wall's own geometry almost every time and reads as
            // nothing spawning. `point + normal * depth` is the same
            // correction `resolve_contact`'s `escape` vector already applies
            // to push the hull back out to the surface.
            let surface = contact.point + contact.normal * contact.depth;
            // The original triggers the `Ship Collision Fx` locator nearest
            // the contact and the burst emits from that node from then on
            // (`Ship_DispatchCollisionFx`); with no locators authored, the
            // surface point itself is mapped into model space so it still
            // rides the hull with rotation.
            let anchor_model = self
                .collision_fx
                .iter()
                .copied()
                .min_by(|a, b| {
                    let da = (model_matrix.transform_point3(*a) - surface).length_squared();
                    let db = (model_matrix.transform_point3(*b) - surface).length_squared();
                    da.total_cmp(&db)
                })
                .unwrap_or_else(|| model_matrix.inverse().transform_point3(surface));
            self.sparks_anchor = Some(anchor_model);
            if let Some(effect) = effect.as_deref() {
                self.sparks.ignite(
                    effect,
                    surface,
                    sparks::severity(evaluated.wall.impact_speed),
                );
            }
            self.sparks_ignitions += 1;
            self.sparks_cooldown = sparks::COLLISION_COOLDOWN;
        }
        // Unconditional, like the exhaust: the emitters keep trickling and
        // already-live particles keep ageing even on a tick with no fresh
        // impact. The anchor is the chosen hull locator under the ship's
        // *current* transform, the way the original's scene-graph node rides
        // the craft.
        let anchor = self
            .sparks_anchor
            .map_or(self.ship().physics.body.position, |local| {
                model_matrix.transform_point3(local)
            });
        if let Some(effect) = effect.as_deref() {
            self.sparks
                .advance(effect, self.dt, anchor, &mut self.sparks_rng);
        }

        evaluated
    }

    /// Below this much movement in a tick, the pad test is swept instead of a
    /// point.
    ///
    /// `Pad_SweptTest`'s `25.0`. It reads backwards at first - the *slow* case
    /// gets the more careful test - and the reason is that the interpolation is
    /// only worth anything when the step is short enough that four samples cover
    /// it. Past this the ship has moved further than a pad is deep and four points
    /// would not close the gap either, so the original stops paying for them.
    /// Confidence 88: measured live against a running PPSSPP session - a
    /// stationary or driving craft never crosses it (`0.000162`/`0.778459`
    /// units of movement observed), a scripted 200-unit same-tick teleport
    /// does (`199.999969`, confirmed to take the direct-test branch by
    /// breakpoint). See `pads.md`'s `Pad_SweptTest` section.
    const PAD_SWEEP_LIMIT: f32 = 25.0;

    /// How many interpolated points the swept test checks.
    ///
    /// `Pad_SweptTest` walks `t = 0.25, 0.5, 0.75, 1.0` - the destination is
    /// included and the origin is not, because the origin was this test's
    /// destination last tick.
    const PAD_SWEEP_STEPS: u32 = 4;

    /// Hands a time trial or a speed lap its free Turbo.
    ///
    /// **Two shipped records say this happens, and they were found
    /// independently.** The disc's own event text is explicit -
    /// `MSC_EVENT_TT` and `MSC_EVENT_SL` each read *"You will be given a free
    /// turbo pickup once per lap"* - and `TimeTrial_HUD.xml` authors a
    /// `PickupBackground` and **exactly one** weapon icon, `TurboIcon`, where
    /// `Arcade_HUD.xml` authors all thirteen and `Zone_HUD.xml` authors none.
    /// A layout carrying one pickup widget for a mode whose `Weapon Pad`s are
    /// hidden is otherwise inexplicable. Confidence **85** on the rule; the
    /// second record was found by `every_weapon_has_an_icon_widget_named_after_it`
    /// failing against the assumption that these layouts carried none.
    ///
    /// **What is not recovered is *when* within the lap.** "Once per lap" is
    /// awarded here on the lap edge, which means the ship gets its first one on
    /// crossing the line to start lap 2 rather than at the start line - and the
    /// original may well hand it over at the start of a lap instead, which is
    /// the same edge one lap earlier. Nothing read says which. The conservative
    /// choice is the one that cannot give a free boost before the clock starts.
    ///
    /// Zone is excluded: it authors no pickup widgets at all, and its event text
    /// promises nothing.
    fn grant_free_turbo(&mut self) {
        if !matches!(self.world.race.mode, Mode::TimeTrial | Mode::SpeedLap) {
            return;
        }
        // The same "only into an empty slot" rule a pad follows, so a player who
        // has not spent last lap's turbo does not silently lose this one - they
        // keep the one they have.
        if !self.world.ships[0].pickup.is_empty() {
            return;
        }
        // Gated on the table, so a disc whose weapon file did not load hands
        // out nothing rather than a Turbo with no duration to fire it for.
        if self
            .weapons
            .as_ref()
            .and_then(|w| w.simple(oag_formats::weapons::Weapon::Turbo))
            .is_none()
        {
            return;
        }
        self.world.ships[0].pickup.weapon = Some(oag_formats::weapons::Weapon::Turbo);
    }

    /// Fires or absorbs whatever the craft is holding.
    ///
    /// **The buttons are recovered and the actions are not.**
    /// `Options_LoadDefaultControlMapping` (`0x0883672c`) maps action 1, *fire*,
    /// to `SQUARE` and action 2, *absorb*, to `CIRCLE`, at confidence 90 - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, whose table is
    /// self-checking on the `accelerate`/`CROSS` row this project already knew.
    /// What each does with the pickup is this engine's, for the reason
    /// `oag_gameplay::pickup` gives at length: no grant, fire or absorb call
    /// site has been found.
    ///
    /// Absorb *pays* the recovered `<Weapon><Stats absorb>` into the pool
    /// through the recovered clamp, so of the two it is the better evidenced.
    ///
    /// Edge-triggered on both, so holding a button spends one pickup rather than
    /// one a tick. Nothing happens with an empty slot, including no sound - the
    /// original's "nothing to fire" cue is not implemented.
    fn spend_pickup(&mut self, snapshot: &InputSnapshot) {
        let fire = snapshot
            .buttons
            .is_pressed(oag_gameplay::input::button::SQUARE);
        let absorb = snapshot
            .buttons
            .is_pressed(oag_gameplay::input::button::CIRCLE);
        if !fire && !absorb {
            return;
        }
        let Some(weapon) = self.world.ships[0].pickup.weapon else {
            return;
        };
        let Some(weapons) = self.weapons.as_ref() else {
            return;
        };

        // Fire wins a same-tick tie. Arbitrary, and stated rather than left to
        // the order of two `if`s: a pad hands out one thing and both buttons
        // spend it, so the two can only race on a tick where the player pressed
        // both.
        if fire {
            match weapon {
                oag_formats::weapons::Weapon::Turbo => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // The file authors no Turbo. Nothing to fire *with*, so
                        // the pickup is kept rather than spent on nothing.
                        return;
                    };
                    self.world.ships[0].physics.turbo_timer = simple.time;
                    // The same visual a speed pad arms, on the same argument:
                    // the plume is what a boost looks like, and there is one
                    // boost. Not recovered for this path - no capture of a fired
                    // Turbo exists - so it is the plume being reused rather than
                    // a reading of what the original shows.
                    self.exhaust[0].boost(exhaust::BOOST_SECONDS);
                }
                oag_formats::weapons::Weapon::Shield => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // As above: nothing to raise a shield *for*, so the
                        // pickup is kept rather than spent on nothing.
                        return;
                    };
                    self.world.ships[0].physics.shield_pickup_timer = simple.time;
                    // No visual. The disc authors a per-team
                    // `Data\Ships\<Team>\<Team>shield.vex` for the shield hit
                    // response (`docs/overview/roadmap.md`) and none of it is
                    // built, so reusing the exhaust plume the way the Turbo does
                    // would be inventing a look rather than reusing one. The
                    // HUD icon leaving the slot is the only feedback today.
                }
                oag_formats::weapons::Weapon::Rocket => {
                    let Some(stats) = weapons.rocket() else {
                        // No authored rocket, so nothing to put in the air.
                        return;
                    };
                    let ship = &self.world.ships[0];
                    // **Three, together, fanned by `<Rocket spread>`** - see
                    // `oag_gameplay::projectile::launch` and
                    // `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`. The
                    // order is the original's, and it matters: it decides which
                    // slot each rocket lands in, and the slot is hashed state.
                    let shots = oag_gameplay::projectile::launch(
                        &ship.physics,
                        &ship.handling.dimensions,
                        &stats,
                        to_format_class(self.class),
                    );
                    // Spent on the *first* shot getting away. A partial volley
                    // is better than a pickup that survives having fired two of
                    // three, and the array cannot fill from one press in a race
                    // this engine can currently run.
                    let mut fired = 0;
                    for (position, velocity) in shots {
                        if self.world.projectiles.spawn(
                            oag_formats::weapons::Weapon::Rocket,
                            position,
                            velocity,
                            0,
                        ) {
                            fired += 1;
                        }
                    }
                    if fired == 0 {
                        // Every slot was taken. Keep the pickup rather than
                        // spend it on a volley that never left - the same rule
                        // the two arms above follow for a missing table.
                        return;
                    }
                }
                // The other ten have no effect to run. Deliberately *not* spent:
                // a pickup that vanishes when fired and does nothing is worse
                // than one the player can still absorb. They cannot reach here
                // anyway while `pickup::IMPLEMENTED` excludes them; the arm
                // exists so adding to that list is a compile-visible choice
                // rather than a silent no-op.
                _ => return,
            }
        } else {
            let Some(amount) = weapons.absorb(weapon) else {
                return;
            };
            let handling = self.world.ships[0].handling;
            oag_physics::damage::add(
                &mut self.world.ships[0].physics,
                &handling.dimensions,
                amount,
            );
        }
        self.world.ships[0].pickup.weapon = None;
    }

    /// One 64-bit fingerprint of everything this race carries from tick to tick.
    ///
    /// [`oag_gameplay::hash::hash_world`] plus the pad state, which lives here
    /// rather than in the world because a pad belongs to the track and not to a
    /// craft - `docs/gameplay/pickups.md` states that and this does not overturn
    /// it. The two are folded into one hasher rather than one hashing the
    /// other's output, so the result is a single stream.
    ///
    /// # Why the pad timers have to be in it
    ///
    /// This is the field the known-gap section of `docs/gameplay/pickups.md`
    /// uses as its worked example: **a pad refresh timer one tick out shifts
    /// every subsequent draw**, because a pad that is ready a tick earlier grants
    /// a pickup a tick earlier and moves the generator with it. Nothing about
    /// that shows in a ship's dynamics until the pickup itself differs, by which
    /// point the cause is thousands of ticks back.
    ///
    /// **Render-only state is not here** - the exhaust, the sparks, the boost
    /// kick, the airbrake flaps and the blast flashes all have their own
    /// generators and none of them may reach the simulation. See
    /// [`Self::boost_kick`].
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut hasher = oag_core::hash::StateHasher::new();
        oag_gameplay::hash::write_world(&mut hasher, &self.world);

        for left in &self.weapon_pad_refresh_left {
            hasher.write_f32(*left);
        }
        // The broadphase caches, which are simulation state and not a cache in
        // the "can be recomputed" sense: a stale entry changes which tick a pad
        // is next measured on, and therefore which tick it triggers.
        // Every racer's row, not only the player's, and every slot of every row
        // whether or not a craft occupies it - the same argument the world hash
        // makes for its inactive ship slots.
        for row in &self.weapon_pad_distance {
            for distance in row {
                hasher.write_f32(*distance);
            }
        }
        for row in &self.pad_distance {
            for distance in row {
                hasher.write_f32(*distance);
            }
        }
        for current in self.weapon_pad_current {
            write_pad_index(&mut hasher, current);
        }
        for current in self.pad_current {
            write_pad_index(&mut hasher, current);
        }
        for previous in self.pad_previous_position {
            match previous {
                None => hasher.write_u8(0),
                Some(position) => {
                    hasher.write_u8(1);
                    hasher.write_vec3(position);
                }
            }
        }
        // Every slot, because every craft recovers now. Fixed length, so the
        // stream's shape does not depend on how many craft are on the grid.
        for cooldown in self.respawn_cooldown {
            hasher.write_u32(cooldown);
        }
        for run in self.respawns_in_a_row {
            hasher.write_u32(run);
        }
        for dwell in self.lost_ticks {
            hasher.write_u32(dwell);
        }

        hasher.finish()
    }

    /// Plays the explosion a rocket that just went off authored.
    ///
    /// **Recovered end to end.** Which of the two files plays:
    /// [`TRACK_BLAST_EFFECT`] from `Rocket_Update`'s (`0x0885d2a8`) two
    /// collision branches, [`CRAFT_BLAST_EFFECT`] from
    /// `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) on the craft-hit path.
    /// Where it plays: a craft hit is drawn at the *struck craft's* own
    /// position dropped by [`CRAFT_BLAST_DROP`], not at the rocket's impact
    /// point. What it looks like: everything, out of the file, through
    /// [`psys::Stage`]. A craft that has since gone inactive falls back to
    /// the impact point rather than reading a stale pose.
    ///
    /// Nothing is drawn when the effect did not load. That is deliberate and
    /// it is the rule for every effect here: an authored particle system is
    /// not something to approximate with billboards, because a stand-in that
    /// reads as *plausible* is how a wrong picture survives review. Until
    /// 2026-08-12 this method drew three expanding additive puffs and a
    /// separate eight-billboard smoke trail, all invented.
    ///
    /// [`Race::sparks`] is deliberately not reused for it: that system
    /// re-anchors to the hull every tick, so a burst ignited at a rocket's
    /// impact would emit from the craft instead. It is one hull-mounted
    /// emitter, and this is why the stage exists alongside it.
    fn ignite_blast(&mut self, point: Vec3, struck: Option<usize>) {
        let (name, at) = self.blast_for(point, struck);
        let Some(effect) = self.effects.get(name).cloned() else {
            return;
        };
        // Neutral severity: the field the collision sparks derive from an
        // impulse is the *hull's*, and nothing on the rocket path has been
        // read as feeding it. See `oag_render::sparks::severity`.
        self.stage.play(&effect, at, 1.0);
    }

    /// Which explosion a hit plays and where, split out from
    /// [`Race::ignite_blast`] so the recovered part can be asserted without a
    /// disc to load the effect from.
    fn blast_for(&self, point: Vec3, struck: Option<usize>) -> (&'static str, Vec3) {
        match struck {
            Some(slot) if self.world.ships[slot].active => (
                CRAFT_BLAST_EFFECT,
                self.world.ships[slot].physics.body.position - Vec3::Y * CRAFT_BLAST_DROP,
            ),
            // A craft that has gone inactive since the hit falls back to the
            // impact point rather than reading a stale pose - still its own
            // effect, because what was struck is what chose the file.
            Some(_) => (CRAFT_BLAST_EFFECT, point),
            None => (TRACK_BLAST_EFFECT, point),
        }
    }

    /// Keeps an [`ENGINE_FLARE_EFFECT`] instance on every active craft's
    /// nozzle, where the source authors one.
    ///
    /// **The PS2 port authors an engine flare as a particle effect and the
    /// PSP does not.** On a PSP-sourced race the effect is absent from the
    /// library, nothing attaches, and [`oag_render::exhaust`]'s procedural
    /// flare draws as it always has. On a PS2-sourced one the asset plays
    /// and the procedural quad steps aside - see
    /// [`Race::engine_flare_effect`], which is what the renderer asks.
    ///
    /// The trigger needs no reverse-engineering: an engine flare is on for
    /// as long as the craft is, which is why this one is wired where the
    /// other PS2-only effects are not. Both its emitters are
    /// [`oag_formats::pob::flags::LOOPING`], so it runs until detached.
    ///
    /// Anchored at the `Engine Flare` locator under the craft's *current*
    /// transform, the same point the procedural flare uses, so the two are
    /// interchangeable rather than merely similar.
    fn advance_engine_flares(&mut self) {
        let Some(effect) = self.effects.get(ENGINE_FLARE_EFFECT).cloned() else {
            return;
        };
        for slot in 0..MAX_SHIPS {
            let nozzle = self.world.ships[slot]
                .active
                .then(|| self.nozzle_of(slot))
                .flatten();
            match (nozzle, self.engine_flare[slot]) {
                (Some(at), Some(playing)) => self.stage.follow(playing, at),
                (Some(at), None) => self.engine_flare[slot] = self.stage.attach(&effect, at, 1.0),
                // Gone or never had a locator: hand the instance back and
                // let its particles fade rather than cutting them.
                (None, Some(playing)) => {
                    self.stage.detach(playing);
                    self.engine_flare[slot] = None;
                }
                (None, None) => {}
            }
        }
    }

    /// Keeps a [`ROCKET_FLARE_EFFECT`] instance riding every projectile in
    /// the air, and takes it off the ones that are gone.
    ///
    /// **Recovered.** `Rocket_Init` (`0x0885cdb8`) attaches the flare at
    /// launch and it rides the rocket for the whole flight; both emitters are
    /// [`oag_formats::pob::flags::LOOPING`], so the 100-tick duration they
    /// author is not a countdown and the effect does not need re-triggering
    /// to outlast it.
    ///
    /// A detonated rocket has its flare **detached, not killed**: emission
    /// stops and the particles already out finish their own lives, so the
    /// smoke outlives the rocket the way it does in the original. That is
    /// also the bug the invented trail could not avoid - it had to drop the
    /// whole streak the instant the slot vacated, or the next projectile to
    /// take that slot would have inherited it.
    ///
    /// **Called after `projectile::step`**, so a flare is anchored where its
    /// rocket ended the tick.
    fn advance_projectile_flares(&mut self) {
        let effect = self.effects.get(ROCKET_FLARE_EFFECT).cloned();
        for (slot, projectile) in self.world.projectiles.slots.iter().enumerate() {
            match (projectile.kind.is_none(), self.projectile_flare[slot]) {
                // Gone: hand the instance back and let it fade out.
                (true, Some(playing)) => {
                    self.stage.detach(playing);
                    self.projectile_flare[slot] = None;
                }
                (true, None) => {}
                (false, Some(playing)) => self.stage.follow(playing, projectile.position),
                // Freshly in the air. `attach` returning `None` means the
                // stage is full of flares already, and this rocket simply
                // flies without one rather than evicting someone else's.
                (false, None) => {
                    if let Some(effect) = effect.as_ref() {
                        self.projectile_flare[slot] =
                            self.stage.attach(effect, projectile.position, 1.0);
                    }
                }
            }
        }
    }

    /// This frame's fallback projectile sprites, in the engine flare's shape.
    ///
    /// `right` and `up` are the camera's, read out of the view matrix the same
    /// way the exhaust's are.
    ///
    /// `modelled` says the caller is drawing the rockets as
    /// [`ROCKET_MODEL_ENTRY`]'s mesh, in which case this draws **nothing**:
    /// the glow around a modelled rocket is [`ROCKET_FLARE_EFFECT`], played
    /// off the disc through [`Race::stage`], and a billboard on top of it
    /// would be a second invented one. What is left here is the case where
    /// the model did not load and a projectile would otherwise be invisible -
    /// see [`PROJECTILE_SPRITE_HALF_SIZE`].
    #[must_use]
    pub fn projectile_sprites(
        &self,
        right: Vec3,
        up: Vec3,
        modelled: bool,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        let mut vertices = Vec::new();
        if modelled {
            return vertices;
        }
        for projectile in &self.world.projectiles.slots {
            if projectile.kind.is_none() {
                continue;
            }
            vertices.extend(exhaust::sprite(
                projectile.position,
                right,
                up,
                PROJECTILE_SPRITE_HALF_SIZE,
                1.0,
            ));
        }
        vertices
    }

    /// Where each live rocket is and how it is oriented, for the model draw.
    ///
    /// **Forward alignment is recovered; the quarter-turn is not.**
    /// `Rocket_Update` (`0x0885d2a8`) rebuilds a basis every tick from the
    /// rocket's normalised velocity and the surface normal under it, hands it to
    /// the model's scene node, and rotates it by a further `-pi/2` about an axis
    /// this project has not resolved - the call is an unresolved import stub.
    /// So this aligns the model along its velocity, which is the evidenced part,
    /// and does **not** invent the extra rotation. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    ///
    /// The original's second basis vector is the *track* normal, which this
    /// engine does not carry on a projectile (its rockets fly straight and never
    /// consult the surface - the same page records that gap). World up stands in,
    /// which only decides the model's roll about its own length.
    ///
    /// One entry per live rocket, in slot order, so the caller can zip it
    /// against its drawables.
    ///
    /// # No rendered frame has yet contained a rocket
    ///
    /// Worth stating rather than leaving to be discovered. What *is* checked:
    /// the model loads off a real disc and its long axis is the one aimed down
    /// the velocity here
    /// (`the_rocket_model_is_longest_along_the_axis_it_is_flown_down`), the
    /// bases below are orthonormal and velocity-aligned including the
    /// straight-up degenerate case, and the draw is wired exactly as the ships'
    /// and plumes' are. What is **not**: a captured frame with a rocket in it.
    ///
    /// The headless capture path cannot produce one. `--race` gives a craft that
    /// holds the throttle and does not steer, so over 2400 ticks it never
    /// reaches a `Weapon Pad`, never gets a pickup, and the telemetry line never
    /// reports one held. A first attempt at this misread three pieces of
    /// **track scenery** as a fanned volley - they render identically in
    /// `time_trial`, where no rocket can exist, which is the check that settles
    /// it and the one to repeat before believing any future frame:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/on.png
    /// cargo run -p oag-game -- --race --mode time_trial  --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/off.png
    /// magick compare -metric AE /tmp/on.png /tmp/off.png null:
    /// ```
    ///
    /// Closing it wants a craft that can drive to a pad - the AI, or an input
    /// script replayed through `oag-trace run --script`.
    #[must_use]
    pub fn rocket_model_matrices(&self) -> Vec<Mat4> {
        self.world
            .projectiles
            .slots
            .iter()
            .filter(|projectile| projectile.kind.is_some())
            .map(|projectile| {
                let forward = projectile.velocity.normalize_or_zero();
                if forward == Vec3::ZERO {
                    return Mat4::from_translation(projectile.position);
                }
                // `Vec3::Y` is a poor reference exactly when the rocket is flying
                // straight up or down; `any_orthonormal_vector` is the fallback
                // rather than a silently degenerate basis.
                let reference = if forward.dot(Vec3::Y).abs() > 0.999 {
                    forward.any_orthonormal_vector()
                } else {
                    Vec3::Y
                };
                let side = forward.cross(reference).normalize_or_zero();
                let up = side.cross(forward);
                Mat4::from_cols(
                    side.extend(0.0),
                    up.extend(0.0),
                    forward.extend(0.0),
                    projectile.position.extend(1.0),
                )
            })
            .collect()
    }

    /// How far the ship moved since the last pad test, and the path it swept.
    ///
    /// One computation shared by both pad triggers, because the original's two -
    /// `Pads_TestCraft` and `WeaponPads_TestCraft` - run in the same craft
    /// update against the same recorded previous position. Computing it twice
    /// would be harmless today and wrong the moment either advanced the record,
    /// which is exactly the mistake `pad_previous_position.replace` invites.
    ///
    /// The sweep's destination is included and its origin is not, because the
    /// origin was the previous test's destination. A stationary ship and a long
    /// jump both fall through to the single point: interpolating a zero-length
    /// step adds nothing, and interpolating a long one does not close the gap.
    fn pad_sweep(&mut self, slot: usize, position: Vec3) -> (f32, Vec<Vec3>) {
        let previous = self.pad_previous_position[slot].replace(position);
        let moved = previous.map_or(f32::INFINITY, |from| position.distance(from));
        let sweep = match previous {
            Some(from) if (0.0..Self::PAD_SWEEP_LIMIT).contains(&moved) && moved > 0.0 => (1
                ..=Self::PAD_SWEEP_STEPS)
                .map(|step| from.lerp(position, step as f32 / Self::PAD_SWEEP_STEPS as f32))
                .collect(),
            _ => vec![position],
        };
        (moved, sweep)
    }

    /// Which way the speed pad under the ship pushes, or `None` if there is none.
    ///
    /// Reimplements `Pads_TestCraft` (`0x08887144`) and the two functions under
    /// it. Called once a tick with the position the tick *starts* at, and returns
    /// the direction `oag_physics`' step-15 term needs; see
    /// [`oag_physics::forces::Environment::pad_hit`], which is deliberately a
    /// per-tick containment answer rather than an entry edge.
    ///
    /// Also does the two things that happen on **entering a new** pad, because
    /// both are edges on the same value the original latches at `craft+0x1d0`:
    /// the Zone score, and arming the exhaust flare.
    ///
    /// `moved` and `sweep` come from [`Self::pad_sweep`] and are shared with
    /// [`Self::test_weapon_pads`], because the original's two trigger functions
    /// run in the same craft update against the same previous position.
    fn test_speedup_pads(
        &mut self,
        slot: usize,
        position: Vec3,
        moved: f32,
        sweep: &[Vec3],
    ) -> Option<Vec3> {
        if self.speedup_pads.is_empty() {
            return None;
        }

        let mut hit = None;
        for (index, pad) in self.speedup_pads.iter().enumerate() {
            // The broadphase. Spend the distance travelled, and skip until it is
            // used up. `moved` is infinite on the first tick, so every pad is
            // measured once before any of them is skipped.
            self.pad_distance[slot][index] -= moved;
            if self.pad_distance[slot][index] > 0.0 {
                continue;
            }

            // Measured from the destination, which is where the cache has to be
            // correct from for the next tick's subtraction to mean anything.
            self.pad_distance[slot][index] = pad.distance(position.to_array());

            // First pad wins. The cache above is still updated for every pad whose
            // turn it was, or a skipped one would keep a stale distance forever.
            if hit.is_none()
                && sweep.iter().any(|point| pad.contains(point.to_array()))
                && let Some(direction) = pad.direction()
            {
                hit = Some((index, Vec3::from_array(direction)));
            }
        }

        // Everything below is the *edge*, and it is deliberately outside the loop:
        // two pads overlapping on one tick is one entry, not two, matching the
        // original's single `craft+0x1d0` slot and its single `DAT_08b3435c` flag,
        // which `Zone_Update` (`0x0882f5cc`) consumes and clears once per tick.
        let entered = hit.map(|(index, _)| index);
        if entered != self.pad_current[slot] {
            self.pad_current[slot] = entered;
            if entered.is_some() {
                // Zone mode only. `Ship_ApplySpeedupPad` raises its flag under
                // the mode selector `zone-mode.md` identifies.
                if slot == 0 && self.world.race.mode == Mode::Zone {
                    self.world.race.score += oag_race::zone::SPEEDUP_PAD_SCORE;
                }
                // The visual, on the same edge and with the same **fixed**
                // duration the original uses. `ExhaustFlare_OnSpeedupPad`
                // (`0x08904f10`) is called from exactly here in
                // `Ship_ApplySpeedupPad`, inside its new-pad branch, and stores a
                // code literal - **not** `<SpeedupPads time>`. So the flare
                // outlives the force rather than expiring with it; see
                // `oag_render::exhaust::BOOST_SECONDS`, which is where the reason
                // is written down.
                //
                // **The craft that crossed the pad, whichever it is.** Every
                // racer carries its own `Exhaust`, so an opponent's boost shows
                // as an opponent's plume rather than as the player's - the
                // original arms `ExhaustFlare_OnSpeedupPad` from inside the
                // per-craft update, with that craft's own flare.
                self.exhaust[slot].boost(exhaust::BOOST_SECONDS);
            }
        }

        hit.map(|(_, direction)| direction)
    }

    /// Crosses the ship against the track's weapon pads, granting a pickup.
    ///
    /// Reimplements `WeaponPads_TestCraft` (`0x0888727c`), which
    /// `docs/ghidra/functions/psp-pulse-usa/pads.md` reads as "the exact mirror
    /// of `Pads_TestCraft`" for the `world+0x10c` list - same swept test, same
    /// per-racer distance cache. So the broadphase and the sweep below are
    /// [`Self::test_speedup_pads`]' verbatim, and what differs is only what
    /// happens on a hit.
    ///
    /// # Three things happen here and only one of them is recovered
    ///
    /// Worth separating at the call site rather than only in
    /// `docs/gameplay/pickups.md`, because they are easy to read as one ported
    /// branch:
    ///
    /// 1. **Stamping the pad** with `<WeaponPad refresh_time>` is the whole of
    ///    what the original's function does on a hit, at confidence 90. It is
    ///    done here on **any** hit, including one that grants nothing, because
    ///    that is what the original does - the stamp is unconditional.
    /// 2. **Granting a pickup at all** is ours. No pickup-grant call site has
    ///    been found anywhere, so a crossing handing something over is this
    ///    project's reading of what a weapon pad is for.
    /// 3. **Only granting into an empty slot** is ours too, and is the one rule
    ///    here with no evidence in either direction. A craft that already holds
    ///    something still stamps the pad, so a full inventory costs the pad its
    ///    cooldown - which is the conservative reading, and the one that cannot
    ///    hand out two pickups from one crossing.
    ///
    /// Called once a tick with the position the tick *starts* at, like the speed
    /// pad's, and **after** it, so both spend the same `moved` and neither can
    /// see a position the other has already advanced past.
    fn test_weapon_pads(&mut self, slot: usize, position: Vec3, moved: f32, sweep: &[Vec3]) {
        if self.weapon_pads.is_empty() {
            return;
        }

        // `WeaponPad_UpdateRefreshTimer` (`0x0892c034`): every pad counts its own
        // stamp down by `dt`, floored at zero, whether or not anything is near
        // it. Outside the hit loop because the original runs it as the class's
        // `update` method on every pad, every tick.
        // **Once a tick, not once a craft.** The timer belongs to the pad, so
        // the first craft through it is the one that runs the countdown and the
        // other seven read what it left. Slot 0 is stepped first, every tick.
        if slot == 0 {
            for left in &mut self.weapon_pad_refresh_left {
                *left = (*left - self.dt).max(0.0);
            }
        }

        let mut hit = None;
        for (index, pad) in self.weapon_pads.iter().enumerate() {
            self.weapon_pad_distance[slot][index] -= moved;
            if self.weapon_pad_distance[slot][index] > 0.0 {
                continue;
            }
            self.weapon_pad_distance[slot][index] = pad.distance(position.to_array());

            if hit.is_none() && sweep.iter().any(|point| pad.contains(point.to_array())) {
                hit = Some(index);
            }
        }

        // The edge, outside the loop for the same reason the speed pad's is: two
        // pads overlapping on one tick is one entry.
        if hit == self.weapon_pad_current[slot] {
            return;
        }
        self.weapon_pad_current[slot] = hit;
        let Some(index) = hit else {
            return;
        };

        // Point 1 above. Unconditional, and before the grant, so a pad that
        // hands nothing over is still spent.
        if self.weapon_pad_refresh_left[index] > 0.0 {
            // Still cooling down. The stamp is not refreshed - re-stamping would
            // make a craft parked on a pad hold it inert forever, and the
            // original cannot reach this branch at all: its timer is what
            // `Pad_ContainsPoint` is asked about before the hit is reported.
            return;
        }
        self.weapon_pad_refresh_left[index] = self.weapon_pad_refresh;

        // Points 2 and 3.
        if !self.world.ships[slot].pickup.is_empty() {
            return;
        }
        let Some(weapons) = self.weapons.as_ref() else {
            return;
        };
        let Some(table) = oag_gameplay::pickup::table_for(weapons, to_format_class(self.class))
        else {
            return;
        };
        // **The `human` column for slot 0 and the `ai` column for the rest**,
        // which is a distinction the shipped `<Pickupodds>` table draws itself -
        // see `docs/gameplay/pickups.md`. `front`/`back` are still unreachable:
        // they need race *positions*, and `oag_race::RaceState` is single-ship,
        // so nobody is placed.
        let who = if slot == 0 {
            oag_gameplay::pickup::Driver::Human
        } else {
            oag_gameplay::pickup::Driver::Ai
        };
        let drawn = oag_gameplay::pickup::draw(&mut self.world.rng, table, who);
        self.world.ships[slot].pickup.weapon = drawn;
    }

    /// Whether this opponent has been away from its own driver's idea of where
    /// it is for long enough to count as lost.
    ///
    /// Advances the dwell counter as a side effect, so it must be called once
    /// per craft per tick and not conditionally - a caller that skipped it
    /// while a cooldown ran would let a craft bank dwell it never spent.
    ///
    /// See [`RESCUE_HALF_WIDTHS`] for why this exists next to the reset volumes
    /// rather than instead of them.
    fn lost_off_the_circuit(&mut self, slot: usize) -> bool {
        let ship = &self.world.ships[slot];
        let index = ship.driver.index as usize;
        let away = ship
            .physics
            .body
            .position
            .distance(self.racing_line.point(index))
            > self.rescue_distance;
        self.lost_ticks[slot] = if away {
            self.lost_ticks[slot].saturating_add(1)
        } else {
            0
        };
        // An empty line puts every point at the origin, so a track with no
        // spline would read every craft as lost and respawn it onto nothing.
        //
        // The same two guards `reset_zone_touched` applies, applied *after* the
        // counter so the dwell is still measured while they hold.
        !self.racing_line.is_empty()
            && !self.respawn_disabled[slot]
            && self.respawn_cooldown[slot] == 0
            && self.lost_ticks[slot] >= RESCUE_TICKS
    }

    /// Whether this tick ended in contact with `Reset` geometry.
    ///
    /// Suppressed while the cooldown runs and once respawning has given up, so
    /// the two guards live in one place rather than at the call site.
    fn reset_zone_touched(&self, slot: usize, env: &Environment, before: Vec3) -> bool {
        if self.respawn_disabled[slot] || self.respawn_cooldown[slot] > 0 {
            return false;
        }
        let ship = &self.world.ships[slot];
        // The same `env` the force law just ran with, so the self-collider
        // exclusion cannot differ between the two.
        oag_physics::reset::contact(&ship.physics, &ship.handling, env, &self.collision, before)
            .is_some()
    }

    /// Puts the ship back on the track after a `Reset` contact.
    ///
    /// # This pose is a guess, not a reading
    ///
    /// `docs/ghidra/functions/psp-pulse-usa/collision.md` records **that** a `Reset`
    /// contact respawns the ship, at confidence 86. **Where it respawns it is not
    /// recorded anywhere**, and searching the RE tree for it found nothing - which
    /// is itself the finding. So this reuses the initial spawn: the racing line at
    /// [`spawn_height`], on the spline sample nearest where the ship was before
    /// the tick that triggered the reset.
    ///
    /// Confidence **40**. That is deliberately low, and the number matters: this
    /// is a placeholder chosen because it reuses code that is already correct for
    /// the race start, not because anything says the original does it. The likelier
    /// real mechanism is a last-passed checkpoint or track section - [`Ship::segment`]
    /// is the field that would hold it, and nothing populates it meaningfully today.
    /// Whoever recovers that should replace this outright rather than tune it.
    ///
    /// The velocity, orientation and every control state go to zero, because
    /// [`Ship::place_at`] resets the whole physics state and keeps only mass and
    /// inertia. Whether the original preserves any speed through a respawn is also
    /// unrecorded.
    fn respawn(&mut self, slot: usize, sample_index: Option<usize>) {
        // **The fallback is the lap's first sample, not the table's.** They are
        // the same on every circuit the disc ships, and this arm fires exactly
        // when the caller had no index to give - which for an opponent means
        // `ai_order` was empty, the one case where "sample zero" has no
        // relationship to any lap at all.
        let sample = sample_index
            .and_then(|index| self.spline.sample(index))
            .or_else(|| self.ai_sample(0))
            .or_else(|| self.spline.start());
        let Some(sample) = sample.copied() else {
            return;
        };

        let ship = &mut self.world.ships[slot];
        let height = spawn_height(&ship.handling);
        let pose = Pose::from_sample(&sample, sample.racing_line, height);
        ship.place_at(pose);

        // **The driver has to be told where it has been put.** `Driver::drive`
        // locates a craft in a 48-sample window around its last index, so a
        // teleport of more than that leaves the driver steering at the piece of
        // track the craft fell off - the same failure the grid had when
        // `Driver::index` started at zero, except mid-race and invisible,
        // because a craft that recovered and then drove into the scenery reads
        // as bad driving rather than as a lost index.
        ship.driver.index = self
            .racing_line
            .nearest(pose.position, 0, self.racing_line.len()) as u32;

        self.respawns[slot] += 1;
        self.respawns_in_a_row[slot] += 1;
        self.respawn_cooldown[slot] = RESPAWN_COOLDOWN_TICKS;
        // The craft is back on its line by definition, so the dwell starts
        // again rather than carrying over and rescuing it a second time on the
        // tick the cooldown expires.
        self.lost_ticks[slot] = 0;

        // Otherwise the ribbon spans the teleport: ten samples of history from
        // wherever the craft fell off, stretched across the track to where it was
        // put back. The camera is snapped for the same reason, and only for the
        // player, who is the only craft one is flown from.
        self.exhaust[slot].clear_trail();

        if self.respawns_in_a_row[slot] >= RESPAWN_GIVE_UP {
            self.respawn_disabled[slot] = true;
            eprintln!(
                "reset: craft {slot} respawned {} times in a row without getting \
                 clear, giving up on it. The recovery pose is probably inside a \
                 Reset volume; see Race::respawn.",
                self.respawns_in_a_row[slot]
            );
        }
    }

    /// What the ship did, as of the last tick.
    #[must_use]
    pub fn telemetry(&self) -> Telemetry {
        let ship = self.ship();
        let position = ship.physics.body.position;
        let (spline_distance, height_above_spline) =
            self.spline
                .nearest(position)
                .map_or((f32::NAN, f32::NAN), |(_, sample, distance)| {
                    let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
                    (distance, (position - Vec3::from_array(sample.pos)).dot(up))
                });

        Telemetry {
            tick: self.world.tick,
            position,
            speed: ship.physics.body.linear_velocity.length(),
            pickup: ship.pickup.weapon,
            projectiles: self.world.projectiles.live(),
            grounded: ship.physics.grounded,
            spline_distance,
            height_above_spline,
        }
    }

    /// Whether the ship is pointing back down the track.
    ///
    /// `dot(craft_forward, sample.tangent) < 0`. **The tangent, not the sample
    /// index** - `HANDOVER.md` records index order as unusable for this, because
    /// `Spline::from_track` concatenates paths in file order rather than travel
    /// order, and on a slow capture most windows step by zero. The dot product has
    /// neither failure mode and reads `+0.9999` against `-0.9999`.
    ///
    /// [`ai_order`] did not change that. It gives the *drivers* a travel-ordered
    /// view; this reads `Spline::nearest`, which is still the whole table in
    /// file order, and has to be - the player can be anywhere the track is,
    /// including on a branch the lap never drives.
    ///
    /// `false` when the ship is not near the spline at all, which is the safe way
    /// round: a spurious warning is worse than a missing one.
    #[must_use]
    pub fn wrong_way(&self) -> bool {
        let ship = self.ship();
        let Some((_, sample, _)) = self.spline.nearest(ship.physics.body.position) else {
            return false;
        };
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        ship.physics.body.forward().dot(tangent) < 0.0
    }

    /// What the HUD shows, as of the last tick.
    ///
    /// Separate from [`Telemetry`], which is a log line and carries diagnostics
    /// no player sees. The fields with no source yet are left at their "unknown"
    /// value rather than filled with a plausible number - `lap` and `place` read
    /// zero, and [`crate::hud`] omits a widget rather than claiming a value it does
    /// not have. See `docs/ui/hud.md`.
    #[must_use]
    pub fn readout(&self) -> crate::hud::Readout {
        let ship = self.ship();
        let race = &self.world.race;
        // Zero still means "unknown", and a track with no closed ring still has
        // no lap counter - the widget is omitted rather than reading 1 of 3 on a
        // course that cannot tell.
        let counted = self.course.is_some();
        crate::hud::Readout {
            speed_kmh: ship.physics.body.linear_velocity.length()
                * oag_render::exhaust::SPEED_TO_KMH,
            speed_full_kmh: crate::hud::DEFAULT_SPEED_FULL_KMH,
            // The ship's own pool, which wall contact spends and absorbing a
            // pickup pays back into - `oag_physics::damage`. (This comment said
            // "nothing depletes this yet" until 2026-08-11, three subsystems
            // after it stopped being true.)
            shield: ship.physics.shield,
            shield_max: ship.handling.dimensions.shield,
            lap: if counted { race.lap } else { 0 },
            // A speed lap and a Zone run have no lap target. Zero is what the HUD
            // already reads as "unknown" and it omits the "of N" half.
            laps: race.laps_target.filter(|_| counted).unwrap_or(0),
            // The player's place in the field, from the same table the results
            // will read - [`Self::player_place`], slot 0 of [`Self::places`].
            //
            // Two gates, both meaning "there is no place to report" rather than
            // "the place is one". `counted`, because with no closed ring there is
            // no distance round the circuit to order the field by and
            // [`Self::places`] answers "everyone is first" - honest as a return
            // value and a lie on screen. And a field of **more than one craft**,
            // because a place is a position among opponents: a time trial, a speed
            // lap and a Zone run all grid the player alone, and so does a single
            // race on a track with no authored `Start Position` node. `1 / 1` is
            // arithmetic rather than a standing, and the widget group is omitted
            // the same way the lap group is on a track that cannot count.
            place: if counted && self.world.ship_count > 1 {
                u32::from(self.player_place())
            } else {
                0
            },
            ships: u32::from(self.world.ship_count),
            race_ticks: self.world.tick,
            lap_ticks: if counted {
                race.lap_ticks(self.world.tick)
            } else {
                self.world.tick
            },
            best_lap_ticks: race.best_lap_ticks,
            wrong_way: self.wrong_way(),
            zone: race.zone.into(),
            score: race.score,
            pickup: ship.pickup.weapon,
        }
    }

    /// The **player's** exhaust animation state.
    ///
    /// Slot 0, which is what every capture and every pinned exhaust number is
    /// taken against. [`Self::exhaust_of`] is the one to reach for when drawing
    /// the field.
    #[must_use]
    pub fn exhaust(&self) -> &Exhaust {
        &self.exhaust[0]
    }

    /// One craft's exhaust animation state.
    ///
    /// Indexed by ship slot, the player at 0. A slot past [`Self::ship_count`]
    /// holds a cold `Exhaust` that nothing advances - drawn, it would be an
    /// invisible flare at whatever pose an unused ship slot carries, so a caller
    /// iterating the field bounds itself by `ship_count` the way the hull draw
    /// does.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn exhaust_of(&self, slot: usize) -> &Exhaust {
        &self.exhaust[slot]
    }

    /// Held throttle on the original's `0..=100` scale, for
    /// [`Race::force_boost_state`]'s synthetic warmup.
    ///
    /// **This constant exists because a `1.0` here was a 100x bug**, found by
    /// the first numeric comparison of our exhaust state against a capture's
    /// (`oag-game --trace-out` against `data/traces/pad0-boost.csv`). The live
    /// path at [`Race::tick`] passes `ship.thrust`, and
    /// `oag_physics::ship::Ship::thrust` is documented as the raw `0..=100`
    /// throttle the original stores at `craft+0x2b8` - the capture reads
    /// `throttle = 100` at the compared tick. Passing `1.0` charged the boost
    /// accumulator at `1/3000` per tick instead of `100/3000`, so a posed frame
    /// never left the speed ramp's floor: `0.5436` against the original's
    /// saturated `1.0000`, an error that did **not** move when the pose age was
    /// corrected, which is what proved it was the code rather than the
    /// parameter.
    ///
    /// A named constant rather than a bare `100.0` so the scale is stated where
    /// it is used; the two are easy to confuse precisely because
    /// `ShipControls::thrust` on the *input* side is `0.0..=1.0`.
    const FULL_THRUST: f32 = 100.0;

    /// Drives the exhaust to the state a speed pad entry `age` seconds ago
    /// would leave it in - `CaptureOptions::pose_boost`.
    ///
    /// Replays [`Exhaust::advance`] at the fixed step rather than poking
    /// fields: enough ticks of full thrust to reach `entry_intensity`,
    /// [`Exhaust::boost`] on the entry edge, then `age` more seconds of the
    /// same advance, so the flare size, the plume reveal timer and the flicker
    /// generator all sit exactly where a real crossing puts them.
    ///
    /// `entry_intensity` is `None` for a saturated ramp, which is what a craft
    /// that has been racing for four seconds or more actually has. **Pass the
    /// measured value when comparing against a capture taken after a
    /// teleport**: `psp-drive.py place` leaves the flare's ramp wherever the
    /// craft's idle time left it. That is not a cosmetic difference -
    /// intensity sets the flare's resting half-size through
    /// `(i * 0.6 + 0.4) * 2.5` (`1.2` against a saturated `2.5`) and every one
    /// of the ribbon's three staggered layer alphas - so a saturated render
    /// against an unsaturated capture differs in *state* before it differs in
    /// anything a renderer does.
    ///
    /// Matching only the entry value is enough to match the whole curve,
    /// because the ramp climbs at the same recovered `0.25`/s on both sides
    /// through the posed age. **That rate is now confirmed numerically**: the
    /// capture's own per-tick rise reads `0.0041708`, and `0.25 / 59.94` is
    /// `0.0041708`.
    ///
    /// **Pass the intensity of the tick *before* the pad fires, not the entry
    /// tick's.** [`Exhaust::boost`] arms the timer and the *next* `advance`
    /// is the entry tick - it both decays `boost_timer` by one step and raises
    /// intensity by one step, which is exactly what the capture's entry row
    /// already shows (`boost_timer` reads `0.783316`, i.e. `0.8 - dt`, not
    /// `0.8`). So passing the entry row's own intensity counts that tick twice.
    /// Measured on `pad0-boost.csv` tick 62: the entry row's `0.1292277` lands
    /// `+0.0080` high, the row before it (`0.1250567`) lands `+0.0003` - and
    /// that remainder is the fixed-60 Hz against variable-59.94 Hz difference
    /// [ADR-0007](../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)
    /// accepts, not a model error.
    ///
    /// `speed` is `None` for the racing `120` units/s. It reaches the picture
    /// only through the speed ramp's floor on the boost accumulator, so it
    /// matters when a capture's speed is far from that.
    pub fn force_boost_state(
        &mut self,
        age: f32,
        entry_intensity: Option<f32>,
        speed: Option<f32>,
    ) {
        let speed = speed.unwrap_or(120.0);
        // Whole ticks, then a **short final tick** for the remainder, so the
        // ramp lands exactly on `target` instead of up to one tick past it.
        //
        // This used to be `ceil`, and the overshoot was real: the first numeric
        // comparison against a capture measured `intensity` `+0.0041` high, one
        // whole tick of `INTENSITY_RISE * dt`, purely from rounding the warmup
        // up. Still a replay of `advance` rather than a poked field - the
        // principle this function is built on - because a shortened step is
        // something the original's own variable timestep does anyway.
        let target = entry_intensity.unwrap_or(1.0).clamp(0.0, 1.0);
        let ticks = target / exhaust::INTENSITY_RISE / self.dt;
        // **The player's, and only the player's.** A pose comparison is against
        // one captured craft; posing the whole field would put seven opponents
        // into a boost the capture says nothing about.
        let (player, rng) = (&mut self.exhaust[0], &mut self.exhaust_rng[0]);
        for _ in 0..(ticks.floor() as u32) {
            player.advance(self.dt, Self::FULL_THRUST, speed, rng);
        }
        let remainder = (ticks - ticks.floor()) * self.dt;
        if remainder > 0.0 {
            player.advance(remainder, Self::FULL_THRUST, speed, rng);
        }
        player.boost(exhaust::BOOST_SECONDS);
        let aged = (age / self.dt).round() as u32;
        for _ in 0..aged {
            player.advance(self.dt, Self::FULL_THRUST, speed, rng);
        }

        // **A posed frame otherwise has no ribbon at all, and that silently
        // wrecks a boost comparison.** `Trail_DrawRibbon` refuses to draw until
        // its ring is full, our [`Exhaust::trail_ready`] reproduces that, and a
        // `--pose-from --ticks 0` capture never runs a tick that would push a
        // sample - so every posed frame before this was missing the one element
        // that carries the exhaust's colour. Measured on the first real pad
        // capture: the original's rear reads magenta (mean `(224, 164, 217)`,
        // peaks near `(252, 109, 214)`, red and blue both far above green) over
        // 6,355 pixels of one frame, against 491 pixels of blue-dominant violet
        // `(180, 141, 229)` in ours - and most of that difference was the
        // missing ribbon, not the flare or the plume.
        //
        // The history is laid down straight, back along the nozzle's own axis at
        // `speed * dt` per sample, oldest pushed first. That is exactly what a
        // real run produces here: a speed pad sits on a straight, and over the
        // ten ticks a full ring spans the captured positions are collinear to
        // far below a pixel. It is a *capture* approximation and nothing in the
        // game uses it - a real race pushes one true sample per tick from
        // `Race::tick`.
        if let Some(nozzle) = self.nozzle() {
            let back = -self.ship().physics.body.forward();
            self.exhaust[0].clear_trail();
            for k in (0..exhaust::TRAIL_SAMPLES).rev() {
                let behind = back * (speed * self.dt * k as f32);
                self.exhaust[0].push_trail(nozzle + behind, back);
            }
        }
    }

    /// The collision sparks' geometry this frame, split by blend class.
    ///
    /// The pool and the effect it plays are handed out together because
    /// neither means anything alone - a particle carries an index into the
    /// effect's emitters rather than a copy of their parameters. Empty when
    /// the disc's own effect did not load.
    #[must_use]
    pub fn spark_vertices(
        &self,
        right: Vec3,
        up: Vec3,
    ) -> (
        Vec<oag_render::mesh::GpuVertex>,
        Vec<oag_render::mesh::GpuVertex>,
    ) {
        self.effects
            .get(sparks::DAMAGE_EFFECT)
            .map(|effect| self.sparks.vertices(effect, right, up))
            .unwrap_or_default()
    }

    /// Everything the [`psys::Stage`] is playing this frame, split by blend
    /// class the same way [`Self::spark_vertices`] is.
    ///
    /// The rocket flares and the detonations today. Uploaded through the same
    /// [`oag_render::psys::Pipeline`] as the sparks - one pass, two buffers,
    /// no third pipeline per effect.
    #[must_use]
    pub fn stage_vertices(
        &self,
        right: Vec3,
        up: Vec3,
    ) -> (
        Vec<oag_render::mesh::GpuVertex>,
        Vec<oag_render::mesh::GpuVertex>,
    ) {
        self.stage.vertices(right, up)
    }

    /// The pool the rocket effects play in.
    #[must_use]
    pub fn stage(&self) -> &psys::Stage {
        &self.stage
    }

    /// Whether this source authors an engine flare as a particle effect, and
    /// it loaded.
    ///
    /// The renderer asks this to decide whether to draw
    /// [`oag_render::exhaust`]'s procedural flare quad: drawing both would
    /// put an invented glow on top of the authored one, which is the exact
    /// thing `CLAUDE.md`'s do-not-invent rule forbids. The trail ribbon and
    /// the boost plume are unaffected - the PS2's effect is the *flare*, and
    /// neither of those has an authored counterpart on either disc.
    #[must_use]
    pub fn engine_flare_effect(&self) -> bool {
        self.effects.get(ENGINE_FLARE_EFFECT).is_some()
    }

    /// How many spark bursts the contact rule has triggered.
    ///
    /// Independent of whether an effect was loaded to play them - see
    /// [`Self::sparks_ignitions`].
    #[must_use]
    pub fn spark_ignitions(&self) -> u32 {
        self.sparks_ignitions
    }

    /// The collision sparks' current particle pool.
    #[must_use]
    pub fn sparks(&self) -> &psys::System {
        &self.sparks
    }

    /// The track's speed-pad trigger volumes.
    ///
    /// Empty on a track that authors none, which is an ordinary state rather than
    /// a failure - see [`Setup::speedup_pads`].
    #[must_use]
    pub fn speedup_pads(&self) -> &[oag_formats::pads::PadVolume] {
        &self.speedup_pads
    }

    /// The player's `engine_flare` locator in **world** space, or `None` when the
    /// ship model carries no `Engine Flare` node.
    ///
    /// Composed through [`Race::ship_model_matrix`], which is the only correct
    /// route: the locator is authored in `.vex` model space, so it has to pick up
    /// [`MODEL_YAW`] exactly as the hull's vertices do. A nozzle built from
    /// `body.forward()` instead would be a half-turn out and would sit on the
    /// ship's nose.
    #[must_use]
    pub fn nozzle(&self) -> Option<Vec3> {
        self.nozzle_of(0)
    }

    /// One craft's `engine_flare` locator in **world** space.
    ///
    /// [`Self::nozzle`] one slot over. The *model-space* locator is the same for
    /// every craft while the whole field wears the player's hull; what differs is
    /// the matrix it is carried through, which is that craft's own pose.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn nozzle_of(&self, slot: usize) -> Option<Vec3> {
        let local = self.nozzles.get(slot).copied().flatten()?;
        Some(model_matrix_of(&self.world.ships[slot]).transform_point3(local))
    }

    /// Where the camera is and what it is aimed at, as a view matrix.
    ///
    /// This is the one place a [`CameraOverride`] takes effect: everything else
    /// that needs the camera - the PVS eye, the fog eye - derives from this
    /// matrix through [`Self::camera_position`], so overriding here overrides
    /// everywhere at once, and a consumer that read the chase camera directly
    /// instead would silently miss the override.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        if let Some(over) = &self.camera_override {
            // A view matrix is the inverse of the camera's world transform.
            return Mat4::from_rotation_translation(over.orientation, over.eye).inverse();
        }
        let target = target_of(self.ship());
        if self.view == crate::display::CameraView::Internal {
            // Rigid, so there is no per-tick state to advance and nothing to
            // snap: the cockpit is bolted to the hull.
            return oag_render::camera::internal::view(target, &self.internal_params);
        }
        self.camera.view(target, &self.chase_params)
    }

    /// The camera's own world position, from the view matrix it produces.
    ///
    /// A view matrix is the inverse of the camera's world transform, so
    /// inverting it back and taking the translation column is the camera's
    /// position. Read out rather than tracked separately so it cannot drift
    /// from what is actually being rendered.
    #[must_use]
    pub fn camera_position(&self) -> Vec3 {
        self.view().inverse().w_axis.truncate()
    }

    /// The authored sections the craft and the camera are in, for PVS culling.
    ///
    /// [`UNPLACED`] means "do not trust this", which the visibility set turns
    /// into *draw everything*. Three things produce it, and all three are
    /// states where culling to a section would be most likely to be visibly
    /// wrong:
    ///
    /// - **The track has no spline**, so nothing can be located at all.
    /// - **The point is off the track**, more than
    ///   [`OFF_TRACK_HALF_WIDTHS`] half-widths from the nearest sample. This is
    ///   the case that matters: a craft that has fallen off, gone airborne over
    ///   a gap or been knocked into scenery is usually still inside some
    ///   authored box and would otherwise get a confidently wrong answer. See
    ///   that constant for why the lookup's own out-of-range fallback does not
    ///   cover this.
    /// - **A respawn is in flight** ([`RESPAWN_COOLDOWN_TICKS`]), where the
    ///   craft teleports and the camera spring is still catching up, so neither
    ///   position describes the shot for several frames.
    ///
    /// The craft and the camera are located independently, so the common case
    /// of a camera swinging off the racing line while the craft is fine
    /// degrades only the camera's half.
    #[must_use]
    pub fn visibility_sections(&self) -> (u8, u8) {
        // The player's, because this places the player's camera. An opponent
        // recovering across the circuit must not blank the shot.
        if self.respawn_cooldown[0] > 0 {
            return (UNPLACED, UNPLACED);
        }
        let ship = self.ship();
        // The simulation already located the craft this tick and left the
        // sample index in `Ship::segment`, so the craft's half is a table read
        // rather than a second scan of the whole track. Doing it again here
        // would cost more per frame than the culling it enables saves.
        // `u16::MAX` is the never-located sentinel, and it is out of range of
        // any real table, so the bounds check covers both.
        let index = usize::from(ship.segment);
        if index >= self.spline.len() {
            return (UNPLACED, UNPLACED);
        }
        let craft = self.section_of(index, ship.physics.body.position);
        // The camera is a chase spring a few units behind, so it is a few
        // samples away in the same table. See `Spline::nearest_within` for why
        // a local search is allowed to miss.
        let camera =
            match self
                .spline
                .nearest_within(self.camera_position(), index, CAMERA_SEARCH_SAMPLES)
            {
                Some((camera_index, _, _)) => self.section_of(camera_index, self.camera_position()),
                None => UNPLACED,
            };
        (craft, camera)
    }

    /// The section of the sample at `index`, or [`UNPLACED`] when `position` is
    /// too far from it to trust. See [`Self::visibility_sections`].
    fn section_of(&self, index: usize, position: Vec3) -> u8 {
        let Some(sample) = self.spline.sample(index) else {
            return UNPLACED;
        };
        let distance = (Vec3::from_array(sample.pos) - position).length();
        let half_width = sample.half_width_left.max(sample.half_width_right);
        if distance > half_width * OFF_TRACK_HALF_WIDTHS {
            return UNPLACED;
        }
        sample.section_id
    }

    /// The projection for a viewport of the given aspect ratio, with the
    /// player's field-of-view setting applied.
    ///
    /// **`<ExternalCameraFar fov>`'s unit is vertical degrees**, settled
    /// 2026-08-09 at confidence 94 against `g_camera_fov_degrees`
    /// (`0x08b34310`) on the running game, and read here as such.
    /// `oag_render::camera::projection` still names its parameter
    /// `fov_radians` precisely so that the conversion has to be written at a call
    /// site rather than assumed in a library. The value read from the disc is
    /// printed in the load report, so the assumption is checkable by whoever looks.
    ///
    /// The value is also authored for **one** viewport shape, the PSP's own, which
    /// is the only one the original ever renders into. A window narrower than that
    /// goes through [`oag_render::camera::fit_vertical_fov`], which holds the
    /// horizontal field the authored shape would have had instead of cropping the
    /// sides away; at the PSP's aspect and anything wider it is the authored value
    /// unchanged. That is a presentation choice and is documented as one.
    ///
    /// `setting` is a **percentage of that authored value** and is applied
    /// *before* the fit, so the two compose in the order a reader would expect:
    /// the player widens the field the disc asked for, and the result is then
    /// fitted to whatever shape the window is. At
    /// [`Fov::AUTHORED`](crate::display::Fov::AUTHORED) - the default - nothing
    /// is applied at all and this is the projection every capture under
    /// `data/traces/` was taken with.
    ///
    /// `near` is 1.0 rather than something tiny: track half-widths are tens of
    /// units, so a near plane at 0.01 spends the depth range on nothing and
    /// z-fights in the distance.
    ///
    /// # The field widens with speed, and that is the original's own behaviour
    ///
    /// `Ship_UpdateCameraRigs` adds `craft+0x790` to both tripod fovs every
    /// frame, and that term is `0.075 * dot(forward, velocity)` **additive
    /// degrees**. Implemented here as [`SPEED_FOV_GAIN_DEG`], which carries the
    /// evidence: verified live against `g_camera_fov_degrees` to a worst
    /// residual of 0.0007 degrees over 19 samples. See
    /// `docs/rendering/projection-vs-the-original.md`.
    ///
    /// [`Self::boost_kick`] is a different thing and is **not** a port of it:
    /// the original's term is present with no boost at all, is driven by speed
    /// rather than by a pad, and adds degrees where `BoostFovKick` multiplies a
    /// tangent. The two compose, in that order.
    ///
    /// **A matched-pose comparison still needs `--camera-fov`.**
    /// [`oag_gameplay::spawn::Ship::place_at`] resets the body, so a posed craft
    /// has zero velocity and this term contributes exactly `0` - the posed frame
    /// renders at the authored field whatever the captured tick's speed was.
    /// Pass the fov computed from that tick's own forward velocity. Any
    /// comparison taken before 2026-08-09 is misregistered by 1.12-1.26x
    /// depending on the tick's speed and has to be re-taken.
    #[must_use]
    pub fn projection(&self, aspect: f32, far: f32, setting: crate::display::Fov) -> Mat4 {
        // An overridden fov stands in for the authored one and still passes
        // through the player's setting, whose default is identity; it exists to
        // match a captured frame's own field exactly, so it must sit at the
        // same point in the chain as the value it replaces. (It settled the
        // authored unit once; that unit is now recovered, and the flag's job is
        // matched-pose comparison - see `Options::fov_deg`.)
        // Each view authors its own fov, in the same unit at the same aspect, so
        // the live one is whichever perspective is being rendered - a cockpit
        // framed with the chase view's field would be the wrong picture with the
        // right camera.
        let view_fov = if self.view == crate::display::CameraView::Internal {
            self.internal_params.fov
        } else {
            self.chase_params.fov
        };
        let authored_fov = self
            .camera_override
            .as_ref()
            .and_then(|over| over.fov_deg)
            .unwrap_or(view_fov);
        // The original's speed widen, added to the authored degrees before
        // anything else - which is where `Ship_UpdateCameraRigs` adds it, and it
        // is additive degrees rather than a tangent multiplier. Composed
        // *before* the player's setting so a widened field scales the whole
        // thing proportionally, the same order the boost kick argues for below.
        //
        // It is deliberately not skipped for a `camera_override`: the override
        // stands in for the authored value and this is a separate physical
        // effect. It costs nothing in the calibration path either, because
        // `Ship::place_at` resets the body, so a posed craft has zero velocity
        // and the term is exactly `0` - which is why matching a captured frame
        // still means passing `--camera-fov` computed from that frame's own
        // forward velocity.
        let body = &self.ship().physics.body;
        let forward_speed = body.forward().dot(body.linear_velocity);
        let widened_deg = (authored_fov + SPEED_FOV_GAIN_DEG * forward_speed)
            .clamp(FOV_GUARD_DEG.0, FOV_GUARD_DEG.1);
        let authored = setting.apply(widened_deg.to_radians());
        // Composed *after* the setting, so a player who has widened the field
        // gets the same proportional kick rather than a fixed number of degrees.
        // The zero case returns the input untouched rather than through
        // `atan(tan(x))`, for the reason `Fov::apply` gives: every capture taken
        // before this effect existed has to stay bit-identical.
        let kicked = if self.boost_kick == 0.0 {
            authored
        } else {
            let widen = 1.0 + self.boost_kick * self.boost_fov_kick.gain();
            2.0 * ((authored * 0.5).tan() * widen).atan()
        };
        let fov = oag_render::camera::fit_vertical_fov(kicked, AUTHORED_ASPECT, aspect);
        oag_render::camera::projection(fov, aspect, 1.0, far)
    }

    /// Where the ship is and how it is oriented, as the mesh shader's model matrix.
    ///
    /// Rotation, translation and the craft's recovered global scale. The scale is
    /// **uniform**, so the shader's assumption - which is what lets it rotate
    /// normals without an inverse transpose - still holds. The model's own origin
    /// is used as it is: no recentring, and the only rotation composed in is
    /// [`MODEL_YAW`], which is a stated finding about the `.vex` and not a nudge
    /// to make the picture look right.
    ///
    /// # The `0.75` is recovered, confirmed three ways, and shared with physics
    ///
    /// `g_craft_scale` (`0x08ab0e1c`) is a code literal `0.75` written once in
    /// `Craft_Construct_q` and read by seventeen sites. Three of them establish
    /// that it belongs on the **render** matrix and not only in the simulation:
    ///
    /// 1. **The craft's world matrix rows carry it**, live-measured: the
    ///    per-sample trail direction `Exhaust_Update` stores is `200000.0` times
    ///    the craft's row 2, and it reads back `150,080` -
    ///    `200000 * 0.75 = 150,000`. See
    ///    `oag_render::exhaust::CRAFT_ROW_SCALE`.
    /// 2. **The drop-shadow divides it back out.** `FUN_089038c8` - a stencil
    ///    shadow-volume pass - computes its ground projection and then scales it
    ///    by `1.0 / g_craft_scale` before `Gu_SetMatrix(2, ...)` installs the
    ///    node's own 4x4 as the GE world matrix. Undoing the scale is only
    ///    correct if the matrix it draws under has it.
    /// 3. **Physics already applies it**, as
    ///    `oag_physics::hover::TARGET_GLOBAL_SCALE` (confidence 92), and
    ///    `Ship_HoverFourCorner`, `Ship_InitCraft`, `Ship_UpdateCraft`,
    ///    `Ship_UpdateCameraRigs` and the `<Misc>` hull dimensions into the
    ///    collider all read the same global. A simulation scaled by `0.75` and a
    ///    mesh drawn at `1.0` is the one combination that is certainly wrong.
    ///
    /// The rigid body's own basis rows are orthonormal - the capture harness
    /// measures them unit-length over 200 ticks - so the scale is not already
    /// arriving through `body.orientation`, and this is where it belongs.
    ///
    /// # There is no residual: the craft is the right size, and the old gap was the projection
    ///
    /// This comment used to record a further `~1.3x` gap and name
    /// `1/0.75^2 = 1.7778` as the live hypothesis for it. **That is refuted, and
    /// the constant here must not be widened to swallow anything.**
    ///
    /// Measured 2026-08-09 by similarity registration on high-passed gradient
    /// magnitude - no brightness threshold anywhere in it - against 16 emulator
    /// frames: **craft-only boxes register at `0.9989`/`0.9985`/`0.9987` once the
    /// background is zeroed, so the mesh is correct to 0.15 %.** The old figure
    /// came from a lamp-centroid metric that is not linear in the scale it
    /// measures (`1.0` -> `1.740`, `0.75` -> `1.191`, `0.5625` -> `1.067`), and
    /// from one false clause: that measurement was recorded as taken "with the
    /// track and buildings behind aligning", and they do not align. Far scenery,
    /// which no mesh scale can move, registers at `1.1195` in the same frame.
    ///
    /// What is actually missing is in [`Race::projection`], not here: the
    /// original adds `0.075 * dot(forward, velocity)` **degrees** to both tripod
    /// fovs every frame, so its field widens with speed and ours does not. See
    /// `docs/rendering/projection-vs-the-original.md`.
    ///
    /// What ships here is the confirmed constant, applied once - which turns out
    /// to be the whole of it.
    #[must_use]
    pub fn ship_model_matrix(&self) -> Mat4 {
        self.ship_model_matrix_of(0)
    }

    /// One craft's model matrix, by ship slot.
    ///
    /// [`Self::ship_model_matrices`] gives the same matrices for drawing the
    /// field, but it *filters* inactive craft, so its indices are not slot
    /// indices. Anything that has to line a craft's slot up with something else
    /// held per slot - its plume, its flare - asks for the matrix by slot here.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn ship_model_matrix_of(&self, slot: usize) -> Mat4 {
        model_matrix_of(&self.world.ships[slot])
    }

    /// How many craft are in play, the player included.
    #[must_use]
    pub fn ship_count(&self) -> u8 {
        self.world.ship_count
    }

    /// One model matrix per craft in play, the player's first.
    ///
    /// The player is index 0 and the seven opponents follow, which is the array
    /// order and **not** the grid order - the player sits on grid slot 8. See
    /// [`Race::start`].
    #[must_use]
    pub fn ship_model_matrices(&self) -> Vec<Mat4> {
        self.world
            .ships
            .iter()
            .take(self.world.ship_count as usize)
            .filter(|ship| ship.active)
            .map(model_matrix_of)
            .collect()
    }
}

/// A craft's model matrix, which is the only correct way to place its mesh.
fn model_matrix_of(ship: &Ship) -> Mat4 {
    let body = &ship.physics.body;
    Mat4::from_rotation_translation(body.orientation, body.position)
        * Mat4::from_rotation_y(MODEL_YAW)
        * Mat4::from_scale(Vec3::splat(oag_render::exhaust::CRAFT_ROW_SCALE))
}

/// The chase camera's view of a ship.
///
/// Three vectors rather than a ship, because `oag-render` may not see a gameplay
/// type; that is rule 1 of `docs/architecture/workspace-layout.md`.
fn target_of(ship: &Ship) -> Target {
    let body = &ship.physics.body;
    Target {
        position: body.position,
        forward: body.forward(),
        up: body.up(),
    }
}

/// A held-button mask, as one snapshot per tick, through the real keyboard path.
///
/// The headless capture and the ground-truth test both need input with no window.
/// They get it by pressing the **keys** a window would have seen, so there is
/// exactly one mapping from a device to a snapshot and the axes are derived in
/// exactly one place - [`oag_input::Keyboard`]. Deriving them a second time here is
/// how "left" ends up meaning two different things in two files.
#[derive(Debug, Default)]
pub struct HeldButtons {
    keyboard: Keyboard,
}

impl HeldButtons {
    /// Holds every button in `mask` down, and nothing else.
    ///
    /// Buttons with no key bound to them are skipped: a mask naming START in a race
    /// is not a fault condition.
    #[must_use]
    pub fn new(mask: u32) -> Self {
        let mut keyboard = Keyboard::new();
        for index in 0..32u8 {
            if mask & (1u32 << index) == 0 {
                continue;
            }
            if let Some(key) = key_for_button(index) {
                keyboard.set_key(&key, true);
            }
        }
        Self { keyboard }
    }

    /// One tick's snapshot.
    pub fn snapshot(&mut self) -> InputSnapshot {
        self.keyboard.snapshot()
    }

    /// Makes exactly `mask`'s buttons held, releasing everything else.
    ///
    /// For script-driven capture: each tick's authored state is a level, and
    /// the keyboard path derives the edges (and the axes) from the level
    /// changes exactly as it would for a real player. Unbound buttons are
    /// skipped, same as [`Self::new`].
    pub fn set_held(&mut self, mask: u32) {
        for index in 0..32u8 {
            if let Some(key) = key_for_button(index) {
                self.keyboard.set_key(&key, mask & (1u32 << index) != 0);
            }
        }
    }

    /// Sets `mask`'s buttons down and clears them, on top of what is held.
    ///
    /// For a gesture that needs *edges* rather than a level. A held button
    /// produces one rising edge and never another, so anything reading
    /// `Input::is_pressed` - the veteran sideshift's double tap, for one - is
    /// invisible to [`Self::new`]'s mask alone. Buttons in both masks stay down:
    /// holding and pulsing the same button is a contradiction, and resolving it
    /// toward held is the reading that does not silently drop a hold.
    pub fn pulse(&mut self, mask: u32, held: u32, down: bool) {
        for index in 0..32u8 {
            if mask & (1u32 << index) == 0 || held & (1u32 << index) != 0 {
                continue;
            }
            if let Some(key) = key_for_button(index) {
                self.keyboard.set_key(&key, down);
            }
        }
    }
}

/// A key that produces the given abstract button.
///
/// The inverse of [`oag_input::keys::map_key`], for the buttons a race uses. An
/// inverse and not a second mapping: a unit test below asserts every pair
/// round-trips, so this table cannot drift away from the one the window uses.
fn key_for_button(index: u8) -> Option<winit::keyboard::Key> {
    use oag_gameplay::input::button;
    use winit::keyboard::{Key, NamedKey};

    let key = match index {
        button::UP => Key::Named(NamedKey::ArrowUp),
        button::DOWN => Key::Named(NamedKey::ArrowDown),
        button::LEFT => Key::Named(NamedKey::ArrowLeft),
        button::RIGHT => Key::Named(NamedKey::ArrowRight),
        button::CROSS => Key::Character("x".into()),
        // Fire and absorb, the two buttons a pickup reads. The same keys
        // `oag_input::keys::map_key` binds them to, so a capture presses what a
        // player presses. **They were missing until weapons existed**, which
        // made `--press square` a silent no-op and a weapon capture impossible -
        // the mask named a button, `key_for_button` returned `None`, and the
        // skip this function documents swallowed it.
        button::SQUARE => Key::Character("c".into()),
        button::CIRCLE => Key::Character("z".into()),
        button::L => Key::Character("q".into()),
        button::R => Key::Character("e".into()),
        _ => return None,
    };
    Some(key)
}

/// Uniforms shared with `oag-render`'s `mesh.wgsl`.
///
/// Declared here rather than reused because `oag_render::mesh_render` only exposes
/// a writer that computes an *orbit* camera from the model's bounding sphere, which
/// is what a viewer wants and is not something a chase camera can use. The layout is
/// the shader's own, and a test below asserts it against
/// `mesh_render::UNIFORMS_SIZE`, so a change on that side is a failing test rather
/// than a silently wrong picture.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    /// Unused, and padding rather than removed - see
    /// `mesh_render`'s own mirror of this layout. Was a global
    /// texture-animation phase, replaced by the authored per-material
    /// keyframe tracks in `mesh_render::TexAnims`.
    _unused: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

/// What one frame's frustum culling did, for the performance overlay.
///
/// Plain counts rather than anything richer: the point is to show the effect
/// of `[graphics] lod` and frustum culling is having, not to profile the
/// renderer.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u32,
}

impl SceneStats {
    fn add(&mut self, other: Self) {
        self.draws_submitted += other.draws_submitted;
        self.draws_culled += other.draws_culled;
        self.triangles += other.triangles;
    }
}

/// The track's authored visibility partition, and where this track's geometry
/// sits in it.
///
/// Built once at load. Per frame it answers one question - which sections may
/// be drawn - in a handful of array reads. See
/// `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
#[derive(Debug)]
pub struct TrackVisibility {
    pvs: oag_formats::pvs::TrackPvs,
    padding: SectionPadding,
    sections: DrawSections,
    /// Mask-exclusive section pairs with overlapping geometry - authored
    /// LOD swaps, which the per-frame union must never re-join. See
    /// [`SwapConflicts`].
    swaps: SwapConflicts,
    /// What the association rule managed, for the load report.
    pub placement: PlacementStats,
}

impl TrackVisibility {
    /// Reads the `section` nodes of a track and places its draw calls in them.
    ///
    /// Returns `None` when the track declares no sections at all - which is
    /// every Pure track, since Pure does not share Pulse's class numbering.
    /// The caller then draws with no first tier, exactly as before.
    #[must_use]
    pub fn build(model: &Model, blob: &[u8], ai: &AiTrack) -> Option<Self> {
        let nodes = oag_formats::vex::nodes(blob).ok()?;
        let pvs = oag_formats::pvs::TrackPvs::from_nodes(blob, &nodes).ok()?;
        if pvs.is_empty() {
            return None;
        }
        // The authored association: a `section` node governs its parent's
        // whole subtree, so each draw call inherits its scene node's group.
        // This is what hides a far-LOD copy of the track while racing on the
        // real one - see `oag_render::pvs`.
        let governing = oag_formats::pvs::governing_sections(blob, &nodes).ok()?;
        let (sections, placement) = DrawSections::place(model, &governing, &pvs);
        let swaps = SwapConflicts::find(&pvs, &sections, model);
        Some(Self {
            pvs,
            padding: SectionPadding::from_track(ai),
            sections,
            swaps,
            placement,
        })
    }

    /// How many authored LOD-swap pairs the track carries, for the load
    /// report.
    #[must_use]
    pub fn swap_pairs(&self) -> usize {
        self.swaps.pair_count()
    }

    /// What may be drawn with the craft in `craft` and the camera in `camera`.
    #[must_use]
    fn set(&self, craft: u8, camera: u8) -> VisibleSet {
        VisibleSet::around(&self.pvs, &self.padding, &self.swaps, craft, camera)
    }
}

/// How far off the nearest spline sample a point may be and still be trusted to
/// name a section, as a multiple of the track's widest half-width.
///
/// **The conservative path exists because a craft can leave the partition, and
/// being outside it is not the same as looking up an id the track does not
/// have.** A craft that has fallen off, is airborne over a gap, or has been
/// knocked into scenery is usually still inside *some* authored box, so it gets
/// a valid answer that is simply wrong for what the camera is now framing - the
/// all-ones fallback never fires on its own. Culling to a stale section exactly
/// when the craft is somewhere unusual is the most visible way this could fail,
/// so distance to the racing line gates it instead.
///
/// Three half-widths is deliberately loose: it must not trip during ordinary
/// wide cornering, only when the craft is somewhere the authored partition was
/// not drawn around. Widening it costs nothing but a little culling; narrowing
/// it risks pop-in. There is no recovered value to match - the original does
/// not have this problem, because it never culls to a camera.
const OFF_TRACK_HALF_WIDTHS: f32 = 3.0;

/// How many samples either side of the craft's own the camera is looked for in.
///
/// At four samples per control-point interval this is a couple of dozen
/// intervals of track, far more than a chase camera trails by, and about
/// thirty-five times cheaper than the whole-table scan it replaces. Missing is
/// safe - see [`Spline::nearest_within`].
const CAMERA_SEARCH_SAMPLES: usize = 96;

/// Whether `draw` should be submitted, in two tiers.
///
/// **The authored PVS first, the frustum second**, which is the ordering the
/// ADR is about: the mask test is an integer `and` and the frustum test is six
/// plane-versus-sphere evaluations, so the cheap one has to run first for the
/// second tier to see less work. Delegated to `oag_render::pvs` so the ordering
/// lives with the types it operates on rather than being re-established at each
/// call site.
///
/// `set` is `None` when `[graphics] pvs_culling` is off or the track has no
/// sections; `frustum` is `None` per [`Drawable::draw`].
fn visible(
    draw: &DrawCall,
    sections: u64,
    set: Option<&VisibleSet>,
    frustum: Option<&Frustum>,
) -> bool {
    oag_render::pvs::visible(draw, sections, set, frustum)
}

/// One model on the GPU: its pipeline, its geometry and its own uniform buffer.
///
/// Two of these are drawn into one render pass. Separate pipelines rather than one
/// shared between the models, because each `mesh_render::build` creates its own bind
/// group layouts and pairing a bind group with another pipeline's layout is a
/// validation error waiting to happen.
struct Drawable {
    model: Model,
    pipeline: wgpu::RenderPipeline,
    alpha_test_pipeline: wgpu::RenderPipeline,
    blend_pipeline: [wgpu::RenderPipeline; 2],
    /// The other two recovered transparent blend classes. Selected per draw
    /// call, because a single model mixes them - see
    /// `oag_formats::vex::Batch::blend_class`.
    additive_pipeline: [wgpu::RenderPipeline; 2],
    unblended_pipeline: [wgpu::RenderPipeline; 2],
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    textures: Vec<wgpu::BindGroup>,
    /// Bind group 2: the fog this drawable is rendered with.
    fog_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame from the track's `fogCube`,
    /// or left at [`mesh_render::Fog::off`] for the sky and for a track that
    /// authors no fog.
    fog: wgpu::Buffer,
    /// Bind group 3: this drawable's authored texture transforms, sampled for
    /// the current tick.
    anim_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame by [`Self::write_anims`], or
    /// left all-identity for a model that authors no track.
    anims: wgpu::Buffer,
}

impl std::fmt::Debug for Drawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawable")
            .field("model", &self.model.label)
            .field("triangles", &(self.model.indices.len() / 3))
            .finish_non_exhaustive()
    }
}

impl Drawable {
    #[allow(clippy::too_many_arguments)]
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        depth: mesh_render::Depth,
        blend: wgpu::BlendState,
        glow: mesh_render::GlowMask,
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
            fog_bind,
            fog_buffer,
            anim_bind,
            anim_buffer,
        } = mesh_render::build(
            device,
            queue,
            &model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
            glow,
        )?;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("race uniforms"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        Ok(Self {
            model,
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
            fog_bind,
            fog: fog_buffer,
            anim_bind,
            anims: anim_buffer,
        })
    }

    fn write(&self, queue: &wgpu::Queue, view_projection: Mat4, model: Mat4) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            _unused: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Samples every authored texture-transform track this model carries at
    /// `seconds` and uploads the table the vertex shader indexes.
    ///
    /// One buffer write per drawable per frame, whatever the geometry: the
    /// curve is evaluated once per *track*, not per vertex or per draw call.
    ///
    /// **Not called for the boost plume**, whose table stays all-identity.
    /// The plume needs its own clock - its flare's life timer, reset at every
    /// reveal, rather than the race's - and already gets it through
    /// [`Self::apply_uv_transform`], which rewrites its vertices directly.
    /// Driving it from here as well would apply the transform twice.
    fn write_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_tracks.is_empty() {
            return;
        }
        let anims = mesh_render::TexAnims::sample(&self.model, seconds);
        queue.write_buffer(&self.anims, 0, bytemuck::bytes_of(&anims));
    }

    /// Applies a texture transform to this model's **authored** UVs and
    /// uploads them: `uv' = uv * scale + offset`, the GE's own
    /// `TEXSCALE`/`TEXOFFSET` arithmetic.
    ///
    /// Called only for the plume, and only on the frames it is visible. This
    /// replaced environment-mapped generation (`oag_render::texgen`) on
    /// 2026-08-10: the plume's compiled list is replayed under `TEXMAPMODE` 0
    /// (settled live - mesh-draw.md, "The plume is replayed under
    /// `TEXMAPMODE` 0"), so the original reads the authored coordinates,
    /// through the keyframed transform `Loaded::boost_uv_transform` carries.
    ///
    /// From `self.model.vertices` every time rather than from the last
    /// frame's buffer, for the same reason [`Self::deflect_airbrakes`] does:
    /// accumulating offsets would drift.
    fn apply_uv_transform(&self, queue: &wgpu::Queue, scale: (f32, f32), offset: (f32, f32)) {
        let transformed: Vec<_> = self
            .model
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                v.texcoord = [
                    v.texcoord[0] * scale.0 + offset.0,
                    v.texcoord[1] * scale.1 + offset.1,
                ];
                v
            })
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&transformed));
    }

    /// Swings this model's airbrake flaps to `left` and `right` radians.
    ///
    /// Rewrites the two flaps' own vertices in place rather than giving them a
    /// transform of their own. The alternative - pulling each flap into its own
    /// `Drawable`, the way the boost plume is one - would duplicate the ship's
    /// whole eight-texture set for two meshes of forty vertices, and a
    /// per-draw-call matrix would need a fourth bind group set on every draw
    /// call of every track. This costs two `write_buffer`s of about a kilobyte
    /// and no GPU state at all, and `exhaust.rs` already rewrites a vertex
    /// buffer every frame, so a mutable one is not a new idea here.
    ///
    /// A no-op on a model with no flaps, which is every track, the sky, the
    /// collision overlay and any ship whose file does not carry them.
    fn deflect_airbrakes(&self, queue: &wgpu::Queue, left: f32, right: f32) {
        for (flap, angle) in self.model.airbrakes.iter().zip([left, right]) {
            let Some(flap) = flap else { continue };
            let swing = flap.deflect(angle);
            let span = flap.vertices.start as usize..flap.vertices.end as usize;
            let Some(base) = self.model.vertices.get(span.clone()) else {
                continue;
            };
            // From the model's own vertices every time, never from the last
            // frame's: accumulating rotations would drift, and worse, would
            // make the rest position depend on how the ship got there.
            let moved: Vec<mesh::GpuVertex> = base
                .iter()
                .map(|v| {
                    let mut out = *v;
                    out.position = swing
                        .transform_point3(Vec3::from_array(v.position))
                        .to_array();
                    out.normal = swing
                        .transform_vector3(Vec3::from_array(v.normal))
                        .to_array();
                    out
                })
                .collect();
            let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
            queue.write_buffer(
                &self.vertices,
                span.start as u64 * stride,
                bytemuck::cast_slice(&moved),
            );
        }
    }

    /// Draws every list, in pipeline order, and reports what it submitted.
    ///
    /// `frustum` is `None` for the ship and the collision overlay: both draw a
    /// handful of batches next to the camera regardless, where the bookkeeping
    /// would cost more than the culling could ever save, and both carry a
    /// non-identity model matrix the track does not - the [`Bounds`] on a
    /// `DrawCall` are in the *model's own* space, valid to test directly
    /// against a world-space frustum only while that space and world space
    /// are the same transform, which is only true here for the track (see the
    /// `Mat4::IDENTITY` passed to [`Self::write`] at each call site).
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        // A model with no placement table has every draw call unplaced, which
        // the first tier always allows. That is the ship and the collision
        // overlay, and any track whose sections did not decode.
        let empty: &[u64] = &[];
        let (opaque, alpha_tested, transparent) = match sections {
            Some(s) => (&s.opaque[..], &s.alpha_tested[..], &s.transparent[..]),
            None => (empty, empty, empty),
        };
        if self.model.indices.is_empty() {
            return stats;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        // Bound once for the whole drawable: fog is per-frame, not per draw call,
        // and group 2 survives the `set_pipeline` calls below.
        pass.set_bind_group(2, &self.fog_bind, &[]);
        // Same reasoning as fog: the table is per frame, not per draw call.
        pass.set_bind_group(3, &self.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        for (index, draw) in self.model.draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(opaque, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Second pipeline, same pass: alpha-tested batches, cutout. See
        // `mesh_render::Built::alpha_test_pipeline`.
        pass.set_pipeline(&self.alpha_test_pipeline);
        for (index, draw) in self.model.alpha_tested_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(alpha_tested, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Third pipeline group, same pass: transparent batches, blended.
        //
        // **The blend equation is the batch's own, not the model's.**
        // `Gfx_BuildBatchStateList` splits `is_transparent()`'s `0x0700` three
        // ways - `0x100` alpha-over, `0x200` additive, `0x400` unblended - and
        // a single model mixes them, so the pipeline is chosen per draw call.
        // This crate drew every one of them alpha-over until that was
        // recovered. See `oag_formats::vex::Batch::blend_class` and
        // `mesh_render::ADDITIVE_BLEND`.
        //
        // Switched only when the class changes rather than grouped by class
        // first: transparent draws are order-dependent by definition, and
        // sorting them to save `set_pipeline` calls would reorder the picture.
        let mut current: Option<(Option<oag_formats::vex::BlendClass>, bool)> = None;
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(transparent, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            let key = (draw.blend, draw.culled);
            if current != Some(key) {
                let set = match draw.blend {
                    Some(oag_formats::vex::BlendClass::Additive) => &self.additive_pipeline,
                    Some(oag_formats::vex::BlendClass::None) => &self.unblended_pipeline,
                    Some(oag_formats::vex::BlendClass::AlphaOver) | None => &self.blend_pipeline,
                };
                pass.set_pipeline(&set[usize::from(draw.culled)]);
                current = Some(key);
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }
}

/// The track and the ship on the GPU, drawn from a chase camera.
#[derive(Debug)]
pub struct Scene {
    track: Drawable,
    /// The track's authored fog volumes, sampled at the camera each frame.
    ///
    /// Empty for a track that authors no `fogCube` - four of the forty - and the
    /// race then renders unfogged, which is what the original does too.
    fog_volumes: Vec<oag_formats::fog::FogVolume>,
    /// The track's `Skycube`, drawn camera-centred before anything else.
    ///
    /// `None` when the file authors no sky, which is every Pure track and every
    /// non-track `.vex`. Its own [`mesh_render::Depth::Sky`] pipelines compare
    /// `Always` and write no depth, so it fills the frame and everything drawn
    /// after covers it - see [`Scene::render`] for why it is not scaled to the
    /// far plane instead.
    sky: Option<Drawable>,
    /// The track's `Speedup Pad` geometry, drawn with the track.
    ///
    /// Same pipeline, same textures, same fog, same world matrix as
    /// [`Self::track`]; separate only because a pad belongs to no visibility
    /// `section`, so it is offered frustum culling but not the PVS. `None` when
    /// the track authors none.
    pads: Option<Drawable>,
    /// The track's `Weapon Pad` geometry, drawn beside [`Self::pads`].
    weapon_pads: Option<Drawable>,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` for a track with no `section` nodes - every Pure track - and the
    /// first tier is then skipped entirely rather than approximated.
    visibility: Option<TrackVisibility>,
    /// One drawable per craft, the player's first.
    ///
    /// Separate drawables over a cloned mesh rather than instancing: a
    /// `Drawable` owns its uniform buffer, and eight craft need eight matrices a
    /// frame. See `oag_render::mesh::Model`'s note on the trade.
    ships: Vec<Drawable>,
    /// One boost plume per craft: ordinary `Drawable`s with their blend pipeline
    /// overridden to [`exhaust::BLEND`] instead of
    /// [`mesh_render::TRANSPARENT_BLEND`].
    ///
    /// **Empty** when the source carries no `shipboost.vex` under this team's
    /// name - see `Loaded::boost_model`. Each is drawn only while its craft's
    /// [`Exhaust::plume_visible`] is true, with that craft's own model matrix,
    /// since the original parents the plume to the craft rather than to the
    /// flare.
    ///
    /// Eight copies of the mesh for the same reason [`Self::ships`] holds eight:
    /// a `Drawable` owns the uniform buffer its model matrix and its UV transform
    /// are written into, and two craft boosting at once need two of each. Empty
    /// rather than `Option<Vec<_>>` so the no-plume source and the iteration read
    /// the same way `ships` does.
    boost: Vec<Option<Drawable>>,
    /// One drawable per projectile slot, for rockets drawn as their own model.
    ///
    /// **Empty** when `Data\Weapons\Rocket.vex` did not load, and the sprite
    /// fallback in [`Race::projectile_sprites`] carries the whole effect then.
    /// Sized to `MAX_PROJECTILES` and clone-per-slot for the same reason
    /// [`Self::ships`] is clone-per-craft: a `Drawable` owns the uniform buffer
    /// its model matrix goes in, and three rockets in the air at once need three
    /// matrices. Eighty-four vertices apiece makes sixteen copies cheap.
    rockets: Vec<Drawable>,
    /// Each slot's own plume's authored texture-transform keyframes, sampled
    /// per frame and applied to that plume's authored UVs - the recovered
    /// mechanism (`TEXMAPMODE` 0 plus the animated `TEXOFFSET` u-scroll; see
    /// `crate::livery::Livery::boost_uv`). `None` falls back to the engine's
    /// own identity default.
    ///
    /// **Per slot, because the plumes are.** Every team read so far carries
    /// the identical track, so sharing one would be invisible today and wrong
    /// the moment a team did not.
    boost_uv_transforms: Vec<Option<oag_formats::vex::TexTransform>>,
    /// The collision soup overlay, present only when `Options::collision` asked
    /// for it. Drawn with the identity transform, same as the track: the
    /// collision geometry is already in world space.
    collision: Option<Drawable>,
    /// The engine flare and the boost plume: the frame's blended pipelines.
    ///
    /// `RefCell` because its per-frame upload needs `&mut` while [`Scene::render`]
    /// stays `&self`. That signature is worth keeping: the alternative threads
    /// `&mut` through `RaceStage::render` and `race::capture` for a buffer write
    /// that `queue` already accepts through a shared reference. The borrow is
    /// taken and released inside `render` with nothing re-entrant in between.
    exhaust: std::cell::RefCell<exhaust::Pipeline>,
    /// Collision sparks. `RefCell` for the same reason [`Self::exhaust`] is.
    sparks: std::cell::RefCell<sparks::Pipeline>,
    /// The recovered bloom, run after the scene pass over whatever the frame
    /// stamped into its alpha channel. `None` when the pipelines would not
    /// build, which costs the glow and nothing else.
    ///
    /// **Not exhaust-specific**, even though the exhaust is currently its only
    /// writer: it blooms the glow mask, and any surface that opts into the mask
    /// is handled by the same three passes. See `oag_render::post::bloom`.
    bloom: Option<oag_render::post::bloom::Bloom>,
    depth: wgpu::Texture,
    /// The colour attachment every pipeline here actually draws into, and its
    /// sample count.
    ///
    /// `Some` for `[graphics] anti_aliasing`'s two MSAA levels: every pipeline
    /// above is built at `sample_count`, and [`Scene::render`] draws them into
    /// this multisampled target and resolves it into the caller's own view at
    /// the end of the one pass. `None` at sample count 1, where there is
    /// nothing to resolve and the pipelines draw straight into the caller's
    /// view - see [`Scene::render`].
    ///
    /// Baked in at [`Scene::new`] rather than read from settings each frame:
    /// every pipeline's `multisample` state is fixed at the moment it is
    /// built, so changing this setting mid-race would need every pipeline
    /// above rebuilt, not just this texture. See
    /// [`crate::display::AntiAliasing::msaa_samples`].
    msaa_color: Option<wgpu::Texture>,
    /// What this scene's pipelines were actually built with, for the
    /// GRAPHICS menu's restart note - see `Session::open_menus` in `main.rs`.
    anti_aliasing: crate::display::AntiAliasing,
    /// Where the far plane goes, from the track's own extent.
    far: f32,
}

impl Scene {
    /// Builds both pipelines and a depth buffer for a viewport of `size`.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure from `oag-render`.
    // Three models plus how to draw them (surface format, viewport, texture
    // filtering); a wrapper struct for one call site would name the grouping
    // without clarifying it.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track_model: Model,
        liveries: &[Livery],
        collision_model: Option<Model>,
        sky_model: Option<Model>,
        pad_model: Option<Model>,
        weapon_pad_model: Option<Model>,
        mode: Mode,
        rocket_model: Option<Model>,
        flare: Option<FlareTexture>,
        noise: Option<FlareTexture>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
        bloom_enabled: bool,
        visibility: Option<TrackVisibility>,
        anti_aliasing: crate::display::AntiAliasing,
        fog_volumes: Vec<oag_formats::fog::FogVolume>,
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let sample_count = anti_aliasing.msaa_samples();
        let scene_depth = mesh_render::Depth::Scene;
        let sky = sky_model
            .filter(|model| !model.indices.is_empty())
            .map(|model| {
                Drawable::new(
                    device,
                    queue,
                    model,
                    format,
                    anisotropy,
                    sample_count,
                    mesh_render::Depth::Sky,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                )
            })
            .transpose()?;
        let track = Drawable::new(
            device,
            queue,
            track_model,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
        )?;
        // One per grid slot, each drawing **its own team's hull**. Built up
        // front rather than on demand, because a `Drawable` needs the device
        // and the pass does not have it. A grid shorter than `GRID_SLOTS`
        // liveries repeats the last one rather than panicking - `livery::load`
        // returns one per slot, so that is a defensive floor and not a path
        // any caller takes.
        let mut ships = Vec::with_capacity(GRID_SLOTS as usize);
        for slot in 0..GRID_SLOTS as usize {
            let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
            ships.push(Drawable::new(
                device,
                queue,
                livery.hull.clone(),
                format,
                anisotropy,
                sample_count,
                scene_depth,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
            )?);
        }
        let collision = collision_model
            .map(|model| {
                Drawable::new(
                    device,
                    queue,
                    model,
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                )
            })
            .transpose()?;
        let pad_drawable = |model: Option<Model>| {
            model
                .filter(|model| !model.indices.is_empty())
                .map(|model| {
                    Drawable::new(
                        device,
                        queue,
                        model,
                        format,
                        anisotropy,
                        sample_count,
                        scene_depth,
                        mesh_render::TRANSPARENT_BLEND,
                        mesh_render::GlowMask::Protected,
                    )
                })
                .transpose()
        };
        let pads = pad_drawable(pad_model)?;
        // `World_CollectNodeLists` clears the node's own visibility bit
        // whenever `g_weapons_enabled` is `0`, which the front end forces for
        // time trial, speed lap and Zone alike - see `Mode::weapons_enabled`.
        // **The same predicate gates the trigger** (`Race::test_weapon_pads`),
        // deliberately: in the original both come from that one global, so a
        // pad that draws is a pad that can be crossed and there is no mode
        // where one is true and the other is not.
        // The geometry is still decoded and still in
        // `Loaded` either way (`the_weapon_pads_are_drawn_where_they_trigger`
        // checks the decode, not the mode), so this is the one place that
        // decision turns into "never uploaded to the GPU at all" rather than
        // "decoded, then hidden" - closer to what clearing a visibility bit
        // before the submit pass actually does than a flag checked at draw
        // time would be.
        let weapon_pads = if mode.weapons_enabled() {
            pad_drawable(weapon_pad_model)?
        } else {
            None
        };
        // The boost plume, drawn additively. It is real geometry (a `.vex`
        // mesh, not a camera-facing quad), so it needs depth testing and a
        // model matrix, which is what `Drawable` already gives every other
        // mesh here rather than a third thing beside `exhaust::Pipeline`.
        // An additive blend in place of `TRANSPARENT_BLEND` is the one thing
        // that has to differ - the model's texture is named `_ADD`, and
        // drawing it with the ordinary lerp blend looks plausible and is
        // wrong.
        //
        // **Which additive blend changed on 2026-08-08, from the ribbon's
        // `TRAIL_BLEND` to the flare's `exhaust::BLEND`, and it is a bug fix
        // rather than a retuning.** The two differ only in the source factor -
        // `One` against `SrcAlpha` - and the plume is the one model here whose
        // authored vertex alpha is not constant: it is bimodal, `0` on the
        // orange rim and `255` in the white core (pinned by
        // `boost_plume_ground_truth.rs`). Under `TRAIL_BLEND` alpha cannot
        // reach the picture at all, so the load site compensated by
        // premultiplying it into the RGB *per vertex* - and that is simply the
        // wrong arithmetic. Premultiplying at a vertex and then interpolating
        // is not interpolating and then multiplying: it maps the rim to
        // **black**, so what interpolates across the fin is black-to-white and
        // the authored orange never exists anywhere on it. `SrcAlpha` does the
        // same multiply *per fragment*, after the rasteriser has interpolated
        // colour and alpha separately, which is what real hardware does and
        // what leaves a dimmed orange fringe fading out along the fin.
        //
        // Measured against the first matched-pose pad capture (boost age
        // `0.367` s, the original's own recorded camera and its measured
        // entry intensity), over the magenta signature `r > 110, b > 110,
        // r - g > 25, b - g > 20` in a crop around the craft:
        //
        // | build | pixels | mean colour | `b - r` |
        // | --- | ---: | --- | ---: |
        // | the original | 9,217 | `(223, 164, 224)` | +1 |
        // | premultiplied, `TRAIL_BLEND` | 493 | `(180, 141, 228)` | +48 |
        // | raw, `BLEND` (this) | 1,993 | `(159, 118, 175)` | +16 |
        //
        // Extent 4x better and the red/blue balance three times closer to the
        // original's neutral. The premultiplied build was blue-dominant; raw
        // alpha alone recovers the hue but draws the fins as hard-edged solid
        // wedges, which is the 2026-08-07 and 2026-08-08 result reproduced
        // exactly, and `SrcAlpha` is what supplies the falloff those wedges are
        // missing. Still 22% of the original's extent - but that comparison is
        // against the original's frame, so it is one of the readings the
        // 2026-08-09 projection finding invalidates: our whole image is zoomed
        // 1.12-1.26x depending on speed, because the original widens its fov
        // with speed and we do not. Re-take it with `--camera-fov` before
        // leaning on the number. The missing bright-pass is unaffected and
        // still real; "a craft drawn too large" was the wrong half and is
        // withdrawn - see `docs/rendering/projection-vs-the-original.md`.
        //
        // **This is an empirical approximation and the real mechanism is
        // something else.** The GE state for the transparent mesh pass is now
        // read: `GU_ALPHA_TEST` is *disabled*, `GU_BLEND` is *enabled* with
        // `GU_FIX` white on both sides, and nothing in the pass scales fragment
        // RGB but vertex colour x texture x the blend - so no source-alpha
        // weight exists on the hardware at all. What supplies the original's
        // falloff is the *texture*: the pass sets `TEXMAPMODE` uvgen 2,
        // environment mapping from the vertex normal and lights 0/1, and
        // `pulse_boost2_ADD`'s own bright-to-dark gradient varies across the
        // fin. Every plume batch declares normals (`vtype = 0x013d` on all 32
        // across 8 teams - `cargo run -p oag-assets --example
        // boost_vertex_type`), so that generation has something to vary with.
        //
        // **Environment-mapped UV generation was implemented here 2026-08-09
        // and replaced 2026-08-10**: the plume's compiled list is replayed
        // under `TEXMAPMODE` 0 (settled by reading the recorded frame stream
        // in GE order - mesh-draw.md, "The plume is replayed under
        // `TEXMAPMODE` 0"), so the original samples the *authored* UVs
        // through the keyframed `TEXOFFSET` u-scroll authored in the file
        // itself. `Drawable::apply_uv_transform` now does exactly that;
        // `oag_render::texgen` remains correct for batches genuinely inside
        // the transparent-pass bracket, which the plume is not.
        //
        // **`SrcAlpha` stays, and it is no longer a stand-in - it is a
        // measured choice that beat the recovered alternative.** The argument
        // for going back to `TRAIL_BLEND` was strong on paper: the recovered
        // GE state really is `GU_FIX` white on both sides, `SrcAlpha` was only
        // ever introduced as a substitute for the missing texture falloff, and
        // that falloff now exists. So it was tried, at the original's own pose
        // (`data/traces/pad0-boost.csv` tick 62), against the original's own
        // frame (`data/shots/pad0-boost/tick00062.png`), over the capture's own
        // plume mask (`min(r, b) - g > 25` and `luma > 60`):
        //
        // | build | plume px | mean | `b - r` | orange px |
        // | --- | ---: | --- | ---: | ---: |
        // | the original | 9,435 | `(220, 165, 237)` | +17.6 | 230 |
        // | ours, authored UVs (what shipped before) | 4,480 | `(184, 143, 203)` | +19.0 | 2,679 |
        // | **ours, texgen + `SrcAlpha` (this)** | **5,996** | `(197, 150, 210)` | +13.3 | **2,041** |
        // | ours, texgen + `TRAIL_BLEND` (the recovered blend) | 1,958 | `(243, 181, 227)` | -15.6 | 7,030 |
        //
        // **The measurements live in
        // `docs/ghidra/functions/psp-pulse-usa/exhaust.md`**, not here. This
        // table was duplicated across three files and re-measured four times -
        // wrong pose age, a boost accumulator charging 100x too slowly,
        // ADR-0020 moving background luminance 54 %, and finally a mask that
        // gated on absolute brightness - and every duplicate drifted. One home.
        //
        // What survives all four, and is why this line reads `exhaust::BLEND`:
        // the recovered `One`/`One` blend is the worst row on every column,
        // restoring the authored `(255, 98, 5)` rim at full strength where the
        // original's plume and ribbon are one violet family with nothing near
        // that orange. Generated coordinates beat authored ones on every
        // column too.
        //
        // **So something in the recovered blend chain is still incomplete**,
        // and that is worth stating rather than papering over: the GE state
        // was read exhaustively - all five setters to their command byte,
        // `Gu_TexFunc` confirmed `MODULATE`/`TCC_RGBA` - and it does not
        // reproduce the picture, while an unrecovered source-alpha weight does.
        // `SrcAlpha` is kept because it measures better, not because it is
        // understood.
        //
        // **Do not read any column stated against the original as calibrated.**
        // This comment used to blame a craft that "renders about 1.4x too
        // large"; that is refuted - the mesh is correct to 0.15 % and the whole
        // frame is zoomed, because the original widens its fov with speed and
        // we do not. At this capture's 148.9 units/s that is roughly 1.25x, and
        // it shifts which of the original's pixels fall inside the mask. The
        // ours-versus-ours results above are unaffected: all three builds
        // render at the same pose, so they share one zoom and a ratio between
        // them cancels it. See `docs/rendering/projection-vs-the-original.md`,
        // `docs/ghidra/functions/psp-pulse-usa/exhaust.md` and `mesh-draw.md`.
        //
        // One per grid slot, cloned the way the hulls above are: every craft can
        // be on a speed pad at once, and eight plumes need eight model matrices
        // and eight sampled UV transforms a frame.
        //
        // **Each slot's own team's plume, sampled through its own team's
        // keyframes.** Eight per-team plumes played through one team's UV
        // track is the kind of wrong that renders plausibly, so the transform
        // is carried per slot beside the model. A slot whose team ships no
        // plume simply has none, which is why this is a `Vec` of `Option`
        // rather than a shorter `Vec` - the draw loop indexes by slot.
        let mut boost = Vec::new();
        let mut boost_uv_transforms = Vec::new();
        for slot in 0..GRID_SLOTS as usize {
            let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
            boost_uv_transforms.push(livery.boost_uv.clone());
            let Some(model) = livery
                .boost
                .as_ref()
                .filter(|model| !model.indices.is_empty())
            else {
                boost.push(None);
                continue;
            };
            {
                boost.push(Some(Drawable::new(
                    device,
                    queue,
                    model.clone(),
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    exhaust::BLEND,
                    // **The plume feeds the bloom.** Its draw path in the
                    // original opens the alpha channel unconditionally, unlike
                    // the hull's - see `mesh_render::GlowMask`. Without this the
                    // boost's brightest surface contributes nothing to the glow
                    // mask, which is the shape of the effect a player notices.
                    mesh_render::GlowMask::Written,
                )?));
            }
        }
        // One per projectile slot, cloned the way the hulls and plumes above are.
        // A rocket's hull is opaque - it is a painted dart, not a glow - so this
        // takes the ordinary transparent blend and the protected glow mask the
        // ships take, and the flare around it stays in the additive pass with
        // the exhaust where it belongs.
        let mut rockets = Vec::new();
        if let Some(model) = rocket_model.filter(|model| !model.indices.is_empty()) {
            for _ in 0..oag_gameplay::projectile::MAX_PROJECTILES {
                rockets.push(Drawable::new(
                    device,
                    queue,
                    model.clone(),
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                )?);
            }
        }
        // 64 is a stand-in size only, and only when the disc's own texture did not
        // decode; `load` has already reported that when it happens.
        let flare = flare.unwrap_or_else(|| FlareTexture::placeholder(64));
        let noise = noise.unwrap_or_else(|| FlareTexture::placeholder(64));
        let exhaust = std::cell::RefCell::new(exhaust::Pipeline::new(
            device,
            queue,
            format,
            &flare,
            &noise,
            sample_count,
        ));
        let sparks = std::cell::RefCell::new(sparks::Pipeline::new(device, format, sample_count));
        // A failure here is reported and dropped rather than propagated: a race
        // without a bloom is a dimmer race, not a broken one.
        let bloom = match bloom_enabled
            .then(|| oag_render::post::bloom::Bloom::new(device, format))
            .transpose()
        {
            Ok(bloom) => bloom,
            Err(e) => {
                eprintln!("bloom unavailable ({e}) - the frame draws without it");
                None
            }
        };

        Ok(Self {
            bloom,
            track,
            visibility,
            ships,
            boost,
            rockets,
            boost_uv_transforms,
            collision,
            sky,
            pads,
            weapon_pads,
            fog_volumes,
            exhaust,
            sparks,
            depth: depth_texture(device, size, sample_count),
            msaa_color: msaa_color_texture(device, format, size, sample_count),
            anti_aliasing,
            far,
        })
    }

    /// Rebuilds the depth buffer, and the MSAA colour target if there is one,
    /// for a new viewport size.
    ///
    /// A colour or depth attachment whose size does not match the others is a
    /// validation error, so this is not optional on resize.
    pub fn resize(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) {
        let sample_count = self.anti_aliasing.msaa_samples();
        self.depth = depth_texture(device, size, sample_count);
        self.msaa_color = msaa_color_texture(device, format, size, sample_count);
    }

    /// What this scene's pipelines were actually built with, for the restart
    /// note - see [`Self::msaa_color`].
    #[must_use]
    pub fn anti_aliasing(&self) -> crate::display::AntiAliasing {
        self.anti_aliasing
    }

    /// Draws one frame into `view`.
    ///
    /// One render pass with one clear: a second pass would either wipe the first's
    /// colour or need its own decision about the depth buffer.
    ///
    /// `cull` is `[graphics] frustum_culling` - off by default, see that
    /// setting's own doc comment for the measurement behind that default.
    ///
    /// Returns what the track's frustum culling did, for the performance
    /// overlay - see [`SceneStats`].
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        viewport: (f32, f32, f32, f32),
        fov: crate::display::Fov,
        cull: bool,
        pvs_cull: bool,
        animated_textures: bool,
    ) -> SceneStats {
        let aspect = viewport.2.max(1.0) / viewport.3.max(1.0);
        let view_projection = race.projection(aspect, self.far, fov) * race.view();
        let frustum = cull.then(|| Frustum::from_view_projection(view_projection));
        // Tier one, built once a frame. Both sections come from the authored
        // spline rather than from the section boxes: a control point's
        // `section_id` is what the artists wrote, while a point-in-box test is
        // something this project invented.
        let visible_set = self
            .visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .map(|visibility| {
                let (craft, camera) = race.visibility_sections();
                visibility.set(craft, camera)
            });
        // The animation clock, in seconds, from the tick and never the wall
        // clock - so a replay of the same tick draws the same frame. The
        // original passes the race clock to every world mesh's updater
        // (`docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The
        // values gap is closed"), and each authored track wraps on its own
        // period, so there is no global phase to keep in step.
        let seconds = race.world.tick as f32 / 60.0;
        // `[graphics] animated_textures` no longer gates a guess, so it no
        // longer gates the track: what draws now is the material's own
        // authored keyframe block. The setting stays as a way to freeze every
        // animated surface at its authored coordinates, which is what a
        // still-frame comparison against a capture wants.
        let track_seconds = if animated_textures { seconds } else { 0.0 };
        // Fog, sampled where the eye is. `oag_formats::fog::sample` reimplements
        // `FogCube_Sample`: the camera is transformed into the volume's space,
        // rejected if outside, and all six parameters interpolated across the
        // box's local Z. Outside every volume - or on a track with none - this
        // is `None` and the drawables keep `Fog::off`.
        //
        // The sky is deliberately left unfogged. It rides on the camera at a
        // radius of 18 to 62 units while fog starts at 30 to 250, so fogging it
        // would drown it in fog colour; the original's sky geometry is authored
        // `_nolight` and stands in for infinity, which is behind the fog rather
        // than inside it.
        let eye = race.camera_position();
        let fog = oag_formats::fog::sample(&self.fog_volumes, eye.to_array())
            .map_or_else(mesh_render::Fog::off, |p| {
                mesh_render::Fog::new(&p, eye.to_array())
            });
        for drawable in [
            Some(&self.track),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        .chain(self.ships.iter())
        {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(&fog));
        }
        // The scenery, on the clock `[graphics] animated_textures` can freeze.
        for drawable in [
            Some(&self.track),
            self.sky.as_ref(),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.write_anims(queue, track_seconds);
        }
        // The craft always animate. Their blink lights are the one animation
        // on the disc confirmed against a frame-accurate capture of the
        // original, so there is nothing about them for that switch to test.
        for drawable in &self.ships {
            drawable.write_anims(queue, seconds);
        }
        // `self.boost` is deliberately left out of both lists, at `Fog::off`
        // from `mesh_render::build`. Whether the original fogs the plume is
        // unrecovered - it is scene geometry like the ship, but additive like
        // the flare, and the flare's own post-projection draw is not answered
        // by fog either way. Left unfogged rather than guessed.

        // The sky rides with the eye. Translating it to the camera is what makes
        // an authored cube tens of units across stand in for a horizon: the
        // camera sits permanently at its centre, so the faces never approach and
        // never need to enclose the track. `race.view()` is the full view
        // matrix, roll included, so the horizon rolls with the ship through a
        // barrel roll exactly as `camera::chase` describes the original's
        // external view doing.
        if let Some(sky) = &self.sky {
            sky.write(
                queue,
                view_projection,
                Mat4::from_translation(race.camera_position()),
            );
        }
        self.track.write(queue, view_projection, Mat4::IDENTITY);
        // How many slots this race fills, which bounds every per-craft loop from
        // here down: the scene always holds a full grid's worth of drawables and a
        // time trial fields one craft.
        let drawn = usize::from(race.ship_count());
        // Every craft in play, the player first. `zip` rather than an index so a
        // race with fewer craft than the scene has drawables writes only the
        // ones it has - the rest keep last frame's uniforms and are not drawn.
        let matrices = race.ship_model_matrices();
        for (drawable, matrix) in self.ships.iter().zip(&matrices) {
            drawable.write(queue, view_projection, *matrix);
        }
        // The flaps move in *model* space, before the ship's own matrix, so
        // this is a vertex write and not a second uniform - see
        // `Drawable::deflect_airbrakes`. Unconditional rather than
        // change-gated: two kilobytes a frame is cheaper than the state needed
        // to know they have not moved, and a gate would have to be invalidated
        // by anything that ever rebuilds the buffer.
        let [left, right] = race.airbrake_flaps();
        if let Some(player) = self.ships.first() {
            player.deflect_airbrakes(queue, left, right);
        }
        // One matrix per rocket in the air. `zip` bounds it the way the ships'
        // loop is bounded: nothing in the air writes nothing, and the drawables
        // past the live count keep last frame's uniforms and are not drawn.
        let rocket_matrices = race.rocket_model_matrices();
        for (drawable, matrix) in self.rockets.iter().zip(&rocket_matrices) {
            drawable.write(queue, view_projection, *matrix);
        }
        // Same model matrix as the ship: the original parents the plume to the
        // craft, not to the flare - see `Loaded::boost_model`. Skipped while
        // hidden rather than written and left undrawn, since there is nothing
        // for the stale buffer contents to affect either way.
        //
        // The UVs are the authored ones, scrolled: the plume draws under
        // `TEXMAPMODE` 0 (settled live - mesh-draw.md, "The plume is replayed
        // under `TEXMAPMODE` 0"), sampling its authored coordinates through
        // the keyframed `TEXSCALE`/`TEXOFFSET` transform the file itself
        // carries. This replaced `oag_render::texgen` here 2026-08-10;
        // texgen's environment mapping is real but belongs to the
        // transparent-pass bracket, which the plume is not inside.
        //
        // The clock is the plume's own life timer, not the race clock: the
        // original's updater receives `flare+0x88` verbatim (measured at its
        // entry, t == the timer on every hit), and that field resets to 0 at
        // each reveal. So every boost plays the 90-frame track exactly once -
        // bright first key at reveal, darkening as it fades, clamped at the
        // last key just as the plume hides (`PLUME_SECONDS` and the track
        // span are both 1.5 s, by authoring, not coincidence). A free-running
        // clock here is visibly wrong in both directions: a boost can start
        // mid-ramp already faded, and one that outlives the wrap re-brightens
        // as a second pulse. `Exhaust::plume_timer` carries exactly the
        // original's reset-at-reveal semantics.
        //
        // **Per craft, and each on its own timer.** Eight craft can be mid-boost
        // at once and their reveals are independent, so the sample time is that
        // craft's `plume_timer` rather than one shared clock. Indexed by slot -
        // not `zip`ped over `ship_model_matrices`, which filters inactive craft
        // and so does not keep slot alignment.
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            // `None` is a slot whose *team* ships no plume, which is a
            // reported absence rather than a hidden one - see `livery::plume`.
            let Some(boost) = boost else {
                continue;
            };
            if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            boost.write(queue, view_projection, race.ship_model_matrix_of(slot));
            let (scale, offset) = match self.boost_uv_transforms.get(slot).and_then(Option::as_ref)
            {
                Some(transform) => {
                    let t = race.exhaust_of(slot).plume_timer() * 60.0;
                    (transform.scale.sample(t), transform.offset.sample(t))
                }
                None => ((1.0, 1.0), (0.0, 0.0)),
            };
            boost.apply_uv_transform(queue, scale, offset);
        }
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY);
        }
        for pads in [self.pads.as_ref(), self.weapon_pads.as_ref()]
            .into_iter()
            .flatten()
        {
            pads.write(queue, view_projection, Mat4::IDENTITY);
        }

        // The camera's own axes, read out of the view matrix: for a view matrix
        // `V`, world-space right and up are rows 0 and 1 of its rotation part.
        // Building the quad from these is what makes it face the viewer, and it is
        // the whole reason this is world-space rather than the original's
        // post-projection sprite.
        let camera = race.view();
        let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
        let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
        // One flare and one ribbon per craft, the player's first, all in the two
        // buffers `exhaust::Pipeline` owns - a flare is six vertices and a ribbon
        // is a fixed 648, so eight of each is one upload and one draw call apiece
        // rather than eight. The budgets are sized for exactly this: see
        // `exhaust::MAX_SPRITES`' table and `exhaust::MAX_TRAILS`.
        //
        // **The player's own flare is not skipped in the cockpit view**, and that
        // is deliberate rather than overlooked: the nozzle sits behind the eye, so
        // it falls outside the frustum on its own. See `Race::draws_own_ship`,
        // which skips the *hull* because a hull drawn around the camera really
        // does put polygons across the middle of the screen.
        let mut vertices = Vec::new();
        let mut trail = Vec::new();
        for slot in 0..drawn {
            // No locator, nothing drawn - rather than a flare at the origin.
            // `break` rather than `continue` because the whole field shares one
            // model, so a missing `Engine Flare` node is missing for every craft.
            let Some(nozzle) = race.nozzle_of(slot) else {
                break;
            };
            let exhaust = race.exhaust_of(slot);
            // The flare quad only where the source authors no effect for it
            // - see `Race::engine_flare_effect`. The ribbon always.
            if !race.engine_flare_effect() {
                vertices.extend(exhaust.vertices(nozzle, right, up));
            }
            trail.extend(exhaust.trail_vertices(right, up));
        }
        // Only the billboard *fallback* for a rocket whose model did not
        // load - the flare around one that did is an asset now, and goes
        // through the particle pipeline below with everything else.
        vertices.extend(race.projectile_sprites(right, up, !self.rockets.is_empty()));
        self.exhaust.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &vertices,
            &trail,
        );
        // The hull's collision sparks and the stage's rocket effects share
        // one pipeline and one pair of buffers: both are `.pob` particles in
        // the same two blend classes, so a second pipeline would buy nothing
        // but a second pass.
        let (mut additive, mut alpha) = race.spark_vertices(right, up);
        let (stage_additive, stage_alpha) = race.stage_vertices(right, up);
        additive.extend(stage_additive);
        alpha.extend(stage_alpha);
        self.sparks.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &additive,
            &alpha,
        );

        let depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        // MSAA draws into its own multisampled attachment and resolves into
        // `view` at the end of this one pass; everything else draws straight
        // into `view`, exactly as before this setting existed. See
        // `Self::msaa_color`.
        let msaa_view = self
            .msaa_color
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let (attachment_view, resolve_target) = match &msaa_view {
            Some(msaa_view) => (msaa_view, Some(view)),
            None => (view, None),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment_view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    // Black rather than the near-black blue this used to clear
                    // to. The clear covers the whole attachment and the scene is
                    // then drawn into a sub-rectangle, so this colour is what
                    // `Aspect`'s bars are made of - and a bar has to read as a
                    // bar. The old value was 0.03/0.04/0.06, dark enough that
                    // losing it costs nothing inside the viewport either.
                    // Black, and **alpha zero**: alpha is the bloom's glow mask
                    // and a frame starts with nothing glowing.
                    // `wgpu::Color::BLACK` has `a: 1.0`, which would mask the
                    // entire frame in and bloom everything.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
        // First, and that ordering is as load-bearing as the exhaust's being
        // last. The sky writes no depth and compares `Always`, so it paints the
        // whole viewport and every later draw covers it wherever the track has
        // geometry; drawn at any other point it would overwrite what is already
        // there. Neither culling tier is offered it: a skybox is never outside
        // the frustum and belongs to no visibility section, which is the
        // behaviour ADR-0011 already assumes for it.
        //
        // Its draw calls are deliberately **not** added to `stats`. Being exempt
        // from both tiers, folding them in would shift the denominator that
        // ADR-0011's and the roadmap's PVS effectiveness figures are quoted
        // against - a silently moved percentage nobody would think to question.
        if let Some(sky) = &self.sky {
            let _ = sky.draw(&mut pass, None, None, None);
        }
        let mut stats = self.track.draw(
            &mut pass,
            self.visibility.as_ref().map(|v| &v.sections),
            visible_set.as_ref(),
            frustum.as_ref(),
        );
        // After the track, so a pad sitting flush on the surface wins the depth
        // test rather than z-fighting whatever it was authored on top of.
        // Frustum culling applies; the PVS does not, because a pad carries no
        // `section` id to look up - the same exemption the sky takes, for a
        // different reason.
        if let Some(pads) = &self.pads {
            stats.add(pads.draw(&mut pass, None, None, frustum.as_ref()));
        }
        if let Some(pads) = &self.weapon_pads {
            stats.add(pads.draw(&mut pass, None, None, frustum.as_ref()));
        }
        // Skipped outright in the cockpit view rather than moved or scaled away:
        // the original sets one flag on the craft and draws no hull, and a draw
        // call not issued is the only version of that with no chance of a stray
        // polygon across the middle of the screen. See `Race::draws_own_ship`.
        // Only the *player's* hull is skipped in the cockpit view. The opponents
        // in front are exactly what a cockpit view is for.
        for (index, drawable) in self.ships.iter().take(drawn).enumerate() {
            if index == 0 && !race.draws_own_ship() {
                continue;
            }
            stats.add(drawable.draw(&mut pass, None, None, None));
        }
        // Rockets, with the hulls: an opaque painted model that occludes and is
        // occluded, not an effect. Bounded by how many matrices were written
        // this frame, for the same reason the plumes are - a drawable whose
        // uniform buffer went unwritten would draw at last frame's pose.
        for drawable in self.rockets.iter().take(rocket_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None));
        }
        // After the ships, so the hulls' depth is already in the buffer: a
        // plume's own blend pipeline writes no depth, the same reasoning as
        // the flares below. One draw per boosting craft, gated on the same
        // craft's plume the write above was gated on - a plume drawn from a
        // uniform buffer that was not written this frame would be last frame's
        // pose.
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            let Some(boost) = boost else {
                continue;
            };
            if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            stats.add(boost.draw(&mut pass, None, None, None));
        }
        if let Some(collision) = &self.collision {
            stats.add(collision.draw(&mut pass, None, None, None));
        }
        // Last, and that ordering is load-bearing: the flare tests depth but does
        // not write it, so the hull's depth has to already be in the buffer for the
        // flare to be occluded by it. Sparks are the same kind of blended,
        // depth-tested-not-written geometry, so they follow right after for the
        // same reason.
        self.exhaust.borrow().draw(&mut pass);
        self.sparks.borrow().draw(&mut pass);
        // The scene pass has to close before the bloom can sample what it drew,
        // so this ends the borrow rather than waiting for the scope to.
        drop(pass);

        // The recovered post-process, reading the alpha channel the ribbon and
        // the flare stamped and adding a blurred copy of the masked colour back
        // over the frame. `view` is the resolved image in both the MSAA and the
        // single-sample case, which is why this runs on it rather than on
        // `attachment_view`. See `oag_render::post::bloom`.
        if let Some(bloom) = &self.bloom {
            bloom.render(device, encoder, view);
        }
        stats
    }
}

fn depth_texture(device: &wgpu::Device, size: (u32, u32), sample_count: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// The multisampled colour attachment MSAA draws into, resolved into the
/// caller's own target at the end of [`Scene::render`]'s one pass.
///
/// `None` at `sample_count` 1: a single-sample scene draws straight into the
/// caller's view and there is nothing here to resolve.
fn msaa_color_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    sample_count: u32,
) -> Option<wgpu::Texture> {
    (sample_count > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("race msaa colour"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    })
}

/// What a headless capture should do before it draws.
#[derive(Debug, Clone)]
pub struct CaptureOptions {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Ticks to advance the simulation first.
    pub ticks: u32,
    /// Buttons held on every one of those ticks.
    pub held: u32,
    /// Drive the run from a committed `.inputs` script instead of `held`/
    /// `pressed` - the same file `scripts/psp-trace.py --script` feeds the
    /// emulator, so one authored input produces both sides of a visual
    /// comparison. Ticks past the script's end coast with everything
    /// released.
    pub input_script: Option<oag_trace::script::Script>,
    /// Which control scheme maps the buttons. `[controls] scheme`.
    ///
    /// Here rather than left at the default because the novice sideshift is a
    /// *gesture*, and a headless run is the only way to exercise one without a
    /// window: `--scheme novice --hold cross,l --press left` is the flick.
    pub scheme: ControlScheme,
    /// Buttons pressed and released on alternating ticks, for gestures that read
    /// an edge rather than a level.
    ///
    /// The same convention the front-end capture uses, so `--press` means one
    /// thing across the whole tool. A tap every other tick is well inside the
    /// veteran sideshift's `0.25 s` window, which makes `--press l` a double-tap
    /// generator.
    pub pressed: u32,
    /// Image size.
    pub size: (u32, u32),
    /// Print a telemetry line every this many ticks. Zero prints none.
    pub log_every: u32,
    /// Keep the player's pickup slot topped up with this weapon.
    ///
    /// **A debug affordance for captures, and the reason it exists is worth
    /// keeping.** A weapon is only visible once something fires it, and a
    /// capture holds the throttle without steering, so it never crosses a
    /// `Weapon Pad` and never receives a pickup. Every visual change to a
    /// projectile was therefore unverifiable from a screenshot - two rocket
    /// changes shipped blind before this existed, and one was wrong: three
    /// pieces of track scenery were read as a fanned volley. See
    /// [`Race::rocket_model_matrices`].
    ///
    /// Written **outside** [`Race::tick`], only when the slot is already empty,
    /// so firing still spends it and nothing here reaches a determinism hash.
    pub give: Option<oag_formats::weapons::Weapon>,
    /// The shape to draw at inside the frame, leaving bars.
    ///
    /// A capture is a picture of a window, so it letterboxes the way a window
    /// does. At the default `--size`, which is the PSP's own shape, every value
    /// of this fills the frame and nothing changes.
    pub aspect: crate::display::Aspect,
    /// Anisotropic filtering level for the track and ship textures.
    pub anisotropy: Anisotropy,
    /// Whether the recovered bloom runs - see `crate::settings::Graphics::bloom`,
    /// which defaults it **off** until its magnitude is calibrated.
    pub bloom: bool,
    /// Which adapter to draw with, for the same reason `aspect` and `fov` are
    /// here: a capture is only evidence about what a player sees if it was
    /// drawn on the device they see it on. A driver is exactly the kind of
    /// thing a rendering difference gets blamed on, so a capture that quietly
    /// used a different one would be the wrong picture to argue from.
    ///
    /// There is no surface here, so an adapter that could not present is still
    /// eligible - which is the one way this list can be wider than the menu's.
    pub renderer: crate::display::Renderer,
    /// The field-of-view setting, for the same reason `aspect` is here: a
    /// capture should frame what a player at these settings would have seen.
    pub fov: crate::display::Fov,
    /// Whether the view frustum culls before the frame is drawn.
    ///
    /// Honoured rather than forced off, so that the screenshot comparison this
    /// project already claims for `[graphics] frustum_culling` can actually be
    /// run from a capture, and so a report of geometry going missing can be
    /// attributed to a tier rather than guessed at.
    pub frustum_culling: bool,
    /// Whether the authored PVS culls before the frame is drawn.
    ///
    /// Here, and honoured, so that `--screenshot` with `[graphics] pvs_culling`
    /// on and off produces two images to compare. **That comparison is the only
    /// way to show the association rule in `oag_render::pvs` places geometry in
    /// the right sections rather than merely in some section**, and it is the
    /// bar that setting has to clear before it can default on - the same one
    /// frustum culling passed. Frustum culling stays off in a capture either
    /// way, so the two images differ by this tier alone.
    pub pvs_culling: bool,
    /// Whether the inferred trackside texture animations run.
    ///
    /// Honoured for the same reason the two culling tiers are: two captures
    /// differing only by this setting are how `[graphics] animated_textures`
    /// gets checked against the running original, and that check is the whole
    /// reason the setting exists.
    pub animated_textures: bool,
    /// How strong the boost's field-of-view kick is. Honoured for a sharper
    /// version of the same reason: the effect is **authored**, so a capture meant
    /// to be compared against the running original wants it at
    /// [`crate::display::BoostFovKick::OFF`], and that comparison is the only
    /// way anyone will find out whether the original has something like it.
    pub boost_fov_kick: crate::display::BoostFovKick,
    /// Which of the three perspectives to render from.
    ///
    /// Honoured because a headless capture is the **only** way to get a frame of
    /// the cockpit view without a window, and therefore the only way anyone
    /// checks it: `--camera-view internal --screenshot`. `[graphics] camera_view`.
    pub camera_view: crate::display::CameraView,
    /// Which anti-aliasing the scene draws with. Honoured for the same reason
    /// the two culling tiers are: a capture is how `[graphics] anti_aliasing`
    /// gets compared against itself off and against the running original.
    pub anti_aliasing: crate::display::AntiAliasing,
    /// Force the exhaust into the state it holds this many seconds after a
    /// speed pad entry, at saturated intensity, before the frame is drawn.
    ///
    /// `--pose-boost`. A posed capture (`--pose-from --ticks 0`) never crosses
    /// a pad, so this is the only way a frame comparison can see the boost
    /// visuals at a chosen age. The state is reached by replaying
    /// [`Exhaust::advance`] rather than by poking fields, so what is captured
    /// is the same trajectory a real crossing produces.
    pub pose_boost: Option<f32>,
    /// With [`Self::pose_boost`]: the intensity at the entry tick, instead of a
    /// saturated ramp. `--pose-intensity`.
    ///
    /// See [`Race::force_boost_state`] for why a saturated default is the wrong
    /// one to compare a teleported capture against.
    pub pose_intensity: Option<f32>,
    /// With [`Self::pose_boost`]: the speed in units/s to advance the exhaust
    /// at, instead of `120`. `--pose-speed`.
    pub pose_speed: Option<f32>,
    /// Capture the frame the way a **window** presents it, rather than the
    /// scene the way it is drawn.
    ///
    /// `None` is the ordinary capture: the scene, straight out of the target it
    /// was drawn into, ungraded, at exactly `size`. That is the right default
    /// for a bug report, and it is deliberately not a picture of a window - see
    /// [`crate::upscale`].
    ///
    /// `Some` puts the whole presentation path in the way: the render scale,
    /// the upscaler, the grade and the aspect bars. **This is the only way to
    /// see an upscaler's output at all**, because the ordinary path never
    /// reaches the blit, and it is therefore what a still-frame comparison
    /// between resamplers has to use. It is also, necessarily, an sRGB pipeline
    /// throughout, exactly as a window is.
    pub presented: Option<Presented>,
}

/// [`Presented`] with the scene size worked out.
#[derive(Debug, Clone, Copy)]
struct PresentedState {
    scene_size: (u32, u32),
    presentation: crate::upscale::Presentation,
}

/// The settings a `--presented` capture needs that an ordinary one does not.
#[derive(Debug, Clone, Copy)]
pub struct Presented {
    /// What fraction of the aspect rectangle the scene is drawn at.
    pub render_scale: crate::display::Scale,
    /// The upscaler, its sharpness, and the grade.
    pub presentation: crate::upscale::Presentation,
}

/// Runs a race headless and writes one frame to a PNG.
///
/// Through the same [`Scene`] the window draws, for the same reason
/// [`crate::capture`] goes through the same renderer the front end's window does: a
/// separate capture path would prove nothing about what a player sees.
///
/// `audio` is advanced one step per simulation tick, in the same loop, so a
/// `--dump-audio` capture is as long as the ticks it was given whatever the
/// machine's speed. Writing the WAV is the caller's, not this function's: a
/// capture reached from [`crate::capture::run`] has already accumulated the
/// front end's ticks into the same buffer, and finishing here would truncate
/// the file to the race leg.
///
/// # Errors
///
/// Propagates adapter and device creation, pipeline building, the readback map and
/// the file write.
pub fn capture(
    loaded: Loaded,
    options: &CaptureOptions,
    audio: &mut crate::audio::Audio,
) -> Result<()> {
    let (width, height) = options.size;
    let Loaded {
        setup,
        hud,
        track_model,
        liveries,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        rocket_model,
        fog_volumes,
        visibility,
        flare,
        noise,
        ..
    } = loaded;
    let mode = setup.mode;
    let mut race = Race::start(setup);
    race.set_boost_fov_kick(options.boost_fov_kick);
    race.set_camera_view(options.camera_view);
    race.set_control_scheme(options.scheme);

    let mut held = HeldButtons::new(options.held);
    for tick in 0..options.ticks {
        if let Some(script) = &options.input_script {
            held.set_held(script.at(tick as usize).buttons);
        } else {
            held.pulse(options.pressed, options.held, tick.is_multiple_of(2));
        }
        let snapshot = held.snapshot();
        // Before the tick, so `spend_pickup` can fire it on this tick's edge.
        if let Some(weapon) = options.give
            && race.world.ships[0].pickup.weapon.is_none()
        {
            race.world.ships[0].pickup.weapon = Some(weapon);
        }
        race.tick(&snapshot);
        // Inside the tick loop and not beside it, for the reason the exhaust
        // and the chase camera are advanced from inside `Race::tick`: what a
        // capture produces has to be a function of the tick count and nothing
        // else, or the same command line gives a different file on a slower
        // machine.
        audio.tick();
        if options.log_every > 0 && race.world.tick.is_multiple_of(u64::from(options.log_every)) {
            println!("{}", describe(&race.telemetry()));
        }
    }
    if let Some(age) = options.pose_boost {
        race.force_boost_state(age, options.pose_intensity, options.pose_speed);
    }
    println!(
        "after {} tick(s): {}",
        race.world.tick,
        describe(&race.telemetry())
    );

    let instance = crate::adapter::instance();
    let adapter = crate::adapter::choose(&instance, None, &options.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-game race offscreen"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    // **`Rgba8Unorm` on both paths now**, because every shader in this pipeline
    // writes gamma-space values and nothing may encode them again - see
    // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
    //
    // `--presented` used to be `Rgba8UnormSrgb` on the grounds that it is a
    // picture of a *window* and should take the window's format. It still is,
    // and it still does: the window stopped encoding in the same change. That
    // the two agreed on the label and not on the value is what made a
    // `--screenshot` and a `--presented` capture of the same frame disagree
    // about the boost plume by up to 73/255, the plume being the one surface
    // whose texels were already re-encoded to compensate for the old upload.
    // The offscreen target's non-sRGB twin, and therefore FSR 1, still work -
    // `remove_srgb_suffix` on a format that has no suffix is the identity. See
    // `crate::upscale`.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // The scene's own size, which presented is the aspect rectangle scaled and
    // otherwise is the whole capture.
    let presented = options.presented.map(|state| PresentedState {
        scene_size: crate::upscale::target_size(
            crate::display::viewport((width, height), options.aspect),
            state.render_scale,
            device.limits().max_texture_dimension_2d,
        ),
        presentation: state.presentation,
    });
    // What the scene - and so its depth buffer - is actually drawn at. A depth
    // attachment whose size does not match the colour one is a validation
    // error, not a bad picture.
    let scene_size = presented.map_or((width, height), |state| state.scene_size);
    let scene = Scene::new(
        &device,
        &queue,
        track_model,
        &liveries,
        collision_model,
        sky_model,
        pad_model,
        weapon_pad_model,
        mode,
        rocket_model,
        flare,
        noise,
        format,
        scene_size,
        options.anisotropy,
        options.bloom,
        visibility,
        options.anti_aliasing,
        fog_volumes,
    )?;

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        // `TEXTURE_BINDING` because the bloom's bright pass samples the frame
        // it was just drawn into - see `oag_render::post::bloom`.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let surface = target.create_view(&wgpu::TextureViewDescriptor::default());
    // Where the scene is drawn. Presented, that is an offscreen target at the
    // render scale which the blit later stretches into the aspect rectangle -
    // the same two-step a window does. Otherwise it is the capture texture
    // itself, and the scene draws into a sub-rectangle of it directly.
    let mut framebuffer = match presented {
        Some(state) => Some(
            crate::upscale::Framebuffer::new(&device, format, state.scene_size)
                .context("building the upscale pipeline")?,
        ),
        None => None,
    };
    let view = match &framebuffer {
        Some(framebuffer) => framebuffer.view().clone(),
        None => surface.clone(),
    };

    // Texture copies want rows aligned to 256 bytes, so the readback buffer is
    // usually wider than the image and needs unpadding.
    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("race capture"),
    });
    // Shaped the same way a window is, so a screenshot frames what a player
    // would have seen at that size rather than a differently-cropped picture.
    // Presented, the offscreen target *is* that rectangle and the bars are what
    // the blit clears around it, so the scene fills its target instead.
    let rect = crate::display::viewport((width, height), options.aspect);
    let viewport = match presented {
        Some(state) => (
            0.0,
            0.0,
            state.scene_size.0 as f32,
            state.scene_size.1 as f32,
        ),
        None => rect,
    };
    // Both tiers follow their settings, because two captures differing only by
    // one of them are how that tier gets validated - see
    // `CaptureOptions::frustum_culling` and `CaptureOptions::pvs_culling`.
    scene.render(
        &device,
        &queue,
        &mut encoder,
        &view,
        &race,
        viewport,
        options.fov,
        options.frustum_culling,
        options.pvs_culling,
        options.animated_textures,
    );

    // The HUD, into the same target. Without this a race screenshot would show
    // the track and no HUD at all, because unlike the front end's capture this
    // path has no `Framebuffer` and so no overlay pass of its own - which would
    // make `--screenshot` useless for the one thing it is most wanted for.
    //
    // **Every path is `Rgba8Unorm` since ADR-0020** - this capture, a
    // `--presented` capture and the window alike - so `Renderer::new`'s fork of
    // the sprite sheet's texture format on `format.is_srgb()` now always takes
    // the raw side, and the HUD's art reaches all three the same way. It used
    // to differ: an ordinary capture was raw while the window and `--presented`
    // were sRGB, which is exactly the disagreement the ADR removed (measured at
    // up to `73/255` on the plume). Text and fills go through the R8 coverage
    // atlas and were unaffected either way. See `docs/ui/hud.md`.
    match crate::hud::Overlay::new(&device, &queue, format, &hud) {
        Ok(Some(mut overlay)) => overlay.draw(
            &device,
            &queue,
            &mut encoder,
            &view,
            &race.readout(),
            viewport,
        ),
        Ok(None) => println!("no HUD layout: capturing without one"),
        Err(why) => println!("the HUD overlay did not build ({why}); capturing without one"),
    }
    // The upscaler, the grade and the blit, through exactly the call the
    // window's frame loop makes.
    if let (Some(framebuffer), Some(state)) = (framebuffer.as_mut(), presented) {
        framebuffer.resolve(
            &device,
            &queue,
            &mut encoder,
            &surface,
            rect,
            &state.presentation,
        );
    }

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;
    let mapped = slice
        .get_mapped_range()
        .context("mapping the readback buffer")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();

    let png = oag_formats::png::encode_rgba(width, height, &pixels);
    std::fs::write(&options.path, png)
        .with_context(|| format!("writing {}", options.path.display()))?;
    println!("wrote {} ({width}x{height})", options.path.display());
    Ok(())
}

/// One telemetry line, for a log or a report.
#[must_use]
pub fn describe(telemetry: &Telemetry) -> String {
    format!(
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}\
         {}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
        match (telemetry.pickup, telemetry.projectiles) {
            (None, 0) => String::new(),
            (held, count) => {
                let held = held.map_or("-", oag_formats::weapons::Weapon::as_type);
                format!("  holding {held}  in the air {count}")
            }
        },
    )
}

#[cfg(test)]
mod tests;
