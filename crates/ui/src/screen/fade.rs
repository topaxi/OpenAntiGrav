//! A widget's own fade-in duration, resolved off `EnableTransition`/
//! `Transition` the same way `interpolate_reveal` resolves an `<Animation>`'s
//! `<Key>` timeline - split out of `screen.rs` under the 1,000-line rule
//! (`scripts/check-file-size.py`), the same shape `reveal.rs` already split
//! off for its own seam.

/// The measured fade-in duration a widget gets when it authors neither
/// `EnableTransition` nor `Transition` at all, and starts hidden
/// (`StartEnabled="false"`).
///
/// Read off a live `pure-psp-eu.chd` boot (PPSSPP v1.20.4, websocket
/// debugger) at the moment `Title Screen->TitleFrame` settles: its own
/// `+0x70`/`+0x74` (`Element_UpdateFade`'s fade-in/fade-out seconds,
/// `docs/ghidra/functions/psp-pure-eu/title-screen.md`) both read exactly
/// `0.1`, at address `0x08ed6880` - matching the address a live-capture
/// two passes earlier already recorded for the same widget on the same
/// boot script, so this is reproducible rather than a one-off heap
/// coincidence. Neither field is written anywhere in
/// `TitleScreen_AssignWordmarkTexture`, which only ORs `TitleFrame`'s own
/// `+0x2c` flags to arm the fade - so `0.1` is already present before that
/// function runs, i.e. a class-wide default baked in at construction, not
/// an XML attribute (confirmed separately: neither `Data\Plugins\PI001\GUI\
/// Skin.xml` nor the activated style skin, `Data\Skins\Default\Skin.xml`,
/// authors any `Fade*`/`Transition*` attribute or global anywhere in either
/// file - a full-text search of both expanded XMLs comes back empty).
///
/// **General, not `TitleFrame`-specific**: the same live boot's `Title
/// Screen->Viewport`, whose own `<Viewport enabletransition="0.7">` sets its
/// *own* `+0x70` to `0.7`, still reads `+0x74` (its `DisableTransition`,
/// unauthored) as this same `0.1` - a second, independent element hitting
/// the identical default. So does every other plain, non-`<Animation>`-
/// wrapped widget on the same screen once walked live: the colour-only
/// "White Background" `Fill`, `PRESS START`, `StartCursor` and `HOLD ON!`
/// (all `0.1`/`0.1`), while every `<Animation>` object itself (the width-
/// wipe wrapper, not the `Image`/`Fill` it wraps) reads `0.0`/`0.0` -
/// `Animation_ConstructFromNode` deliberately overwrites the same class
/// default rather than never receiving one, so an `<Animation>`-wrapped
/// widget's own reveal is the `TextureWidth` wipe alone, not a second,
/// redundant alpha fade on top of it.
///
/// **Scoped to `StartEnabled="false"` widgets, not applied unconditionally.**
/// The live reads above show the real engine does *not* gate this default on
/// `StartEnabled` at all - `PRESS START` etc. start enabled and still get
/// `0.1`. This build narrows that on purpose: `Screens::from_xml_with_fallbacks`
/// is the one generic parser both Pulse's and Pure's own front end run
/// through, and only Pure EU was live-measured. Pulse's own equivalent
/// mechanism is independently confirmed to exist (`Widget_CreateFromElement`/
/// `Widget_UpdateTransitionFraction`,
/// `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`) but its own
/// unauthored-default value was never read there, and its one
/// `StartEnabled="false"` `Image` (`GameShareBackdrop`) authors its own
/// literal `transition="0"` regardless. Gating on `StartEnabled="false"`
/// keeps this default from reaching every already-visible widget on every
/// other screen of every other title on the strength of one Pure EU capture -
/// a deliberately narrower boundary than the executable's own unconditional
/// one, not a claim that `StartEnabled="true"` widgets never fade in the
/// original.
pub const MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS: f32 = 0.1;

/// A widget's own resolved fade-in duration: its own authored `own` value
/// (`EnableTransition`, falling back to `Transition`, read directly off the
/// node) if present, else whatever cascaded down from an enclosing container
/// (`<LeftLayer transition="...">`'s own inheritance, `inherited` - already
/// `0.0` when nothing upstream ever authored either), else - only when the
/// widget also starts hidden - [`MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS`].
///
/// **Does not distinguish an inherited literal `0` from a true absence.**
/// Pulse's own title-bar `<LeftLayer transition="0">` cascades a real,
/// authored `0.0` to its children (`docs/ghidra/functions/psp-pulse-usa/
/// race-box-screens.md`'s own capture: solid from the first frame, not
/// faded) - and `inherited == 0.0` here reads identically whether that is
/// what happened or whether nothing was ever authored at all. `own` does not
/// have this gap (a widget's own node is read directly, so `None` only ever
/// means "not present on this element"), which is what keeps the one known
/// live case - Pulse's `GameShareBackdrop`, itself `StartEnabled="false"`
/// but authoring its own literal `transition="0"` - resolving to `0.0`
/// rather than being promoted to the default. A hypothetical
/// `StartEnabled="false"` widget with no attribute of its own, nested inside
/// a container that itself cascades an authored `0`, would be
/// indistinguishable from true absence here and would receive the default
/// it should not - no such widget is known to exist in either title's own
/// corpus, but the gap is real and undocumented nowhere else, so it is
/// written down here rather than silently relied on.
#[must_use]
pub fn resolve_fade_in(own: Option<f32>, inherited: f32, start_enabled: bool) -> f32 {
    match own.or(if inherited > 0.0 {
        Some(inherited)
    } else {
        None
    }) {
        Some(value) => value,
        None if !start_enabled => MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS,
        None => inherited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hidden_widget_with_nothing_authored_gets_the_measured_default() {
        assert_eq!(
            resolve_fade_in(None, 0.0, false),
            MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS
        );
    }

    #[test]
    fn a_visible_widget_with_nothing_authored_stays_instant() {
        assert_eq!(resolve_fade_in(None, 0.0, true), 0.0);
    }

    #[test]
    fn a_widgets_own_enabletransition_wins_over_the_default_even_when_hidden() {
        // Pulse's `GameShareBackdrop`: `StartEnabled="false"` but its own
        // literal `transition="0"` must stick, not be promoted.
        assert_eq!(resolve_fade_in(Some(0.0), 0.0, false), 0.0);
        assert_eq!(resolve_fade_in(Some(0.7), 0.0, false), 0.7);
    }

    #[test]
    fn an_inherited_transition_wins_over_the_default() {
        // `<LeftLayer transition="0.5">` cascading to a hidden child with no
        // attribute of its own.
        assert_eq!(resolve_fade_in(None, 0.5, false), 0.5);
    }
}
