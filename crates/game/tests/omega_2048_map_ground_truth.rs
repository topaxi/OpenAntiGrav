//! Omega's 2048 campaign map definition (`Campaign2048_Definition.xml`) reads to the structure
//! 2048's own `newFEshell` does, season cards spelled `<aCanvasLabel>`, and Omega ranks the
//! shared XP total by its own `XPRanks.xml`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^omega_2048_map_ground_truth::/)'
//! ```

use oag_ui::frontend::canvas_cells;

const OMEGA_SCREEN: &str = "Cell Selection 2048";
const OMEGA_ENTRY: &str = r"Data\Plugins\Frontend\Gui\Campaign2048_Definition.xml";

fn labels(
    archives: &mut oag_assets::Archives,
    entry: &str,
    screen: &str,
) -> Vec<oag_ui::screen::CanvasLabel> {
    let blob = archives.read_name(entry).expect("the definition");
    let xml = if oag_tables::fexml::is_fexml(&blob) {
        oag_tables::fexml::expand(&blob).expect("expands")
    } else {
        String::from_utf8(blob).expect("text")
    };
    oag_ui::screen::Screens::from_xml(&xml)
        .by_name(screen)
        .unwrap_or_else(|| panic!("screen {screen}"))
        .canvas_labels
        .clone()
}

/// Omega's canvas labels are 2048's, name for name and cell for cell; the three season cards
/// are the ones Omega spells `aCanvasLabel`, and only they are read through that spelling.
#[test]
#[ignore = "needs the decrypted PS4 package pair and the 2048 EU package"]
fn omegas_definition_reads_to_the_structure_2048s_does() {
    let (Some(omega), Some(vita)) = (
        oag_testdata::exact("data/extracted/ps4"),
        oag_testdata::exact("data/extracted/vita/PCSF00007"),
    ) else {
        return;
    };
    let mut omega = oag_omega::open(&omega.display().to_string()).expect("omega");
    let ours = labels(&mut omega, OMEGA_ENTRY, OMEGA_SCREEN);
    let options = oag_game::boot::Options {
        language: Some("English".to_string()),
        source: vita.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("omega-2048-map-vita"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (shell, _, _) = oag_game::boot::load_shell(&options).expect("the 2048 shell");
    let theirs = shell
        .screens
        .by_name("newFEshell")
        .expect("2048's canvas screen")
        .canvas_labels
        .clone();
    assert!(!ours.is_empty(), "Omega's definition yields labels");
    let cards = canvas_cells::season_cards(&ours);
    assert_eq!(
        cards.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["2048SeasonText", "2049SeasonText", "2050SeasonText"],
        "the aCanvasLabel spelling is read"
    );
    for card in &cards {
        let other = theirs
            .iter()
            .find(|label| label.name == card.name)
            .expect("2048 has the same card");
        assert_eq!(
            (card.cell, card.size, card.linked_event.as_ref()),
            (other.cell, other.size, other.linked_event.as_ref()),
            "{}",
            card.name
        );
    }
    let names = |labels: &[oag_ui::screen::CanvasLabel]| {
        let mut names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
        names.sort();
        names
    };
    assert_eq!(names(&ours), names(&theirs), "same labels as 2048");
}

/// The rank gate reads Omega's own table: its thresholds are half 2048's.
#[test]
#[ignore = "needs the decrypted PS4 package pair and the 2048 EU package"]
fn omegas_rank_table_is_half_2048s() {
    let (Some(omega), Some(vita)) = (
        oag_testdata::exact("data/extracted/ps4"),
        oag_testdata::exact("data/extracted/vita/PCSF00007"),
    ) else {
        return;
    };
    let mut o = oag_omega::open(&omega.display().to_string()).expect("omega");
    let (_, ranks_o) = oag_2048::campaign::xp_tables(&mut o).expect("omega xp");
    let (_, mut v) = (
        (),
        oag_source::title::open_source(&vita.display().to_string(), Vec::new(), Vec::new())
            .expect("2048"),
    );
    let (_, ranks_v) = oag_2048::campaign::xp_tables(&mut v.archives).expect("2048 xp");
    assert_eq!(ranks_o.maximum, ranks_v.maximum);
    for (o, v) in ranks_o.thresholds.iter().zip(&ranks_v.thresholds) {
        assert_eq!(o * 2, *v);
    }
    assert!(ranks_o.meets(ranks_v.thresholds[0] / 2, Some(1)));
    assert!(!ranks_v.meets(ranks_v.thresholds[0] / 2, Some(1)));
}

/// Omega ships 2048's `SP.xml` descriptions, and its hosted map words and centres the card as
/// 2048's does.
#[test]
#[ignore = "needs the decrypted PS4 package pair"]
fn omegas_hosted_card_words_its_description_and_centres_the_pass_group() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let options = oag_game::boot::Options {
        language: Some("English".to_string()),
        source: omega.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-2048-card"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (shell, _, _) = oag_game::boot::load_shell(&options).expect("omega shell");
    assert_eq!(
        shell
            .campaign_events
            .iter()
            .filter(|event| event.card.description.is_some())
            .count(),
        38
    );
    let name = "2048 - Event 1";
    let seed = oag_game::boot::map_seed::MapSeed::of(&shell).expect("Omega hosts the map");
    let mut frontend = seed.open(
        &oag_game::records::Store::default(),
        "omega",
        &options.source,
        String::new(),
    );
    assert!(frontend.select_campaign_event(name));
    let mut input = oag_gameplay::input::Input::new();
    for tick in 0..600u32 {
        if frontend.event_card_open() {
            break;
        }
        let cross = oag_gameplay::input::Button::Cross.bit();
        input.begin_frame(if tick.is_multiple_of(2) { cross } else { 0 });
        frontend.update(1.0 / 60.0, &mut input, None);
    }
    assert!(frontend.event_card_open(), "the card of {name} opens");
    let x = frontend
        .draw_list()
        .iter()
        .find_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, x, .. } if text == "PASS" => Some(*x),
            _ => None,
        })
        .expect("the PASS line");
    assert!(
        x != 552.0,
        "centred by the atlas's widths, not at the fallback x"
    );
}
