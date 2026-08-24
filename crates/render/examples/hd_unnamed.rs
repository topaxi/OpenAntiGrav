//! Scratch probe: how much of each PSARC this project can still not name.
//!
//! A PSARC's table of contents holds a 16-byte digest of each path and no
//! path, so an entry whose name has never been guessed is invisible to every
//! by-name reader in the workspace - it is not missing from the archive, it is
//! missing from the vocabulary. This counts that gap per archive.

use oag_assets::Container;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    for n in 0..=6 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{n:02}.PSARC");
        let Ok(container) = Container::open(&spec) else {
            continue;
        };
        let total = container.entry_count();
        let known = oag_hd::entry_names()
            .iter()
            .filter(|name| container.contains(name))
            .count();
        println!(
            "DATA{n:02}: {total:>6} entries, {known:>6} named, {:>6} unnamed ({:.1}%)",
            total - known,
            (total - known) as f64 / total as f64 * 100.0
        );
    }
    Ok(())
}
