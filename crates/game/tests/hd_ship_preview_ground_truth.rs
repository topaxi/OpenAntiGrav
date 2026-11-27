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
        cache: oag_testdata::scratch::cache("hd-ship-preview-ground-truth"),
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
        cache: oag_testdata::scratch::cache("hd-ship-preview-ground-truth"),
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
            oag_game::preview::variant::model_or_default(&mut archives, &entry, None, None, None)
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
/// with its own textures, and the widget is drawn at the rest pose the screen's update
/// writes over it. Dropping `ship_preview_hull` or the pose fails the first two
/// assertions; routing the hull back through `preview::model` fails the load.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_draws_every_teams_ps4_hull_at_the_updates_rest_pose() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let options = oag_game::boot::Options {
        language: None,
        source: path.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("omega-ship-preview-ground-truth"),
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
    let pose = front_end.ship_preview_pose.expect("the update's rest pose");
    let widget = shell
        .ship_select
        .as_ref()
        .and_then(|layout| layout.hd.as_deref())
        .and_then(|screen| screen.ship_model)
        .expect("the ShipModel widget");
    assert_eq!(
        widget.origin,
        [960.0, 540.0],
        "the origin stays as authored"
    );
    assert_eq!(widget.near, 0.1, "so does nearZ");
    assert_eq!([widget.rot_x, widget.rot_y], pose.rotation);
    assert_eq!(
        [widget.position[0], widget.position[1], widget.z],
        pose.position
    );
    assert_eq!(
        (widget.scale, widget.fov_degrees, widget.zoom),
        (pose.scale, pose.fov_degrees, pose.zoom)
    );

    assert_eq!(
        shell.teams.len(),
        19,
        "twelve HD-era teams, the two Tigrons and the five 2048-era teams"
    );
    the_model_row_follows_the_discs_unlock_rows(&shell, &mut archives, title);
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
            oag_game::preview::variant::model_or_default(&mut archives, &entry, None, None, None)
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
        cache: oag_testdata::scratch::cache("hd-ship-preview-ground-truth"),
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
    the_model_row_follows_the_discs_unlock_rows(&shell, &mut archives, title);

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
    let box_z = oag_game::preview::ship_model::box_z(&fit, None, Some(f32::from(length) / 1000.0))
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
            oag_game::preview::ship_model::box_z(&fit, None, Some(concept(team))).expect("a box");
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
    let (view_projection, placed) = oag_game::preview::ship_model::framed(
        widget,
        Some((fit, framing)),
        &oag_game::preview::ship_model::control::Craft::default(),
        space,
        safe_area,
    );
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

/// A still of `team`'s Ship Select `step_seconds` after a step, as RGBA.
fn ship_select_still(
    image: &std::path::Path,
    team: &str,
    step_seconds: &str,
) -> oag_texture::replacer::Image {
    let dir = oag_testdata::scratch::dir("hd-ship-swap-ground-truth");
    std::fs::create_dir_all(dir.join("config/oag")).expect("a temp directory");
    std::fs::write(
        dir.join("config/oag/settings.toml"),
        format!("[race]\nteam = \"{team}\"\n"),
    )
    .expect("a settings file");
    let out = dir.join(format!("{step_seconds}.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "1920x1080",
            "--menu-page",
            "ship-select",
        ])
        .args([
            "--menu-picker-seconds",
            "5",
            "--menu-picker-step-seconds",
            step_seconds,
        ])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page ship-select");
    let still = oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG");
    let _ = std::fs::remove_dir_all(&dir);
    still
}

/// Where the hull's left and right edges are, pixels: the 2nd and 98th percentile of x of what `still` draws beyond
/// `empty` (the same page while the craft is cut, the gap after a step) inside `SHIP MODEL`'s
/// frame, `[left, top, right, bottom]` in the still's own pixels. A percentile, not an edge, so
/// the backdrop's embers drifting between two frames do not widen it.
fn hull_edges(
    still: &oag_texture::replacer::Image,
    empty: &oag_texture::replacer::Image,
    frame: [u32; 4],
) -> [u32; 2] {
    let mut across = Vec::new();
    for y in frame[1]..frame[3] {
        for x in frame[0]..frame[2] {
            let at = ((y * still.width + x) * 4) as usize;
            if (0..3).any(|c| still.rgba[at + c].abs_diff(empty.rgba[at + c]) > 40) {
                across.push(x);
            }
        }
    }
    assert!(across.len() > 1000, "no hull in the frame");
    across.sort_unstable();
    [
        across[across.len() * 2 / 100],
        across[across.len() * 98 / 100],
    ]
}

/// How wide the hull is: [`hull_edges`]' span.
fn hull_width(
    still: &oag_texture::replacer::Image,
    empty: &oag_texture::replacer::Image,
    frame: [u32; 4],
) -> u32 {
    let [left, right] = hull_edges(still, empty, frame);
    right - left
}

/// The live screen's swap through the real binary: the first frames after a step draw the new
/// hull at the last model's depth, so it is larger than at rest when the last model rested
/// nearer and smaller when it rested farther, and it has settled within a second and a half.
/// The roster steps from EGX (`<FE length>` 3.8) to Assegai (4.4), and from Qirex (4.5) to
/// Piranha (3.9). A fixed start scale cannot be both.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images and a GPU adapter"]
fn a_step_starts_the_hull_at_the_last_models_depth_and_settles() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    for (team, larger) in [("Assegai", true), ("Piranha", false)] {
        let empty = ship_select_still(&image, team, "0.03");
        let width = |step: &str| {
            f64::from(hull_width(
                &ship_select_still(&image, team, step),
                &empty,
                [740, 200, 1660, 740],
            ))
        };
        let (first, settled) = (width("0.07"), width("3"));
        let late = width("1.5") / settled;
        let early = first / settled;
        eprintln!(
            "{team}: {first} px first frame, {settled} settled, ratio {early:.3}, 1.5 s {late:.3}"
        );
        assert_eq!(
            early > 1.0,
            larger,
            "{team}: {first} px against {settled} settled"
        );
        assert!(
            (early - 1.0).abs() > 0.02,
            "{team}: {first} px against {settled} settled"
        );
        assert!(
            (late - 1.0).abs() < 0.01,
            "{team}: {late:.3} a second and a half on"
        );
    }
}

/// The cut after a step is 3 screen frames from the selection (`hull_swap::GAP_FRAMES`; the
/// original's craft is absent at RAM frames 3 to 5 and back at 6, `team-selection-model.md`,
/// "The step against RAM"): the picture 2 frames after a step draws no craft, 3 frames after it
/// draws the new one. The frame-6 step pair reads this frame.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images and a GPU adapter"]
fn a_step_cuts_the_craft_for_three_frames_and_not_four() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let frames = |n: f32| format!("{:.5}", n / 60.0);
    let empty = ship_select_still(&image, "Harimau", "0.001");
    let drawn = |step: &str| {
        let still = ship_select_still(&image, "Harimau", step);
        let frame = [740, 200, 1660, 740];
        let mut changed = 0;
        for y in frame[1]..frame[3] {
            for x in frame[0]..frame[2] {
                let at = ((y * still.width + x) * 4) as usize;
                changed += usize::from(
                    (0..3).any(|c| still.rgba[at + c].abs_diff(empty.rgba[at + c]) > 40),
                );
            }
        }
        changed
    };
    let (two, three) = (drawn(&frames(2.0)), drawn(&frames(3.0)));
    eprintln!("pixels beyond the empty frame: {two} at 2 frames, {three} at 3");
    assert!(two < 1000, "the craft is still cut at 2 frames: {two}");
    assert!(three > 20_000, "the craft is back at 3 frames: {three}");
}

/// The live screen through `--drive`, the real session ticking the picker, a press of the pad
/// and frames read back: after a step right (Assegai to Mirage on a fresh profile) the settled
/// hull is the width the capture draws for Mirage at rest. A screen that does not tick the fit
/// draws the hull on the widget's own pose, a different size.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images and a GPU adapter"]
fn the_live_screen_draws_the_hull_where_the_fit_rests_it() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let dir = oag_testdata::scratch::dir("hd-ship-swap-live");
    std::fs::create_dir_all(dir.join("mods")).expect("a scratch directory");
    let shot = |name: &str| dir.join(format!("{name}.png"));
    let script = format!(
        "boot; select racebox; press cross; until page=race; select launch_race; \
         press cross; until screen=picker; press cross; until screen.id=ship-select; \
         until opening_reads=landed; ticks 120; press right; screenshot {gap}; \
         ticks 240; screenshot {settled}; quit",
        gap = shot("gap").display(),
        settled = shot("settled").display(),
    );
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--no-intro",
            "--drive",
            &script,
        ])
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("OAG_MODS", dir.join("mods"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --drive");
    let read = |name: &str| {
        oag_texture::replacer::decode_png(&std::fs::read(shot(name)).expect("a screenshot"))
            .expect("a PNG")
    };
    let (empty, settled) = (read("gap"), read("settled"));
    // `SHIP MODEL`'s frame, 740..1660 by 200..740 of the 1920 by 1080 grid, in this picture.
    let scale = empty.width as f32 / 1920.0;
    let frame = [740.0, 200.0, 1660.0, 740.0].map(|edge| (edge * scale) as u32);
    let live = hull_edges(&settled, &empty, frame).map(|x| f64::from(x) / f64::from(empty.width));
    let capture_empty = ship_select_still(&image, "Mirage", "0.03");
    let capture = ship_select_still(&image, "Mirage", "3");
    let at_rest = hull_edges(&capture, &capture_empty, [740, 200, 1660, 740])
        .map(|x| f64::from(x) / f64::from(capture.width));
    eprintln!("live edges {live:.4?} of the width, capture at rest {at_rest:.4?}");
    // The left edge is the hull's own; the right edge picks up the backdrop's embers, which
    // are not the same two frames. The widget's own pose puts them 0.017 and 0.078 off.
    assert!(
        (live[0] - at_rest[0]).abs() < 0.01,
        "{live:.4?} against {at_rest:.4?}"
    );
    assert!(
        (live[1] - at_rest[1]).abs() < 0.04,
        "{live:.4?} against {at_rest:.4?}"
    );
}

/// Omega's craft sits where its own frame has it: Feisar's concept hull on a fresh profile,
/// 1030 to 1357 across and 358 to 685 down a 1920 by 1080 frame (the original's frame, read as
/// the craft's pixels against the page: saturated or dark). The rest pose is the screen
/// update's own values and the picture zoom was fitted to this same frame, so this holds the
/// wiring (rotation, position, scale, lens, origin) and not the zoom's cause. Dropping the
/// pose draws the widget as authored (about 1.6 times the size, 70 pixels to the left), and
/// putting back HD's fit-and-settle starts it at another size.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/ and the original's frame under data/reference/compare-page/omega"]
fn omega_poses_feisars_concept_hull_where_the_original_draws_it() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let (mut archives, title) = boot_archives(&source, "omega-rest-pose");
    let front_end = title.front_end.expect("a front end");
    let pose = front_end.ship_preview_pose.expect("the update's rest pose");
    let widget = oag_ui_screens::picker::hd::ShipModel {
        origin: [960.0, 540.0],
        position: [pose.position[0], pose.position[1]],
        z: pose.position[2],
        scale: pose.scale,
        near: 0.1,
        fov_degrees: pose.fov_degrees,
        zoom: pose.zoom,
        rot_x: pose.rotation[0],
        rot_y: pose.rotation[1],
        obey_safe_zone: true,
    };
    let join = title.race.team_variants_for("Feisar").map(|v| v.join);
    let location = oag_game::preview::variant::variant_location(
        join,
        r"Data\art\published\hdships\Feisar",
        "_c1",
    );
    let entry = oag_game::preview::ship_entry(title, &location, oag_race::Mode::default());
    let model = oag_game::preview::psp2_scene::model(&mut archives, &entry)
        .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
    let (view_projection, placed) = oag_game::preview::ship_model::matrices(
        &widget,
        &oag_game::preview::ship_model::control::Craft::default(),
        oag_display::space::Space::HD,
        1.0,
    );
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
    let skip = across.len() / 2000;
    let drawn = [
        across[skip],
        across[across.len() - 1 - skip],
        down[skip],
        down[down.len() - 1 - skip],
    ];
    eprintln!("omega Feisar _c1 edges {drawn:?}");
    for (ours, theirs) in drawn.into_iter().zip([1030.0, 1357.0, 358.0, 685.0]) {
        assert!((ours - theirs).abs() < 10.0, "{drawn:?}");
    }
}

/// A title's archives and record, booted from `source` the way the previews' tests need them.
fn boot_archives(
    source: &std::path::Path,
    tag: &str,
) -> (oag_assets::Archives, &'static oag_title::Title) {
    let options = oag_game::boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache(&format!("{tag}-ship-light-ground-truth")),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (_, archives, title) = oag_game::boot::load_shell(&options).expect("the title boots");
    (archives, title)
}

/// The craft's light is the front end's own `.envsettings`, read from the disc: the Fury
/// style's file in the Fury style and the HD style's in the HD one, at HD's halved lit output
/// times the tone stage's exposure at the front end's adapted luma (the HD style's, unread, at the clamp). The tone constants the
/// title carries are what the Fury file authors for those keys, so a table that drifts from the
/// disc fails here. A title with no `model_lighting` draws no such light.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn the_craft_is_lit_by_the_front_ends_envsettings_and_its_tone_stage() {
    use oag_tables::envsettings::{
        AMBIENT_COLOUR, EnvSettings, SUN_COLOUR, SUN_DIRECTION, SUN_SPECULAR_SCALE,
        TONE_ADAPTION_BOOST, TONE_DARKENING_CLAMP, TONE_MAXIMUM_BRIGHTNESS,
    };
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let (mut archives, title) = boot_archives(&image, "hd");
    let lighting = title
        .front_end
        .and_then(|front_end| front_end.model_lighting)
        .expect("HD's front end names what lights its models");
    let tone = lighting.tone.expect("HD's front end has its tone stage");

    let blob = archives
        .read_name(lighting.fury_file)
        .expect("the Fury style's file is on the disc");
    let file = EnvSettings::parse(&String::from_utf8(blob).expect("text")).expect("parses");
    assert_eq!(file.scalar(TONE_ADAPTION_BOOST), Some(tone.boost));
    assert_eq!(file.scalar(TONE_DARKENING_CLAMP), Some(tone.clamp));
    assert_eq!(file.scalar(TONE_MAXIMUM_BRIGHTNESS), Some(tone.maximum));

    let mut report = Vec::new();
    let light = oag_game::preview::model_light(&mut archives, title, true, None, &mut report)
        .unwrap_or_else(|| panic!("no light: {report:?}"));
    assert_eq!(light.enabled, 1.0, "an authored rig, not the stand-in");
    assert_eq!(Some(light.sun), file.vec3(SUN_COLOUR));
    assert_eq!(Some(light.ambient), file.vec3(AMBIENT_COLOUR));
    assert_eq!(Some(light.direction), file.direction(SUN_DIRECTION));
    assert_eq!(Some(light.specular_scale), file.scalar(SUN_SPECULAR_SCALE));
    let exposure = tone.maximum - (tone.adapted_fury * tone.boost).min(tone.clamp);
    assert!(
        (light.output_scale - 0.5 * exposure).abs() < 1e-5,
        "output scale {} against 0.5 x {exposure}",
        light.output_scale
    );
    assert!(
        (1.7..1.73).contains(&light.output_scale),
        "the scale the frame fit measured is 1.725 ({})",
        light.output_scale
    );

    // A capture that states the adapted luma (a hidden backdrop adapts darker) replaces the read.
    let stated =
        oag_game::preview::model_light(&mut archives, title, true, Some(0.01451), &mut report)
            .expect("the stated luma still lights the rig");
    let stated_exposure = tone.maximum - (0.01451 * tone.boost).min(tone.clamp);
    assert!(
        (stated.output_scale - 0.5 * stated_exposure).abs() < 1e-5
            && stated.output_scale > light.output_scale,
        "output scale {} at a stated luma, {} at the title's read",
        stated.output_scale,
        light.output_scale
    );

    let hd_style = oag_game::preview::model_light(&mut archives, title, false, None, &mut report)
        .expect("the HD style's file reads");
    assert_ne!(hd_style.sun, light.sun, "the two styles are two files");
    assert!(
        (hd_style.output_scale - 0.5 * (tone.maximum - tone.clamp)).abs() < 1e-6,
        "the HD style's white page sits at the clamp: {}",
        hd_style.output_scale
    );
}

/// The mean luma of the hull's lit pixels (those over 60) in the 716,150 1012x650 box of a
/// 1920x1080 frame, where the craft stands on Ship Select: the backdrop's embers are dimmer and
/// move between frames, the hull does not.
fn hull_box_luma(still: &oag_texture::replacer::Image) -> f64 {
    let scale = still.width as f32 / 1920.0;
    let [left, top, right, bottom] =
        [716.0, 150.0, 1728.0, 800.0].map(|edge| (edge * scale) as u32);
    let (mut sum, mut count) = (0.0, 0u32);
    for y in top..bottom {
        for x in left..right {
            let at = ((y * still.width + x) * 4) as usize;
            let [r, g, b] = [0, 1, 2].map(|c| f64::from(still.rgba[at + c]));
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            if luma > 60.0 {
                sum += luma;
                count += 1;
            }
        }
    }
    assert!(count > 10_000, "no hull in the box");
    sum / f64::from(count)
}

/// Both paths a player's frame takes bind the light: the capture and the live screen
/// (`--drive`) draw Ship Select's Feisar from the front end's rig, each above the level it draws
/// at with the stand-in (capture 121 against 180 lit, live 108 against 146). The live screen
/// draws dimmer than the capture in both states, so each is held to its own floor, halfway.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images and a GPU adapter"]
fn ship_selects_hull_is_as_bright_as_the_original_on_both_paths() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let capture = hull_box_luma(&ship_select_still(&image, "Feisar", "3"));
    eprintln!("capture hull box luma {capture:.2}");
    assert!(capture > 150.0, "the capture's hull is {capture:.2}");

    let dir = oag_testdata::scratch::dir("hd-ship-light-live");
    std::fs::create_dir_all(dir.join("mods")).expect("a scratch directory");
    let shot = dir.join("settled.png");
    let script = format!(
        "boot; select racebox; press cross; until page=race; select launch_race; \
         press cross; until screen=picker; press cross; until screen.id=ship-select; \
         until opening_reads=landed; ticks 120; {steps}ticks 240; screenshot {}; quit",
        shot.display(),
        // A fresh profile opens the screen on Assegai; Feisar is seven steps left.
        steps = "press left; ticks 30; ".repeat(7),
    );
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--no-intro",
            "--drive",
            &script,
        ])
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("OAG_MODS", dir.join("mods"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --drive");
    let live = hull_box_luma(
        &oag_texture::replacer::decode_png(&std::fs::read(&shot).expect("a screenshot"))
            .expect("a PNG"),
    );
    eprintln!("live hull box luma {live:.2}");
    assert!(live > 127.0, "the live screen's hull is {live:.2}");
}

/// The honeycomb's grey fill is not one value: the original's frame reads 43 beside the
/// selected column and eases to 23 at the grid's edge (`HexColFade`'s 25 and a little under),
/// where the authored `HexCol` over black is 50 everywhere. Held to the fill's own greys, which
/// no padlock, thumbnail or text reaches below 15 or above 48; a flat grid has none of them.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images and a GPU adapter"]
fn the_honeycomb_fill_fades_toward_the_grids_edge() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let still = ship_select_still(&image, "Feisar", "3");
    let width = still.width as usize;
    let mut fills = Vec::new();
    for y in (440..860).step_by(20) {
        for x in 160..720 {
            let at = (y * width + x) * 4;
            let [r, g, b] = [still.rgba[at], still.rgba[at + 1], still.rgba[at + 2]];
            if r == g && g == b && (15..=48).contains(&r) {
                fills.push(r);
            }
        }
    }
    let (lowest, highest) = (fills.iter().min().copied(), fills.iter().max().copied());
    eprintln!(
        "{} grey fill pixels, {lowest:?} to {highest:?}",
        fills.len()
    );
    assert!(fills.len() > 2_000, "the grid's fills: {}", fills.len());
    assert!(lowest.is_some_and(|v| v <= 28), "no faded fill: {lowest:?}");
    assert!(
        highest.is_some_and(|v| v >= 38),
        "no fill near the selection: {highest:?}"
    );
}

/// Omega's craft is lit by Omega's own `fe.EnvSettings`, in its own key spelling and with its
/// sun scale: ambient `0.2 0.3 0.32` and a sun of `0.748 0.906 1` at intensity 2, which are not
/// HD's rig. No tone stage and no halved lit output: neither is read for Omega.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_lights_its_craft_from_its_own_front_end_file() {
    use oag_tables::envsettings::{
        EnvSettings, OMEGA_SUN_DIFFUSE_INTENSITY, PSP2_AMBIENT_COLOUR, PSP2_SUN_DIFFUSE_COLOUR,
    };
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let (mut archives, title) = boot_archives(&source, "omega");
    let lighting = title
        .front_end
        .and_then(|front_end| front_end.model_lighting)
        .expect("Omega's front end names what lights its models");
    assert!(
        lighting.tone.is_none(),
        "Omega's front-end adaptation is unread"
    );
    let blob = archives
        .read_name(lighting.hd_file)
        .expect("Omega's front-end settings are in the package");
    let file = EnvSettings::parse(&String::from_utf8(blob).expect("text")).expect("parses");

    let mut report = Vec::new();
    let light = oag_game::preview::model_light(&mut archives, title, false, None, &mut report)
        .unwrap_or_else(|| panic!("no light: {report:?}"));
    assert_eq!(light.enabled, 1.0);
    assert_eq!(Some(light.ambient), file.vec3(PSP2_AMBIENT_COLOUR));
    assert_eq!(light.ambient, [0.2, 0.3, 0.32]);
    let intensity = file
        .scalar(OMEGA_SUN_DIFFUSE_INTENSITY)
        .expect("the file authors the sun's intensity");
    let sun = file.vec3(PSP2_SUN_DIFFUSE_COLOUR).expect("the sun colour");
    assert_eq!(light.sun, sun.map(|c| c * intensity));
    assert_eq!(light.output_scale, 1.0, "no halved lit output on Omega");
    assert_eq!(light.nova, 1.0, "Omega's own prelit combination");
}

/// Ship Select's model row on HD and Omega follows each `PI_TeamModel`'s own `<Unlock>` row
/// (`oag_game::unlock::ModelGate`, the boot's own gate): Feisar on a fresh profile offers
/// `concept1` (`_c1`) and `normal` (`""`) alone; its own loyalty of 4000 adds `SKIN1`
/// (`livery2`), 8000 `SKIN2` (`livery3`); gold on all 87 cells of the `HD` campaign's grids adds
/// `chrome` (`livery4`), gold on all 80 of `Fury`'s adds `chrome_c1` and (`grid8`'s points met)
/// `nitro`. Every one of the twelve HD-era teams carries the same seven rows.
fn the_model_row_follows_the_discs_unlock_rows(
    shell: &oag_game::boot::Shell,
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
) {
    use oag_game::records::{Medal, Store};
    let gate = &shell.model_gate;
    assert!(gate.gates(), "{}: the model rows are evaluated", title.name);
    let feisar = shell
        .teams
        .iter()
        .find(|team| team.id == "Feisar")
        .expect("Feisar");
    let rows: Vec<(&str, Option<&str>, String)> = feisar
        .models
        .iter()
        .filter(|model| !model.name.starts_with("vr_"))
        .map(|model| {
            let row = model.unlock.first();
            (
                model.name.as_str(),
                model.texture_location.as_deref(),
                format!(
                    "{:?} {:?} {:?}",
                    row.and_then(|row| row.loyalty),
                    row.and_then(|row| row.medals.as_ref())
                        .map(|medals| (medals.campaign.clone(), medals.at_least)),
                    row.and_then(|row| row.grid.clone()),
                ),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "chrome_c1",
                Some("livery4"),
                r#"None Some((Some("FURY"), 80)) None"#.to_string()
            ),
            (
                "nitro",
                Some("livery1"),
                r#"None None Some("grid8")"#.to_string()
            ),
            ("concept1", Some("livery1"), "None None None".to_string()),
            ("normal", Some("livery1"), "None None None".to_string()),
            ("SKIN1", Some("livery2"), "Some(4000) None None".to_string()),
            ("SKIN2", Some("livery3"), "Some(8000) None None".to_string()),
            (
                "chrome",
                Some("livery4"),
                r#"None Some((Some("HD"), 87)) None"#.to_string()
            ),
        ],
        "{}",
        title.name
    );
    let offered = |records: &Store| -> Vec<String> {
        oag_game::livery_row::gated_variant_choices(title, "Feisar", &shell.teams, records, gate)
            .into_iter()
            .map(|choice| choice.value)
            .collect()
    };
    assert_eq!(offered(&Store::default()), ["_c1", ""], "{}", title.name);
    let mut loyal = Store::default();
    loyal.record_loyalty(title, "Feisar", 4000);
    assert_eq!(offered(&loyal), ["_c1", "", "@livery2"], "{}", title.name);
    loyal.record_loyalty(title, "Feisar", 4000);
    assert_eq!(
        offered(&loyal),
        ["_c1", "", "@livery2", "@livery3"],
        "{}",
        title.name
    );

    let grids = oag_game::campaign::read_grids(
        archives,
        title.campaign.model_unlocks.expect("a grids definition"),
        title.campaign.grid_archive,
    )
    .expect("the campaign grids read");
    let campaign = oag_game::records::campaign_title(title, archives.layout.platform);
    let golds = |which: &str| {
        let mut records = Store::default();
        let mut cells = 0;
        for grid in grids.iter().filter(|grid| {
            grid.campaign
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case(which))
        }) {
            for cell in &grid.cells {
                records.record_campaign(&campaign, &cell.name, Some(Medal::Gold), None);
                cells += 1;
            }
        }
        (records, cells)
    };
    let (hd, cells) = golds("HD");
    assert_eq!(cells, 87, "{}: the HD campaign's cells", title.name);
    assert_eq!(offered(&hd), ["_c1", "", "@livery4"], "{}", title.name);
    let (fury, cells) = golds("Fury");
    assert_eq!(cells, 80, "{}: the Fury campaign's cells", title.name);
    assert_eq!(
        offered(&fury),
        ["_c1@livery4", "_n1", "_c1", ""],
        "{}",
        title.name
    );

    // The same seven rows on every HD-era team, the threshold against its own name.
    let mut hd_era = 0;
    for team in &shell.teams {
        let names: Vec<&str> = team
            .models
            .iter()
            .map(|model| model.name.as_str())
            .collect();
        if !names.contains(&"SKIN1") || team.models.iter().all(|model| model.unlock.is_empty()) {
            continue;
        }
        hd_era += 1;
        let skin1 = team
            .models
            .iter()
            .find(|model| model.name == "SKIN1")
            .expect("SKIN1");
        assert_eq!(
            skin1.unlock[0].team.as_deref(),
            Some(team.id.as_str()),
            "{}",
            team.id
        );
        assert_eq!(skin1.unlock[0].loyalty, Some(4000), "{}", team.id);
    }
    assert_eq!(
        hd_era, 12,
        "{}: twelve HD-era teams gate their models",
        title.name
    );
}

/// Feisar's Ship Select, `--menu-entry-seconds` as given (`None` is the flag absent), as RGBA.
fn ship_select_entry_still(
    image: &std::path::Path,
    entry_seconds: Option<&str>,
) -> oag_texture::replacer::Image {
    let dir = oag_testdata::scratch::dir("hd-entry-wipe-ground-truth");
    std::fs::create_dir_all(dir.join("config/oag")).expect("a temp directory");
    std::fs::write(
        dir.join("config/oag/settings.toml"),
        "[race]\nteam = \"Feisar\"\n",
    )
    .expect("a settings file");
    let out = dir.join(format!("{}.png", entry_seconds.unwrap_or("none")));
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "1920x1080",
            "--menu-page",
            "ship-select",
        ])
        .args(["--fury-tint", "0", "--no-hull-shake"]);
    if let Some(seconds) = entry_seconds {
        command.args(["--menu-entry-seconds", seconds]);
    }
    let status = command
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page ship-select");
    let still = oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG");
    let _ = std::fs::remove_dir_all(&dir);
    still
}

/// Pixels of `a` and `b` that differ by more than 40 in a channel inside `[left, top, right, bottom]`.
fn differing(
    a: &oag_texture::replacer::Image,
    b: &oag_texture::replacer::Image,
    frame: [u32; 4],
) -> usize {
    let mut count = 0;
    for y in frame[1]..frame[3] {
        for x in frame[0]..frame[2] {
            let at = ((y * a.width + x) * 4) as usize;
            if (0..3).any(|c| a.rgba[at + c].abs_diff(b.rgba[at + c]) > 40) {
                count += 1;
            }
        }
    }
    count
}

/// Team Selection's `Logo` waits its authored 0.5 s from the page opening and ramps over 0.7 s
/// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Wired: page entry"): at 0.3 s the
/// banner panel is bare, at 0.6 s a slab of the logo shows, at 1.3 s the logo is whole, and a
/// still with no stated time is the whole logo. This fails if the delay, the entry clock or its
/// flag is dropped: with any of them gone the 0.3 s still shows the logo.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn the_team_logo_waits_its_delay_and_wipes_on_the_page_entry() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let logo = [260, 220, 640, 330];
    let settled = ship_select_entry_still(&image, None);
    let whole = ship_select_entry_still(&image, Some("1.3"));
    let waiting = ship_select_entry_still(&image, Some("0.3"));
    let slab = ship_select_entry_still(&image, Some("0.6"));
    assert_eq!(
        differing(&settled, &whole, logo),
        0,
        "no stated time is finished"
    );
    let ink = differing(&waiting, &whole, logo);
    assert!(
        ink > 3000,
        "the whole logo is {ink} pixels over the bare panel"
    );
    let slab_ink = differing(&waiting, &slab, logo);
    assert!(
        (100..ink / 2).contains(&slab_ink),
        "0.6 s is a slab: {slab_ink} of {ink} pixels"
    );
}

/// Vineta K's Track Select, the Fury backdrop off, with `flags` added, as RGBA.
fn track_select_still(
    image: &std::path::Path,
    tag: &str,
    flags: &[&str],
) -> oag_texture::replacer::Image {
    let dir = oag_testdata::scratch::dir("hd-track-hex-ground-truth");
    std::fs::create_dir_all(dir.join("config/oag")).expect("a temp directory");
    let out = dir.join(format!("{tag}.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--no-audio",
            "--size",
            "1920x1080",
            "--menu-page",
            "track-select",
            "--fury-tint",
            "0",
            "--no-hull-shake",
        ])
        .args(flags)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --menu-page track-select");
    let still = oag_texture::replacer::decode_png(&std::fs::read(&out).expect("the screenshot"))
        .expect("a PNG");
    let _ = std::fs::remove_dir_all(&dir);
    still
}

/// Track Select's hex grid is `HexSelection` with its own update (`ship-select-ring.md`, "Track
/// Select"): the ring pulses on the widget's own counter, which a still pins with
/// `--menu-ring-seconds` (white at 0, the `HD_Blue` mix 0.195 s in), and a circuit step slides the
/// lattice a sixth of a pitch on its first frame. This fails if the grid is drawn without the
/// picker's slide or the ring ignores its clock: the stills would be identical.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso under data/images"]
fn the_track_grids_ring_and_slide_follow_their_clocks() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let grid = [180, 440, 860, 720];
    let ring = [460, 560, 640, 680];
    let rest = track_select_still(&image, "rest", &["--menu-ring-seconds", "0.195"]);
    let white = track_select_still(&image, "white", &["--menu-ring-seconds", "0"]);
    let band = differing(&rest, &white, ring);
    assert!(
        band > 150,
        "the ring's pulse moves {band} pixels of its band"
    );
    let same = track_select_still(&image, "same", &["--menu-ring-seconds", "0.195"]);
    assert_eq!(differing(&rest, &same, grid), 0, "a still is repeatable");
    let stepped = track_select_still(
        &image,
        "stepped",
        &[
            "--menu-ring-seconds",
            "0.195",
            "--menu-picker-step-seconds",
            "0.05",
        ],
    );
    let moved = differing(&rest, &stepped, grid);
    assert!(
        moved > 2000,
        "three frames into a step the lattice has moved: {moved} pixels"
    );
}
