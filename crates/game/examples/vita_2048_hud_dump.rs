//! Dumps every widget of one of Wipeout 2048's composed HUD layouts, so a
//! frame of the running original can be matched against the authored `x`/`y`
//! and source rectangles - the derivation `oag_2048::hud::ALWAYS_ON` rests on
//! (`docs/formats/2048-hud.md`, "Captured on Vita3K").
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_2048_hud_dump -- \
//!     data/extracted/vita/PCSF00007/base 'Data\XML\2048_hud\Arcade_HUD.xml'
//! ```

use oag_game::hud;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let source = args
        .next()
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base".to_string());
    let root = args
        .next()
        .unwrap_or_else(|| oag_2048::hud::skins::played::ARCADE.to_string());

    let mut archives = oag_2048::open(&source)?;
    let mut read = |p: &str| archives.read_name(p).ok();
    let composed =
        hud::compose(&root, &mut read).ok_or_else(|| anyhow::anyhow!("composing {root} failed"))?;

    println!("root {root}");
    println!("files {:?}", composed.files);
    println!("missing {:?}", composed.missing);
    println!("skipped {:?}", composed.layout.skipped);
    println!(
        "-- sprites ({}), paint order",
        composed.layout.sprites.len()
    );
    for s in &composed.layout.sprites {
        println!(
            "Image {:<32} rect=[{:>6.1} {:>6.1} {:>6.1} {:>6.1}] uv=[{:>4.0} {:>4.0} {:>4.0} {:>4.0}] color={:?} rot={} src={}",
            s.name,
            s.rect[0],
            s.rect[1],
            s.rect[2],
            s.rect[3],
            s.uv[0],
            s.uv[1],
            s.uv[2],
            s.uv[3],
            s.color,
            s.rotation,
            s.src
        );
    }
    println!("-- fills ({})", composed.layout.fills.len());
    for f in &composed.layout.fills {
        println!("Fill  {:<32} rect={:?} color={:?}", f.name, f.rect, f.color);
    }
    println!("-- labels ({})", composed.layout.labels.len());
    for l in &composed.layout.labels {
        println!(
            "Text  {:<32} x={:>6.1} y={:>6.1} font={:?} scale={} align={:?} valign={:?} id={:?} string={:?}",
            l.name, l.x, l.y, l.font, l.scale, l.align, l.vertalign, l.idstring, l.string
        );
    }
    println!("-- models ({})", composed.layout.models.len());
    for m in &composed.layout.models {
        println!("Model {m:?}");
    }
    Ok(())
}
