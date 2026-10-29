//! A foreign track: a Unity circuit read by `oag-ballisticng` and mapped into
//! this project's own track model, `oag_trackfile::Track` (ADR-0062, decision 4;
//! ADR-0063), the plain data the race consumes.
//!
//! **Nothing here is measured off the original game.** [`map`] turns what the
//! content layer *read* into this engine's shapes, and every constant that turns
//! a reading into a number is in [`choice`], each labelled *chosen, not
//! measured*. What the content does not author, or this mapping has no
//! counterpart for, is named in [`Track::notes`] - drawn as nothing, not
//! stood in for.
//!
//! A race on a foreign track runs on a host title's physics, rules and ship. The
//! output is `oag_vex::track` spline data, collision nodes, pad volumes and a
//! start slot; the simulation reads them like a disc circuit's and learns nothing
//! of where they came from.

#[cfg(feature = "view")]
pub mod art;
#[cfg(feature = "ballisticng")]
pub mod choice;
#[cfg(feature = "ballisticng")]
mod frame;
#[cfg(feature = "ballisticng")]
mod map;
pub mod survey;

use std::path::Path;

use oag_trackfile::Track;

/// Reads the `.wtrk` at `path` and maps it.
///
/// # Errors
///
/// A file the wrapper or the Unity reader refuses, a track class the content
/// layer refuses, or a track with no lap loop. The message names which.
#[cfg(feature = "ballisticng")]
pub fn read(path: &Path) -> anyhow::Result<Track> {
    let file =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
    let wtrk = oag_ballisticng::wtrk::Wtrk::parse(&file)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let bundle = oag_unity::bundle::Bundle::parse(wtrk.bundle(&file))
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let assets = oag_unity::assets::Assets::open(&bundle)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let track = oag_ballisticng::track::Track::read(&assets)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    map::map(&wtrk, &track)
}

/// Whether `path` is one of this project's own track manifests (`*.track.toml`)
/// rather than a Workshop `.wtrk`.
#[must_use]
pub fn is_track_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "toml")
}

/// Reads the track at `path` whichever format it is in, with the lines the
/// loader report starts from: what the file said about itself, what it
/// carries nothing for, and what the reader left unread.
///
/// # Errors
///
/// As [`read`] for a `.wtrk`, and as [`oag_trackfile::read`] for a manifest.
pub fn read_any(path: &Path) -> anyhow::Result<(Track, Vec<String>)> {
    if !is_track_file(path) {
        #[cfg(feature = "ballisticng")]
        {
            let track = read(path)?;
            let report = track.notes.clone();
            return Ok((track, report));
        }
        #[cfg(not(feature = "ballisticng"))]
        return Err(no_workshop_support(path));
    }
    let loaded =
        oag_trackfile::read(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let mut report = vec![format!(
        "track file {:?} ({}), schema v{}",
        loaded.track.title,
        loaded.track.id,
        oag_trackfile::FORMAT_MAJOR
    )];
    report.extend(loaded.track.notes.iter().cloned());
    report.push(format!(
        "not authored by schema v{}, so drawn as nothing and not borrowed from the host: {}",
        oag_trackfile::FORMAT_MAJOR,
        oag_trackfile::ABSENT.join(", ")
    ));
    report.extend(loaded.unread.iter().map(|u| format!("not read: {u}")));
    Ok((loaded.track, report))
}

/// The title a track names as its host (`Track::host`, ADR-0076 decision 13),
/// or `None` for a track that names none (a Workshop track mapped in memory).
///
/// # Errors
///
/// A host spelled as no title this build knows, named with the ones it does.
pub fn host_title(track: &Track) -> anyhow::Result<Option<&'static oag_title::Title>> {
    let Some(host) = track.host.as_deref() else {
        return Ok(None);
    };
    oag_source::title::by_slug(host).map(Some).ok_or_else(|| {
        anyhow::anyhow!(
            "track.host {host:?} is no title this build knows (it knows {})",
            oag_source::title::ALL
                .iter()
                .map(|t| oag_source::layers::slug(t))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Refuses a track raced with a track source of another title than its host.
/// What it borrows by name (host textures, the sky circuit) resolves only from
/// its host's disc, so on another source it would draw black; another title
/// races it only as a Race Remix, the host as the track source and the other
/// title's craft.
///
/// # Errors
///
/// As [`host_title`], and the mismatch, naming both titles and the remix
/// that races it.
pub fn check_host(track: &Track, source: &oag_title::Title) -> anyhow::Result<()> {
    let Some(host) = host_title(track)? else {
        return Ok(());
    };
    if oag_source::layers::slug(host) == oag_source::layers::slug(source) {
        return Ok(());
    }
    anyhow::bail!(
        "{:?} runs on {} (track.host = {:?}); this race's track source is {}. Race it on \
         {}, or on {}'s craft as a Race Remix with {} as the track source (RACE REMIX, or \
         --source <{} disc> --craft-source <{} disc>)",
        track.title,
        host.name,
        track.host.as_deref().unwrap_or_default(),
        source.name,
        host.name,
        source.name,
        host.name,
        host.name,
        source.name,
    )
}

/// The catalogue entry for the track at `path` (a `.wtrk` or a track-file
/// manifest): a synthetic [`catalogue::Track`](crate::catalogue::Track) carrying
/// the file as data, so nothing downstream branches on a title or a name to tell
/// it from a disc circuit. Its `id` is `foreign:` and the track's own id (the
/// Workshop id for a `.wtrk`), which is also what keys its records and ghosts.
///
/// # Errors
///
/// A file the `.wtrk` wrapper or the manifest reader refuses.
pub fn entry(path: &Path) -> anyhow::Result<crate::catalogue::Track> {
    let id = if is_track_file(path) {
        oag_trackfile::read(path)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
            .track
            .id
    } else {
        workshop_id(path)?
    };
    Ok(crate::catalogue::Track {
        id: format!("foreign:{id}"),
        location: path.display().to_string(),
        reversed: false,
        available_in_zone: false,
        unlock_grid: None,
        sound_register: None,
        foreign: Some(path.to_path_buf()),
    })
}

/// The Workshop id a `.wtrk` carries, which keys its records and ghosts.
#[cfg(feature = "ballisticng")]
fn workshop_id(path: &Path) -> anyhow::Result<String> {
    let file =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
    Ok(oag_ballisticng::wtrk::Wtrk::parse(&file)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
        .workshop_id
        .to_string())
}

#[cfg(not(feature = "ballisticng"))]
fn workshop_id(path: &Path) -> anyhow::Result<String> {
    Err(no_workshop_support(path))
}

/// The refusal for a `.wtrk` in a build without the `ballisticng` feature.
#[cfg(not(feature = "ballisticng"))]
fn no_workshop_support(path: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "{}: Workshop tracks (.wtrk) need a build with the `ballisticng` feature, which no \
         shipped build enables",
        path.display()
    )
}
