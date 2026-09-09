//! Reading a race's handling parameters and collision geometry off a disc, for
//! a replay.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change beyond the
//! speed class arriving as a **name** - see [`checked_class`].

use anyhow::{Context, Result};
use log::{info, warn};
use oag_formats::collision;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::{CollisionWorld, Handling};
use oag_pulse as pulse;
use oag_tables::handling;

/// Spell-checks `--class` against every measured ladder.
///
/// Not the check that decides a race: which rungs *this* disc authors is settled
/// by the files themselves, in [`load`]. This exists so a typo is a message
/// about the command line rather than a failure a long way in. See
/// `oag_title::SpeedClasses::MEASURED`.
pub(crate) fn checked_class(raw: &str) -> Result<&str> {
    let class = raw.trim();
    anyhow::ensure!(
        oag_title::SpeedClasses::is_measured_name(class),
        "{raw:?} is not a speed class"
    );
    Ok(class)
}

/// Reads the handling parameters and the track's collision geometry off a disc.
///
/// The same two reads `oag_game::race::load` does, without the spline, the models
/// or the camera: a comparison run is seeded from the recording rather than from
/// a grid slot, so it needs nothing that decides where a ship starts.
///
/// **Which archives the source has, not which archives a PSP disc has.** Both
/// entry names below are spelled the same on both releases, so the layout is the
/// whole of the difference - [`oag_assets::Archives`] finds `Data.wad` or `WADS2.WAD`
/// by name and searches the companion archive too. This deliberately prints
/// nothing extra: `run` output is compared byte-for-byte against earlier
/// captures, so the layout appears only in an error's context.
pub(crate) fn load(
    source: &str,
    track: &str,
    team: &str,
    class: &str,
) -> Result<(Handling, CollisionWorld)> {
    let mut archives = pulse::open(source)?;
    let where_from = archives.layout.describe();

    let track_blob = archives
        .read_name(track)
        .with_context(|| format!("reading {track} out of {where_from}"))?;
    let nodes = collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{track}: {e}"))?;
    let collision = collision_world(&nodes);
    info!(
        "{track}: {} collision node(s) -> {} collider(s)",
        nodes.len(),
        collision.colliders().len()
    );

    let stats_name = handling::entry_name(team);
    let stats_blob = archives
        .read_name(&stats_name)
        .with_context(|| format!("reading {stats_name} out of {where_from}"))?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    // The speed-pad tunables are engine-wide rather than per team, so they come
    // out of a second file. A capture that crosses a pad is compared against a
    // force law that has them; without this the replay would be missing step 15
    // entirely and the difference would be read as a fit error somewhere else.
    let global = archives
        .read_name(handling::GLOBAL_ENTRY)
        .ok()
        .and_then(|blob| handling::global_from_blob(&blob).ok())
        .flatten();
    if global.is_none() {
        warn!(
            "{}: unreadable, so this replay applies no speed-pad boost",
            handling::GLOBAL_ENTRY
        );
    }
    // Resolved by **name**, so a title whose ladder is not Pulse's reaches its
    // own `<GlobalClass>`. A rung this file does not author gets no boost
    // rather than another rung's.
    let pad_tunables = global
        .as_ref()
        .and_then(|global| global.class_named(class))
        .map(|(pads, _, _)| pads)
        .unwrap_or_default();
    // `<Special speedpad_jump>` rides along: a capture taken with the pitch axis
    // held over a pad tilts, and a replay that dropped the field would read that
    // as a force-law error.
    let special = global
        .as_ref()
        .map(|global| global.special)
        .unwrap_or_default();
    let handling = handling_for(&stats, class, pad_tunables, special).with_context(|| {
        format!(
            "{stats_name} authors no <Class name=\"{class}\"> - it carries {}",
            stats.ladder()
        )
    })?;
    info!(
        "{stats_name}: team {:?}, {class} class, mass {}, ride_height {}",
        stats.team, handling.physical.mass, handling.antigrav.ride_height
    );

    Ok((handling, collision))
}
