//! The example mods' campaign (`assets/mods/examples/example-campaign/`, the
//! `mod-examples` submodule, ADR-0077), with the track-only `hillside/` beside it,
//! against Pulse, the one title it is for, on the PSP and the PS2: it compiles, every event's circuit is one the
//! disc offers, every team it names is on the disc's roster, and each cell
//! plans as the race it authors. One test per disc so they run side by side.
//! The catalogue reads no DLC pack, so the example names base-disc teams only
//! (Pulse's Auricom is not on its base disc).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/custom_campaign_ground_truth::/)'
//! ```

use oag_game::campaign::custom::{Custom, Installed};
use oag_game::campaign::launch;

/// The example mods, the campaign and the track-only Hillside it races: the
/// `mod-examples` submodule, or `None` (said on stderr) when it is not checked out.
fn example_mods() -> Option<[oag_mod::Mod; 2]> {
    let dir = oag_testdata::repo_root().join("assets/mods/examples");
    if !dir.join("example-campaign/oag-mod.toml").is_file() {
        eprintln!("skipped: the example mods submodule is not checked out");
        return None;
    }
    Some(["example-campaign", "hillside"].map(|name| {
        oag_mod::Mod::open(&dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e:#}"))
    }))
}

fn check(title: &'static oag_title::Title, source: &std::path::Path) {
    let Some(mods) = example_mods() else {
        return;
    };
    let installed = Installed::for_title(&mods, title, &|_| false);
    assert!(installed.failed.is_empty(), "{:?}", installed.failed);
    let [custom]: [Custom; 1] = installed.campaigns.try_into().expect("one campaign");
    let catalogue = oag_game::remix::catalogue(&source.display().to_string())
        .unwrap_or_else(|e| panic!("cataloguing {}: {e:#}", source.display()));
    let has_team = |team: &str| {
        catalogue
            .teams
            .iter()
            .any(|choice| choice.value.eq_ignore_ascii_case(team))
    };
    let ladder = title
        .race
        .speed_classes
        .map_or(&[][..], |classes| classes.names);
    for grid in &custom.grids {
        for cell in &grid.cells {
            let event = custom.event(&cell.name).expect("every cell has its extras");
            // A track file the mod carries is its own circuit; everything else
            // is the disc's.
            let plan =
                launch::plan_cell_for(cell, "venom", ladder, |_, id| match &event.track_file {
                    Some(file) => Some(file.path.display().to_string()),
                    None => catalogue
                        .track(id)
                        .map(oag_raceplay::catalogue::Track::entry_name),
                })
                .unwrap_or_else(|refusal| panic!("{} on {}: {refusal}", cell.name, title.name));
            assert!(!plan.class_is_fallback, "{}", cell.name);
            for team in event.team.iter().chain(&event.teams) {
                assert!(
                    has_team(team),
                    "{}: {} has no team {team}",
                    cell.name,
                    title.name
                );
            }
        }
    }
    // Every event sits in a slot the title's own `Cell Selection` draws a hex
    // at: a cell anywhere else would never be seen.
    let mut archives =
        oag_source::title::open_source(&source.display().to_string(), Vec::new(), Vec::new())
            .unwrap_or_else(|e| panic!("opening {}: {e:#}", source.display()))
            .archives;
    let disc = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [480.0, 272.0],
        &oag_hud::sprite::Sheet::build(&[], &mut Vec::new()),
        &[],
        title,
        &[],
    )
    .expect("the disc's campaign screens load");
    let slots = disc.cell_layout.hex_slots();
    assert!(!slots.is_empty(), "{} authors no Outline_x_y", title.name);
    for cell in custom.grids.iter().flat_map(|grid| &grid.cells) {
        let at = cell.grid_coords().expect("a slot");
        assert!(
            slots.contains(&at),
            "{}: {} draws no hex at {at:?}",
            cell.name,
            title.name
        );
    }
    // Two track files: Hillside out of `oag.hillside`, and the campaign's own
    // Practice Octagon.
    let files: Vec<&str> = custom
        .events
        .values()
        .filter_map(|event| Some(event.track_file.as_ref()?.id.as_str()))
        .collect();
    assert_eq!(files, ["hillside", "practice-octagon"]);
    // The tournament keeps its three legs, in order.
    let tour = &custom.grids[1].cells[0];
    let plan = launch::plan_cell_for(tour, "venom", ladder, |_, id| {
        catalogue
            .track(id)
            .map(oag_raceplay::catalogue::Track::entry_name)
    })
    .unwrap();
    assert_eq!(plan.mode, oag_race::Mode::Tournament);
    assert_eq!(plan.leg_entries.len(), 3);
    assert_eq!(
        custom.records_title,
        format!("{}/mod:oag.example-campaign/night-run", title.slug())
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_example_campaign_resolves_on_pulse() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    check(oag_pulse::TITLE, &image);
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_example_campaign_resolves_on_ps2_pulse() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    check(oag_pulse::TITLE, &image);
}

/// No other title offers it: the mod is for Pulse alone (its track file borrows
/// Pulse art), Pure draws a list, and 2048 has no hex screens.
#[test]
fn the_example_campaign_is_for_no_other_title() {
    let Some(mods) = example_mods() else {
        return;
    };
    for title in [
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
    ] {
        let installed = Installed::for_title(&mods, title, &|_| true);
        assert!(installed.campaigns.is_empty() && installed.failed.is_empty());
    }
}
