//! `oag-trace`: run our simulation over a recording and say where the two part
//! company.
//!
//! ```sh
//! # what is in a capture at all
//! oag-trace show data/traces/talons-junction-time-trial-lap.csv
//!
//! # replay it through our physics, on the track it was captured on
//! oag-trace run data/traces/talons-junction-time-trial-lap.csv \
//!     --source data/images/pulse-psp-usa.chd --hold cross --out /tmp/ours.csv
//!
//! # or diff two traces that already exist
//! oag-trace compare data/traces/talons-junction-time-trial-lap.csv /tmp/ours.csv
//! ```
//!
//! Traces are derived game data and live under `data/traces/`, which is
//! gitignored; nothing here can run in CI.

mod loader;
mod locator;
mod logging;

use loader::{checked_class, load};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use log::{debug, info, warn};
use oag_core::math::Vec3;
use oag_gameplay::ControlScheme;
use oag_gameplay::Ship;
use oag_gameplay::input::button_from_name;
use oag_gameplay::spawn::{Pose, box_inertia, spawn_height};
use oag_physics::{CollisionWorld, Environment, Handling, Ray, Raycaster};
use oag_pulse as pulse;
use oag_race::Course;
use oag_trace::compare::Tolerances;
use oag_trace::replay::{Basis, DeltaSource, DriveOptions, Held, Inputs, Options};
use oag_trace::trace::AngularReading;
use oag_trace::{Script, Trace, compare, plan, replay};
use oag_vex::{track, vex};

/// The track a recording is assumed to have been taken on unless another is
/// named: the one `oag-game` races on, measured in `oag_pulse::race::DEFAULT_TRACK`.
const DEFAULT_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// The team whose `handlingstats.xml` is read unless another is named: the
/// `oag-game` default, and the reference scenario's in `docs/reverse-engineering/ppsspp-debugger.md`.
const DEFAULT_TEAM: &str = "Assegai";

/// Our own fixed timestep, for `--fixed-dt`. ADR-0007.
const FIXED_DT: f32 = 1.0 / 60.0;

#[derive(Debug, Parser)]
#[command(
    name = "oag-trace",
    about = "Compare a simulation run against a trace captured from the original",
    long_about = None
)]
struct Cli {
    #[command(flatten)]
    log: oag_log::tool::LogArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Describe a captured trace: length, frame times, distance travelled.
    Show {
        /// The CSV `scripts/psp-trace.py` wrote.
        trace: PathBuf,
    },
    /// Replay a capture through our physics and compare the two.
    Run {
        /// The CSV `scripts/psp-trace.py` wrote.
        trace: PathBuf,
        /// A disc image, or a directory extracted with `oag-unpack`.
        ///
        /// Without one the run has no handling parameters and no track, which
        /// makes it a ballistic coast: useful for checking the harness, useless
        /// for checking the physics.
        #[arg(long)]
        source: Option<String>,
        /// Archive entry name of the track's `.vex`.
        #[arg(long, default_value = DEFAULT_TRACK)]
        track: String,
        /// Team, which selects the handling stats.
        #[arg(long, default_value = DEFAULT_TEAM)]
        team: String,
        /// Speed class the capture was taken in.
        #[arg(long, default_value = "venom")]
        class: String,
        /// Write the simulated trace here, in the same columns.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Drive the run from a committed input script, the same file
        /// `psp-trace.py --script` drove the capture with.
        ///
        /// The only mode where a *varying* input is known rather than inferred.
        /// See `oag_trace::script` for the format and
        /// `verification/scenarios/` for the committed ones.
        #[arg(long, value_name = "FILE", conflicts_with_all = ["hold", "steer", "airbrake_left", "airbrake_right"])]
        script: Option<PathBuf>,
        /// Release the script's first N ticks, as the capture harness did.
        ///
        /// **Pass the same N the capture was taken with**, which for everything
        /// under `data/traces/` is `2` - `just scripted-emu` runs `psp-trace.py
        /// --script-lead 2`. That option sends the script's tick `k + N` at the
        /// breakpoint for tick `k`, so the first N states never reach the
        /// emulator at all, and a replay that applies them is driving an input
        /// the capture never saw. See `oag_trace::script::Script::with_capture_lead`
        /// for why two ticks survive a 3,146-tick run rather than washing out.
        #[arg(long, value_name = "N", default_value_t = 0, requires = "script")]
        script_lead: usize,
        /// Hold a button for the whole run, e.g. `--hold cross`. Repeatable.
        ///
        /// Matches `psp-trace.py --hold`. Giving any held input switches the run
        /// off the recording's own control states, which are already ramped, and
        /// onto the constant input the capture was actually taken with.
        #[arg(long, value_name = "BUTTON")]
        hold: Vec<String>,
        /// Hold the stick at this X, `-1..=1`. Implies a held run.
        #[arg(long, allow_negative_numbers = true)]
        steer: Option<f32>,
        /// Hold the left airbrake here, `0..=1`. Implies a held run.
        #[arg(long)]
        airbrake_left: Option<f32>,
        /// Hold the right airbrake here, `0..=1`. Implies a held run.
        #[arg(long)]
        airbrake_right: Option<f32>,
        /// Step at our own 60 Hz instead of the recording's own frame times.
        #[arg(long)]
        fixed_dt: bool,
        /// Put the ship back on the recording every N ticks.
        ///
        /// Without this a run is seeded once, at tick 0, and a long comparison
        /// measures how far two chaotic trajectories drift apart rather than how
        /// good the force law is. The original integrates the frame durations it
        /// measured, so **two captures of the same script from the original
        /// itself diverge by 100 units at tick 495** - no implementation can pass
        /// a single-seeded three-thousand-tick comparison.
        ///
        /// With it, the recording becomes a sequence of independent N-tick
        /// comparisons and every error is a statement about one window. Use both:
        /// the single-seeded run says how long we track, the reseeded one says
        /// how wrong the physics is per window. They are different questions.
        #[arg(long, value_name = "N")]
        reseed: Option<NonZeroUsize>,
        /// How the recorded basis maps onto the body's axes.
        #[arg(long, value_enum, default_value_t = BasisArg::LeftUpForward)]
        basis: BasisArg,
        /// What the recorded angular-velocity column means.
        ///
        /// Both the sign and the frame are open questions - see
        /// `oag_trace::trace::AngularReading` - so this is a switch, and
        /// `oag-trace show` scores all four readings against the recording's own
        /// basis derivative without needing a run at all.
        #[arg(long, value_enum, default_value_t = AngularArg::NegatedLocal)]
        angular: AngularArg,
        /// Control scheme: `veteran` or `novice`.
        ///
        /// **Leave this alone unless you know the capture was taken under the
        /// other one.** A script says `l`, not "left airbrake" or "sideshift
        /// button", so the scheme decides what an `l` in the recording's own
        /// script *means* - and a replay driven under a scheme the capture was
        /// not taken under can only make the original disagree with itself.
        ///
        /// In practice nothing in `data/traces/` is affected: replaying
        /// `talons-junction-clean-lap.csv` under both schemes is **byte-identical**
        /// despite 58 uses of `l`/`r`, because the autopilot holds its airbrakes
        /// rather than tapping them and never centres the stick while holding
        /// one, so neither gesture ever fires. The flag is here for the day a
        /// capture of a *sideshift* exists, which is what
        /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md` gives the recipe
        /// for.
        #[arg(long, default_value_t = ControlScheme::default())]
        scheme: ControlScheme,
        /// Ignore the track's collision geometry: a ship with nothing to hover on.
        #[arg(long)]
        no_collision: bool,
        #[command(flatten)]
        tolerances: ToleranceArgs,
    },
    /// Validate an input script and say what it does.
    ///
    /// `--expand` writes one line per tick, which is exactly what
    /// `scripts/input_script.py --expand` writes: diffing the two is a proof
    /// that the emulator side and this side read a script identically, which is
    /// the whole premise of driving both from one file.
    Script {
        /// The script, e.g. `verification/scenarios/steer-both-ways.inputs`.
        script: PathBuf,
        /// Write one line per tick instead of a summary.
        #[arg(long)]
        expand: bool,
    },
    /// Run an input script through our physics, with no capture involved.
    ///
    /// `run` needs a recording: it takes the run's length, its per-tick delta and
    /// its initial condition from one. This needs only the script and a disc -
    /// the ship starts on the track's own start line the way `oag-game --race`
    /// puts it there - so a scenario can be exercised against our side before, or
    /// without, anybody capturing it. `just scripted-sim` is this.
    Drive {
        /// The input script, e.g. `verification/scenarios/steer-left.inputs`.
        script: PathBuf,
        /// Start the craft here, `x,y,z`, instead of on the track's own grid slot.
        ///
        /// A script planned by `oag-trace plan --start` was planned from a pose
        /// the track's authored `Start Position` is not - the emulator's time
        /// trial line is about 138 units past it on Talon's Junction - so
        /// replaying it from the grid slot runs a different scenario. Pass the
        /// same pose the plan was made from and the two runs are one run.
        #[arg(long, value_parser = parse_vec3)]
        start: Option<Vec3>,
        /// Turn the craft this many degrees off the spline tangent at `--start`,
        /// positive toward its right. Ignored without `--start`.
        #[arg(long, default_value_t = 0.0, requires = "start")]
        start_yaw: f32,
        /// Release the script's first N ticks, as the capture harness did.
        ///
        /// Only worth setting when the run is going to be held next to a capture
        /// of the same script: everything under `data/traces/` was taken through
        /// `psp-trace.py --script-lead 2`, which never sends the script's first
        /// two ticks. See `oag_trace::script::Script::with_capture_lead`.
        #[arg(long, value_name = "N", default_value_t = 0)]
        script_lead: usize,
        /// A disc image, or a directory extracted with `oag-unpack`.
        #[arg(long)]
        source: String,
        /// Archive entry name of the track's `.vex`.
        #[arg(long, default_value = DEFAULT_TRACK)]
        track: String,
        /// Team, which selects the handling stats.
        #[arg(long, default_value = DEFAULT_TEAM)]
        team: String,
        /// Speed class to run in.
        #[arg(long, default_value = "venom")]
        class: String,
        /// How many ticks to step. Defaults to the script's own length.
        #[arg(long)]
        ticks: Option<usize>,
        /// How often the report prints a row.
        #[arg(long, default_value_t = 100)]
        every: usize,
        /// Write the run here, in a capture's own columns.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Ignore the track's collision geometry: a ship with nothing to hover on.
        #[arg(long)]
        no_collision: bool,
        /// How our axes are written into the recorded basis columns.
        #[arg(long, value_enum, default_value_t = BasisArg::LeftUpForward)]
        basis: BasisArg,
        /// Which reading the angular-velocity column is written in.
        #[arg(long, value_enum, default_value_t = AngularArg::NegatedLocal)]
        angular: AngularArg,
        /// Control scheme: `veteran` or `novice`.
        ///
        /// Only the sideshift gesture differs, so a scenario that never asks for
        /// one runs identically under either. `novice` is what makes a
        /// hold-and-flick scenario mean anything. See
        /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
        #[arg(long, default_value_t = ControlScheme::default())]
        scheme: ControlScheme,
    },
    /// Dump a track's driveable spline as CSV, one row per resampled point.
    ///
    /// The autopilot in `scripts/psp-autopilot.py` steers the original along
    /// this: a scripted lap has to be *authored* from somewhere, and the
    /// authored racing line is already on the disc. Nothing here simulates
    /// anything - it is the spline the track file carries, resampled and lifted
    /// to hover height, in the same world coordinates a capture records.
    Track {
        /// A disc image, or a directory extracted with `oag-unpack`.
        #[arg(long)]
        source: String,
        /// Archive entry name of the track's `.vex`.
        #[arg(long, default_value = DEFAULT_TRACK)]
        track: String,
        /// Curve samples per control-point interval.
        #[arg(long, default_value_t = Course::STEPS_PER_SEGMENT)]
        steps: usize,
    },
    /// Dump a track's pad trigger volumes as CSV, one row per pad.
    ///
    /// Each row carries the pad's world-space centre, its push axis and its
    /// arc-length progress along the course. `--before N` adds the on-spline
    /// point N units *upstream* of the pad - position at hover height, tangent
    /// and up - which is exactly the pose `scripts/psp-drive.py place` needs to
    /// set a craft up for a run at the pad.
    Pads {
        /// A disc image, or a directory extracted with `oag-unpack`.
        #[arg(long)]
        source: String,
        /// Archive entry name of the track's `.vex`.
        #[arg(long, default_value = DEFAULT_TRACK)]
        track: String,
        /// Also emit the approach point this many units before each pad.
        /// Repeatable: `--before 50 --before 120`.
        #[arg(long = "before")]
        before: Vec<f32>,
    },
    /// Find an input script that drives our simulation through a point on the track.
    ///
    /// The plan-then-replay half of `scripts/psp-autopilot.py`: instead of
    /// steering the original over a websocket and recording the result, this
    /// steers *our* craft and records the result, at no emulator cost. What comes
    /// out is a committed-format input script, and the one thing worth doing with
    /// it is replaying it into PPSSPP - `scripts/psp-trace.py --script <file>
    /// --script-lead 2` - which is the only thing that shows whether the plan
    /// transfers. See `docs/tools/autopilot-planning.md`.
    ///
    /// ```sh
    /// oag-trace plan --source data/images/pulse-psp-usa.chd --pad 0 \
    ///     --start 6.07,-50.07,-196.10 --out /tmp/pad0.inputs
    /// ```
    Plan(PlanArgs),
    /// Compare two traces that already exist.
    Compare {
        /// The recording, from the original.
        recorded: PathBuf,
        /// The simulated run, from `oag-trace run --out`.
        simulated: PathBuf,
        #[command(flatten)]
        tolerances: ToleranceArgs,
    },
}

/// Everything `plan` was asked for.
#[derive(Debug, Args)]
struct PlanArgs {
    /// A disc image, or a directory extracted with `oag-unpack`.
    #[arg(long)]
    source: String,
    /// Archive entry name of the track's `.vex`.
    #[arg(long, default_value = DEFAULT_TRACK)]
    track: String,
    /// Team, which selects the handling stats.
    #[arg(long, default_value = DEFAULT_TEAM)]
    team: String,
    /// Speed class to run in.
    #[arg(long, default_value = "venom")]
    class: String,
    /// Aim at speed pad N, numbered as `oag-trace pads` numbers them.
    ///
    /// The gate's centre, direction and width all come off the pad, and the
    /// pad's own trigger box is tested as well as the plane, so the report
    /// says whether the craft would actually have set the pad off.
    #[arg(long, conflicts_with = "gate")]
    pad: Option<usize>,
    /// Aim at an arbitrary world point instead: `x,y,z`.
    #[arg(long, value_parser = parse_vec3)]
    gate: Option<Vec3>,
    /// Direction to cross `--gate` in, `x,y,z`. Defaults to the spline
    /// tangent there, which is what a gate across the track means.
    #[arg(long, value_parser = parse_vec3, requires = "gate")]
    gate_dir: Option<Vec3>,
    /// Half the gate's width, for the hit report. Defaults to the pad's own
    /// half width, or 5 units for a `--gate`.
    #[arg(long)]
    gate_half_width: Option<f32>,
    /// Start the craft here, `x,y,z`, instead of on the track's own grid slot.
    ///
    /// **The emulator does not start where our race does.** On Talon's
    /// Junction the authored `Start Position` sits about 138 units behind
    /// where a time trial actually begins, so a plan made from the grid slot
    /// is a plan for a different run. Read the craft's position out of the
    /// emulator - `scripts/psp-trace.py` writes it - and pass it here.
    #[arg(long, value_parser = parse_vec3)]
    start: Option<Vec3>,
    /// Turn the craft this many degrees off the spline tangent at `--start`,
    /// positive toward its right. Ignored without `--start`.
    #[arg(long, default_value_t = 0.0)]
    start_yaw: f32,
    /// Give up after this many ticks without reaching the gate.
    #[arg(long, default_value_t = 1800)]
    max_ticks: usize,
    /// Hold nothing for the first N ticks, because `psp-trace.py
    /// --script-lead N` never sends them. Match the two or the run planned
    /// here is not the run replayed there.
    #[arg(long, default_value_t = 2)]
    lead: usize,
    /// Keep planning this many ticks past the crossing.
    #[arg(long, default_value_t = 60)]
    after: usize,
    /// Write the script here. Without it, it goes to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Also write the planned run as a trace CSV, in a capture's columns.
    #[arg(long)]
    trace_out: Option<PathBuf>,
    /// How often the report prints a row.
    #[arg(long, default_value_t = 30)]
    every: usize,
    /// Lookahead distance at a standstill.
    #[arg(long, default_value_t = 18.0)]
    look_min: f32,
    /// Extra lookahead per unit of speed.
    #[arg(long, default_value_t = 0.55)]
    look_speed: f32,
    /// Lookahead ceiling.
    #[arg(long, default_value_t = 90.0)]
    look_max: f32,
    /// Lateral error below which nothing is held.
    #[arg(long, default_value_t = 0.045)]
    deadband: f32,
    /// Lateral error past which the inside airbrake comes on as well.
    #[arg(long, default_value_t = 0.32)]
    brake_at: f32,
    /// Control scheme the emitted script is meant to be read under.
    #[arg(long, default_value_t = ControlScheme::default())]
    scheme: ControlScheme,
}

/// How the recorded basis maps onto the body's axes.
///
/// Two readings, not four: the recorded basis is positively oriented and the
/// body's is not, so the flips come in pairs. See `oag_trace::replay::Basis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BasisArg {
    /// The measured reading: the capture's `right_*` columns are the ship's left.
    LeftUpForward,
    /// What the column names say, with row 2 then necessarily the tail.
    RightUpBack,
}

impl From<BasisArg> for Basis {
    fn from(value: BasisArg) -> Self {
        match value {
            BasisArg::LeftUpForward => Self::LeftUpForward,
            BasisArg::RightUpBack => Self::RightUpBack,
        }
    }
}

/// What the recorded angular-velocity column holds: a sign and a frame, both
/// open. See `oag_trace::trace::AngularReading`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AngularArg {
    /// Body-local components, negated. The default, from the PS2 sign result and
    /// the PS2 page's name for the field.
    NegatedLocal,
    /// Body-local components, in the physical sign.
    Local,
    /// World space, negated.
    NegatedWorld,
    /// World space, in the physical sign.
    World,
}

impl From<AngularArg> for AngularReading {
    fn from(value: AngularArg) -> Self {
        match value {
            AngularArg::NegatedLocal => Self::NegatedLocal,
            AngularArg::Local => Self::Local,
            AngularArg::NegatedWorld => Self::NegatedWorld,
            AngularArg::World => Self::World,
        }
    }
}

/// Overrides for the protocol's tolerance table.
///
/// Every one defaults to what
/// `docs/reverse-engineering/verification-protocol.md` says, and the table there
/// is explicitly provisional, so these exist to widen or tighten a comparison
/// while a measurement is being argued about - not to make one pass.
#[derive(Debug, Clone, Copy, Args)]
struct ToleranceArgs {
    /// Position, absolute.
    #[arg(long)]
    position_absolute: Option<f32>,
    /// Position, relative.
    #[arg(long)]
    position_relative: Option<f32>,
    /// Orientation, radians.
    #[arg(long)]
    orientation: Option<f32>,
    /// Velocity, relative.
    #[arg(long)]
    velocity_relative: Option<f32>,
    /// An absolute floor under the velocity comparison. Off by default; see the
    /// field docs on `oag_trace::compare::Tolerances`.
    #[arg(long)]
    velocity_absolute: Option<f32>,
    /// Control states, relative.
    #[arg(long)]
    control_relative: Option<f32>,
    /// An absolute floor under the control comparison, on the `0..=100` scale.
    #[arg(long)]
    control_absolute: Option<f32>,
    /// Angular velocity, relative.
    #[arg(long)]
    angular_velocity_relative: Option<f32>,
    /// An absolute floor under the angular-velocity comparison, in rad/s.
    /// **Nonzero by default**, unlike the other two floors; see the field docs on
    /// `oag_trace::compare::Tolerances`.
    #[arg(long)]
    angular_velocity_absolute: Option<f32>,
    /// Timers, absolute, in seconds.
    #[arg(long)]
    timer_absolute: Option<f32>,
    /// `0..=1` quantities (intensity, speed ramp, boost accumulator), absolute.
    #[arg(long)]
    unit_interval_absolute: Option<f32>,
    /// World-space lengths (the flare half-size), absolute, in units.
    #[arg(long)]
    length_absolute: Option<f32>,
    /// The relative half of the length tolerance.
    #[arg(long)]
    length_relative: Option<f32>,
}

impl From<ToleranceArgs> for Tolerances {
    fn from(args: ToleranceArgs) -> Self {
        let default = Self::default();
        Self {
            position_absolute: args.position_absolute.unwrap_or(default.position_absolute),
            position_relative: args.position_relative.unwrap_or(default.position_relative),
            orientation_radians: args.orientation.unwrap_or(default.orientation_radians),
            velocity_relative: args.velocity_relative.unwrap_or(default.velocity_relative),
            velocity_absolute: args.velocity_absolute.unwrap_or(default.velocity_absolute),
            control_relative: args.control_relative.unwrap_or(default.control_relative),
            control_absolute: args.control_absolute.unwrap_or(default.control_absolute),
            angular_velocity_relative: args
                .angular_velocity_relative
                .unwrap_or(default.angular_velocity_relative),
            angular_velocity_absolute: args
                .angular_velocity_absolute
                .unwrap_or(default.angular_velocity_absolute),
            timer_absolute: args.timer_absolute.unwrap_or(default.timer_absolute),
            unit_interval_absolute: args
                .unit_interval_absolute
                .unwrap_or(default.unit_interval_absolute),
            length_absolute: args.length_absolute.unwrap_or(default.length_absolute),
            length_relative: args.length_relative.unwrap_or(default.length_relative),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::start(&cli.log);
    match cli.command {
        Command::Show { trace } => show(&trace),
        Command::Script { script, expand } => {
            let parsed = read_script(&script)?;
            if expand {
                print!("{}", parsed.to_expanded());
            } else {
                println!("{} tick(s)", parsed.len());
                print!("{}", parsed.to_text());
            }
            Ok(())
        }
        Command::Track {
            source,
            track,
            steps,
        } => dump_track(&source, &track, steps),
        Command::Pads {
            source,
            track,
            before,
        } => dump_pads(&source, &track, &before),
        Command::Plan(args) => plan_scenario(args),
        Command::Drive {
            script,
            start,
            start_yaw,
            script_lead,
            source,
            track,
            team,
            class,
            ticks,
            every,
            out,
            no_collision,
            basis,
            angular,
            scheme,
        } => drive_scenario(DriveArgs {
            script,
            start,
            start_yaw,
            script_lead,
            source,
            track,
            team,
            class,
            ticks,
            every,
            out,
            no_collision,
            basis,
            angular,
            scheme,
        }),
        Command::Compare {
            recorded,
            simulated,
            tolerances,
        } => {
            let recorded = read_trace(&recorded)?;
            let simulated = read_trace(&simulated)?;
            println!("{}", compare(&recorded, &simulated, &tolerances.into()));
            Ok(())
        }
        Command::Run {
            trace,
            source,
            track,
            team,
            class,
            out,
            script,
            script_lead,
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            reseed,
            basis,
            angular,
            scheme,
            no_collision,
            tolerances,
        } => run(RunArgs {
            trace,
            source,
            track,
            team,
            class,
            out,
            script,
            script_lead,
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            reseed,
            basis,
            angular,
            scheme,
            no_collision,
            tolerances,
        }),
    }
}

/// Everything `drive` was asked for.
#[derive(Debug)]
struct DriveArgs {
    script: PathBuf,
    start: Option<Vec3>,
    start_yaw: f32,
    script_lead: usize,
    source: String,
    track: String,
    team: String,
    class: String,
    ticks: Option<usize>,
    every: usize,
    out: Option<PathBuf>,
    no_collision: bool,
    basis: BasisArg,
    angular: AngularArg,
    scheme: ControlScheme,
}

/// Runs a scenario through our physics from the track's own start line.
///
/// The report is the deliverable here, not the CSV: what a scenario run answers
/// is "did the ship go where the script asked, and if not, from which tick", and
/// a wall of 3,000 rows answers that worse than fifteen. So every `--every`
/// ticks it prints where the ship is, how fast, whether it is grounded, and **how
/// far it is from the track's own spline** - the last being the one number that
/// says whether the run is still on the circuit, and the reason this needs the
/// spline as well as the collision mesh.
fn drive_scenario(args: DriveArgs) -> Result<()> {
    let script = read_script(&args.script)?.with_capture_lead(args.script_lead);
    if script.is_empty() {
        bail!("{}: the script has no ticks in it", args.script.display());
    }
    if args.every == 0 {
        bail!("--every must be at least 1");
    }
    let ticks = args.ticks.unwrap_or(script.len());
    if ticks > script.len() {
        warn!(
            "{} covers {} tick(s) and --ticks asks for {ticks}, so its last \
             state is held for the remaining {}",
            args.script.display(),
            script.len(),
            ticks - script.len(),
        );
    }

    let class = checked_class(&args.class)?;
    let (handling, collision) = load(&args.source, &args.track, &args.team, class)?;
    let samples = locator::load_samples(Some(&args.source), &args.track)?;
    let start_position = load_start_position(&args.source, &args.track)?;
    let start = samples
        .first()
        .copied()
        .context("the track's spline has no samples, so there is nowhere to start")?;
    let collision = if args.no_collision {
        info!("--no-collision, so the hover probes see nothing");
        CollisionWorld::new()
    } else {
        collision
    };

    // The same placement `oag_raceplay` makes, and for the same reason: the
    // track's own `Start Position` when it has one, at the height the hover spring
    // rests at, and the first spline sample only when it has not. A scenario run
    // that started anywhere else would not be the scenario - unless the caller
    // says otherwise, which is what `--start` is: a script planned from the
    // emulator's own start line has to be replayed from it.
    let pose = match (args.start, &start_position) {
        (Some(position), _) => {
            let near = nearest_sample(&samples, position);
            info!(
                "--start ({:.3}, {:.3}, {:.3}), attitude from spline sample {near} \
                 yawed {:.1} degrees - not the track's own grid slot",
                position.x, position.y, position.z, args.start_yaw,
            );
            Pose::from_position_on_sample(&samples[near], position, args.start_yaw.to_radians())
        }
        (None, start_position) => match start_position {
            Some(slot) => {
                let origin = Vec3::from_array(slot.position) + Vec3::Y * 20.0;
                let ground = collision
                    .raycast(Ray::new(origin, Vec3::NEG_Y, 80.0), None, false)
                    .map(|hit| hit.point.y);
                info!(
                    "{}: Start Position {:?} facing {:?}, ground {ground:?}",
                    args.track, slot.position, slot.forward
                );
                Pose::from_start_position(slot, ground, spawn_height(&handling))
            }
            None => {
                warn!(
                    "{}: no Start Position node, starting on the spline",
                    args.track
                );
                Pose::from_sample(&start, start.racing_line, spawn_height(&handling))
            }
        },
    };
    let mut ship = Ship::default();
    ship.physics.body.mass = handling.physical.mass;
    ship.physics.body.inertia = box_inertia();
    ship.place_at(pose);

    let options = DriveOptions {
        ticks,
        dt: FIXED_DT,
        basis: args.basis.into(),
        angular: args.angular.into(),
        scheme: args.scheme,
    };
    println!(
        "{}: {} tick(s) at {} Hz, {} on {}, {:?} class",
        args.script.display(),
        ticks,
        (1.0 / FIXED_DT).round(),
        args.team,
        args.track,
        class,
    );
    println!(
        "start: ({:.3}, {:.3}, {:.3}), spawn height {:.3} above the surface line",
        pose.position.x,
        pose.position.y,
        pose.position.z,
        spawn_height(&handling),
    );
    if args.script_lead > 0 {
        println!(
            "--script-lead {}: the first {} tick(s) of the script are released, \
             the way a capture taken with psp-trace.py --script-lead {} never \
             received them",
            args.script_lead, args.script_lead, args.script_lead,
        );
    }
    println!();

    let run = replay::drive(
        ship.physics,
        &handling,
        &Environment::default(),
        &collision,
        &script.states,
        &options,
        Some(&samples),
    );

    report(&run, &samples, args.every);

    if let Some(path) = &args.out {
        std::fs::write(path, run.to_csv())
            .with_context(|| format!("writing {}", path.display()))?;
        info!("wrote {}", path.display());
    }
    Ok(())
}

/// Parses an `x,y,z` command-line vector.
fn parse_vec3(text: &str) -> Result<Vec3, String> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    let [x, y, z] = parts.as_slice() else {
        return Err(format!("{text:?} is not three comma-separated numbers"));
    };
    let mut out = [0.0f32; 3];
    for (slot, part) in out.iter_mut().zip([x, y, z]) {
        *slot = part
            .parse()
            .map_err(|_| format!("{part:?} is not a number"))?;
    }
    Ok(Vec3::from_array(out))
}

/// Plans an input script that drives our craft through a gate, and writes it out.
///
/// The report is where the run went and where it crossed; the artefact is the
/// script. Everything the run needs beyond the physics comes from the same disc
/// the physics does - the pad's box, the racing line, the handling stats - so a
/// plan is reproducible from an image and a command line, with nothing tuned by
/// hand in between.
fn plan_scenario(args: PlanArgs) -> Result<()> {
    if args.every == 0 {
        bail!("--every must be at least 1");
    }
    let class = checked_class(&args.class)?;
    let (handling, collision) = load(&args.source, &args.track, &args.team, class)?;
    let blob = read_track_blob(&args.source, &args.track)?;
    let ai = ai_of(&blob, &args.track)?;
    let samples: Vec<track::Sample> = resample(&ai, Course::STEPS_PER_SEGMENT)
        .into_iter()
        .map(|(_, sample)| sample)
        .collect();
    if samples.is_empty() {
        bail!("{}: the track's spline has no samples", args.track);
    }

    // The gate: a pad's own box and push axis when there is a pad, an authored
    // point and the spline's tangent there when there is not.
    let (gate, volume) = match (args.pad, args.gate) {
        (Some(index), _) => {
            let nodes = vex::nodes(&blob).context("walking the node tree")?;
            let volumes = oag_vex::pads::volumes(&blob, &nodes, vex::CLASS_SPEEDUP_PAD);
            let pad = volumes.get(index).copied().with_context(|| {
                format!(
                    "{} has {} speed pad(s), so there is no pad {index}",
                    args.track,
                    volumes.len()
                )
            })?;
            let direction = pad
                .direction()
                .context("the pad's transform has a degenerate push axis")?;
            // The box's own half extent across its push axis, which is what
            // "did the craft go through the pad" is actually asking about.
            let half = args
                .gate_half_width
                .unwrap_or((pad.max[0] - pad.min[0]) * 0.5);
            (
                plan::Gate::new(
                    Vec3::from_array(pad.centre()),
                    Vec3::from_array(direction),
                    half,
                ),
                Some(pad),
            )
        }
        (None, Some(centre)) => {
            let direction = args.gate_dir.unwrap_or_else(|| {
                let near = nearest_sample(&samples, centre);
                Vec3::from_array(samples[near].tangent)
            });
            (
                plan::Gate::new(centre, direction, args.gate_half_width.unwrap_or(5.0)),
                None,
            )
        }
        (None, None) => bail!("nothing to aim at: pass --pad N or --gate x,y,z"),
    };

    // Where the craft starts. `--start` is the one that matters for a plan meant
    // to be replayed into the emulator, because the emulator's start line is not
    // the track's authored grid slot - see the flag's own help.
    let pose = match args.start {
        Some(position) => {
            let near = nearest_sample(&samples, position);
            info!(
                "start: ({:.3}, {:.3}, {:.3}), attitude from spline sample {near} \
                 yawed {:.1} degrees",
                position.x, position.y, position.z, args.start_yaw,
            );
            Pose::from_position_on_sample(&samples[near], position, args.start_yaw.to_radians())
        }
        None => {
            let slot = load_start_position(&args.source, &args.track)?;
            match &slot {
                Some(slot) => {
                    let origin = Vec3::from_array(slot.position) + Vec3::Y * 20.0;
                    let ground = collision
                        .raycast(Ray::new(origin, Vec3::NEG_Y, 80.0), None, false)
                        .map(|hit| hit.point.y);
                    info!(
                        "start: the track's own Start Position {:?}. This is *not* where \
                         a time trial begins - pass --start to plan for the emulator's.",
                        slot.position,
                    );
                    Pose::from_start_position(slot, ground, spawn_height(&handling))
                }
                None => {
                    warn!("start: no Start Position node, starting on the spline");
                    Pose::from_sample(&samples[0], samples[0].racing_line, spawn_height(&handling))
                }
            }
        }
    };

    let path = plan::Path::to_gate(&samples, pose.position, &gate, 60.0)
        .context("building the line from the start to the gate")?;
    let mut ship = Ship::default();
    ship.physics.body.mass = handling.physical.mass;
    ship.physics.body.inertia = box_inertia();
    ship.place_at(pose);

    let options = plan::PlanOptions {
        drive: DriveOptions {
            ticks: args.max_ticks,
            dt: FIXED_DT,
            basis: Basis::default(),
            angular: AngularReading::default(),
            scheme: args.scheme,
        },
        lead: args.lead,
        after: args.after,
        tuning: plan::Tuning {
            look_min: args.look_min,
            look_speed: args.look_speed,
            look_max: args.look_max,
            deadband: args.deadband,
            brake_at: args.brake_at,
            ..plan::Tuning::default()
        },
        volume,
    };

    println!(
        "gate ({:.1}, {:.1}, {:.1}) facing ({:.3}, {:.3}, {:.3}), half width {:.1}",
        gate.centre.x,
        gate.centre.y,
        gate.centre.z,
        gate.direction.x,
        gate.direction.y,
        gate.direction.z,
        gate.half_width,
    );
    println!(
        "line: {} point(s), gate at {}, {} on {}, {:?} class, lead {} tick(s)",
        path.points.len(),
        path.gate_index,
        args.team,
        args.track,
        class,
        args.lead,
    );
    println!();

    let planned = plan::to_gate(
        ship.physics,
        &handling,
        &Environment::default(),
        &collision,
        &path,
        &gate,
        &options,
        Some(&samples),
    );
    report(&planned.trace, &samples, args.every);
    println!();

    match planned.crossing {
        Some(crossing) => {
            let hit = if crossing.offset.abs() <= gate.half_width {
                "inside"
            } else {
                "OUTSIDE"
            };
            println!(
                "crossed the gate on tick {} at ({:.1}, {:.1}, {:.1}), {:.2} units \
                 {} of centre - {hit} its half width - doing {:.1}",
                crossing.tick,
                crossing.point.x,
                crossing.point.y,
                crossing.point.z,
                crossing.offset.abs(),
                if crossing.offset >= 0.0 {
                    "right"
                } else {
                    "left"
                },
                crossing.speed,
            );
        }
        None => println!(
            "never reached the gate in {} tick(s) - the script below is not a plan",
            planned.trace.len()
        ),
    }
    match (options.volume.is_some(), planned.inside) {
        (true, Some(tick)) => println!("inside the pad's own trigger box from tick {tick}"),
        (true, None) => {
            println!("never inside the pad's own trigger box, so the pad would not fire")
        }
        (false, _) => {}
    }
    println!("{} tick(s) of input planned", planned.script.len());

    let text = planned.script.to_text();
    match &args.out {
        Some(path) => {
            std::fs::write(path, &text).with_context(|| format!("writing {}", path.display()))?;
            info!("wrote {}", path.display());
        }
        None => println!("\n{text}"),
    }
    if let Some(path) = &args.trace_out {
        std::fs::write(path, planned.trace.to_csv())
            .with_context(|| format!("writing {}", path.display()))?;
        info!("wrote {}", path.display());
    }
    Ok(())
}

/// Index of the spline sample whose surface point is nearest a world position.
fn nearest_sample(samples: &[track::Sample], position: Vec3) -> usize {
    samples
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = Vec3::from_array(a.pos).distance_squared(position);
            let db = Vec3::from_array(b.pos).distance_squared(position);
            da.total_cmp(&db)
        })
        .map_or(0, |(index, _)| index)
}

/// The run report: a row every `every` ticks, then what the whole run did.
fn report(run: &Trace, samples: &[track::Sample], every: usize) {
    println!("tick        position                  speed  grounded  off-spline");
    let mut rows: Vec<usize> = (0..run.len()).step_by(every).collect();
    if rows.last() != Some(&(run.len() - 1)) {
        rows.push(run.len() - 1);
    }
    for index in rows {
        let frame = &run.frames[index];
        println!(
            "{:>5}  ({:>9.1},{:>8.1},{:>9.1})  {:>6.1}     {:>4.2}  {:>10.1}",
            frame.tick,
            frame.position.x,
            frame.position.y,
            frame.position.z,
            frame.velocity.length(),
            frame.grounded,
            off_spline(frame.position, samples),
        );
    }

    let grounded = run.frames.iter().filter(|f| f.grounded > 0.0).count();
    let worst = run
        .frames
        .iter()
        .map(|f| off_spline(f.position, samples))
        .fold(0.0f32, f32::max);
    // The first tick after which the ship never touches the surface again: a run
    // that leaves the track and one that scrapes a crest look identical in a
    // grounded *count* and are not the same failure.
    let lost = run
        .frames
        .iter()
        .rposition(|f| f.grounded > 0.0)
        .filter(|last| *last + 1 < run.len())
        .map(|last| last + 1);
    let finite = run
        .frames
        .iter()
        .all(|f| f.position.is_finite() && f.velocity.is_finite());

    println!();
    println!("{}", run.summary());
    println!(
        "grounded on {grounded}/{} tick(s); worst distance from the spline {worst:.1}",
        run.len()
    );
    match lost {
        Some(tick) => println!("left the surface for good at tick {tick} and never regained it"),
        None => println!("still in contact with the track on the last tick"),
    }
    if !finite {
        println!("WARNING: the run went non-finite - that is a bug, not a divergence");
    }
}

/// Distance from a point to the nearest spline sample.
///
/// A linear scan, and deliberately: `oag_raceplay::Spline` scans too, because
/// the comparison feeds simulation state and the order it happens in must not
/// vary between runs. Here it only feeds a report, but the same answer for the
/// same input is worth more than the microseconds.
fn off_spline(position: oag_core::math::Vec3, samples: &[track::Sample]) -> f32 {
    samples
        .iter()
        .map(|s| (oag_core::math::Vec3::from_array(s.pos) - position).length())
        .fold(f32::INFINITY, f32::min)
}

/// Reads a track's authored grid slot out of its `.vex`, for `drive`.
///
/// `Ok(None)` when the file carries no `Start Position` node, which is not an
/// error: a track without one is a track a ship starts on the spline of, the
/// same fallback `oag_raceplay` takes.
fn load_start_position(source: &str, name: &str) -> Result<Option<track::StartPosition>> {
    let mut archives = pulse::open(source)?;
    let blob = archives
        .read_name(name)
        .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))?;
    let nodes = vex::nodes(&blob).context("walking the node tree")?;
    let Some(node) = nodes
        .iter()
        .find(|n| n.class_id == vex::CLASS_START_POSITION)
    else {
        return Ok(None);
    };
    let payload = blob
        .get(node.payload())
        .context("the Start Position payload runs past the end of the file")?;
    Ok(track::start_position(payload, vex::byte_order(&blob)))
}

/// Reads a track's `WO Track` spline graph out of its `.vex`, for `track` and
/// for `drive`.
fn read_track_blob(source: &str, name: &str) -> Result<Vec<u8>> {
    let mut archives = pulse::open(source)?;
    archives
        .read_name(name)
        .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
}

fn ai_of(blob: &[u8], name: &str) -> Result<track::AiTrack> {
    let nodes = vex::nodes(blob).context("walking the node tree")?;
    // By the class id this file's own version word implies, not by the version-6
    // constant: `CLASS_WO_TRACK` finds nothing in a version-4 `.vex`, which is
    // every track on Pure's disc.
    let node =
        track::find_node(blob, &nodes).with_context(|| format!("{name} has no WO Track node"))?;
    let payload = blob
        .get(node.payload())
        .context("the WO Track payload runs past the end of the file")?;
    let ai = track::parse(payload).map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
    debug!(
        "{name}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    );
    Ok(ai)
}

pub(crate) fn load_ai(source: &str, name: &str) -> Result<track::AiTrack> {
    ai_of(&read_track_blob(source, name)?, name)
}

/// Every path resampled, with the path each sample came from.
///
/// The same `Course::STEPS_PER_SEGMENT` and the same order `oag_raceplay::Spline` and
/// `oag_render::track` use, so a dumped line, a drawn line and the line a
/// scenario run starts on are one set of points.
pub(crate) fn resample(ai: &track::AiTrack, steps: usize) -> Vec<(usize, track::Sample)> {
    let mut samples = Vec::new();
    for (index, path) in ai.paths.iter().enumerate() {
        for segment in 0..path.points.len() {
            for step in 0..steps {
                if let Some(sample) = path.sample(segment, step as f32 / steps as f32) {
                    samples.push((index, sample));
                }
            }
        }
    }
    samples
}

/// Writes a track's resampled spline to stdout as CSV.
///
/// One row per sample, in the order the curve runs, carrying the frame
/// (`tangent`, `lateral`, `down`), both half-widths, the AI corridor and the
/// authored `racing_line` offset. Two derived columns come along because every
/// consumer computes them the same way and one of them is easy to get backwards:
///
/// - `lift_*` is the sample lifted off the surface by
///   [`oag_vex::track::HOVER_LIFT`], which is where ships actually fly.
/// - `line_*` is that point moved across the track by `racing_line` along
///   `lateral`, which is the authored line itself.
///
/// Nothing is renormalised: the interpolated axes are neither unit length nor
/// exactly perpendicular, and that is the original's behaviour rather than a
/// decode error - `oag_gameplay::spawn` says so at length.
fn dump_track(source: &str, name: &str, steps: usize) -> Result<()> {
    if steps == 0 {
        bail!("--steps must be at least 1");
    }
    let ai = load_ai(source, name)?;

    println!(
        "index,path,pos_x,pos_y,pos_z,lift_x,lift_y,lift_z,line_x,line_y,line_z,\
         tan_x,tan_y,tan_z,lat_x,lat_y,lat_z,down_x,down_y,down_z,\
         half_left,half_right,ai_left,ai_right,racing_line,section,flags"
    );
    let samples = resample(&ai, steps);
    for (index, (path_index, s)) in samples.iter().enumerate() {
        let lift = [
            s.pos[0] - track::HOVER_LIFT * s.down[0],
            s.pos[1] - track::HOVER_LIFT * s.down[1],
            s.pos[2] - track::HOVER_LIFT * s.down[2],
        ];
        let line = [
            lift[0] + s.racing_line * s.lateral[0],
            lift[1] + s.racing_line * s.lateral[1],
            lift[2] + s.racing_line * s.lateral[2],
        ];
        let mut row = vec![index.to_string(), path_index.to_string()];
        for v in [s.pos, lift, line, s.tangent, s.lateral, s.down] {
            row.extend(v.iter().map(|c| format!("{c:.7}")));
        }
        for v in [
            s.half_width_left,
            s.half_width_right,
            s.ai_bound_left,
            s.ai_bound_right,
            s.racing_line,
        ] {
            row.push(format!("{v:.7}"));
        }
        row.push(s.section_id.to_string());
        row.push(s.flags.to_string());
        println!("{}", row.join(","));
    }
    debug!("{} sample(s) at {steps} per segment", samples.len());
    Ok(())
}

/// Writes a track's pad trigger volumes to stdout as CSV.
///
/// One row per pad, speedup pads first then weapon pads, each numbered within
/// its class - `pad 3` of class `speedup` is stable across runs because
/// [`vex::nodes`] walks the file in file order. `progress` is
/// arc-length distance from the start line along the course ring, and `offset`
/// is how far the pad's centre sits from its nearest ring point - large means
/// the pad is off the primary ring (a shortcut branch) and its `progress` and
/// approach points should not be trusted.
///
/// Every `--before N` adds three column triples: the ring point N units
/// upstream of the pad lifted to hover height (`beforeN_x/y/z`, where ships
/// fly and where a teleported craft should be placed), the spline tangent
/// there (`beforeN_tan_*`, the direction to face and to align velocity with)
/// and the surface up (`beforeN_up_*`, the negated authored `down`).
fn dump_pads(source: &str, name: &str, before: &[f32]) -> Result<()> {
    let blob = read_track_blob(source, name)?;
    let ai = ai_of(&blob, name)?;
    let nodes = vex::nodes(&blob).context("walking the node tree")?;

    let start = nodes
        .iter()
        .find(|n| n.class_id == vex::CLASS_START_POSITION)
        .and_then(|n| blob.get(n.payload()))
        .and_then(|payload| track::start_position(payload, vex::byte_order(&blob)));
    let course = Course::from_track(&ai, start.map(|s| Vec3::from_array(s.position)))
        .context("the track's primary spline chain does not close into a ring")?;

    // The same resampling the ring itself was built from, so a ring index maps
    // back onto a full sample - tangent and down - by nearest position.
    let samples = resample(&ai, Course::STEPS_PER_SEGMENT);

    let mut header =
        vec!["pad,class,centre_x,centre_y,centre_z,dir_x,dir_y,dir_z,progress,offset".to_owned()];
    for n in before {
        for triple in ["", "_tan", "_up"] {
            for axis in ["x", "y", "z"] {
                header.push(format!("before{n:.0}{triple}_{axis}"));
            }
        }
    }
    println!("{}", header.join(","));

    let classes = [
        ("speedup", vex::CLASS_SPEEDUP_PAD),
        ("weapon", vex::CLASS_WEAPON_PAD),
    ];
    let mut total = 0usize;
    for (label, class_id) in classes {
        let volumes = oag_vex::pads::volumes(&blob, &nodes, class_id);
        for (index, pad) in volumes.iter().enumerate() {
            let centre = Vec3::from_array(pad.centre());
            let direction = pad.direction().unwrap_or([0.0; 3]);
            let located = course
                .locate(centre, None)
                .context("locating a pad on the course ring")?;

            let mut row = vec![index.to_string(), label.to_owned()];
            for v in [centre.to_array(), direction] {
                row.extend(v.iter().map(|c| format!("{c:.7}")));
            }
            row.push(format!("{:.7}", located.progress));
            row.push(format!("{:.7}", located.offset));

            for n in before {
                // "N units before" walks the long way round: forward by
                // `length - N` is backward by N on a closed ring, and stays
                // inside `advance`'s documented contract.
                let upstream = course.advance(located.index, course.length() - n);
                let point = course
                    .position(upstream)
                    .context("an advanced ring index is always in range")?;
                let sample = samples
                    .iter()
                    .map(|(_, s)| s)
                    .min_by(|a, b| {
                        let da = point.distance_squared(Vec3::from_array(a.pos));
                        let db = point.distance_squared(Vec3::from_array(b.pos));
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .context("a course ring always has samples")?;
                let lift = [
                    point.x - track::HOVER_LIFT * sample.down[0],
                    point.y - track::HOVER_LIFT * sample.down[1],
                    point.z - track::HOVER_LIFT * sample.down[2],
                ];
                let up = [-sample.down[0], -sample.down[1], -sample.down[2]];
                for v in [lift, sample.tangent, up] {
                    row.extend(v.iter().map(|c| format!("{c:.7}")));
                }
            }
            println!("{}", row.join(","));
            total += 1;
        }
    }
    debug!(
        "{total} pad(s), course length {:.1}, start at ring index {}",
        course.length(),
        course.start_index()
    );
    Ok(())
}

fn show(path: &Path) -> Result<()> {
    let trace = read_trace(path)?;
    println!("{}", trace.summary());
    Ok(())
}

/// Everything `run` was asked for.
#[derive(Debug)]
struct RunArgs {
    trace: PathBuf,
    source: Option<String>,
    track: String,
    team: String,
    class: String,
    out: Option<PathBuf>,
    script: Option<PathBuf>,
    script_lead: usize,
    hold: Vec<String>,
    steer: Option<f32>,
    airbrake_left: Option<f32>,
    airbrake_right: Option<f32>,
    fixed_dt: bool,
    reseed: Option<NonZeroUsize>,
    basis: BasisArg,
    angular: AngularArg,
    scheme: ControlScheme,
    no_collision: bool,
    tolerances: ToleranceArgs,
}

fn run(args: RunArgs) -> Result<()> {
    let recorded = read_trace(&args.trace)?;
    if recorded.is_empty() {
        bail!("{}: no ticks in the trace", args.trace.display());
    }

    let class = checked_class(&args.class)?;
    let (handling, collision) = match &args.source {
        Some(source) => load(source, &args.track, &args.team, class)?,
        None => {
            warn!(
                "no --source, so there are no handling parameters and no track. \
                 This run is a ballistic coast and says nothing about the force law."
            );
            let mut handling = Handling::ZERO;
            // The integrator divides by the body's mass, and the parameter set's
            // zero would make every step NaN. One, so the run is a coast rather
            // than an error, and said out loud rather than hidden.
            handling.physical.mass = 1.0;
            (handling, CollisionWorld::new())
        }
    };
    let samples = locator::load_samples(args.source.as_deref(), &args.track)?;
    let collision = if args.no_collision {
        info!("--no-collision, so the hover probes see nothing");
        CollisionWorld::new()
    } else {
        collision
    };
    let options = Options {
        inputs: inputs(&args, recorded.len())?,
        dt: if args.fixed_dt {
            DeltaSource::Fixed(FIXED_DT)
        } else {
            DeltaSource::Trace
        },
        basis: args.basis.into(),
        angular: args.angular.into(),
        scheme: args.scheme,
        reseed: args.reseed,
        pads: locator::load_speedup_pads(args.source.as_deref(), &args.track)?,
    };
    if args.fixed_dt {
        warn!(
            "--fixed-dt steps at {FIXED_DT}s while the recording's own deltas vary, \
             so some of any divergence is the timestep rather than the physics"
        );
    }

    println!("{}", recorded.summary());
    println!();

    let simulated = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &collision,
        &options,
        Some(&samples),
    );
    if let Some(path) = &args.out {
        // The reseed interval rides along in the file itself. A reseeded run's
        // columns are indistinguishable from a single-seeded one's, and a CSV
        // outlives the terminal it was produced in.
        let csv = match args.reseed {
            Some(every) => format!(
                "# reseeded from the recording every {} tick(s)\n{}",
                every,
                simulated.to_csv()
            ),
            None => simulated.to_csv(),
        };
        std::fs::write(path, csv).with_context(|| format!("writing {}", path.display()))?;
        info!("wrote {}", path.display());
    }

    if let Some(every) = args.reseed {
        println!(
            "reseeded from the recording every {every} tick(s), so what follows is \
             {every}-tick window error, not a trajectory comparison. Nothing below \
             says how long our run tracks the original - re-run without --reseed \
             for that."
        );
        println!();
    }

    println!(
        "{}",
        compare(&recorded, &simulated, &args.tolerances.into())
    );
    Ok(())
}

/// How the replay is driven: an authored script, a fixed input the capture was
/// taken with, or - failing both - the recording's own control states.
fn inputs(args: &RunArgs, ticks: usize) -> Result<Inputs> {
    if let Some(path) = &args.script {
        let script = read_script(path)?;
        if script.is_empty() {
            bail!("{}: the script has no ticks in it", path.display());
        }
        if script.len() < ticks {
            warn!(
                "{} covers {} tick(s) and the recording has {ticks}, so the \
                 script's last state is held for the remaining {}. That is nearly \
                 always an over-short script rather than an intent.",
                path.display(),
                script.len(),
                ticks - script.len(),
            );
        }
        return Ok(Inputs::Scripted(
            script.with_capture_lead(args.script_lead).states,
        ));
    }
    if args.hold.is_empty()
        && args.steer.is_none()
        && args.airbrake_left.is_none()
        && args.airbrake_right.is_none()
    {
        return Ok(Inputs::FromTrace);
    }
    let mut buttons = 0u32;
    for name in &args.hold {
        let button = button_from_name(name)
            .with_context(|| format!("{name:?} is not a button name the front end knows"))?;
        buttons |= button.bit();
    }
    Ok(Inputs::Held(Held {
        buttons,
        stick_x: args.steer.unwrap_or(0.0),
        stick_y: 0.0,
        airbrake_left: args.airbrake_left.unwrap_or(0.0),
        airbrake_right: args.airbrake_right.unwrap_or(0.0),
    }))
}

fn read_script(path: &Path) -> Result<Script> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Script::parse(&text).with_context(|| format!("parsing {}", path.display()))
}

fn read_trace(path: &Path) -> Result<Trace> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Trace::parse(&text).with_context(|| format!("parsing {}", path.display()))
}
