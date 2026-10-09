//! Writes every texture a composed Wipeout 2048 HUD layout names as a PNG,
//! plus one PNG per sprite cut at its authored source rectangle, so a frame
//! of the running original can be matched sprite-by-sprite
//! (`docs/formats/2048-hud.md`, "Captured on Vita3K").
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_2048_hud_atlas -- \
//!     data/extracted/vita/PCSF00007/base 'Data\XML\2048_hud\Arcade_HUD.xml' /tmp/out
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use oag_hud as hud;
use oag_texture::gxt;

fn decode(blob: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let parsed = gxt::Gxt::parse(blob).ok()?;
    let texture = parsed.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    Some((
        u32::from(texture.width),
        u32::from(texture.height),
        rgba.into_iter().flatten().collect(),
    ))
}

fn crop(width: u32, height: u32, rgba: &[u8], uv: [f32; 4]) -> Option<(u32, u32, Vec<u8>)> {
    let (u, v, w, h) = (uv[0], uv[1], uv[2], uv[3]);
    // A negative width authors a horizontal flip; cut the unflipped rect.
    let (x0, cw) = if w < 0.0 { (u + w, -w) } else { (u, w) };
    let (y0, ch) = if h < 0.0 { (v + h, -h) } else { (v, h) };
    let (x0, y0, cw, ch) = (x0 as i64, y0 as i64, cw as i64, ch as i64);
    if cw <= 0
        || ch <= 0
        || x0 < 0
        || y0 < 0
        || x0 + cw > i64::from(width)
        || y0 + ch > i64::from(height)
    {
        return None;
    }
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in y0..y0 + ch {
        let row = ((y * i64::from(width) + x0) * 4) as usize;
        out.extend_from_slice(&rgba[row..row + (cw * 4) as usize]);
    }
    Some((cw as u32, ch as u32, out))
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let source = args
        .next()
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base".to_string());
    let root = args
        .next()
        .unwrap_or_else(|| oag_2048::hud::skins::played::ARCADE.to_string());
    let out = args
        .next()
        .unwrap_or_else(|| ["data", "shots", "2048-hud-atlas"].join("/"));
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;

    let mut archives = oag_2048::open(&source)?;
    let composed = {
        let mut read = |p: &str| archives.read_name(p).ok();
        hud::compose(&root, &mut read).ok_or_else(|| anyhow::anyhow!("composing {root} failed"))?
    };

    let mut textures: BTreeMap<String, (u32, u32, Vec<u8>)> = BTreeMap::new();
    for sprite in &composed.layout.sprites {
        let entry = oag_2048::hud::texture_entry(&sprite.src);
        if textures.contains_key(&entry) {
            continue;
        }
        let Ok(blob) = archives.read_name(&entry) else {
            println!("missing texture {entry}");
            continue;
        };
        let Some(decoded) = decode(&blob) else {
            println!("undecodable texture {entry}");
            continue;
        };
        let stem = entry
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(&entry)
            .to_string();
        let png = oag_texture::png::encode_rgba(decoded.0, decoded.1, &decoded.2);
        std::fs::write(out.join(format!("{stem}.png")), png)?;
        println!("{entry}: {}x{} -> {stem}.png", decoded.0, decoded.1);
        textures.insert(entry, decoded);
    }

    for sprite in &composed.layout.sprites {
        let entry = oag_2048::hud::texture_entry(&sprite.src);
        let Some((w, h, rgba)) = textures.get(&entry) else {
            continue;
        };
        match crop(*w, *h, rgba, sprite.uv) {
            Some((cw, ch, pixels)) => {
                let png = oag_texture::png::encode_rgba(cw, ch, &pixels);
                std::fs::write(out.join(format!("sprite-{}.png", sprite.name)), png)?;
            }
            None => println!("{}: uv {:?} outside {w}x{h}", sprite.name, sprite.uv),
        }
    }
    Ok(())
}
