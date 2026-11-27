//! How a headless race runs: forced events, the seed, telemetry and the pickup.

/// How a headless race runs: forced events, the seed, telemetry and the pickup.
#[derive(clap::Args, Debug)]
pub(crate) struct RaceRunArgs {
    /// With `--race --screenshot`, arm the camera's impact shake at the end of
    /// a tick as a wall hit of that severity would: `TICK:SEVERITY`, severity
    /// `0..=1`, e.g. `120:1.0` and then `--ticks 121` for the first frame of a
    /// hard hit.
    ///
    /// A verification aid, there because a hard hit is hard to produce on
    /// demand and the first shaken frame is where the shake meets the motion
    /// blur and the temporal upscaler. See [`Race::force_shake`].
    #[arg(long, value_name = "TICK:SEVERITY", requires = "race")]
    pub(crate) force_shake: Option<String>,

    /// With `--race --screenshot`, open the boost's field-of-view kick at the
    /// end of a tick with no boost behind it: `TICK:OPEN`, `OPEN` in `0..=1`
    /// (fully open at 1), e.g. `120:0.15` and then `--ticks 121` for the frame
    /// whose field has just widened. The craft is not boosted, the kick closes
    /// again at its usual rate, and under `boost_fov_kick = 0` it widens
    /// nothing.
    ///
    /// A verification aid: it changes the lens alone on a still craft, which is
    /// how the motion blur is shown to leave a widening field unsmeared. See
    /// [`Race::force_boost_kick`].
    #[arg(long, value_name = "TICK:OPEN", requires = "race")]
    pub(crate) force_boost_kick: Option<String>,

    /// Seed the world generator, instead of `race::SEED` (`--race`, headless) or a fresh
    /// seed per race (the front end).
    ///
    /// A verification aid - see `race::Options::seed`. The one thing in a race
    /// that draws from the generator today is which pickup a weapon pad hands
    /// over, so this is how a capture is made to show a chosen weapon.
    #[cfg(feature = "dev-cli")]
    #[arg(long)]
    pub(crate) seed: Option<u64>,

    /// In a race, print a telemetry line every this many ticks. Zero prints
    /// none.
    #[arg(long, default_value_t = 60)]
    pub(crate) log_every: u32,

    /// Keep handing the player this weapon whenever their pickup slot is empty - and, with
    /// `--players`, every other viewed slot, so each view has one to show.
    ///
    /// **A debug affordance, and worth saying why it exists.** A weapon can only
    /// be *seen* once something fires it, and `--race` holds the throttle
    /// without steering, so it never crosses a `Weapon Pad` and never receives a
    /// pickup. That made every visual change to a projectile unverifiable except
    /// by playing the game by hand - and two rocket changes shipped blind before
    /// this existed, one of them wrong (three pieces of track scenery read as a
    /// volley; see `oag_raceplay::Race::rocket_model_matrices`).
    ///
    /// Spelled as the `type` attribute the weapon table uses - `Rocket`,
    /// `Missile`, `Cannon` - and matched case-insensitively.
    ///
    /// It writes the player's pickup slot from **outside** the simulation, in
    /// the frame loop rather than in `Race::tick`, so nothing here can reach a
    /// determinism hash. Combine with `--press square` to fire repeatedly:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --give rocket \
    ///     --hold cross --press square --ticks 900 --screenshot /tmp/shot.png
    /// ```
    #[cfg(feature = "dev-cli")]
    #[arg(long, value_name = "WEAPON")]
    pub(crate) give: Option<String>,
    /// With `--give`, hand the weapon over at tick 0 only instead of topping the
    /// slot up whenever it is empty, so a screenshot after the press shows the
    /// pickup spent. A `--screenshot` race capture only.
    #[cfg(feature = "dev-cli")]
    #[arg(long, requires = "give")]
    pub(crate) give_once: bool,
}
