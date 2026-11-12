//! What this table is asserted to be, without a disc.
//!
//! The archive-backed half is `crates/omega/tests/omega_title_ground_truth.rs`,
//! which re-reads `skin.xml` and the grid count off `data09.psarc` rather than
//! trusting the constants in `frontend.rs`.

use super::*;

/// Omega ships no archive name HD's or 2048's own candidates would match, so
/// there is nothing for a foreign-serial deny-list to rule out - a directory
/// source never reaches the serial check either way (see
/// [`crate::TITLE::foreign_serials`]'s own doc comment).
#[test]
fn foreign_serials_is_empty_on_the_same_terms_2048s_is() {
    assert!(TITLE.foreign_serials.is_empty());
}

/// Every archive is named exactly once across the three roles, and all nine
/// are there - the same "extra is a set, a missing row is nothing mounted"
/// check `oag_hd`'s own test runs.
#[test]
fn all_nine_archives_are_mounted_and_none_twice() {
    let mut named = TITLE.archive_names();
    assert_eq!(named.len(), 9, "{named:?}");
    named.sort();
    named.dedup();
    assert_eq!(named.len(), 9, "one archive is named in two roles");
    for name in &named {
        assert!(
            name.starts_with("uroot/data") && name.ends_with(".psarc"),
            "{name}"
        );
    }
}

/// `data09` - the newest layer of the front end - is the *bulk*
/// candidate, so it is searched before every other archive this title
/// mounts. This is the whole of what makes the patch-ahead-of-base design
/// work; a future edit that moved it into `extra` would compile fine and
/// silently stop winning collisions, which is exactly the failure this
/// assertion exists to catch.
#[test]
fn data09_is_the_bulk_candidate_not_an_extra_one() {
    assert_eq!(TITLE.archives.data, [(archives::DATA09, Platform::Ps4)]);
    assert!(
        !TITLE
            .archives
            .extra
            .iter()
            .any(|(name, _)| *name == archives::DATA09)
    );
}

/// The circuit-model table names Omega's own folders, finds a circuit by the
/// last component of its location and says nothing for one it has no row for:
/// HD's `03_Track` is not a folder on this disc.
#[test]
fn circuit_models_are_looked_up_by_omega_folder() {
    let front_end = frontend::FRONT_END;
    assert!(front_end.draws_circuit_models());
    assert_eq!(
        front_end
            .circuit_model(r"Data\Environments\03_Moa_Therma")
            .as_deref(),
        Some(r"Data\Environments\03_Moa_Therma\FE\track03.vex")
    );
    assert_eq!(
        front_end
            .circuit_model(r"Data\Environments2048\altima")
            .as_deref(),
        Some(r"Data\Environments2048\altima\FE\Track01.vex")
    );
    assert_eq!(front_end.circuit_model(r"Data\Environments\03_Track"), None);
    let mut folders: Vec<_> = front_end
        .circuit_models
        .iter()
        .map(|row| row.environment)
        .collect();
    folders.sort_unstable();
    folders.dedup();
    assert_eq!(
        folders.len(),
        front_end.circuit_models.len(),
        "one row each"
    );
}

/// Omega draws the two-player HUD its own disc ships, the way HD does: dropping the
/// row would put its two views back on the whole-screen layout scaled to fit.
#[test]
fn the_two_player_hud_is_the_one_the_disc_authors() {
    let split = TITLE.hud.split.expect("the split row");
    assert_eq!(split.view_height, 540);
    for root in split.arcade.iter().chain(&split.elimination) {
        assert!(
            root.starts_with("Data/xml/splitscreen_hud/splitscreen_"),
            "{root}"
        );
    }
    assert_eq!(split.arcade.len() + split.elimination.len(), 4);
}

/// Omega's HUD always-on list is HD's, one definition: Omega's Eliminator authors `LapPanel2` too
/// (`omega_hud_ground_truth`), so a list of its own would drift again the way a copy did.
#[test]
fn omega_draws_hds_always_on_widgets() {
    assert_eq!(TITLE.hud_art.always_on, oag_hd::TITLE.hud_art.always_on);
    assert!(TITLE.hud_art.always_on.contains(&"LapPanel2"));
}

/// Omega carries HD's grid hover, so its craft is held low on the grid and released after GO.
/// Dropping the `launch_hover` wiring from `race.rs` leaves a craft that floats at its race height.
#[test]
fn omega_carries_the_grid_hover_hd_does() {
    let hover = TITLE.race.launch_hover.expect("Omega's grid hover");
    assert_eq!(hover.grid_cap.value, 3.03825);
    assert_eq!(hover.release_rate.value, 1.0);
    assert_eq!(hover.release_rate.origin, oag_title::Origin::Measured);
    assert_eq!(
        hover.grid_cap.value,
        oag_hd::TITLE
            .race
            .launch_hover
            .expect("HD's")
            .grid_cap
            .value
    );
}

#[test]
fn omega_culls_by_the_material_state_bit_as_hd_does() {
    assert!(TITLE.looks.material_render_state.applies_everywhere());
    assert_eq!(
        TITLE.looks.material_render_state.origin,
        oag_title::Origin::InheritedFrom("Wipeout HD")
    );
}

/// Omega names a loading screen: five features and six marks as `.gnf`, one styling, a still per
/// mode and no clip. `loading: None` again fails here.
#[test]
fn omega_names_a_loading_screen_of_gnf_files() {
    let loading = TITLE.loading.expect("Omega names a loading screen");
    assert_eq!(loading.features.len(), 1);
    let style = &loading.features[0];
    assert_eq!(style.features.len(), 5);
    assert!(style.mode_icons.is_empty(), "no .bik ships on Omega");
    assert!(style.features.iter().all(|f| f.image.ends_with(".gnf")));
    assert_eq!(loading.chrome.len(), 6);
    assert!(loading.chrome.iter().all(|(_, e)| e.ends_with(".gnf")));
    assert!(style.mode_stills.entry(Some(3)).is_some(), "single race");
    assert_ne!(
        style.mode_stills.entry(Some(5)),
        style.mode_stills.entry(Some(8))
    );
    assert!(style.mode_stills.entry(Some(0xd)).is_none());
    assert_eq!(
        style.mode_stills.entries().count(),
        6,
        "five modes and single race"
    );
    assert!(loading.layout.is_some());
}

/// A 2048-era circuit (either slash, any case) is in the lineage that carries 2048's HUD set and
/// laps; an HD-era circuit and the Zone circuits are in none.
#[test]
fn only_the_2048_era_circuits_are_in_the_2048_lineage() {
    let race = TITLE.race;
    let found = race
        .circuit_lineage("Data/Environments2048/Altima/track.vex")
        .expect("a 2048-era circuit");
    assert_eq!(found.layouts.arcade, r"Data\XML\2048_hud\Arcade_HUD.xml");
    assert_eq!(found.laps_by_class, Some([4, 4, 5, 5]));
    assert_eq!(found.grid, Some(oag_title::Platform::Vita));
    for hd in [
        r"Data\environments\tech_de_ra\track.vex",
        r"Data\environments\zone_1\track.vex",
    ] {
        assert!(race.circuit_lineage(hd).is_none(), "{hd}");
    }
}

/// Omega's craft is posed from a borrowed pose, not framed by a fit, and a step plays HD's
/// depth law on it: the settle's data is HD's own, one definition. Dropping the field leaves the
/// craft cutting in at rest size with no settle.
#[test]
fn omega_plays_hds_ship_select_settle_on_its_borrowed_pose() {
    let front_end = TITLE.front_end.expect("Omega's front end");
    assert!(front_end.ship_preview_pose.is_some());
    assert!(front_end.ship_preview_fit.is_none());
    assert_eq!(
        front_end.ship_preview_settle,
        Some(oag_hd::frontend::SHIP_FIT)
    );
}
