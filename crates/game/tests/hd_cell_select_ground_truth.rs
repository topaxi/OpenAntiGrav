//! Wipeout HD/Fury's `Cell Selection`, against the disc: the grid's flyer back,
//! the four emblems, the corner marks and the barcode all reach the sheet and
//! the draw list.
//!
//! The picture of the screen is `data/scratch/hd-cell-select/` and
//! `docs/ui/campaign-screens.md`, "`Cell Selection`'s card, field, icons and
//! brackets".
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_cell_select_ground_truth)'
//! ```

use oag_hud::sprite::Sheet;
use oag_ui::frontend::Draw;
use oag_ui::language::{CircuitNames, StringTable};
use oag_ui_screens::campaign::hd::{CellArt, cell_brackets, cell_emblems, hd_cell_draw_list};
use oag_ui_screens::campaign::{CellSelection, GridSummary};
use oag_ui_screens::picker::FaceScales;

struct Opened {
    campaign: oag_game::campaign::Campaign,
}

fn open() -> Option<Opened> {
    let source = oag_testdata::exact("data/images/hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&source.display().to_string()).expect("open hd");
    let xml = String::from_utf8(
        archives
            .read_name(oag_hd::TITLE.plugin_definition)
            .expect("the plugin definition"),
    )
    .expect("plain text");
    let tracks = oag_raceplay::catalogue::tracks(&xml);
    assert!(!tracks.is_empty(), "the catalogue names circuits");
    let base = Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &StringTable::default(),
        FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_hd::TITLE,
        &tracks,
    )
    .expect("the campaign loads");
    Some(Opened { campaign })
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_grids_back_and_every_cells_circuit_emblem_is_loaded() {
    let Some(opened) = open() else { return };
    let campaign = &opened.campaign;
    let flyers = campaign.flyers.as_ref().expect("the flyers decode");
    for grid in &campaign.grids {
        let name = grid
            .flyer_name
            .as_deref()
            .expect("every grid names a flyer");
        assert!(
            flyers.has(&oag_game::flyer::Flyers::cell_show(name)),
            "{name}'s back card"
        );
        for cell in &grid.cells {
            let id = cell.track.as_deref().unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let src = campaign
                .circuit_emblems
                .get(&id.to_lowercase())
                .unwrap_or_else(|| panic!("{} on {id} has no emblem", cell.name));
            assert!(campaign.sprites.get(src).is_some(), "{src} is on the sheet");
        }
    }
    for src in cell_emblems::sheet_sources().into_iter().chain([
        cell_brackets::SRC.to_string(),
        cell_emblems::BARCODE_SRC.to_string(),
    ]) {
        assert!(
            campaign.sprites.get(&src).is_some(),
            "{src} is on the sheet"
        );
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_first_cell_draws_four_emblems_and_a_frame_of_corner_marks() {
    let Some(opened) = open() else { return };
    let campaign = &opened.campaign;
    let grid = campaign.grids.get(8).expect("Fury's first grid");
    let model = CellSelection::new(grid.cells.clone());
    let skin = oag_ui::menu::Skin::new(
        oag_hd::frontend::MENU_SKIN,
        oag_display::space::Space::HD,
        33.0,
    );
    let layers = hd_cell_draw_list(
        &model,
        &campaign.cell_layout,
        &skin,
        &oag_ui::menu::Frame::default(),
        &StringTable::default(),
        &CircuitNames::default(),
        0,
        8,
        &GridSummary::from_grid(grid),
        &CellArt {
            next_flyer: None,
            fury: true,
            track_emblems: &campaign.circuit_emblems,
        },
        None,
        false,
        &|src| campaign.sprites.get(src),
        &[],
    );
    let starts_at = |src: &str| {
        let placed = campaign.sprites.get(src).expect("on the sheet");
        layers.body.iter().any(|draw| {
            matches!(draw, Draw::Sprite { uv, .. }
                if uv[0] == placed.x as f32 && uv[1] == placed.y as f32)
        })
    };
    let first = model.selected().expect("a first cell");
    let emblem = campaign
        .circuit_emblems
        .get(&first.track.as_deref().unwrap_or("").to_lowercase())
        .expect("its circuit");
    assert!(starts_at(emblem), "the circuit's emblem");
    assert!(
        starts_at(r"Data\FE\Images\singlerace_bw.gtf"),
        "a single race"
    );
    assert!(starts_at(r"Data\FE\Images\weaponson_bw.gtf"), "weapons on");
    let marks = layers
        .body
        .iter()
        .filter(|draw| {
            let placed = campaign
                .sprites
                .get(cell_brackets::SRC)
                .expect("on the sheet");
            matches!(draw, Draw::Sprite { uv, .. }
                if (uv[0] - placed.x as f32).abs() <= 16.0
                    && uv[1].abs() >= placed.y as f32
                    && uv[2].abs() == 16.0)
        })
        .count();
    assert!(
        marks >= 4 * 4,
        "four marks on each of the screen's brackets: {marks}"
    );
}

/// Every cell of all sixteen grids: each emblem the screen can name for it is on
/// the sheet, and the only cells that name none are the ones documented as having
/// no such file.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_cell_of_every_grid_names_emblems_that_are_on_the_sheet() {
    let Some(opened) = open() else { return };
    let campaign = &opened.campaign;
    let mut cells = 0;
    let mut no_event_icon = Vec::new();
    for grid in &campaign.grids {
        for cell in &grid.cells {
            cells += 1;
            for slot in [
                "Event Emblem",
                "Track Emblem",
                "Speed Class Emblem",
                "Weapons Emblem",
            ] {
                match cell_emblems::source(slot, cell, &campaign.circuit_emblems) {
                    Some(src) => assert!(
                        campaign.sprites.get(&src).is_some(),
                        "{}: {slot} names {src}, which is not on the sheet",
                        cell.name
                    ),
                    None => match slot {
                        "Event Emblem" => {
                            no_event_icon.push(format!("{} {:?}", cell.name, cell.mode));
                        }
                        "Speed Class Emblem" => {
                            // A class that is not one of the four speed classes: Zone's,
                            // Detonator's (`Zone`) and NitroBattle's own.
                            assert!(
                                ["Zone", "NitroBattle"].contains(&cell.class.as_str()),
                                "{} class {:?}",
                                cell.name,
                                cell.class
                            );
                        }
                        "Track Emblem" => assert!(
                            cell.track.as_deref().is_none_or(str::is_empty)
                                || cell.mode == oag_tables::race_campaign::Mode::Tournament,
                            "{} on {:?} has no circuit emblem",
                            cell.name,
                            cell.track
                        ),
                        other => panic!("{}: {other} names nothing", cell.name),
                    },
                }
            }
        }
    }
    assert!(cells >= 160, "{cells} cells walked");
    assert!(
        no_event_icon
            .iter()
            .all(|cell| cell.contains("NitroBattle")),
        "only NitroBattle has no mode icon: {no_event_icon:?}"
    );
}
