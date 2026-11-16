//! PPSSPP's own `.zim` files, the only real ones there are to read.
//!
//! `#[ignore]`d and never run in CI: needs a PPSSPP install's `assets/` (`/usr/share/ppsspp/assets`,
//! or `OAG_PPSSPP_ASSETS`). A texture pack holds no `.zim` that the 2026 pack or the 2020 folder
//! ships; these three (zstd-compressed, RGBA8888 and RGBA4444) are the oracle.
//!
//! `OAG_ZIM_DUMP=<dir>` writes each decoded picture as a PNG to look at.

use std::path::PathBuf;

use oag_texture::zim;

fn assets() -> Option<PathBuf> {
    let dir = std::env::var_os("OAG_PPSSPP_ASSETS")
        .map_or_else(|| PathBuf::from("/usr/share/ppsspp/assets"), PathBuf::from);
    if dir.join("font_atlas.zim").is_file() {
        Some(dir)
    } else {
        eprintln!("skip: no PPSSPP assets at {}", dir.display());
        None
    }
}

#[test]
#[ignore = "needs a PPSSPP install's assets/ (OAG_PPSSPP_ASSETS)"]
fn ppsspps_own_zim_files_decode_to_their_declared_size() {
    let Some(dir) = assets() else { return };
    // (file, width, height, flags): 0x1000 zstd, 0x40 dithered, low nibble the pixel format.
    for (name, width, height, flags) in [
        ("asciifont_atlas.zim", 2048, 256, 0x1040u32),
        ("font_atlas.zim", 2048, 2048, 0x1040),
        ("ppge_atlas.zim", 2048, 2048, 0x1041),
    ] {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            flags,
            "{name}: the flags this test was written against"
        );
        let image = zim::decode(&bytes).unwrap_or_else(|why| panic!("{name}: {why}"));
        assert_eq!((image.width, image.height), (width, height), "{name}");
        assert_eq!(image.rgba.len(), (width * height * 4) as usize, "{name}");
        let opaque = image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] == 255)
            .count();
        let clear = image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] == 0)
            .count();
        eprintln!(
            "{name}: {opaque} opaque, {clear} clear of {}",
            width * height
        );
        assert!(
            opaque > 0 && clear > 0,
            "{name}: an atlas of glyphs has ink and ground"
        );
        if let Some(out) = std::env::var_os("OAG_ZIM_DUMP") {
            let png = oag_texture::png::encode_rgba(width, height, &image.rgba);
            std::fs::write(PathBuf::from(out).join(format!("{name}.png")), png).unwrap();
        }
    }
}
