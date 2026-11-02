//! How a title's own boot sequence goes, as a table rather than as branches.
//!
//! The [ADR-0023] half of a title package: `oag-game` holds one generic mechanism,
//! and which screens it walks, in what order, playing which movies, is here.
//!
//! # Measured, not derived
//!
//! **A front-end XML's declared entry point is not the runtime's.** Both titles'
//! `Skin.xml` declare the language picker first. Pure's runtime agrees; Pulse's
//! does not: a cold boot opens straight into `LogoFMV` playing its intro and runs
//! on to `Show Logo`, with no picker. So the order cannot be read out of the data,
//! inferred from a screen-name probe, or shared between titles by analogy. Each
//! title's chain is a measurement, evidenced on `docs/architecture/pure-boot.md`
//! and `docs/architecture/frontend-boot.md`.
//!
//! # Nothing here recognises a source
//!
//! Nothing here names a type from `oag-game` (the dependency rules forbid the
//! edge), and which title a source is was settled from its serial before any XML
//! was parsed: a profile is *handed* to the mechanism. Where a source's own screen
//! list matters, the mechanism passes a predicate.
//!
//! # What is absent
//!
//! **What a state does** (which buttons leave it, how long it holds, how it
//! draws) is behaviour, in `oag-game`. A table encoding it would be a
//! state-machine interpreter in a data crate, the `trait Game` ADR-0022 refused.
//! This module says *which* screens and *which* movies.
//!
//! [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md

/// One step of a boot sequence: a screen, and the movie it plays there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootStep {
    /// The screen's name, spelled as the title's front-end XML spells it, or as its
    /// executable does where the two differ and the runtime settled it.
    pub state: &'static str,
    /// The archive entry this screen plays, or `None` for a step with no movie.
    ///
    /// `None` is a fact: Pure's boot step plays nothing, its first frame after
    /// power-on is the picker itself (cold-boot confirmed on both pressings). A
    /// `hash:`-prefixed spec is accepted for a movie whose name is unrecovered.
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
/// **Exists because the alternative was a lie or a refusal.** [`BootProfile::chain`]
/// is a measurement, so a title whose order is only declared could not be
/// expressed: HD's `skin.xml` declared the screens in order and no capture of a PS3
/// running existed, so filling `chain` silently would put a hypothesis in a field
/// whose contract is that it never holds one, and leaving it empty refused a front
/// end whose layout is fully recovered.
///
/// HD's chain has since been watched (three cold boots on RPCS3,
/// `docs/formats/hd-frontend.md`, "The boot chain, declared and then watched"), and
/// 2048 is `Declared` in its place under
/// [ADR-0054](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md):
/// one Vita3K boot corroborates its order, short of the three independent boots
/// HD's `Measured` needed. The field is about *whichever* title's declaration
/// arrives before its capture does.
///
/// Every caller that shows a boot to a person says which it got ([ADR-0025]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A cold boot of the original was watched, and this is the order it went in.
    ///
    /// Both PSP titles and HD. Pulse's is why the distinction exists: its XML
    /// declares the picker first and its runtime opens on `LogoFMV`, so a build
    /// trusting the declaration would have shipped the wrong sequence unawares.
    ///
    /// **"Watched" is not "watched on the console."** HD's is three boots on RPCS3,
    /// which is why its docs page scores the chain 85. The enum grades nothing
    /// (ADR-0025 rejected a confidence number in the type); the score lives on the
    /// docs page with its evidence.
    Measured,
    /// The title's own front-end XML declares this order, and nothing has watched it
    /// run.
    ///
    /// A real reading of real data (the redirects were followed screen by screen)
    /// and **not** evidence about the runtime. Anything user-facing that walks such
    /// a chain says so rather than presenting it as the disc's behaviour.
    ///
    /// **2048 is here since [ADR-0054]**: its chain is corroborated by one Vita3K
    /// cold boot, not the three independent boots ADR-0025's bar asks for
    /// (`oag_2048::frontend::BOOT_PROFILE`). The variant stays live because a
    /// declaration arriving as a measurement by omission must not be possible;
    /// `oag_game::boot::provenance::caveat` enforces it, unit-tested rather than
    /// disc-tested for that reason.
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
    /// Not defaulted and not an `Option`: the author of a compile-time profile
    /// writes one of two words, so a declaration cannot arrive as a measurement by
    /// omission.
    pub provenance: Provenance,
    /// Every screen a default boot walks, in order, starting at index 0.
    ///
    /// An ordered chain because the sequences differ in *length*: Pulse's is three
    /// screens and Pure's five, two between the picker and the movie.
    pub chain: &'static [BootStep],
    /// Where `--reel` starts, for a title that has such a leg.
    ///
    /// `None` is "no off-path reel state anyone has found", and the flag is refused
    /// by name rather than pointed at another title's screen. Both titles ship a
    /// reel *file*; only Pulse has an evidenced state that plays it.
    pub reel: Option<BootStep>,
    /// The looping backdrop this title's menus sit on, if it ships one. `None` is a
    /// measurement: neither Pure pressing carries `Data\Movies\Backdrop.PMF`.
    pub menu_backdrop: Option<&'static str>,
    /// Whether this title's menus sit on the HD-style `BackgroundAnim` widget: a 3D
    /// scene named by the skin, filtered to a grey drawing and drawn behind the rows.
    ///
    /// **Per-title because the widget is in every HD-lineage skin and only some
    /// titles have a reading worth drawing.** HD's `skin.xml` authors the same
    /// `<BackgroundAnim>` and this build draws nothing there (`false`). Omega ships
    /// no Fury clouds, so this widget is the only backdrop it can have.
    pub menu_scene: bool,
    /// The screen whose own solid fills the language picker inherits. `None` where
    /// the picker carries its own background; `Some` where it is a child screen with
    /// no fill and would draw on black (Pure's, on `Intro Screen`'s white).
    pub picker_backdrop_parent: Option<&'static str>,
    /// Whether the language picker is drawn from the faces this build loaded rather
    /// than from `oag_ui`'s hard-coded table of Pulse's line heights and `menu` face.
    ///
    /// Two things follow. The row pitch is one line of the loaded `Default` face.
    /// And the picker's `ControlTextConfirmButton`/`ControlTextBackButton` draw in
    /// the `Buttons` face: their strings are button-glyph codepoints
    /// (`FE_CONFIRM_BUTTON` is `"\u{3b5}"`, present only in `PS_BUTTONS.fnt`), they
    /// author `font="menu"`, and HD's plugins declare no `menu` role, so the glyph
    /// would draw as an empty box. The original's picker was never watched on HD, so
    /// the face is **chosen, not measured**.
    ///
    /// The picker's `<Menu>` authors no pitch, so the step is one line of the
    /// widget's font. Pulse's table is that for Pulse; on HD and Omega it is 13
    /// against a 33 or 64 unit face and rows print over each other. `false` keeps
    /// the table, which Pulse, Pure and 2048 draw against matched captures.
    pub picker_from_loaded_faces: bool,
    /// `FEGlobals` this title's `Skin.xml` leaves undeclared, measured. Merged in
    /// only for keys the disc does not declare; a real declaration wins.
    pub fallback_globals: &'static [(&'static str, &'static str)],
    /// The `<Screen>` of name-only `<Variable>`s in this title's `skin.xml` whose
    /// values overwrite the skin's globals of the same name, if it carries one (see
    /// `oag_ui::screen::Screens::from_xml_with_fallbacks`).
    ///
    /// HD ships one baked skin per archive (Fury's colours in one, HD's in another),
    /// so it has no such block; Omega folds the styles into one file and the
    /// executable copies one style's values over the base block's. `None` where
    /// there is nothing to overwrite.
    pub colour_template: Option<&'static str>,
    /// `Image` widgets this title's XML gives no `src` at all (assigned
    /// programmatically on the original, the gap [`Self::fallback_globals`] fills
    /// for a colour), matched by the widget's `name` against an archive entry a
    /// content scan found by its picture rather than its unrecovered name.
    ///
    /// Each value takes [`BootStep::movie`]'s `hash:` spelling: a WAD directory holds
    /// only a name's hash. Merged in only where the widget's XML leaves `src` unset.
    pub fallback_images: &'static [(&'static str, &'static str)],
}

impl BootProfile {
    /// Where a default boot starts, and what plays there.
    ///
    /// # Panics
    ///
    /// If the chain is empty: a title with no boot sequence cannot be opened, and a
    /// profile is a compile-time constant, so that is a typo.
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
    /// `usable` is asked rather than a screen model passed, and answers two
    /// questions the caller is better placed for: whether *this pressing* carries
    /// the screen, and whether this build can drive it yet. A step failing either is
    /// stepped over. `None` once the chain runs out (the end of the boot, not an
    /// error).
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
        menu_scene: false,
        picker_backdrop_parent: None,
        picker_from_loaded_faces: false,
        fallback_globals: &[],
        colour_template: None,
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
        // A pressing missing the screen, or a screen this build cannot drive yet,
        // must not stall the boot.
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
