//! Scratch probe: describe `.fnt` files and write each embedded atlas as a PNG.
//!
//! `cargo run -p oag-tools --example hud_font_probe -- <file.fnt>...`
//!
//! Writes `<file>.atlas.png` beside each input. A pre-outlined font (Pulse's
//! `PulseHud.fnt`) is written as its palette colour; a plain coverage font
//! (HD's, Omega's) is written as alpha on black, because its RGB is constant
//! white and a viewer that composites on white shows nothing.

fn main() {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path).expect("read font");
        let metrics = match oag_texture::fnt::Metrics::parse(&blob) {
            Ok(m) => m,
            Err(e) => {
                println!("{path}: metrics error {e}");
                continue;
            }
        };
        println!(
            "{path}: {} bytes, line_height {}, glyphs {}, unknown {}, atlas_at {}, embedded {}",
            blob.len(),
            metrics.line_height,
            metrics.glyphs.len(),
            metrics.unknown,
            metrics.atlas_at,
            metrics.has_embedded_atlas(&blob)
        );
        for ch in ['0', '8', 'A'] {
            if let Some(g) = metrics.glyphs.iter().find(|g| g.codepoint == ch as u16) {
                println!(
                    "  '{ch}': {}x{} adv {} uv ({},{})-({},{})",
                    g.width, g.height, g.advance, g.u0, g.v0, g.u1, g.v1
                );
            }
        }
        let font = match oag_texture::fnt::Font::parse(&blob) {
            Ok(f) => f,
            Err(e) => {
                println!("  atlas error: {e}");
                continue;
            }
        };
        println!("  atlas {}x{}", font.width, font.height);
        let outlined = font.is_outlined();
        let rgba: Vec<u8> = font
            .to_rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                if outlined {
                    [p[0], p[1], p[2], 255]
                } else {
                    [p[3], p[3], p[3], 255]
                }
            })
            .collect();
        let png =
            oag_texture::png::encode_rgba(u32::from(font.width), u32::from(font.height), &rgba);
        std::fs::write(format!("{path}.atlas.png"), png).expect("write png");
    }
}
