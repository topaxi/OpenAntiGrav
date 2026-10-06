//! Omega's menu blocks: the box behind each strip entry draws off Omega's own
//! `.gnf` art, the right way up, the way HD's draws off its `.gtf`.
//!
//! **The reference is HD's front end on this engine, not a PS4.** No PS4
//! emulator exists in this project's toolchain, so "Omega draws like HD" means
//! the decoded art in the sheet is the same picture and the block table is
//! HD's, which is `chosen, not measured` for Omega
//! (`oag_omega::frontend::MENU_BLOCKS`).
//!
//! What this fails on: dropping `MENU_SKIN.blocks`, or dropping the row
//! reversal `FrontEnd::bottom_up_gnf` asks for - Omega's `file2.gnf` is HD's
//! `file2.gtf` in HD's bottom-up file order, so unreversed the nine-patch is
//! mirrored and its fill swatch reads alpha 0.004 (the 2026-10-06 report: menu
//! items drawn as bare outlines with no fill).

use std::path::PathBuf;

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

fn hd() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn shell(source: &std::path::Path) -> oag_game::boot::Shell {
    let options = oag_game::boot::Options {
        language: None,
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-menu-blocks-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    oag_game::boot::load_shell(&options)
        .expect("the front end boots")
        .0
}

/// Every texel's alpha of `name` in `shell`'s sheet, row by row.
fn alphas(shell: &oag_game::boot::Shell, name: &str) -> Vec<u8> {
    let placed = shell.sprites.get(name).expect("in the sheet");
    let (w, h) = (placed.width, placed.height);
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = shell
                .sprites
                .alpha_at(
                    placed,
                    (x as f32 + 0.5) / w as f32,
                    (y as f32 + 0.5) / h as f32,
                )
                .expect("inside the sheet");
            out.push((a * 255.0).round() as u8);
        }
    }
    out
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_menu_blocks_draw_with_the_swatch_fill_the_executable_samples() {
    let Some(source) = omega() else { return };
    let shell = shell(&source);
    assert!(
        oag_omega::frontend::MENU_SKIN.blocks.is_some(),
        "Omega's skin carries blocks"
    );
    assert!(
        oag_omega::frontend::FRONT_END
            .bottom_up_gnf
            .contains(&"file2")
    );

    let art = shell
        .frame
        .blocks
        .expect("the nine-patch decoded, so the frame carries block art");
    assert!(
        (art.fill_alpha - 110.0 / 255.0).abs() < 2.0 / 255.0,
        "the swatch texel is alpha 109 (BC7) against HD's 110, not the 0.004 an unreversed sheet reads: {}",
        art.fill_alpha
    );
    assert!(
        (art.solid_alpha - 1.0).abs() < 2.0 / 255.0,
        "{}",
        art.solid_alpha
    );
    assert!(art.cursor.is_some() && art.arrow.is_some());
    assert!(
        shell
            .report
            .iter()
            .any(|line| line.starts_with("menu blocks: HD style, fill swatch alpha 0.427")),
        "reported: {:#?}",
        shell.report
    );
}

#[test]
#[ignore = "needs the decrypted PS4 package pair and the HD/Fury image"]
fn omegas_block_art_is_hds_picture_in_the_sheet() {
    let (Some(omega), Some(hd)) = (omega(), hd()) else {
        return;
    };
    let (omega, hd) = (shell(&omega), shell(&hd));
    let blocks = oag_omega::frontend::MENU_SKIN
        .blocks
        .expect("Omega's skin carries blocks");
    for name in [blocks.frame_texture, blocks.cursor_texture] {
        // BC7 is lossy: Omega's transparent texels decode to alpha 0 or 1 and
        // its swatch 109 against HD's 110, so equal means within 8.
        let (o, h) = (alphas(&omega, name), alphas(&hd, name));
        assert_eq!(o.len(), h.len(), "{name}");
        let off = o
            .iter()
            .zip(&h)
            .filter(|(a, b)| a.abs_diff(**b) > 8)
            .count();
        assert_eq!(
            off, 0,
            "{name}: {off} texels of Omega's sheet picture differ from HD's"
        );
    }
}

fn body(
    menu: &oag_ui::menu::Menu,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
) -> Vec<oag_ui::frontend::Draw> {
    oag_ui::menu::draw_list(
        menu,
        skin,
        &|_| Vec::new(),
        &|text| text.len() as f32 * 20.0,
        None,
        frame,
        false,
    )
    .flatten()
}

/// The widest filled rectangle short of the page's own clear, which is the
/// focused block's body.
fn widest_fill(draws: &[oag_ui::frontend::Draw]) -> f32 {
    draws
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Fill { rect, .. } if rect[2] < 1000.0 => Some(rect[2]),
            _ => None,
        })
        .fold(0.0, f32::max)
}

/// What the maintainer reported as "they do not animate": with no block art
/// Omega's strip took the measured-tab path, whose tabs are fixed to their
/// text. With it the focused block eases wider at one sixth a tick and the
/// others narrow, through the same `Menu::focus_of` HD's strip is driven by.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_strip_blocks_ease_when_the_focus_moves() {
    use oag_gameplay::input::{Button, Input};
    let Some(source) = omega() else { return };
    let shell = shell(&source);
    let skin = oag_ui::menu::Skin::new(
        oag_omega::frontend::MENU_SKIN,
        oag_display::space::Space::OMEGA,
        32.0,
    );
    let definition = oag_ui::menu::Definition::parse(
        oag_ui::menu::BUILT_IN,
        &oag_ui::language::StringTable::default(),
    )
    .expect("the built-in menu parses");
    let mut menu = oag_ui::menu::Menu::new(definition);
    menu.set_strip_layout(true);

    let at_rest = body(&menu, &skin, &shell.frame);
    // 298 + 70: the focused block at rest, in the grid Omega authors in.
    assert!(
        (widest_fill(&at_rest) - 368.0).abs() < 40.0,
        "{}",
        widest_fill(&at_rest)
    );
    let pieces = at_rest
        .iter()
        .filter(|draw| {
            !matches!(
                draw,
                oag_ui::frontend::Draw::Fill { .. } | oag_ui::frontend::Draw::Text { .. }
            )
        })
        .count();
    assert!(
        pieces >= 6 * 8,
        "six blocks of eight border pieces, not six flat tabs: {pieces}"
    );

    let mut input = Input::new();
    input.begin_frame(1 << Button::Right.index());
    menu.update(&mut input);
    assert_eq!(menu.selected(), 1);
    menu.tick_focus(1.0 / 6.0);
    assert!((menu.focus_of(0) - 5.0 / 6.0).abs() < 1e-5);
    assert!((menu.focus_of(1) - 1.0 / 6.0).abs() < 1e-5);
    let mid = body(&menu, &skin, &shell.frame);
    assert_ne!(at_rest, mid, "the strip did not move with the focus");
}
