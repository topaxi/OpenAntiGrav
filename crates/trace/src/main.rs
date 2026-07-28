//! `oag-trace`: run our simulation over a recording and say where the two part
//! company.
//!
//! ```sh
//! # what is in a capture at all
//! oag-trace show data/traces/venom-straight.csv
//!
//! # replay it through our physics, on the track it was captured on
//! oag-trace run data/traces/venom-straight.csv \
//!     --source data/images/pulse-psp-usa.chd --hold cross --out /tmp/ours.csv
//!
//! # or diff two traces that already exist
//! oag-trace compare data/traces/venom-straight.csv /tmp/ours.csv
//! ```
//!
//! Traces are derived game data and live under `data/traces/`, which is
//! gitignored; nothing here can run in CI.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use oag_assets::pulse;
use oag_formats::{collision, handling};
use oag_gameplay::input::button_from_name;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::{CollisionWorld, Environment, Handling, SpeedClass};
use oag_trace::compare::Tolerances;
use oag_trace::replay::{Basis, DeltaSource, Held, Inputs, Options};
use oag_trace::trace::AngularReading;
use oag_trace::{Script, Trace, compare, replay};

/// The track a recording is assumed to have been taken on unless another is
/// named. The same default `oag-game` races on, and the directory the reference
/// scenario's Talon's Junction actually lives in - see `oag_game::race::DEFAULT_TRACK`
/// for the measurement that settled it against the previously assumed `01_Track`.
const DEFAULT_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// The team whose `handlingstats.xml` is read unless another is named. The
/// same default `oag-game` races on, and the team the reference scenario in
/// `docs/reverse-engineering/ppsspp-debugger.md` uses.
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
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
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
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            basis,
            angular,
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
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            basis,
            angular,
            no_collision,
            tolerances,
        }),
    }
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
    hold: Vec<String>,
    steer: Option<f32>,
    airbrake_left: Option<f32>,
    airbrake_right: Option<f32>,
    fixed_dt: bool,
    basis: BasisArg,
    angular: AngularArg,
    no_collision: bool,
    tolerances: ToleranceArgs,
}

fn run(args: RunArgs) -> Result<()> {
    let recorded = read_trace(&args.trace)?;
    if recorded.is_empty() {
        bail!("{}: no ticks in the trace", args.trace.display());
    }

    let class = SpeedClass::from_name(&args.class)
        .with_context(|| format!("{:?} is not a speed class", args.class))?;
    let (handling, collision) = match &args.source {
        Some(source) => load(source, &args.track, &args.team, class)?,
        None => {
            eprintln!(
                "note: no --source, so there are no handling parameters and no track. \
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
    let collision = if args.no_collision {
        eprintln!("note: --no-collision, so the hover probes see nothing");
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
    };
    if args.fixed_dt {
        eprintln!(
            "note: --fixed-dt steps at {FIXED_DT}s while the recording's own deltas vary, \
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
    );
    if let Some(path) = &args.out {
        std::fs::write(path, simulated.to_csv())
            .with_context(|| format!("writing {}", path.display()))?;
        eprintln!("wrote {}", path.display());
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
            eprintln!(
                "note: {} covers {} tick(s) and the recording has {ticks}, so the \
                 script's last state is held for the remaining {}. That is nearly \
                 always an over-short script rather than an intent.",
                path.display(),
                script.len(),
                ticks - script.len(),
            );
        }
        return Ok(Inputs::Scripted(script.states));
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
        let index = button_from_name(name)
            .with_context(|| format!("{name:?} is not a button name the front end knows"))?;
        buttons |= 1u32 << (index & 0x1f);
    }
    Ok(Inputs::Held(Held {
        buttons,
        stick_x: args.steer.unwrap_or(0.0),
        stick_y: 0.0,
        airbrake_left: args.airbrake_left.unwrap_or(0.0),
        airbrake_right: args.airbrake_right.unwrap_or(0.0),
    }))
}

/// Reads the handling parameters and the track's collision geometry off a disc.
///
/// The same two reads `oag_game::race::load` does, without the spline, the models
/// or the camera: a comparison run is seeded from the recording rather than from
/// a grid slot, so it needs nothing that decides where a ship starts.
///
/// **Which archives the source has, not which archives a PSP disc has.** Both
/// entry names below are spelled the same on both releases, so the layout is the
/// whole of the difference - [`pulse::Archives`] finds `Data.wad` or `WADS2.WAD`
/// by name and searches the companion archive too. This deliberately prints
/// nothing extra: `run` output is compared byte-for-byte against earlier
/// captures, so the layout appears only in an error's context.
fn load(
    source: &str,
    track: &str,
    team: &str,
    class: SpeedClass,
) -> Result<(Handling, CollisionWorld)> {
    let mut archives = pulse::Archives::open(source)?;
    let where_from = archives.layout.describe();

    let track_blob = archives
        .read_name(track)
        .with_context(|| format!("reading {track} out of {where_from}"))?;
    let nodes = collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{track}: {e}"))?;
    let collision = collision_world(&nodes);
    eprintln!(
        "{track}: {} collision node(s) -> {} collider(s)",
        nodes.len(),
        collision.colliders().len()
    );

    let stats_name = handling::entry_name(team);
    let stats_blob = archives
        .read_name(&stats_name)
        .with_context(|| format!("reading {stats_name} out of {where_from}"))?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    let handling = handling_for(&stats, class);
    eprintln!(
        "{stats_name}: team {:?}, {class:?} class, mass {}, ride_height {}",
        stats.team, handling.physical.mass, handling.antigrav.ride_height
    );

    Ok((handling, collision))
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
