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
//! **Chosen, not measured - no confidence score**: which medal glyph an
//! earned tier draws, which key the pad uses for the change-craft button, the
//! orange a refused craft turns Change craft (the original pulses
//! orange/blue), where the objective text starts (see
//! [`Frontend::draw_objective_page`]), the scale of the forced craft on the
//! objective page's glyph row, and the trophy heading's fit and the callout's
//! wrapped height on the trophy page. The panels are
//! `Transparent2048` and the glyphs `White2048`, both authored.
//!
//! # Pages
//!
//! The card has the pages `CampaignEventCard_BuildPageList` (`0x8105114a`)
//! builds, of which this build draws four: kind `0`, the pass objective
//! (`FUN_81055150`); kind `1`, the leaderboard (`FUN_810540c4`), as it draws
//! with no network - Personal tab selected, Friends and Global greyed; kind
//! `4`, the rules (`FUN_810535fe`): class, laps, a forced craft and the craft
//! classes the event allows, at the positions of the executable's own table.
//! The card opens on the rules page when the player's craft is refused.
//!
//! Page kind `2` (`FUN_81052fb4`) is the trophy and cup art - see the `trophy`
//! module - and the weapon callout, the elite row, the trophy glyph and the
//! forced craft's quads are drawn on pages `0` and `4`. **Not drawn, by name**:
//! kind `3` (`FUN_81052810`, a runtime field, probably unreachable) and the
//! personal record row of the leaderboard (this build keeps no per-event result
//! beyond the medal).

mod callout;
mod objective;
mod trophy;

pub use callout::CardWeapons;
pub use trophy::CardTrophy;

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
    /// The pass objective's worded line, `None` when the type is not one the
    /// original words.
    pub objective: Option<String>,
    /// `M_NUMOFLAPS`, when a lap race authors one above zero.
    pub laps: Option<u32>,
    /// The speed class glyph's texture name.
    pub class_icon: Option<String>,
    /// The photo's texture name, `None` when no circuit is authored.
    pub photo: Option<String>,
    /// The circuit emblem's texture name.
    pub emblem: Option<String>,
    /// The speed class's caption (`Speed_Class_C_0` and so on, resolved),
    /// under the class glyph on the rules page.
    pub class_label: Option<String>,
    /// `Callout_Lap`/`Callout_Laps` with the count filled in.
    pub lap_label: Option<String>,
    /// The craft the event forces (`M_PPLAYERSHIPMODELDATA`), if any.
    pub forced_craft: Option<CardCraft>,
    /// One glyph and caption per craft class the event still allows, when it
    /// restricts the choice at all (`FE_SHIP_*_ONLY`).
    pub allowed_classes: Vec<CardRestriction>,
    /// The leaderboard page's own words.
    pub tabs: CardTabs,
    /// The event's button shape gives it a trophy page (kind `2`). The page
    /// exists whether or not [`Self::trophy`] names art for it.
    pub has_trophy_page: bool,
    /// The trophy or cup the page draws, `None` when the original names none.
    pub trophy: Option<CardTrophy>,
    /// The weapon callout (`FUN_810626ce`): `None` when the event has no
    /// callout item at all, a Zone or Speed Lap event or one that offers every
    /// weapon.
    pub weapons: Option<CardWeapons>,
    /// `FE_ELITE_PASS`, the label of the elite row of the objective page.
    pub elite_label: Option<String>,
    /// The elite objective's worded line (`M_ELITEOBJECTIVE`).
    pub elite_objective: Option<String>,
}

/// A forced craft on the rules page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardCraft {
    /// The team logo's texture name (`Team_Logos/Icon_Team_*`).
    pub logo: Option<String>,
    /// The craft class's icon (`Icon_Ship_*`, the `_proto` one for a
    /// prototype), drawn beside the logo.
    pub type_icon: Option<String>,
    /// The team's name as the language table words it.
    pub caption: String,
}

/// One allowed craft class on the rules page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardRestriction {
    /// `Team_Logos/Icon_Ship_{Combat,Agility,Racer}_1col`.
    pub icon: String,
    /// `FE_SHIP_COMBAT_ONLY` and its siblings, resolved.
    pub label: String,
}

/// The words of page kind `1`, resolved once by the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardTabs {
    /// `FE_PERSONAL`, `FE_FRIENDS`, `FE_GLOBAL`.
    pub tabs: [String; 3],
    /// `FE_CURRENT_BEST`.
    pub current_best: String,
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
    /// Kind `1`: the leaderboard tabs (`FUN_810540c4`).
    Leaderboard,
    /// Kind `2`: the trophy or cup (`FUN_81052fb4`).
    Trophy,
    /// Kind `4`: the event's rules (`FUN_810535fe`).
    Rules,
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
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Combat_1col.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Agility_1col.gtf",
    r"Data\FE\NewImages\Team_Logos\Icon_Ship_Racer_1col.gtf",
    r"Data\FE\NewImages\callout\trophy.gtf",
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
const TROPHY_GLYPH: &str = CARD_TEXTURES[17];

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

    /// Puts the campaign map's cursor on the event called `name`. `false`
    /// when there is none. A capture-harness hook (`--until card:N:NAME`).
    pub fn select_campaign_event(&mut self, name: &str) -> bool {
        match self
            .campaign
            .events
            .iter()
            .position(|event| event.name == name)
        {
            Some(index) => {
                self.campaign.selected = index;
                true
            }
            None => false,
        }
    }

    /// Which page the card shows, `None` while it is closed.
    #[must_use]
    pub fn event_card_page(&self) -> Option<usize> {
        self.campaign.card.map(|card| card.page)
    }

    fn card_pages(event: &MapEvent) -> Vec<Page> {
        let mut pages = Vec::new();
        if event.card.has_objective {
            pages.push(Page::Objective);
        }
        pages.push(Page::Leaderboard);
        if event.card.has_trophy_page {
            pages.push(Page::Trophy);
        }
        if Self::rules_items(&event.card) > 0 {
            pages.push(Page::Rules);
        }
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
        // `CampaignEventCard_BuildPageList` opens the card on the rules page
        // when the player's current craft is refused (`DAT_816c782c`).
        let page = if self.card_launch_allowed() {
            0
        } else {
            Self::card_pages(event)
                .iter()
                .position(|page| *page == Page::Rules)
                .unwrap_or(0)
        };
        self.campaign.card = Some(CardState { page });
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

    fn left_text(text: &str, x: f32, y: f32, scale: f32, color: [f32; 4], wrap: f32) -> Draw {
        Draw::Text {
            x,
            y,
            scale,
            color,
            border: None,
            align: Align::Left,
            text: text.to_string(),
            wrap_width: Some(wrap),
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
        // `Transparent2048` (`0xc0ffffff`) is the vertex colour
        // `CampaignEventCard_DrawPanel` writes into both panel quads.
        let panel = self.global_colour("Transparent2048");
        out.push(Draw::Fill {
            rect: HEADER,
            color: panel,
        });
        out.push(Draw::Fill {
            rect: BODY,
            color: panel,
        });
        if let Some(emblem) = card
            .emblem
            .as_deref()
            .filter(|src| self.placed(src).is_some())
        {
            // One textured quad in `Blue2048` (`FUN_81060584`'s colour
            // argument): the texture is a white square with the glyph cut
            // out as alpha, so the tint makes the square navy.
            self.card_sprite(emblem, [432.0, 103.0, 62.0, 62.0], blue, out);
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
            out.push(Self::card_text(kind, CENTRE_X, 140.0, 0.54, blue, None));
        }
        self.card_sprite(
            event.kind.texture_name(),
            [870.0, 102.0, 64.0, 64.0],
            blue,
            out,
        );

        match pages.get(card_state.page) {
            Some(Page::Objective) => self.draw_objective_page(event, out),
            Some(Page::Leaderboard) => self.draw_leaderboard_page(event, out),
            Some(Page::Trophy) => self.draw_trophy_page(event, out),
            Some(Page::Rules) => self.draw_rules_page(event, out),
            None => {}
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
        self.card_sprite(icon, icon_rect, white, out);
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

/// `FUN_810535fe`'s placement table (`0x8151c9c8`): for `n` items, the
/// centres of item `0..n`, row `n - 1`. Copied as authored, including the
/// five-item row that leaves the lower left empty.
const RULES_LAYOUT: [[(f32, f32); 6]; 6] = [
    [
        (682.0, 277.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ],
    [
        (602.0, 277.0),
        (762.0, 277.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ],
    [
        (582.0, 234.0),
        (682.0, 340.0),
        (782.0, 234.0),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ],
    [
        (602.0, 234.0),
        (762.0, 234.0),
        (602.0, 340.0),
        (762.0, 340.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ],
    [
        (562.0, 234.0),
        (682.0, 234.0),
        (802.0, 234.0),
        (682.0, 340.0),
        (802.0, 340.0),
        (0.0, 0.0),
    ],
    [
        (562.0, 234.0),
        (682.0, 234.0),
        (802.0, 234.0),
        (562.0, 340.0),
        (682.0, 340.0),
        (802.0, 340.0),
    ],
];

/// Page kind `1`'s three tabs (`FUN_810540c4`): `x` of each, then the shared
/// `y`, width and height.
const TAB_X: [f32; 3] = [471.0, 613.0, 755.0];
const TAB_RECT: (f32, f32, f32) = (186.0, 138.0, 44.0);

impl Frontend {
    /// How many icons page kind `4` would draw for `card` - the count
    /// `CampaignEventCard_BuildPageList` gates the page on. The weapon
    /// callout (`FUN_810626ce`) is an item in the original and is not read
    /// here, so an event that only has weapons to say gets no rules page.
    fn rules_items(card: &EventCard) -> usize {
        usize::from(card.class_icon.is_some())
            + usize::from(card.lap_label.is_some())
            + usize::from(card.forced_craft.is_some())
            + usize::from(card.weapons.is_some())
            + card.allowed_classes.len()
    }

    /// Page kind `1`, as the original draws it with no network: the Personal
    /// tab selected (`FUN_81258f8e` is `0`, so `DAT_816c7840 = 1`), Friends
    /// and Global greyed out. The record row (the player's best result and
    /// XP, `FUN_8106ed02`) is not drawn: this build keeps no per-event
    /// result beyond the medal.
    pub(super) fn draw_leaderboard_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let orange = self.global_colour("Orange2048");
        let grey = self.global_colour("Grey2048");
        let white = self.global_colour("White2048");
        let dim = [white[0], white[1], white[2], 0.25];
        let tabs = &event.card.tabs;
        for (index, x) in TAB_X.into_iter().enumerate() {
            let (fill, ink) = match index {
                0 => (orange, white),
                _ => (grey, dim),
            };
            out.push(Draw::Fill {
                rect: [x, TAB_RECT.0, TAB_RECT.1, TAB_RECT.2],
                color: fill,
            });
            out.push(Self::card_text(
                &tabs.tabs[index],
                x + TAB_RECT.1 * 0.5,
                TAB_RECT.0 + 13.0,
                0.7,
                ink,
                None,
            ));
        }
        out.push(Draw::Fill {
            rect: [471.0, 234.0, 422.0, 37.0],
            color: grey,
        });
        out.push(Self::card_text(
            &tabs.current_best,
            CENTRE_X,
            243.0,
            0.7,
            white,
            None,
        ));
        if matches!(
            self.campaign.state_of(self.campaign.selected),
            ProgressState::Open | ProgressState::Locked
        ) {
            out.push(Self::card_text("--", CENTRE_X, 332.0, 1.0, blue, None));
        }
    }

    /// Page kind `4` (`FUN_810535fe`): what the event asks of the player,
    /// as icons with a caption each - class, laps, a forced craft, then the
    /// craft classes it still allows. The weapon callout is not drawn.
    pub(super) fn draw_rules_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let card = &event.card;
        let n = Self::rules_items(card).clamp(1, 6);
        let mut row = RULES_LAYOUT[n - 1];
        // `FUN_810535fe`: with exactly two items, one of them a callout wider
        // than 140 at the 44 unit measure, the pair stacks on the centre line.
        if n == 2
            && card
                .weapons
                .as_ref()
                .is_some_and(|w| callout::row_width(w.icons.len(), 44.0) > callout::STACK_ABOVE)
        {
            row[0] = (682.0, 234.0);
            row[1] = (682.0, 340.0);
        }
        let mut slot = 0;
        let mut place = || {
            let at = row[slot.min(5)];
            slot += 1;
            at
        };
        let caption = |text: &str, at: (f32, f32), out: &mut Vec<Draw>| {
            out.push(Self::card_text(
                text,
                at.0,
                at.1 + 38.0,
                0.6,
                blue,
                Some(140.0),
            ));
        };
        if let Some(class) = card.class_icon.as_deref() {
            let at = place();
            self.card_sprite(class, [at.0 - 32.0, at.1 - 32.0, 64.0, 64.0], blue, out);
            if let Some(label) = &card.class_label {
                caption(label, at, out);
            }
        }
        if let (Some(laps), Some(label)) = (card.laps, &card.lap_label) {
            let at = place();
            self.card_sprite(NUM_LAPS, [at.0 - 32.0, at.1 - 32.0, 64.0, 64.0], blue, out);
            out.push(Self::card_text(
                &laps.to_string(),
                at.0,
                at.1 - 10.0,
                0.6,
                blue,
                None,
            ));
            caption(label, at, out);
        }
        if let Some(weapons) = &card.weapons {
            let at = place();
            self.draw_callout_item(weapons, at, out);
        }
        if let Some(craft) = &card.forced_craft {
            let at = place();
            // `FUN_81061808(x, y - 32, ...)` draws two 64 unit quads, both
            // white: the team logo from `x`, and the class icon from `x - 40`
            // on top of its left edge (disassembled, 0x81061808). The caption
            // is centred 16 right of the item and reads `"%s %s"`: team, then
            // class label.
            let white = [1.0, 1.0, 1.0, 1.0];
            if let Some(logo) = craft.logo.as_deref() {
                self.card_sprite(logo, [at.0, at.1 - 32.0, 64.0, 64.0], white, out);
            }
            if let Some(icon) = craft.type_icon.as_deref() {
                self.card_sprite(icon, [at.0 - 40.0, at.1 - 32.0, 64.0, 64.0], white, out);
            }
            caption(&craft.caption, (at.0 + 16.0, at.1), out);
        }
        for allowed in &card.allowed_classes {
            let at = place();
            self.card_sprite(
                &allowed.icon,
                [at.0 - 32.0, at.1 - 32.0, 64.0, 64.0],
                blue,
                out,
            );
            caption(&allowed.label, at, out);
        }
    }
}
