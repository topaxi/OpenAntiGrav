//! Whether a campaign cell's own difficulty rung actually changes how fast
//! the AI drives - the check `docs/gameplay/ai.md`'s campaign section and
//! `Session::launch_campaign_cell`'s own doc both point at: before this,
//! `race_options.difficulty` never moved for a campaign launch, so a Hard
//! medal was won against whatever the RACE page's own ambient difficulty
//! happened to be.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # Method
//!
//! `grid_00.xml`'s own `grid0_2_1` (`16_Track`, `Race`, `Venom`,
//! `skillEasy="1.1"` `skill="1.75"` `skillHard="2.5"` -
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`) is read straight
//! off `Data.wad`, and `16_Track`'s own `stats.xml` straight off
//! `FEData.wad` - the same two reads
//! `Session::launch_campaign_cell`/`resolve_campaign_ai_skill_scale` makes
//! at launch, just made directly here instead of through the composition
//! root. `oag_tables::track_stats::resolve_skill_scale` turns the pair into
//! two scales, `oag_ai::Difficulty::tune_at_scale` turns each scale into a
//! `Tuning`, and each `Tuning` drives **one AI craft alone on the circuit**
//! (`ai_span_sweep.rs`'s own `solo_on` shape: no rivals, so nothing but the
//! tuning can move the lap time) for the same seed, the same track, the
//! same everything else.
//!
//! A solo lap rather than a fielded race with opponents **on purpose**:
//! this project's own `RubberBanding`/`PosBalancing` refusal
//! (`docs/gameplay/ai.md#what-we-build-instead`) means nothing here reads
//! another craft's position, so a fielded race would only add weapon-pickup
//! and traffic noise to the same signal a lone lap already isolates.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;

fn image() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-usa.chd")
}

/// `grid0_2_1` off `Data.wad`'s own `grid_00.xml` - the exact cell
/// `race-campaign.md`'s own worked example quotes.
fn talons_junction_cell() -> Option<oag_tables::race_campaign::Cell> {
    let image = image()?;
    let spec = format!("{}:{}", image.display(), oag_pulse::archives::DATA);
    let mut archive = oag_assets::Archive::open(&spec).expect("opening Data.wad");
    let blob = archive
        .read_name(&oag_pulse::campaign::entry_name(0))
        .expect("grid_00.xml");
    let grid = oag_tables::race_campaign::from_blob(&blob).expect("parsing grid_00.xml");
    let cell = grid
        .cells
        .into_iter()
        .find(|c| c.name == "grid0_2_1")
        .expect("grid_00.xml carries grid0_2_1");
    assert_eq!(cell.track.as_deref(), Some("16_Track"));
    Some(cell)
}

/// `16_Track`'s own `stats.xml` off `FEData.wad`.
fn talons_junction_stats() -> Option<oag_tables::track_stats::TrackStats> {
    let image = image()?;
    let spec = format!("{}:{}", image.display(), oag_pulse::archives::FEDATA);
    let mut archive = oag_assets::Archive::open(&spec).expect("opening FEData.wad");
    let blob = archive
        .read_name(r"Data\Environments\16_Track\stats.xml")
        .expect("16_Track's stats.xml");
    Some(oag_tables::track_stats::from_blob(&blob).expect("parsing stats.xml"))
}

/// One AI-driven lap of `16_Track` at `tuning`, every other slot switched
/// off - `ai_span_sweep.rs`'s own `solo_on` shape, copied rather than
/// shared across a test-only crate boundary neither file has.
fn solo_lap_at(tuning: oag_ai::Tuning) -> Option<u64> {
    let image = image().expect("disc image present");
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        // An archive entry name, not the catalogue id - see
        // `oag_raceplay::catalogue::Track::entry_name`'s own doc; `ai_span_sweep.rs`
        // records the same trap. `16_Track`'s own directory, confirmed
        // directly (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`
        // cites the identical path for its own `stats.xml` read).
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 16_Track");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_ai_tuning(tuning);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    const LONE: usize = 1;
    // `ai_span_sweep.rs`'s own budget - a Novice-scaled driver on a
    // technical circuit can take several times a measured lap to complete
    // one clean.
    const TICKS: u64 = 18_000;
    let mut lap = race.sim.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut best: Option<u64> = None;
    for tick in 0..TICKS {
        race.tick(&PlayerInputs::none());
        let now = race.sim.world.ships[LONE].standing.lap;
        if now != lap {
            if lap > 1 {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
        }
    }
    best
}

/// **The headline claim**: a cell raced at Hard drives its AI faster than
/// the identical cell raced at Easy.
///
/// Not an exact-time pin - `CLAUDE.md`'s own rule against asserting a speed
/// as though it were measured applies here exactly as it does in
/// `race_ground_truth.rs` - only the ordering [`oag_ai::Difficulty::tune_at_scale`]
/// itself promises: Hard's scale sits strictly above Easy's on
/// `AI_ResolveSkillScale`'s own curve, and `tune_at_scale` is monotone in
/// its axes by construction (`every_axis_rises_with_the_level` in
/// `crates/ai/src/difficulty/tests.rs`).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cell_raced_at_hard_drives_its_ai_faster_than_at_easy() {
    let (Some(cell), Some(stats)) = (talons_junction_cell(), talons_junction_stats()) else {
        return;
    };

    let easy_scale = oag_tables::track_stats::resolve_skill_scale(
        &cell,
        oag_tables::race_campaign::Difficulty::Easy,
        Some(&stats),
    )
    .expect("grid0_2_1 carries a skillEasy");
    let hard_scale = oag_tables::track_stats::resolve_skill_scale(
        &cell,
        oag_tables::race_campaign::Difficulty::Hard,
        Some(&stats),
    )
    .expect("grid0_2_1 carries a skillHard");
    assert!(
        hard_scale > easy_scale,
        "Hard ({hard_scale}) should resolve above Easy ({easy_scale}) on 16_Track's own curve"
    );

    let easy_tuning = oag_ai::Difficulty::tune_at_scale(easy_scale, &oag_ai::Tuning::default());
    let hard_tuning = oag_ai::Difficulty::tune_at_scale(hard_scale, &oag_ai::Tuning::default());

    let easy_lap = solo_lap_at(easy_tuning);
    let hard_lap = solo_lap_at(hard_tuning);

    let describe = |lap: Option<u64>| {
        lap.map_or("did not clean a lap in budget".to_string(), |ticks| {
            format!("{:.1}s", ticks as f32 / 60.0)
        })
    };
    println!(
        "16_Track/grid0_2_1: Easy (scale {easy_scale:.3}) best lap {}, \
         Hard (scale {hard_scale:.3}) best lap {}",
        describe(easy_lap),
        describe(hard_lap),
    );

    // `None` (never completed a clean lap in budget) is the slowest outcome
    // there is, so it sorts as "no faster than any completed time" here -
    // `Some(x) <= None` is false under the derived `Option` order, which is
    // backwards for this comparison, hence the explicit match rather than a
    // bare `<=`.
    let hard_no_slower_than_easy = match (easy_lap, hard_lap) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(easy), Some(hard)) => hard <= easy,
    };
    assert!(
        hard_no_slower_than_easy,
        "Hard's best lap ({}) should be no slower than Easy's ({}) - the \
         campaign's own difficulty rung is not reaching the AI",
        describe(hard_lap),
        describe(easy_lap),
    );
}
