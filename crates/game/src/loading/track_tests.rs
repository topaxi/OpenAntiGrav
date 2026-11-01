//! A track's own loading screen, off a hand-built title and a folder written here.

use super::layout_tests::hd_screen_assets;
use super::*;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-track-loading-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(path: &std::path::Path, side: u32) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), side, side);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer
        .write_image_data(&vec![200u8; (side * side * 4) as usize])
        .unwrap();
}

const MINI: &str = "screen = \"loading\"\n[[layout]]\nname = \"mini\"\ngrid = [100, 100]\n\
[[layout.element]]\nid = \"c\"\ndraw = \"text\"\nsource = \"caption\"\nat = [1, 2]\n\
[[layout.element]]\nid = \"t\"\ndraw = \"text\"\nsource = \"feature.text\"\nat = [1, 20]\n";

fn texts(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } | Draw::FacedText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn drawn(screen: &Screen) -> Vec<Draw> {
    screen.draw_list(
        Phase::Race,
        &Progress {
            current: Some("Vineta K".into()),
            ..Progress::default()
        },
        &Atlas::build(),
    )
}

#[test]
fn a_track_supplies_its_caption_its_feature_its_layout_and_its_picture() {
    let dir = folder("all");
    std::fs::write(dir.join("run.loading.toml"), MINI).unwrap();
    png(&dir.join("run.png"), 16);
    let track = TrackLoading::read(
        &dir,
        &oag_trackfile::Loading {
            layout: Some("run.loading.toml".into()),
            caption: Some("ASTEROID RUN".into()),
            feature_image: Some("run.png".into()),
            feature_text: Some("The outer ring has no barrier.".into()),
            ..oag_trackfile::Loading::default()
        },
    );
    assert!(track.notes.is_empty(), "{:?}", track.notes);
    let assets = hd_screen_assets();
    let (_, picture) = track.sheet(&assets).expect("the picture is in the sheet");
    assert_eq!((picture[2], picture[3]), (16.0, 16.0));
    let mut screen = Screen::new(&assets, AUTHORED_LINE_HEIGHT, 0, None);
    screen.apply_track(&track, Some(picture));
    let said = texts(&drawn(&screen));
    assert_eq!(
        said,
        ["LOADING... ASTEROID RUN", "The outer ring has no barrier."],
        "the track's layout draws only what it names, with the track's own words"
    );
    assert_eq!(
        screen.space().size,
        (100.0, 100.0),
        "in the track's own grid"
    );
}

#[test]
fn what_a_track_leaves_out_stays_the_titles() {
    let track = TrackLoading::read(
        &folder("none"),
        &oag_trackfile::Loading {
            caption: Some("ASTEROID RUN".into()),
            ..oag_trackfile::Loading::default()
        },
    );
    let mut screen = Screen::new(&hd_screen_assets(), AUTHORED_LINE_HEIGHT, 0, None);
    screen.apply_track(&track, None);
    let said = texts(&drawn(&screen));
    assert!(
        said.contains(&"LOADING... ASTEROID RUN".to_string()),
        "{said:?}"
    );
    // the title's feature is still drawn: the track named no feature key
    assert!(said.contains(&"PILOT ASSIST".to_string()), "{said:?}");
    assert_eq!(screen.space().size, (1920.0, 1080.0), "the title's grid");
}

#[test]
fn naming_a_feature_key_makes_the_track_the_feature_and_a_missing_one_draws_blank() {
    let track = TrackLoading::read(
        &folder("feature"),
        &oag_trackfile::Loading {
            feature_title: Some("NO WALLS".into()),
            ..oag_trackfile::Loading::default()
        },
    );
    let mut screen = Screen::new(&hd_screen_assets(), AUTHORED_LINE_HEIGHT, 0, None);
    screen.apply_track(&track, None);
    let said = texts(&drawn(&screen));
    assert!(said.contains(&"NO WALLS".to_string()), "{said:?}");
    for title_only in ["PILOT ASSIST", "DESCRIPTION"] {
        assert!(
            !said.iter().any(|t| t == title_only),
            "{title_only} in {said:?}"
        );
    }
    assert!(
        !said.iter().any(|t| t.starts_with("Pilot Assist can")),
        "no other feature's paragraph: {said:?}"
    );
    assert_eq!(screen.feature_slot(), None);
}

#[test]
fn a_layout_or_picture_that_will_not_read_is_said_and_the_titles_stands() {
    let dir = folder("bad");
    std::fs::write(dir.join("bad.toml"), "screen = \"split-hud\"\n").unwrap();
    std::fs::write(dir.join("bad.png"), b"not a png").unwrap();
    let track = TrackLoading::read(
        &dir,
        &oag_trackfile::Loading {
            layout: Some("bad.toml".into()),
            feature_image: Some("bad.png".into()),
            ..oag_trackfile::Loading::default()
        },
    );
    assert_eq!(track.notes.len(), 2, "{:?}", track.notes);
    assert!(track.layout.is_none() && track.image.is_none());
    let mut screen = Screen::new(&hd_screen_assets(), AUTHORED_LINE_HEIGHT, 0, None);
    screen.apply_track(&track, None);
    assert_eq!(screen.space().size, (1920.0, 1080.0), "the title's layout");
    assert_eq!(
        track.sheet(&hd_screen_assets()).map(|_| ()),
        None,
        "no picture decoded, none added"
    );
}
