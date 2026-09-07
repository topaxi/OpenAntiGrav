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

#[cfg(test)]
mod tests;
