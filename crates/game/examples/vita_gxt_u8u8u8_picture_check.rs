//! Scratch probe: `U8U8U8` (`0x98`, 13 files, the last undecoded uncompressed
//! format byte) closes its length arithmetic at 3 bytes a texel, tightly
//! packed, no floor needed (`vita_gxt_u8u8u8_extent.rs`: `plain3` matches
//! `declared` exactly on all 13). What is still open is channel order and
//! whether it is twiddled like `U8U8U8U8` or raster like `.gtf`'s own linear
//! textures - both untestable from the length check alone, so this decodes
//! every combination and looks at the picture, the same method that settled
//! `U8U8U8U8`'s and `UBC2`'s equivalent questions.
//!
//! All 13 files are `scepresents/scee_presents_<language>.gxt`, 512x64, a
//! first-party splash - legible art with a recognisable logo, so a wrong
//! channel order or tiling is expected to be obviously wrong rather than
//! merely off.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_gxt_u8u8u8_picture_check
//! ```

const ENTRY: &str = "data/FE/NewImages/scepresents/scee_presents_ENG.gxt";
const BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

const ALL_LANGUAGES: [&str; 13] = [
    "ENG", "danish", "dutch", "finnish", "fr", "ger", "it", "norway", "polish", "port", "russian",
    "sp", "swede",
];

fn twiddle(x: u32, y: u32, across: u32, down: u32) -> u32 {
    let (mut w, mut h) = (across, down);
    let (mut mask_x, mut mask_y, mut shift) = (0u32, 0u32, 0u32);
    while w > 1 || h > 1 {
        if w > 1 && h > 1 {
            mask_y |= 1 << shift;
            mask_x |= 1 << (shift + 1);
            shift += 2;
        } else if w > 1 {
            mask_x |= 1 << shift;
            shift += 1;
        } else {
            mask_y |= 1 << shift;
            shift += 1;
        }
        w >>= 1;
        h >>= 1;
    }
    scatter(x, mask_x) | scatter(y, mask_y)
}

fn scatter(value: u32, mask: u32) -> u32 {
    let (mut out, mut remaining, mut source) = (0u32, mask, value);
    while remaining != 0 {
        let lowest = remaining.isolate_lowest_one();
        if source & 1 != 0 {
            out |= lowest;
        }
        source >>= 1;
        remaining &= !lowest;
    }
    out
}

fn decode(texels: &[u8], width: u32, height: u32, order: [usize; 3], twiddled: bool) -> Vec<u8> {
    let mut out = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height {
        for x in 0..width {
            let index = if twiddled {
                twiddle(x, y, width, height) as usize
            } else {
                (y * width + x) as usize
            };
            let at = index * 3;
            let Some(texel) = texels.get(at..at + 3) else {
                continue;
            };
            let out_at = (y as usize * width as usize + x as usize) * 4;
            out[out_at] = texel[order[0]];
            out[out_at + 1] = texel[order[1]];
            out[out_at + 2] = texel[order[2]];
            out[out_at + 3] = 255;
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(BASE)?;

    for language in ALL_LANGUAGES {
        let entry = format!("data/FE/NewImages/scepresents/scee_presents_{language}.gxt");
        let blob = archive.read_path(&entry)?;
        let width = u16::from_le_bytes([blob[32 + 0x18], blob[32 + 0x19]]) as u32;
        let height = u16::from_le_bytes([blob[32 + 0x1a], blob[32 + 0x1b]]) as u32;
        let offset = u32::from_le_bytes(blob[32..36].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(blob[36..40].try_into().unwrap()) as usize;
        let texels = &blob[offset..offset + length];
        let mut max_channel_spread = 0u8;
        for texel in texels.as_chunks::<3>().0 {
            let (mn, mx) = (
                texel.iter().min().copied().unwrap_or(0),
                texel.iter().max().copied().unwrap_or(0),
            );
            max_channel_spread = max_channel_spread.max(mx - mn);
        }
        println!("{language}: {width}x{height}, max channel spread {max_channel_spread}");
    }

    let blob = archive.read_path(ENTRY)?;
    let width = u16::from_le_bytes([blob[32 + 0x18], blob[32 + 0x19]]) as u32;
    let height = u16::from_le_bytes([blob[32 + 0x1a], blob[32 + 0x1b]]) as u32;
    let offset = u32::from_le_bytes(blob[32..36].try_into().unwrap()) as usize;
    let length = u32::from_le_bytes(blob[36..40].try_into().unwrap()) as usize;
    println!("{ENTRY}: {width}x{height}, texels at {offset}+{length}");
    let texels = &blob[offset..offset + length];

    // How much does channel order even matter here? If every texel is
    // (near-)grayscale, RGB and BGR decode to the same picture and this
    // sample cannot discriminate order at all - worth knowing before reading
    // a wrong-looking match as confirmation.
    let mut max_channel_spread = 0u8;
    for texel in texels.as_chunks::<3>().0 {
        let (mn, mx) = (
            texel.iter().min().copied().unwrap_or(0),
            texel.iter().max().copied().unwrap_or(0),
        );
        max_channel_spread = max_channel_spread.max(mx - mn);
    }
    println!("max |channel_a - channel_b| over any texel: {max_channel_spread}");

    std::fs::create_dir_all("data/shots")?;
    let orders: [(&str, [usize; 3]); 3] =
        [("rgb", [0, 1, 2]), ("bgr", [2, 1, 0]), ("grb", [1, 0, 2])];
    for (name, order) in orders {
        for twiddled in [false, true] {
            let rgba = decode(texels, width, height, order, twiddled);
            let label = if twiddled { "twiddled" } else { "raster" };
            std::fs::write(
                format!("data/shots/2048_u8u8u8_scee_{name}_{label}.png"),
                oag_texture::png::encode_rgba(width, height, &rgba),
            )?;
            println!("wrote data/shots/2048_u8u8u8_scee_{name}_{label}.png");
        }
    }
    Ok(())
}
