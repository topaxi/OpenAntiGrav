//! Scratch probe: does `PS_BUTTONS.fnt` decode on the HD/Fury disc, and do
//! the codepoints `FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON`/`DifficultyButtonIcon`/
//! `ControlTextNavigationButton` actually resolve to (`ε`/`γ`/`δ`,
//! `Δ`/`Γ`/`Β`/`Α`) carry real glyph art rather than an empty or placeholder
//! box?
//!
//! Goes through `oag_assets::Archives::open`/`read_font`, the same path
//! `oag_game::boot::load_font` uses - **not** a raw `psarc::Archive::read_path`
//! by the first of `archive.paths()`'s own matches, which this file's first
//! version did and which read the *wrong entry*: `DATA02.PSARC` carries both
//! `/data/fe/fonts/ps_buttons.gtf` and `/data/fe/fonts/ps_buttons.fnt`, in
//! that order, and `.first()` over an unfiltered `contains("ps_buttons")`
//! list took the `.gtf` texture, not the `.fnt` metrics file - a plain
//! selection bug, not an archive-precedence one. Its 524,416 bytes starting
//! `01 05 00 00...` are a `.gtf` header (`01 05` reads as a `.gtf` version
//! word once identified as one - see `oag_texture::gtf`), not a corrupted or
//! wrongly-resolved `.fnt`. `frontend_bnk_probe.rs`'s own `.find(|p|
//! p.ends_with(".bnk"))` avoids exactly this trap by filtering on the
//! extension that matters, which the first version of this file skipped.
//! Going through `Archives::read_font` instead sidesteps the whole class of
//! mistake: it takes a role-resolved filename, not a raw substring match, so
//! there is no path list to pick the wrong entry from. `oag-hd`/`oag-title`
//! are `[dev-dependencies]` only (`Cargo.toml`'s own comment), so this does
//! not put `oag-tools` on the wrong side of `just check-deps`.
//!
//! `cargo run -p oag-tools --example hd_buttons_font_probe -- <hd iso> [out.png]`

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .expect("usage: hd_buttons_font_probe <hd iso> [out.png]");
    let out_path = args.next();

    let mut archives = oag_assets::Archives::open(&image, oag_hd::TITLE)
        .unwrap_or_else(|e| panic!("open {image}: {e}"));

    let name = r"Data\FE\Fonts\PS_BUTTONS.fnt";
    println!("== reading {name} through Archives::read_font ==");
    let font = archives
        .read_font(name)
        .unwrap_or_else(|e| panic!("{name} unreadable: {e}"));
    println!(
        "{}x{} atlas, {} glyphs, line height {}",
        font.width,
        font.height,
        font.glyphs.len(),
        font.line_height
    );

    println!("\n== codepoints this build actually asks the Buttons face for ==");
    for (label, ch) in [
        ("FE_CONFIRM_BUTTON", 'ε'),
        ("FE_BACK_BUTTON", 'γ'),
        ("DifficultyButtonIcon", 'δ'),
        ("ControlTextNavigationButton[0]", 'Δ'),
        ("ControlTextNavigationButton[1]", 'Γ'),
        ("ControlTextNavigationButton[2]", 'Β'),
        ("ControlTextNavigationButton[3]", 'Α'),
    ] {
        let cp = ch as u32;
        let glyph = font.glyphs.iter().find(|g| u32::from(g.codepoint) == cp);
        match glyph {
            Some(g) => println!(
                "{label} {ch:?} (U+{cp:04X}): {}x{} box at ({}, {}), advance {}",
                g.width, g.height, g.u0, g.v0, g.advance
            ),
            None => println!("{label} {ch:?} (U+{cp:04X}): NOT in this font's glyph table"),
        }
    }

    if let Some(out) = out_path {
        // The palette's own RGB is white-on-white for this face (every menu
        // face measured so far is) - alpha, not colour, carries the glyph
        // shape, so this renders `alpha_at` as luma over solid black rather
        // than dumping `to_rgba` (which is legible on a checkerboard viewer,
        // not this tool's plain `Read`).
        let (w, h) = (usize::from(font.width), usize::from(font.height));
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let a = font.alpha_at(x, y);
                let at = (y * w + x) * 4;
                rgba[at] = a;
                rgba[at + 1] = a;
                rgba[at + 2] = a;
                rgba[at + 3] = 255;
            }
        }
        let png = oag_texture::png::encode_rgba(font.width.into(), font.height.into(), &rgba);
        std::fs::write(&out, &png).expect("write atlas png");
        println!("\nwrote whole atlas (alpha as luma over black) to {out}");
    }
}
