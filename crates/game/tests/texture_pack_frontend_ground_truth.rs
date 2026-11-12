//! A PPSSPP texture pack is drawn in the front end's faces, as it already is in the race HUD's.
//!
//! **`#[ignore]`d and never run in CI.** Needs `pulse-psp-eu.chd` and the Pulse pack
//! (`just fetch-texture-packs`); skips when either is absent.
//!
//! The pack is switched on through [`boot::Options::find_texture_packs`], the one door the
//! composition root uses, and what it changed is read off the faces the boot built: a
//! replaced face's atlas is the picture's size and each of its texels counts `1/f` grid units
//! (`oag_ui::font::Atlas::from_replaced_font`), a disc face's is the `.fnt`'s and counts one.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use oag_game::boot;
use oag_raceplay::texture_pack::Finder;
use oag_texture::replacer::{Files, Replacements};

struct Dir(PathBuf);

impl Files for Dir {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(name)).ok()
    }
}

fn pack_dir(name: &str) -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/texture-packs")
        .join(name);
    if dir.join("textures.ini").is_file() {
        Some(dir)
    } else {
        eprintln!(
            "skip: {} is absent (just fetch-texture-packs)",
            dir.display()
        );
        None
    }
}

fn finder(dir: &Path) -> Finder {
    let dir = dir.to_path_buf();
    Finder(Arc::new(move || {
        let ini = std::fs::read_to_string(dir.join("textures.ini")).unwrap();
        vec![
            Replacements::new("test pack", &ini, Box::new(Dir(dir.clone())))
                .map(Arc::new)
                .map_err(|why| why.to_string()),
        ]
    }))
}

/// The front end's shell for `image`, the pack on when `pack` names one.
fn shell(image: &str, pack: Option<&Path>) -> Option<boot::Shell> {
    let image = oag_testdata::image(image)?;
    let options = boot::Options {
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::default(),
        language: None,
        movie: None,
        cache: std::env::temp_dir().join("oag-texture-pack-frontend"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: pack.map(finder),
    };
    let (shell, _archives, _title) = boot::load_shell(&options).expect("the shell");
    Some(shell)
}

fn faces(shell: &boot::Shell) -> Vec<&oag_ui::font::Atlas> {
    std::iter::once(&shell.font)
        .chain(shell.menu_font.as_ref())
        .chain(shell.title_font.as_ref())
        .chain(shell.buttons_font.as_ref())
        .collect()
}

fn from_pack(shell: &boot::Shell) -> Vec<&String> {
    shell
        .report
        .iter()
        .filter(|line| line.contains("from the texture pack"))
        .collect()
}

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn a_pack_that_is_on_draws_the_front_ends_default_face_and_off_changes_nothing() {
    let Some(dir) = pack_dir("WipEout-Pulse-Upscale-Version") else {
        return;
    };
    let (Some(off), Some(on)) = (
        shell("data/images/pulse-psp-eu.chd", None),
        shell("data/images/pulse-psp-eu.chd", Some(&dir)),
    ) else {
        return;
    };
    assert!(
        !off.report.iter().any(|line| line.contains("texture pack")),
        "off says nothing of a pack: {:#?}",
        off.report
    );
    assert!(
        faces(&off)
            .iter()
            .all(|face| (face.texel_scale - 1.0).abs() < f32::EPSILON),
        "off draws the disc's faces"
    );
    assert!(
        from_pack(&on)
            .iter()
            .any(|line| line.starts_with("font ") && line.contains("pulse_text")),
        "the default face (`pulse_text.fnt`) is drawn from the pack: {:#?}",
        on.report
    );
    assert!(
        on.font.width > off.font.width && on.font.texel_scale < 1.0,
        "the default face is the pack's picture at {}x{}, each texel {} grid units",
        on.font.width,
        on.font.height,
        on.font.texel_scale
    );
    // A face keeps its size on screen and its line: only its texels grow.
    assert!((on.font.line_height - off.font.line_height).abs() < f32::EPSILON);
    assert!(
        oag_texture::replacer::active().is_none(),
        "the pack is switched off when the faces are built, not held into the session"
    );
}

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn every_face_the_pack_names_is_drawn_from_it_and_keeps_its_line_height() {
    let Some(dir) = pack_dir("WipEout-Pulse-Upscale-Version") else {
        return;
    };
    let (Some(off), Some(on)) = (
        shell("data/images/pulse-psp-eu.chd", None),
        shell("data/images/pulse-psp-eu.chd", Some(&dir)),
    ) else {
        return;
    };
    let (off, on) = (faces(&off), faces(&on));
    assert_eq!(off.len(), on.len());
    for (disc, pack) in off.iter().zip(&on) {
        assert!(
            (disc.line_height - pack.line_height).abs() < f32::EPSILON,
            "a face keeps its line height"
        );
        assert!(
            pack.width >= disc.width,
            "a face's picture is never smaller than the disc's atlas"
        );
    }
    assert!(
        on.iter().filter(|face| face.texel_scale < 1.0).count() >= 2,
        "more than one of the front end's faces is the pack's"
    );
}

/// The side every backend we draw on can make a texture (wgpu's default limit, which the web
/// and most phones are held to), and the Steam Deck's.
const SIDES: [u32; 2] = [8192, 16_384];

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn no_face_a_pack_draws_is_taller_or_wider_than_a_device_can_make() {
    let Some(dir) = pack_dir("WipEout-Pulse-Upscale-Version") else {
        return;
    };
    let Some(on) = shell("data/images/pulse-psp-eu.chd", Some(&dir)) else {
        return;
    };
    for face in faces(&on) {
        eprintln!("face {}x{}", face.width, face.height);
        for side in SIDES {
            assert!(
                face.width <= side && face.height <= side,
                "a {}x{} face does not fit a device whose textures are {side} a side: {:#?}",
                face.width,
                face.height,
                on.report
            );
        }
    }
}

const ASSETS: &str = r"Data\FE\Images\pulse_assets.mip";

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn a_pack_that_is_on_draws_the_front_ends_sheet_art_and_asks_for_the_larger_picture() {
    let Some(dir) = pack_dir("WipEout-Pulse-Upscale-Version") else {
        return;
    };
    let (Some(off), Some(on)) = (
        shell("data/images/pulse-psp-eu.chd", None),
        shell("data/images/pulse-psp-eu.chd", Some(&dir)),
    ) else {
        return;
    };
    assert_eq!(off.sprites.scale(ASSETS), [1.0, 1.0], "off: the disc's art");
    let [sx, sy] = on.sprites.scale(ASSETS);
    assert!(
        sx > 1.0 && sy > 1.0,
        "on: the pack's picture of {ASSETS} ({sx}x{sy}): {:#?}",
        on.report
    );
    // The layout's own pixels stay put: the placement keeps the disc's size.
    assert_eq!(
        on.sprites.get(ASSETS).unwrap().width,
        off.sprites.get(ASSETS).unwrap().width
    );
    // A frame mark cut off that picture is drawn where it was and samples the larger one.
    let marks = |shell: &boot::Shell| -> Vec<([f32; 4], [f32; 4])> {
        shell
            .frame
            .marks
            .iter()
            .filter_map(|draw| match draw {
                oag_ui::frontend::Draw::Sprite { rect, uv, .. } => Some((*rect, *uv)),
                _ => None,
            })
            .collect()
    };
    let (disc, pack) = (marks(&off), marks(&on));
    assert!(
        !disc.is_empty() && disc.len() == pack.len(),
        "{disc:?} against {pack:?}"
    );
    assert!(
        disc.iter().zip(&pack).any(|(disc, pack)| {
            disc.0 == pack.0 && (pack.1[2] - disc.1[2] * sx).abs() < 0.01 && pack.1[2] > disc.1[2]
        }),
        "a mark samples {sx}x the texels it did and draws the same rect: {disc:?} against {pack:?}"
    );
    for side in SIDES {
        assert!(
            on.sprites.width <= side && on.sprites.height <= side,
            "a {}x{} sheet does not fit a device whose textures are {side} a side",
            on.sprites.width,
            on.sprites.height
        );
    }
}

#[test]
#[ignore = "needs pure-psp-eu.chd and the Pure pack"]
fn pure_keeps_the_discs_faces_with_a_pack_on_because_its_palette_moves() {
    let Some(dir) = pack_dir("WipEout-Pure-HD-Texture-Pack") else {
        return;
    };
    let (Some(off), Some(on)) = (
        shell("data/images/pure-psp-eu.chd", None),
        shell("data/images/pure-psp-eu.chd", Some(&dir)),
    ) else {
        return;
    };
    assert!(
        from_pack(&on).is_empty(),
        "a runtime-palette title's faces are not asked of a pack: {:#?}",
        on.report
    );
    assert!(
        on.report
            .iter()
            .any(|line| line.contains("rewrites at run time")),
        "and the report says why: {:#?}",
        on.report
    );
    for (disc, pack) in faces(&off).iter().zip(faces(&on)) {
        assert_eq!(disc.width, pack.width);
        assert!((pack.texel_scale - 1.0).abs() < f32::EPSILON);
    }
    assert_eq!(
        (on.sprites.width, on.sprites.height),
        (off.sprites.width, off.sprites.height),
        "nor is its sprite sheet"
    );
}
