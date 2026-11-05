//! Why Omega's campaign screens draw through HD's list, and why one widget is
//! left out of it - against the disc.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^omega_campaign_screens_ground_truth::/)'
//! ```

#[test]
fn omega_and_hd_draw_hd_campaign_screens_and_nothing_else_does() {
    for (title, expected) in [
        (oag_hd::TITLE, true),
        (oag_omega::TITLE, true),
        (oag_pulse::TITLE, false),
        (oag_pure::TITLE, false),
        (oag_2048::TITLE, false),
    ] {
        assert_eq!(
            oag_game::campaign::draws_hd_campaign(title),
            expected,
            "{}",
            title.name
        );
    }
}

/// Omega's `Cell Selection` authors `RecordsButton` at `DifficultyButton`'s own
/// position, so only one can be on screen - the premise of dropping the
/// Records one - and both carry the raw placeholder texts a Pulse-shaped draw
/// list would print.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_cell_selection_authors_records_and_difficulty_buttons_in_one_place() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let xml = String::from_utf8(
        archives
            .read_name(oag_omega::campaign::SCREEN_ENTRY)
            .expect("CellMode_Definition.xml"),
    )
    .expect("utf-8");
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let screen = screens
        .screens
        .iter()
        .find(|s| s.name == "Cell Selection")
        .expect("Cell Selection");
    let at = |name: &str| {
        screen
            .texts
            .iter()
            .find(|t| t.name.as_deref() == Some(name))
            .map(|t| (t.x, t.y))
    };
    assert!(at("DifficultyButton").is_some());
    assert_eq!(at("RecordsButton"), at("DifficultyButton"));
}

/// Omega's campaign carries its three-way `Campaign Selection` (2048, HD, Fury, each opening its
/// own grid screen) and decodes every flyer card, front and back, through the PS4 scene path:
/// dropping the chooser read, the card loader or the `psp2` dispatch in `flyer` fails this.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_campaign_carries_fury_s_layout_and_every_card() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let base = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_omega::TITLE,
        &[],
    )
    .expect("the campaign loads");
    let chooser = campaign
        .chooser
        .as_ref()
        .expect("Omega authors Campaign Selection");
    assert!(campaign.selection_layout.is_some());
    // The list's order, where its redirect sends each entry and the grids that screen pages, all
    // read off the disc's XML: three campaigns, three different grid screens, three ranges.
    let entries: Vec<_> = chooser
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.id.as_str(),
                entry.screen.as_str(),
                entry.grids.clone(),
                entry.layout.screen.name.as_str(),
            )
        })
        .collect();
    assert_eq!(
        entries,
        [
            (
                "FE_RC",
                "Grid Selection 2048",
                16..19,
                "Grid Selection 2048"
            ),
            ("FE_RC_HD", "Grid Selection", 0..8, "Grid Selection"),
            (
                "FE_RC_FURY",
                "Grid Selection Fury",
                8..16,
                "Grid Selection Fury"
            ),
        ]
    );
    let flyers = campaign
        .flyers
        .expect("Omega's screen authors the flyer widget");
    let broken: Vec<&String> = flyers
        .report
        .iter()
        .filter(|line| !line.contains("00_flyer"))
        .collect();
    assert!(
        broken.is_empty(),
        "cards that would not decode: {broken:#?}"
    );
    let fury = &campaign.grids[oag_hd::campaign::FURY_GRID_RANGE];
    assert_eq!(fury.len(), 8);
    assert_eq!(fury[0].flyer_name.as_deref(), Some("09_Blitzed"));
    // Each choice opens its own first grid: HD's and Fury's differ, and 2048's three are not loaded
    // (`grid_16`..`grid_18` author no `RequiredPoints`), which the session turns into a notice.
    let first = |range: std::ops::Range<usize>| {
        campaign
            .grids
            .get(range)
            .and_then(|grids| grids.first())
            .map(|grid| grid.name.clone())
    };
    assert_eq!(first(entries[1].2.clone()).as_deref(), Some("grid0"));
    assert_eq!(first(entries[2].2.clone()).as_deref(), Some("grid8"));
    assert_eq!(first(entries[0].2.clone()), None);
    // The three cards of the chooser itself, in list order, each placed (the 2048 one has no
    // PS3 counterpart: it is `2048_campaign`'s own model).
    let shows = flyers.selection_shows(oag_ui_screens::campaign::selection::Campaign::new(0));
    assert_eq!(
        shows
            .iter()
            .map(|show| show.flyer.as_str())
            .collect::<Vec<_>>(),
        ["2048_campaign", "hd_campaign", "fury_campaign"]
    );
    for show in &shows {
        assert!(
            flyers
                .card_rect(show, oag_display::space::Space::HD)
                .is_some(),
            "{} has no rectangle",
            show.flyer
        );
    }
    // The card stands where the shadPS4 frame has it (x 816-1628, y 151-702 of 1920 by 1080,
    // measured off `omega/menu-grid-select`), not at HD's `GRID_POSE` (x 747-1688).
    let show = flyers.grid_show("09_Blitzed");
    assert_eq!(
        show.pose.offset,
        oag_omega::TITLE.campaign.grid_card_pose.unwrap().offset
    );
    let [x, y, width, height] = flyers
        .card_rect(&show, oag_display::space::Space::HD)
        .expect("the card is loaded");
    for (got, want) in [
        (x, 816.0),
        (y, 151.0),
        (x + width, 1628.0),
        (y + height, 702.0),
    ] {
        assert!(
            (got - want).abs() < 25.0,
            "card edge {got} against the frame's {want}"
        );
    }
}

/// The PS4 `00_flyer.rcsmodel` is the card's shell: its outline and the reflection's fade
/// come off the PS4 container (`flyer::shell`'s `load_psp2`), where the PS3 chunk reader
/// refused it and every card was a plain rectangle.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_cards_take_the_ps4_shell() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let shell = oag_game::flyer::Shell::load(&mut archives).expect("the PS4 shell reads");
    assert_eq!(
        shell.outline.len(),
        25,
        "twenty-five front triangles, as HD's"
    );
    assert!(
        (shell.half_size[0] - 51.2).abs() < 0.01,
        "{:?}",
        shell.half_size
    );
    assert!(
        (shell.half_size[1] - 33.3).abs() < 0.1,
        "{:?}",
        shell.half_size
    );
    assert!(
        (shell.fade.top_alpha - 76.0 / 255.0).abs() < 1e-6,
        "0x4c at the edge"
    );
}

/// The card materials read their coverage the way HD does: `simpletextureandtexturealpha`
/// from the picture's own alpha, `basicalpha` and `scrollingalpha` from the red channel of
/// the elements atlas (bound second) - read off the red channel or unbound they draw the
/// white boxes and dark blocks Omega's cards had. Walks every card of every grid, front and
/// back: the slot guard must never skip one.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_card_materials_read_their_coverage_channels() {
    use oag_mesh::mesh::slots;
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let base = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_omega::TITLE,
        &[],
    )
    .expect("the campaign loads");
    let channel = |slots: u32| (slots >> 3) & 3;
    let mut flyers: Vec<&str> = campaign
        .grids
        .iter()
        .filter_map(|grid| grid.flyer_name.as_deref())
        .collect();
    flyers.sort_unstable();
    flyers.dedup();
    assert!(flyers.len() >= 16, "{flyers:?}");
    let mut atlas_draws = 0;
    for flyer in flyers {
        let front = format!("Data/FE/Flyers/{flyer}/flyer.vex");
        let (model, _) = oag_game::preview::psp2_scene::model_named(&mut archives, &front)
            .unwrap_or_else(|e| panic!("{front}: {e:#}"));
        assert!(
            model
                .vertices
                .iter()
                .all(|v| channel(v.slots) == 3 || v.slots & slots::ALPHA_FROM_SECOND != 0),
            "{front}: every vertex reads the picture's alpha or an atlas"
        );
        let back = format!("Data/FE/Flyers/{flyer}/flyer_Back.vex");
        let (model, _) = oag_game::preview::psp2_scene::model_named(&mut archives, &back)
            .unwrap_or_else(|e| panic!("{back}: {e:#}"));
        let atlas_slots: std::collections::BTreeSet<usize> = model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .filter(|draw| {
                let first = model.indices[draw.range.start as usize] as usize;
                model.vertices[first].slots & slots::ALPHA_FROM_SECOND != 0
            })
            .filter_map(|draw| draw.texture)
            .collect();
        let geometry = archives
            .read_name(&back.replace(".vex", ".rcsmodel"))
            .expect("the back's geometry");
        let parsed = oag_rcs::rcsmodel::psp2::parse(&geometry).expect("the back parses");
        let mut owners: Vec<usize> = parsed
            .submeshes
            .iter()
            .filter_map(|sub| sub.material)
            .collect();
        owners.sort_unstable();
        owners.dedup();
        let wanted = owners
            .iter()
            .filter(|&&m| {
                let name = parsed.materials[m].name.as_str();
                (name.ends_with("/basicalpha.rcsmaterial")
                    || name.ends_with("/scrollingalpha.rcsmaterial"))
                    && parsed.materials[m].sampler(0xa826_2b61).is_some()
            })
            .count();
        assert!(wanted >= 2, "{back}: {wanted}");
        assert_eq!(
            atlas_slots.len(),
            wanted,
            "{back}: every basicalpha and scrollingalpha material binds its atlas (the slot guard skipped one)"
        );
        for slot in atlas_slots {
            let atlas = model.lightmaps.get(slot).and_then(Option::as_ref);
            let picture = model.textures[slot].as_ref().expect("a picture");
            assert!(
                atlas.is_some_and(|t| t.label != picture.label),
                "{back}: slot {slot} binds an atlas second, apart from its picture {}",
                picture.label
            );
            atlas_draws += 1;
        }
    }
    assert!(atlas_draws >= 32);
}

/// Mean colour of the opaque texels of one crop of the campaign sheet, and the crop's opaque rows
/// relative to its own top. `uv` is `[x, y, width, height]` in the sheet.
struct Crop {
    rgb: [f32; 3],
    rows: std::ops::RangeInclusive<u32>,
}

fn crop_of(sheet: &oag_hud::sprite::Sheet, uv: [f32; 4]) -> Crop {
    let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
    let (mut first, mut last) = (u32::MAX, 0);
    for row in 0..uv[3] as u32 {
        for column in 0..uv[2] as u32 {
            let x = uv[0] as u32 + column;
            let y = uv[1] as u32 + row;
            let at = ((y * sheet.width + x) * 4) as usize;
            let texel = &sheet.rgba[at..at + 4];
            if texel[3] >= 128 {
                for (total, channel) in sum.iter_mut().zip(texel) {
                    *total += f32::from(*channel);
                }
                count += 1.0;
                first = first.min(row);
                last = row;
            }
        }
    }
    Crop {
        rgb: sum.map(|total| total / count),
        rows: first..=last,
    }
}

/// `Target0/1/2 Medal` author HD's crop on Omega's own `CellMode_Definition.xml` (`v` 0, 61,
/// 122, 60 px frames), the atlas is HD's 61 px frame grid, and the picture it holds stores each
/// difficulty block bronze, silver, gold from the top. So the crop a title draws has to come from
/// its [`oag_title::Campaign::medal_tier_rows`], not from the authored `v` alone. Read off the
/// draw list `Cell Selection` really produces (the three sprites that sample the atlas, in
/// `Target0`, `1`, `2` order) and sampled from the sheet they sample: gold reads yellow (green
/// over red high), bronze orange (green over red low) and silver grey, in the plain, hook and
/// swirl blocks alike. Dropping `medal_tier_rows` from Omega, or `CellArt` ignoring it, draws
/// bronze at gold's slot, the player's report. The two `oag-game` sites that fill `CellArt`
/// from the title are covered by the pair render, not by this test.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_medal_crops_read_gold_silver_bronze_in_every_difficulty_block() {
    use oag_tables::race_campaign::Difficulty;
    use oag_ui::frontend::Draw;
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let xml = String::from_utf8(
        archives
            .read_name(oag_omega::campaign::SCREEN_ENTRY)
            .expect("CellMode_Definition.xml"),
    )
    .expect("utf-8");
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let cell = screens
        .screens
        .iter()
        .find(|screen| screen.name == "Cell Selection")
        .expect("Cell Selection");
    for (name, v) in [
        ("Target0 Medal", 0.0),
        ("Target1 Medal", 61.0),
        ("Target2 Medal", 122.0),
    ] {
        let image = cell
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(
            (image.u, image.v, image.width, image.height),
            (Some(0.0), Some(v), Some(60.0), Some(60.0)),
            "{name}: Omega authors HD's crop"
        );
    }

    let base = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_omega::TITLE,
        &[],
    )
    .expect("the campaign loads");
    let atlas = r"Data\FE\Images\Hexmedal_HD.gtf";
    let placed = campaign
        .sprites
        .get(atlas)
        .expect("the atlas is on the sheet");
    assert_eq!((placed.width, placed.height), (976, 549), "Omega's atlas");

    let grid = campaign.grids.last().expect("a grid");
    let skin = oag_ui::menu::Skin::new(
        oag_omega::frontend::MENU_SKIN,
        oag_display::space::Space::OMEGA,
        33.0,
    );
    let in_atlas = |uv: &[f32; 4]| {
        uv[0] >= placed.x as f32
            && uv[1] >= placed.y as f32
            && uv[0] < (placed.x + placed.width) as f32
            && uv[1] < (placed.y + placed.height) as f32
    };
    for difficulty in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
        let mut model = oag_ui_screens::campaign::CellSelection::new(grid.cells.clone());
        model.set_difficulty(difficulty);
        let layers = oag_ui_screens::campaign::hd::hd_cell_draw_list(
            &model,
            &campaign.cell_layout,
            &skin,
            &oag_ui::menu::Frame::default(),
            &oag_ui::language::StringTable::default(),
            &oag_ui::language::CircuitNames::default(),
            0,
            campaign.grids.len(),
            &oag_ui_screens::campaign::GridSummary::from_grid(grid),
            &oag_ui_screens::campaign::hd::CellArt {
                next_flyer: None,
                fury: false,
                medal_tier_rows: oag_omega::TITLE.campaign.medal_rows(),
                track_emblems: &campaign.circuit_emblems,
            },
            None,
            false,
            &|src| campaign.sprites.get(src),
            &[],
        );
        let crops: Vec<[f32; 4]> = layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { uv, .. } if in_atlas(uv) => Some(*uv),
                _ => None,
            })
            .collect();
        assert_eq!(
            crops.len(),
            3,
            "{difficulty:?}: a fresh profile draws the three target icons and no badge"
        );
        let mut ratio = Vec::new();
        for uv in crops {
            let crop = crop_of(&campaign.sprites, uv);
            // The frame is 60 rows with the icon inside it and a clear row under it: the
            // 61 px pitch and the 183 px block pitch hold on Omega's 976x549.
            assert!(
                *crop.rows.start() >= 1 && *crop.rows.end() <= 58,
                "{difficulty:?}: icon rows {:?} leave the frame",
                crop.rows
            );
            let [r, g, b] = crop.rgb;
            eprintln!(
                "{difficulty:?} {uv:?}: rgb {r:.0} {g:.0} {b:.0} g/r {:.3}",
                g / r
            );
            ratio.push((g / r, r - b));
        }
        let [
            (gold, gold_warmth),
            (_, silver_warmth),
            (bronze, bronze_warmth),
        ] = [ratio[0], ratio[1], ratio[2]];
        assert!(
            silver_warmth.abs() < 10.0,
            "{difficulty:?}: silver is the neutral one, red minus blue {silver_warmth:.1}"
        );
        assert!(
            gold_warmth > 30.0 && bronze_warmth > 30.0,
            "{difficulty:?}: gold and bronze are the warm ones, red minus blue {gold_warmth:.1} and {bronze_warmth:.1}"
        );
        assert!(
            gold > bronze + 0.05,
            "{difficulty:?}: gold is yellower than bronze: green/red {gold:.3} against {bronze:.3}"
        );
    }
}
