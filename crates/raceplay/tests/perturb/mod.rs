//! A test-only perturbation for judging our AI on a distribution, not on one
//! chaotic run.
//!
//! This engine's AI gates flip on noise: whether a re-learned hull plan
//! verifies (`oag_ai::SpeedPlan::adapt`) turns on trajectory differences far
//! below anything a law changes, so a single bit-exact run of a gate row is a
//! coin (`docs/physics/airborne-ground-truth.md`, "Our AI against the air
//! laws"). A gate built on [`SCALES`] runs the same scenario once per scale and
//! asserts a median.
//!
//! **The perturbation is data, not code.** [`apply`] scales every craft's
//! `Physical::flight_gravity` (the airborne-only gravity term,
//! `oag_physics::passive::gravity`) by `1 + eps` on the loaded `Setup`, before
//! `Race::start`. No simulation crate changes, so the determinism scan and the
//! committed hashes never see it, and `eps = 0` is the unperturbed race
//! bit for bit. It reaches the race and every speed plan the race learns,
//! because both read the craft's `Handling`. The player's and every opponent's
//! file get the same factor, so the set of distinct hulls the plan selection
//! compares (`tried.contains`, `== reference`) is unchanged.
//!
//! Rejected: the race seed (`Options::seed`). It re-draws the roster
//! (`oag_race::roster::teams_for_slots`), so the lone craft flies a different
//! team, and no speed-plan build reads the generator at all. A pitch torque in
//! the force law (the sweep the doc above used) needs a switch in
//! `oag_physics`. **Chosen, not measured.**
//!
//! `tests/<dir>/mod.rs` is not a test target of its own, which is why this is
//! a directory.

#![allow(dead_code)]

use oag_raceplay::Setup;

/// The perturbation set: `0` (the unperturbed run) and four magnitudes either
/// side. `1e-6` is about eight `f32` ulps of `1.0`. Nine, so a median ignores
/// four outliers.
pub const SCALES: [f32; 9] = [0.0, 1e-3, -1e-3, 1e-4, -1e-4, 1e-5, -1e-5, 1e-6, -1e-6];

/// The subset a gate runs: the unperturbed race and one scale per magnitude,
/// signs alternating. Five, not [`SCALES`]' nine, for the suite's time ceiling
/// (`scripts/check-test-budget.py`): each scale is a whole race, and the two
/// gates built on this cost about 170 s of test time per scale. A median of
/// five ignores two outliers. **Chosen, not measured.**
pub const GATE: [f32; 5] = [0.0, 1e-3, -1e-4, 1e-5, -1e-6];

/// The offset named by `OAG_PERTURB`, else `0`: a scratch switch that moves
/// every run of every test calling [`apply`] by one more factor, so a sweep
/// can ask how often a gate itself flips (`OAG_PERTURB=1e-5 cargo nextest
/// run ...`). Never set in `just test-data`.
pub fn offset() -> f32 {
    std::env::var("OAG_PERTURB")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(0.0)
}

/// Scales every craft's airborne gravity on `setup` by `1 + eps` (and by
/// `1 + offset()`). A no-op at `eps = 0` with no offset; a single-run test
/// calls it with `0` so the sweep switch reaches it too.
pub fn apply(setup: &mut Setup, eps: f32) {
    let factor = (1.0 + eps) * (1.0 + offset());
    if factor == 1.0 {
        return;
    }
    setup.handling.physical.flight_gravity *= factor;
    for handling in setup.opponent_handling.iter_mut().flatten() {
        handling.physical.flight_gravity *= factor;
    }
}

/// The median of `values` (the upper one of an even count). `values` must not
/// be empty.
pub fn median<T: Ord + Copy>(values: &[T]) -> T {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}
