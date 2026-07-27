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
//! - **There is no inertia tensor in the ship data.** `<Misc>` carries hull
//!   dimensions, but nothing observed says how the original builds a tensor from
//!   them, so [`oag_physics::Body::inertia`] stays at its default of `(1, 1, 1)`.
//!   The hover probes apply their force away from the centre of mass, so the torque
//!   they generate is divided by 1 rather than by a real moment: expect a ship far
//!   more willing to pitch and roll than the original's. Deriving a box inertia
//!   from the hull would be an invented constant, which is worse than a visible
//!   oddity.
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
use oag_assets::{Archive, pulse};
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
use oag_render::mesh::Model;
use oag_render::{mesh, mesh_render, track as track_render};

/// The track a race is flown on unless another is named.
///
/// One of the two names `oag-view` already draws, so the picture can be compared
/// against a tool that predates the simulation.
pub const DEFAULT_TRACK: &str = r"Data\Environments\01_Track\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
pub const DEFAULT_TEAM: &str = "Feisar";

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
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: "data/images/pulse-psp-usa.chd".to_string(),
            track: DEFAULT_TRACK.to_string(),
            team: DEFAULT_TEAM.to_string(),
            class: SpeedClass::Venom,
            art: false,
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
    let spec = pulse::archive_spec(&options.source, pulse::archives::DATA);

    let (ai, label) = track_render::load(&spec, &options.track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    let mut archive = Archive::open(&spec)?;
    let read = |archive: &mut Archive, name: &str| -> Result<Vec<u8>> {
        archive
            .read_name(name)
            .with_context(|| format!("reading {name} out of {spec}"))
    };

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice. See the
    // wanted-change note in `docs/tools/oag-game.md`.
    let track_blob = read(&mut archive, &options.track)?;
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
    let stats_blob = read(&mut archive, &stats_name)?;
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
    let ship_blob = read(&mut archive, &ship_name)?;
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

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));

    Ok(Loaded {
        setup: Setup {
            ai,
            spline,
            collision,
            handling,
            chase,
        },
        track_model,
        ship_model,
        report,
    })
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

/// How far above the track's surface line a ship starts: `<Antigrav ride_height>`.
///
/// Three candidate heights exist and they are three different quantities. Which one
/// a ship starts at changes the first second of every race, so the choice is written
/// down here rather than left as a number in a constructor.
///
/// - [`oag_formats::track::HOVER_LIFT`] is where the load pass puts the **AI line**,
///   by lifting each control point three units off the surface. It says nothing
///   about ships. Starting there leaves the suspension compressed by the difference,
///   and on the observed data one frame of the spring at that compression throws the
///   ship clear of the track. Measured, not predicted, which is why this is not it.
/// - [`oag_physics::hover::target_height`] is the height the spring **holds**. On the
///   observed data it comes out *greater* than `ride_height`, which is also the
///   length of the probe raycast, so a ship starting at its own target height cannot
///   see the ground at all and begins in free fall. That the target can exceed the
///   reach is a property of the recovered reading - `target_height` records its
///   additive `antigrav_height_adjust` term as a guess at confidence 50 - and not of
///   this choice.
/// - `ride_height` is the furthest the suspension can see the ground and the primary
///   term of the height it holds. A ship starting there is in contact on its first
///   frame, which is the one thing all three readings agree a ship on a grid is.
///
/// A ship starting at `ride_height` is in contact on **one** probe, not two: the two
/// probes sit fore and aft along the hull, so at exactly the cast length they
/// straddle the limit and one of them drops out. Measured over the first two seconds
/// of a real race, one probe is in contact on about seven ticks in ten and both on
/// about one in twenty. A single probe is an off-centre force, so it is a pitch
/// torque applied every tick; that is recorded rather than corrected, because which
/// of the reading, the probe placement or the missing inertia tensor is wrong is
/// exactly what M3's trace comparison is for.
///
/// This is only where a ship *starts*: the height it settles at is emergent from the
/// force law, as `oag_gameplay::spawn::Pose::from_sample` says.
#[must_use]
pub fn spawn_height(handling: &Handling) -> f32 {
    handling.antigrav.ride_height
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
}

impl Race {
    /// Starts a race: one ship, on the racing line, at the start of the spline.
    ///
    /// The mass is copied into the rigid body because the force law reads
    /// `handling.physical.mass` while the integrator reads `body.mass`. They are one
    /// quantity stored twice and keeping them equal is this layer's job -
    /// `oag_physics` deliberately does not do it, so that a mismatch stays visible.
    ///
    /// The inertia is left at its default; see the module documentation for why that
    /// is a known limitation rather than an oversight.
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
        }
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
        let evaluated = oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            &self.collision,
            self.dt,
        );

        self.world.tick += 1;
        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);
        evaluated
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
    ) -> Result<Self> {
        let (pipeline, _placeholder, vertices, indices, textures) =
            mesh_render::build(device, queue, &model, format)?;

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
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track_model: Model,
        ship_model: Model,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let track = Drawable::new(device, queue, track_model, format)?;
        let ship = Drawable::new(device, queue, ship_model, format)?;
        Ok(Self {
            track,
            ship,
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
        format,
        (width, height),
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
                ride_height: 4.0,
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

        // Lifted off the surface line by the probe's own reach, which is not
        // `track::HOVER_LIFT`; see `spawn_height`.
        assert_eq!(spawn_height(&handling), 4.0);
        assert_ne!(spawn_height(&handling), track::HOVER_LIFT);
        let up = (-Vec3::from_array(start.down)).normalize();
        let offset = race.ship().physics.body.position - Vec3::from_array(start.pos);
        assert!(
            (offset.dot(up) - spawn_height(&handling)).abs() < 1e-4,
            "{offset}"
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
