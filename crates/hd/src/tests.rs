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
    // Eight names, seven archives: `DATA03` is also the bulk fallback a PSN
    // install (which has no `DATA00`) resolves to, and the asset layer mounts it
    // once. Exactly one archive is named twice, and it is that one.
    assert_eq!(named.len(), 8, "{named:?}");
    named.sort();
    named.dedup();
    assert_eq!(named.len(), 7, "{named:?}");
    assert_eq!(
        TITLE
            .archive_names()
            .iter()
            .filter(|n| n.ends_with("DATA03.PSARC"))
            .count(),
        2
    );
    for name in &named {
        assert!(
            name.starts_with("USRDIR/DATA") && name.ends_with(".PSARC"),
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

/// The front end is wired, and its boot chain has now been watched.
///
/// **This test has asserted three different things and kept one intent.** It
/// began as `front_end == None` - a refusal, back when the only way to withhold
/// an unmeasured chain was to withhold the whole front end. ADR-0025 moved the
/// seam to `oag_title::Provenance` and it became "nobody has quietly upgraded
/// this to `Measured`". On 2026-09-05 somebody upgraded it *loudly*, so it is
/// now the other guard: **nobody may quietly downgrade it either**, and the
/// evidence lives beside the assertion rather than in a commit message.
///
/// The capture: `just rpcs3-bootchain`, three times, savedata moved aside so
/// `FirstPlay` is not skipped, every run walking all eight steps of
/// `frontend::BOOT_CHAIN` in order with nothing between them. Kept under
/// `data/reference/hd-boot-chain/`; transcript on `docs/formats/hd-frontend.md`.
///
/// **What is still not claimed**, and what a future reader should not read out
/// of `Measured`: RPCS3 is not a PS3, and no frame of the language picker was
/// ever caught - it is entered on every boot and may auto-redirect without ever
/// presenting. `Provenance` is two-valued by design and grades neither.
#[test]
fn the_front_end_is_wired_and_its_chain_has_been_watched() {
    let front_end = TITLE
        .front_end
        .expect("HD's front end is wired; see frontend::FRONT_END");
    assert_eq!(
        front_end.boot.provenance,
        oag_title::Provenance::Measured,
        "three cold boots on RPCS3, 2026-09-05; see data/reference/hd-boot-chain/"
    );
    assert!(front_end.boot.provenance.is_measured());
    assert_eq!(frontend::MENU_SKIN.menu_x, 800.0, "1920-wide, not 480-wide");
    assert_eq!(frontend::BOOT.chain, frontend::BOOT_CHAIN);
    assert_eq!(frontend::BOOT_CHAIN.len(), 8);
    assert_eq!(
        frontend::BOOT_CHAIN
            .iter()
            .filter(|step| step.movie.is_some())
            .count(),
        1,
        "only Studio Logo plays anything"
    );
    // The order three boots actually went in, spelled out here rather than only in
    // prose: a step silently reordered or dropped is exactly the regression a
    // `Measured` label makes expensive, and `len() == 8` does not catch it.
    assert_eq!(
        frontend::BOOT_CHAIN
            .iter()
            .map(|step| step.state)
            .collect::<Vec<_>>(),
        vec![
            "Language Selection",
            "PreFMVConnect",
            "Studio Logo",
            "EpilepsyWarning",
            "FirstPlay",
            "Save Warning",
            "EULA",
            "Update Announcement",
        ],
        "data/reference/hd-boot-chain/cold-01, cold-02 and cold-03, then Main Menu"
    );
}

/// The cycle reproduces the two fragment-program constants read off RPCS3 on
/// `01_vineta_k` (`data/scratch/hd-weapon-pads/cap1`, draws 108 and 109), one
/// pad in the third keyframe's span and one in the first's, both from the same
/// frame and both at the one scale `1/255`.
///
/// The position of each is solved from its green channel alone (the only
/// channel that moves in both spans), so the red channel and the third lane
/// are the checks.
#[test]
fn the_weapon_pad_cycle_reproduces_both_measured_constants() {
    let oag_title::weapon_pad::WeaponPadGlow::Cycle(cycle) = TITLE.weapon_pad_glow else {
        panic!("HD's weapon pads cycle");
    };
    // Third keyframe to the fourth: green rises 0 -> 16.
    let a = cycle.colour(2.0 + 0.5208);
    assert!((a[0] - 1.16612).abs() < 2e-4, "{a:?}");
    assert!((a[1] - 0.032_683_5).abs() < 2e-4, "{a:?}");
    assert_eq!(a[2], 0.0);
    // First keyframe to the second: green falls 16 -> 0.
    let b = cycle.colour(0.35071);
    assert!((b[0] - 1.65581).abs() < 2e-4, "{b:?}");
    assert!((b[1] - 0.040_742_8).abs() < 2e-4, "{b:?}");
    assert_eq!(b[2], 0.0);
    // Cooling is the vector the static initialiser writes, read live as well.
    assert_eq!(cycle.cooling, [0.025, 0.0, 0.01]);
}
