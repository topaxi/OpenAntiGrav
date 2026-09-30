//! How a title's own boot sequence goes, as a table rather than as branches.
//!
//! This is the [ADR-0023] half of a title package: `oag-game` holds one generic
//! mechanism, and which screens it walks - in what order, playing which movies -
//! is here, filled in by `oag-pulse` and `oag-pure`.
//!
//! # Measured, not derived
//!
//! The reason this is a table and not a rule is that **a front-end XML's
//! declared entry point is not the runtime's.** Both titles' `Skin.xml` declare
//! the language picker first. Pure's runtime agrees; Pulse's does not - a cold
//! boot opens straight into `LogoFMV` playing its intro and runs on to
//! `Show Logo`, with no picker in between. So the order cannot be read out of the
//! data, cannot be inferred from a screen-name probe, and cannot be shared
//! between titles by analogy. Each title's chain is a measurement, and the
//! evidence for each is on that title's own page:
//! `docs/architecture/pure-boot.md` and `docs/architecture/frontend-boot.md`.
//!
//! # No `Screens`, and nothing here recognises a source
//!
//! Nothing in this module names a type from `oag-game` - it could not, the
//! dependency rules forbid the edge - and nothing here decides which title a
//! source is either. That was settled from the source's serial before any XML was
//! parsed, so a profile is *handed* to the mechanism rather than matched against
//! it. Where a source's own screen list still matters, the mechanism passes a
//! predicate and the screen model stays where it lives.
//!
//! # What is deliberately absent
//!
//! **What a state does.** Which buttons leave it, how long it holds, how it
//! draws: all behaviour, all in `oag-game`. A table that encoded it would be a
//! state-machine interpreter living in a data crate, which is the `trait Game`
//! ADR-0022 refused wearing a data hat. This module says *which* screens and
//! *which* movies, and stops there.
//!
//! [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md

/// One step of a boot sequence: a screen, and the movie it plays there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootStep {
    /// The screen's name, spelled exactly as the title's own front-end XML
    /// spells it - or as its executable does, where the two differ and the
    /// runtime was what settled it.
    pub state: &'static str,
    /// The archive entry this screen plays, or `None` for a step with no movie
    /// at all.
    ///
    /// `None` is a fact rather than a gap. Pure's boot step plays nothing: its
    /// first frame after power-on is the picker itself, cold-boot confirmed on
    /// both pressings. A `hash:`-prefixed spec is accepted for a movie whose name
    /// has not been recovered.
    pub movie: Option<&'static str>,
}

impl BootStep {
    /// A step that plays no movie.
    #[must_use]
    pub const fn screen(state: &'static str) -> Self {
        Self { state, movie: None }
    }

    /// A step that plays one.
    #[must_use]
    pub const fn playing(state: &'static str, movie: &'static str) -> Self {
        Self {
            state,
            movie: Some(movie),
        }
    }
}

/// Where a chain's order came from, which is not the same question as what the
/// order is.
///
/// **This field exists because the alternative was a lie or a refusal.** This
/// module's own docs open by recording that a front-end XML's declared entry
/// point is not the runtime's, and [`BootProfile::chain`] was therefore defined
/// as a measurement - so a title whose order is only declared could not be
/// expressed at all. Wipeout HD was that title: its `skin.xml` declared the
/// screens in order and no capture of a PS3 running existed, so filling `chain`
/// in silently would have put a hypothesis in the one field whose contract was
/// that it never held one, and leaving it empty refused a front end whose layout
/// is fully recovered.
///
/// **HD's chain has since been watched** (2026-09-05, three cold boots on
/// RPCS3), and Wipeout 2048 landed `Declared` in its place under
/// [ADR-0054](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md) -
/// one Vita3K boot corroborates its order, short of the three independent
/// ones HD's own `Measured` needed. This field was never about any one
/// title holding the value at a given moment; it is about *whichever*
/// title's declaration arrives before its capture does, which on
/// 2026-09-20 is 2048.
///
/// Widening the type is the third answer, and it is the honest one: the chain
/// still says what the order is, and this says how much that is worth. Every
/// caller that shows a boot to a person is expected to say which it got - see
/// [ADR-0025].
///
/// [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A cold boot of the original was watched, and this is the order it went
    /// in.
    ///
    /// Both PSP titles, and Wipeout HD since 2026-09-05. Pulse's is the reason
    /// the distinction is needed at all: its XML declares the picker first and
    /// its runtime opens on `LogoFMV` instead, so a build that had trusted the
    /// declaration would have shipped the wrong sequence and had no way to know.
    ///
    /// **"Watched" is not "watched on the console."** HD's is three boots on
    /// RPCS3, which is why its docs page scores the chain 85 rather than higher.
    /// This enum is deliberately two-valued and grades nothing - ADR-0025
    /// rejected a confidence number in the type, and the rubric score lives on
    /// the docs page where it can carry its evidence.
    Measured,
    /// The title's own front-end XML declares this order, and nothing has
    /// watched it run.
    ///
    /// A real reading of real data - the redirects were followed screen by
    /// screen - and **not** evidence about the runtime. Anything user-facing
    /// that walks such a chain says so rather than presenting it as the disc's
    /// behaviour.
    ///
    /// **Wipeout 2048 is in this state since [ADR-0054]**, HD having held it
    /// before that and had its own chain watched (2026-09-05). 2048's own
    /// chain is corroborated by one Vita3K cold boot rather than watched
    /// three independent times the way ADR-0025's own bar asks for, so it
    /// stays `Declared` rather than moving to `Measured` on a single run -
    /// see `oag_2048::frontend::BOOT_PROFILE`'s own doc comment. The variant
    /// stays live either way, because the whole point of the field is that a
    /// declaration arriving as a measurement by omission must not be
    /// possible. `oag_game::boot::provenance::caveat` is where it is
    /// enforced, and it is unit-tested rather than disc-tested for exactly
    /// this reason.
    ///
    /// [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
    Declared,
}

impl Provenance {
    /// Whether this order has been watched rather than only read.
    #[must_use]
    pub fn is_measured(self) -> bool {
        matches!(self, Self::Measured)
    }
}

/// One title's boot sequence, as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootProfile {
    /// Where [`Self::chain`]'s order came from. See [`Provenance`].
    ///
    /// Deliberately not defaulted and not an `Option`: a profile is a
    /// compile-time constant in a title package, and making the author write
    /// one of two words is what stops a fourth title's declaration from
    /// arriving as a measurement by omission.
    pub provenance: Provenance,
    /// Every screen a default boot walks, in order, starting at index 0.
    ///
    /// An ordered chain rather than a set of decision points, because the
    /// sequences genuinely differ in *length*: Pulse's is three screens and
    /// Pure's is five, two of which sit between the picker and the movie. An
    /// earlier shape here - a boot leg plus a single after-the-picker target -
    /// could not express those two at all, and was discarded once they were
    /// measured.
    pub chain: &'static [BootStep],
    /// Where `--reel` starts, for a title that has such a leg at all.
    ///
    /// `None` is not "no default": it is "this title has no off-path reel state
    /// anyone has found", and the flag is refused by name rather than quietly
    /// pointed at another title's screen. Both titles ship a reel *file*; only
    /// Pulse has an evidenced state that plays it.
    pub reel: Option<BootStep>,
    /// The looping backdrop this title's menus sit on, if it ships one.
    ///
    /// `None` for a title that ships none, which is a measurement: neither Pure
    /// pressing carries `Data\Movies\Backdrop.PMF`.
    pub menu_backdrop: Option<&'static str>,
    /// The screen whose own solid fills the language picker inherits.
    ///
    /// `None` where the picker carries its own background. `Some` where it is a
    /// child screen with no fill of its own and would otherwise draw on black -
    /// Pure's, which sits on `Intro Screen`'s white.
    pub picker_backdrop_parent: Option<&'static str>,
    /// Whether the language picker's row pitch is one line of the `Default`
    /// face this build loaded, rather than `oag_ui`'s hard-coded table of
    /// Pulse's line heights.
    ///
    /// The picker's `<Menu>` states no pitch (the disc authors x, y, scale,
    /// colour and alignment only), so the step is one line of the widget's own
    /// font. Pulse's table happens to be that for Pulse; on HD and Omega it is
    /// 13 against a 33 or 64 unit face and the rows print over each other.
    /// `false` keeps the table, which is what Pulse, Pure and 2048 draw
    /// against captures matched with it.
    pub picker_row_pitch_from_face: bool,
    /// `FEGlobals` this title's own `Skin.xml` leaves undeclared, measured.
    ///
    /// Merged in only for keys the disc does not declare; a real declaration
    /// always wins. Empty for a title that declares everything its screens name.
    pub fallback_globals: &'static [(&'static str, &'static str)],
    /// `Image` widgets this title's own XML gives no `src` at all - assigned
    /// programmatically on the original, the same class of gap
    /// [`Self::fallback_globals`] fills for a colour - matched here by the
    /// widget's own `name` attribute against an archive entry a content scan
    /// found by its picture rather than its (unrecovered) name.
    ///
    /// Each value takes [`BootStep::movie`]'s own `hash:`-prefixed spelling,
    /// for the same reason: a WAD directory holds only a name's hash, so an
    /// entry nobody has named a string for is still addressable by it. Merged
    /// in only where a widget's own XML leaves `src` unset; a real one always
    /// wins. Empty for a title with no such gap found yet.
    pub fallback_images: &'static [(&'static str, &'static str)],
}

impl BootProfile {
    /// Where a default boot starts, and what plays there.
    ///
    /// # Panics
    ///
    /// If the chain is empty. A title with no boot sequence is not a title this
    /// build can open, and every profile is a compile-time constant, so an empty
    /// one is a typo rather than a runtime condition.
    #[must_use]
    pub fn start(&'static self) -> &'static BootStep {
        self.chain
            .first()
            .expect("a boot profile names at least one screen")
    }

    /// The step for a named screen, if the chain has one.
    #[must_use]
    pub fn step(&'static self, state: &str) -> Option<&'static BootStep> {
        self.chain.iter().find(|step| step.state == state)
    }

    /// The next step after `state` that `usable` accepts.
    ///
    /// `usable` is asked rather than a screen model passed, and it answers two
    /// questions at once that the caller is better placed to answer than this
    /// crate is: whether *this pressing* carries the screen, and whether this
    /// build can drive it yet. A step failing either is stepped over rather than
    /// stalling the boot on it.
    ///
    /// `None` once the chain runs out, which is the end of the boot sequence
    /// rather than an error.
    #[must_use]
    pub fn next_after(
        &'static self,
        state: &str,
        usable: impl Fn(&str) -> bool,
    ) -> Option<&'static BootStep> {
        let at = self.chain.iter().position(|step| step.state == state)?;
        self.chain[at + 1..].iter().find(|step| usable(step.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "A";
    const B: &str = "B";
    const C: &str = "C";

    static CHAIN: BootProfile = BootProfile {
        provenance: Provenance::Measured,
        chain: &[
            BootStep::screen(A),
            BootStep::screen(B),
            BootStep::playing(C, "movie"),
        ],
        reel: None,
        menu_backdrop: None,
        picker_backdrop_parent: None,
        picker_row_pitch_from_face: false,
        fallback_globals: &[],
        fallback_images: &[],
    };

    #[test]
    fn a_boot_starts_at_the_head_of_its_chain() {
        assert_eq!(CHAIN.start().state, A);
        assert!(CHAIN.start().movie.is_none());
    }

    #[test]
    fn a_step_is_found_by_its_screen_name() {
        assert_eq!(CHAIN.step(C).unwrap().movie, Some("movie"));
        assert!(CHAIN.step("nothing").is_none());
    }

    #[test]
    fn the_next_step_is_the_one_after_it() {
        assert_eq!(CHAIN.next_after(A, |_| true).unwrap().state, B);
    }

    #[test]
    fn a_step_this_source_cannot_use_is_stepped_over() {
        // The case the predicate exists for: a pressing missing the screen, or a
        // screen this build cannot drive yet, must not stall the boot on it.
        assert_eq!(CHAIN.next_after(A, |state| state != B).unwrap().state, C);
    }

    #[test]
    fn the_end_of_the_chain_is_the_end_of_the_sequence() {
        assert!(CHAIN.next_after(C, |_| true).is_none());
        assert!(
            CHAIN.next_after(A, |_| false).is_none(),
            "nothing usable ahead is the same answer as nothing ahead"
        );
    }

    #[test]
    fn a_screen_not_in_the_chain_has_no_next() {
        assert!(CHAIN.next_after("nothing", |_| true).is_none());
    }
}
