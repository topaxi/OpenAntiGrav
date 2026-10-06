//! Wipeout HD/Fury's `EndRace Podium`: three racers' names on a podium, drawn
//! off the screen's own authored widgets plus the three positions its
//! constructor computes.
//!
//! **Reached by `--menu-page endrace-podium` only, and rightly.** Nothing in
//! any archive's XML names this screen, but the code does: the multiplayer
//! race managers' end-of-race sequencer (`0x000459d8`, a member of the
//! `MPRaceManager` family - `ps3-hdfury-eu/race-manager.md`) enters it at
//! `0x00045e4c` once its end deadline has passed, holds it 16 seconds and
//! leaves for `Kill Game Transition`; mid-series it goes to `Load Next Race`
//! instead. This project has no multiplayer, so the live single-player flow
//! stays Results -> Menu. See
//! `docs/ghidra/functions/ps3-hdfury-eu/endrace-podium.md`.
//!
//! What is measured (the slot setter `0x00220108`, read 2026-10-06; every
//! `0x2f..` TOC slot resolved with `scripts/ps3-toc.py`):
//!
//! - A slot's column is `slot * 0x168` (360) apart on the 1920 grid, and the
//!   place picks the slot: first place is slot 2, second slot 1, third slot 3
//!   (the middle column is the winner).
//! - The name (`pod_text`) sits at `x = 70 + 360 slot`,
//!   `y = 532 - 80 (4 - place)`; the heading (`pod_head`) at
//!   `x = 120 + 360 slot`, `y = 485 - 80 (4 - place)`, with the idstring
//!   `IG_HUD_1ST`/`2ND`/`3RD` by place.
//! - The plinth (`pod_dots`, `dot.gtf`) at `x = 60 + 360 slot`,
//!   `y = 565 - 80 (4 - place)`, 352 wide and `80 (4 - place)` tall, so every
//!   plinth ends on `y = 565` and the winner's is tallest; its sampled
//!   rectangle takes the same two numbers.
//! - The local player's name and plinth are `HD_Blue`, anyone else's
//!   `0xff646464`.
//!
//! **Chosen, not measured**: the local player's `HD_Blue` value
//! ([`PLAYER_INK`]: `DATA06`'s, the code reads the archive's own, which this
//! layout does not resolve), and the screen's title - `FE_ENDRACE_PODIUM` is
//! in `DATA05`/`DATA06`'s English table and not in `DATA02`'s, the one this
//! build's string table is read from, so the title draws nothing (an honest
//! absence) until the table is read per copy.
//!
//! Never drawn: the ship portraits (`pod_img.N`, a texture the code picks per
//! record) and the eight `b_b.N` badge panels (a per-player badge list with no
//! state here).
//!
//! No pointer targets: the screen authors no `NavigationController` and no
//! redirect, so there is nothing to select or confirm.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::argb_to_rgba;

use crate::endrace::Layout;
use crate::endrace::draw::{fill_draw, image_draw, text_draw};

/// One racer on the podium.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodiumSlot {
    /// The pilot's display name. The original prints the race record's string
    /// at `+0x50`, the same field `EndRace Results`' `Grid1.r` shows and this
    /// project has not identified, so what a live race would put here is
    /// open.
    pub name: String,
    /// Whether this is the local player - drawn in a different ink.
    pub player: bool,
}

/// The top three of a race, first place first. `None` leaves that column
/// empty, as the original does for a race with fewer than three craft.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HdPodium {
    pub places: [Option<PodiumSlot>; 3],
}

const COLUMN_STEP: f32 = 360.0;
const PLACE_STEP: f32 = 80.0;
const NAME_X: f32 = 70.0;
const NAME_Y_BOTTOM: f32 = 532.0;
const HEAD_X: f32 = 120.0;
const HEAD_Y_BOTTOM: f32 = 485.0;
const PLINTH_X: f32 = 60.0;
const PLINTH_BOTTOM: f32 = 565.0;
const PLINTH_WIDTH: f32 = 352.0;
/// The ink `pod_dots` and `pod_text` carry for anyone but the local player.
const OTHER_INK: u32 = 0xff64_6464;
/// `HD_Blue` as `DATA06` spells it (`0xff8ac0ca`, the literal `Block_Construct`
/// also compiles in). The code colours the local player's name with the
/// archive's own `FEGlobals->HD_Blue`, which differs per archive (a red on
/// Fury's `DATA00`) and which this layout does not resolve: **chosen** from
/// the one archive that authors the screen.
const PLAYER_INK: u32 = 0xff8a_c0ca;

/// The slot (column) a 1-based place stands in: 2nd on the left, 1st in the
/// middle, 3rd on the right.
fn slot_of(place: usize) -> usize {
    match place {
        1 => 2,
        2 => 1,
        _ => 3,
    }
}

fn place_idstring(place: usize) -> &'static str {
    match place {
        1 => "IG_HUD_1ST",
        2 => "IG_HUD_2ND",
        _ => "IG_HUD_3RD",
    }
}

/// `EndRace Podium`'s draw list. See the module doc for what is measured and
/// what is chosen.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts hd_rewards_draw_list takes"
)]
pub fn hd_podium_draw_list(
    model: &HdPodium,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        if fill.name.is_none() {
            out.push(fill_draw(fill));
        }
    }
    for image in &screen.images {
        if image.name.as_deref().is_some_and(|n| n.starts_with("pod_")) {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref();
        if name.is_some_and(|n| n.starts_with("pod_") || n.starts_with("b_t")) {
            continue;
        }
        if let (None, Some(string)) = (name, &text.string) {
            out.push(text_draw(text, string, layout));
        }
    }
    let find = |name: &str| {
        screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
    };
    let (heading, name_text) = (find("pod_head.1"), find("pod_text.1"));
    let plinth = screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("pod_dots.1"));
    for (index, entry) in model.places.iter().enumerate() {
        let Some(entry) = entry else { continue };
        let place = index + 1;
        let slot = slot_of(place) as f32;
        let rise = PLACE_STEP * (4 - place) as f32;
        let ink = if entry.player { PLAYER_INK } else { OTHER_INK };
        if let Some(authored) = plinth {
            if let Some(placed) = sprites(&authored.src) {
                let mut drawn = authored.clone();
                drawn.x = PLINTH_X + COLUMN_STEP * slot;
                drawn.y = PLINTH_BOTTOM - rise;
                drawn.width = Some(PLINTH_WIDTH);
                drawn.height = Some(rise);
                drawn.texture_width = Some(PLINTH_WIDTH);
                drawn.texture_height = Some(rise);
                drawn.color = ink;
                out.push(image_draw(&drawn, placed));
            }
        }
        if let Some(authored) = heading {
            let mut head = text_draw(authored, strings.get_or_id(place_idstring(place)), layout);
            if let Draw::Text { x, y, .. } = &mut head {
                *x = HEAD_X + COLUMN_STEP * slot;
                *y = HEAD_Y_BOTTOM - rise;
            }
            out.push(head);
        }
        if let Some(authored) = name_text {
            let mut name = text_draw(authored, &entry.name, layout);
            if let Draw::Text { x, y, color, .. } = &mut name {
                *x = NAME_X + COLUMN_STEP * slot;
                *y = NAME_Y_BOTTOM - rise;
                *color = argb_to_rgba(ink);
            }
            out.push(name);
        }
    }
    layers.body = out;
    layers
}
