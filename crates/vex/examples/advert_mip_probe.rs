//! Scratch probe: what `mip_count` Talon's Junction's advert-board textures
//! declare. Found `hub_banner_GLOW.tga` at `mip_count = 5`, which is what
//! `ModelTexture::mip_count`'s doc measured a synthesised-chain cap against.
//! Written for the open question of what blurs Talon's Junction's advert
//! boards; capping the chain at the authored depth turned out not to be it.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = "data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad";
    let name = r"Data\Environments\16_Track\track.vex";
    let data = oag_assets::Container::open(spec)?.read_entry(name)?;
    let textures = oag_vex::vex::textures(&data)?;
    for t in textures.into_iter().flatten() {
        let label = t
            .asset_path
            .as_deref()
            .or(t.name.as_deref())
            .unwrap_or("<unnamed>");
        if label.to_ascii_lowercase().contains("banner")
            || label.to_ascii_lowercase().contains("advert")
        {
            println!(
                "{label}: {}x{} bpp={} mip_count={}",
                t.width, t.height, t.bits_per_pixel, t.mip_count
            );
        }
    }
    Ok(())
}
