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
//! The **driveable ribbon**, for the same reason `oag-view --track` draws it: it is
//! the geometry the simulation spawns on, so a picture of ship-plus-ribbon shows
//! directly whether the ship is where the physics thinks it is. `--art` draws the
//! track's art meshes instead, which is prettier and proves less.
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
//! - **The camera's `fov` unit is unrecovered.** It is read as degrees and
//!   converted explicitly in [`Race::projection`]; the value read is in the load
//!   report so a reader can judge the assumption.
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
use oag_core::math::{Mat4, Vec3};
use oag_core::{TickClock, TickRate};
use oag_formats::track::{AiTrack, Sample};
use oag_formats::{collision, handling};
use oag_gameplay::{
    InputSnapshot, Pose, Ship, World, collision_world, handling_for, ship_controls,
};
use oag_input::Keyboard;
use oag_physics::{CollisionWorld, Environment, Evaluated, Handling, SpeedClass};
use oag_render::camera::chase::{Chase, ChaseParams, Target};
use oag_render::collision as render_collision;
use oag_render::mesh::Model;
use oag_render::mesh_render::Anisotropy;
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

/// The archive entry name of a team's `.vex` model.
///
/// Assembled the way the loader assembles it, with backslashes, which is what the
/// name hash needs.
#[must_use]
pub fn ship_entry_name(team: &str) -> String {
    format!(r"Data\Ships\{team}\Ship.vex")
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
    /// Draw the track's art meshes instead of the driveable ribbon.
    pub art: bool,
    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of - the same view `oag-view --collision` draws, wireframe and
    /// Cage-excluded, on top of whichever track model was chosen above.
    pub collision: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: crate::source::DEFAULT_IMAGE.to_string(),
            track: DEFAULT_TRACK.to_string(),
            team: DEFAULT_TEAM.to_string(),
            class: SpeedClass::Venom,
            art: false,
            collision: false,
        }
    }
}

/// Everything a race needs with no GPU anywhere in sight.
///
/// Split out from [`Loaded`] so a test can build one by hand, or take one off a
/// disc, and run a whole race against it on a machine with no graphics driver.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The decoded spline graph, as the file has it.
    pub ai: AiTrack,
    /// The spline resampled for locating a ship, and for placing it.
    pub spline: Spline,
    /// Every collidable triangle of the track.
    pub collision: CollisionWorld,
    /// The force law's parameter set for one team in one speed class.
    pub handling: Handling,
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
}

/// A [`Setup`] plus the geometry to draw it with.
#[derive(Debug)]
pub struct Loaded {
    /// The simulation half.
    pub setup: Setup,
    /// What to draw for the track.
    pub track_model: Model,
    /// What to draw for the ship.
    pub ship_model: Model,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
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

    let stats_name = handling::entry_name(&options.team);
    let stats_blob = read(&mut archives, &stats_name)?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once
    // and this is not the place it happens.
    let handling = handling_for(&stats, options.class);
    let far = stats.external_camera_far;
    let chase = ChaseParams {
        fov: far.fov,
        lookat_height: far.lookat_height,
        lookat_length: far.lookat_length,
        pos_height: far.pos_height,
        pos_length: chase_pos_length(far.pos_length),
        spring_horiz: far.spring_horiz,
        spring_vert: far.spring_vert,
    };
    report.push(format!(
        "{stats_name}: team {:?}, {:?} class, mass {}, ride_height {}",
        stats.team, options.class, handling.physical.mass, handling.antigrav.ride_height
    ));
    report.push(format!(
        "<ExternalCameraFar>: fov {} read as degrees (the unit is unrecovered), \
         pos_length {} on disc -> eye {} up and {} back",
        chase.fov, far.pos_length, chase.pos_height, chase.pos_length
    ));

    let ship_name = ship_entry_name(&options.team);
    let ship_blob = read(&mut archives, &ship_name)?;
    let ship_model = mesh::build(&ship_name, &ship_blob)?;
    report.push(format!(
        "{ship_name}: {} triangle(s), model centre {:?}, radius {:.2}",
        ship_model.indices.len() / 3,
        ship_model.centre,
        ship_model.radius
    ));

    let track_model = if options.art {
        mesh::build(&options.track, &track_blob)?
    } else {
        track_render::build_model(&label, &ai)
    };
    report.push(format!(
        "drawing the track's {}: {} triangle(s), radius {:.0}",
        if options.art {
            "art meshes"
        } else {
            "driveable ribbon"
        },
        track_model.indices.len() / 3,
        track_model.radius
    ));

    for model in [&ship_model, &track_model] {
        if let Some(line) = untextured_note(model) {
            report.push(line);
        }
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

    Ok(Loaded {
        setup: Setup {
            ai,
            spline,
            collision,
            handling,
            chase,
        },
        track_model,
        collision_model,
        ship_model,
        report,
    })
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
/// **This is the one place a PS2 race is knowingly worse than a PSP one, and it
/// is reported rather than papered over.** Measured on the real discs: the PSP's
/// `Assegai\Ship.vex` fills 8 of 8 slots and `16_Track\track.vex` 135 of 135,
/// while the PS2's fill **0 of 5** and **0 of 140**. The slots are there because
/// the file still declares one `Texture` node each; the pixels are not, because a
/// PS2 model's textures are separate archive entries gathered into a nested WAD of
/// Graphics Synthesizer upload packets, and *which* entry belongs to *which* model
/// is not recovered. `oag_render::mesh::ps2_texture_set` decodes such a set once
/// something names it - `oag-view --mesh ... --textures <entry>` is where that is
/// done by hand - so what is missing is the lookup, not the decoder. See
/// `docs/formats/ps2-texture.md`.
///
/// The model still draws: `Drawable::draw` binds the white fallback for a draw
/// with no texture slot. So a PS2 race is a correctly-shaped untextured one, and
/// nothing here guesses at a texture set to avoid saying that.
///
/// Deliberately keyed on the model rather than on the platform. A PSP model with a
/// slot that will not decode deserves the same line, and the PS2 disc carries
/// PSP-format batches too - so "which disc is this" is never the right question to
/// ask about one mesh.
#[must_use]
fn untextured_note(model: &Model) -> Option<String> {
    let slots = model.textures.len();
    let decoded = model.textures.iter().filter(|t| t.is_some()).count();
    if slots == 0 || decoded == slots {
        return None;
    }
    Some(format!(
        "{}: {decoded} of {slots} texture slot(s) decoded, drawing the rest \
         untextured. PS2 models keep their textures in separate archive entries \
         and the model-to-texture-set lookup is not recovered; see \
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
    chase_params: ChaseParams,
    camera: Chase,
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
}

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
            spline,
            collision,
            handling,
            chase,
            ..
        } = setup;

        let mut world = World::new(SEED);
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = box_inertia();
        if let Some(sample) = spline.start() {
            ship.place_at(Pose::from_sample(
                sample,
                sample.racing_line,
                spawn_height(&handling),
            ));
        }
        world.ship_count = 1;

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        Self {
            world,
            collision,
            spline,
            chase_params: chase,
            camera,
            // ADR-0007: 60 Hz, from the clock rather than from a literal, so there
            // is one place the rate is decided.
            dt: TickClock::new(TickRate::DEFAULT).rate().dt(),
            respawn_cooldown: 0,
            respawns_in_a_row: 0,
            respawn_disabled: false,
            respawns: 0,
        }
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
        let controls = ship_controls(snapshot);

        // `track_up` is the *track's* own up at the ship, not the ship's. Nothing in
        // the recovered force law consumes it yet - it is there for the magnetic
        // hold - so this cannot perturb the simulation today.
        let nearest = self
            .spline
            .nearest(self.world.ships[0].physics.body.position);
        let track_up = nearest
            .and_then(|(_, sample, _)| (-Vec3::from_array(sample.down)).try_normalize())
            .unwrap_or(Vec3::Y);
        let index = nearest.map(|(index, _, _)| index);

        let ship = &mut self.world.ships[0];
        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            ship.segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        let env = Environment {
            track_up,
            ..Environment::default()
        };
        let before = ship.physics.body.position;
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
        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);
        evaluated
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
    /// `docs/ghidra/functions/psp-pulse/collision.md` records **that** a `Reset`
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

    /// Where the camera is and what it is aimed at, as a view matrix.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        self.camera.view(target_of(self.ship()), &self.chase_params)
    }

    /// The projection for a viewport of the given aspect ratio.
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
    /// `near` is 1.0 rather than something tiny: track half-widths are tens of
    /// units, so a near plane at 0.01 spends the depth range on nothing and
    /// z-fights in the distance.
    #[must_use]
    pub fn projection(&self, aspect: f32, far: f32) -> Mat4 {
        let fov = oag_render::camera::fit_vertical_fov(
            self.chase_params.fov.to_radians(),
            AUTHORED_ASPECT,
            aspect,
        );
        oag_render::camera::projection(fov, aspect, 1.0, far)
    }

    /// Where the ship is and how it is oriented, as the mesh shader's model matrix.
    ///
    /// Rotation and translation only, so the shader's assumption of uniform scale -
    /// which is what lets it rotate normals without an inverse transpose - holds.
    /// The model's own origin is used as it is: no recentring, and the only rotation
    /// composed in is [`MODEL_YAW`], which is a stated finding about the `.vex` and
    /// not a nudge to make the picture look right.
    #[must_use]
    pub fn ship_model_matrix(&self) -> Mat4 {
        let body = &self.ship().physics.body;
        Mat4::from_rotation_translation(body.orientation, body.position)
            * Mat4::from_rotation_y(MODEL_YAW)
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
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    textures: Vec<wgpu::BindGroup>,
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
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
    ) -> Result<Self> {
        let (pipeline, _placeholder, vertices, indices, textures) =
            mesh_render::build(device, queue, &model, format, anisotropy)?;

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
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
        })
    }

    fn write(&self, queue: &wgpu::Queue, view_projection: Mat4, model: Mat4) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.model.indices.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        for draw in &self.model.draws {
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
}

/// The track and the ship on the GPU, drawn from a chase camera.
#[derive(Debug)]
pub struct Scene {
    track: Drawable,
    ship: Drawable,
    /// The collision soup overlay, present only when `Options::collision` asked
    /// for it. Drawn with the identity transform, same as the track: the
    /// collision geometry is already in world space.
    collision: Option<Drawable>,
    depth: wgpu::Texture,
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
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let track = Drawable::new(device, queue, track_model, format, anisotropy)?;
        let ship = Drawable::new(device, queue, ship_model, format, anisotropy)?;
        let collision = collision_model
            .map(|model| Drawable::new(device, queue, model, format, anisotropy))
            .transpose()?;
        Ok(Self {
            track,
            ship,
            collision,
            depth: depth_texture(device, size),
            far,
        })
    }

    /// Rebuilds the depth buffer for a new viewport size.
    ///
    /// A depth attachment whose size does not match the colour attachment is a
    /// validation error, so this is not optional on resize.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.depth = depth_texture(device, size);
    }

    /// Draws one frame into `view`.
    ///
    /// One render pass with one clear: a second pass would either wipe the first's
    /// colour or need its own decision about the depth buffer.
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        size: (u32, u32),
    ) {
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let view_projection = race.projection(aspect, self.far) * race.view();
        self.track.write(queue, view_projection, Mat4::IDENTITY);
        self.ship
            .write(queue, view_projection, race.ship_model_matrix());
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY);
        }

        let depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.03,
                        g: 0.04,
                        b: 0.06,
                        a: 1.0,
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
        self.track.draw(&mut pass);
        self.ship.draw(&mut pass);
        if let Some(collision) = &self.collision {
            collision.draw(&mut pass);
        }
    }
}

fn depth_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
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
    /// Image size.
    pub size: (u32, u32),
    /// Print a telemetry line every this many ticks. Zero prints none.
    pub log_every: u32,
    /// Anisotropic filtering level for the track and ship textures.
    pub anisotropy: Anisotropy,
}

/// Runs a race headless and writes one frame to a PNG.
///
/// Through the same [`Scene`] the window draws, for the same reason
/// [`crate::capture`] goes through the same renderer the front end's window does: a
/// separate capture path would prove nothing about what a player sees.
///
/// # Errors
///
/// Propagates adapter and device creation, pipeline building, the readback map and
/// the file write.
pub fn capture(loaded: Loaded, options: &CaptureOptions) -> Result<()> {
    let (width, height) = options.size;
    let Loaded {
        setup,
        track_model,
        ship_model,
        collision_model,
        ..
    } = loaded;
    let mut race = Race::start(setup);

    let mut held = HeldButtons::new(options.held);
    for _ in 0..options.ticks {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        if options.log_every > 0 && race.world.tick % u64::from(options.log_every) == 0 {
            println!("{}", describe(&race.telemetry()));
        }
    }
    println!(
        "after {} tick(s): {}",
        race.world.tick,
        describe(&race.telemetry())
    );

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .context("no GPU adapter available")?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-game race offscreen"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    // Rgba8Unorm rather than an sRGB format: the readback goes straight into a PNG,
    // so a second gamma encode would double-correct.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let scene = Scene::new(
        &device,
        &queue,
        track_model,
        ship_model,
        collision_model,
        format,
        (width, height),
        options.anisotropy,
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
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

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
    scene.render(&queue, &mut encoder, &view, &race, (width, height));
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

    /// A model declaring `slots` texture slots of which the first `decoded` filled.
    fn model(slots: usize, decoded: usize) -> Model {
        Model {
            label: "Ship.vex".to_string(),
            vertices: Vec::new(),
            indices: vec![0; 6],
            draws: Vec::new(),
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

    #[test]
    fn a_held_cross_becomes_thrust() {
        let mut held = HeldButtons::new(1 << button::CROSS);
        let controls = ship_controls(&held.snapshot());
        assert_eq!(controls.thrust, 1.0);
        assert_eq!(controls.steer_x, 0.0);
    }

    #[test]
    fn holding_left_steers_left() {
        let mut held = HeldButtons::new(1 << button::LEFT);
        assert_eq!(ship_controls(&held.snapshot()).steer_x, -1.0);
    }

    /// A mask naming a button no key produces must not panic, and must not leak into
    /// the controls either.
    #[test]
    fn a_button_with_no_key_is_ignored() {
        let mut held = HeldButtons::new(1 << button::START);
        assert_eq!(
            ship_controls(&held.snapshot()),
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
        Setup {
            ai,
            spline,
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
            },
        }
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

    /// The spawn height is the spring's rest height, not its target and not the
    /// probe's reach.
    ///
    /// Round numbers chosen so the sag is checkable by hand, and **not** a ship's:
    /// with `normal_gravity` 5 and `track_gravity` 80 the gradient is
    /// `0.3 * HOVER_K * 85`, so the sag is `5` over that. `mass` is deliberately not
    /// 1, to pin that it cancels.
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

        let gradient = 0.3 * oag_physics::hover::HOVER_K * 85.0;
        let target = 5.5 * oag_physics::hover::TARGET_GLOBAL_SCALE;
        let expected = target - 5.0 / gradient;
        assert!(
            (spawn_height(&handling) - expected).abs() < 1e-5,
            "{} was not {expected}",
            spawn_height(&handling)
        );

        // Strictly below the reach, with real headroom above it. The margin used to be
        // the spring's sag alone, 0.147 units, because the target was read as equal to
        // the cast length; `TARGET_GLOBAL_SCALE` is measured at 0.75 now, so a resting
        // ship sits about 1.5 units below the height at which it loses the ground.
        let headroom = handling.antigrav.ride_height - spawn_height(&handling);
        assert!(spawn_height(&handling) < handling.antigrav.ride_height);
        assert!(
            headroom > 1.0,
            "only {headroom} of probe headroom; the suspension has no travel again"
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
