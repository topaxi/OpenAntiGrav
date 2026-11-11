//! A PPSSPP texture pack is drawn in place of the disc's textures on a Pulse race.
//!
//! **`#[ignore]`d and never run in CI.** Needs `pulse-psp-eu.chd` and the Pulse pack
//! (`just fetch-texture-packs`); skips when either is absent.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^texture_pack_ground_truth::/)'
//! ```
//!
//! The pack is switched on through [`race::Options::find_texture_packs`], the one door the
//! composition root uses, and what it changed is read off the loaded track model: the size
//! of each texture slot. A pack's pictures are a multiple of the disc's, so a slot that
//! grew is a slot the pack replaced; nothing in the report or a counter stands in for it.

use std::path::PathBuf;
use std::sync::Arc;

use oag_raceplay as race;
use oag_texture::replacer::{Files, Replacements};

const PACK: &str = "WipEout-Pulse-Upscale-Version";

struct Dir(PathBuf);

impl Files for Dir {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(name)).ok()
    }
}

fn pack_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/texture-packs")
        .join(PACK);
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

/// A race on the disc's default circuit, with `serials` rewritten into the pack's `[games]`
/// when given, and the pack on at all only when `with_pack`.
fn load(
    dir: &std::path::Path,
    with_pack: bool,
    games: Option<&'static str>,
) -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let dir = dir.to_path_buf();
    let finder = with_pack.then(|| {
        race::texture_pack::Finder(Arc::new(move || {
            let mut ini = std::fs::read_to_string(dir.join("textures.ini")).unwrap();
            if let Some(games) = games {
                ini = ini.replace("UCES00465 = textures.ini", games);
            }
            vec![
                Replacements::new("pulse pack", &ini, Box::new(Dir(dir.clone())))
                    .map(Arc::new)
                    .map_err(|why| why.to_string()),
            ]
        }))
    });
    Some(
        race::load(&race::Options {
            source: image.display().to_string(),
            find_texture_packs: finder,
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// `(width, height)` of every slot of the track model, in slot order.
fn sizes(loaded: &race::Loaded) -> Vec<Option<(u32, u32)>> {
    loaded
        .track_model
        .textures
        .iter()
        .map(|slot| slot.as_ref().map(|t| (t.width, t.height)))
        .collect()
}

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn a_pack_that_is_on_replaces_track_textures_with_larger_pictures_and_off_changes_nothing() {
    let Some(dir) = pack_dir() else { return };
    let (Some(off), Some(on)) = (load(&dir, false, None), load(&dir, true, None)) else {
        return;
    };
    assert!(
        on.report
            .iter()
            .any(|line| line.ends_with("from the texture pack")),
        "a whole-image .mip (a flare) draws from the pack: {:#?}",
        on.report
    );
    assert!(!off.report.iter().any(|line| line.contains("texture pack")));
    let (off, on) = (sizes(&off), sizes(&on));
    assert_eq!(
        off.len(),
        on.len(),
        "slots are positional and never compacted"
    );
    let grown = off
        .iter()
        .zip(&on)
        .filter(|(disc, pack)| match (disc, pack) {
            (Some((dw, dh)), Some((pw, ph))) => pw > dw && ph >= dh && pw % dw == 0,
            _ => false,
        })
        .count();
    assert!(
        grown > 20,
        "the pack replaced only {grown} track texture(s)"
    );
    assert!(
        on.iter()
            .zip(&off)
            .all(|(pack, disc)| pack.is_some() == disc.is_some()),
        "a replacement fills the slot it replaces and no other"
    );
}

const HUD_SHEET: &str = "Data\\HUD\\Textures\\PulseHUD.mip";

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn a_pack_that_is_on_draws_the_hud_sheet_and_fonts_at_its_own_size_and_off_changes_nothing() {
    let Some(dir) = pack_dir() else { return };
    let (Some(off), Some(on)) = (load(&dir, false, None), load(&dir, true, None)) else {
        return;
    };
    // `pulse_text.fnt`, the flyby text and the Eliminator kill column: white ink whose upscaled
    // edge rounds to 254, over white under zero alpha. Read as outlined, its body pass drew
    // every glyph box solid.
    let panel = on
        .track_panel
        .as_ref()
        .expect("the default circuit has a flyby panel");
    for (name, face) in [
        ("panel menu", &panel.menu_font),
        ("panel default", &panel.default_font),
    ] {
        assert!(face.texel_scale < 1.0, "{name} is the pack's picture");
        assert!(!face.is_outlined(), "{name}: a pack face is one grey");
        assert!(
            face.coverage
                .iter()
                .zip(&face.luma)
                .all(|(&coverage, &luma)| coverage > 0 || luma == 0),
            "{name}: no body under zero coverage"
        );
    }
    let (off, on) = (&off.hud, &on.hud);

    assert_eq!(off.sheet.scale(HUD_SHEET), [1.0, 1.0]);
    let [sx, sy] = on.sheet.scale(HUD_SHEET);
    assert!(sx > 1.0 && sx == sy, "the HUD sheet was drawn at {sx}x{sy}");
    let (disc, pack) = (
        off.sheet.get(HUD_SHEET).unwrap(),
        on.sheet.get(HUD_SHEET).unwrap(),
    );
    assert_eq!(
        (disc.width, disc.height),
        (pack.width, pack.height),
        "the placement stays in the disc's pixels"
    );
    assert_eq!(on.sheet.width as f32, off.sheet.width as f32 * sx);

    for (name, disc, pack) in [
        ("HUD", &off.font, &on.font),
        ("small", &off.small_font, &on.small_font),
    ] {
        assert_eq!(disc.texel_scale, 1.0, "{name}");
        let factor = pack.width / disc.width;
        assert!(factor > 1, "{name} atlas is {}px wide", pack.width);
        assert_eq!(pack.texel_scale, 1.0 / factor as f32, "{name}");
        assert_eq!(pack.line_height, disc.line_height, "{name}: grid units");
        for ch in ['0', 'A', 'k'] {
            let (a, b) = (disc.cell(ch).unwrap(), pack.cell(ch).unwrap());
            assert_eq!(
                (b.width, b.height, b.advance),
                (
                    a.width * factor,
                    a.height * factor,
                    a.advance * factor as f32
                ),
                "{name} {ch:?}: the same glyph on a {factor}x atlas"
            );
        }
    }
}

#[test]
#[ignore = "needs pulse-psp-eu.chd and the Pulse pack"]
fn a_pack_that_does_not_name_the_discs_serial_is_not_used_and_says_so() {
    let Some(dir) = pack_dir() else { return };
    let (Some(plain), Some(other)) = (
        load(&dir, false, None),
        load(&dir, true, Some("UCUS98712 = textures.ini")),
    ) else {
        return;
    };
    assert_eq!(
        sizes(&plain),
        sizes(&other),
        "another game's pack changes nothing"
    );
    assert!(
        other
            .report
            .iter()
            .any(|line| line.starts_with("texture pack not used: pulse pack")
                && line.contains("UCES-00465")),
        "{:#?}",
        other.report
    );
    assert!(oag_texture::replacer::active().is_none());
}
