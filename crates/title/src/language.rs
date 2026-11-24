//! Which language plugins a release offers, as its executable lists them.
//!
//! A disc carries a **superset** of what a release offers: both Wipeout Pure
//! pressings ship the same nine plugins, yet the USA picker offers three
//! languages and the EU one five, and the Pulse PSP EU disc moves English to a
//! plugin the USA disc does not have. What decides it is a plugin manifest in
//! the executable - a null-terminated table of pointers to `Data\Plugins\PI0NN`
//! strings that the boot walks in order - and that is per release, so it is
//! data keyed by the disc's serial, in the same shape
//! `oag_pure::frontend::localised_movie_region` already keys on.
//!
//! The newer titles have **no manifest and no picker**: their executables pick
//! one language plugin from the console's system language and the release's
//! region, so what a release offers is the set that choice can reach. That is
//! the same shape of data - a list per release, keyed by the package's
//! `TITLE_ID` - and is recorded in [`LanguageManifest::evidence`] the same way.

/// One release's language plugins, in the order its executable's manifest lists
/// them.
///
/// [`Self::plugins`] holds the **offered** languages only: the manifest also names
/// plugins that are not a language (`PI001` the skin, `PI004` billboards), and those
/// are left out. A plugin that is a string overlay on a language, not a language
/// (on Pure USA `PI012`, a US-spelling overlay on `PI000`), is in [`Self::overlays`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageManifest {
    /// The disc serial, normalised to `AAAA-NNNNN`, as
    /// [`oag_assets`'s `Layout::serial`] reports it.
    ///
    /// [`oag_assets`'s `Layout::serial`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/assets/src/source.rs
    pub serial: &'static str,
    /// The plugin tokens, in manifest order. Manifest order is the picker's
    /// order on Pure (measured); on Pulse it is carried over from Pure.
    pub plugins: &'static [&'static str],
    /// The plugins this release's manifest loads that overlay the chosen language's
    /// strings and are not a language themselves, in manifest order: later ones win.
    /// Empty where the release loads none. A disc may carry a plugin its release
    /// does not load (Pure EU ships `PI012` byte-identical to the USA's and never
    /// loads it), so this is the release's list, never "whatever the disc holds".
    pub overlays: &'static [&'static str],
    /// Where the table was read and what was left out, for the doc page.
    pub evidence: &'static str,
}

/// The plugins a source offers: its own release's manifest when `serial` names
/// one, otherwise the title's default list.
///
/// `assumed` is the release a source with **no readable serial** is taken to be
/// (an extracted tree that kept no `param.sfo`); it never overrides a serial the
/// source did report.
#[must_use]
pub fn offered<'a>(
    manifests: &'a [LanguageManifest],
    default: &'a [&'static str],
    assumed: Option<&str>,
    serial: Option<&str>,
) -> &'a [&'static str] {
    serial
        .or(assumed)
        .and_then(|serial| manifests.iter().find(|m| m.serial == serial))
        .map_or(default, |m| m.plugins)
}

/// The overlay plugins a source offers: its release's [`LanguageManifest::overlays`], resolved
/// as [`offered`] resolves a release. Empty for a release no manifest records.
#[must_use]
pub fn overlays_of(
    manifests: &[LanguageManifest],
    assumed: Option<&str>,
    serial: Option<&str>,
) -> &'static [&'static str] {
    serial
        .or(assumed)
        .and_then(|serial| manifests.iter().find(|m| m.serial == serial))
        .map_or(&[], |m| m.overlays)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: LanguageManifest = LanguageManifest {
        serial: "AAAA-00001",
        plugins: &["PI000"],
        overlays: &["PI012"],
        evidence: "",
    };

    #[test]
    fn a_known_serial_gets_its_own_list_and_anything_else_the_default() {
        let default: &[&str] = &["PI000", "PI001"];
        assert_eq!(offered(&[A], default, None, Some("AAAA-00001")), &["PI000"]);
        assert_eq!(offered(&[A], default, None, Some("BBBB-00002")), default);
        assert_eq!(offered(&[A], default, None, None), default);
    }

    #[test]
    fn an_assumed_release_stands_in_for_a_missing_serial_and_only_that() {
        let default: &[&str] = &["PI000", "PI001"];
        let assumed = Some("AAAA-00001");
        assert_eq!(offered(&[A], default, assumed, None), &["PI000"]);
        assert_eq!(offered(&[A], default, assumed, Some("BBBB-00002")), default);
    }

    #[test]
    fn an_overlay_belongs_to_its_release_alone() {
        assert_eq!(overlays_of(&[A], None, Some("AAAA-00001")), &["PI012"]);
        assert!(overlays_of(&[A], None, Some("BBBB-00002")).is_empty());
        assert!(overlays_of(&[A], None, None).is_empty());
        assert_eq!(
            overlays_of(&[A], Some("AAAA-00001"), None),
            &["PI012"],
            "an assumed release stands in for a missing serial"
        );
    }
}
