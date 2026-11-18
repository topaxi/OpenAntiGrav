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
mod roster;
mod seams;
mod sim;
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
#[cfg(feature = "view")]
mod zone_livery;
#[cfg(feature = "view")]
pub use campaign::load_event;
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
        load_view(sim, options)
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
