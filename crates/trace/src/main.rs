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
use oag_assets::{Archive, pulse};
use oag_formats::{collision, handling};
use oag_gameplay::input::button_from_name;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::{CollisionWorld, Environment, Handling, SpeedClass};
use oag_trace::compare::Tolerances;
use oag_trace::replay::{Basis, DeltaSource, Held, Inputs, Options};
use oag_trace::{Trace, compare, replay};

/// The track a recording is assumed to have been taken on unless another is
/// named. The same default `oag-game` races on.
const DEFAULT_TRACK: &str = r"Data\Environments\01_Track\track.vex";

/// The team whose `handlingstats.xml` is read unless another is named.
const DEFAULT_TEAM: &str = "Feisar";

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
        /// Ignore the track's collision geometry: a ship with nothing to hover on.
        #[arg(long)]
        no_collision: bool,
        #[command(flatten)]
        tolerances: ToleranceArgs,
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
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Show { trace } => show(&trace),
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
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            basis,
            no_collision,
            tolerances,
        } => run(RunArgs {
            trace,
            source,
            track,
            team,
            class,
            out,
            hold,
            steer,
            airbrake_left,
            airbrake_right,
            fixed_dt,
            basis,
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
    hold: Vec<String>,
    steer: Option<f32>,
    airbrake_left: Option<f32>,
    airbrake_right: Option<f32>,
    fixed_dt: bool,
    basis: BasisArg,
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
        inputs: inputs(&args)?,
        dt: if args.fixed_dt {
            DeltaSource::Fixed(FIXED_DT)
        } else {
            DeltaSource::Trace
        },
        basis: args.basis.into(),
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

/// How the replay is driven: the recording's own control states, or a fixed
/// input the capture was taken with.
fn inputs(args: &RunArgs) -> Result<Inputs> {
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
fn load(
    source: &str,
    track: &str,
    team: &str,
    class: SpeedClass,
) -> Result<(Handling, CollisionWorld)> {
    let spec = pulse::archive_spec(source, pulse::archives::DATA);
    let mut archive = Archive::open(&spec)?;

    let track_blob = archive
        .read_name(track)
        .with_context(|| format!("reading {track} out of {spec}"))?;
    let nodes = collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{track}: {e}"))?;
    let collision = collision_world(&nodes);
    eprintln!(
        "{track}: {} collision node(s) -> {} collider(s)",
        nodes.len(),
        collision.colliders().len()
    );

    let stats_name = handling::entry_name(team);
    let stats_blob = archive
        .read_name(&stats_name)
        .with_context(|| format!("reading {stats_name} out of {spec}"))?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    let handling = handling_for(&stats, class);
    eprintln!(
        "{stats_name}: team {:?}, {class:?} class, mass {}, ride_height {}",
        stats.team, handling.physical.mass, handling.antigrav.ride_height
    );

    Ok((handling, collision))
}

fn read_trace(path: &Path) -> Result<Trace> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Trace::parse(&text).with_context(|| format!("parsing {}", path.display()))
}
