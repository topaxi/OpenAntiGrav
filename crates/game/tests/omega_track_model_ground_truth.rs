//! Omega's `Track Select` draws each circuit's own front-end model in its
//! `CIRCUIT MODEL` frame: every row of the title's table resolves to a PS4
//! scene that builds a non-empty mesh and a `FrontEndConstantFranelBlend`
//! ramp read off the model's own uniforms, and every circuit the screen lists
//! either has a row or is named below as having no model on the disc.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_track_model_ground_truth)'
//! ```

use std::path::PathBuf;

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

fn open(
    source: &std::path::Path,
) -> (
    oag_game::boot::Shell,
    oag_assets::Archives,
    &'static oag_title::Title,
) {
    let options = oag_game::boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-track-model-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    oag_game::boot::load_shell(&options).expect("Omega boots")
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_circuit_of_the_table_builds_a_model_and_a_ramp() {
    let Some(source) = omega() else {
        return;
    };
    let (shell, mut archives, title) = open(&source);
    let front_end = title.front_end.expect("Omega ships a front end");
    assert_eq!(front_end.circuit_models.len(), 26);
    // The wiring, not only the table: the screen reads, asks for a mesh on the
    // Track screen only, and carries the widget's own pose.
    let layout = shell.track_select.as_ref().expect("Track Creation reads");
    let widget = layout
        .hd_track
        .as_ref()
        .and_then(|screen| screen.model)
        .expect("the screen authors a TrackModel");
    assert_eq!(widget.origin, [960.0, 540.0]);
    assert_eq!(widget.offset, [0.525, 0.125]);
    assert_eq!(widget.z, -2.05);
    assert_eq!(widget.ortho_scale, [0.012; 3]);
    assert!(oag_game::preview::draws_circuit_model(
        title.front_end,
        oag_ui_screens::picker::Kind::Track
    ));
    assert!(!oag_game::preview::draws_circuit_model(
        title.front_end,
        oag_ui_screens::picker::Kind::Ship
    ));
    for row in front_end.circuit_models {
        let folder = if ["altima", "arena", "bridge", "cathedral", "mall"]
            .contains(&row.environment)
            || ["park", "sol", "square", "subway", "tower"].contains(&row.environment)
        {
            "Environments2048"
        } else {
            "Environments"
        };
        let location = format!(r"Data\{folder}\{}", row.environment);
        let entry = oag_game::preview::track_entry(front_end, &location, false)
            .unwrap_or_else(|| panic!("{location}: no model entry"));
        let mut model = oag_game::preview::psp2_scene::circuit_model(&mut archives, &entry)
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        assert!(model.indices.len() / 3 > 100, "{entry} is a circuit's mesh");
        let ramp = oag_game::preview::track_model::Ramp::of(&mut archives, &entry, &mut model)
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        // The fresnel term: dark face-on, bright at the grazing end.
        let (edge, face) = (ramp.at(0.0), ramp.at(1.0));
        assert!(
            edge[0] > face[0] + 0.5,
            "{entry}: ramp {edge:?} to {face:?}"
        );
        assert!(model.vertices.iter().all(|vertex| vertex.lit == 0.0));
        eprintln!(
            "{entry}: {} vertices, ramp {face:?} -> {edge:?}",
            model.vertices.len()
        );
    }
    // Every circuit the screen lists has a model, and says so.
    let mut without: Vec<String> = shell
        .tracks
        .iter()
        .filter(|track| !track.reversed)
        .filter(|track| front_end.circuit_model(&track.location).is_none())
        .map(|track| format!("{} ({})", track.id, track.location))
        .collect();
    without.sort();
    assert!(without.is_empty(), "circuits with no model: {without:?}");
}
