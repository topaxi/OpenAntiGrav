//! The plain Wipeout HD PSN download (`NPEA00057`, v3.00) opens as HD, and what
//! `docs/formats/hd-psn.md` says about it still holds.
//!
//! **`#[ignore]`d and never run in CI.** It needs the install folder under
//! `data/extracted/ps3/hd-psn-eu/` (steps in `docs/overview/installing.md`).
//! Skips without it, and fails under `OAG_REQUIRE_GAME_DATA=1` when it is
//! missing.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-hd --run-ignored all \
//!     -E 'binary(hd_psn_ground_truth)'
//! ```

use oag_assets::Archives;

const INSTALL: &str = "data/extracted/ps3/hd-psn-eu";

fn has(archives: &Archives, name: &str) -> bool {
    archives.locate(name).is_some()
}

fn opened() -> Option<Archives> {
    let path = oag_testdata::exact(INSTALL)?;
    Some(oag_hd::open(path.to_str().expect("utf-8 path")).expect("the install opens as HD"))
}

/// Census: four archives mounted, bulk `DATA03`, companion `DATA02`, 6,674
/// entries (53 + 5,292 + 1,275 + 54, counted from each archive's own manifest).
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn four_archives_mount_in_the_chosen_order() {
    let Some(archives) = opened() else { return };

    let tail = |spec: &str| spec.rsplit('/').next().unwrap_or(spec).to_ascii_lowercase();
    assert_eq!(tail(&archives.layout.data), "data03.psarc");
    assert_eq!(
        archives.layout.fe.as_deref().map(tail).as_deref(),
        Some("data02.psarc")
    );
    let extra: Vec<String> = archives.layout.extra.iter().map(|s| tail(s)).collect();
    assert_eq!(extra, ["data01.psarc", "data04.psarc"], "{extra:?}");
    assert!(archives.layout.patch.is_empty(), "v3.00 is a full package");

    let total: usize = std::iter::once(&archives.data)
        .chain(archives.fe.as_ref())
        .chain(&archives.extra)
        .map(oag_assets::Container::entry_count)
        .sum();
    assert_eq!(total, 6_674);
}

/// The PSN install is the PSN variant of the title and keeps HD's name.
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn the_install_is_the_psn_variant_and_keeps_the_name() {
    let Some(archives) = opened() else { return };
    let title = oag_hd::title_of(&archives);
    assert_eq!(title.name, oag_hd::TITLE.name);
    assert_eq!(title.race.track, oag_hd::psn::DEFAULT_TRACK);
    assert_ne!(title.race.track, oag_hd::TITLE.race.track);
    assert!(title.front_end.is_some_and(|f| f.team_select.is_none()));
}

/// The PSN defaults resolve, and the Fury names the disc's `TITLE` carries do
/// not - which is the whole reason the variant exists.
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn psn_names_resolve_and_fury_names_are_absent() {
    let Some(archives) = opened() else { return };
    let title = oag_hd::title_of(&archives);

    assert!(has(&archives, title.race.track), "{}", title.race.track);
    assert!(has(
        &archives,
        &format!("/data/ships/{}/handlingstats.xml", title.race.team)
    ));
    if let oag_title::exhaust::Exhaust::Authored(path) = title.exhaust {
        assert!(has(&archives, path), "{path}");
    } else {
        panic!("HD authors its ribbon");
    }

    for fury in [
        oag_hd::race::DEFAULT_TRACK,
        "/data/environments/zone_1/track.vex",
        "/data/ribboneffects/enginetrail_bluered_triangle.rcsmodel",
        r"Data\Plugins\Frontend\Gui\Team_Selection_Definition.xml",
        r"Data\Plugins\Frontend\Gui\Track_Selection_Definition.xml",
        r"Data\FE\Images\file2.gtf",
    ] {
        assert!(!has(&archives, fury), "{fury} is Fury's, not in PSN");
    }
}

/// The eight circuits, each forward and reversed, and all twelve teams.
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn eight_circuits_twice_and_twelve_teams() {
    let Some(archives) = opened() else { return };
    const CIRCUITS: [&str; 8] = [
        "01_vineta_k",
        "02_track",
        "03_track",
        "04_chenghou_project",
        "05_ubermall",
        "10_sebenco_climb",
        "12_sol_2",
        "15_anulpha_pass",
    ];
    for circuit in CIRCUITS {
        for file in ["track.vex", "track_reversed.vex"] {
            let path = format!("/data/environments/{circuit}/{file}");
            assert!(has(&archives, &path), "{path}");
        }
    }
    const TEAMS: [&str; 12] = [
        "ag_systems",
        "assegai",
        "auricom",
        "egx",
        "feisar",
        "goteki",
        "harimau",
        "icaras",
        "mirage",
        "piranha",
        "qirex",
        "triakis",
    ];
    for team in TEAMS {
        let path = format!("/data/ships/{team}/handlingstats.xml");
        assert!(has(&archives, &path), "{path}");
    }
}

/// The roster the front end serves is `DATA03`'s twelve-team definition, not
/// `DATA02`'s older eight: the order of the mounts decides it
/// (`oag_hd::names::FRONT_END_PLUGIN_DEFINITION`).
#[test]
#[ignore = "needs the PSN install in data/extracted/ps3/hd-psn-eu"]
fn the_served_definition_lists_twelve_teams_and_sixteen_circuits() {
    let Some(mut archives) = opened() else { return };
    let name = oag_hd::names::FRONT_END_PLUGIN_DEFINITION;
    let holder = archives.locate(name).expect("the definition resolves");
    assert!(
        holder.to_ascii_lowercase().ends_with("data03.psarc"),
        "{holder}"
    );
    let text = String::from_utf8_lossy(&archives.read_name(name).unwrap()).into_owned();
    assert_eq!(text.matches("<PI_Team ").count(), 12);
    assert_eq!(text.matches("<PI_Track ").count(), 16);
}
