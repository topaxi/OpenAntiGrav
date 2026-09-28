//! Scratch probe: dump line_height and the 'S' glyph ink box for
//! `pulse_text.fnt` (Default) and `Pulse_20.fnt` (Menu), to settle whether
//! the campaign detail panel's undersized text is a face-scale bug or a
//! font-substitution bug. See `docs/ui/campaign-screens.md`.
use anyhow::{Context, Result};
use oag_assets::Archive;

fn dump(archive: &mut Archive, path: &str, label: &str) -> Result<()> {
    let blob = archive.read_name(path).with_context(|| path.to_string())?;
    let metrics = oag_texture::fnt::Metrics::parse(&blob)?;
    let s = metrics
        .glyphs
        .iter()
        .find(|g| g.codepoint == u16::from(b'S'))
        .context("no 'S' glyph")?;
    println!(
        "{label} ({path}): line_height={} 'S' width={} height={}",
        metrics.line_height, s.width, s.height
    );
    Ok(())
}

fn main() -> Result<()> {
    let spec = std::env::args()
        .nth(1)
        .expect("usage: campaign_font_probe <image>:PSP_GAME/USRDIR/Data.wad");
    let mut archive = Archive::open(&spec)?;
    dump(&mut archive, r"Data\FE\Fonts\pulse_text.fnt", "Default")?;
    dump(&mut archive, r"Data\FE\Fonts\Pulse_14.fnt", "Small/Title")?;
    dump(&mut archive, r"Data\FE\Fonts\Pulse_20.fnt", "Menu")?;
    Ok(())
}
