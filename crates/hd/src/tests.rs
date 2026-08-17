//! What this table is asserted to be, without a disc.
//!
//! The disc-backed half is `crates/hd/tests/hd_title_ground_truth.rs`, which
//! re-derives the rosters from the manifest rather than trusting them.

use super::*;

/// The deny-list rules out, never in - the same asymmetry the other two title
/// packages have, and HD's own serial deliberately gets no verdict.
#[test]
fn both_psp_titles_are_ruled_out_and_hds_own_serial_gets_no_verdict() {
    assert_eq!(TITLE.foreign_title("UCUS-98712"), Some("Wipeout Pulse"));
    assert_eq!(TITLE.foreign_title("UCUS-98612"), Some("Wipeout Pure"));
    assert_eq!(TITLE.foreign_title("BCES-00664"), None, "HD's own EU disc");
    assert_eq!(
        TITLE.foreign_title("BCUS-98844"),
        None,
        "an uncatalogued one"
    );
}

/// Every archive is named exactly once across the three roles, and all seven are
/// there.
///
/// The check that would have caught mounting six: `extra` is a *set*, so a
/// missing row is not a fallback that quietly picks something else - it is an
/// archive nothing ever opens. Four teams live in `DATA03` alone.
#[test]
fn all_seven_archives_are_mounted_and_none_twice() {
    let mut named = TITLE.archive_names();
    assert_eq!(named.len(), 7, "{named:?}");
    named.sort();
    named.dedup();
    assert_eq!(named.len(), 7, "one archive is named in two roles");
    for name in &named {
        assert!(
            name.starts_with("PS3_GAME/USRDIR/DATA") && name.ends_with(".PSARC"),
            "{name}"
        );
    }
}

/// The default circuit is one this crate also lists, spelled the same way.
///
/// Two constants that have to agree and no compiler check that they do: the
/// default is a full path and the roster is directory names, so a rename of
/// either alone would leave a default that resolves to nothing.
#[test]
fn the_default_circuit_is_in_the_roster() {
    assert!(
        names::ENVIRONMENTS
            .iter()
            .any(|env| names::track(env) == race::DEFAULT_TRACK),
        "{} is not one of the {} environments listed",
        race::DEFAULT_TRACK,
        names::ENVIRONMENTS.len()
    );
}

/// The default team is a team rather than one of the three mode ships.
///
/// `detonator` and `zone` parse as ship files and author no speed class at all,
/// so racing one would load and then behave nothing like a race. Worth an
/// assertion precisely because it would not fail loudly.
#[test]
fn the_default_team_is_a_team_and_not_a_mode_ship() {
    assert!(names::TEAMS.contains(&race::DEFAULT_TEAM));
    assert!(!names::MODE_SHIPS.contains(&race::DEFAULT_TEAM));
}

/// HD's team ids are lowercase where the PSP titles' are capitalised, and the
/// circuit names diverge outright.
///
/// Both differences are load-bearing and neither is visible in a diff of two
/// title crates, so they are asserted against the other packages directly.
#[test]
fn the_ids_differ_from_the_psp_titles_in_exactly_the_two_measured_ways() {
    assert_ne!(
        race::DEFAULT_TRACK,
        oag_pulse::race::DEFAULT_TRACK,
        "HD calls this circuit talons_junction and Pulse calls it 16_Track"
    );
    assert_eq!(
        race::DEFAULT_TEAM,
        oag_pulse::race::DEFAULT_TEAM.to_ascii_lowercase(),
        "same team, and the case is the whole of the difference"
    );
    assert_eq!(oag_pure::race::DEFAULT_TEAM, oag_pulse::race::DEFAULT_TEAM);
}

/// The front end is recovered as data and deliberately not wired.
///
/// **Asserted rather than left implicit**, because `front_end: None` next to a
/// populated `frontend` module reads exactly like an oversight. It is not: see
/// that module's docs. Whoever sets this to `Some` should have measured the boot
/// chain, and this test is where they will be told so.
#[test]
fn the_layout_is_recovered_and_the_boot_chain_is_still_only_declared() {
    assert!(
        TITLE.front_end.is_none(),
        "the declared chain is a hypothesis; BootProfile::chain is a measurement"
    );
    assert_eq!(frontend::MENU_SKIN.menu_x, 800.0, "1920-wide, not 480-wide");
    assert_eq!(frontend::DECLARED_CHAIN.len(), 8);
    assert_eq!(
        frontend::DECLARED_CHAIN
            .iter()
            .filter(|step| step.movie.is_some())
            .count(),
        1,
        "only Studio Logo plays anything"
    );
}
