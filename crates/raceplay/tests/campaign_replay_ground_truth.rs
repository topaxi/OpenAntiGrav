//! Campaign replays a player recorded on a Steam Deck open, rebuild, reproduce every stored
//! hash to their last tick and seek to their end, through the path WATCH takes: the header's
//! `Options::for_replay` over this machine's disc, the ordinary load with the packs mounted,
//! then `Viewer::open` over the loaded setup (`docs/ui/replays.md`, "The viewer").
//!
//! The five files are one Pulse EU Flash tournament (`[setup] mode = "tournament"`, the original
//! driver at a campaign cell's skill). **They were written before `[options] campaign_skill`
//! existed**, and a campaign cell's scale also tunes the field and the finished player's thrust,
//! so unedited they disagree at tick 360 or 420 (`docs/formats/replay-file.md`). The test adds
//! the key, by name, with the value this build writes for them ([`add_campaign_skill`]): the
//! cell's scale, which the original driver's `ai_skill` already carries. They are a player's, not game content, but they are kept
//! out of git all the same: the test reads them from `OAG_CAMPAIGN_REPLAYS` (a directory) or
//! `data/reference/replays/pulse-tournament/`, and skips without them.
//!
//! **They desync, and that is expected.** A recording reproduces only the physics that wrote it.
//! Pulse's weapon blasts now post their push through `pending_impulse` from the craft's nose
//! point (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`, "Wired: every blast posts
//! its push"), which the recordings predate, so each leg diverges at the first blast that
//! reaches a craft; the maintainer accepts that a physics change desyncs old recordings. Each
//! leg names its first diverging tick ([`Expect::Desync`]): a leg that stops diverging, or
//! diverges elsewhere, fails. Re-recorded files go back to [`Expect::Complete`].
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all campaign_replay_ground_truth
//! ```

use std::path::{Path, PathBuf};

use oag_raceplay as race;
use oag_raceplay::timeline::Pass;
use oag_replay::Replay;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The recording `name`, or `None` (and a skip) without it.
fn recording(name: &str) -> Option<Replay> {
    let dir = std::env::var_os("OAG_CAMPAIGN_REPLAYS").map_or_else(
        || root().join("data/reference/replays/pulse-tournament"),
        PathBuf::from,
    );
    let path = dir.join(name);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipping: {} not present", path.display());
        return None;
    };
    Some(Replay::from_bytes(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

/// What this build writes into a campaign recording's header and these files predate: the
/// cell's scale, which they carry as the original driver's `ai_skill`.
fn add_campaign_skill(replay: &mut Replay) {
    let options = &mut replay.header.options;
    assert!(
        !options.contains_key("campaign_skill"),
        "the file already names it"
    );
    let scale = options
        .get("ai_skill")
        .cloned()
        .expect("a campaign cell flown by the original driver records its scale");
    options.insert("campaign_skill".to_string(), scale);
}

/// The disc they were recorded on, or `None` without it.
fn source() -> Option<PathBuf> {
    let path = root().join("data/images/pulse-psp-eu.chd");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// What a recording's opening pass is expected to do on this build.
#[derive(Clone, Copy)]
enum Expect {
    /// Reproduce every stored hash to the last tick.
    Complete,
    /// Diverge from the stored hashes first at this tick.
    Desync(u64),
}

/// Opens `name` as WATCH does, plays the opening pass to its end and seeks the race on screen
/// to the last tick, or checks the tick it diverges at when `expect` says it must.
fn watches_to_the_end(name: &str, expect: Expect) {
    let (Some(mut replay), Some(source)) = (recording(name), source()) else {
        return;
    };
    add_campaign_skill(&mut replay);
    let mut options = race::Options::for_replay(&replay.header, &source.display().to_string())
        .unwrap_or_else(|e| panic!("{name}: {e:#}"));
    assert_eq!(options.mode, oag_race::Mode::Tournament, "{name}");
    // The packs a player has mounted, as the session's race box mounts them: the header's
    // opponent teams name all four Pulse packs' teams.
    options.dlc = vec![root().join("data/dlc")];
    let loaded = race::load(&options).unwrap_or_else(|e| panic!("{name}: loading: {e:#}"));
    let mut on_screen = race::Race::start(loaded.setup.clone());
    let ticks = replay.ticks();
    let mut viewer = race::viewer::Viewer::open(loaded.setup, replay)
        .unwrap_or_else(|e| panic!("{name}: refused at tick zero: {e}"));
    viewer.begin(&mut on_screen).expect("seats the race");
    let pass = loop {
        match viewer.pass() {
            Pass::Running => std::thread::sleep(std::time::Duration::from_millis(50)),
            done => break done,
        }
    };
    if let Expect::Desync(tick) = expect {
        match pass {
            Pass::Desynced(desync) => assert_eq!(
                desync.tick, tick,
                "{name}: the opening pass over {ticks} ticks diverges elsewhere"
            ),
            other => panic!("{name}: expected a desync at tick {tick}, got {other:?}"),
        }
        println!("{name}: desyncs at tick {tick}, as expected");
        return;
    }
    assert_eq!(
        pass,
        Pass::Complete,
        "{name}: the opening pass over {ticks} ticks"
    );
    assert!(!viewer.stops_matching(), "{name}: the mismatch banner");
    assert_eq!(viewer.built(), Some(ticks), "{name}: every tick is built");
    viewer.seek(&mut on_screen, ticks);
    assert_eq!(viewer.at(), ticks, "{name}: the playhead reaches the end");
    println!(
        "{name}: {ticks} ticks reproduced, {} stored hashes, finished in place {:?}",
        ticks / u64::from(viewer.header().hash_interval.max(1)),
        viewer.header().finish.as_ref().map(|f| f.place)
    );
}

macro_rules! deck_tests {
    ($($test:ident: $file:literal, $expect:expr;)*) => {$(
        #[test]
        #[ignore = "needs data/images/pulse-psp-eu.chd and the recorded files"]
        fn $test() {
            watches_to_the_end($file, $expect);
        }
    )*};
}

deck_tests! {
    leg_1_a_deck_tournament_replay_watches_to_its_end: "oag-wipeout-pulse-1794126092-1.oagr", Expect::Desync(1260);
    leg_2_a_deck_tournament_replay_watches_to_its_end: "oag-wipeout-pulse-1794126226-1.oagr", Expect::Desync(1320);
    leg_3_a_deck_tournament_replay_watches_to_its_end: "oag-wipeout-pulse-1794126364-1.oagr", Expect::Desync(960);
    leg_4_a_deck_tournament_replay_watches_to_its_end: "oag-wipeout-pulse-1794126501-1.oagr", Expect::Desync(1020);
    leg_5_a_deck_tournament_replay_watches_to_its_end: "oag-wipeout-pulse-1794126656-1.oagr", Expect::Desync(1500);
}
