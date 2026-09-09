//! Decode a `.mip` texture to a binary PPM on stdout, for eyeballing an
//! extracted texture without a GPU: `cargo run -p oag-formats --example
//! mip-to-ppm -- flare.mip > flare.ppm`.

use std::io::Write;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: mip-to-ppm <file.mip>");
    let data = std::fs::read(&path).expect("reading the texture");
    let texture = oag_texture::texture::Texture::parse(&data).expect("parsing the texture");
    let rgba = texture.to_rgba();
    let (w, h) = (texture.width as usize, texture.height as usize);
    let mut out = std::io::stdout().lock();
    write!(out, "P6\n{w} {h}\n255\n").unwrap();
    for px in rgba.chunks(4).take(w * h) {
        out.write_all(&px[..3]).unwrap();
    }
}
