//! The 19 pieces an HD craft breaks into are on the disc and load
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The debris throw").
//!
//! **`#[ignore]`d and never run in CI**: needs a decrypted PS3 image in `data/images`. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all -E 'test(/^ship_debris_ground_truth::/)'`.

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_ships_its_19_pieces_and_they_load_with_geometry() {
    let Some(path) = oag_testdata::exact("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let spec = oag_title::destroyed::DestroyedCraft::HD
        .ship_debris
        .expect("HD throws debris");
    let mut report = Vec::new();
    let models = oag_livery::ship_debris::load(
        &mut archives,
        oag_hd::race::DEFAULTS.ships_for("feisar"),
        Default::default(),
        spec,
        &mut report,
    )
    .unwrap_or_else(|| panic!("the pieces load: {report:?}"));
    assert_eq!(models.pieces.len(), spec.pieces, "{report:?}");
    assert!(models.pieces.iter().all(|piece| !piece.is_empty()));
    assert_eq!(
        models.debris.vertices.len() as u32,
        models.stride * oag_livery::ship_debris::COPIES as u32
    );
}
