//! The race box's two selection screens against the disc they are read off:
//! `Track Creation` and `Team Selection` in `Selection_Definition.xml`, the
//! ratings in `Definition.xml`, and the two preview meshes each screen shows.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.
//!
//! # What this pins that the unit tests cannot
//!
//! `oag_ui::picker`'s own tests parse a hand-written miniature of the file.
//! This reads the real one through the same boot the game uses, so the two
//! parser gaps that hid the screens for a day - `<Item>`/`<LeftLayer>` not
//! being walked, and `OffsetX`/`OffsetY` not being summed - would fail here
//! rather than draw an empty panel. The numbers are the XML's own, checked
//! against `docs/ui/selection-screens.md`'s capture.

use std::path::PathBuf;

use oag_game::boot;
use oag_ui::picker::{Details, Entry, Kind, Picker};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn shell(path: &std::path::Path) -> boot::Shell {
    let options = boot::Options {
        source: path.to_string_lossy().into_owned(),
        language: None,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-selection-screens-ground-truth"),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        // Every assertion here is about the XML and two meshes; nothing pays
        // for a transcode.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    boot::load_shell(&options).expect("the shell loads").0
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_screens_read_with_their_panel_at_the_authored_offset() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let track = shell.track_select.as_ref().expect("Track Creation reads");
    let ship = shell.ship_select.as_ref().expect("Team Selection reads");

    // `<LeftLayer OffsetX="290" OffsetY="25">` holds `Infogradient` at
    // `y="1"`, so the panel is at (290, 26); `Team Selection`'s layer is
    // `OffsetY="45"`, so its panel is twenty lower.
    assert_eq!(track.panel, [290.0, 26.0, 170.0, 200.0]);
    assert_eq!(ship.panel, [290.0, 46.0, 170.0, 162.0]);

    // The three stat rules under `<Item name="line bg1" OffsetY="142">` and
    // its siblings: two 85-wide gradient halves each, at the layer's x.
    let rules: Vec<(f32, f32)> = track
        .screen
        .fills
        .iter()
        .filter(|fill| fill.gradient.is_some() && fill.height == Some(14.0))
        .map(|fill| (fill.x, fill.y))
        .collect();
    assert_eq!(
        rules,
        vec![
            (290.0, 167.0),
            (375.0, 167.0),
            (290.0, 182.0),
            (375.0, 182.0),
            (290.0, 197.0),
            (375.0, 197.0)
        ],
        "{rules:?}"
    );

    // The four rating bars, `<Item OffsetY="44">` and on, at x="65" y="3"
    // inside the `OffsetX="14"` group under the `OffsetY="45"` layer:
    // 290 + 14 + 65 across, 45 + 44 + 3 down.
    let bars: Vec<(String, f32, f32)> = ship
        .screen
        .images
        .iter()
        .filter(|image| image.name.as_deref().is_some_and(|n| n.ends_with(" Bar")))
        .map(|image| (image.name.clone().unwrap(), image.x, image.y))
        .collect();
    assert_eq!(
        bars,
        vec![
            ("Speed Bar".to_string(), 369.0, 92.0),
            ("Thrust Bar".to_string(), 369.0, 107.0),
            ("Handling Bar".to_string(), 369.0, 122.0),
            ("Shield Bar".to_string(), 369.0, 137.0),
            ("Loyalty Bar".to_string(), 304.0, 181.0),
        ],
        "{bars:?}"
    );
    // The livery arrows are children of the `skin` text widget.
    assert!(
        ship.screen
            .images
            .iter()
            .any(|image| image.name.as_deref() == Some("skin left arrow")),
        "{:?}",
        ship.screen.images
    );
    // Every image the two screens name is in the sheet, `hex_bg.mip` included.
    for image in track.screen.images.iter().chain(&ship.screen.images) {
        assert!(
            shell.sprites.get(&image.src).is_some(),
            "{} is not in the sprite sheet",
            image.src
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ratings_are_the_ones_the_capture_shows() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let rating = |id: &str| {
        shell
            .teams
            .iter()
            .find(|team| team.id == id)
            .and_then(|team| team.rating)
            .map(|r| [r.speed, r.thrust, r.handling, r.shield])
    };
    // `docs/ui/selection-screens.md`: the four bars on the 2026-09-09
    // PPSSPP capture, team by team.
    assert_eq!(rating("Assegai"), Some([8, 8, 9, 7]));
    assert_eq!(rating("Qirex"), Some([8, 7, 8, 9]));
    assert_eq!(rating("AG_Systems"), Some([7, 9, 9, 8]));
    assert_eq!(rating("Piranha"), Some([10, 6, 6, 9]));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_preview_meshes_resolve_and_are_what_the_screens_show() {
    let Some(path) = image() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    // The circuit outline: a single ribbon at track scale, not a flythrough
    // - its radius is the circuit's own, hundreds of units, on 364 vertices.
    let blob = archives
        .read_name(r"Data\Environments\16_Track\FE\forward.vex")
        .expect("the outline resolves");
    let outline = oag_render::mesh::build("outline", &blob).expect("it decodes");
    assert_eq!(outline.vertices.len(), 364);
    assert!(
        outline.radius > 500.0,
        "track scale, not a corridor: radius {}",
        outline.radius
    );
    // The craft: the front-end hull, a few units across.
    let blob = archives
        .read_name(r"Data\Ships\Assegai\ship_FE.vex")
        .expect("the hull resolves");
    let hull = oag_render::mesh::build("hull", &blob).expect("it decodes");
    assert!(hull.vertices.len() > 1000, "{}", hull.vertices.len());
    assert!(hull.radius < 20.0, "craft scale: radius {}", hull.radius);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_track_picker_draws_the_discs_own_rows() {
    let Some(path) = image() else {
        return;
    };
    let shell = shell(&path);
    let layout = shell.track_select.as_ref().expect("Track Creation reads");
    let entries: Vec<Entry> = shell
        .tracks
        .iter()
        .map(|track| Entry {
            id: track.id.clone(),
            label: shell.strings.get_or_id(&track.id).to_string(),
            details: Details::Track {
                info: ["-".into(), "-".into(), "-".into()],
            },
        })
        .collect();
    let picker = Picker::new(Kind::Track, entries, Some("16_Track"), None);
    let face = shell.menu_font.as_ref().unwrap_or(&shell.font);
    let skin = oag_ui::menu::Skin::new(shell.menu_skin, shell.space, face.line_height);
    let layers = oag_ui::picker::draw_list(
        &picker,
        layout,
        &skin,
        &shell.frame,
        None,
        false,
        &|src| shell.sprites.get(src),
        &|text| oag_ui::font::measure(face, text),
    );
    let texts: Vec<String> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    // The counter - over the disc's own circuits, no pack mounted here -
    // the three stat titles off the string table, and the circuit's own
    // name: none of them spelled in this repository.
    let counter = format!("1 / {}", shell.tracks.len());
    assert!(texts.contains(&counter), "{counter}: {texts:?}");
    for id in ["IG_HUD_DISTANCE", "IG_HUD_LAP_REC", "ER_RR"] {
        let title = shell
            .strings
            .get(id)
            .expect("the string table names the row");
        assert!(texts.iter().any(|t| t == title), "{title:?} in {texts:?}");
    }
    let sprites = layers
        .body
        .iter()
        .filter(|draw| matches!(draw, oag_ui::frontend::Draw::Sprite { .. }))
        .count();
    assert_eq!(sprites, 3, "up arrow, down arrow, hex grid");
}
