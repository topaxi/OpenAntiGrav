//! Does the steering term turn the ship at the rate the original turns at?
//!
//! [`oag_physics::forces::YAW_INVERSE_INERTIA`] is the yaw entry of the body's
//! inverse inertia tensor, **read** out of `Body_SetBoxInertia` (`0x0884e1ac`)
//! and its single call site rather than fitted. It used to be a fitted stand-in
//! called `YAW_DRIVE_CALIBRATION`, and this test used to be what stopped that
//! number drifting; it now does something better - it checks a recovered
//! constant against captures it was **not** derived from.
//!
//! That distinction matters for how a failure here should be read. Under the old
//! name a failure meant "the fit has gone stale, refit it". Now it means "the
//! recovered value and the hardware disagree", which is a finding about the
//! reading, not a licence to adjust the constant.
//!
//! `#[ignore]`d and never run in CI: it needs `data/traces/` and a disc image
//! under `data/images/`, both gitignored. Run it with `just test-data`.
//!
//! This lives in `oag-trace` rather than next to the constant because reading the
//! shipped `Turning.amount` needs `oag-assets` and `oag-formats`, and `oag-physics`
//! depends on `oag-core` and nothing else, deliberately. Reading the value off the
//! disc at run time is also what keeps it out of the repository - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_core::math::Vec3;
use oag_formats::handling;
use oag_gameplay::handling_for;
use oag_physics::engine::steering;
use oag_physics::forces::YAW_INVERSE_INERTIA;
use oag_physics::passive::YAW_DAMPING;
use oag_physics::{Handling, ShipState};
use oag_pulse as pulse;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// Paths in `data/` are workspace-relative, but a test's working directory is its
/// own package root. Same resolution as `crates/assets/tests/pmf_ground_truth.rs`.
fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The team and class the captures in `data/traces/` were recorded with.
///
/// Not inferred from the filenames: the reference scenario is written down as
/// "Time Trial, Venom, Talon's Junction White, Assegai" in
/// `docs/reverse-engineering/ppsspp-debugger.md` and `docs/tools/oag-trace.md`,
/// both of which also give the `--team Assegai --class venom` command line. This
/// matters more than it looks - `Turning.amount` spans a factor of 1.38 across the
/// teams, so the wrong one here would silently move the constant these tests
/// validate.
const TEAM: &str = "Assegai";
const CLASS: oag_physics::SpeedClass = oag_physics::SpeedClass::Venom;

/// How far the replayed yaw rate may sit from the recorded one, in rad/s, as an
/// RMS over the window. The signal itself is near `1.5`, so this is a few percent
/// of it: loose enough not to fail on an unrelated change to an adjacent term,
/// tight enough that losing the tensor entirely (a 22x error) cannot pass.
///
/// Measured, on the day the recovered constant replaced the fitted one:
///
/// | | `steer-left` | `steer-right` |
/// | --- | ---: | ---: |
/// | fitted `0.0452` | `0.1208` | `0.1995` |
/// | recovered `0.046296` | `0.1430` | `0.1808` |
///
/// **The recovered value is not a regression**: it trades a little accuracy on
/// the left capture for more on the right, and its mean squared error across the
/// two is marginally the lower of the two. Recorded rather than smoothed over,
/// because "the read value happens to also fit better" would be a claim worth
/// distrusting, and it is not the claim - the two are within each other's noise,
/// which is exactly what agreement at this level should look like.
const MAX_RMS_ERROR: f32 = 0.25;

/// The window replayed. Both captures end up in a wall - the scenario notes in
/// `verification/scenarios/` record the same thing, 30 ticks of full lock put the
/// real ship into one - and a collision is not what the steering term models.
const WINDOW: usize = 110;

/// Above this, the recording is describing an impact rather than a turn.
const IMPACT_YAW: f32 = 3.0;

/// One step of the isolated yaw equation:
///
/// ```text
/// omega' = steering(steer) * YAW_INVERSE_INERTIA + YAW_DAMPING * omega
/// ```
///
/// Isolated deliberately. A full `oag_physics::step` would fold in hover, grip and
/// wall response, none of which this constant describes and all of which carry
/// their own open questions - the replay's known divergences would show up here
/// as a steering failure.
fn advance_yaw(omega: f32, steer: f32, handling: &Handling, dt: f32) -> f32 {
    let state = ShipState {
        steer,
        ..ShipState::default()
    };

    let drive = steering(&state, handling) * YAW_INVERSE_INERTIA;

    omega + (drive + YAW_DAMPING * omega) * dt
}

fn shipped_handling() -> Handling {
    // Resolved by name, the way `oag-trace run` and `oag-game` do, so this reads
    // the same file off a PS2 pressing without knowing where the PS2 puts it.
    let mut archives = pulse::open(&workspace(IMAGE).to_string_lossy()).expect("open the source");

    let name = handling::entry_name(TEAM);
    let blob = archives.read_name(&name).expect("read handlingstats.xml");
    let stats = handling::from_blob(&blob).expect("parse handlingstats.xml");

    // Zero: neither of these scenarios crosses a speed pad, so the boost must
    // contribute nothing and a non-zero magnitude could only mask that.
    handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
}

/// The yaw rate the recording actually shows, differentiated out of the captured
/// basis exactly the way `docs/ghidra/functions/psp-pulse-usa/engine.md` measured it
/// on hardware: `dot(cross(fwd_t, fwd_t+1), up) / dt`.
fn recorded_yaw(capture: &str) -> Vec<(f32, f32, f32)> {
    let path = workspace(&format!("data/traces/talons-junction-{capture}.csv"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let trace = oag_trace::Trace::parse(&text).expect("parse trace");

    trace
        .frames
        .windows(2)
        .take(WINDOW)
        .filter_map(|pair| {
            let (previous, current) = (&pair[0], &pair[1]);
            let yaw = Vec3::cross(previous.forward, current.forward).dot(current.up) / current.dt;

            (yaw.abs() < IMPACT_YAW).then_some((current.steer, yaw, current.dt))
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/ and captures in data/traces/"]
fn the_recovered_yaw_inertia_reproduces_the_recorded_yaw_rate() {
    let handling = shipped_handling();

    for capture in ["steer-left", "steer-right"] {
        let samples = recorded_yaw(capture);
        assert!(
            samples.len() > 100,
            "{capture}: only {} usable sample(s); the capture is not what this \
             test assumes",
            samples.len()
        );

        // Seeded from the recording's own first sample: angular velocity is not a
        // captured column, so there is no other honest initial condition.
        let mut omega = samples[0].1;
        let mut squared_error = 0.0;
        for &(steer, measured, dt) in &samples {
            let error = omega - measured;
            squared_error += error * error;
            omega = advance_yaw(omega, steer, &handling, dt);
        }
        let rmse = (squared_error / samples.len() as f32).sqrt();

        assert!(
            rmse < MAX_RMS_ERROR,
            "{capture}: RMS yaw error {rmse} rad/s against a signal near 1.5 \
             (limit {MAX_RMS_ERROR}). The recovered YAW_INVERSE_INERTIA no \
             longer reproduces this capture - a disagreement between the \
             reading and the hardware, not a constant to retune."
        );
    }
}

/// The sign, which is the other half of "steers correctly" and is what a
/// reimplementation mapping the original's row 0 onto its own `+x` gets wrong.
#[test]
#[ignore = "needs a disc image in data/images/ and captures in data/traces/"]
fn holding_left_yaws_positive_and_holding_right_yaws_negative() {
    for (capture, expected) in [("steer-left", 1.0_f32), ("steer-right", -1.0)] {
        let samples = recorded_yaw(capture);
        let mean: f32 = samples.iter().map(|s| s.1).sum::<f32>() / samples.len() as f32;

        assert_eq!(
            mean.signum(),
            expected,
            "{capture}: recorded mean yaw {mean} rad/s has the wrong sign"
        );

        let handling = shipped_handling();
        let mean_steer: f32 = samples.iter().map(|s| s.0).sum::<f32>() / samples.len() as f32;
        let state = ShipState {
            steer: mean_steer,
            ..ShipState::default()
        };
        assert_eq!(
            steering(&state, &handling).signum(),
            expected,
            "{capture}: our steering term disagrees in sign with the capture"
        );
    }
}
