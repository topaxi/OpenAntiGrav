//! A ship picker's preview follows the selected variant: which directory the
//! hull is read from, and what draws when a variant ships none.

use anyhow::Result;
use oag_mesh::mesh::Model;

/// The directory a team's `variant` races out of: `location` joined with the
/// variant's suffix by the title's own [`oag_title::VariantJoin`] (HD's
/// `Feisar` + `_c1` is `Feisar_c1`). `location` itself for the classic hull's
/// empty suffix or a team that carries no variant table.
#[must_use]
pub fn variant_location(
    join: Option<oag_title::VariantJoin>,
    location: &str,
    variant: &str,
) -> String {
    match join {
        Some(join) if !variant.is_empty() => join.combine(location, variant),
        _ => location.to_string(),
    }
}

/// [`model`] for a variant's own entry, falling back to `fallback` (the
/// team's default hull) when the variant ships no such asset, and saying so
/// in the log rather than inventing one. Returns the entry name that loaded.
///
/// # Errors
///
/// Neither entry loads.
pub fn model_or_default(
    archives: &mut oag_assets::Archives,
    entry: &str,
    fallback: Option<&str>,
) -> Result<(Model, String)> {
    match super::model(archives, entry) {
        Ok(model) => Ok((model, entry.to_string())),
        Err(error) => {
            let Some(fallback) = fallback.filter(|fallback| *fallback != entry) else {
                return Err(error);
            };
            log::warn!(
                "{entry}: {error:#} - the variant has no hull of its own, drawing {fallback}"
            );
            super::model(archives, fallback).map(|model| (model, fallback.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::variant_location;
    use oag_title::VariantJoin;

    #[test]
    fn a_suffix_joins_the_teams_own_directory() {
        let join = Some(VariantJoin::Suffix);
        assert_eq!(
            variant_location(join, r"Data\Ships\Feisar", "_c1"),
            r"Data\Ships\Feisar_c1"
        );
        assert_eq!(
            variant_location(join, r"Data\Ships\Feisar", ""),
            r"Data\Ships\Feisar"
        );
        assert_eq!(
            variant_location(None, r"Data\Ships\Feisar", "_c1"),
            r"Data\Ships\Feisar"
        );
    }
}
