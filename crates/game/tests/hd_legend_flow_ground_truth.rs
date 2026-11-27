//! HD's footer legend is laid out by the original's flow law, read off the real skin, strings and
//! faces (`docs/ghidra/functions/ps3-hdfury-eu/navigation-legend.md`).
//!
//! The authored x of the prompts (490, 534, 720, 764, 900, 944) is not where the original draws
//! them: `NavigationController` places each from the `NAVIGATION` word on. The numbers asserted
//! are the law's prediction for the layout's authored 1920x1080 units; on `hd/menu-cell-select`
//! they land at screen x 460, 502, 639, 682, 771 and 814, which the frame reads at 460, 502, 639,
//! 682, 771 and 814.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^hd_legend_flow_ground_truth::/)'
//! ```

use oag_game::boot;
use oag_ui::frontend::{self, Draw};

/// What `NavigationController` lays out on the Cell Selection page: glyph and word of `Confirm`,
/// `Back` and the registered `Difficulty` prompt.
const LAW: [f32; 6] = [439.0, 483.0, 626.0, 670.0, 763.0, 807.0];

fn options(image: &std::path::Path) -> boot::Options {
    boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("hd-legend-flow-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    }
}

fn text_x(draw: &Draw) -> Option<(&str, f32)> {
    match draw {
        Draw::Text { text, x, .. } | Draw::FacedText { text, x, .. } => Some((text, *x)),
        _ => None,
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_legend_and_the_difficulty_prompt_flow_by_the_originals_law() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let (shell, mut archives, title) =
        boot::load_shell(&options(&image)).expect("HD's front end is wired");
    let legend = shell
        .nav_legend
        .as_ref()
        .expect("HD's skin authors a legend");
    assert!(legend.is_flowed(), "HD's title data flows its legend");
    let draws = legend.draw(&oag_ui_screens::picker::FaceScales::default());
    let xs: Vec<f32> = draws.iter().filter_map(text_x).map(|(_, x)| x).collect();
    // Navigation's two Texts keep their authored x (160 and 260); the controller's four follow.
    for (law, got) in LAW[..4].iter().zip(xs.iter().filter(|x| **x > 400.0)) {
        assert!(
            (law - got).abs() < 1.0,
            "legend prompt {got} against the law's {law}"
        );
    }
    assert!(xs.contains(&260.0), "NAVIGATION is the anchor: {xs:?}");

    let grid = [shell.space.size.0, shell.space.size.1];
    let globals: Vec<(&str, &str)> = shell
        .screens
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let tracks = shell.tracks.clone();
    let mut campaign = oag_game::campaign::load(
        &mut archives,
        &shell.strings,
        oag_ui_screens::picker::FaceScales::default(),
        grid,
        &shell.sprites,
        &globals,
        title,
        &tracks,
    )
    .expect("the campaign loads");
    let law = title
        .front_end
        .and_then(|front_end| front_end.legend_flow)
        .expect("HD's title data carries the flow");
    campaign.flow_footer(
        law,
        &boot::fonts::legend_ink(&shell.font, shell.buttons_font.as_ref()),
    );
    let x = |name: &str| {
        campaign
            .cell_layout
            .screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("Cell Selection authors {name}"))
            .x
    };
    for (name, want) in [
        ("DifficultyButtonIcon", LAW[4]),
        ("DifficultyButton", LAW[5]),
    ] {
        assert!(
            (x(name) - want).abs() < 1.0,
            "{name} at {} against the law's {want}",
            x(name)
        );
    }
    let campaign_legend = campaign
        .nav_legend
        .as_ref()
        .expect("the campaign carries a legend");
    assert!(campaign_legend.is_flowed());
}
