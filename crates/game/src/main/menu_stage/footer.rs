//! The ordinary menu pages' own footer: the front-end root's `Confirm`/`Back`
//! legend and its scrolling tip ticker - split out of `menu_stage.rs` once
//! wiring the ticker in pushed that file past
//! `scripts/check-file-size.py`'s 1,000-line ceiling. The legend half is a
//! plain move (`MenuStage::render` drew it inline before); the ticker half
//! is new - see `MenuStage::render`'s own call site for both.

use oag_ui::font;
use oag_ui::frontend::Draw;
use oag_ui_screens::picker;

use super::MenuStage;

/// The front-end root's `Confirm`/`Back` legend under Wipeout HD/Fury's own
/// `Team Selection` - `Back` shown, since the screen is always reached from
/// another. An RPCS3 frame of the screen shows both under it
/// (`screen-Team-Selection.png`, `racebox` walk). Empty on every other
/// title's picker, whose screens author their own footer text.
pub(super) fn hd_nav(
    legend: &Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    atlas: &font::Atlas,
    picker: &crate::picker_stage::PickerStage,
) -> Vec<Draw> {
    match legend {
        Some(legend) if picker.layout.is_hd() => legend.draw_gated(
            &picker::FaceScales::default(),
            &|text: &str| font::measure(atlas, text),
            true,
        ),
        _ => Vec::new(),
    }
}

/// The ticker's own clip candidate: the draw's index in a flattened list,
/// plus its viewport's `(left, right)` in screen space - unresolved against
/// the value marquee's own clip until [`resolve_clip`] combines the two.
type TickerClip = Option<(usize, (f32, f32))>;

impl MenuStage {
    /// The front-end root's own `Confirm`/`Back` legend - see
    /// [`Self::nav_legend`]'s own doc for what `None` means.
    ///
    /// `Confirm` always; `Back` only past the tree's own root
    /// (`self.menu.depth() > 1`) - this build's own tree has no disc screen
    /// to read a gate off, but the boundary itself is measured, not guessed:
    /// see `docs/architecture/menus.md`'s "A mouse and a finger" section
    /// (`NavigationButtons` census, confidence ~70) for the two real RPCS3
    /// captures - `Main Menu` alone shows `Confirm` only, a page reached
    /// from it shows both - this reproduces.
    #[must_use]
    fn nav_legend_overlay(&self) -> Vec<Draw> {
        let Some(legend) = &self.nav_legend else {
            return Vec::new();
        };
        let measure = |text: &str| font::measure(&self.default_atlas, text);
        legend.draw_gated(
            &picker::FaceScales::default(),
            &measure,
            self.menu.depth() > 1,
        )
    }

    /// The footer's own scrolling tip ticker, at [`Self::ticker_elapsed`] -
    /// see [`Self::ticker`]'s own doc. `None` on a source with none, no
    /// tips honestly to show, or (briefly, once a cycle) the gap between one
    /// tip leaving and the next entering - see
    /// `oag_ui_screens::campaign::footer::ticker_draw`'s own doc for that last case.
    ///
    /// Never gated on depth the way [`Self::nav_legend_overlay`]'s `Back`
    /// is: nothing measured suggests the disc hides the ticker on any
    /// ordinary page it appears on. The bound pair alongside the draw is the
    /// ticker's own viewport, `(left, right)` in screen space - what a
    /// caller turns into `Renderer::render_with`'s own `clip` once it has
    /// found the draw's index in its own flattened list, the same
    /// `oag_ui_screens::campaign::footer`-driven idiom
    /// `crate::campaign_stage::CampaignStage::ticker_clip_bounds` already
    /// uses for the Race Campaign's own footer.
    ///
    /// `measure` reads [`Self::default_atlas`]: the ticker draws in the
    /// `Default` role, the face the disc's own tips are lowercase in.
    #[must_use]
    fn ticker_overlay(&self, tips: &[String]) -> Option<(Draw, (f32, f32))> {
        let layout = self.ticker.as_ref()?;
        let measure = |text: &str| font::measure(&self.default_atlas, text);
        let draw = oag_ui_screens::campaign::footer::ticker_draw(
            layout,
            self.ticker_elapsed,
            tips,
            &picker::FaceScales::default(),
            &measure,
        )?;
        let [x, _, width, _] = layout.viewport;
        Some((draw, (x, x + width)))
    }

    /// The footer a picker carries: the `Confirm`/`Back` legend and the ticker,
    /// on a title that draws a ticker at all and whose picker is its own
    /// layout (not HD's, which [`hd_nav`] covers). Pulse's pickers author the
    /// `Help` prompt and nothing else of the footer, and the original shows
    /// the rest under them. `None` elsewhere. The clip's index is into the
    /// returned draws; [`append`] moves it.
    #[must_use]
    pub(super) fn picker_footer(&self, tips: &[String]) -> Option<(Vec<Draw>, TickerClip)> {
        let own = self.picker.as_ref().is_some_and(|p| !p.layout.is_hd());
        (own && self.ticker.is_some()).then(|| self.footer_overlay(Vec::new(), tips))
    }

    /// [`Self::nav_legend_overlay`] and [`Self::ticker_overlay`] appended to
    /// `list`, in that order - `MenuStage::render`'s own call site, pulled
    /// out here so the `self.change.is_none()` gate that skips both
    /// mid-transition is one branch there rather than two. The second half
    /// of the pair is the ticker's own clip, unresolved: see
    /// [`resolve_clip`] for what a caller still has to combine it with.
    #[must_use]
    pub(super) fn footer_overlay(
        &self,
        list: Vec<Draw>,
        ticker_tips: &[String],
    ) -> (Vec<Draw>, TickerClip) {
        let list: Vec<Draw> = list.into_iter().chain(self.nav_legend_overlay()).collect();
        match self.ticker_overlay(ticker_tips) {
            Some((draw, bounds)) => {
                let index = list.len();
                (
                    list.into_iter().chain(std::iter::once(draw)).collect(),
                    Some((index, bounds)),
                )
            }
            None => (list, None),
        }
    }
}

/// `footer`'s draws after `flat`, with the ticker's clip index moved to where
/// the ticker landed in the joined list. `footer` is [`MenuStage::picker_footer`]'s.
#[must_use]
pub(super) fn append(
    mut flat: Vec<Draw>,
    footer: Option<(Vec<Draw>, TickerClip)>,
) -> (Vec<Draw>, TickerClip) {
    let Some((extra, clip)) = footer else {
        return (flat, None);
    };
    let at = flat.len();
    flat.extend(extra);
    (flat, clip.map(|(index, bounds)| (index + at, bounds)))
}

/// The clip `Self::render` hands `Renderer::render_with`, combining the value
/// marquee's own (`marquee_clip`, from `oag_ui_screens::marquee::apply`) with the
/// ticker's (`ticker_clip`, from [`MenuStage::ticker_overlay`]) - the render
/// pass has exactly one clip slot, and the two can genuinely both want it on
/// the same frame (an overflowing settings value while the footer ticker is
/// also scrolling).
///
/// **Chosen: the marquee wins.** An overflowing value is the thing a player
/// is reading; an unclipped ticker for as long as that row stays focused and
/// overflowing - not just one frame, since nothing here times out a held
/// selection - is a page-edge overdraw past its own footer strip, not a
/// correctness bug. `ticker_clip`'s own
/// index is shifted by one exactly when `frozen_race`, the same way the Race
/// Campaign's own ticker clip is (`CampaignStage::ticker_clip_bounds`'s call
/// site) - the pause overlay's `Fill` is prepended ahead of the unshifted
/// list either index was found against, and only on that path.
#[must_use]
pub(super) fn resolve_clip(
    marquee_clip: Option<(usize, f32, f32)>,
    ticker_clip: TickerClip,
    frozen_race: bool,
) -> Option<(usize, f32, f32)> {
    marquee_clip.or_else(|| {
        let (index, (left, right)) = ticker_clip?;
        let index = if frozen_race { index + 1 } else { index };
        Some((index, left, right))
    })
}
