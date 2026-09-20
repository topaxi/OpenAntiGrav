//! Whether a boot report says its chain was only read, or says nothing.
//!
//! Its own file rather than a few lines inside `boot.rs`: that file is
//! baselined by `scripts/check-file-size.py` and may shrink but not grow, and
//! this arrived as a *net addition* - the `if` it replaces was five lines and
//! the reasoning that has to travel with it is not. See
//! [ADR-0025](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md).

/// The report line a boot of a [`oag_title::Provenance::Declared`] chain gets,
/// or `None` for a chain somebody has watched.
///
/// **ADR-0025's whole enforcement is this function.** What replaced the old
/// `front_end: None` refusal is a label, and a label nobody prints is worth
/// nothing - so it is said on every such boot, near the top where the reader is
/// still looking, before any of it is drawn. A sequence read out of the disc's
/// XML is not a sequence anyone has watched, and a screenshot of one must not be
/// filed as evidence of what the original does.
///
/// It is a function rather than an `if` inside `load_shell` for a reason that
/// first arrived on 2026-09-05: Wipeout HD's chain was watched that day and
/// briefly left no title `Declared`, so the disc-backed test that used to
/// assert this line appears had nothing left to assert it against. Wipeout
/// 2048 is `Declared` again since [ADR-0054] (one Vita3K boot corroborates
/// its order rather than the three independent ones HD's own `Measured`
/// needed - see `oag_2048::frontend::BOOT_PROFILE`), so the line is back to
/// being exercised on a real title, but the function stays regardless of
/// which titles currently hold which value: it is testable without a disc,
/// an `if` inside a 200-line loader is not, and the guarantee would
/// otherwise quietly stop being covered every time the roster happens to
/// agree on `Measured`.
///
/// [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
pub(crate) fn caveat(title: &str, provenance: oag_title::Provenance) -> Option<String> {
    (!provenance.is_measured()).then(|| {
        format!(
            "{title}: this order is what its front-end XML declares, not a boot \
             anyone has watched"
        )
    })
}

#[cfg(test)]
mod tests {
    /// A declared chain says so in the report, and a measured one says nothing.
    ///
    /// **ADR-0025's guarantee, and this stayed the only unconditional place it
    /// is covered even while a disc-backed title also exercised it again.**
    /// The disc-backed test that used to assert the line appeared did so
    /// against Wipeout HD; two cold boots on RPCS3 on 2026-09-05 upgraded it
    /// to `Measured`, and the assertion had nothing left to fire on until
    /// Wipeout 2048 landed `Declared` in its place under [ADR-0054]. Keeping
    /// this unit test regardless is what stops the ADR's enforcement from
    /// depending on the roster always containing a `Declared` title - a
    /// future title arriving as a declaration would print nothing and nobody
    /// would know, on a day this project happened to be between two discless
    /// checks.
    ///
    /// [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
    #[test]
    fn a_declared_chain_is_labelled_and_a_measured_one_is_not() {
        let declared = super::caveat("Some Title", oag_title::Provenance::Declared)
            .expect("a declared chain must be labelled");
        assert!(
            declared.contains("declares") && declared.contains("not a boot anyone has watched"),
            "the label has to say which it is: {declared}"
        );
        assert!(declared.starts_with("Some Title:"), "named: {declared}");
        assert_eq!(
            super::caveat("Some Title", oag_title::Provenance::Measured),
            None,
            "a watched chain carries no caveat"
        );
    }
}
