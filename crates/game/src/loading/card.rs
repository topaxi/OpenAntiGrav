//! The launched event's card behind the race load's box, and what happens to it while the load
//! runs: a game tip in the photo's place, then the mode page and a slide to the middle.
//!
//! `docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "How the launched event's card reaches
//! the screen". The painter is the front end's own ([`oag_ui::frontend::CardPaint`]); this holds
//! the state the original keeps in statics: whether the tip or the mode page is up, and the slide.

use oag_title::loading::TipClass;
use oag_ui::font::Atlas;
use oag_ui::frontend::{Backdrop, Draw, EventIcon, GameTip, LaunchedCard};

use super::Screen;
use super::assets::GameTipSet;

/// The slide's step a draw, and how far it goes (`CampaignEventCard_DrawCellPanel`:
/// `x - (1 - (1 - s)^2) * 202`, `s += 0.08`).
const SLIDE_STEP: f32 = 0.08;
const SLIDE_DISTANCE: f32 = 202.0;

/// A card shown behind the race load.
#[derive(Debug)]
pub(super) struct CardBackdrop {
    card: LaunchedCard,
    tip: Option<GameTip>,
    mode_page: bool,
    slide: f32,
}

/// The class of a campaign event as the tip pick reads it.
fn tip_class(kind: EventIcon) -> TipClass {
    match kind {
        EventIcon::Race => TipClass::Race,
        EventIcon::SpeedLap => TipClass::SpeedLap,
        EventIcon::Zone => TipClass::Zone,
        EventIcon::Elimination => TipClass::Combat,
    }
}

impl CardBackdrop {
    pub(super) fn new(card: LaunchedCard, tips: Option<&GameTipSet>, draw: u64) -> Self {
        let tip = tips.and_then(|set| {
            let rows: Vec<u8> = set
                .tips
                .candidates(tip_class(card.event().kind))
                .into_iter()
                .filter(|row| set.rows.get(usize::from(*row)).is_some_and(Option::is_some))
                .collect();
            let pick = oag_core::rng::Rng::new(draw).next_u32() as usize;
            let row = *rows.get(pick % rows.len().max(1))?;
            set.rows.get(usize::from(row))?.clone()
        });
        Self {
            card,
            tip,
            mode_page: false,
            slide: 0.0,
        }
    }

    /// The race manager exists: the card gains its mode page and turns to it.
    pub(super) fn show_mode_page(&mut self) {
        self.mode_page = true;
    }

    /// One draw of the original's loading thread: the slide climbs once the tip and the photo
    /// are both off.
    pub(super) fn step(&mut self) {
        if self.mode_page {
            self.slide = (self.slide + SLIDE_STEP).min(1.0);
        }
    }

    /// Where the card is, as the offset the whole card is drawn at.
    pub(super) fn offset(&self) -> f32 {
        -(1.0 - (1.0 - self.slide) * (1.0 - self.slide)) * SLIDE_DISTANCE
    }

    pub(super) fn draws(&self, atlas: &Atlas, out: &mut Vec<Draw>) {
        let paint = self.card.paint(self.mode_page, Some(atlas));
        let backdrop = match (&self.tip, self.mode_page) {
            (_, true) => Backdrop::None,
            (Some(tip), false) => Backdrop::Tip(tip),
            (None, false) => Backdrop::Photo,
        };
        let from = out.len();
        paint.draw_panel(backdrop, out);
        let dx = self.offset();
        for draw in &mut out[from..] {
            draw.shift_x(dx);
        }
    }
}

impl Screen {
    /// Puts the launched event's card behind the screen's box. `draw` seeds the tip pick.
    pub fn set_card(&mut self, card: LaunchedCard, tips: Option<&GameTipSet>, draw: u64) {
        self.card = Some(CardBackdrop::new(card, tips, draw));
    }

    /// Whether a card is behind the box.
    #[must_use]
    pub fn has_card(&self) -> bool {
        self.card.is_some()
    }

    /// The game tip on the card, if it shows one.
    #[must_use]
    pub fn card_tip(&self) -> Option<&GameTip> {
        self.card.as_ref().and_then(|card| card.tip.as_ref())
    }

    /// Whether the card has turned to its mode page.
    #[must_use]
    pub fn card_on_mode_page(&self) -> bool {
        self.card.as_ref().is_some_and(|card| card.mode_page)
    }

    /// How far the card has slid, in grid units (zero or negative).
    #[must_use]
    pub fn card_offset(&self) -> f32 {
        self.card.as_ref().map_or(0.0, CardBackdrop::offset)
    }
}
