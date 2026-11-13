//! Wipeout 2048's season cards on the campaign map: the three
//! `<CanvasLabel>`s `2048SeasonText`, `2049SeasonText` and `2050SeasonText`,
//! each a 14 x 3 block of lattice cells the canvas paints a crop of
//! `canvasTexture.gxt` onto
//! (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
//! "The season cards"). Needs the EU package under
//! `data/extracted/vita/PCSF00007`.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_ui::frontend::{self, canvas_cells};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn options(source: &Path) -> boot::Options {
    boot::Options {
        language: Some("English".to_string()),
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-2048-season-cards-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    }
}

/// The atlas as RGBA, and the cards the disc's own front end authors.
fn disc() -> Option<(Vec<u8>, u32, u32, Vec<canvas_cells::Card>)> {
    let source = source()?;
    let (shell, mut archives, title) = boot::load_shell(&options(&source)).expect("the shell");
    let touch = title
        .front_end
        .and_then(|front_end| front_end.touch.as_ref())
        .expect("a touch front end");
    let screen = shell
        .screens
        .by_name(touch.canvas_screen)
        .expect("the canvas screen");
    let cards = canvas_cells::season_cards(&screen.canvas_labels);
    let blob = archives
        .read_name(touch.canvas_texture)
        .expect("canvasTexture.gxt");
    let gxt = oag_texture::gxt::Gxt::parse(&blob).expect("a .gxt");
    let texture = gxt.only().expect("one texture");
    let rgba: Vec<u8> = texture
        .to_rgba(&blob)
        .expect("decodes")
        .into_iter()
        .flatten()
        .collect();
    Some((
        rgba,
        u32::from(texture.width),
        u32::from(texture.height),
        cards,
    ))
}

/// Pixels of an RGBA image whose blue exceeds red by 40 (the cards' ink).
fn ink(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|p| p[3] > 0 && i32::from(p[2]) > i32::from(p[0]) + 40 && p[0] < 140)
        .count()
}

/// Every card's cells resolve to rectangles inside the decoded atlas, and the
/// 2048 card's crop is where the disc authors it: 14 x 3 cells at (51, 29), the
/// atlas from `v` = 0.4785, holding the wordmark's ink.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_season_cards_resolve_to_atlas_rectangles_decoded_from_the_disc() {
    let Some((atlas, width, height, cards)) = disc() else {
        return;
    };
    assert_eq!((width, height), (2048, 2048));
    let names: Vec<&str> = cards.iter().map(|card| card.name.as_str()).collect();
    assert_eq!(
        names,
        ["2048SeasonText", "2049SeasonText", "2050SeasonText"]
    );
    let first = &cards[0];
    assert_eq!((first.cell, first.size), ((51, 29), (14, 3)));
    assert_eq!(first.origin, (0.0, 0.478_515_625));
    assert_eq!(first.linked_event, None);
    assert_eq!(
        cards[1].linked_event.as_deref(),
        Some("2049 - Event 1"),
        "the 2049 card waits on its season's first event"
    );
    for card in &cards {
        let shifts = canvas_cells::card_shifts(card);
        assert_eq!(shifts.len(), 42, "{}: 14 x 3 cells", card.name);
        // The union of the cells' shifted triangles, in atlas pixels.
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for (cell, shift) in &shifts {
            let at = [shift[0] * width as f32, shift[1] * height as f32];
            // The cell's own atlas triangle: apex up spans x 14..107, apex down 60..155.
            let (left, right) = if canvas_cells::is_up(*cell) {
                (14.0, 107.0)
            } else {
                (60.0, 155.0)
            };
            low = [low[0].min(at[0] + left), low[1].min(at[1] + 40.0)];
            high = [high[0].max(at[0] + right), high[1].max(at[1] + 122.0)];
        }
        assert!(
            low[0] >= 0.0 && low[1] >= 0.0 && high[0] <= width as f32 && high[1] <= height as f32,
            "{}: the crop {low:?}..{high:?} lies in the atlas",
            card.name
        );
        let inked = {
            let (x0, y0, x1, y1) = (low[0] as u32, low[1] as u32, high[0] as u32, high[1] as u32);
            (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    let at = ((y * width + x) * 4) as usize;
                    i32::from(atlas[at + 2]) > i32::from(atlas[at]) + 40 && atlas[at] < 140
                })
                .count()
        };
        assert!(
            inked > 2000,
            "{}: {inked} ink pixels in its crop",
            card.name
        );
        let picture = canvas_cells::bake_card(&atlas, width, height, card).expect("bakes");
        assert!(
            ink(&picture.rgba) > 1500,
            "{}: the baked picture carries the card's ink",
            card.name
        );
    }
}

/// The opening map of a profile that has not reached 2049 draws 2048's card and
/// not the others, at the cells the disc places it.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/ and a GPU adapter"]
fn the_season_2048_card_is_drawn_on_the_map() {
    let Some(source) = source() else { return };
    let scratch = std::env::temp_dir().join(format!("oag-season-card-{}", std::process::id()));
    let records = scratch.join("config/oag");
    std::fs::create_dir_all(&records).expect("creating the scratch directory");
    std::fs::write(
        records.join("settings.toml"),
        "[dev]\nunlock_everything = false\n",
    )
    .expect("writing the settings");
    let out = scratch.join("map.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&source)
        .args([
            "--until",
            "map:2048 - Event 1",
            "--press",
            "cross",
            "--ticks",
            "1",
        ])
        .args(["--no-audio", "--no-video", "--size", "960x544"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --until map");
    let png = std::fs::read(&out).expect("reading the screenshot");
    std::fs::remove_dir_all(&scratch).ok();
    let pixels = pixels_of(&png);
    // The card's wordmark sits left of the first tile and above the lattice's
    // rings: frame 12 of the original has about 16,000 ink pixels there.
    let mut inked = 0;
    for y in 150..300_usize {
        for x in 0..420_usize {
            let at = (y * 960 + x) * 4;
            let p = &pixels[at..at + 4];
            if i32::from(p[2]) > i32::from(p[0]) + 40 && p[0] < 140 {
                inked += 1;
            }
        }
    }
    assert!(inked > 8000, "the card's ink: {inked} pixels");
}

/// The frame's RGBA pixels: the capture writes stored (uncompressed) deflate
/// blocks, so a reader needs only the chunk and block framing.
fn pixels_of(png: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = usize::from(u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]));
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = 1 + 960 * 4;
    assert_eq!(raw.len(), stride * 544, "an RGBA PNG of the frame's size");
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].iter().copied())
        .collect()
}
