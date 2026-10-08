//! The four pictures `Cell Selection` puts beside Event Type, Track, Speed
//! Class and Weapons.
//!
//! The XML authors four `<Image name="* Emblem">` with a 96 by 96 rect and no
//! `src` ([`oag_ui::screen::Slot`]), so the picture is chosen in code, and the
//! disc authors no table for it. What the disc does ship is a set of white
//! `*_bw.gtf` icons named exactly as the screen's own values are: one per
//! mode (`singlerace_bw`, `timetrial_bw`, `speedlap_bw`, `zone_bw`,
//! `tournament_bw`, `head2head_bw`, `eliminator_bw`, `detonator_bw`), one per
//! speed class (`venom_bw`, `flash_bw`, `rapier_bw`, `phantom_bw`), the
//! weapons pair (`weaponson_bw`, `weaponsoff_bw`) and one
//! `trackselectemblem_bw.gtf` in each circuit's `FE` folder. The settled frame
//! of `09_blitzed`'s first cell shows four of them, in `150,150,150` - the
//! single-race swirl, the circuit's ring, the one-star `VC` and the `X` - each
//! the picture of the file its name says.
//!
//! **The mapping is by file name, not read from a table**, confidence 70: four
//! of four icons on one frame agree, and the executable's own strings carry
//! the neighbouring `weapons_on_icon`/`VectorVenom` names but no `_bw` table.
//! A mode with no icon of that name (`NitroBattle`, `Custom Grid`, `AI Race`)
//! draws no picture rather than a neighbour's.

use oag_tables::race_campaign::{Cell, Mode};
use oag_ui::frontend::{Draw, Placed};
use oag_ui::screen::{Image, Slot};

use super::super::draw::image_draw;

/// Where the icons live on the disc.
const DIRECTORY: &str = r"Data\FE\Images";

/// The colour the frame draws them in: `150,150,150`, the headings' grey.
const TINT: u32 = 0xFF96_9696;

/// The barcode strip the screen authors under the hex field; on no sheet until
/// this names it, since nothing else draws it.
pub const BARCODE_SRC: &str = r"Data\FE\Images\flyerbarcode.gtf";

/// The mode icons' stems.
const MODE_ICONS: [&str; 8] = [
    "singlerace",
    "timetrial",
    "speedlap",
    "zone",
    "tournament",
    "head2head",
    "eliminator",
    "detonator",
];

/// The four speed classes' stems.
const CLASS_ICONS: [&str; 4] = ["venom", "flash", "rapier", "phantom"];

/// The weapons pair's stems.
const WEAPON_ICONS: [&str; 2] = ["weaponson", "weaponsoff"];

fn path(stem: &str) -> String {
    format!(r"{DIRECTORY}\{stem}_bw.gtf")
}

/// Every icon a sheet needs for this screen, as the archive path and the
/// `src` the draw asks for (the same string).
#[must_use]
pub fn sheet_sources() -> Vec<String> {
    MODE_ICONS
        .iter()
        .chain(&CLASS_ICONS)
        .chain(&WEAPON_ICONS)
        .map(|stem| path(stem))
        .collect()
}

/// `<location>\FE\TrackSelectEmblem_bw.gtf`, the circuit's white emblem.
#[must_use]
pub fn track_emblem_src(location: &str) -> String {
    format!(
        r"{}\FE\TrackSelectEmblem_bw.gtf",
        location.trim_end_matches('\\')
    )
}

/// The mode's icon stem.
fn mode_stem(mode: &Mode) -> Option<&'static str> {
    Some(match mode {
        Mode::Race => "singlerace",
        Mode::TimeTrial => "timetrial",
        Mode::SpeedLap => "speedlap",
        Mode::Zone => "zone",
        Mode::Tournament => "tournament",
        Mode::Head2Head => "head2head",
        Mode::Elimination => "eliminator",
        Mode::Other(name) if name.eq_ignore_ascii_case("Detonator") => "detonator",
        _ => return None,
    })
}

/// The `src` of the picture `slot` shows for `cell`; `None` for no picture.
fn source(
    slot: &str,
    cell: &Cell,
    track_emblem: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    match slot {
        "Event Emblem" => mode_stem(&cell.mode).map(path),
        "Track Emblem" => cell.track.as_deref().and_then(track_emblem),
        "Speed Class Emblem" => {
            let class = cell.class.to_ascii_lowercase();
            (cell.mode != Mode::Zone && CLASS_ICONS.contains(&class.as_str())).then(|| path(&class))
        }
        "Weapons Emblem" => Some(path(if cell.weapons {
            "weaponson"
        } else {
            "weaponsoff"
        })),
        _ => None,
    }
}

/// The rule across the page's card, `GridTopBar`: a rect with no picture and no
/// colour, which the frame draws `150,150,150` and solid (**measured on one
/// frame, confidence 70**).
pub(super) fn rule(slots: &[Slot]) -> Option<Draw> {
    let slot = slots
        .iter()
        .find(|slot| slot.name.as_deref() == Some("GridTopBar"))?;
    Some(Draw::Fill {
        rect: [slot.x, slot.y, slot.width?, slot.height?],
        color: oag_ui::screen::argb_to_rgba(TINT),
    })
}

/// Every emblem `cell` shows, drawn into each of `slots`' rects.
pub(super) fn draws(
    slots: &[Slot],
    cell: &Cell,
    track_emblem: &dyn Fn(&str) -> Option<String>,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Draw> {
    slots
        .iter()
        .filter_map(|slot| {
            let name = slot.name.as_deref()?;
            let src = source(name, cell, track_emblem)?;
            let placed = sprites(&src)?;
            let image = Image {
                name: slot.name.clone(),
                src,
                x: slot.x,
                y: slot.y,
                width: slot.width,
                height: slot.height,
                centred: false,
                color: TINT,
                u: None,
                v: None,
                texture_width: None,
                texture_height: None,
                auto_load: false,
                reveal: Vec::new(),
                transition: 0.0,
                start_enabled: true,
            };
            Some(image_draw(&image, placed))
        })
        .collect()
}

#[cfg(test)]
mod tests;
