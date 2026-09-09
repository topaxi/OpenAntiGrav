//! Measures the magstrip hold against the **inverted section of a recorded lap**.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image *and* a capture,
//! neither of which this project ships. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(maglock_ground_truth)'
//! ```
//!
//! # What is being measured, and why it is not a replay
//!
//! `docs/physics/cornering-ground-truth.md` measures, over a whole lap of Talon's
//! Junction, that the identity `body+0x150 = -omega(basis)` holds to `8 %` on
//! ordinary track and **collapses inside the track's inverted section**, where it
//! reads `0.023` on pitch and `0.110` on roll: nearly all of the rotation there
//! happens to the basis without passing through the angular-velocity column. That
//! residual is what `oag_physics::maglock` claims to be.
//!
//! The claim cannot be checked by replaying the lap. The `omega_*` capture's input
//! script was deliberately not committed (see `docs/tools/oag-trace.md`), and the
//! committed lap's replay leaves the recording by tick 150 anyway, so by the time
//! either reaches tick 1,072 nothing is comparable. What *is* available is
//! per-tick: at every recorded pose, the recording says how much the basis turned
//! and how much of that the momentum column accounts for, and the hold can be run
//! once from that same pose and asked how much it produces. That is a one-tick
//! seeded measurement rather than a trajectory comparison, and it is the strongest
//! instrument the available data supports.
//!
//! Everything here is kinematic: no handling value enters the comparison, only the
//! recorded basis, the recorded angular velocity and the track's own geometry.
//!
//! # What it measured, on the USA PSP disc and the `omega_*` lap
//!
//! ```text
//! inverted samples 283, mag contacts 258, hold engaged 283, ray fallback 0
//! mean angle between our axis and the recorded up 2.07 deg
//! residual rms 1.2542 rad/s, hold rms 2.3048 rad/s
//! fitted scale 0.383, explained 49.5 %
//! control: 5 of 1959 upright poses have a strip under them
//! ```
//!
//! Four things worth reading off that, in decreasing order of how much they
//! settle:
//!
//! - **The stretch is a magstrip, measured rather than assumed.** `258` of `283`
//!   inverted poses have `Mag Floor` geometry under them (`91 %`) against `5` of
//!   `1,959` upright ones (`0.3 %`). The lap found the identity breaking in the
//!   inverted section; the probe finds a strip in the inverted section; neither
//!   measurement knew about the other.
//! - **The residual reproduces.** `1.25` rad/s rms here against the `1.186` rad/s
//!   `cornering-ground-truth.md` reports, from a different tool over a different
//!   partition of the same capture.
//! - **The hold explains half of it with nothing fitted**: `49.5 %` of the
//!   residual's energy, i.e. the two rotations agree in direction to `cos = 0.70`.
//!   The crate had **no** mechanism that could rotate a basis outside the
//!   integrator before this, so the figure to compare against is zero.
//! - **What is left over is a magnitude, and it points at the locator rather than
//!   the hold.** The fitted scale is `0.383`: a full-blend hold snaps the ship
//!   onto the axis in one tick, and the original demonstrably does not sit exactly
//!   on it - `2.07` degrees off, in line with the `1.6` degree median
//!   `cornering-ground-truth.md` measures against the track's own normal. So
//!   either the blend on that stretch is not `1.0`, or this crate's axis is not
//!   quite the original's: the spline is resampled at four points per control-point
//!   interval and the neighbour is taken in table order, where the original
//!   evaluates the curve and follows the junction graph. Both are locator
//!   questions. **Nothing here is tuned to close that gap** - a fitted blend would
//!   destroy the measurement it is fitted to.

use std::path::{Path, PathBuf};

use oag_core::math::{Mat3, Quat, Vec3};
use oag_game::race;
use oag_physics::{Body, Environment, ShipState, maglock};

/// Where the identity is broken, from the capture's own `up.y` column.
///
/// Not the tick range `cornering-ground-truth.md` quotes (`1,072`-`1,329`): the
/// partition there is by `up.y < 0.85` and it is reproduced here the same way, so
/// this file does not depend on the two captures numbering their ticks alike.
const INVERTED_UP_Y: f32 = 0.85;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One recorded tick, in the columns this test reads.
struct Row {
    dt: f32,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    position: Vec3,
    velocity: Vec3,
    /// `body+0x150`, body-local, as recorded.
    omega: Vec3,
}

impl Row {
    /// The recorded basis as this crate's orientation.
    ///
    /// **Row 0 of the recording is the ship's *left*.** The capture names it
    /// `right_*` and it is not: the recorded basis is positively oriented while
    /// this crate's is not (`right x up = -forward`, forward being `-Z`), so the
    /// mapping takes an odd number of sign flips and the measured one is row 0.
    /// That is `oag_trace::replay::Basis::LeftUpForward`, restated here rather
    /// than depended on, and it is worth the restatement: with the columns taken
    /// at their names the basis is a reflection, `Quat::from_mat3` returns
    /// nonsense, and this test reads a residual of 32 rad/s where the capture has
    /// 1.2.
    fn orientation(&self) -> Quat {
        let z = -self.forward.normalize_or_zero();
        let x = {
            let right = -self.right.normalize_or_zero();
            (right - z * right.dot(z)).normalize_or_zero()
        };
        let y = z.cross(x);
        assert!(
            (y - self.up.normalize_or_zero()).length() < 1e-2,
            "the recorded basis is not the one this reading describes"
        );
        Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize()
    }
}

/// The capture this file reads, and the one reference trace in the workspace
/// that **no checkout has**.
const CAPTURE: &str = "talons-junction-time-trial-lap-omega.csv";

/// Reads the capture, or `None` - and this is the one place in the workspace
/// where `None` is still the honest answer.
///
/// # Why this is not `require_capture`
///
/// The sibling ground-truth tests were changed - `5e940717` for `oag-trace`'s
/// two, `chase_camera_ground_truth.rs` for this crate's other one - so that a
/// missing reference **panics unconditionally**: those files are in git's index,
/// so absence means a broken working tree.
///
/// This one is different, and the difference is not a matter of taste.
/// [ADR-0046](../../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
/// puts `CAPTURE` on `.gitignore`'s re-inclusion list, but **it was never
/// committed**, because it was already gone when the ADR was written - its own
/// table lists it `missing`, and its Consequences section says so outright:
/// "the five missing captures are not recovered by this." `git ls-files
/// data/traces/` confirms it: four names, not six. So there is nothing for a
/// panic to point at. A hard failure here would make `just test-data` red
/// forever on a file that no checkout, no branch and no reflog contains, and a
/// permanently red suite is a suite nobody reads.
///
/// **It is also not gated on `OAG_REQUIRE_GAME_DATA`**, which it used to be.
/// That variable means "the optional inputs I do have must be found" - it
/// escalates a *disc image*, which a contributor can supply. This capture
/// cannot be supplied; it has to be re-recorded to the recipe in
/// `docs/reverse-engineering/ppsspp-debugger.md`. Escalating it made
/// `OAG_REQUIRE_GAME_DATA=1 just test-data` fail on the one thing in the run
/// that no one could act on, which buries the failures that can be acted on.
///
/// What keeps the gap from going quiet again is [`the_capture_is_still_on_the_allowlist`]:
/// the moment someone decides this trace is not a reference after all and drops
/// it from `.gitignore`, that test fails and this comment has to be rewritten.
/// And the moment the file *arrives*, this returns `Some` and the measurement
/// below runs - there is no path where a present capture is skipped.
fn rows() -> Option<Vec<Row>> {
    let path = repo().join("data/traces").join(CAPTURE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            println!(
                "skipping: {} is absent from every known checkout - ADR-0046 allowlists \
                 it but it was never committed, so it comes back only by re-recording \
                 to docs/reverse-engineering/ppsspp-debugger.md. This is the one \
                 reference trace whose absence is not a broken checkout.",
                path.display()
            );
            return None;
        }
    };

    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let at = |name: &str| -> usize {
        header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("the capture has no {name} column"))
    };
    let (dt, omega_x) = (at("dt"), at("omega_x"));
    let (right_x, up_x, fwd_x) = (at("right_x"), at("up_x"), at("fwd_x"));
    let (pos_x, vel_x) = (at("pos_x"), at("vel_x"));

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<f32> = line
            .split(',')
            .map(|field| field.parse::<f32>().unwrap_or(f32::NAN))
            .collect();
        let vec3 = |base: usize| Vec3::new(fields[base], fields[base + 1], fields[base + 2]);
        rows.push(Row {
            dt: fields[dt],
            right: vec3(right_x),
            up: vec3(up_x),
            forward: vec3(fwd_x),
            position: vec3(pos_x),
            velocity: vec3(vel_x),
            omega: vec3(omega_x),
        });
    }
    Some(rows)
}

/// The body-frame rotation rate that takes `from` to `to` in `dt`.
fn rate(from: Quat, to: Quat, dt: f32) -> Vec3 {
    let delta = from.inverse() * to;
    // The shorter of the two ways round, so a basis that barely moved does not
    // read as a nearly-full turn the other way.
    let delta = if delta.w < 0.0 { -delta } else { delta };
    let (axis, angle) = delta.to_axis_angle();
    axis * (angle / dt)
}

/// Does the hold reproduce the rotation the momentum column cannot account for?
///
/// Run at every recorded pose inside the inverted stretch:
///
/// - `basis` is the rotation rate the recording's own basis shows between this
///   tick and the next, in the body frame.
/// - `residual = basis + omega`, because the recorded column is `-omega(basis)`;
///   on ordinary track this is near zero and inside the stretch it is almost the
///   whole of `basis`.
/// - `hold` is what `oag_physics::maglock` does to that same pose in one tick,
///   with the track's own spline supplying the axis and the track's own mag-floor
///   geometry supplying the contact.
///
/// The comparison is a one-parameter least-squares fit of `hold` onto `residual`,
/// reported as a scale and a fraction explained - the same instrument
/// `cornering-ground-truth.md` uses for every other claim it makes.
#[test]
#[ignore = "needs data/images and data/traces"]
fn the_hold_reproduces_the_rotation_the_momentum_column_does_not() {
    let Some(image) = image() else { return };
    let Some(rows) = rows() else { return };

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        ..race::Options::default()
    })
    .expect("the default track loads");
    let setup = loaded.setup;

    let mut residual_sum = 0.0f64;
    let mut hold_sum = 0.0f64;
    let mut cross_sum = 0.0f64;
    let mut engaged = 0usize;
    let mut from_ray = 0usize;
    let mut axis_angle = 0.0f64;
    let mut considered = 0usize;
    let mut contacts = 0usize;

    for pair in rows.windows(2) {
        let (row, next) = (&pair[0], &pair[1]);
        if row.up.y >= INVERTED_UP_Y || row.dt <= 0.0 {
            continue;
        }
        considered += 1;

        let basis_rate = rate(row.orientation(), next.orientation(), row.dt);
        let residual = basis_rate + row.omega;

        let mut state = ShipState {
            body: Body {
                position: row.position,
                orientation: row.orientation(),
                linear_velocity: row.velocity,
                mass: setup.handling.physical.mass,
                ..Body::default()
            },
            // The stretch is flown fully locked: five frames at `0.2` is a twelfth
            // of a second, and the ship has been inverted for hundreds of ticks by
            // the time any of these samples is taken.
            mag_lock_blend: 1.0,
            ..ShipState::default()
        };

        let nearest = setup.spline.nearest(row.position);
        let env = Environment {
            track_sample: nearest.map(|(_, sample, _)| race::Spline::track_sample(sample)),
            track_sample_next: nearest
                .and_then(|(index, _, _)| setup.spline.sample(index + 1))
                .map(race::Spline::track_sample),
            ..Environment::default()
        };

        let contact = maglock::probe(&state, &env, &setup.collision);
        if contact.is_some() {
            contacts += 1;
        }

        let before = state.body.orientation;
        let target = oag_physics::hover::target_height(&setup.handling, 1.0, 0.0);
        let Some(hold) = maglock::update(&mut state, &env, contact, target) else {
            continue;
        };
        engaged += 1;
        if hold.from_ray {
            from_ray += 1;
        }
        axis_angle += f64::from(
            hold.axis
                .normalize_or_zero()
                .dot(row.up.normalize_or_zero())
                .clamp(-1.0, 1.0)
                .acos(),
        );

        let hold_rate = rate(before, state.body.orientation, row.dt);

        residual_sum += f64::from(residual.length_squared());
        hold_sum += f64::from(hold_rate.length_squared());
        cross_sum += f64::from(residual.dot(hold_rate));
    }

    // The control, and the half of the claim that is about *localisation*: an
    // ordinary stretch of the same lap should have no strip under it, so the probe
    // should find nothing and the blend should never leave zero. The lap measured
    // the same thing from the opposite end - the identity holds everywhere else -
    // and the two arrive at the inverted section independently.
    let mut upright = 0usize;
    let mut upright_contacts = 0usize;
    for row in &rows {
        if row.up.y < 0.99 || row.dt <= 0.0 {
            continue;
        }
        upright += 1;
        let state = ShipState {
            body: Body {
                position: row.position,
                orientation: row.orientation(),
                ..Body::default()
            },
            ..ShipState::default()
        };
        let nearest = setup.spline.nearest(row.position);
        let env = Environment {
            track_sample: nearest.map(|(_, sample, _)| race::Spline::track_sample(sample)),
            ..Environment::default()
        };
        if maglock::probe(&state, &env, &setup.collision).is_some() {
            upright_contacts += 1;
        }
    }

    let scale = if hold_sum > 0.0 {
        cross_sum / hold_sum
    } else {
        0.0
    };
    // The fraction of the residual's energy the fitted hold accounts for.
    let explained = if residual_sum > 0.0 {
        (cross_sum * cross_sum) / (hold_sum * residual_sum)
    } else {
        0.0
    };

    println!(
        "inverted samples {considered}, mag contacts {contacts}, hold engaged {engaged}, \
         ray fallback {from_ray}\n\
         mean angle between our axis and the recorded up {:.2} deg\n\
         residual rms {:.4} rad/s, hold rms {:.4} rad/s\n\
         fitted scale {scale:.3}, explained {:.1} %\n\
         control: {upright_contacts} of {upright} upright poses have a strip under them",
        (axis_angle / engaged.max(1) as f64).to_degrees(),
        (residual_sum / considered.max(1) as f64).sqrt(),
        (hold_sum / considered.max(1) as f64).sqrt(),
        100.0 * explained,
    );

    assert!(
        considered > 100,
        "the capture has {considered} inverted samples, which is not a stretch"
    );
    assert!(
        contacts * 2 > considered,
        "the mag probe found a strip under only {contacts} of {considered} inverted \
         poses; the hold cannot be what holds the ship there if the strip is not \
         under it"
    );
    assert!(
        upright_contacts * 20 < upright,
        "the mag probe fires on {upright_contacts} of {upright} upright poses, so it \
         is not localised to the inverted section at all"
    );
    assert!(
        explained > 0.25,
        "the hold explains {:.1} % of the rotation the momentum column does not, \
         which is too little to claim it is the mechanism",
        100.0 * explained
    );
}

/// The allowlist in `.gitignore` is the only register of what counts as a
/// reference trace - ADR-0046 says so in as many words: "the list is the
/// allowlist in `.gitignore` and nothing else. There is no second register to
/// drift out of sync with it."
///
/// So this is not a second register either; it reads the same one. It exists
/// because [`rows`] skips rather than panics, and a skip is exactly how the
/// five missing captures went unnoticed for weeks. If someone concludes this
/// trace is not worth re-recording and drops its name, that decision has to
/// come with rewriting `rows`' comment and the docs that cite the capture
/// (`docs/physics/cornering-ground-truth.md`,
/// `docs/physics/angular-velocity-column.md`) - and this is what makes it.
///
/// Not `#[ignore]`d: it reads one tracked file in the repository and needs no
/// disc, no capture and no GPU, so it runs in `just test` where it is useful.
#[test]
fn the_capture_is_still_on_the_allowlist() {
    let gitignore = std::fs::read_to_string(repo().join(".gitignore")).expect(".gitignore");
    let line = format!("!/data/traces/{CAPTURE}");
    assert!(
        gitignore.lines().any(|l| l.trim() == line),
        "{CAPTURE} is no longer re-included in .gitignore, so it is no longer a \
         reference trace by ADR-0046's own definition - but maglock_ground_truth \
         still names it and skips when it is absent. Settle one or the other."
    );
}
