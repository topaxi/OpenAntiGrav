//! The ways in to a [`Screens`]: reading a front-end XML file with the
//! fallbacks a title's own tables supply.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; the constructors moved as they were, with the
//! one shared reader [`Screens::read`] replacing the two copies of its body.

use super::{Screens, parse};

impl Screens {
    /// Reads a front-end XML file.
    #[must_use]
    pub fn from_xml(xml: &str) -> Self {
        Self::from_xml_with_fallback_globals(xml, &[])
    }

    /// [`Self::from_xml`], with `fallback` globals seeded in wherever the
    /// file itself leaves a name undeclared.
    ///
    /// **Before widget collection, not after** - a global has to exist while
    /// [`Self::text_from_node`]/[`Self::image_from_node`] resolve `color`,
    /// `x` and the rest, because those bake the resolved value into the
    /// widget once and never look the name up again. Merging `fallback` into
    /// [`Self::globals`] after `from_xml` has already returned looks
    /// plausible and does nothing - every widget that needed it has already
    /// been built with whatever `resolve` found at parse time.
    ///
    /// A real declaration always wins: `fallback` only fills a name the
    /// file's own `<Variable global="...">` list never mentions, the same
    /// contract `entry().or_insert()` gives everywhere else in this crate.
    ///
    /// Carries no `fallback_images` of its own - see
    /// [`Self::from_xml_with_fallbacks`] for the widget this file's own XML
    /// leaves with no `src` at all, which is a different kind of gap from an
    /// undeclared colour and needs its own table.
    #[must_use]
    pub fn from_xml_with_fallback_globals(xml: &str, fallback: &[(&str, &str)]) -> Self {
        Self::from_xml_with_fallbacks(xml, fallback, &[])
    }

    /// [`Self::from_xml_with_fallback_globals`], plus `fallback_images`: an
    /// `Image` widget's `src`, keyed by the widget's own `name` attribute, for
    /// a widget whose XML declares none at all - assigned programmatically on
    /// the original. Consulted only where a widget's own XML leaves `src`
    /// unset; a real one always wins, the same contract `fallback` carries for
    /// a colour.
    #[must_use]
    pub fn from_xml_with_fallbacks(
        xml: &str,
        fallback: &[(&str, &str)],
        fallback_images: &[(&str, &str)],
    ) -> Self {
        Self::read(xml, fallback, fallback_images, false)
    }

    /// [`Self::from_xml_with_fallback_globals`], with a colour-only `Image`'s
    /// own `OffsetX`/`OffsetY` folded into its [`Fill`] - see
    /// [`Self::fold_fill_offsets`]'s field doc for who needs which reading.
    #[must_use]
    pub fn from_xml_folding_fill_offsets(xml: &str, fallback: &[(&str, &str)]) -> Self {
        Self::read(xml, fallback, &[], true)
    }

    fn read(
        xml: &str,
        fallback: &[(&str, &str)],
        fallback_images: &[(&str, &str)],
        fold_fill_offsets: bool,
    ) -> Self {
        let root = parse(xml);
        let mut out = Self {
            fold_fill_offsets,
            ..Self::default()
        };
        for node in &root.children {
            out.collect_globals(node, None);
        }
        for &(name, value) in fallback {
            out.globals
                .entry(name.to_string())
                .or_insert_with(|| value.to_string());
        }
        for node in &root.children {
            out.collect(node, None, fallback_images);
        }
        out
    }
}
