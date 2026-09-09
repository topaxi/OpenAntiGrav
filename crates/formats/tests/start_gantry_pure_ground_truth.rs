//! What Wipeout Pure's `TrackStartup.xml` does **not** author: a slot 8.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! `docs/rendering/start-gantry.md` recovers Pulse's start gantry as slot 8 of
//! every circuit's `<TrackStartup>` billboard list, naming
//! `321Go_StartFinish.vex`. Pure's own equivalent file was never read for
//! content before this - `docs/formats/pure-status.md` reached the plugin id
//! and stopped. Reading all sixteen circuits finds **no `num="8"` at all**,
//! model or colour, on any of them - see the page's "Wipeout Pure" section for
//! the write-up. This test is the reproducible half of that claim: read every
//! circuit's own file straight off the disc and assert none of them names a
//! slot 8.
//!
//! Absence over an open-ended search (no gantry-shaped file found anywhere
//! else on the disc) cannot be asserted the same way; that half stays a
//! documented finding rather than a test. What *is* checkable and exhaustive
//! is this file's own schema, circuit by circuit, and that is what this test
//! pins so a future parser change cannot regress it silently.

use std::path::PathBuf;

use oag_formats::trackstartup::TrackStartup;

/// Every circuit Pure's `Data\Plugins\PI001\Definition.xml` declares a
/// `<Values location="...">` for, race, classic and Zone alike - 16 in total,
/// matching the `332 nodes across all 16 circuits` count
/// `docs/formats/pure-status.md` already cites for `Anim Transform`.
const CIRCUITS: &[&str] = &[
    r"Data\Environments\01_Vineta_K",
    r"Data\Environments\03_Modesto_Heights",
    r"Data\Environments\04_Chenghou_Project",
    r"Data\Environments\07_Blue_Ridge",
    r"Data\Environments\08_Sinucit",
    r"Data\Environments\09_Citta_Nuova_V",
    r"Data\Environments\10_Sebenco_Climb",
    r"Data\Environments\12_Sol_2",
    r"Data\ClassicsNS\01_Karbonis",
    r"Data\ClassicsNS\02_Sagarmartha",
    r"Data\ClassicsNS\03_Manor_Top",
    r"Data\ClassicsNS\04_Mandrashee",
    r"Data\Zone\01_Zone",
    r"Data\Zone\02_Zone",
    r"Data\Zone\03_Zone",
    r"Data\Zone\04_Zone",
];

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn no_pure_circuit_authors_a_billboard_slot_8() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_pure::open(image.to_str().expect("image path is utf-8")).expect("open as Pure");

    let mut checked = 0usize;
    let mut total_billboards = 0usize;
    let mut max_num_seen = 0u32;

    for circuit in CIRCUITS {
        let name = format!("{circuit}\\TrackStartup.xml");
        let Ok(blob) = archives.read_name(&name) else {
            panic!("{name} did not resolve, so the circuit list above is stale");
        };
        let xml = String::from_utf8_lossy(&blob);
        let manifest = TrackStartup::parse(&xml);
        checked += 1;
        total_billboards += manifest.billboards.len();
        for billboard in &manifest.billboards {
            max_num_seen = max_num_seen.max(billboard.num);
        }
        assert!(
            manifest.billboard(8).is_none(),
            "{name} authors a slot 8 - Pure has grown a start gantry the doc page \
             does not know about: {:?}",
            manifest.billboard(8)
        );
        // Confirms the fixed-parser guard applies here too: none of Pure's own
        // circuits should reproduce the `14_Track`-shaped bug where a slot
        // survives from outside `<TrackStartup>`.
        assert!(
            manifest.billboards.iter().all(|b| b.num <= 8),
            "{name} names a billboard past the documented 1-8 range: {:?}",
            manifest.billboards
        );
    }

    assert_eq!(checked, CIRCUITS.len(), "every listed circuit resolved");
    println!(
        "{checked} circuit(s), {total_billboards} billboard(s) total, highest num seen {max_num_seen}"
    );
    assert!(
        max_num_seen <= 7,
        "the highest slot number seen was {max_num_seen}, which contradicts this test's \
         own claim that num 8 never appears"
    );
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn no_pure_billboard_names_a_model() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_pure::open(image.to_str().expect("image path is utf-8")).expect("open as Pure");

    let mut with_location = 0usize;
    let mut with_colour = 0usize;

    for circuit in CIRCUITS {
        let name = format!("{circuit}\\TrackStartup.xml");
        let Ok(blob) = archives.read_name(&name) else {
            continue;
        };
        let xml = String::from_utf8_lossy(&blob);
        let manifest = TrackStartup::parse(&xml);
        for billboard in &manifest.billboards {
            match billboard.location() {
                Some(_) => with_location += 1,
                None => with_colour += 1,
            }
        }
    }

    println!("{with_location} billboard(s) with a location, {with_colour} with only a colour");
    assert_eq!(
        with_location, 0,
        "a Pure circuit now names a billboard location - re-check whether it is the gantry"
    );
    assert!(
        with_colour > 0,
        "expected at least the race circuits' colour-only slots"
    );
}
