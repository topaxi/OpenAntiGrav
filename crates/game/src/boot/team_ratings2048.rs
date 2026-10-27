//! The Select Team panel's numbers: `<FE>` of every native craft's
//! `PI_TeamModel` in `Data\Plugins\teams\Definition.xml`.
//!
//! The panel reads the ship record the plugin builds (`0x8113a3b0` hands
//! `+0x1c4`, `+0x1cc`, `+0x1d0` and `+0x1d8` of it to `TeamInfo`; the record's
//! `+0x1b0` is the `modellocation` and `+0x1ac` the `texturelocation`, which is
//! how it is known to be a `PI_TeamModel`). `<FE speed thrust handling shield
//! length firepower description>` puts those at `+0x1c4`, `+0x1c8`, `+0x1cc`,
//! `+0x1d0`, `+0x1d4` and `+0x1d8`. **Not** `HandlingStats/<team>/<n>`'s own `<FE>`,
//! which agrees on Feisar Speed but not everywhere (Feisar Fighter: speed 5 here,
//! 3 there). Read off the disc, never transcribed: a craft whose `<FE>` is missing
//! gets no rows and a report line, never a zero bar.

use oag_2048::race::{NATIVE_TEAMS, SHIP_TYPES};
use oag_tables::fexml::{self, Node};
use oag_ui::frontend::TeamRating;

/// The plugin's `PI_TeamModel` names, in `SHIP_TYPES`' order (the roster's word for
/// `fighter` is `combat`).
const MODELS: [&str; 4] = ["combat", "agility", "speed", "prototype"];

/// Every native craft's rating, in `NATIVE_TEAMS` then craft order. Empty off 2048
/// (`touch` is false there), whose roster has no such teams.
pub fn load(touch: bool, documents: &[String], report: &mut Vec<String>) -> Vec<TeamRating> {
    if !touch {
        return Vec::new();
    }
    let roots: Vec<Node> = documents.iter().map(|xml| fexml::parse(xml)).collect();
    let mut ratings = Vec::new();
    for team in NATIVE_TEAMS {
        for (craft, model) in MODELS.iter().enumerate() {
            match read(&roots, team, model) {
                Some(rows) => ratings.push(TeamRating { team, craft, rows }),
                None => report.push(format!(
                    "team panel: {team} {} has no <FE>",
                    SHIP_TYPES[craft]
                )),
            }
        }
    }
    report.push(format!(
        "team panel: {} of {} craft ratings read",
        ratings.len(),
        NATIVE_TEAMS.len() * MODELS.len()
    ));
    ratings
}

/// `speed`, `handling`, `shield`, `firepower` of one `PI_TeamModel`'s `<FE>`.
fn read(roots: &[Node], team: &str, model: &str) -> Option<[f32; 4]> {
    let team = roots.iter().find_map(|root| named(root, "PI_Team", team))?;
    let model = team.children.iter().find(|child| {
        child.name == "PI_TeamModel"
            && child
                .attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case(model))
    })?;
    let fe = model.children.iter().find(|child| child.name == "FE")?;
    let number = |attribute: &str| fe.attr(attribute)?.trim().parse::<f32>().ok();
    Some([
        number("speed")?,
        number("handling")?,
        number("shield")?,
        number("firepower")?,
    ])
}

/// The first descendant of `node` (or `node`) called `element` whose `name`
/// attribute is `name`.
fn named<'a>(node: &'a Node, element: &str, name: &str) -> Option<&'a Node> {
    if node.name == element
        && node
            .attr("name")
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| named(child, element, name))
}
