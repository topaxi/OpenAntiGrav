//! [`load`]: everything a race needs, read out of one disc image, and the report
//! of what came back.

use super::*;
#[cfg(feature = "view")]
mod audio;
#[cfg(feature = "view")]
mod cameras;
#[cfg(feature = "view")]
pub mod campaign;
#[cfg(feature = "view")]
mod caustic;
mod closing_report;
mod code_laws;
#[cfg(feature = "view")]
mod countdown;
#[cfg(feature = "view")]
mod engine_light;
#[cfg(feature = "view")]
mod environment;
mod family;
#[cfg(feature = "view")]
mod gantry_visibility;
#[cfg(feature = "view")]
mod geometry;
mod global;
#[cfg(feature = "view")]
mod glow_mask;
#[cfg(feature = "view")]
mod intro;
#[cfg(feature = "view")]
mod lens_flare;
#[cfg(feature = "view")]
mod lod_bias;
#[cfg(feature = "view")]
pub(crate) mod magstrip_wake;
#[cfg(feature = "view")]
mod mod_assets;
mod muzzle;
mod original_laws;
#[cfg(feature = "view")]
mod pads;
mod pose;
#[cfg(feature = "view")]
mod pulse_ps2;
mod pulse_psp;
#[cfg(feature = "view")]
pub(super) mod ripple;
pub(crate) mod roster;
mod seams;
pub(crate) mod sim;
pub use sim::{SimLoad, load_sim};
#[cfg(feature = "view")]
mod skin;
mod start_boost_report;
mod surfaces;
mod track_art;
mod track_inputs;
mod track_stats;
#[cfg(feature = "view")]
mod trail_hits;
#[cfg(feature = "view")]
mod view;
pub use track_stats::authored_length;
#[cfg(feature = "view")]
pub use view::load_view;
mod variant;
#[cfg(feature = "view")]
mod water;
#[cfg(feature = "view")]
mod weapon_models;
mod zone_cameras;
#[cfg(feature = "view")]
mod zone_livery;
#[cfg(feature = "view")]
pub use campaign::{load_event, load_event_sim};
#[cfg(feature = "view")]
pub use environment::{authored_light, hd_bloom_params};
use oag_source::pair::craft_of;

/// A disc circuit as a track in this project's own format, opened the way
/// [`load`] opens its source. **Local use only**: the result is disc content.
///
/// # Errors
///
/// Whatever [`load`] would refuse the circuit for, and a circuit with no
/// `Start Position` node.
pub fn export_track(options: &Options) -> Result<oag_trackfile::Track> {
    let (packs, pure_packs, _) = oag_source::dlc::packs_from_defaults(
        &options.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    let pair = oag_source::pair::SourcePair::open(&options.source, None, packs, pure_packs)?;
    let (opened, _, title, _) = pair.into_parts();
    let mut archives = opened.archives.memoising_reads();
    track_art::export(options, &mut archives, title)
}

/// The host textures a track file names, or every texture of one circuit, written as PNG
/// files under `out` from the disc `options` opens (`foreign::art::host_export`).
/// **Local use only**: the result is disc content.
///
/// `what` is a `.track.toml` (the textures its art borrows, and its sky circuit's) or a
/// circuit's `.vex` entry (all of that circuit's textures).
///
/// # Errors
///
/// Whatever [`load`] would refuse the source for, a track file that will not read, a
/// circuit that does not read on this disc, and a file that will not write.
#[cfg(feature = "view")]
pub fn export_host_textures(
    options: &Options,
    what: &str,
    out: &std::path::Path,
) -> Result<crate::foreign::art::host_export::Exported> {
    use crate::foreign::art::host_export::{Exported, export_entry};
    use std::collections::{BTreeMap, BTreeSet};

    let (mut archives, slug) = host_archives(options)?;
    let mut all = Exported::default();
    let mut merge = |done: Exported| {
        all.written.extend(done.written);
        all.skipped.extend(done.skipped);
    };
    if what.ends_with(".track.toml") {
        let loaded = oag_trackfile::read(std::path::Path::new(what))?;
        let Some(art) = loaded.track.art.as_ref() else {
            anyhow::bail!("{what} has no art section, so it borrows no texture");
        };
        let mut wanted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for source in &art.textures {
            if let oag_trackfile::art::TextureSource::Host { entry, name } = source {
                wanted
                    .entry(entry.clone())
                    .or_default()
                    .insert(name.clone());
            }
        }
        for (entry, names) in &wanted {
            merge(export_entry(&mut archives, &slug, entry, Some(names), out)?);
        }
        if let Some(sky) = art.sky.as_ref() {
            merge(export_entry(&mut archives, &slug, sky, None, out)?);
        }
    } else {
        merge(export_entry(&mut archives, &slug, what, None, out)?);
    }
    Ok(all)
}

/// The textures of the circuit `entry` a host reference can name, in the spelling it names
/// them: its `Texture` nodes' paths, or on HD, 2048 and Omega the texture files its materials
/// name. Nothing is written, and no picture is decoded for the list beyond what the circuit's
/// own build decodes.
///
/// # Errors
///
/// As [`export_host_textures`].
#[cfg(feature = "view")]
pub fn list_host_textures(options: &Options, entry: &str) -> Result<Vec<String>> {
    let (mut archives, _) = host_archives(options)?;
    crate::foreign::art::host_export::list_entry(&mut archives, entry)
}

/// [`export_host_textures`] for the circuit `entry` and only the textures `names` spells.
///
/// # Errors
///
/// As [`export_host_textures`].
#[cfg(feature = "view")]
pub fn export_named_host_textures(
    options: &Options,
    entry: &str,
    names: &std::collections::BTreeSet<String>,
    out: &std::path::Path,
) -> Result<crate::foreign::art::host_export::Exported> {
    let (mut archives, slug) = host_archives(options)?;
    crate::foreign::art::host_export::export_entry(&mut archives, &slug, entry, Some(names), out)
}

/// The archives of the source `options` opens, memoising reads, and its title's slug.
#[cfg(feature = "view")]
fn host_archives(options: &Options) -> Result<(oag_assets::Archives, String)> {
    let (packs, pure_packs, _) = oag_source::dlc::packs_from_defaults(
        &options.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    let pair = oag_source::pair::SourcePair::open(&options.source, None, packs, pure_packs)?;
    let (opened, _, title, _) = pair.into_parts();
    Ok((
        opened.archives.memoising_reads(),
        oag_source::layers::slug(title),
    ))
}

/// Loads a track, a ship and its handling out of a disc image: the sim phase
/// ([`load_sim`]), then, in a build that draws, the view phase (`load_view`) off what it
/// read.
///
/// # Errors
///
/// Propagates a missing archive, a missing entry, a `.vex` that does not decode,
/// a `WO Track` payload the spline parser cannot account for, and a
/// `handlingstats.xml` with an attribute missing.
pub fn load(options: &Options) -> Result<Loaded> {
    let sim = load_sim(options)?;
    #[cfg(feature = "view")]
    {
        let mods_differ = sim.mods_differ.clone();
        load_view(sim, options).map_err(|e| sim::named(e, mods_differ.as_deref()))
    }

    #[cfg(not(feature = "view"))]
    {
        without_view(sim)
    }
}

/// What [`load`] returns in a build that draws nothing: the sim phase's half, with the
/// stages a load reports and the report's last lines.
#[cfg(not(feature = "view"))]
fn without_view(sim: SimLoad) -> Result<Loaded> {
    let SimLoad {
        setup,
        modules,
        title,
        circuit,
        mut report,
        mut archives,
        mut craft,
        ..
    } = sim;
    stages::reach(stages::Stage::TrackBuilt)?;
    stages::reach(stages::Stage::WorldBuilt)?;
    stages::reach(stages::Stage::CraftsBuilt)?;
    closing_report::finish(&mut report, &mut archives, craft.as_mut());
    Ok(Loaded {
        title,
        circuit,
        platform: archives.layout.platform,
        setup,
        view: ViewSetup,
        modules,
        report,
    })
}
