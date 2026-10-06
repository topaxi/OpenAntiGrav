//! Scratch probe: renders the button-prompt codepoints of one face to a
//! contact sheet (alpha as luma over black, 6x, one cell per codepoint) so
//! which physical control each one draws can be read off the picture.
//!
//! `cargo run -p oag-tools --example prompt_glyph_probe -- <pulse|hd> <image> <font name> <out.png> <codepoints...>`
use oag_texture::fnt::Font;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [title, image, name, out, chars @ ..] = args.as_slice() else {
        panic!("usage: prompt_glyph_probe <pulse|hd> <image> <font> <out.png> <chars>");
    };
    let font: Font = match title.as_str() {
        "pulse" => oag_assets::Archives::open(image, oag_pulse::TITLE)
            .expect("open")
            .read_font(name),
        _ => oag_assets::Archives::open(image, oag_hd::TITLE)
            .expect("open")
            .read_font(name),
    }
    .expect("font");
    let chars: Vec<char> = chars.iter().flat_map(|s| s.chars()).collect();
    let zoom = 6usize;
    let cell = 40usize;
    let (w, h) = (cell * zoom * chars.len(), cell * zoom);
    let mut rgba = vec![0u8; w * h * 4];
    for px in rgba.chunks_exact_mut(4) {
        px[3] = 255;
    }
    for (i, ch) in chars.iter().enumerate() {
        let Some(g) = font.glyphs.iter().find(|g| u32::from(g.codepoint) == *ch as u32) else {
            println!("{ch:?} U+{:04X}: absent", *ch as u32);
            continue;
        };
        println!(
            "{ch:?} U+{:04X}: {}x{} advance {} line_height {}",
            *ch as u32, g.width, g.height, g.advance, font.line_height
        );
        for y in 0..usize::from(g.height).min(cell) {
            for x in 0..usize::from(g.width).min(cell) {
                let a = font.alpha_at(usize::from(g.u0) + x, usize::from(g.v0) + y);
                for dy in 0..zoom {
                    for dx in 0..zoom {
                        let at = ((y * zoom + dy) * w + i * cell * zoom + x * zoom + dx) * 4;
                        rgba[at..at + 3].fill(a);
                    }
                }
            }
        }
    }
    let png = oag_texture::png::encode_rgba(w as u32, h as u32, &rgba);
    std::fs::write(out, png).expect("write");
}
