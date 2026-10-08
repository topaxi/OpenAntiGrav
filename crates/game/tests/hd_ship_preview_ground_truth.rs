//! HD's `Team Selection` draws the race hull in its `ShipModel` frame: the file
//! resolves, and framing it from its bulk gives a hull-sized sphere, where its
//! raw bound is inflated by one stray vertex.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_ship_preview_ground_truth)'
//! ```

#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn the_race_hull_frames_to_a_hull_sized_sphere() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let options = oag_game::boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-hd-ship-preview-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (_, mut archives, title) = oag_game::boot::load_shell(&options).expect("HD boots");
    assert_eq!(
        title
            .front_end
            .and_then(|front_end| front_end.ship_preview_hull),
        Some("ship.vex")
    );
    for team in ["Assegai", "Feisar"] {
        let entry = format!(r"Data\Ships\{team}\ship.vex");
        let mut model = oag_game::preview::model(&mut archives, &entry)
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        assert!(
            model.indices.len() / 3 > 5_000,
            "{entry} has a hull's triangles"
        );
        oag_game::preview::frame_hull(&mut model);
        assert!(
            (3.0..30.0).contains(&model.radius),
            "{entry}: framed radius {}",
            model.radius
        );
    }
}
