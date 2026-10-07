//! A race load answers its repeated archive reads from memory.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(race_load_read_memo_ground_truth)'
//! ```
//!
//! An HD race load reads each material's `.rcsmaterial` once per pass that
//! inspects it and once per model that uses it: 15,194 reads of 758 entries
//! on Vineta K, which inflated 3.2 GB for 262 MB of data and took 7.9 s
//! before `oag_assets::read_memo` (`docs/architecture/load-time.md`). This
//! fails if `race::load` stops asking for the memo.

use oag_raceplay as race;

const TRACK: &str = "/data/environments/01_vineta_k/track.vex";

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_hd_race_load_answers_most_reads_from_the_read_memo() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");

    let line = loaded
        .report
        .iter()
        .find(|line| line.starts_with("archive reads: "))
        .unwrap_or_else(|| panic!("no read memo line in {:#?}", loaded.report));
    println!("{line}");
    let mut numbers = line
        .split_whitespace()
        .filter_map(|word| word.parse::<usize>().ok());
    let (hits, reads) = (numbers.next().unwrap(), numbers.next().unwrap());
    assert!(
        hits * 10 >= reads * 9,
        "only {hits} of {reads} reads came from the memo"
    );
}
