//! The 3-D flyer HD/Fury's campaign screens draw beside their widgets: what
//! the disc authors about it, and the archive entries it is composed from.
//!
//! **A flyer is a flat promotional card**, not a craft - a 102.4 by 66.6 unit
//! quad carrying a wordmark, a silhouette, sponsor marks and a stripe, with a
//! reflection under it. Every campaign grid has one, `Data\FE\Flyers\<name>\`,
//! and the two campaigns have a pair more (`Fury_Campaign`, `HD_Campaign`).
//! See `docs/ui/campaign-screens.md`'s "The flyer behind `Grid Selection`".
//!
//! # What the disc authors, and where
//!
//! - **The widget**: `<Flyer name="FlyerModel">` (`Grid Selection`) and
//!   `FuryCampaignFlyerModel`/`HDCampaignFlyerModel` (`Campaign Selection`)
//!   sit at the **top level** of `CellMode_Definition.xml`, siblings of the
//!   named `<Screen>`s rather than children of them, so
//!   [`crate::screen::Screens::collect`] never attributes them to one. [`read`]
//!   walks the raw tree for them, the way [`crate::campaign::hd`]'s title is
//!   read.
//! - **Which model**: the widget's own `Src` is `Data\FE\Flyers\00_flyer.vex`
//!   on all three, a 1.6 KB placeholder (one `cardShape` quad, a
//!   `card_reflectShape`, a dummy texture). The runtime replaces it: the
//!   executable carries the format strings `Data/FE/Flyers/%s/`,
//!   `%sflyer.vex`, `%sflyer_Back.vex` and `Data\FE\Flyers\%s\Logo.gtf`, and
//!   every grid's `<Values FlyerName="01_uplift">` (see
//!   `oag_tables::race_campaign::Grid::flyer_name`) is the `%s`. Read off
//!   `strings EBOOT.elf`, confidence 85 - the call site that formats them is
//!   unread.
//!
//! # What it does not author
//!
//! The projection (field of view, how `OriginX`/`OriginY`, `x y z`, `RotY`,
//! `RotationCentreOffsetX` and `orthoScale` combine) and the settled pose of
//! `Grid Selection`'s card are in the native `Flyer` widget class, unread.
//! `RotY="1.5"` read as radians is a card turned 86 degrees, which no
//! settled RPCS3 frame shows, so the game-side camera
//! (`oag_game::preview::flyer_view_projection`) is **chosen, not measured**.

use crate::screen::{Screens, parse};

/// `Grid Selection`'s `<Flyer>` widget.
pub const GRID_WIDGET: &str = "FlyerModel";

/// `Campaign Selection`'s two `<Flyer>` widgets, Fury's on the left.
pub const FURY_CAMPAIGN_WIDGET: &str = "FuryCampaignFlyerModel";
pub const HD_CAMPAIGN_WIDGET: &str = "HDCampaignFlyerModel";

/// The two campaign cards' `FlyerName`s: the folders
/// `Data/FE/Flyers/fury_campaign/` and `Data/FE/Flyers/hd_campaign/`, which no
/// grid names.
pub const FURY_CAMPAIGN_FLYER: &str = "fury_campaign";
pub const HD_CAMPAIGN_FLYER: &str = "hd_campaign";

/// One `<Flyer>` widget's own values. Nothing here is a pixel: `x y z` and
/// the rotations are the model's pose, `origin` the screen point the camera is
/// centred on, in the screen's own authored grid.
#[derive(Debug, Clone, PartialEq)]
pub struct FlyerWidget {
    /// The widget's `name` - `FlyerModel`, `FuryCampaignFlyerModel`.
    pub name: String,
    /// `Src` - always the `00_flyer.vex` placeholder. Kept as authored.
    pub src: String,
    /// `OriginX`/`OriginY`.
    pub origin: [f32; 2],
    /// `nearZ`/`farZ`.
    pub depth: [f32; 2],
    /// `x`/`y`/`z`.
    pub position: [f32; 3],
    /// `RotX`/`RotY`.
    pub rotation: [f32; 2],
    /// `RotationCentreOffsetX`: where along X the rotation pivots, from the
    /// model's origin.
    pub rotation_centre_offset_x: f32,
    /// `orthoScale`, authored on the two `Campaign Selection` flyers and not
    /// on `Grid Selection`'s.
    pub ortho_scale: Option<f32>,
}

/// Every `<Flyer>` in `xml`, in document order.
///
/// `RotY="0.6f"` carries a trailing `f` on the `Campaign Selection` pair (a
/// C float literal copied into XML), which the number reader would refuse, so
/// it is stripped here.
#[must_use]
pub fn read(xml: &str, screens: &Screens) -> Vec<FlyerWidget> {
    let root = parse(xml);
    let mut out = Vec::new();
    let mut stack = vec![&root];
    while let Some(node) = stack.pop() {
        if node.name.eq_ignore_ascii_case("Flyer") {
            let number = |key: &str| {
                node.value(key)
                    .map(|value| value.trim().trim_end_matches(['f', 'F']))
                    .and_then(|value| screens.number(Some(value)))
            };
            out.push(FlyerWidget {
                name: node.attr("name").unwrap_or_default().to_string(),
                src: node.value("Src").unwrap_or_default().to_string(),
                origin: [
                    number("OriginX").unwrap_or(0.0),
                    number("OriginY").unwrap_or(0.0),
                ],
                depth: [
                    number("nearZ").unwrap_or(1.0),
                    number("farZ").unwrap_or(1000.0),
                ],
                position: [
                    number("x").unwrap_or(0.0),
                    number("y").unwrap_or(0.0),
                    number("z").unwrap_or(0.0),
                ],
                rotation: [number("RotX").unwrap_or(0.0), number("RotY").unwrap_or(0.0)],
                rotation_centre_offset_x: number("RotationCentreOffsetX").unwrap_or(0.0),
                ortho_scale: number("orthoScale"),
            });
            continue;
        }
        // Document order: pushed reversed so the first child pops first.
        stack.extend(node.children.iter().rev());
    }
    out
}

/// `Data/FE/Flyers/<name>/` - the executable's own `%s` folder template.
#[must_use]
pub fn folder(flyer_name: &str) -> String {
    format!("Data/FE/Flyers/{flyer_name}/")
}

/// The card's front: the executable's `%sflyer.vex` over [`folder`].
#[must_use]
pub fn front_entry(flyer_name: &str) -> String {
    format!("{}flyer.vex", folder(flyer_name))
}

/// The card's back: the executable's `%sflyer_Back.vex` over [`folder`].
#[must_use]
pub fn back_entry(flyer_name: &str) -> String {
    format!("{}flyer_Back.vex", folder(flyer_name))
}

/// The grid's logo: the executable's `Data\FE\Flyers\%s\Logo.gtf`, which is
/// also the spelling the screen's own `flyerlogo` widget carries (with
/// `01_uplift` as the `%s`).
#[must_use]
pub fn logo_entry(flyer_name: &str) -> String {
    format!(r"Data\FE\Flyers\{flyer_name}\Logo.gtf")
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<Screen name="Campaign Selection" type="CampaignSelection">
<Flyer name="FuryCampaignFlyerModel" DisableTransition="0">
<Values OriginX="640" OriginY="450" nearZ="1.0" farZ="1000.0" Src="Data\FE\Flyers\00_flyer.vex" x="0.0" y="-33.3" z="-70.0" RotX="0.0" RotY="0.6f" RotationCentreOffsetX="-30.0" orthoScale="0.75"></Values>
</Flyer>
</Screen>
<Flyer name="FlyerModel" DisableTransition="0">
<Values OriginX="960" OriginY="540" nearZ="1.0" farZ="1000.0" Src="Data\FE\Flyers\00_flyer.vex" x="80.0" y="-33.3" z="-200.0" RotX="0.0" RotY="1.5" RotationCentreOffsetX="-60.0"></Values>
</Flyer>"#;

    /// Both shapes: nested in a screen with a `0.6f` literal and an
    /// `orthoScale`, and top-level without one.
    #[test]
    fn reads_nested_and_top_level_flyers() {
        let screens = Screens::from_xml(XML);
        let flyers = read(XML, &screens);
        assert_eq!(flyers.len(), 2);
        assert_eq!(flyers[0].name, "FuryCampaignFlyerModel");
        assert_eq!(flyers[0].rotation, [0.0, 0.6]);
        assert_eq!(flyers[0].ortho_scale, Some(0.75));
        assert_eq!(flyers[1].name, "FlyerModel");
        assert_eq!(flyers[1].position, [80.0, -33.3, -200.0]);
        assert_eq!(flyers[1].origin, [960.0, 540.0]);
        assert_eq!(flyers[1].rotation_centre_offset_x, -60.0);
        assert_eq!(flyers[1].ortho_scale, None);
    }

    #[test]
    fn entries_follow_the_executables_format_strings() {
        assert_eq!(
            front_entry("01_uplift"),
            "Data/FE/Flyers/01_uplift/flyer.vex"
        );
        assert_eq!(
            back_entry("01_uplift"),
            "Data/FE/Flyers/01_uplift/flyer_Back.vex"
        );
        assert_eq!(
            logo_entry("02_warped"),
            r"Data\FE\Flyers\02_warped\Logo.gtf"
        );
    }
}
