//! HD's `Track Select` draws each circuit's own front-end model in its
//! `CIRCUIT MODEL` frame: all twelve rows of the title's table resolve to a
//! scene that builds a non-empty mesh.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^hd_track_model_ground_truth::/)'
//! ```

fn open() -> Option<(oag_assets::Archives, &'static oag_title::Title)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let options = oag_game::boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("hd-track-model-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (_, archives, title) = oag_game::boot::load_shell(&options).expect("HD boots");
    Some((archives, title))
}

#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn every_circuit_of_the_table_builds_a_model() {
    let Some((mut archives, title)) = open() else {
        return;
    };
    let front_end = title.front_end.expect("HD ships a front end");
    assert_eq!(front_end.circuit_models.len(), 12);
    // The wiring, not only the table: the Track screen asks for a mesh and
    // the Team screen does not take the circuit's path.
    assert!(oag_game::preview::draws_mesh(
        title,
        oag_ui_screens::picker::Kind::Track
    ));
    assert!(oag_game::preview::draws_circuit_model(
        title.front_end,
        oag_ui_screens::picker::Kind::Track
    ));
    assert!(!oag_game::preview::draws_circuit_model(
        title.front_end,
        oag_ui_screens::picker::Kind::Ship
    ));
    for row in front_end.circuit_models {
        let location = format!(r"Data\Environments\{}", row.environment);
        let entry = oag_game::preview::track_entry(front_end, &location, false)
            .unwrap_or_else(|| panic!("{location}: no model entry"));
        let mut model = oag_game::preview::model(&mut archives, &entry)
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        assert!(model.indices.len() / 3 > 100, "{entry} is a circuit's mesh");
        // The material is `cf_fetracks`: a ramp, bright at the grazing end
        // and dark face-on, on every circuit's own `fe_grad.gtf`.
        let ramp = oag_game::preview::track_model::Ramp::take(&mut model)
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        let (edge, face) = (ramp.at(0.0), ramp.at(1.0));
        assert!(
            edge[0] > face[0] + 0.2,
            "{entry}: ramp {edge:?} to {face:?}"
        );
        assert!(model.vertices.iter().all(|vertex| vertex.lit == 0.0));
        // What the live screen pays every frame for this circuit.
        let started = std::time::Instant::now();
        let mut vertices = model.vertices.clone();
        ramp.shade(
            &mut vertices,
            oag_core::math::Mat4::from_translation(oag_core::math::Vec3::new(0.0, 0.0, -180.0)),
        );
        eprintln!(
            "{entry}: {} vertices, {} bytes, recolour {:?}",
            vertices.len(),
            std::mem::size_of_val(&vertices[..]),
            started.elapsed()
        );
    }
}

/// The screen's update copies the selected circuit's own `RotX`, `PosY` and `PosZ` over the
/// widget, so the entries the catalogue reads carry them: Vineta K's are `0.35`, `-20` and
/// `-135`, and every circuit that has a model has a pose.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn every_circuit_with_a_model_carries_its_entrys_pose() {
    let Some((mut archives, title)) = open() else {
        return;
    };
    let front_end = title.front_end.expect("HD ships a front end");
    let law = front_end
        .circuit_turntable
        .expect("HD's update is read: the turntable law");
    assert_eq!((law.step, law.origin_y), (0.01, 440.0));
    let blob = archives
        .read_name(title.plugin_definition)
        .expect("plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("expand the definition");
    let tracks = oag_raceplay::catalogue::tracks(&xml);
    let vineta = tracks
        .iter()
        .find(|track| track.location.ends_with("01_Vineta_K") && !track.reversed)
        .expect("Vineta K");
    assert_eq!(
        vineta.model_pose,
        Some(oag_title::CircuitPose {
            rot_x: 0.35,
            pos_y: -20.0,
            pos_z: -135.0
        })
    );
    for row in front_end.circuit_models {
        let track = tracks
            .iter()
            .find(|track| {
                !track.reversed
                    && track
                        .location
                        .rsplit('\\')
                        .next()
                        .is_some_and(|folder| folder.eq_ignore_ascii_case(row.environment))
            })
            .unwrap_or_else(|| panic!("{}: no forward entry", row.environment));
        let pose = track
            .model_pose
            .unwrap_or_else(|| panic!("{}: the entry authors no pose", row.environment));
        assert!(
            (0.2..0.6).contains(&pose.rot_x) && (-300.0..-100.0).contains(&pose.pos_z),
            "{}: {pose:?}",
            row.environment
        );
    }
}
