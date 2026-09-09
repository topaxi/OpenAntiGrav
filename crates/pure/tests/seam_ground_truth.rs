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
//! ADR-0022 claims title is a data axis. That claim is cheap to make and easy to
//! satisfy decoratively - a crate of constants nobody exercises looks identical
//! to a working seam. This is the difference: a second title's disc opening
//! through the *same* code path, and a parser that used to refuse its file
//! reading it.
//!
//! It also supplies the evidence the handling-schema change did not have when it
//! landed. That change let the parser accept a five-rung ladder and treat three
//! Pulse-era attributes as optional, but could only be tested against synthetic
//! documents shaped like Pure's, because opening Pure's disc needed this crate.

/// The Pure disc this reads. Its serial is `UCUS-98612`.
const IMAGE: &str = "data/images/pure-psp-usa.chd";

/// `None`, with a printed reason, when the disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image() -> Option<String> {
    oag_testdata::image(IMAGE).map(|path| path.display().to_string())
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

/// Pure's own `handlingstats.xml` parses, chain of schema differences and all.
///
/// # This test was red on purpose for a while, and clearing it is the finding
///
/// Pointing the parser at a real Pure file turned up **more schema differences
/// than `pure-status.md` recorded at the time**, one behind the other, each only
/// visible once the one before it was fixed. All four are on that page now:
///
/// 1. five `<Class>` rungs rather than four - was recorded, and handled
/// 2. `easyshield`, `weight_distribution`, `sideshift` absent - was recorded,
///    and handled
/// 3. **no `<FE>` element at all** - unrecorded until this test found it.
///    Handled: `Stats::fe` is an `Option`, which costs nothing because `<FE>` is
///    presentation and no simulation path reads it
/// 4. **no `<pitch>` element** - unrecorded too, and deliberately left unhandled
///    for a while
///
/// Four is where this test used to stop, on the grounds that `<pitch>` feeds
/// `oag_physics::Pitch`, so making it optional changes a simulation input and
/// ADR-0009 item 2 keeps second-title simulation work behind M4's exit.
///
/// **Cleared 2026-08-12**, and the reasoning that changed is worth keeping: the
/// gate is on *implementing Pure's physics*, and making the parser report what
/// the file says is not that. `Class::pitch` is an `Option`, so `oag-formats`
/// now describes the document faithfully - which is its whole job - and the
/// decision about what a ship with no authored pitch response does was moved up
/// to `oag_gameplay::handling::PITCH_STAND_IN`, where it is labelled an
/// invention, reported by name at every race load, and asserted unreachable on
/// Pulse by `no_pulse_class_reaches_the_pitch_stand_in`. That is a different
/// thing from quietly defaulting a physics input, which is what the old comment
/// was right to refuse.
///
/// **There are no more elements behind `<pitch>`**, and that is a measurement
/// rather than a hope: `handling_schema_ground_truth` walks every shipped file
/// on both discs and asserts the entire element and attribute difference in one
/// comparison. `<pitch>` was the last of it, which is why this test now asserts
/// a successful parse rather than the next blocker.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_handlingstats_is_read_up_to_its_next_schema_difference() {
    use oag_tables::handling;

    let Some(source) = image() else { return };
    let mut archives = oag_pure::open(&source).expect("Pure's disc opens");

    // Feisar is the one team `pure-status.md` names on both discs, so it is the
    // one that can be asked for without inventing a roster.
    let entry = oag_pure::names::handling_stats("Feisar");
    let blob = archives
        .read_name(&entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    let stats = handling::from_blob(&blob).unwrap_or_else(|e| {
        panic!(
            "{entry} no longer parses: {e:?}. A schema difference has appeared \
             behind the four this test's doc comment lists - record it in \
             docs/formats/pure-status.md rather than only fixing it"
        )
    });

    // The three differences that made it parse, asserted rather than assumed, so
    // "it parses" cannot come to mean "the parser got laxer than the file is".
    assert_eq!(
        stats.classes.len(),
        5,
        "Pure authors a fifth rung below Pulse's slowest"
    );
    assert!(stats.fe.is_none(), "Pure authors no <FE> element");
    assert!(
        stats.classes.iter().all(|class| class.pitch.is_none()),
        "Pure authors no <pitch> in any class; if one appears, \
         `oag_gameplay::handling::PITCH_STAND_IN` is no longer what this title \
         flies on and its docs need re-reading"
    );
    // The fifth rung is outside `SpeedClass`, which is what `raw_name` exists
    // for. Named rather than counted, because "five classes" would also be true
    // of a file whose ladder was Pulse's plus a duplicate.
    assert!(
        stats
            .classes
            .iter()
            .any(|class| class.name.is_none() && !class.raw_name.is_empty()),
        "one rung should sit outside Pulse's four-name ladder"
    );
}
