//! Wipeout 2048's downloadable campaigns, as the boot hands them to the front
//! end and the session launches them.
//!
//! The HD and Fury packages (`dlc1.psarc`, `dlc2.psarc`) open two campaigns
//! whose cells are Wipeout HD's own `PI_Cell`s: the sixteen
//! `Data\Plugins\grids\grid_NN.xml` the base package and `data2.psarc` ship,
//! `grid_00` to `grid_07` tagged `Campaign="HD"` and `grid_08` to `grid_15`
//! `"Fury"`. A circuit those cells race that a package supplies is why the
//! package is the gate: every circuit the `HD` grids race is in `dlc1.psarc`
//! or the base package, every one the `Fury` grids race is in `dlc2.psarc`
//! (`docs/formats/2048-dlc.md`).
//!
//! **What this reads and what it chooses.** Read off the disc: the cells, their
//! circuits, modes, classes, laps, weapons flags, and where each node sits
//! relative to the others (`oag_2048::campaign::tier`). Chosen, not measured, and
//! each said where it is made: the icon a mode draws, that a cell this build
//! cannot race (HD's Detonator and Nitro Battle) is drawn closed, and that the
//! campaign flies the HD roster's own default craft until the player picks
//! another HD one.

use oag_raceplay::catalogue::Track;
use oag_tables::race_campaign::{Cell, Grid, Medal, Mode};
use oag_ui::frontend::{EventCard, EventIcon, MapEvent, Tier};
use oag_ui::language::{CircuitNames, StringTable};

use super::campaign2048::{card_tabs, class_icon, class_label, class_ordinal, weapons_off};

/// One mounted package's campaign: the package and the grids that carry its
/// `Campaign=` tag, in file order.
#[derive(Debug, Clone)]
pub struct TierGrids {
    pub add_on: &'static oag_title::AddOn,
    pub grids: Vec<Grid>,
}

/// The campaigns of the packages `archives` mounted, each with its grids. A
/// package whose grids will not read is reported and has no campaign.
pub fn read(archives: &mut oag_assets::Archives, report: &mut Vec<String>) -> Vec<TierGrids> {
    let mounted = archives.add_ons.clone();
    mounted
        .into_iter()
        .filter_map(
            |add_on| match crate::campaign::read_grids(archives, add_on.grids, None) {
                Ok(grids) => {
                    let grids: Vec<Grid> = grids
                        .into_iter()
                        .filter(|grid| grid.campaign.as_deref() == Some(add_on.campaign))
                        .collect();
                    report.push(format!(
                        "{}: {} {} grid(s), {} cell(s), opened by {}",
                        add_on.grids,
                        grids.len(),
                        add_on.campaign,
                        grids.iter().map(|grid| grid.cells.len()).sum::<usize>(),
                        add_on.archive
                    ));
                    Some(TierGrids { add_on, grids })
                }
                Err(error) => {
                    report.push(format!(
                        "{}: {error:#} - the {} campaign has no cells",
                        add_on.grids, add_on.campaign
                    ));
                    None
                }
            },
        )
        .collect()
}

/// The map of every mounted package's campaign, read off `archives`: what the
/// front end is given at boot. Empty where no package is mounted.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    tracks: &[Track],
    names: &CircuitNames,
    strings: &StringTable,
    report: &mut Vec<String>,
) -> Vec<Tier> {
    tiers(&read(archives, report), tracks, names, strings)
}

/// The cell called `name`, with the campaign and grid it is in.
#[must_use]
pub fn find<'a>(tiers: &'a [TierGrids], name: &str) -> Option<(&'a TierGrids, &'a Grid, &'a Cell)> {
    tiers.iter().find_map(|tier| {
        tier.grids.iter().find_map(|grid| {
            grid.cells
                .iter()
                .find(|cell| cell.name == name)
                .map(|cell| (tier, grid, cell))
        })
    })
}

/// The index in `grid0`, `grid12`: the number the node table is keyed on.
fn grid_index(grid: &Grid) -> Option<usize> {
    grid.name
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
}

/// The map of each mounted package's campaign. A cell whose circuit this source
/// does not offer is still a node (the card says which circuit); launching it is
/// refused by name when it is tapped.
#[must_use]
pub fn tiers(
    grids: &[TierGrids],
    tracks: &[Track],
    names: &CircuitNames,
    strings: &StringTable,
) -> Vec<Tier> {
    grids
        .iter()
        .map(|tier| Tier {
            tile: tier.add_on.tile.to_string(),
            events: tier
                .grids
                .iter()
                .flat_map(|grid| {
                    let index = grid_index(grid);
                    grid.cells
                        .iter()
                        .filter_map(move |cell| Some((index?, cell)))
                })
                .filter_map(|(index, cell)| {
                    node(&tier.add_on.nodes, index, cell, tracks, names, strings)
                })
                .collect(),
        })
        .collect()
}

fn node(
    nodes: &oag_title::NodeMap,
    grid: usize,
    cell: &Cell,
    tracks: &[Track],
    names: &CircuitNames,
    strings: &StringTable,
) -> Option<MapEvent> {
    let (x, y) = nodes.step(grid, cell.grid_coords()?)?;
    let track = cell
        .track
        .as_deref()
        .and_then(|id| tracks.iter().find(|track| track.id == id));
    let circuit = match (track, cell.track.as_deref()) {
        (Some(track), _) => oag_raceplay::catalogue::label(track, names, strings, tracks),
        (None, Some(id)) => id.to_string(),
        (None, None) => "no circuit".to_string(),
    };
    let kind = icon(&cell.mode);
    let mut detail = format!("{circuit} / {}", cell.mode);
    detail.push_str(&format!(" / {}", cell.class));
    if let Some(laps) = cell.laps.filter(|&laps| laps > 0) {
        detail.push_str(&format!(" / {laps} laps"));
    }
    Some(MapEvent {
        name: cell.name.clone(),
        x,
        y,
        detail,
        requires: None,
        kind,
        forced_craft: None,
        refused_craft: Vec::new(),
        card: card(cell, track, &circuit, kind, strings),
    })
}

/// The icon a cell's mode draws. **Chosen, not measured**: the four icons are
/// 2048's own mode glyphs (`EventIcon`), and HD's Detonator and Nitro Battle
/// have none - a crosshair and a radar are the nearest.
fn icon(mode: &Mode) -> EventIcon {
    match mode {
        Mode::Race | Mode::Tournament | Mode::Head2Head | Mode::CustomGrid | Mode::AiRace => {
            EventIcon::Race
        }
        Mode::TimeTrial | Mode::SpeedLap => EventIcon::SpeedLap,
        Mode::Zone => EventIcon::Zone,
        Mode::Elimination => EventIcon::Elimination,
        Mode::Other(name) if name.eq_ignore_ascii_case("NitroBattle") => EventIcon::Zone,
        Mode::Other(_) => EventIcon::Elimination,
    }
}

fn card(
    cell: &Cell,
    track: Option<&Track>,
    circuit: &str,
    kind: EventIcon,
    strings: &StringTable,
) -> EventCard {
    let ordinal = class_ordinal(&cell.class);
    let has_class = kind != EventIcon::Zone;
    let kind_id = match (kind, &cell.mode) {
        (EventIcon::Race, _) => "IG_HUD_RACE",
        (EventIcon::SpeedLap, Mode::SpeedLap) => "Speed Lap",
        (EventIcon::SpeedLap, _) => "Time Trial",
        (EventIcon::Zone, _) => "Zone",
        (EventIcon::Elimination, _) => "FE_GAMEMODE_ELIM",
    };
    let laps = cell.laps.filter(|&laps| laps > 0);
    // The circuit's photo is a file of its own per circuit; a circuit driven
    // backwards shows its forward twin's.
    let photo = cell.track.as_deref().map(|id| {
        let forward = id.strip_suffix("_R").unwrap_or(id);
        format!(r"Data\FE\NewImages\trackscreens\{forward}.gtf")
    });
    let emblem = track.map(|track| format!(r"{}\fe\TrackSelectEmblem.gtf", track.location));
    EventCard {
        title: circuit.to_string(),
        kind_label: strings.get(kind_id).map(str::to_string),
        has_objective: false,
        laps,
        class_icon: ordinal.filter(|_| has_class).and_then(class_icon),
        class_label: ordinal
            .filter(|_| has_class)
            .and_then(|o| class_label(strings, o)),
        lap_label: laps.and_then(|laps| {
            let id = if laps == 1 {
                "Callout_Lap"
            } else {
                "Callout_Laps"
            };
            Some(strings.get(id)?.replace("%d", &laps.to_string()))
        }),
        weapons: (!cell.weapons).then(|| weapons_off(strings)).flatten(),
        tabs: card_tabs(strings),
        photo,
        emblem,
        ..EventCard::default()
    }
}

/// The textures the cards of `tiers` ask for beyond the fixed set.
pub(super) fn card_textures(tiers: &[Tier]) -> Vec<&str> {
    let mut names = Vec::new();
    for card in tiers
        .iter()
        .flat_map(|tier| &tier.events)
        .map(|event| &event.card)
    {
        let weapon_icons = card.weapons.iter().flat_map(|weapons| weapons.icons.iter());
        for name in weapon_icons
            .chain(card.photo.iter())
            .chain(card.emblem.iter())
        {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
    }
    names
}

/// Whether `name`, a cell, is closed: the grid it is in has not earned the
/// points the grid before it asks, or this build cannot race its mode.
///
/// The grid law is the one HD's own campaign screens apply
/// (`CampaignStage::grid_is_unlocked`): a grid is open when it authors no lock,
/// when it has earned any points, or when the grid it names in its
/// `<Unlock Grid>` has earned its `RequiredPoints`. **Inherited, not measured on
/// 2048**: the executable's own gate on these nodes is unread. A cell whose mode
/// this build cannot race is closed whatever the save holds, so the player is not
/// offered a card whose launch would refuse - **chosen, not measured**.
///
/// `unlock_all` (`--unlock-all`, a debug build's `[dev] unlock_everything`)
/// opens the grids and leaves the modes closed.
#[must_use]
pub fn closed(
    tiers: &[TierGrids],
    records: &crate::records::Store,
    title: &str,
    unlock_all: bool,
    name: &str,
) -> bool {
    let Some((tier, grid, cell)) = find(tiers, name) else {
        return false;
    };
    if crate::campaign::race_mode_for_cell(cell.mode.clone()).is_none() {
        return true;
    }
    if unlock_all {
        return false;
    }
    let medal_of: &dyn Fn(&str) -> Option<Medal> =
        &crate::unlock::campaign_medal_of(records, title);
    let open = !grid.locked
        || grid.points_earned(medal_of) > 0
        || grid.unlock_grid.as_deref().is_none_or(|target| {
            oag_tables::race_campaign::grid_points_met(&tier.grids, target, medal_of)
        });
    !open
}

/// Closes the nodes of `frontend`'s downloadable campaigns that `records` has
/// not opened: [`closed`] over the grids `source` carries. A no-op on a title
/// with no packages, and on a source whose packages are not mounted.
///
/// Reads the grids afresh rather than keeping them from boot, as every other
/// screen of this build that reads the disc does, so `boot` carries no grids
/// the front end has no use for.
pub fn refresh_locks(
    frontend: &mut oag_ui::frontend::Frontend,
    title: &'static oag_title::Title,
    source: &str,
    records: &crate::records::Store,
    campaign_title: &str,
) {
    if title.archives.add_ons.is_empty() {
        return;
    }
    let Ok(opened) = oag_source::title::open_source(source, Vec::new(), Vec::new()) else {
        return;
    };
    let mut archives = opened.archives;
    let grids = read(&mut archives, &mut Vec::new());
    let unlock_all = crate::unlock::unlock_all();
    frontend
        .refresh_campaign_locks(|name| closed(&grids, records, campaign_title, unlock_all, name));
}
