//! What every [`Draw`] can do to itself: fade, zoom, hand out its colour -
//! plus [`Draw::title`], which builds one rather than transforming one, kept
//! here anyway because it is a second place the variant list has to stay in
//! step (see its own doc).
//!
//! Split out of `draw.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, a move with no behaviour change - and the
//! one place a new variant has to be added to move and fade with the page
//! it is on, which is the reason the three methods were kept together.

use super::Draw;

impl Draw {
    /// The colour this draw is modulated by, when it has one.
    ///
    /// `None` for [`Self::Video`] and [`Self::FuryBackdrop`] alone: a movie
    /// frame and the point-cloud pass have no alpha channel to fade and are
    /// never part of a page.
    ///
    /// **It exists so a new variant is one edit rather than three.** The menu's
    /// zoom and fade helpers each used to spell the variant list out, so adding
    /// [`Self::RotatedSprite`] meant remembering both - and forgetting one is a
    /// widget that silently does not fade with the page it is on.
    pub fn colour_mut(&mut self) -> Option<&mut [f32; 4]> {
        match self {
            Self::Text { color, .. }
            | Self::FacedText { color, .. }
            | Self::Fill { color, .. }
            | Self::ChamferedFill { color, .. }
            | Self::Sprite { color, .. }
            | Self::RotatedSprite { color, .. }
            | Self::TiledSprite { color, .. }
            | Self::BlendedSprite { color, .. } => Some(color),
            // The left edge's, which is the one a caller reading "the colour"
            // of a rule that fades in from the left would expect. Fading
            // both edges is [`Self::fade`]'s job, not this accessor's.
            Self::GradientFill { left, .. } => Some(left),
            Self::Video { .. } | Self::FuryBackdrop(_) | Self::SceneBackdrop(_) => None,
        }
    }

    /// Scales this draw about a point and multiplies its alpha.
    ///
    /// The whole of the menu's page-change effect - see
    /// [`crate::menu::Layers::zoomed`], which calls it once per body draw.
    /// **It lives here rather than beside its one caller** for the reason
    /// [`Self::colour_mut`] does: it spells the variant list out, so a new
    /// variant that is not added here silently does not move with the page it
    /// is on. Next to the type, that is one file to check.
    ///
    /// No variant needed adding for any of it: `Text`, `Fill` and `Sprite`
    /// already carry a position, a size or a scale, and a colour whose fourth
    /// channel is the alpha.
    pub fn zoom(&mut self, origin: (f32, f32), scale: f32, alpha: f32) {
        let about = |value: f32, from: f32| from + (value - from) * scale;
        // The chamfer before the match, so the two fill variants stay one arm.
        // It scales with its own quad: a fixed-size cut left on a shrinking
        // corner would eat more of the tab the smaller the tab got.
        if let Self::ChamferedFill { chamfer, .. } = self {
            chamfer[0] *= scale;
            chamfer[1] *= scale;
        }
        match self {
            Self::Text {
                x,
                y,
                scale: size,
                color,
                ..
            }
            | Self::FacedText {
                x,
                y,
                scale: size,
                color,
                ..
            } => {
                *x = about(*x, origin.0);
                *y = about(*y, origin.1);
                *size *= scale;
                color[3] *= alpha;
            }
            Self::Fill { rect, color }
            | Self::ChamferedFill { rect, color, .. }
            | Self::Sprite { rect, color, .. }
            | Self::RotatedSprite { rect, color, .. }
            | Self::TiledSprite { rect, color, .. }
            | Self::BlendedSprite { rect, color, .. } => {
                rect[0] = about(rect[0], origin.0);
                rect[1] = about(rect[1], origin.1);
                rect[2] *= scale;
                rect[3] *= scale;
                color[3] *= alpha;
            }
            Self::GradientFill { rect, left, right } => {
                rect[0] = about(rect[0], origin.0);
                rect[1] = about(rect[1], origin.1);
                rect[2] *= scale;
                rect[3] *= scale;
                left[3] *= alpha;
                right[3] *= alpha;
            }
            // A movie and the point-cloud pass have no alpha channel to fade
            // and are never part of a page.
            Self::Video { .. } | Self::FuryBackdrop(_) | Self::SceneBackdrop(_) => {}
        }
    }

    /// Multiplies this draw's alpha, leaving it where it is.
    ///
    /// Both edges of a [`Self::GradientFill`], which is the one variant
    /// [`Self::colour_mut`] cannot fade on its own.
    pub fn fade(&mut self, alpha: f32) {
        if let Self::GradientFill { left, right, .. } = self {
            left[3] *= alpha;
            right[3] *= alpha;
        } else if let Some(color) = self.colour_mut() {
            color[3] *= alpha;
        }
    }

    /// [`Self::FacedText`] when `role` is `Some`, the plain [`Self::Text`]
    /// this build has always emitted otherwise - the general form
    /// [`Self::title`] and `oag_ui::campaign::footer`'s own `Default`-role
    /// prompts are each one fixed shape of. Always `border: None`
    /// and `wrap_width: None`: every caller so far draws a single
    /// unbordered line, the same reason [`Self::title`] never took either.
    #[must_use]
    pub fn in_role(
        role: Option<&'static str>,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        align: crate::frontend::Align,
        text: String,
    ) -> Self {
        match role {
            None => Self::Text {
                x,
                y,
                scale,
                color,
                border: None,
                align,
                text,
                wrap_width: None,
            },
            Some(role) => Self::FacedText {
                role,
                x,
                y,
                scale,
                color,
                border: None,
                align,
                text,
                wrap_width: None,
            },
        }
    }

    /// A screen title: [`Self::in_role`] pinned to [`crate::frontend::Align::Left`].
    ///
    /// Shared by `crate::menu::draw_list` and `crate::picker::draw_list`,
    /// whose title pushes were one `Draw::Text` literal each before this
    /// variant existed and would otherwise now be two, one per role. Lives
    /// in this module rather than beside either caller for the reason this
    /// file exists at all: the variant list belongs with the type, not with
    /// whichever call site reaches for it first.
    #[must_use]
    pub fn title(
        role: Option<&'static str>,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        text: String,
    ) -> Self {
        Self::in_role(role, x, y, scale, color, crate::frontend::Align::Left, text)
    }
}
