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

use anyhow::{Context, Result};
use oag_assets::pulse;
use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Quat, Vec3};
use oag_core::{Rng, TickClock, TickRate};
use oag_formats::track::{AiTrack, Sample, StartPosition};
use oag_formats::vex;
use oag_formats::{collision, handling};
use oag_gameplay::{
    ControlScheme, InputSnapshot, Pose, Ship, World, collision_world, handling_for, ship_controls,
    to_format_class,
};
use oag_input::Keyboard;
use oag_physics::{CollisionWorld, Environment, Evaluated, Handling, SpeedClass};
use oag_race::{Course, Mode, RaceState};
use oag_render::camera::chase::{Chase, ChaseParams, Target};
use oag_render::camera::internal::InternalParams;
use oag_render::collision as render_collision;
use oag_render::exhaust::{self, Exhaust, FlareTexture};
use oag_render::mesh::{DrawCall, Model};
use oag_render::mesh_render::Anisotropy;
use oag_render::pvs::{
    DrawSections, PlacementStats, SectionPadding, SwapConflicts, UNPLACED, VisibleSet,
};
use oag_render::sparks::{self, Sparks};
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
pub const DEFAULT_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
///
/// **Assegai, not Feisar** - chosen to match the reference scenario used for
/// every PPSSPP capture this project has taken (Time Trial, Venom, Talon's
/// Junction White, Assegai; see
/// `docs/reverse-engineering/ppsspp-debugger.md`'s "reference scenario"
/// section), so `just play --race`'s defaults and a captured trace are
/// directly comparable without passing `--team`/`--class` every time.
pub const DEFAULT_TEAM: &str = "Assegai";

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

/// The world's generator seed.
///
/// Fixed and arbitrary: nothing in the recovered force law draws from the
/// generator, so this exists only so the seed is written down in one place for
/// whenever something does. See `crates/core/src/rng.rs`.
pub const SEED: u64 = 1;

/// Seed for the exhaust's flicker, kept distinct from [`SEED`].
///
/// The exhaust draws two numbers per tick and the simulation must not see them:
/// sharing a generator would let the picture change the physics, which is what
/// `docs/architecture/determinism.md` forbids. A separate seed also means the two
/// streams cannot be mistaken for each other when reading a capture.
pub const EXHAUST_SEED: u64 = 0xe8_a5_71_00;

/// Seed for spark spawn parameters, kept distinct from [`SEED`] and
/// [`EXHAUST_SEED`] for the same determinism reason.
pub const SPARKS_SEED: u64 = 0x5_9a_2b_00;

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
    let model = if mode == Mode::Zone { "Zone" } else { "Ship" };
    format!(r"Data\Ships\{team}\{model}.vex")
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
        "Zoneboost"
    } else {
        "shipboost"
    };
    format!(r"Data\Ships\{team}\{model}.vex")
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
/// See [`pulse::Archives::read_preceding`] for the rule itself and the
/// evidence behind it.
fn ps2_texture_set(
    archives: &mut pulse::Archives,
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
    /// Archive entry name of the track's `.vex`.
    pub track: String,
    /// Team name, which selects both the handling stats and the model.
    pub team: String,
    /// Speed class the handling parameters are read for.
    pub class: SpeedClass,
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
    /// Replace the disc's authored fov (in the same unrecovered unit, read as
    /// degrees) with this value. `None` keeps the authored one - the honest
    /// default while the unit stands unrecovered, and the calibration knob for
    /// settling it: iterate until the framing matches a captured shot.
    pub fov_deg: Option<f32>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: crate::source::DEFAULT_IMAGE.to_string(),
            track: DEFAULT_TRACK.to_string(),
            team: DEFAULT_TEAM.to_string(),
            class: SpeedClass::Venom,
            mode: Mode::default(),
            ribbon: false,
            collision: false,
            lod: mesh::Lod::Both,
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
    /// The `engine_flare` locator, in the ship model's own space.
    ///
    /// `None` means the model carries no `Engine Flare` node, and the exhaust is
    /// then not drawn rather than guessed at. Every team whose `Ship.vex`
    /// resolves by name has **exactly one**, a direct child of `world` - see
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`. One nozzle, centred, not one
    /// per visible engine.
    pub nozzle: Option<Vec3>,
    /// The `Ship Collision Fx` locators, in the ship model's own space.
    ///
    /// The original attaches up to 10 and `Ship_DispatchCollisionFx`
    /// (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) triggers the
    /// one **nearest the contact**; the spark burst then emits from that
    /// node as it rides the hull. Empty means the model authors none, and
    /// the burst falls back to anchoring at the contact point itself.
    pub collision_fx: Vec<Vec3>,
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
    /// What to draw for the ship.
    pub ship_model: Model,
    /// What to draw for the boost plume, additively, while
    /// [`Exhaust::plume_visible`] is true.
    ///
    /// `Data\Ships\<Team>\shipboost.vex`, loaded beside [`Self::ship_model`].
    /// `None` when the source carries no entry under that name for this
    /// team - reported rather than failing the race, since the PS2 set may
    /// not carry it under the same name as the PSP one does. Two meshes,
    /// already spread apart in the file's own space (confirmed by rendering
    /// it with `oag-view --mesh` and by its node tree having no `Transform`
    /// between `World` and the two `Mesh` nodes), so this is drawn once in
    /// ship space rather than mounted on a locator.
    pub boost_model: Option<Model>,
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

/// The `engine_flare` locator's position in the ship model's own space.
///
/// A locator class stores a 4x4 in its 64-byte payload exactly as a `Transform`
/// does, which is why [`vex::class_world_transforms`] can compose it with the
/// parent chain. Row 3 is the translation.
///
/// Takes the **first** node if a model somehow had several. Every team checked has
/// exactly one, so this is a total order on a set of size one rather than a policy.
fn engine_flare(ship_blob: &[u8]) -> Option<Vec3> {
    let nodes = vex::nodes(ship_blob).ok()?;
    let m = vex::class_world_transforms(ship_blob, &nodes, vex::CLASS_ENGINE_FLARE)
        .into_iter()
        .next()?;
    Some(Vec3::new(m[12], m[13], m[14]))
}

/// Every `Ship Collision Fx` locator's position in the ship model's own
/// space - the same 4x4-payload decode as [`engine_flare`], kept all rather
/// than first: the original picks the nearest to each contact.
fn collision_fx_locators(ship_blob: &[u8]) -> Vec<Vec3> {
    let Ok(nodes) = vex::nodes(ship_blob) else {
        return Vec::new();
    };
    vex::class_world_transforms(ship_blob, &nodes, vex::CLASS_SHIP_COLLISION_FX)
        .into_iter()
        .map(|m| Vec3::new(m[12], m[13], m[14]))
        .collect()
}

/// Decodes a `.mip` texture out of the archive set.
///
/// Returns the pixels and a line for the load report, or the reason it could not -
/// **as text, not as `None`**. A missing entry and a blob that does not parse are
/// different problems with different fixes (name mining versus the decoder), and a
/// silent fallback hides which one happened.
fn mip_texture(
    archives: &mut pulse::Archives,
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
    let mut archives = pulse::Archives::open(&options.source)?;
    report.push(archives.layout.describe());

    let spec = archives
        .locate(&options.track)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} is in none of this source's archives ({})",
                options.track,
                archives.layout.describe()
            )
        })?
        .to_string();

    let (ai, label) = track_render::load(&spec, &options.track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    let read = |archives: &mut pulse::Archives, name: &str| -> Result<Vec<u8>> {
        archives
            .read_name(name)
            .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
    };

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice. See the
    // wanted-change note in `docs/tools/oag-game.md`. On the PS2 that costs more
    // than it does on the PSP, where nothing is compressed at all: 5,861 of
    // `WADS2.WAD`'s 7,200 entries are LZSS.
    let track_blob = read(&mut archives, &options.track)?;
    let start_position = start_position_of(&track_blob);
    match start_position {
        Some(slot) => report.push(format!(
            "Start Position: {:?} facing {:?}",
            slot.position, slot.forward
        )),
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes =
        collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", options.track))?;
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
            oag_formats::pads::volumes(&track_blob, &nodes, oag_formats::vex::CLASS_SPEEDUP_PAD)
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

    let stats_name = handling::entry_name(&options.team);
    let stats_blob = read(&mut archives, &stats_name)?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
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

    let ship_name = ship_entry_name(&options.team, options.mode);
    let ship_blob = read(&mut archives, &ship_name)?;
    let mut ship_model = mesh::build_with_textures(&ship_name, &ship_blob, None, options.lod)?;
    // The PS2 signature: `Texture` nodes exist (the model wants textures) but
    // every one is missing (its embedded block was empty). Only then is the
    // directory-position heuristic worth trying - see `ps2_texture_set`.
    if !ship_model.textures.is_empty()
        && ship_model.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(&mut archives, &ship_name)
    {
        ship_model =
            mesh::build_with_textures(&ship_name, &ship_blob, Some(external), options.lod)?;
    }
    report.push(format!(
        "{ship_name}: {} triangle(s), model centre {:?}, radius {:.2}",
        ship_model.indices.len() / 3,
        ship_model.centre,
        ship_model.radius
    ));

    // The plume `ExhaustFlare_Init` reveals once `boost_timer` passes
    // `exhaust::BOOST_GATE` - `shipboost.vex`, or `Zoneboost.vex` in Zone mode,
    // matching whichever hull `ship_entry_name` picked. Absence is reported
    // rather than failing the race: the PS2 set may not carry it under the
    // same name the PSP one does, and a missing boost plume is a missing
    // feature, not a broken load.
    let boost_name = boost_entry_name(&options.team, options.mode);
    let boost_model = match archives.read_name(&boost_name) {
        Ok(blob) => match mesh::build_with_textures(&boost_name, &blob, None, options.lod) {
            Ok(model) => {
                report.push(format!(
                    "{boost_name}: {} triangle(s) - drawn additively while the plume is up",
                    model.indices.len() / 3
                ));
                // **The authored vertex alpha is left alone here, and the
                // blend at the draw site weights by it per fragment.** This
                // used to premultiply alpha into the RGB, and that was a real
                // bug rather than a compensation: premultiplying at a vertex
                // and then letting the rasteriser interpolate is not the same
                // arithmetic as interpolating and then multiplying, and for
                // this model the difference is the whole visual.
                //
                // What the authored data holds, measured on every batch of
                // every PSP team's `shipboost.vex` and pinned by
                // `boost_plume_ground_truth.rs`, is why:
                //
                // - the vertex colours are exactly **two** values,
                //   `(255, 98, 5, 0)` on the rim and `(255, 255, 255, 255)` in
                //   the core, 29 and 22 of a 51-vertex batch, nothing between;
                // - `pulse_boost2_ADD` is 64x16 with a **constant alpha of
                //   238**, so the texture supplies no gradient either.
                //
                // Premultiplied per vertex, the rim becomes `(0, 0, 0)` and
                // what crosses the fin is black-to-white: **the authored
                // orange exists nowhere on it**, which is exactly the
                // "reads white and un-orange" symptom. Weighted per fragment,
                // colour interpolates orange-to-white while alpha
                // interpolates `0`-to-`1` and the two meet at the fragment,
                // leaving a dimmed orange fringe that fades out - the
                // original's own feathered edge.
                //
                // Measured against the first matched-pose pad capture, over
                // the magenta signature in a crop around the craft: the
                // original reads `(224, 166, 227)`, the premultiplied build
                // `(151, 93, 212)` - blue-dominant, hue lost - and the raw
                // build `(222, 168, 230)`, within three units per channel on
                // all three. See the draw site in `Scene::new` for the full
                // table and for what remains unrecovered: the GE's own route
                // for this alpha, since `Mesh_SetBatchDrawState` programs
                // `GU_FIX` white on both sides *after* the material's own
                // display list.
                //
                // Two suspects were raised and **refuted** earlier, recorded
                // so nobody spends the same afternoon on them. The plume's
                // authored `u` never leaves the first texel (`[0.000, 0.008]`
                // against a `v` of `[0.031, 0.953]`), so it samples one column
                // of `pulse_boost2_ADD`; rendering with `u` scaled by 128, and
                // again with `u`/`v` swapped, changed the picture not at all.
                // And the texture is neither dropped nor mis-bound: replacing
                // it with hard horizontal stripes banded the two trailing
                // streaks while leaving the wedges flat. What that second test
                // did expose is still open - the two short batches (9 and 10
                // vertices) decode to a **single** UV point, `u [0.008,
                // 0.008]`, `v [0.031, 0.031]`, so no texture content can reach
                // them whatever the scale, while the 51-vertex batches carry a
                // real `v` sweep and do band.
                // **The load-time texel re-encode that used to sit here is
                // gone, and putting it back would be a double-encode.** It
                // existed only to cancel `Drawable`'s `Rgba8UnormSrgb` upload,
                // whose sampler linearised the disc's bytes before the shader
                // saw them. That upload is raw now
                // ([ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)),
                // so the sampler already hands the shader the disc's own
                // values and there is nothing left to compensate for. The
                // effect it was compensating for is real - `One`/`One` on
                // linearised texels crushes the halo's mid-tones by ~30% of
                // encoded brightness - which is why the fix moved to the
                // upload rather than being dropped.
                Some(model)
            }
            Err(e) => {
                report.push(format!(
                    "{boost_name}: {} bytes, does not parse ({e}) - no boost plume this run",
                    blob.len()
                ));
                None
            }
        },
        Err(e) => {
            report.push(format!(
                "{boost_name}: not in the archive set ({e}) - no boost plume this run"
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
        let mut track_model =
            mesh::build_with_textures(&options.track, &track_blob, None, options.lod)?;
        // Same PS2 signature and the same directory-position heuristic as the
        // ship above. Checked separately for tracks specifically (not just
        // assumed from the ship result): the entry directly before a track's
        // own `.vex` decodes as a texture set with the same entry count as the
        // model's `Texture` nodes on 27 of the 32 `<n>_Track`/`track_reversed`
        // pairs on the PS2 disc, off by only 1-2 slots (never more) on the
        // rest - see `docs/formats/ps2-texture.md`.
        if !track_model.textures.is_empty()
            && track_model.textures.iter().all(Option::is_none)
            && let Some(external) = ps2_texture_set(&mut archives, &options.track)
        {
            ps2_track_textures = Some(external.clone());
            track_model = mesh::build_with_textures(
                &options.track,
                &track_blob,
                Some(external),
                options.lod,
            )?;
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
        let sky = mesh::build_sky(&options.track, &track_blob, ps2_track_textures.clone())?;
        if sky.indices.is_empty() {
            report.push("the track authors no Skycube; the sky stays black".to_string());
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
        let pads = mesh::build_pads(&options.track, &track_blob, ps2_track_textures.clone())?;
        if pads.indices.is_empty() {
            report.push("the track authors no Speedup Pad geometry".to_string());
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
        Some(&ship_model),
        Some(&track_model),
        sky_model.as_ref(),
        pad_model.as_ref(),
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

    let nozzle = engine_flare(&ship_blob);
    match nozzle {
        Some(at) => report.push(format!(
            "{ship_name}: engine_flare locator at {at:?} in model space"
        )),
        None => report.push(format!(
            "{ship_name}: no Engine Flare node - the exhaust will not be drawn"
        )),
    }

    let collision_fx = collision_fx_locators(&ship_blob);
    report.push(if collision_fx.is_empty() {
        format!("{ship_name}: no Ship Collision Fx nodes - sparks anchor at the contact point")
    } else {
        format!(
            "{ship_name}: {} Ship Collision Fx locator(s) for the spark anchor",
            collision_fx.len()
        )
    });

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
            nozzle,
            collision_fx,
            speedup_pads,
            class_gravity_scale,
            pose_override,
            camera_override: options.camera,
        },
        hud,
        track_model,
        collision_model,
        sky_model,
        pad_model,
        fog_volumes,
        ship_model,
        boost_model,
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
/// The two layouts this does not reach - `Arcade_HUD.xml` and
/// `Elimination_HUD.xml` - are in [`crate::hud::layouts`] waiting for the modes
/// that use them.
#[must_use]
pub const fn hud_layout(mode: Mode) -> &'static str {
    match mode {
        Mode::TimeTrial | Mode::SpeedLap => crate::hud::layouts::TIME_TRIAL,
        Mode::Zone => crate::hud::layouts::ZONE,
    }
}

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
fn load_hud(
    archives: &mut pulse::Archives,
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
    // `read_image`, not `read_name`: the PS2 keeps this atlas under an entry
    // its own XML's name does not hash to. See `pulse::PS2_IMAGES`.
    match archives.read_image(crate::hud::ATLAS) {
        Ok(blob) => {
            let mut notes = Vec::new();
            let built =
                crate::sprite::Sheet::build(&[(crate::hud::ATLAS.to_string(), blob)], &mut notes);
            report.extend(notes);
            if built.get(crate::hud::ATLAS).is_some() {
                report.push(format!(
                    "HUD atlas {}: {}x{} sheet",
                    crate::hud::ATLAS,
                    built.width,
                    built.height
                ));
                sheet = built;
            } else {
                report.push(format!("HUD atlas {} did not decode", crate::hud::ATLAS));
            }
        }
        Err(why) => report.push(format!(
            "HUD atlas {} unavailable ({why})",
            crate::hud::ATLAS
        )),
    }

    let font = hud_font(archives, pulse::names::fonts::HUD, report);
    let small_font = hud_font(archives, pulse::names::fonts::SMALL, report);

    // The HUD's captions are `idstring` keys - `IG_HUD_LAP`, `IG_HUD_BEST` - and
    // without a table `StringTable::get_or_id` falls back to the key itself, which
    // put `ig_hud_lap` on screen where `LAP` belongs. The preferred language is the
    // player's saved one; a race reached through `--race` has no settings to read,
    // so this takes the chain's default rather than threading one through.
    let languages = crate::boot::load_languages(archives, report);
    let strings = crate::boot::load_strings(archives, &languages, None, report);

    crate::hud::Assets {
        layout,
        sheet,
        font,
        small_font,
        strings,
    }
}

/// Reads one `.fnt`, falling back to the built-in glyphs and saying so.
///
/// The same shape as `crate::boot::load_font`, which reads the front end's
/// `Default` font, and now the same read: both go through
/// [`pulse::Archives::read_font`], so a PS2 source finds the glyph atlas the
/// disc keeps in the entry after the `.fnt` rather than falling back to 5x7.
/// Kept separate only so the report line says which font is being talked about.
///
/// Both HUD fonts are pre-outlined on both discs - six distinct greys, alpha
/// covering glyph *plus* border - so `Atlas::from_font`'s body/outline split
/// applies unchanged here; see `docs/formats/fnt.md`.
fn hud_font(
    archives: &mut pulse::Archives,
    name: &str,
    report: &mut Vec<String>,
) -> crate::font::Atlas {
    match archives.read_font(name).map_err(|e| e.to_string()) {
        Ok(font) => {
            report.push(format!(
                "HUD font {name}: {}x{} atlas, {} glyph(s), line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            crate::font::Atlas::from_font(&font)
        }
        Err(why) => {
            report.push(format!(
                "HUD font {name} unavailable ({why}); drawing with 5x7"
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
    let node = nodes
        .iter()
        .find(|node| node.class_id == vex::CLASS_START_POSITION)?;
    oag_formats::track::start_position(blob.get(node.payload())?)
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
}

/// A race in progress: the world, the track it is on, and the camera behind it.
#[derive(Debug)]
pub struct Race {
    /// The simulation state.
    pub world: World,
    collision: CollisionWorld,
    spline: Spline,
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
    /// Ticks left before a `Reset` contact can respawn again.
    ///
    /// See [`RESPAWN_COOLDOWN_TICKS`].
    respawn_cooldown: u32,
    /// How many respawns have happened back to back, for [`RESPAWN_GIVE_UP`].
    respawns_in_a_row: u32,
    /// Set once respawning has given up, so the complaint is printed once.
    respawn_disabled: bool,
    /// How many times the ship has been respawned this race, for tests and for
    /// the load report.
    respawns: u32,
    /// The exhaust's animation state, advanced on the simulation tick.
    ///
    /// Here rather than in `World` for the same reason [`Chase`] is: it is
    /// render-only state, so it must not enter a snapshot a replay or a
    /// determinism hash reads. `physics/src/probe.rs` destructures `ShipState`
    /// exhaustively on purpose, and adding a visual field there would move the
    /// pinned hashes for no reason.
    exhaust: Exhaust,
    /// The flicker's generator, deliberately **not** `world.rng`.
    ///
    /// The exhaust draws two random numbers per tick. Taking them from the
    /// simulation's generator would make the picture change what the simulation
    /// does next - the determinism rules exist to prevent exactly that. Seeded, so
    /// a capture at tick *n* is still reproducible.
    exhaust_rng: Rng,
    /// The `engine_flare` locator in model space, when the ship model has one.
    nozzle: Option<Vec3>,
    /// Collision sparks' particle pool, advanced on the simulation tick.
    ///
    /// Here rather than in `World`, for the same reason [`Self::exhaust`] is -
    /// see `oag_render::sparks`'s module doc comment.
    sparks: Sparks,
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
    /// the latch `docs/formats/track.md` used to guess it was: `Pad_SweptTest_q`
    /// (`0x0888686c`) subtracts how far the craft moved from each entry every
    /// tick and only runs the real containment test on entries that reach zero,
    /// then stores the freshly measured distance back. A ship 900 units from a pad
    /// moving 2 units a tick is skipped for 450 ticks for the cost of one
    /// subtraction.
    ///
    /// **One slot per pad, not eight.** The original keeps a slot per racer;
    /// there is one ship here, and widening this is part of whatever change adds
    /// the other seven rather than something to carry unexercised.
    pad_distance: Vec<f32>,
    /// Which pad the ship was inside last tick, the original's `craft+0x1d0`.
    ///
    /// `None` outside every pad. Only a *change* of value counts as entering a new
    /// pad, which is what gates the Zone score - standing still on one pad does
    /// not pay repeatedly.
    pad_current: Option<usize>,
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
    /// position" case in `Pads_TestCraft_q` and takes the single-point path.
    pad_previous_position: Option<Vec3>,
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
            zone,
            spline,
            course,
            start_position,
            collision,
            handling,
            chase,
            chase_close,
            internal,
            nozzle,
            collision_fx,
            speedup_pads,
            class_gravity_scale,
            pose_override,
            camera_override,
            ..
        } = setup;

        let mut world = World::new(SEED);
        world.race = RaceState::new(mode);
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = box_inertia();
        // The pool starts full. Nothing drains it yet - see `Ship::shield` - so
        // this is what the bar reads all race, and what Zone's perfect-zone
        // recharge clamps back up to.
        ship.shield = handling.dimensions.shield;
        // The override wins outright rather than being an offset from the grid
        // slot: it exists to put the craft at a position read off somewhere
        // else, and anything added to that would make the two disagree.
        if let Some(pose) = pose_override
            .or_else(|| spawn_pose(&spline, start_position.as_ref(), &collision, &handling))
        {
            ship.place_at(pose);
        }
        world.ship_count = 1;

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        Self {
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
            respawn_cooldown: 0,
            respawns_in_a_row: 0,
            respawn_disabled: false,
            respawns: 0,
            // Cold, then snapped on the first tick. A race starts from a standing
            // start with no thrust, so there is nothing to snap *to* here.
            exhaust: Exhaust::new(),
            exhaust_rng: Rng::new(EXHAUST_SEED),
            nozzle,
            collision_fx,
            sparks: Sparks::new(),
            sparks_rng: Rng::new(SPARKS_SEED),
            // No sync frame to be mid-scrape on, so the first tick's contact -
            // if any - is always read as a fresh impact.
            sparks_cooldown: 0.0,
            sparks_anchor: None,
            // Every pad starts due for a real test. `Pad_Bind` zeroes the same
            // cache at load, so the first tick measures rather than trusting a
            // distance nothing has computed yet.
            pad_distance: vec![0.0; speedup_pads.len()],
            speedup_pads,
            class_gravity_scale,
            pad_current: None,
            pad_previous_position: None,
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

    /// How many times a `Reset` contact has respawned the ship this race.
    #[must_use]
    pub fn respawns(&self) -> u32 {
        self.respawns
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

    /// The resampled spline, for a caller that wants to measure against it.
    #[must_use]
    pub fn spline(&self) -> &Spline {
        &self.spline
    }

    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// The snapshot is mapped through [`oag_gameplay::ship_controls`], which is the
    /// one place "cross is thrust" is written down.
    pub fn tick(&mut self, snapshot: &InputSnapshot) -> Evaluated {
        let controls = ship_controls(snapshot, self.scheme);

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
        let pad_hit = self.test_speedup_pads(before);
        let ship = &mut self.world.ships[0];
        let env = Environment {
            track_sample,
            track_sample_next,
            auto_speed,
            pad_hit,
            class_gravity_scale: self.class_gravity_scale,
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

        self.respawn_cooldown = self.respawn_cooldown.saturating_sub(1);
        if self.reset_zone_touched(&env, before) {
            // `index` is where the ship was *before* this tick moved it, which is
            // as close to "last known good" as this loop can cheaply get.
            self.respawn(index);
        } else if self.respawn_cooldown == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.respawns_in_a_row = 0;
        }

        self.world.tick += 1;

        // The race rules run last of the simulation, on the position the step
        // produced and the tick it produced it on. Running them before the step
        // would test last tick's position against this tick's clock, which is a
        // whole tick of error on a quantity whose job is to be exact at one
        // instant. A track with no closed ring simply has no lap counter; the
        // load report already said so.
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
                ship.shield = (ship.shield + zone.recharge).min(max);
            }
        }

        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);

        // Advanced here, on the fixed tick, and not in the frame loop. That is
        // what makes the headless `capture` path - which calls only `tick` -
        // produce the same flare at the same tick count as the window does, and
        // it is the same reason the chase camera is advanced from here.
        let ship = &self.world.ships[0].physics;
        let thrust = ship.thrust;
        let speed = ship.body.linear_velocity.length();
        self.exhaust
            .advance(self.dt, thrust, speed, &mut self.exhaust_rng);

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

        // One sample per tick, which is what `Trail_Update` does per frame - it
        // takes no `dt` at all. The direction is the nozzle's own backwards axis so
        // a segment keeps the orientation the craft had when it was laid down,
        // rather than swinging with the current pose as the ship turns.
        if let Some(nozzle) = self.nozzle() {
            let back = -self.ship().physics.body.forward();
            self.exhaust.push_trail(nozzle, back);
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
            self.sparks
                .ignite(surface, contact.normal, evaluated.wall.impact_speed);
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
        self.sparks.advance(self.dt, anchor, &mut self.sparks_rng);

        evaluated
    }

    /// Below this much movement in a tick, the pad test is swept instead of a
    /// point.
    ///
    /// `Pad_SweptTest_q`'s `25.0`. It reads backwards at first - the *slow* case
    /// gets the more careful test - and the reason is that the interpolation is
    /// only worth anything when the step is short enough that four samples cover
    /// it. Past this the ship has moved further than a pad is deep and four points
    /// would not close the gap either, so the original stops paying for them.
    /// Confidence 65: the threshold is read off the call site rather than out of a
    /// named constant.
    const PAD_SWEEP_LIMIT: f32 = 25.0;

    /// How many interpolated points the swept test checks.
    ///
    /// `Pad_SweptTest_q` walks `t = 0.25, 0.5, 0.75, 1.0` - the destination is
    /// included and the origin is not, because the origin was this test's
    /// destination last tick.
    const PAD_SWEEP_STEPS: u32 = 4;

    /// Which way the speed pad under the ship pushes, or `None` if there is none.
    ///
    /// Reimplements `Pads_TestCraft_q` (`0x08887144`) and the two functions under
    /// it. Called once a tick with the position the tick *starts* at, and returns
    /// the direction `oag_physics`' step-15 term needs; see
    /// [`oag_physics::forces::Environment::pad_hit`], which is deliberately a
    /// per-tick containment answer rather than an entry edge.
    ///
    /// Also does the two things that happen on **entering a new** pad, because
    /// both are edges on the same value the original latches at `craft+0x1d0`:
    /// the Zone score, and arming the exhaust flare.
    fn test_speedup_pads(&mut self, position: Vec3) -> Option<Vec3> {
        if self.speedup_pads.is_empty() {
            return None;
        }

        // How far the ship travelled since this test last ran, which is what the
        // broadphase spends and what decides swept versus single-point.
        let previous = self.pad_previous_position.replace(position);
        let moved = previous.map_or(f32::INFINITY, |from| position.distance(from));

        // The swept path, destination last. A stationary ship and a long jump both
        // fall through to the single point: interpolating a zero-length step adds
        // nothing, and interpolating a long one does not close the gap.
        let sweep: Vec<Vec3> = match previous {
            Some(from) if (0.0..Self::PAD_SWEEP_LIMIT).contains(&moved) && moved > 0.0 => (1
                ..=Self::PAD_SWEEP_STEPS)
                .map(|step| from.lerp(position, step as f32 / Self::PAD_SWEEP_STEPS as f32))
                .collect(),
            _ => vec![position],
        };

        let mut hit = None;
        for (index, pad) in self.speedup_pads.iter().enumerate() {
            // The broadphase. Spend the distance travelled, and skip until it is
            // used up. `moved` is infinite on the first tick, so every pad is
            // measured once before any of them is skipped.
            self.pad_distance[index] -= moved;
            if self.pad_distance[index] > 0.0 {
                continue;
            }

            // Measured from the destination, which is where the cache has to be
            // correct from for the next tick's subtraction to mean anything.
            self.pad_distance[index] = pad.distance(position.to_array());

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
        if entered != self.pad_current {
            self.pad_current = entered;
            if entered.is_some() {
                // Zone mode only. `Ship_ApplySpeedupPad` raises its flag under
                // the mode selector `zone-mode.md` identifies.
                if self.world.race.mode == Mode::Zone {
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
                self.exhaust.boost(exhaust::BOOST_SECONDS);
            }
        }

        hit.map(|(_, direction)| direction)
    }

    /// Whether this tick ended in contact with `Reset` geometry.
    ///
    /// Suppressed while the cooldown runs and once respawning has given up, so
    /// the two guards live in one place rather than at the call site.
    fn reset_zone_touched(&self, env: &Environment, before: Vec3) -> bool {
        if self.respawn_disabled || self.respawn_cooldown > 0 {
            return false;
        }
        let ship = &self.world.ships[0];
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
    fn respawn(&mut self, sample_index: Option<usize>) {
        let sample = sample_index
            .and_then(|index| self.spline.sample(index))
            .or_else(|| self.spline.start());
        let Some(sample) = sample.copied() else {
            return;
        };

        let ship = &mut self.world.ships[0];
        let height = spawn_height(&ship.handling);
        ship.place_at(Pose::from_sample(&sample, sample.racing_line, height));

        self.respawns += 1;
        self.respawns_in_a_row += 1;
        self.respawn_cooldown = RESPAWN_COOLDOWN_TICKS;

        // Otherwise the ribbon spans the teleport: ten samples of history from
        // wherever the craft fell off, stretched across the track to where it was
        // put back. The camera is snapped for the same reason.
        self.exhaust.clear_trail();

        if self.respawns_in_a_row >= RESPAWN_GIVE_UP {
            self.respawn_disabled = true;
            eprintln!(
                "reset: {} respawns in a row without getting clear, giving up. \
                 The recovery pose is probably inside a Reset volume; see \
                 Race::respawn.",
                self.respawns_in_a_row
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
            // Nothing depletes this yet - no weapons, and track-contact damage is
            // unrecovered - so the bar reads full for the whole race. It is the
            // ship's own pool rather than the parameter now, because Zone's
            // perfect-zone recharge writes it.
            shield: ship.shield,
            shield_max: ship.handling.dimensions.shield,
            lap: if counted { race.lap } else { 0 },
            // A speed lap and a Zone run have no lap target. Zero is what the HUD
            // already reads as "unknown" and it omits the "of N" half.
            laps: race.laps_target.filter(|_| counted).unwrap_or(0),
            // One ship on the grid until grid formation is recovered: seven of the
            // eight slots are laid out by unread code.
            place: 0,
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
        }
    }

    /// The exhaust's current animation state.
    #[must_use]
    pub fn exhaust(&self) -> &Exhaust {
        &self.exhaust
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
        for _ in 0..(ticks.floor() as u32) {
            self.exhaust
                .advance(self.dt, Self::FULL_THRUST, speed, &mut self.exhaust_rng);
        }
        let remainder = (ticks - ticks.floor()) * self.dt;
        if remainder > 0.0 {
            self.exhaust
                .advance(remainder, Self::FULL_THRUST, speed, &mut self.exhaust_rng);
        }
        self.exhaust.boost(exhaust::BOOST_SECONDS);
        let aged = (age / self.dt).round() as u32;
        for _ in 0..aged {
            self.exhaust
                .advance(self.dt, Self::FULL_THRUST, speed, &mut self.exhaust_rng);
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
            self.exhaust.clear_trail();
            for k in (0..exhaust::TRAIL_SAMPLES).rev() {
                let behind = back * (speed * self.dt * k as f32);
                self.exhaust.push_trail(nozzle + behind, back);
            }
        }
    }

    /// The collision sparks' current particle pool.
    #[must_use]
    pub fn sparks(&self) -> &Sparks {
        &self.sparks
    }

    /// The `engine_flare` locator in **world** space, or `None` when the ship
    /// model carries no `Engine Flare` node.
    ///
    /// Composed through [`Race::ship_model_matrix`], which is the only correct
    /// route: the locator is authored in `.vex` model space, so it has to pick up
    /// [`MODEL_YAW`] exactly as the hull's vertices do. A nozzle built from
    /// `body.forward()` instead would be a half-turn out and would sit on the
    /// ship's nose.
    #[must_use]
    pub fn nozzle(&self) -> Option<Vec3> {
        let local = self.nozzle?;
        Some(self.ship_model_matrix().transform_point3(local))
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
        if self.respawn_cooldown > 0 {
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
    /// **`<ExternalCameraFar fov>`'s unit is unrecovered.** It is read here as
    /// degrees, and `oag_render::camera::projection` names its parameter
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
    #[must_use]
    pub fn projection(&self, aspect: f32, far: f32, setting: crate::display::Fov) -> Mat4 {
        // An overridden fov stands in for the authored one and still passes
        // through the player's setting, whose default is identity; it exists to
        // calibrate the authored value's unrecovered unit against a captured
        // frame, so it must sit at exactly the same point in the chain.
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
        let authored = setting.apply(authored_fov.to_radians());
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
    /// # A residual remains, and it is a separate bug
    ///
    /// **`0.75` does not fully close the measured gap, and that is deliberately
    /// not absorbed here.** From the original's own recorded camera, with the
    /// track and buildings behind aligning, the wingtip lamp centroids sit
    /// `108.8 px` apart in the original against `189.3 px` in ours at scale
    /// `1.0` - a ratio of `1.740`, where `1/0.75` is `1.333`. Something else
    /// also makes our craft too large, by roughly a further `1.3x`.
    ///
    /// Do **not** widen this constant to swallow that. `1/0.75^2 = 1.7778` sits
    /// 2.2 % from the measurement, which makes "the original scales the mesh
    /// twice" the live hypothesis, but the second site was searched for and not
    /// found, and the lamp-centroid metric is **not linear in the scale it
    /// measures** (`1.0` -> `1.740`, `0.75` -> `1.191`, `0.5625` -> `1.067`,
    /// where linear would give `1.305` and `0.979`), so it cannot tell one
    /// candidate factor from another. A factor chosen to make it read `1.000`
    /// would be fitted to the instrument. The two unopened calls in
    /// `Craft_Construct_q` (`Ship_LoadModel`, `FUN_0884dab4`) and our own
    /// per-batch position scale in `oag_formats::vex` are where to look next;
    /// a non-emissive silhouette measurement is what would replace the metric.
    ///
    /// What ships here is the confirmed constant, applied once, with the
    /// unexplained remainder left visible rather than cancelled.
    #[must_use]
    pub fn ship_model_matrix(&self) -> Mat4 {
        let body = &self.ship().physics.body;
        Mat4::from_rotation_translation(body.orientation, body.position)
            * Mat4::from_rotation_y(MODEL_YAW)
            * Mat4::from_scale(Vec3::splat(oag_render::exhaust::CRAFT_ROW_SCALE))
    }
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
    anim_phase: f32,
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
    blend_pipeline: wgpu::RenderPipeline,
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
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
            fog_bind,
            fog_buffer,
        } = mesh_render::build(
            device,
            queue,
            &model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
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
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
            fog_bind,
            fog: fog_buffer,
        })
    }

    fn write(&self, queue: &wgpu::Queue, view_projection: Mat4, model: Mat4, anim_phase: f32) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            anim_phase,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Regenerates this model's texture coordinates the way the PSP GE's
    /// transparent pass does, and uploads them.
    ///
    /// `Mesh_BeginTransparentPass` sets `TEXMAPMODE` uvgen 2 - environment
    /// (shade) mapping from the vertex normal and lights 0/1 - and nothing in
    /// the batch loop undoes it, so a transparent batch's **authored** texture
    /// coordinates are never read on the original. See
    /// [`oag_render::texgen`], which carries the formula and the measurement
    /// of what feeding the authored ones instead did to the boost plume.
    ///
    /// Called only for the plume, and only on the frames it is visible. The
    /// general transparent pass is deliberately left alone: switching every
    /// transparent batch on every track and ship to generated coordinates is
    /// the blast radius `mesh-draw.md` has twice refused to take off one
    /// pass, and it is not what the reported symptom needs.
    ///
    /// Per frame rather than once at load, because the dot product is between
    /// a world-space light direction and a world-space normal: the
    /// coordinates move as the **ship** turns. From `self.model.vertices`
    /// every time rather than from the last frame's buffer, for the same
    /// reason [`Self::deflect_airbrakes`] does.
    fn generate_env_uvs(&self, queue: &wgpu::Queue, model: Mat4) {
        let mut generated = Vec::new();
        oag_render::texgen::environment_map(
            &self.model.vertices,
            &mut generated,
            model,
            oag_render::texgen::PLUME_LIGHT_0,
            oag_render::texgen::PLUME_LIGHT_1,
        );
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&generated));
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

        // Third pipeline, same pass: transparent batches, blended. See
        // `mesh_render::Built::blend_pipeline`.
        pass.set_pipeline(&self.blend_pipeline);
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(transparent, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
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

/// The period of the global texture-animation clock, in ticks.
///
/// Every animated surface is a whole number of V sweeps per this period (see
/// `oag_render::mesh::ANIMATED_TEXTURES`), which is what makes the wrap back to
/// phase zero seamless instead of a visible jump: at the wrap every surface is
/// an exact number of full texture heights along, and V wraps too.
///
/// 120 rather than 60 so a surface running at half the blink light's speed can
/// still be expressed as an integer.
const ANIM_PERIOD_TICKS: u64 = 120;

/// The global texture-animation clock's phase for `tick`, 0.0 up to 1.0.
///
/// **Confidence: 85** for the mechanism (a V-axis palette scroll - see
/// `docs/formats/vex.md`, "The animation is authored in the texture, on its V
/// axis"), and a narrower, separately-checkable claim for the rate. A live
/// capture (120 frame-accurate PPSSPP screenshots, pixels sampled at a real
/// light) measured one authored 8-row cycle repeating every 29-31 ticks; the
/// blink texture repeats that 8-row cycle twice across its 16 rows, so one full
/// scroll of all 16 rows is twice that, 60 ticks - which is `BLINK_V_CYCLES`
/// (2.0) sweeps per this 120-tick period, one second at the fixed 60 Hz.
///
/// From `world.tick`, never the wall clock, so a replay of the same tick draws
/// the same frame.
fn anim_phase(tick: u64) -> f32 {
    (tick % ANIM_PERIOD_TICKS) as f32 / ANIM_PERIOD_TICKS as f32
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
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` for a track with no `section` nodes - every Pure track - and the
    /// first tier is then skipped entirely rather than approximated.
    visibility: Option<TrackVisibility>,
    ship: Drawable,
    /// The boost plume: an ordinary `Drawable` with its blend pipeline
    /// overridden to [`exhaust::TRAIL_BLEND`] instead of
    /// [`mesh_render::TRANSPARENT_BLEND`].
    ///
    /// `None` when the source carries no `shipboost.vex` under this team's
    /// name - see `Loaded::boost_model`. Drawn only while
    /// [`Exhaust::plume_visible`] is true, with the ship's own model matrix,
    /// since the original parents it to the craft rather than to the flare.
    boost: Option<Drawable>,
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
        ship_model: Model,
        collision_model: Option<Model>,
        sky_model: Option<Model>,
        pad_model: Option<Model>,
        boost_model: Option<Model>,
        flare: Option<FlareTexture>,
        noise: Option<FlareTexture>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
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
        )?;
        let ship = Drawable::new(
            device,
            queue,
            ship_model,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            mesh_render::TRANSPARENT_BLEND,
        )?;
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
                )
            })
            .transpose()?;
        let pads = pad_model
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
                )
            })
            .transpose()?;
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
        // missing. Still 22% of the original's extent - the rest is a missing
        // bright-pass and a craft drawn too large, both recorded on the page.
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
        // **Environment-mapped UV generation is now implemented**, 2026-08-09,
        // for this model alone - `oag_render::texgen`, called from
        // `Drawable::generate_env_uvs`. Every transparent batch on every track
        // and ship is still left on authored coordinates, which is the blast
        // radius this comment used to give as the reason for not doing it at
        // all; scoping it to the plume gets the recovered mechanism without
        // taking that risk.
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
        // **Do not read the extent column as a calibrated ratio.** Our craft
        // renders about 1.4x too large at this very camera pose, so anything
        // measured in pixels here carries an unresolved scale error; see
        // `HANDOVER.md`, "The craft render scale invalidates matched-pose pixel
        // comparison".
        // See `docs/ghidra/functions/psp-pulse-usa/exhaust.md` and
        // `mesh-draw.md`.
        let boost = boost_model
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
                    // TEMPORARY measurement gate - remove before committing.
                    if std::env::var_os("OAG_ONE_ONE").is_some() {
                        exhaust::TRAIL_BLEND
                    } else {
                        exhaust::BLEND
                    },
                )
            })
            .transpose()?;
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

        Ok(Self {
            track,
            visibility,
            ship,
            boost,
            collision,
            sky,
            pads,
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
        let scroll = anim_phase(race.world.tick);
        // The ship's lights keep scrolling either way: `[graphics]
        // animated_textures` exists to test the *inferred* track entries
        // against a capture, and the ship's behaviour is not inferred.
        let track_scroll = if animated_textures { scroll } else { 0.0 };
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
            Some(&self.ship),
            self.collision.as_ref(),
            self.pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(&fog));
        }
        // `self.boost` is deliberately left out of this list, at `Fog::off`
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
                0.0,
            );
        }
        self.track
            .write(queue, view_projection, Mat4::IDENTITY, track_scroll);
        self.ship
            .write(queue, view_projection, race.ship_model_matrix(), scroll);
        // The flaps move in *model* space, before the ship's own matrix, so
        // this is a vertex write and not a second uniform - see
        // `Drawable::deflect_airbrakes`. Unconditional rather than
        // change-gated: two kilobytes a frame is cheaper than the state needed
        // to know they have not moved, and a gate would have to be invalidated
        // by anything that ever rebuilds the buffer.
        let [left, right] = race.airbrake_flaps();
        self.ship.deflect_airbrakes(queue, left, right);
        // Same model matrix as the ship: the original parents the plume to the
        // craft, not to the flare - see `Loaded::boost_model`. Skipped while
        // hidden rather than written and left undrawn, since there is nothing
        // for the stale buffer contents to affect either way.
        if let Some(boost) = &self.boost
            && race.exhaust().plume_visible()
            && std::env::var_os("OAG_NO_PLUME").is_none()
        {
            let model = race.ship_model_matrix();
            boost.write(queue, view_projection, model, 0.0);
            // TEMPORARY measurement gate - remove before committing.
            if std::env::var_os("OAG_NO_TEXGEN").is_none() {
                boost.generate_env_uvs(queue, model);
            }
        }
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY, track_scroll);
        }
        if let Some(pads) = &self.pads {
            pads.write(queue, view_projection, Mat4::IDENTITY, track_scroll);
        }

        // The camera's own axes, read out of the view matrix: for a view matrix
        // `V`, world-space right and up are rows 0 and 1 of its rotation part.
        // Building the quad from these is what makes it face the viewer, and it is
        // the whole reason this is world-space rather than the original's
        // post-projection sprite.
        let camera = race.view();
        let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
        let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
        let (vertices, trail) = match race.nozzle() {
            Some(nozzle) => (
                race.exhaust().vertices(nozzle, right, up),
                race.exhaust().trail_vertices(right, up),
            ),
            // No locator, nothing drawn - rather than a flare at the origin.
            None => (Vec::new(), Vec::new()),
        };
        self.exhaust.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &vertices,
            &trail,
        );
        let (spark_additive, spark_alpha) = race.sparks().vertices(right, up);
        self.sparks.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &spark_additive,
            &spark_alpha,
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
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        // Skipped outright in the cockpit view rather than moved or scaled away:
        // the original sets one flag on the craft and draws no hull, and a draw
        // call not issued is the only version of that with no chance of a stray
        // polygon across the middle of the screen. See `Race::draws_own_ship`.
        if race.draws_own_ship() {
            stats.add(self.ship.draw(&mut pass, None, None, None));
        }
        // After the ship, so the hull's depth is already in the buffer: the
        // plume's own blend pipeline writes no depth, the same reasoning as
        // the flare below.
        if let Some(boost) = &self.boost
            && race.exhaust().plume_visible()
            && std::env::var_os("OAG_NO_PLUME").is_none()
        {
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
    /// The shape to draw at inside the frame, leaving bars.
    ///
    /// A capture is a picture of a window, so it letterboxes the way a window
    /// does. At the default `--size`, which is the PSP's own shape, every value
    /// of this fills the frame and nothing changes.
    pub aspect: crate::display::Aspect,
    /// Anisotropic filtering level for the track and ship textures.
    pub anisotropy: Anisotropy,
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
        ship_model,
        collision_model,
        sky_model,
        pad_model,
        boost_model,
        fog_volumes,
        visibility,
        flare,
        noise,
        ..
    } = loaded;
    let mut race = Race::start(setup);
    race.set_boost_fov_kick(options.boost_fov_kick);
    race.set_camera_view(options.camera_view);
    race.set_control_scheme(options.scheme);

    let mut held = HeldButtons::new(options.held);
    for tick in 0..options.ticks {
        held.pulse(options.pressed, options.held, tick.is_multiple_of(2));
        let snapshot = held.snapshot();
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
        ship_model,
        collision_model,
        sky_model,
        pad_model,
        boost_model,
        flare,
        noise,
        format,
        scene_size,
        options.anisotropy,
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
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
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_formats::track;
    use oag_gameplay::input::button;
    use oag_input::keys::map_key;

    /// The hull and its plume must come from the same family, which is the
    /// regression this guards: the plume load hardcoded `shipboost.vex` while
    /// the hull switched on mode, so a Zone race drew a `Zone.vex` hull with the
    /// `Ship.vex` plume. Asserted as the *pairing* rather than as two
    /// independent strings, because the pairing is the thing that was wrong.
    #[test]
    fn the_boost_plume_follows_the_hull_its_mode_selects() {
        for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
            let hull = ship_entry_name("Feisar", mode);
            let plume = boost_entry_name("Feisar", mode);
            let stem = if mode == Mode::Zone { "Zone" } else { "ship" };
            assert!(
                plume.contains(stem),
                "{mode:?}: hull {hull} but plume {plume} - they are not the same family"
            );
        }
        assert_eq!(
            boost_entry_name("Feisar", Mode::Zone),
            r"Data\Ships\Feisar\Zoneboost.vex"
        );
        assert_eq!(
            boost_entry_name("Feisar", Mode::TimeTrial),
            r"Data\Ships\Feisar\shipboost.vex"
        );
    }

    /// A model declaring `slots` texture slots of which the first `decoded` filled.
    fn model(slots: usize, decoded: usize) -> Model {
        Model {
            airbrakes: [None, None],
            label: "Ship.vex".to_string(),
            vertices: Vec::new(),
            indices: vec![0; 6],
            draws: Vec::new(),
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: (0..slots)
                .map(|index| {
                    (index < decoded).then(|| mesh::ModelTexture {
                        label: format!("#{index}"),
                        width: 1,
                        height: 1,
                        rgba: vec![255; 4],
                    })
                })
                .collect(),
            centre: [0.0; 3],
            radius: 1.0,
            mesh_count: 1,
        }
    }

    #[test]
    fn anim_phase_starts_at_zero_and_wraps_every_period() {
        assert_eq!(anim_phase(0), 0.0);
        assert!((anim_phase(ANIM_PERIOD_TICKS / 2) - 0.5).abs() < 1e-6);
        assert_eq!(anim_phase(ANIM_PERIOD_TICKS), 0.0);
        assert_eq!(anim_phase(ANIM_PERIOD_TICKS * 3), 0.0);
    }

    /// The blink light kept the exact behaviour it had when the scroll was a
    /// dedicated 60-tick uniform rather than a shared 120-tick clock with a
    /// per-vertex rate: 2 sweeps per 120 ticks is 1 sweep per 60.
    #[test]
    fn the_blink_light_still_sweeps_once_every_sixty_ticks() {
        let sweeps = |tick| anim_phase(tick) * oag_render::mesh::ANIMATED_TEXTURES[0].1;
        assert_eq!(sweeps(0), 0.0);
        assert!((sweeps(30) - 0.5).abs() < 1e-6);
        assert!((sweeps(60) - 1.0).abs() < 1e-6);
        assert!((sweeps(90) - 1.5).abs() < 1e-6);
    }

    /// The PS2 shape, measured: five slots declared and none of them filled.
    #[test]
    fn slots_that_all_failed_to_decode_are_reported_with_the_count() {
        let note = untextured_note(&model(5, 0)).expect("an untextured model is reported");
        assert!(note.contains("Ship.vex"), "{note}");
        assert!(note.contains("0 of 5 texture slot(s)"), "{note}");
        // The reader has to be able to get from the line to the finding, because
        // the line on its own reads like a decode failure and it is not one.
        assert!(note.contains("ps2-texture.md"), "{note}");
    }

    /// A model that got everything it asked for is silent, which is every PSP
    /// model measured: 8 of 8 on the ship, 135 of 135 on the track.
    #[test]
    fn a_fully_textured_model_is_not_reported() {
        assert_eq!(untextured_note(&model(8, 8)), None);
    }

    #[test]
    fn a_partly_textured_model_is_reported_too() {
        let note = untextured_note(&model(8, 3)).expect("a partial decode is worth saying");
        assert!(note.contains("3 of 8 texture slot(s)"), "{note}");
    }

    /// **The regression this rule exists for.** The driveable ribbon is generated
    /// geometry with no texture slots at all, on either disc, and the earlier
    /// "no textures" test reported it on the *PSP* disc - blaming a PS2 texture-set
    /// gap for a mesh that was never textured and never came off a `.vex`.
    #[test]
    fn a_model_that_declares_no_slots_wanted_none_and_is_not_reported() {
        assert_eq!(untextured_note(&model(0, 0)), None);
    }

    /// The inverse table must really be the inverse, or the headless capture and the
    /// window disagree about what a key means and only one of them is ever tested.
    #[test]
    fn every_key_in_the_inverse_table_maps_back_to_its_button() {
        for index in 0..32u8 {
            let Some(key) = key_for_button(index) else {
                continue;
            };
            assert_eq!(
                map_key(&key),
                Some(index),
                "button {index} maps to {key:?}, which maps back to something else"
            );
        }
    }

    /// The buttons a race needs must all be reachable with no window.
    #[test]
    fn thrust_steering_and_both_airbrakes_all_have_a_key() {
        for wanted in [
            button::CROSS,
            button::LEFT,
            button::RIGHT,
            button::UP,
            button::DOWN,
            button::L,
            button::R,
        ] {
            assert!(key_for_button(wanted).is_some(), "no key for {wanted}");
        }
    }

    /// A pulsed button produces a *press* every other tick, a held one does not.
    ///
    /// The whole reason `pulse` exists: the veteran sideshift reads
    /// `Input::is_pressed`, and `--press` was reaching only the front end, so
    /// the manoeuvre could not be exercised in the mode a player uses. If this
    /// ever reports one edge and then silence, `--press l` has quietly become a
    /// hold again.
    #[test]
    fn a_pulsed_button_keeps_producing_edges_and_a_held_one_does_not() {
        let mut pulsed = HeldButtons::new(0);
        let mut held = HeldButtons::new(1 << button::L);
        let (mut pulsed_edges, mut held_edges) = (0, 0);

        for tick in 0..8u32 {
            pulsed.pulse(1 << button::L, 0, tick.is_multiple_of(2));
            if pulsed.snapshot().buttons.is_pressed(button::L) {
                pulsed_edges += 1;
            }
            if held.snapshot().buttons.is_pressed(button::L) {
                held_edges += 1;
            }
        }

        assert_eq!(pulsed_edges, 4, "one rising edge every other tick");
        assert_eq!(held_edges, 1, "a hold rises once and never again");
    }

    /// Holding and pulsing the same button resolves toward held.
    ///
    /// Otherwise `--hold q --press q` would silently drop the hold on every odd
    /// tick, which reads as an airbrake that stutters for no visible reason.
    #[test]
    fn pulsing_a_button_that_is_also_held_leaves_it_down() {
        let mut buttons = HeldButtons::new(1 << button::L);
        buttons.pulse(1 << button::L, 1 << button::L, false);
        assert!(buttons.snapshot().buttons.is_held(button::L));
    }

    #[test]
    fn a_held_cross_becomes_thrust() {
        let mut held = HeldButtons::new(1 << button::CROSS);
        let controls = ship_controls(&held.snapshot(), ControlScheme::default());
        assert_eq!(controls.thrust, 1.0);
        assert_eq!(controls.steer_x, 0.0);
    }

    #[test]
    fn holding_left_steers_left() {
        let mut held = HeldButtons::new(1 << button::LEFT);
        assert_eq!(
            ship_controls(&held.snapshot(), ControlScheme::default()).steer_x,
            -1.0
        );
    }

    /// A mask naming a button no key produces must not panic, and must not leak into
    /// the controls either.
    #[test]
    fn a_button_with_no_key_is_ignored() {
        let mut held = HeldButtons::new(1 << button::START);
        assert_eq!(
            ship_controls(&held.snapshot(), ControlScheme::default()),
            oag_physics::ShipControls::default()
        );
    }

    /// The uniform block is `oag-render`'s, not ours. If that side grows a field
    /// this catches it before the picture goes quietly wrong.
    #[test]
    fn the_uniform_block_matches_the_shaders() {
        assert_eq!(
            std::mem::size_of::<Uniforms>() as u64,
            mesh_render::UNIFORMS_SIZE
        );
    }

    /// A straight synthetic path, so the locator and the spawn can be tested with no
    /// disc image. Eight control points along `+x`, twelve units wide either side.
    fn straight_track() -> AiTrack {
        let count = 8usize;
        let paths_at = track::HEADER_LEN + track::RESERVED_LEN;
        let junctions_at = paths_at + track::PATH_LEN;
        let points_at = junctions_at + track::JUNCTION_LEN;
        let mut out = vec![0u8; points_at + count * track::POINT_LEN];
        out[0..4].copy_from_slice(&track::MAGIC.to_le_bytes());
        out[4..8].copy_from_slice(&0x105u32.to_le_bytes());
        out[8..12].copy_from_slice(&1u32.to_le_bytes());
        out[12..16].copy_from_slice(&1u32.to_le_bytes());
        out[paths_at..paths_at + 4].copy_from_slice(&(count as u32).to_le_bytes());
        for k in 0..count {
            let at = points_at + k * track::POINT_LEN;
            let mut put = |off: usize, v: f32| {
                out[at + off..at + off + 4].copy_from_slice(&v.to_le_bytes());
            };
            // Position along +x, tangent +x, down -y, lateral +z.
            put(0x00, k as f32 * 10.0);
            put(0x10, 1.0);
            put(0x24, -1.0);
            put(0x38, 1.0);
            put(0x44, 12.0);
            put(0x48, 12.0);
        }
        track::parse(&out).expect("the synthetic track must parse")
    }

    fn setup(handling: Handling) -> Setup {
        let ai = straight_track();
        let spline = Spline::from_track(&ai);
        // A straight is not a loop, so there is no ring and no lap counter. That
        // is the point for these tests: they are about the force law and the
        // camera, and a `None` course is the honest state for the track they run
        // on rather than a stub that counts laps on a line.
        let course = Course::from_track(&ai, None);
        Setup {
            airbrake_graphics: oag_gameplay::AirbrakeGraphics {
                amount: 0.4363323,
                up_speed: 30.0,
                down_speed: 30.0,
            },
            mode: Mode::TimeTrial,
            // A time trial does not read it, and these tests never run a Zone
            // race: the numbers are the disc's and there is no disc here.
            zone: None,
            ai,
            spline,
            course,
            // The synthetic track has no authored slot, which is the fallback
            // path: every assertion below is about a ship placed on the spline.
            start_position: None,
            collision: CollisionWorld::new(),
            handling,
            // Round numbers, chosen to make the geometry readable. None of these
            // claims to be the game's; a real race reads all seven off the disc.
            chase: ChaseParams {
                fov: 60.0,
                lookat_height: 1.0,
                lookat_length: 10.0,
                pos_height: 2.0,
                pos_length: 8.0,
                spring_horiz: 4.0,
                spring_vert: 2.0,
                // `1.0` rather than the craft's `0.75`, so the round numbers
                // above stay the numbers a reader can check the geometry
                // against. A real load passes the scale through.
                craft_scale: 1.0,
            },
            // The nearer external view, deliberately a *different* set of round
            // numbers from `chase`: a test that cycles to it and back has to be
            // able to tell the two apart. Note these go into `Setup` already
            // converted, so unlike a real load no `chase_params` scale or sign
            // flip is applied to them.
            chase_close: ChaseParams {
                fov: 50.0,
                lookat_height: 1.0,
                lookat_length: 10.0,
                pos_height: 1.0,
                pos_length: 4.0,
                spring_horiz: 4.0,
                spring_vert: 2.0,
                craft_scale: 1.0,
            },
            // The cockpit view. `length` positive puts the eye ahead of the
            // craft's origin, which is the sign the original's own measured
            // sample pins - see `oag_render::camera::internal`.
            internal: InternalParams {
                fov: 65.0,
                headtilt: 3.0,
                height: 1.0,
                length: 5.0,
                pitch: 0.0,
            },
            // A synthetic setup has no ship model, so no locators either. The
            // exhaust still ticks; it just has nowhere to be drawn, which is the
            // same path a model with no `Engine Flare` node takes. Sparks
            // likewise fall back to anchoring at the contact point.
            nozzle: None,
            collision_fx: Vec::new(),
            // A synthetic track authors no pads, which is also what every Pure
            // track does: an empty set is an ordinary state, not a stub.
            speedup_pads: Vec::new(),
            // These tests run on a synthetic straight and want the ordinary
            // spawn and the ordinary chase camera; `--pose` and its camera are
            // capture aids with nothing to say here.
            pose_override: None,
            camera_override: None,
            // The identity, so every assertion below is about the force law and
            // not about a scale. This is also what a race gets when the
            // engine-wide file is unreadable.
            class_gravity_scale: 1.0,
        }
    }

    /// A pad whose volume is `half` units across in every direction, centred on
    /// `at`, pushing along world `+z`.
    ///
    /// The identity basis is what makes the push direction readable: row 2 of an
    /// identity matrix is `+z`, so the boost the assertions look for is `+z`.
    fn pad_at(at: Vec3, half: f32) -> oag_formats::pads::PadVolume {
        let mut to_world = [0.0f32; 16];
        to_world[0] = 1.0;
        to_world[5] = 1.0;
        to_world[10] = 1.0;
        to_world[15] = 1.0;
        to_world[12] = at.x;
        to_world[13] = at.y;
        to_world[14] = at.z;
        oag_formats::pads::PadVolume {
            to_world,
            min: [-half; 3],
            max: [half; 3],
            disabled: 0.0,
        }
    }

    /// A race whose ship is inside a pad from its first tick, and one that is
    /// nowhere near one, so a test can difference them.
    fn race_with_pads(mode: Mode, pads: Vec<oag_formats::pads::PadVolume>) -> Race {
        let mut handling = hulled_handling();
        // Invented, and large enough that the boost is unmistakable against the
        // rest of the force law rather than lost in it. Deliberately **not** a
        // round hundred: the shipped `amount` is one, and ADR-0006 keeps shipped
        // values out of this repository even where they would read as arbitrary.
        handling.speedup_pads = oag_physics::params::SpeedupPads {
            amount: 37.0,
            time: 0.5,
        };
        let mut setup = setup(handling);
        setup.mode = mode;
        setup.speedup_pads = pads;
        Race::start(setup)
    }

    /// A pad big enough to hold the ship wherever `spawn_pose` puts it, so the
    /// test is about the trigger rather than about the spawn.
    fn enveloping_pad() -> Vec<oag_formats::pads::PadVolume> {
        vec![pad_at(Vec3::ZERO, 1.0e6)]
    }

    #[test]
    fn a_track_with_no_pads_never_boosts() {
        let mut race = race_with_pads(Mode::TimeTrial, Vec::new());
        for _ in 0..120 {
            let evaluated = race.tick(&InputSnapshot::default());
            assert_eq!(evaluated.speedup_pad, Vec3::ZERO);
        }
        assert_eq!(race.ship().physics.pad_timer, 0.0);
    }

    /// The whole chain, end to end: containment in `oag_formats::pads`, the
    /// direction off the pad's matrix, `Environment::pad_hit`, and the force term
    /// in `oag_physics`. It is deliberately one test, because each link is
    /// worthless without the others and a failure anywhere reads the same way.
    #[test]
    fn a_ship_inside_a_pad_is_pushed_along_the_pads_own_axis() {
        let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
        let evaluated = race.tick(&InputSnapshot::default());

        assert_ne!(evaluated.speedup_pad, Vec3::ZERO, "no boost was applied");
        assert_eq!(
            evaluated.speedup_pad.normalize(),
            Vec3::Z,
            "the boost must follow row 2 of the pad's matrix"
        );
        assert!(race.ship().physics.pad_timer > 0.0);
    }

    /// Leaving the pad does not end the boost, it starts the countdown. This is
    /// the re-arm semantics `ShipState::pad_timer` documents, seen from outside.
    #[test]
    fn the_boost_outlives_the_pad_and_then_expires() {
        let mut race = race_with_pads(Mode::TimeTrial, vec![pad_at(Vec3::ZERO, 1.0e6)]);
        race.tick(&InputSnapshot::default());
        assert!(race.ship().physics.pad_timer > 0.0);

        // Take the pad away, which is the same to the trigger as driving off it.
        race.speedup_pads.clear();
        let mut boosted_ticks = 0;
        for _ in 0..120 {
            if race.tick(&InputSnapshot::default()).speedup_pad != Vec3::ZERO {
                boosted_ticks += 1;
            }
        }
        // `time` is 0.5 s at 60 Hz, so about thirty ticks - asserted as a range
        // rather than a count, because the exact tick the timer crosses zero on
        // is float arithmetic and not the thing under test.
        assert!(
            (25..=35).contains(&boosted_ticks),
            "{boosted_ticks} ticks of boost after leaving the pad"
        );
        assert_eq!(
            race.ship().physics.pad_timer,
            0.0,
            "the timer never expired"
        );
    }

    /// `<GravityMul airborne>` must reach the gravity term, and it must reach the
    /// **grounded** half of it.
    ///
    /// The name says airborne and the VFPU pair chain puts it on the grounded
    /// lane - see `oag_formats::handling::GravityMul`. So this asserts the
    /// counter-intuitive half: a heavier scale changes a ship resting on the
    /// ground, and the identity leaves the term exactly as it was before the
    /// value was decoded.
    #[test]
    fn the_class_gravity_scale_reaches_the_grounded_half_of_gravity() {
        fn settled_gravity(scale: f32) -> f32 {
            let mut handling = hulled_handling();
            // A non-zero `normal_gravity`, or the scale has nothing to multiply.
            handling.physical.normal_gravity = 10.0;
            let mut setup = setup_with(
                handling,
                vec![plane(1, 0.0, oag_physics::Surface::Floor, 0)],
            );
            setup.class_gravity_scale = scale;
            let mut race = Race::start(setup);

            // Several ticks, not one: gravity reads the *previous* frame's
            // groundedness, so the first tick sees zero contacts however solidly
            // the ship is resting on the floor.
            let mut evaluated = race.tick(&InputSnapshot::default());
            for _ in 0..20 {
                evaluated = race.tick(&InputSnapshot::default());
            }
            assert!(
                race.ship().physics.grounded > 0.0,
                "the ship never found the floor, so the grounded lane is not under test"
            );
            evaluated.gravity.y
        }

        let identity = settled_gravity(1.0);
        let heavier = settled_gravity(2.0);
        assert!(identity < 0.0, "gravity must pull down");
        assert!(
            heavier < identity,
            "doubling the scale must pull harder: {heavier} against {identity}"
        );
        // And the term is linear in it, which is what a *scale* means.
        assert!((heavier - identity * 2.0).abs() < 1e-3);
    }

    /// The flare **outlives** the force, and it is armed with a fixed duration.
    ///
    /// This is the opposite of what an earlier revision asserted. The original
    /// arms the flare on the entry edge with a code literal
    /// ([`exhaust::BOOST_SECONDS`]), while the force runs for the speed class's
    /// own `<SpeedupPads time>` - a fraction of it on every shipped class. So a
    /// pad is a short shove and a long look, and tying the two together is the
    /// obvious-looking mistake this pins against.
    #[test]
    fn the_flare_outlives_the_force_and_ignores_the_class_tunable() {
        let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
        // Well under the flare's fixed duration, so the two are distinguishable.
        assert!(
            race.ship().handling.speedup_pads.time < exhaust::BOOST_SECONDS,
            "the fixture must make the flare the longer of the two"
        );

        race.tick(&InputSnapshot::default());
        // Armed with the constant, not with the class's `time`.
        assert!(
            (race.exhaust().boost_timer() - (exhaust::BOOST_SECONDS - race.dt())).abs() < 1e-4,
            "the flare was armed with {} rather than {}",
            race.exhaust().boost_timer(),
            exhaust::BOOST_SECONDS
        );

        race.speedup_pads.clear();
        let mut force_ticks = 0;
        let mut flare_ticks = 0;
        for _ in 0..120 {
            let evaluated = race.tick(&InputSnapshot::default());
            force_ticks += u32::from(evaluated.speedup_pad != Vec3::ZERO);
            flare_ticks += u32::from(race.exhaust().boost_timer() > 0.0);
        }
        assert!(
            flare_ticks > force_ticks,
            "the flare ran {flare_ticks} tick(s) and the force {force_ticks}; the \
             flare must outlast it"
        );
    }

    /// Zone pays for a pad **once**, on entry, no matter how long the ship sits
    /// on it - `Zone_Update` consumes and clears a flag rather than counting.
    #[test]
    fn zone_scores_once_for_entering_a_pad_and_not_again_while_inside() {
        const TICKS: u32 = 90;
        let mut padded = race_with_pads(Mode::Zone, enveloping_pad());
        let mut bare = race_with_pads(Mode::Zone, Vec::new());
        for _ in 0..TICKS {
            padded.tick(&InputSnapshot::default());
            bare.tick(&InputSnapshot::default());
        }
        assert_eq!(
            padded.world.race.score - bare.world.race.score,
            oag_race::zone::SPEEDUP_PAD_SCORE,
            "a pad held for {TICKS} ticks must pay exactly once"
        );
    }

    /// And the other modes pay nothing at all, which is the `DAT_08b31048 == 6`
    /// gate on the flag's only writer.
    #[test]
    fn only_zone_mode_scores_for_a_speed_pad() {
        for mode in [Mode::TimeTrial, Mode::SpeedLap] {
            let mut race = race_with_pads(mode, enveloping_pad());
            for _ in 0..90 {
                race.tick(&InputSnapshot::default());
            }
            assert!(
                race.ship().physics.pad_timer > 0.0,
                "{mode:?}: the pad did not fire at all, so the score assertion proves nothing"
            );
            assert_eq!(race.world.race.score, 0, "{mode:?} scored for a pad");
        }
    }

    /// The authored kick widens the view while the boost runs and closes again
    /// afterwards, and **turning it off leaves the projection bit-identical** to
    /// what it was before the effect existed. That second half is the one that
    /// matters: it is what makes the setting usable for a comparison against a
    /// capture of the original.
    #[test]
    fn the_boost_kick_widens_the_view_and_turning_it_off_changes_nothing() {
        use crate::display::{BoostFovKick, Fov};

        fn x_scale(m: Mat4) -> f32 {
            m.to_cols_array()[0]
        }

        let mut kicked = race_with_pads(Mode::TimeTrial, enveloping_pad());
        let mut off = race_with_pads(Mode::TimeTrial, enveloping_pad());
        off.set_boost_fov_kick(BoostFovKick::OFF);

        let resting = kicked.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED);
        for _ in 0..30 {
            kicked.tick(&InputSnapshot::default());
            off.tick(&InputSnapshot::default());
        }

        let open = kicked.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED);
        assert!(
            x_scale(open) < x_scale(resting),
            "a wider field means a smaller x scale, and this one did not move"
        );
        assert_eq!(
            off.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED)
                .to_cols_array(),
            resting.to_cols_array(),
            "with the kick off the matrix must be exactly the pre-effect one"
        );

        // Take the pad away and let the boost expire; the kick must return to
        // *exactly* the resting matrix rather than to something near it.
        kicked.speedup_pads.clear();
        for _ in 0..600 {
            kicked.tick(&InputSnapshot::default());
        }
        assert_eq!(
            kicked
                .projection(16.0 / 9.0, 1000.0, Fov::AUTHORED)
                .to_cols_array(),
            resting.to_cols_array(),
            "the kick never fully closed"
        );
    }

    /// The three non-zero tiers are the point of turning the toggle into a
    /// magnitude: a player who found [`BoostFovKick::DEFAULT`] too subtle to
    /// notice needs the stronger rows to actually be stronger, not just
    /// differently labelled.
    #[test]
    fn a_stronger_tier_widens_the_view_further() {
        use crate::display::{BoostFovKick, Fov};

        fn x_scale(m: Mat4) -> f32 {
            m.to_cols_array()[0]
        }

        let widen_at = |tier: BoostFovKick| {
            let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
            race.set_boost_fov_kick(tier);
            for _ in 0..30 {
                race.tick(&InputSnapshot::default());
            }
            x_scale(race.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED))
        };

        let scales: Vec<f32> = BoostFovKick::OFFERED
            .iter()
            .copied()
            .map(widen_at)
            .collect();
        // A wider field is a *smaller* x scale, so ascending tiers descend here.
        assert!(
            scales.windows(2).all(|pair| pair[0] > pair[1]),
            "the tiers must widen monotonically: {scales:?}"
        );
    }

    /// **The load-bearing test of this whole feature.** Cycling the camera is
    /// presentation, and the determinism rules say presentation may not reach the
    /// simulation - so the proof has to be a hash, not a reading of the code.
    ///
    /// Two races from one seed, driven with identical input for identical tick
    /// counts. One of them walks the full camera cycle in the middle, twice
    /// round, including in and out of the cockpit view. The exhaustive state hash
    /// (`oag_physics::probe::hash_state`, which destructures `ShipState` field by
    /// field on purpose) has to come out bit-identical at every tick, and so does
    /// the seeded generator.
    #[test]
    fn cycling_the_camera_changes_no_simulation_state() {
        use oag_core::hash::StateHasher;
        use oag_physics::probe::hash_state;

        fn hash(race: &Race) -> u64 {
            let mut hasher = StateHasher::new();
            hash_state(&mut hasher, &race.ship().physics);
            hasher.write_u64(race.world.tick);
            hasher.finish()
        }

        // Thrust held, so the craft is moving rather than parked: a stationary
        // ship would hash the same however badly the camera behaved.
        let held = 1u32 << oag_gameplay::input::button::CROSS;

        let mut control = Race::start(setup(Handling::default()));
        let mut cycled = Race::start(setup(Handling::default()));

        let mut control_input = HeldButtons::new(held);
        let mut cycled_input = HeldButtons::new(held);

        for tick in 0..240u32 {
            control.tick(&control_input.snapshot());
            cycled.tick(&cycled_input.snapshot());
            // Eleven changes over the run, so every view is entered and left
            // several times while the craft flies - and eleven is deliberately
            // not a multiple of three, so the run does not end back on the view
            // it started from and the assertions below have something to see.
            if tick > 0 && tick % 20 == 0 {
                cycled.set_camera_view(cycled.camera_view().next());
            }
            assert_eq!(hash(&control), hash(&cycled), "diverged at tick {tick}");
            // The seeded generator too, by value: a camera that drew a random
            // number from the *simulation's* generator - rather than from the
            // separate ones `Race` keeps for the exhaust and the sparks - would
            // desynchronise every later tick, and this catches it on the first.
            assert_eq!(
                control.world.rng, cycled.world.rng,
                "the simulation's generator moved at tick {tick}"
            );
        }

        // The cycle really did move: otherwise this test passes by doing nothing,
        // which is the failure mode a test of this shape is prone to.
        assert_eq!(control.camera_view(), crate::display::CameraView::default());
        assert_ne!(cycled.camera_view(), control.camera_view());
        // And the two cameras really are looking at different things, so the
        // hashes above are equal despite a genuinely different picture.
        assert_ne!(control.view(), cycled.view());
    }

    /// The three views have to be three *different* cameras. A dispatch that fell
    /// through to the far block for all of them would satisfy every other test
    /// here.
    #[test]
    fn each_view_frames_the_craft_from_its_own_block() {
        use crate::display::{CameraView, Fov};

        let mut race = Race::start(setup(Handling::default()));
        let mut eyes = Vec::new();
        let mut fovs = Vec::new();
        for view in CameraView::ALL {
            race.set_camera_view(view);
            eyes.push((view, race.camera_position()));
            fovs.push((
                view,
                race.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED),
            ));
        }

        for (index, (view, eye)) in eyes.iter().enumerate() {
            for (other, other_eye) in &eyes[index + 1..] {
                assert!(
                    (*eye - *other_eye).length() > 1e-3,
                    "{view} and {other} share an eye at {eye}"
                );
            }
        }
        // Each block authors its own fov in the fixture, so the projections have
        // to differ too - which is what catches a `view()` that dispatches beside
        // a `projection()` that does not.
        for (index, (view, projection)) in fovs.iter().enumerate() {
            for (other, other_projection) in &fovs[index + 1..] {
                assert_ne!(projection, other_projection, "{view} and {other}");
            }
        }

        // The geometry, not just the difference: the cockpit eye is *ahead* of the
        // craft and both chase eyes are behind it. A sign error here is the single
        // most likely mistake in the whole feature.
        let craft = race.ship().physics.body.position;
        let forward = race.ship().physics.body.forward();
        race.set_camera_view(CameraView::Internal);
        assert!((race.camera_position() - craft).dot(forward) > 0.0);
        for view in [CameraView::Close, CameraView::Far] {
            race.set_camera_view(view);
            assert!(
                (race.camera_position() - craft).dot(forward) < 0.0,
                "{view} is in front of the craft"
            );
        }
        // And close is closer than far, which is what the two names mean.
        race.set_camera_view(CameraView::Close);
        let close = (race.camera_position() - craft).length();
        race.set_camera_view(CameraView::Far);
        assert!(close < (race.camera_position() - craft).length());

        // The hull is drawn in both external views and in neither of the other
        // states, which is the one thing `Scene::render` branches on.
        for view in [CameraView::Close, CameraView::Far] {
            race.set_camera_view(view);
            assert!(race.draws_own_ship(), "{view}");
        }
        race.set_camera_view(CameraView::Internal);
        assert!(!race.draws_own_ship());
    }

    /// The craft's global `0.75` scale reaches the external rig and none of the
    /// internal one. Both halves are measurements off the original - see
    /// [`chase_params`] and `oag_render::camera::internal` - and both are cheap to
    /// undo by accident.
    ///
    /// It reaches the rig as [`ChaseParams::craft_scale`] and **not** as four
    /// pre-multiplied offsets, which is the correction this test pins: the two
    /// differ once the spring has state, by `0.121` RMS against `0.008` over the
    /// capture. Asserting the four offsets come through *unscaled* is what stops
    /// somebody folding the scale back in and double-applying it.
    #[test]
    fn the_external_blocks_carry_the_crafts_global_scale() {
        let scale = oag_physics::hover::TARGET_GLOBAL_SCALE;
        let block = handling::ExternalCamera {
            fov: 60.0,
            lookat_height: 4.0,
            lookat_length: 8.0,
            pos_height: 4.0,
            pos_length: -16.0,
            spring_horiz: 3.0,
            spring_vert: 5.0,
        };
        let params = chase_params(block);

        assert_eq!(params.craft_scale, scale);
        // Authored, unscaled: the rig applies `craft_scale` itself, at the end,
        // about the craft, which is where the original applies it.
        assert_eq!(params.lookat_height, 4.0);
        assert_eq!(params.lookat_length, 8.0);
        assert_eq!(params.pos_height, 4.0);
        // Negated but not scaled: the sign flip is [`chase_pos_length`]'s.
        assert_eq!(params.pos_length, 16.0);
        // Not lengths, so never scaled wherever the scale is applied.
        assert_eq!(params.fov, 60.0);
        assert_eq!(params.spring_horiz, 3.0);
        assert_eq!(params.spring_vert, 5.0);

        // The number three independent measurements of the original agree on: an
        // authored `(-15, +4)` close block puts the eye 11.64 units from the
        // craft. See [`chase_params`] for all three. Measured off the settled
        // camera the rig actually produces, so it covers the scale wherever the
        // scale now lives.
        let close = chase_params(handling::ExternalCamera {
            pos_height: 4.0,
            pos_length: -15.0,
            ..block
        });
        let target = oag_render::camera::chase::Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: Vec3::Y,
        };
        let distance =
            (oag_render::camera::chase::anchor(target, &close) - target.position).length();
        assert!((distance - 11.643).abs() < 0.01, "{distance}");
    }

    /// A camera override must move every reader of the camera at once: the
    /// view matrix, and through it the derived eye that PVS culling and fog
    /// sampling read. A consumer left on the chase camera would frame the
    /// geometry from one place and cull it from another - wrong picture, no
    /// error - which is why `Race::view` is the single seam.
    #[test]
    fn a_camera_override_moves_the_view_and_the_derived_eye_together() {
        let eye = Vec3::new(12.0, 34.0, -56.0);
        let orientation = Quat::from_rotation_y(0.83);
        let mut setup = setup(Handling::default());
        setup.camera_override = Some(CameraOverride {
            eye,
            orientation,
            fov_deg: None,
        });
        let race = Race::start(setup);

        assert!((race.camera_position() - eye).length() < 1e-4);
        // The view maps the eye to the origin and world axes into camera axes:
        // a point one unit along the camera's own -Z lands on (0, 0, -1).
        let ahead = eye + orientation * Vec3::NEG_Z;
        let mapped = race.view().transform_point3(ahead);
        assert!((mapped - Vec3::NEG_Z).length() < 1e-4, "{mapped}");
    }

    /// The fov half of the override slots in exactly where the authored value
    /// sits, so a calibrated value and the disc's own go through the same
    /// setting and the same aspect fit.
    #[test]
    fn a_camera_fov_override_stands_in_for_the_authored_fov() {
        use crate::display::Fov;

        let mut with_override = setup(Handling::default());
        with_override.camera_override = Some(CameraOverride {
            eye: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            fov_deg: Some(75.0),
        });
        let overridden = Race::start(with_override);

        let mut same_authored = setup(Handling::default());
        same_authored.chase = ChaseParams {
            fov: 75.0,
            ..same_authored.chase
        };
        let authored = Race::start(same_authored);

        assert_eq!(
            overridden.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED),
            authored.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED)
        );
    }

    /// The field-of-view setting has to reach the matrix, and its default has
    /// to leave that matrix exactly as it was before the setting existed - the
    /// ground-truth captures under `data/traces/` were taken against it.
    #[test]
    fn the_field_of_view_setting_widens_the_projection_and_defaults_to_the_authored_one() {
        use crate::display::Fov;

        let race = Race::start(setup(Handling::default()));
        let aspect = AUTHORED_ASPECT;
        let authored = race.projection(aspect, 1000.0, Fov::AUTHORED);

        // The projection's first column scales x by `1 / (tan(fov/2) * aspect)`,
        // so a wider field is a *smaller* number there. Read off the matrix
        // rather than recomputed, which would only restate `Fov::apply`.
        let x_scale = |m: Mat4| m.col(0).x;
        let wide: Fov = "150".parse().expect("parse");
        let narrow: Fov = "75".parse().expect("parse");
        assert!(
            x_scale(race.projection(aspect, 1000.0, wide)) < x_scale(authored),
            "150 % has to show more"
        );
        assert!(
            x_scale(race.projection(aspect, 1000.0, narrow)) > x_scale(authored),
            "75 % has to show less"
        );

        // And the untouched setting is the matrix the fit alone produces.
        let unchanged = oag_render::camera::projection(
            oag_render::camera::fit_vertical_fov(
                race.chase_params.fov.to_radians(),
                AUTHORED_ASPECT,
                aspect,
            ),
            aspect,
            1.0,
            1000.0,
        );
        assert_eq!(authored, unchanged);
    }

    #[test]
    fn the_locator_finds_the_nearest_sample() {
        let spline = Spline::from_track(&straight_track());
        assert!(!spline.is_empty());
        assert_eq!(spline.len(), 8 * Spline::STEPS_PER_SEGMENT);
        assert_eq!(spline.path_of(0), Some(0));

        let start = *spline.start().expect("a first sample");
        let at = Vec3::from_array(start.pos);
        assert!(spline.distance_to(at).expect("a distance") < 1e-3);

        // Straight up from the first sample is still nearest that sample.
        let (index, _, distance) = spline.nearest(at + Vec3::Y * 5.0).expect("a sample");
        assert_eq!(index, 0);
        assert!((distance - 5.0).abs() < 1e-3, "{distance}");
    }

    #[test]
    fn the_widest_half_width_comes_from_the_track() {
        let spline = Spline::from_track(&straight_track());
        assert!((spline.max_half_width() - 12.0).abs() < 1e-3);
    }

    /// A local search finds what a whole-table search would, when the answer is
    /// inside the window.
    #[test]
    fn a_local_search_agrees_with_a_whole_table_one_inside_its_window() {
        let spline = Spline::from_track(&straight_track());
        let target = Vec3::from_array(spline.sample(20).expect("a sample").pos);
        let (whole, _, _) = spline.nearest(target).expect("a sample");
        let (local, _, distance) = spline
            .nearest_within(target, 20, 8)
            .expect("a sample in the window");
        assert_eq!(local, whole);
        assert!(distance < 1e-3, "{distance}");
    }

    /// And when the answer is outside the window it returns something too far
    /// away, which the caller turns into "draw everything" rather than into a
    /// confident wrong section. See `Spline::nearest_within`.
    #[test]
    fn a_local_search_that_misses_reports_a_distance_the_caller_will_reject() {
        let spline = Spline::from_track(&straight_track());
        let far_end = Vec3::from_array(spline.sample(30).expect("a sample").pos);
        let (index, _, distance) = spline
            .nearest_within(far_end, 0, 2)
            .expect("a sample in the window");
        assert!(index <= 2, "the search stayed inside its window");
        assert!(
            distance > 12.0 * OFF_TRACK_HALF_WIDTHS,
            "a miss must be far enough to be rejected, was {distance}"
        );
    }

    /// **The conservative path, which nothing else exercises.** A craft on the
    /// racing line names its section; one that has left the track names
    /// nothing, so the visible set becomes everything.
    #[test]
    fn a_position_off_the_track_refuses_to_name_a_section() {
        let race = Race::start(setup(Handling::default()));
        let spline = race.spline();
        let sample = *spline.sample(4).expect("a sample");
        let on_line = Vec3::from_array(sample.pos);

        assert_eq!(
            race.section_of(4, on_line),
            sample.section_id,
            "a craft on the racing line names its own section"
        );

        // The synthetic track is 12 units wide either side, so the gate is at
        // 36. Checked either side of it rather than at one distance, so the
        // test would fail if the threshold were dropped entirely.
        let half_width = 12.0 * OFF_TRACK_HALF_WIDTHS;
        let lateral = Vec3::from_array(sample.lateral);
        assert_eq!(
            race.section_of(4, on_line + lateral * (half_width - 1.0)),
            sample.section_id,
            "still on the track at just under the gate"
        );
        assert_eq!(
            race.section_of(4, on_line + lateral * (half_width + 1.0)),
            UNPLACED,
            "just past the gate, nothing may be culled on this position's word"
        );
        assert_eq!(
            race.section_of(4, on_line + Vec3::Y * 200.0),
            UNPLACED,
            "far above the track - airborne over a gap, or fallen through"
        );
        assert_eq!(
            race.section_of(9_999, on_line),
            UNPLACED,
            "a sample index the table does not have"
        );
    }

    /// A respawn in flight teleports the craft and leaves the camera spring
    /// catching up, so neither position describes the shot.
    #[test]
    fn a_respawn_in_flight_makes_everything_visible() {
        let mut race = Race::start(setup(Handling::default()));
        race.respawn_cooldown = 1;
        assert_eq!(race.visibility_sections(), (UNPLACED, UNPLACED));

        // And the resulting set really is everything, not merely two unknown
        // ids - this is the property the whole conservative path exists for.
        let pvs = oag_formats::pvs::TrackPvs::empty();
        let padding = SectionPadding::default();
        let (craft, camera) = race.visibility_sections();
        assert!(
            VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), craft, camera)
                .is_everything()
        );
    }

    /// A parameter set with a real hull, so the reset probes have something to
    /// probe with, and no gravity, so a ship only moves when a test moves it.
    fn hulled_handling() -> Handling {
        Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                ..Default::default()
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 8.0,
                ..Default::default()
            },
            dimensions: oag_physics::params::Dimensions {
                width: 2.0,
                height: 1.0,
                length: 4.0,
                ..Default::default()
            },
            ..Handling::ZERO
        }
    }

    /// A large quad, as a collider of one class.
    ///
    /// `axis` picks the plane: 1 is horizontal at height `at`, 0 is vertical at
    /// `x = at`.
    fn plane(
        axis: usize,
        at: f32,
        surface: oag_physics::Surface,
        collider: u32,
    ) -> oag_physics::TriangleSoup {
        let corner = |u: f32, v: f32| {
            let mut p = [0.0f32; 3];
            p[axis] = at;
            let others: Vec<usize> = (0..3).filter(|&k| k != axis).collect();
            p[others[0]] = u;
            p[others[1]] = v;
            p
        };
        oag_physics::TriangleSoup::new(
            vec![
                corner(-500.0, -500.0),
                corner(500.0, -500.0),
                corner(500.0, 500.0),
                corner(-500.0, 500.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            surface,
            collider,
        )
    }

    fn setup_with(handling: Handling, colliders: Vec<oag_physics::TriangleSoup>) -> Setup {
        let mut setup = setup(handling);
        for c in colliders {
            setup.collision.push(c);
        }
        setup
    }

    /// The feature, end to end: a ship that leaves the track and crosses `Reset`
    /// geometry is put back on the spline.
    ///
    /// The reset plane is **horizontal and below**, which is where a real one
    /// sits, and the hull probes are horizontal - so only the swept ray can find
    /// it. That is deliberate: it is the path a falling ship actually takes.
    #[test]
    fn a_ship_that_falls_through_a_reset_plane_is_put_back_on_the_track() {
        let handling = hulled_handling();
        let setup = setup_with(
            handling,
            vec![plane(1, -40.0, oag_physics::Surface::Reset, 0)],
        );
        let mut race = Race::start(setup);
        assert_eq!(race.respawns(), 0);

        // Throw it off the track. No gravity in this fixture, so nothing else can
        // move it and the only thing under test is the reset path.
        {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -20.0, 0.0);
            body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
        }

        for _ in 0..10 {
            race.tick(&InputSnapshot::default());
        }

        assert_eq!(race.respawns(), 1, "the reset plane never triggered");

        let body = race.world.ships[0].physics.body;
        // Back on the track: near a spline sample, above it, and stopped.
        let distance = race.spline().distance_to(body.position).expect("a sample");
        assert!(distance < 20.0, "respawned {distance} from the spline");
        assert!(body.position.y > 0.0, "respawned at {:?}", body.position);
        assert_eq!(body.linear_velocity, Vec3::ZERO);
    }

    /// Nothing but `Reset` geometry may respawn a ship. The same plane in the same
    /// place under a different class must leave the ship where it fell.
    #[test]
    fn falling_through_any_other_class_does_not_respawn() {
        for surface in [
            oag_physics::Surface::Wall,
            oag_physics::Surface::Floor,
            oag_physics::Surface::MagFloor,
        ] {
            let setup = setup_with(hulled_handling(), vec![plane(1, -40.0, surface, 0)]);
            let mut race = Race::start(setup);
            {
                let body = &mut race.world.ships[0].physics.body;
                body.position = Vec3::new(20.0, -20.0, 0.0);
                body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
            }
            for _ in 0..10 {
                race.tick(&InputSnapshot::default());
            }
            assert_eq!(race.respawns(), 0, "{surface:?} respawned the ship");
        }
    }

    /// **The firehose trap.** A ship held against a wall for many ticks must
    /// spawn one spark burst per `oag_render::sparks::COLLISION_COOLDOWN`,
    /// not one burst per tick of the ensuing scrape - the shape of bug this
    /// guards against is the same one `oag_physics::wall::STUN_PER_CONTACT`'s
    /// doc comment records this crate cost a session of play-testing to, for
    /// the collision stun rather than sparks. Ten ticks (`10/60 s`) is well
    /// inside the cooldown, so this only checks the *no-refire-yet* half; see
    /// `a_sustained_scrape_refires_after_the_cooldown_elapses` for the other
    /// half.
    ///
    /// The plane and approach are copied from
    /// [`falling_through_any_other_class_does_not_respawn`], a known-working
    /// `Surface::Wall` fixture, rather than a fresh vertical wall: getting a
    /// triangle's winding backwards produces a silent "no contact" rather
    /// than a loud failure, and this fixture is already proven to register.
    #[test]
    fn a_sustained_scrape_spawns_sparks_once_not_every_tick() {
        let handling = hulled_handling();
        let setup = setup_with(
            handling,
            vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
        );
        let mut race = Race::start(setup);

        // Reset to an inbound approach before every tick, so each one sees a
        // fresh impact rather than the ship bouncing away after the first.
        let push_toward_wall = |race: &mut Race| {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -39.7, 0.0);
            body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        };

        push_toward_wall(&mut race);
        let evaluated = race.tick(&InputSnapshot::default());
        assert!(
            evaluated.wall.impact,
            "the fixture never reaches the wall - not what this test means to check"
        );
        assert_eq!(
            race.sparks().ignitions(),
            1,
            "the first impact never ignited"
        );
        assert!(
            race.sparks().alive_count() > 0,
            "the first impact spawned no sparks"
        );

        // A live particle count cannot distinguish one burst from many any
        // more - a single burst trickles new particles for 32 ticks by
        // design - so the trigger cadence is asserted on the ignition
        // counter directly.
        for tick in 0..10 {
            push_toward_wall(&mut race);
            race.tick(&InputSnapshot::default());
            assert_eq!(
                race.sparks().ignitions(),
                1,
                "tick {tick} of the same scrape ignited another burst before the cooldown elapsed"
            );
        }
    }

    /// The other half of the firehose-trap guard: unlike a one-shot edge
    /// latch, `ShipCollisionFx_Trigger`'s recovered behaviour is a periodic
    /// re-fire - `oag_render::sparks::COLLISION_COOLDOWN` (`0.8` s) after the
    /// last burst, for as long as contact continues. A burst's own trailing
    /// embers can outlive the cooldown (up to `32 + 30` ticks, about
    /// `1.03` s), so an empty pool is *not* a precondition of the re-fire
    /// any more - the ignition counter is the unambiguous signal.
    #[test]
    fn a_sustained_scrape_refires_after_the_cooldown_elapses() {
        let handling = hulled_handling();
        let setup = setup_with(
            handling,
            vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
        );
        let mut race = Race::start(setup);
        let dt = race.dt();

        let push_toward_wall = |race: &mut Race| {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -39.7, 0.0);
            body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        };

        push_toward_wall(&mut race);
        let evaluated = race.tick(&InputSnapshot::default());
        assert!(
            evaluated.wall.impact,
            "the fixture never reaches the wall - not what this test means to check"
        );
        assert_eq!(race.sparks().ignitions(), 1);

        // Run past the cooldown: a second burst must have ignited, and only
        // one.
        let ticks = (oag_render::sparks::COLLISION_COOLDOWN / dt).ceil() as usize + 1;
        for _ in 0..ticks {
            push_toward_wall(&mut race);
            race.tick(&InputSnapshot::default());
        }
        assert_eq!(
            race.sparks().ignitions(),
            2,
            "no second burst ignited after the cooldown elapsed"
        );
    }

    /// The guard against the failure that would otherwise look like a hang: a
    /// recovery pose that is itself inside a reset volume respawns forever.
    ///
    /// Here the trigger is a vertical plane half a unit from the spawn, inside the
    /// hull's own half-width, so every recovery lands straight back in it.
    /// Respawning must stop rather than freeze the race.
    #[test]
    fn a_recovery_pose_inside_a_reset_volume_gives_up_instead_of_looping() {
        let handling = hulled_handling();
        let start = *setup(handling).spline.start().expect("a first sample");
        // Half a unit to the side of wherever the ship is placed, well inside the
        // one-unit hull half-width.
        let beside = Vec3::from_array(start.pos).x + 0.5;
        let setup = setup_with(
            handling,
            vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
        );

        let mut race = Race::start(setup);
        for _ in 0..(RESPAWN_COOLDOWN_TICKS * (RESPAWN_GIVE_UP + 3)) {
            race.tick(&InputSnapshot::default());
        }

        assert_eq!(
            race.respawns(),
            RESPAWN_GIVE_UP,
            "respawning did not stop after {RESPAWN_GIVE_UP} tries"
        );
    }

    /// The cooldown is what stops one contact from respawning on every tick while
    /// the ship is still overlapping the trigger.
    #[test]
    fn a_respawn_is_not_repeated_on_the_very_next_tick() {
        let handling = hulled_handling();
        let start = *setup(handling).spline.start().expect("a first sample");
        let beside = Vec3::from_array(start.pos).x + 0.5;
        let setup = setup_with(
            handling,
            vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
        );

        let mut race = Race::start(setup);
        race.tick(&InputSnapshot::default());
        assert_eq!(race.respawns(), 1);
        for _ in 0..(RESPAWN_COOLDOWN_TICKS - 1) {
            race.tick(&InputSnapshot::default());
            assert_eq!(race.respawns(), 1, "respawned again inside the cooldown");
        }
    }

    /// A ship must arrive on the track with its mass in the body, or the hover
    /// spring and the integrator disagree about how heavy it is.
    #[test]
    fn a_started_race_puts_one_ship_on_the_spline_with_its_mass_set() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 7.5,
                ..Default::default()
            },
            antigrav: oag_physics::params::Antigrav {
                // Chosen so the resulting spawn height cannot coincide with
                // `track::HOVER_LIFT`, which the assertion below distinguishes it from.
                ride_height: 8.0,
                ..Default::default()
            },
            pitch: oag_physics::params::Pitch {
                antigrav_height_adjust: 0.5,
                ..Default::default()
            },
            ..Handling::ZERO
        };
        let setup = setup(handling);
        let start = *setup.spline.start().expect("a first sample");
        let race = Race::start(setup);

        assert_eq!(race.world.ship_count, 1);
        assert!(race.ship().active);
        assert_eq!(race.ship().physics.body.mass, 7.5);
        assert_eq!(race.ship().handling.physical.mass, 7.5);
        assert_eq!(race.dt(), 1.0 / 60.0);

        // Lifted off the surface line by the height its own suspension holds it at,
        // which is neither `track::HOVER_LIFT` nor the probe's full reach; see
        // `spawn_height`. This fixture has no gravity, so the sag is zero and the rest
        // height is the target itself: `ride_height` scaled by the measured
        // `TARGET_GLOBAL_SCALE`, with the `antigrav_height_adjust` of 0.5 contributing
        // nothing because `craft+0x74` reads zero in the running game.
        assert_eq!(
            spawn_height(&handling),
            8.0 * oag_physics::hover::TARGET_GLOBAL_SCALE
        );
        assert_ne!(spawn_height(&handling), track::HOVER_LIFT);
        // Well inside the probes' reach, which is what gives the suspension travel.
        assert!(spawn_height(&handling) < handling.antigrav.ride_height);
        let up = (-Vec3::from_array(start.down)).normalize();
        let offset = race.ship().physics.body.position - Vec3::from_array(start.pos);
        assert!(
            (offset.dot(up) - spawn_height(&handling)).abs() < 1e-4,
            "{offset}"
        );
    }

    /// The spawn height is the spring's rest height plus the probes' own drop,
    /// not the spring's target and not the probe's reach.
    ///
    /// Round numbers chosen so the sag is checkable by hand, and **not** a ship's:
    /// two probes give a gradient of `2 * 0.3 * HOVER_K * (normal_gravity +
    /// track_gravity)`, and a grounded craft carries `normal_gravity +
    /// track_gravity` (gravity plus `oag_physics::hover::DOWNFORCE_SCALE`'s
    /// downforce), so the sag is exactly `1.25` whatever the two gravities are.
    /// The centre of mass then sits a probe drop above that. `mass` is
    /// deliberately not 1, to pin that it cancels.
    #[test]
    fn a_ship_spawns_at_the_height_its_own_suspension_holds_it_at() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 3.0,
                normal_gravity: 5.0,
                track_gravity: 80.0,
                flight_gravity: 95.0,
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 5.5,
                ..Default::default()
            },
            ..Handling::ZERO
        };

        let gradient = 2.0 * 0.3 * oag_physics::hover::HOVER_K * 85.0;
        let target = 5.5 * oag_physics::hover::TARGET_GLOBAL_SCALE;
        let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
        let expected = target - 85.0 / gradient + drop;
        assert!(
            (spawn_height(&handling) - expected).abs() < 1e-5,
            "{} was not {expected}",
            spawn_height(&handling)
        );

        // Real travel in **both** directions, which is the property that matters
        // and the one the old geometry could not have. A probe reaches exactly as
        // far as the target (`oag_physics::hover::probe`), so its travel is the
        // whole `target`: the resting probe sits `1.25` up from full compression
        // and `1.25` short of losing the ground. That symmetry is the recovered
        // model's, not a tuning: both numbers are the same sag.
        let probe_rest = spawn_height(&handling) - drop;
        assert!(probe_rest > 0.0 && probe_rest < target);
        assert!(
            (probe_rest - (target - 1.25)).abs() < 1e-5,
            "the probe rests at {probe_rest}, not 1.25 below its {target} target"
        );

        // And mass really does cancel.
        let heavier = Handling {
            physical: oag_physics::params::Physical {
                mass: 50.0,
                ..handling.physical
            },
            ..handling
        };
        assert_eq!(spawn_height(&heavier), spawn_height(&handling));
    }

    /// The tensor is the recovered box and does **not** depend on the ship.
    ///
    /// This test used to assert the opposite: that the tensor was built from
    /// `<Misc width/height/length>` at `<Physical mass>`, with roll the cheapest
    /// axis because the hull is long. Both halves are now refuted -
    /// `Body_SetBoxInertia`'s single call site passes the code literal
    /// `(12, 8, 12)` at a mass of `0.9`, the hull dimensions go to the collider
    /// instead, and the box is square in plan so **pitch and roll are equal** and
    /// yaw is the odd axis out. Kept as a test rather than deleted because "the
    /// inertia varies per craft" is exactly the assumption that would come back.
    #[test]
    fn the_inertia_tensor_is_the_recovered_box_and_is_the_same_for_every_ship() {
        let inertia = box_inertia();

        assert!((inertia.x - 15.6).abs() < 0.01, "pitch was {}", inertia.x);
        assert!((inertia.y - 21.6).abs() < 0.01, "yaw was {}", inertia.y);
        assert!((inertia.z - 15.6).abs() < 0.01, "roll was {}", inertia.z);

        // Square in plan, so the two attitude axes cost the same and yaw is the
        // hardest. A hull-derived tensor could not produce this.
        assert_eq!(inertia.x, inertia.z);
        assert!(inertia.y > inertia.x);
        assert_ne!(inertia, Vec3::ONE);
    }

    /// A ship at rest on flat ground stays at rest.
    ///
    /// This is the assertion whose absence let a ship fall through the floor with no
    /// input held: on the shipped parameters the placeholder inertia tensor turned
    /// single-probe contact into a 38 rad/s pitch oscillator that explicit Euler grew
    /// about 5 % a tick, and the ship inverted and left the world by tick 250 while the
    /// player was not touching the controls. Reverting either the tensor or the spawn
    /// height fails this within two seconds of simulated time.
    ///
    /// A flat floor and no track, deliberately: the same failure reproduced with no
    /// spline, no collision soup and no camera, so this pins the force law rather than
    /// the composition. Ten seconds is long enough that 5 %-per-tick growth would be
    /// astronomically visible.
    #[test]
    fn a_ship_at_rest_on_flat_ground_stays_at_rest() {
        use oag_physics::{Body, ShipControls, ShipState, Surface, TriangleSoup};

        // The shape of the observed parameters. Round numbers, and not a ship's.
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                normal_gravity: 5.0,
                track_gravity: 80.0,
                flight_gravity: 95.0,
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 5.5,
                rebound: 0.6,
                landing_rebound: 0.5,
                ..Default::default()
            },
            dimensions: oag_physics::params::Dimensions {
                width: 5.0,
                height: 3.5,
                length: 13.0,
                ..Default::default()
            },
            pitch: oag_physics::params::Pitch {
                pitch_damping: 3.0,
                // Set, and required not to matter: see `hover::target_height`.
                antigrav_height_adjust: 1.0,
                ..Default::default()
            },
            ..Handling::ZERO
        };

        let mut floor = CollisionWorld::new();
        floor.push(TriangleSoup::new(
            vec![
                [-5000.0, 0.0, -5000.0],
                [-5000.0, 0.0, 5000.0],
                [5000.0, 0.0, 5000.0],
                [5000.0, 0.0, -5000.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            Surface::Floor,
            0,
        ));

        let start = spawn_height(&handling);
        let mut state = ShipState {
            body: Body {
                position: Vec3::new(0.0, start, 0.0),
                mass: handling.physical.mass,
                inertia: box_inertia(),
                ..Body::default()
            },
            grounded: 1.0,
            ..ShipState::default()
        };

        let mut worst_tilt = 0.0f32;
        let mut airborne = 0u32;
        for _ in 0..600 {
            let evaluated = oag_physics::step(
                &mut state,
                &ShipControls::default(),
                &handling,
                &Environment::default(),
                &floor,
                1.0 / 60.0,
            );
            if evaluated.hover.contacts == 0 {
                airborne += 1;
            }
            let tilt = state.body.up().dot(Vec3::Y).clamp(-1.0, 1.0).acos();
            worst_tilt = worst_tilt.max(tilt);
        }

        assert_eq!(
            airborne, 0,
            "the ship left the ground on {airborne} tick(s)"
        );
        assert!(
            worst_tilt < 0.01,
            "the ship tilted to {worst_tilt} rad with no input held"
        );
        assert!(
            (state.body.position.y - start).abs() < 0.5,
            "the ship drifted from {start} to {} with no input held",
            state.body.position.y
        );
        assert!(
            state.body.linear_velocity.length() < 1.0,
            "the ship was moving at {} with no input held",
            state.body.linear_velocity.length()
        );
    }

    /// With no collision geometry there is nothing to hover on, so the ship must
    /// fall - and must fall *finitely*. The cheapest possible check that the force
    /// law is really being driven from here.
    #[test]
    fn a_ship_over_nothing_falls_and_stays_finite() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                flight_gravity: 10.0,
                normal_gravity: 10.0,
                track_gravity: 10.0,
            },
            ..Handling::ZERO
        };
        let mut race = Race::start(setup(handling));
        let start = race.ship().physics.body.position;

        let mut held = HeldButtons::new(0);
        for _ in 0..60 {
            let snapshot = held.snapshot();
            race.tick(&snapshot);
        }

        let now = race.ship().physics.body.position;
        assert!(now.is_finite(), "{now}");
        assert!(now.y < start.y, "nothing to hover on, so it must fall");
        assert_eq!(race.ship().physics.grounded, 0.0);
        assert_eq!(race.world.tick, 60);
        assert!(race.telemetry().spline_distance.is_finite());
    }
}
