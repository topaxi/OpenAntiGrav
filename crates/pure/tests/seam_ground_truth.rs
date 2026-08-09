//! Opens a real Wipeout Pure disc through the same `oag-assets` mechanism that
//! opens Pulse's, with nothing different but the table.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! ADR-0021 claims title is a data axis. That claim is cheap to make and easy to
//! satisfy decoratively - a crate of constants nobody exercises looks identical
//! to a working seam. This is the difference: a second title's disc opening
//! through the *same* code path, and a parser that used to refuse its file
//! reading it.
//!
//! It also supplies the evidence the handling-schema change did not have when it
//! landed. That change let the parser accept a five-rung ladder and treat three
//! Pulse-era attributes as optional, but could only be tested against synthetic
//! documents shaped like Pure's, because opening Pure's disc needed this crate.

use std::path::{Path, PathBuf};

/// The Pure disc this reads. Its serial is `UCUS-98612`.
const IMAGE: &str = "data/images/pure-psp-usa.chd";

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// `None`, with a printed reason, when the disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image() -> Option<String> {
    let path = workspace(IMAGE);
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    assert!(
        std::env::var("OAG_REQUIRE_GAME_DATA").as_deref() != Ok("1"),
        "{IMAGE} is required but absent"
    );
    println!("skipping: {IMAGE} not present");
    None
}

/// The seam, stated as one assertion: Pure's disc opens, and it opens through
/// `oag_assets::Archives` rather than through anything Pure-specific.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_disc_opens_through_the_same_mechanism_as_pulses() {
    let Some(source) = image() else { return };

    let archives = oag_pure::open(&source).expect("Pure's disc opens as Pure");
    assert_eq!(
        archives.layout.data,
        format!("{source}:{}", oag_pure::archives::DATA)
    );
    assert!(
        archives.fe.is_some(),
        "Pure ships FE.wad beside Data.wad, so the companion archive resolves"
    );
}

/// The deny-lists work in both directions on real discs, which is the thing
/// that stops name matching opening one title as the other.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pulse_refuses_pures_disc_by_serial() {
    let Some(source) = image() else { return };

    let error = oag_pulse::open(&source).expect_err("Pulse must refuse Pure's disc");
    let message = error.to_string();
    assert!(
        message.contains("Wipeout Pure"),
        "the refusal should name what the disc actually is: {message}"
    );
}

/// The evidence the handling-schema change was missing: Pure's own
/// `handlingstats.xml` parses, with the five-rung ladder and the three absent
/// attributes that used to make this parser refuse it outright.
///
/// # Currently red, deliberately, and this is the finding
///
/// Pointing the parser at a real Pure file turned up **more schema differences
/// than `pure-status.md` recorded at the time**, one behind the other, each only
/// visible once the one before it was fixed. All four are on that page now:
///
/// 1. five `<Class>` rungs rather than four - was recorded, and handled
/// 2. `easyshield`, `weight_distribution`, `sideshift` absent - was recorded,
///    and handled
/// 3. **no `<FE>` element at all** - unrecorded until this test found it.
///    Handled: `Stats::fe` is now an `Option`, which costs nothing because
///    `<FE>` is presentation and no simulation path reads it
/// 4. **no `<pitch>` element** - unrecorded too, and *not* handled here
///
/// Four is where this stops on purpose. `<pitch>` feeds `oag_physics::Pitch`,
/// so making it optional is a change to a simulation input, and ADR-0009 item 2
/// keeps second-title simulation work behind M4's exit. Doing it under the
/// momentum of a chain of small fixes is exactly how a physics regression gets
/// in unnoticed.
///
/// **There are no more elements behind `<pitch>`**, and that is now a
/// measurement rather than a hope: `handling_schema_ground_truth` walks every
/// shipped file on both discs and asserts the entire element and attribute
/// difference in one comparison. `<pitch>` is the last of it. The same pass
/// found two things this one-file probe could not - a Pure file with no
/// `<Class>` blocks at all, and the engine-wide `Data\XML\HandlingStats.xml`,
/// which parses unchanged - both recorded in `pure-status.md`.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_handlingstats_is_read_up_to_its_next_schema_difference() {
    use oag_formats::handling;

    let Some(source) = image() else { return };
    let mut archives = oag_pure::open(&source).expect("Pure's disc opens");

    // Feisar is the one team `pure-status.md` names on both discs, so it is the
    // one that can be asked for without inventing a roster.
    let entry = oag_pure::names::handling_stats("Feisar");
    let blob = archives
        .read_name(&entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    // Pinned as it is, not as it should be: this asserts the *next* blocker, so
    // it fails loudly the day someone clears it and has to come back here and
    // record what the file then does. Same shape as `pvs_ground_truth`'s
    // "Pure finds zero sections" assertion.
    let error = handling::from_blob(&blob).expect_err(
        "if Pure's handlingstats.xml now parses, the schema chain has moved - \
         re-read this test's doc comment and record what changed",
    );
    assert!(
        matches!(error, handling::Error::MissingElement { element: "pitch" }),
        "the next blocker moved from <pitch> to something else: {error:?}"
    );
}
