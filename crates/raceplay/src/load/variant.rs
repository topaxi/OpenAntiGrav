//! The player's own alternate hull file, resolved once `load` knows the
//! title - see `oag_title::race::HullVariant`.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// `options.hull_variant`, checked against `title`'s own
/// [`oag_title::RaceDefaults::hull_variants`]. `None` when nothing was
/// asked for; a mismatch - the stem is not one of the title's own, or the
/// title has no such axis at all - is reported rather than silently
/// ignored, the same shape `Options::team`'s own "unrecognised" case takes
/// elsewhere in `load.rs`.
pub(super) fn resolve(
    asked: Option<&str>,
    title: &oag_title::Title,
    report: &mut Vec<String>,
) -> Option<&'static str> {
    let asked = asked?;
    match title.race.hull_variants {
        Some(variants) => match variants.iter().find(|v| v.stem == asked) {
            Some(variant) => Some(variant.stem),
            None => {
                report.push(format!(
                    "{}: no {asked:?} hull variant; racing the baseline hull instead",
                    title.name
                ));
                None
            }
        },
        None => {
            report.push(format!(
                "{}: no hull-variant axis at all; racing the baseline hull instead",
                title.name
            ));
            None
        }
    }
}

/// The hull stem every craft flies on `track`, off the circuit's own `PI_Track` `type`:
/// [`oag_title::RaceDefaults::hull_by_track_type`] of the craft's title, looked up for the kind the
/// plugin definition declares `track` under. `None` when the title swaps no hull by type, when the
/// definition will not read, or when `track` is declared under another type.
///
/// Every slot flies it, the player's own pick included: the original's hull path is built off the
/// circuit's flag before the team's (`docs/ghidra/functions/psp-pure-usa/ship-models.md`).
pub(super) fn circuit_hull(
    archives: &mut oag_assets::Archives,
    circuit_title: &oag_title::Title,
    craft_title: &oag_title::Title,
    track: &str,
    report: &mut Vec<String>,
) -> Option<&'static str> {
    if craft_title.race.hull_by_track_type.is_empty() {
        return None;
    }
    let documents = crate::catalogue::documents(archives, &[circuit_title.plugin_definition]);
    let flat = |path: &str| path.replace('/', "\\").to_ascii_lowercase();
    let track = flat(track);
    let (kind, stem) = craft_title
        .race
        .hull_by_track_type
        .iter()
        .find(|(kind, _)| {
            documents.iter().any(|xml| {
                crate::catalogue::tracks_of_kind(xml, kind)
                    .iter()
                    .any(|entry| track.starts_with(&format!("{}\\", flat(&entry.location))))
            })
        })?;
    report.push(format!(
        "{}: a {kind} circuit - every craft flies its {stem}.vex hull",
        craft_title.name
    ));
    Some(stem)
}

#[cfg(test)]
mod tests;
