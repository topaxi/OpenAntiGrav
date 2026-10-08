//! A mouse pointed at a hexagon on `Cell Selection` lands on that hexagon, on
//! the two titles that draw HD's hex sprites (Omega and Wipeout HD/Fury),
//! against the disc.
//!
//! Their hex sprites are 128x64 power-of-two textures holding a 72x62 hexagon,
//! and hit-testing the texture's own rectangle put every target 28 units right
//! of the art and far taller than it, so a hover on `grid0_3_2` selected its
//! padlocked neighbour `grid0_2_2` and a confirm did nothing. The premise is
//! measured below rather than assumed; the test then drives the same targets
//! the live mouse path builds (`oag_game::campaign::hit`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(campaign_pointer_ground_truth)'
//! ```

use oag_ui::language::StringTable;
use oag_ui_screens::campaign::pointer::{self, What};
use oag_ui_screens::campaign::{CellSelection, Layout};
use oag_ui_screens::picker::FaceScales;

use oag_hud::sprite::Sheet;

const OUTLINE: &str = r"Data\FE\Images\Hexagon_HD.mip";

struct Loaded {
    campaign: oag_game::campaign::Campaign,
}

fn open_omega() -> Option<Loaded> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    let mut archives = oag_omega::open(&source.display().to_string()).expect("open omega");
    Some(load(&mut archives, oag_omega::TITLE))
}

fn open_hd() -> Option<Loaded> {
    let source = oag_testdata::exact("data/images/hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&source.display().to_string()).expect("open hd");
    Some(load(&mut archives, oag_hd::TITLE))
}

fn load(archives: &mut oag_assets::Archives, title: &'static oag_title::Title) -> Loaded {
    load_on(archives, title, [1920.0, 1080.0])
}

fn load_on(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    grid: [f32; 2],
) -> Loaded {
    let base = Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        archives,
        &StringTable::default(),
        FaceScales::default(),
        grid,
        &base,
        &[],
        title,
        &[],
    )
    .expect("the campaign loads");
    Loaded { campaign }
}

/// Where each occupied cell's visible hexagon is drawn, and its circumradius,
/// worked out from the authored `Outline_x_y` widget and the decoded pixels -
/// independently of the targets under test.
fn drawn_hexagons(
    campaign: &oag_game::campaign::Campaign,
    model: &CellSelection,
) -> Vec<(usize, [f32; 2], f32)> {
    let layout: &Layout = &campaign.cell_layout;
    let mut out = Vec::new();
    for (index, cell) in model.cells().iter().enumerate() {
        let Some((x, y)) = cell.grid_coords() else {
            continue;
        };
        let name = format!("Outline_{x}_{y}");
        let image = layout
            .screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some(name.as_str()))
            .unwrap_or_else(|| panic!("{name} is authored"));
        let [ex, ey, ew, eh] = campaign
            .sprites
            .opaque_extent(&image.src)
            .expect("the hex art has visible pixels");
        out.push((
            index,
            [image.x + ex + ew / 2.0, image.y + ey + eh / 2.0],
            ew / 2.0,
        ));
    }
    out
}

/// HD's and Omega's own measured premise: the texture is padded well past its art.
fn the_hex_texture_is_padded_past_its_art(loaded: &Loaded) {
    let campaign = &loaded.campaign;
    let outline = campaign
        .sprites
        .get(OUTLINE)
        .expect("Hexagon_HD.mip is on the sheet");
    let art = campaign
        .sprites
        .opaque_extent(OUTLINE)
        .expect("Hexagon_HD.mip has visible pixels");
    assert!(
        art[2] < outline.width as f32 * 0.75,
        "the premise: the texture ({}x{}) is padded well past its art ({}x{})",
        outline.width,
        outline.height,
        art[2],
        art[3]
    );
}

fn every_cell_answers_a_pointer_on_its_own_hexagon(loaded: &Loaded) {
    let campaign = &loaded.campaign;
    let mut checked = 0;
    for (index, grid) in campaign.grids.iter().enumerate() {
        let model = CellSelection::new(grid.cells.clone());
        let targets =
            oag_game::campaign::hit::cell_targets(&model, &campaign.cell_layout, &campaign.sprites);
        for (cell, centre, radius) in drawn_hexagons(campaign, &model) {
            // The centre and six points at 0.7 of the circumradius around it:
            // each is well inside its own hexagon and, on a staggered grid,
            // nearer a neighbour's region than a wrongly sized target would be.
            let mut probes = vec![(centre[0], centre[1])];
            for step in 0..6 {
                let angle = step as f32 * std::f32::consts::FRAC_PI_3;
                probes.push((
                    centre[0] + radius * 0.7 * angle.cos(),
                    centre[1] + radius * 0.7 * angle.sin(),
                ));
            }
            for at in probes {
                let hit = pointer::hit(&targets, at).map(|target| target.what);
                assert_eq!(
                    hit,
                    Some(What::Hex(cell)),
                    "grid {index}: a pointer at {at:?}, on {}'s own hexagon, landed elsewhere",
                    model.cells()[cell].name
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no cell was probed");
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_cells_answer_a_pointer_on_their_own_hexagon() {
    let Some(omega) = open_omega() else { return };
    the_hex_texture_is_padded_past_its_art(&omega);
    every_cell_answers_a_pointer_on_its_own_hexagon(&omega);
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_cells_answer_a_pointer_on_their_own_hexagon() {
    let Some(hd) = open_hd() else { return };
    the_hex_texture_is_padded_past_its_art(&hd);
    every_cell_answers_a_pointer_on_its_own_hexagon(&hd);
}

/// Pulse authors its hexes at an explicit size in a 480x272 grid, and the
/// trimmed targets must still answer on every cell: this title is the reference
/// and was not the one that was broken.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_cells_answer_a_pointer_on_their_own_hexagon() {
    let Some(source) = oag_testdata::exact("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let opened =
        oag_source::title::open_source(&source.display().to_string(), Vec::new(), Vec::new())
            .expect("open pulse");
    let mut archives = opened.archives;
    let pulse = load_on(&mut archives, oag_pulse::TITLE, [480.0, 272.0]);
    every_cell_answers_a_pointer_on_its_own_hexagon(&pulse);
}
