//! Wipeout 2048's per-event card: what a tap on a campaign-map node opens
//! before anything launches.
//!
//! The original's card is native code, not a `NEWGUI` screen
//! (`docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`); its
//! layout was recovered 2026-09-29 from `CampaignEventCard_Draw` (`0x810f1196`),
//! its shared panel painter `FUN_81055a16` and the page-list builder
//! `FUN_8105114a`, and cross-checked against the live Vita3K frame
//! `data/reference/2048-frontend/14-campaign-map-event-card-unity-square.png`.
//!
//! # What is authored, what is measured, what is not drawn
//!
//! **Authored** (off the disc): the circuit name, laps and class
//! (`SP.xml`), the pass objective's type and target (`SP.xml`) and every
//! string it is worded with (the language table), the photo
//! (`NewImages\trackscreens\<Track>.gxt`, chosen by the same
//! `M_TRACKNAME` -> file table `FUN_81060744` walks), the circuit emblem
//! (`NewImages\tracks\<track>.gxt`), and the three button glyphs.
//!
//! **Measured** (constants in this file): every rectangle - the photo at
//! `x=16`, the 404x334 body, the header and body panels at `x=420`, the
//! three 122x96 buttons at `x=562/692/822` and their 142x116 hit rects at
//! `x=552/682/812` (`CampaignEventCard_HandleInput`, `0x810f2164`), the page
//! arrows and the dot row - are literals in those functions, and the frame
//! agrees with them to the pixel.
//!
//! **Chosen, not measured - no confidence score**: the mode subtitle's
//! wording (the function that words it, `FUN_812b021a`, is unread), which
//! medal glyph an earned tier draws, which key the pad uses for the
//! change-craft button, and the two panels' opaque white (the frame reads
//! `254,254,254`; the original's alpha ramp is unread).
//!
//! **Not drawn, by name**: page kinds `1`-`4` (`FUN_810540c4`,
//! `FUN_81052fb4`, `FUN_81052810`, `FUN_810535fe`) - kind `1` is on every
//! single-player card (`FUN_8105114a`), so it is a page the player can reach
//! and it is blank; the elite-pass row (drawn only for two objective
//! kinds, `FUN_81055150`'s `param_3[0xb6]`, unread); the weapon and craft
//! class restriction icons (`FUN_81061db6`); the objective line of a
//! `BEAT_VALUE` event, whose text is either a per-mode override
//! (`vtable+0x6c` of the event, unread) or `FE_SCORE_POINTS`, which is not
//! in the disc's string table. The page count the original shows (three on
//! the reference frame) comes from predicates over fields this build does
//! not read, so this build shows the pages it can count: the objective page
//! when the event authors a pass objective, and kind `1`.

use crate::pointer::{Pointer, contains};

use super::campaign_map::ProgressState;
use super::*;
use oag_2048::frontend::states as w2048;

/// What the card says about one event, resolved to text by the caller so
/// this crate opens no file. Every field is `None`/empty when the disc
/// authors nothing for it - nothing here is a default wording.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventCard {
    /// The circuit's authored display name (`M_DISPLAYNAME`), e.g.
    /// `UNITY SQUARE`. Empty on a Zone event, which authors no circuit.
    pub title: String,
    /// The kind line under the title. **Chosen**: see this module's docs.
    pub kind_label: Option<String>,
    /// `FE_PASS` resolved, drawn above the objective when the event authors
    /// a pass objective.
    pub pass_label: Option<String>,
    /// The event authors an `M_PASSOBJECTIVE` - page kind `0` exists.
    pub has_objective: bool,
    /// The pass objective's worded line, `None` when its wording is not
    /// recovered (`BEAT_VALUE`).
    pub objective: Option<String>,
    /// `M_NUMOFLAPS`, when a lap race authors one above zero.
    pub laps: Option<u32>,
    /// The speed class glyph's texture name.
    pub class_icon: Option<String>,
    /// The photo's texture name, `None` when no circuit is authored.
    pub photo: Option<String>,
    /// The circuit emblem's texture name.
    pub emblem: Option<String>,
}

/// Where the card is: which page it shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CardState {
    /// Index into the event's pages.
    pub page: usize,
}

/// One page of the card, by the original's own kind number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    /// Kind `0`: the pass objective (`FUN_81055150`).
    Objective,
    /// Kind `1`: unread, drawn blank.
    Unread,
}

/// The card's own textures, spelled the way every front-end `Src=` is.
pub const CARD_TEXTURES: &[&str] = &[
    r"Data\FE\NewImages\callout\play.gtf",
    r"Data\FE\NewImages\callout\cross.gtf",
    r"Data\FE\NewImages\callout\num_laps.gtf",
    r"Data\FE\NewImages\Arrow.gtf",
    r"Data\FE\NewImages\Button_Indicator_Dot.gtf",
    r"Data\FE\NewImages\Icon_Team_HomeBut.gtf",
    r"Data\FE\NewImages\medals\Icon_no_pass_medal.gtf",
    r"Data\FE\NewImages\medals\Icon_pass_medal.gtf",
    r"Data\FE\NewImages\medals\Icon_elite_pass_medal.gtf",
    r"Data\FE\NewImages\speedclass\d_class.gtf",
    r"Data\FE\NewImages\speedclass\c_class.gtf",
    r"Data\FE\NewImages\speedclass\b_class.gtf",
    r"Data\FE\NewImages\speedclass\a_class.gtf",
    r"Data\FE\NewImages\speedclass\ap_class.gtf",
];

const PLAY: &str = CARD_TEXTURES[0];
const CROSS: &str = CARD_TEXTURES[1];
const NUM_LAPS: &str = CARD_TEXTURES[2];
const ARROW: &str = CARD_TEXTURES[3];
const DOT: &str = CARD_TEXTURES[4];
const SHIP: &str = CARD_TEXTURES[5];
const MEDAL_NONE: &str = CARD_TEXTURES[6];
const MEDAL_PASS: &str = CARD_TEXTURES[7];
const MEDAL_ELITE: &str = CARD_TEXTURES[8];

/// The photo body: `FUN_81060744` draws `x..x+404`, and the body is 334
/// tall in the frame (texture pixels `0..404` by `0..334` of a 512x512).
const PHOTO: [f32; 4] = [16.0, 92.0, 404.0, 334.0];
const HEADER: [f32; 4] = [420.0, 92.0, 524.0, 86.0];
const BODY: [f32; 4] = [420.0, 182.0, 524.0, 244.0];
/// Centre of the two panels.
const CENTRE_X: f32 = 682.0;
const BUTTON_SIZE: (f32, f32) = (122.0, 96.0);
const BUTTON_Y: f32 = 432.0;
/// The drawn `x` of Change craft, Back and Launch.
const BUTTON_X: [f32; 3] = [562.0, 692.0, 822.0];
/// Hit rects: `CampaignEventCard_HandleInput`'s literals, ten units out on
/// each side of the drawn rect.
const HIT_SIZE: (f32, f32) = (142.0, 116.0);
const HIT_Y: f32 = 422.0;
const HIT_X: [f32; 3] = [552.0, 682.0, 812.0];
/// `FUN_81055a16`'s two page-arrow tap regions: `x=420` and `x=682`, 262 wide.
const ARROW_TAP: [f32; 2] = [420.0, 682.0];
const ARROW_TAP_WIDTH: f32 = 262.0;

/// The three buttons, in the order `CampaignEventCard_HandleInput` tests
/// them, which is what settles the 12-unit overlap between Change craft's
/// hit rect and Back's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button3 {
    Back,
    Launch,
    ChangeCraft,
}

fn hit_rect(index: usize) -> [f32; 4] {
    [HIT_X[index], HIT_Y, HIT_SIZE.0, HIT_SIZE.1]
}

impl Frontend {
    /// Whether the card is up over the campaign map.
    #[must_use]
    pub fn event_card_open(&self) -> bool {
        self.campaign.card.is_some()
    }

    fn card_pages(event: &MapEvent) -> Vec<Page> {
        let mut pages = Vec::new();
        if event.card.has_objective {
            pages.push(Page::Objective);
        }
        pages.push(Page::Unread);
        pages
    }

    /// Opens the card for the selected event. A locked event refuses, as it
    /// always has.
    pub(super) fn open_event_card(&mut self) {
        let Some(event) = self.campaign.events.get(self.campaign.selected) else {
            return;
        };
        if matches!(
            self.campaign.state_of(self.campaign.selected),
            ProgressState::Locked
        ) {
            self.notes.push(format!(
                "{}: {:?} is locked, refusing to open its card",
                w2048::NEW_FE_SHELL,
                event.name
            ));
            return;
        }
        self.campaign.card = Some(CardState::default());
    }

    /// Whether the player's current craft may fly the selected event - the
    /// same check the Launch button dims on.
    fn card_launch_allowed(&self) -> bool {
        self.campaign
            .events
            .get(self.campaign.selected)
            .is_none_or(|event| self.craft_refused_for(event).is_none())
    }

    /// The card's buttons: Change craft only exists when the event forces
    /// no craft (`event+0x3c == 0`).
    fn card_has_change_craft(&self) -> bool {
        self.campaign
            .events
            .get(self.campaign.selected)
            .is_some_and(|event| event.forced_craft.is_none())
    }

    fn press_card_button(&mut self, button: Button3) {
        match button {
            Button3::Back => self.campaign.card = None,
            // Not gated on `card_launch_allowed`: `launch_selected_event`
            // re-checks the craft and says why it refused, and the original
            // eats the tap silently either way.
            Button3::Launch => {
                self.launch_selected_event();
                if self.touch.launch.is_some() {
                    self.campaign.card = None;
                }
            }
            Button3::ChangeCraft => {
                if self.card_has_change_craft() {
                    self.redirect_touch(w2048::NEW_FE_SHELL, "change craft", w2048::TEAM);
                }
            }
        }
    }

    fn step_card_page(&mut self, delta: isize) {
        let Some(event) = self.campaign.events.get(self.campaign.selected) else {
            return;
        };
        let count = Self::card_pages(event).len();
        if count < 2 {
            return;
        }
        if let Some(card) = self.campaign.card.as_mut() {
            card.page = (card.page as isize + delta).rem_euclid(count as isize) as usize;
        }
    }

    /// The pad on the card: cross launches, circle goes back, square changes
    /// craft (**chosen** - the original's card is touch only), left and right
    /// turn the page.
    pub(super) fn update_event_card(&mut self, input: &mut Input) {
        for (pad, action) in [
            (Button::Circle, Some(Button3::Back)),
            (Button::Cross, Some(Button3::Launch)),
            (Button::Start, Some(Button3::Launch)),
            (Button::Square, Some(Button3::ChangeCraft)),
            (Button::Left, None),
            (Button::Right, None),
        ] {
            if !input.is_pressed(pad) {
                continue;
            }
            input.consume_press(pad);
            match action {
                Some(button) => self.press_card_button(button),
                None => self.step_card_page(if pad == Button::Left { -1 } else { 1 }),
            }
            return;
        }
    }

    /// The pointer on the card: a tap in a button's hit rect presses it, a
    /// tap on either half of the body turns the page, the secondary button
    /// goes back.
    pub(super) fn event_card_pointer(&mut self, pointer: &Pointer) -> bool {
        if pointer.back {
            self.campaign.card = None;
            return true;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        if !pointer.clicked {
            return true;
        }
        for (button, index) in [
            (Button3::Back, 1),
            (Button3::Launch, 2),
            (Button3::ChangeCraft, 0),
        ] {
            if contains(hit_rect(index), at) {
                self.press_card_button(button);
                return true;
            }
        }
        let body = [BODY[0], BODY[1], BODY[2], BODY[3]];
        if contains(body, at) {
            let half = usize::from(at.0 >= ARROW_TAP[1]);
            debug_assert!(contains(
                [ARROW_TAP[half], BODY[1], ARROW_TAP_WIDTH, BODY[3]],
                at
            ));
            self.step_card_page(if half == 0 { -1 } else { 1 });
        }
        true
    }

    fn card_sprite(&self, src: &str, rect: [f32; 4], color: [f32; 4], out: &mut Vec<Draw>) {
        if let Some(placed) = self.placed(src) {
            out.push(Self::sprite_at(rect, placed, color));
        }
    }

    fn card_text(
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        wrap: Option<f32>,
    ) -> Draw {
        Draw::Text {
            x,
            y,
            scale,
            color,
            border: None,
            align: Align::Centre,
            text: text.to_string(),
            wrap_width: wrap,
        }
    }

    /// The card, over the map.
    pub(super) fn draw_event_card(&self, out: &mut Vec<Draw>) {
        let (Some(card_state), Some(event)) = (
            self.campaign.card,
            self.campaign.events.get(self.campaign.selected),
        ) else {
            return;
        };
        let blue = self.global_colour("Blue2048");
        let orange = self.global_colour("Orange2048");
        let white = [1.0, 1.0, 1.0, 1.0];
        let card = &event.card;
        let pages = Self::card_pages(event);

        if let Some(photo) = card.photo.as_deref().and_then(|src| self.placed(src)) {
            out.push(Draw::Sprite {
                rect: PHOTO,
                uv: [photo.x as f32, photo.y as f32, PHOTO[2], PHOTO[3]],
                color: white,
            });
        }
        out.push(Draw::Fill {
            rect: HEADER,
            color: white,
        });
        out.push(Draw::Fill {
            rect: BODY,
            color: white,
        });
        if let Some(emblem) = card
            .emblem
            .as_deref()
            .filter(|src| self.placed(src).is_some())
        {
            // The texture is a white glyph on alpha: the navy square behind it
            // is the panel's own.
            out.push(Draw::Fill {
                rect: [432.0, 103.0, 62.0, 62.0],
                color: blue,
            });
            self.card_sprite(emblem, [432.0, 103.0, 62.0, 62.0], white, out);
        }
        if !card.title.is_empty() {
            out.push(Self::card_text(
                &card.title,
                CENTRE_X,
                100.0,
                0.82,
                blue,
                None,
            ));
        }
        if let Some(kind) = &card.kind_label {
            out.push(Self::card_text(kind, CENTRE_X, 140.0, 0.6, blue, None));
        }
        self.card_sprite(
            event.kind.texture_name(),
            [870.0, 102.0, 64.0, 64.0],
            blue,
            out,
        );

        match pages.get(card_state.page) {
            Some(Page::Objective) => self.draw_objective_page(event, out),
            Some(Page::Unread) | None => {}
        }

        if pages.len() > 1 {
            self.draw_page_furniture(card_state.page, pages.len(), blue, orange, out);
        }

        let allowed = self.card_launch_allowed();
        let launch_tint = if allowed {
            blue
        } else {
            [blue[0], blue[1], blue[2], 0.25]
        };
        self.draw_card_button(BUTTON_X[2], PLAY, launch_tint, white, out);
        self.draw_card_button(BUTTON_X[1], CROSS, blue, white, out);
        if self.card_has_change_craft() {
            let tint = if allowed { blue } else { orange };
            self.draw_card_button(BUTTON_X[0], SHIP, tint, white, out);
        }
    }

    fn draw_card_button(
        &self,
        x: f32,
        icon: &str,
        tint: [f32; 4],
        white: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        out.push(Draw::Fill {
            rect: [x, BUTTON_Y, BUTTON_SIZE.0, BUTTON_SIZE.1],
            color: tint,
        });
        let icon_rect = [
            x + (BUTTON_SIZE.0 - 64.0) * 0.5,
            BUTTON_Y + (BUTTON_SIZE.1 - 64.0) * 0.5,
            64.0,
            64.0,
        ];
        let tinted = [white[0], white[1], white[2], tint[3]];
        self.card_sprite(icon, icon_rect, tinted, out);
    }

    fn draw_objective_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let white = [1.0, 1.0, 1.0, 1.0];
        let card = &event.card;
        let medal = match self.campaign.state_of(self.campaign.selected) {
            ProgressState::Passed => MEDAL_PASS,
            ProgressState::Elite => MEDAL_ELITE,
            ProgressState::Locked | ProgressState::Open => MEDAL_NONE,
        };
        self.card_sprite(medal, [488.0, 235.0, 52.0, 52.0], white, out);
        if let Some(label) = &card.pass_label {
            out.push(Self::card_text(label, 702.0, 229.0, 0.6, blue, Some(345.0)));
        }
        if let Some(text) = &card.objective {
            out.push(Self::card_text(text, 702.0, 256.0, 0.9, blue, Some(345.0)));
        }
        out.push(Draw::Fill {
            rect: [482.0, 329.0, 400.0, 1.0],
            color: blue,
        });
        let icons = usize::from(card.class_icon.is_some()) + usize::from(card.laps.is_some());
        let mut x = CENTRE_X + 28.0 - icons as f32 * 56.0 * 0.5;
        if let Some(class) = card.class_icon.as_deref() {
            self.card_sprite(class, [x - 22.4, 347.6, 44.8, 44.8], blue, out);
            x += 56.0;
        }
        if let Some(laps) = card.laps {
            self.card_sprite(NUM_LAPS, [x - 22.4, 347.6, 44.8, 44.8], blue, out);
            out.push(Self::card_text(
                &laps.to_string(),
                x,
                358.0,
                0.6,
                blue,
                None,
            ));
        }
    }

    fn draw_page_furniture(
        &self,
        page: usize,
        count: usize,
        blue: [f32; 4],
        orange: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        if let Some(arrow) = self.placed(ARROW) {
            out.push(Self::sprite_at([429.0, 266.0, 32.0, 32.0], arrow, blue));
            out.push(Draw::RotatedSprite {
                rect: [904.0, 266.0, 32.0, 32.0],
                uv: [
                    arrow.x as f32,
                    arrow.y as f32,
                    arrow.width as f32,
                    arrow.height as f32,
                ],
                color: blue,
                rotation: std::f32::consts::PI,
            });
        }
        for at in 0..count {
            let x = CENTRE_X - (count - 1) as f32 * 9.0 + at as f32 * 18.0;
            let color = if at == page { orange } else { blue };
            self.card_sprite(DOT, [x - 8.0, 405.0, 16.0, 16.0], color, out);
        }
    }
}
