//! HD's `Team Selection` draws the race hull in its `ShipModel` frame: the file
//! resolves, and framing it from its bulk gives a hull-sized sphere, where its
//! raw bound is inflated by one stray vertex.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^hd_ship_preview_ground_truth::/)'
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
        find_texture_packs: None,
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

/// The preview follows the selected variant: each of a team's three
/// directories carries a hull of its own, and the three are not one mesh.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn each_variant_directory_has_a_hull_of_its_own() {
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
        find_texture_packs: None,
    };
    let (_, mut archives, title) = oag_game::boot::load_shell(&options).expect("HD boots");
    let join = title.race.team_variants_for("Feisar").map(|v| v.join);
    assert!(join.is_some(), "HD offers Feisar a variant table");
    let mut shapes = Vec::new();
    for variant in ["", "_c1", "_n1"] {
        let location =
            oag_game::preview::variant::variant_location(join, r"Data\Ships\Feisar", variant);
        let entry = oag_game::preview::ship_entry(title, &location, oag_race::Mode::default());
        let (model, loaded) =
            oag_game::preview::variant::model_or_default(&mut archives, &entry, None, None)
                .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        assert_eq!(loaded, entry, "no fallback for a hull the disc ships");
        shapes.push((model.vertices.len(), model.indices.len()));
    }
    shapes.sort_unstable();
    shapes.dedup();
    assert_eq!(
        shapes.len(),
        3,
        "three directories, three hulls: {shapes:?}"
    );
}

/// Omega's `Team Selection` draws its craft too: every team's race hull is a PS4
/// `.rcsmodel` that `preview::model` (the PS3 and PSP path) does not read, so the
/// preview goes through `psp2_scene::model`; each hull frames to a hull-sized sphere
/// with its own textures (HD's own sizes, which is what lets HD's pose frame it),
/// and the screen is drawn at the pose the title names, not the widget's own
/// `z=-2.05`. Dropping `ship_preview_hull` or the pose fails the first two
/// assertions; routing the hull back through `preview::model` fails the load.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_draws_every_teams_ps4_hull_at_the_pose_it_borrows() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let options = oag_game::boot::Options {
        language: None,
        source: path.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-ship-preview-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (shell, mut archives, title) = oag_game::boot::load_shell(&options).expect("Omega boots");
    let front_end = title.front_end.expect("a front end");
    assert_eq!(front_end.ship_preview_hull, Some("ship.vex"));
    let pose = front_end.ship_preview_pose.expect("a borrowed pose");
    let widget = shell
        .ship_select
        .as_ref()
        .and_then(|layout| layout.hd.as_deref())
        .and_then(|screen| screen.ship_model)
        .expect("the ShipModel widget");
    assert_eq!((widget.origin, widget.z), (pose.origin, pose.z));
    assert_eq!(
        (widget.rot_x, widget.rot_y),
        (0.4, -0.5),
        "rotations as authored"
    );

    assert_eq!(
        shell.teams.len(),
        19,
        "twelve HD-era teams, the two Tigrons and the five 2048-era teams"
    );
    // Every craft a team offers: the HD-era directories, and the 2048-era teams' four
    // numbered ones, whose hull is `<Team>\<n>\ship.vex` (the bare team directory has none).
    let mut hulls = 0;
    for team in &shell.teams {
        let table = title.race.team_variants_for(&team.id);
        let suffixes: Vec<&str> =
            table.map_or(vec![""], |t| t.variants.iter().map(|v| v.suffix).collect());
        for suffix in suffixes {
            let location = oag_game::preview::variant::variant_location(
                table.map(|t| t.join),
                &team.location,
                suffix,
            );
            let entry = oag_game::preview::ship_entry(title, &location, oag_race::Mode::default());
            let mut model = oag_game::preview::psp2_scene::model(&mut archives, &entry)
                .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
            assert!(
                model.indices.len() / 3 > 5_000,
                "{entry} has a hull's triangles"
            );
            assert!(
                model.textures.iter().flatten().count() > 0,
                "{entry} draws with its own textures"
            );
            oag_game::preview::frame_hull(&mut model);
            assert!(
                (3.0..30.0).contains(&model.radius),
                "{entry}: framed radius {}",
                model.radius
            );
            hulls += 1;
        }
    }
    assert_eq!(hulls, 12 * 3 + 2 + 5 * 4, "every craft of every team");

    // The preview follows the chosen model: Feisar's three directories are three hulls.
    let join = title.race.team_variants_for("Feisar").map(|v| v.join);
    let mut shapes = Vec::new();
    for variant in ["", "_c1", "_n1"] {
        let location = oag_game::preview::variant::variant_location(
            join,
            r"Data\art\published\hdships\Feisar",
            variant,
        );
        let entry = oag_game::preview::ship_entry(title, &location, oag_race::Mode::default());
        let (model, loaded) =
            oag_game::preview::variant::model_or_default(&mut archives, &entry, None, None)
                .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
        assert_eq!(loaded, entry);
        shapes.push((model.vertices.len(), model.indices.len()));
    }
    shapes.sort_unstable();
    shapes.dedup();
    assert_eq!(
        shapes.len(),
        3,
        "three directories, three hulls: {shapes:?}"
    );
}

/// The craft is framed by the screen's own fit, not the widget's authored pose:
/// the title names the fit, every model's `<FE>` authors the length it fits to,
/// and Feisar's hull then lands where an RPCS3 frame of the screen at rest has
/// it (Safe Area 96, the hull's silhouette 750 to 1637 across and 256 to 639
/// down a 1920 by 1080 frame). The authored pose put it 946 to 1701 across.
/// Four more teams' concept hulls, of four other lengths, land where their own
/// rest frames have them. Dropping `ship_preview_fit`, the length off `<FE>`
/// (or holding it at one value), or the Safe Area scale each moves a
/// silhouette out of the tolerance.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn the_screens_own_fit_lands_feisar_where_the_original_draws_it() {
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
        find_texture_packs: None,
    };
    let (shell, mut archives, title) = oag_game::boot::load_shell(&options).expect("HD boots");
    let fit = title
        .front_end
        .and_then(|front_end| front_end.ship_preview_fit)
        .expect("HD names its screen's fit");
    let widget = shell
        .ship_select
        .as_ref()
        .and_then(|layout| layout.hd.as_deref())
        .and_then(|screen| screen.ship_model)
        .expect("the ShipModel widget");
    assert!(widget.obey_safe_zone, "ObeySafeZone is authored");

    // Every model that authors ratings authors a length, eight values in all.
    let mut lengths: Vec<u16> = shell
        .teams
        .iter()
        .flat_map(|team| &team.models)
        .map(|model| {
            model
                .length_milli
                .unwrap_or_else(|| panic!("{} authors no length", model.model_location))
        })
        .collect();
    lengths.sort_unstable();
    lengths.dedup();
    assert_eq!(lengths, [3200, 3800, 3900, 4000, 4100, 4200, 4400, 4500]);

    let feisar = shell
        .teams
        .iter()
        .find(|team| team.id == "Feisar")
        .expect("Feisar");
    let length = feisar.variant_stats(oag_title::VariantJoin::Suffix, [""])[0]
        .and_then(|stats| stats.length_milli)
        .expect("Feisar's classic hull authors a length");
    assert_eq!(length, 3800);
    let box_z = oag_game::preview::ship_model::box_z(&fit, false, Some(f32::from(length) / 1000.0))
        .expect("a box");
    let space = oag_display::space::Space::HD;
    let framing = oag_game::preview::ship_model::rest(&fit, space.display_aspect, box_z);
    assert!((framing.depth - 12.05).abs() < 0.06, "{framing:?}");

    let drawn = silhouette(
        &mut archives,
        r"Data\Ships\Feisar\ship.vex",
        (&widget, &fit, box_z),
        0.96,
    );
    for (ours, theirs) in drawn.into_iter().zip([750.0, 1637.0, 256.0, 639.0]) {
        assert!((ours - theirs).abs() < 5.0, "{drawn:?}");
    }

    // The depth follows each model's own length: four more teams' concept hulls,
    // lengths 4.5, 4.4, 3.9 and 3.2 (the last rests on the floor), against RPCS3
    // rest frames of each. Those frames are window grabs at another scale, so
    // the edges are mapped back to the unscaled grid by the screen's two rules,
    // and only the edges the frame shows against black are held.
    let concept = |team: &str| {
        let team = shell
            .teams
            .iter()
            .find(|entry| entry.id == team)
            .unwrap_or_else(|| panic!("{team}"));
        let length = team.variant_stats(oag_title::VariantJoin::Suffix, ["_c1"])[0]
            .and_then(|stats| stats.length_milli)
            .unwrap_or_else(|| panic!("{}: no length", team.id));
        f32::from(length) / 1000.0
    };
    // (team, directory, left, right, top, bottom).
    type Edges = [Option<f32>; 4];
    let frames: [(&str, &str, Edges); 4] = [
        (
            "Qirex",
            "Qirex_c1",
            [Some(786.0), None, Some(286.0), Some(706.0)],
        ),
        (
            "Assegai",
            "Assegai_c1",
            [Some(755.0), Some(1627.0), Some(283.0), None],
        ),
        (
            "Piranha",
            "piranha_c1",
            [None, Some(1665.0), Some(268.0), None],
        ),
        (
            "Icaras",
            "Icaras_c1",
            [None, Some(1594.0), Some(263.0), None],
        ),
    ];
    for (team, directory, edges) in frames {
        let box_z =
            oag_game::preview::ship_model::box_z(&fit, false, Some(concept(team))).expect("a box");
        let entry = format!(r"Data\Ships\{directory}\ship.vex");
        let drawn = silhouette(&mut archives, &entry, (&widget, &fit, box_z), 1.0);
        for (ours, theirs) in drawn.into_iter().zip(edges) {
            if let Some(theirs) = theirs {
                assert!((ours - theirs).abs() < 6.0, "{entry}: {drawn:?}");
            }
        }
    }
}

/// The left, right, top and bottom of a hull's vertices on a 1920 by 1080 grid
/// as the screen's fit draws it inside `safe_area`.
fn silhouette(
    archives: &mut oag_assets::Archives,
    entry: &str,
    (widget, fit, box_z): (
        &oag_ui_screens::picker::hd::ShipModel,
        &oag_title::ShipFit,
        [f32; 2],
    ),
    safe_area: f32,
) -> [f32; 4] {
    let model =
        oag_game::preview::model(archives, entry).unwrap_or_else(|e| panic!("{entry}: {e:#}"));
    let space = oag_display::space::Space::HD;
    let framing = oag_game::preview::ship_model::rest(fit, space.display_aspect, box_z);
    let (view_projection, placed) =
        oag_game::preview::ship_model::framed(widget, Some((fit, framing)), space, safe_area);
    let matrix = view_projection * placed;
    let (mut across, mut down): (Vec<f32>, Vec<f32>) = model
        .vertices
        .iter()
        .map(|vertex| {
            let at = matrix.project_point3(oag_core::math::Vec3::from(vertex.position));
            ((at.x * 0.5 + 0.5) * 1920.0, (0.5 - at.y * 0.5) * 1080.0)
        })
        .unzip();
    across.sort_by(f32::total_cmp);
    down.sort_by(f32::total_cmp);
    // The stray vertex `frame_hull` documents is one of thousands.
    let skip = across.len() / 2000;
    [
        across[skip],
        across[across.len() - 1 - skip],
        down[skip],
        down[down.len() - 1 - skip],
    ]
}
