//! Wipeout HD/Fury's `EndRace Podium`: three racers' names on a podium, drawn
//! off the screen's own authored widgets plus the three positions its
//! constructor computes.
//!
//! **Reached by `--menu-page endrace-podium` only.** Nothing in any archive's
//! XML names this screen (no `goto=` in all seven), and the executable's one
//! caller of its slot setter (`0x00220820`, which fills the three slots off
//! the race manager's records) is entered from a path this project has not
//! traced - see `docs/ghidra/functions/ps3-hdfury-eu/endrace-podium.md`. The
//! live flow stays Results -> Menu until that gate is read.
//!
//! What is measured (the slot setter `0x00220108`, read 2026-10-06):
//!
//! - A slot's column is `slot * 0x168` (360) apart on the 1920 grid, and the
//!   place picks the slot: first place is slot 2, second slot 1, third slot 3
//!   (the middle column is the winner). Slot 2 lands on the authored
//!   `pod_head.2` `x="795"`, which is what pins the 360 step.
//! - The name sits at `x = 70 + 360 * slot`, `y = 532 - 80 * (4 - place)`.
//!
//! What is **chosen, not measured**: the heading's `y` (`name y - 62`, which
//! puts first place's heading on its authored `y="230"`), the heading text
//! (`IG_HUD_1ST`/`2ND`/`3RD`, all present in HD's English table; the three
//! authored `pod_head` idstrings are all `IG_HUD_1ST` and the code is what
//! overwrites them, its string pointers unread), and the local player's
//! ink (`HD_Blue`, see [`PLAYER_INK`]), and the screen's own title: its
//! `FE_ENDRACE_PODIUM` is in `DATA05`/`DATA06`'s English table and not in
//! `DATA02`'s, the one this build's string table is read from, so the title
//! draws nothing (an honest absence) until the table is read per copy.
//!
//! Never drawn: the ship portraits (`pod_img.N`, a `miniBW.tga` the code
//! picks per team), the dots plinth images, and the eight `b_b.N` badge
//! panels (an online achievement system with no state here).
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
    /// The pilot's display name.
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
const NAME_X: f32 = 70.0;
const NAME_Y_FIRST: f32 = 532.0 - 80.0 * 3.0;
const PLACE_STEP: f32 = 80.0;
const HEAD_ABOVE_NAME: f32 = 62.0;
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
    let heading = screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("pod_head.1"));
    let name_text = screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("pod_text.1"));
    for (index, entry) in model.places.iter().enumerate() {
        let Some(entry) = entry else { continue };
        let place = index + 1;
        let slot = slot_of(place);
        let name_y = NAME_Y_FIRST + PLACE_STEP * (place - 1) as f32;
        if let Some(heading) = heading {
            let mut head = text_draw(heading, strings.get_or_id(place_idstring(place)), layout);
            if let Draw::Text { x, y, .. } = &mut head {
                *x = heading.x + COLUMN_STEP * (slot as f32 - 2.0);
                *y = name_y - HEAD_ABOVE_NAME;
            }
            out.push(head);
        }
        if let Some(authored) = name_text {
            let mut name = text_draw(authored, &entry.name, layout);
            if let Draw::Text { x, y, color, .. } = &mut name {
                *x = NAME_X + COLUMN_STEP * slot as f32;
                *y = name_y;
                if entry.player {
                    *color = argb_to_rgba(PLAYER_INK);
                }
            }
            out.push(name);
        }
    }
    layers.body = out;
    layers
}
