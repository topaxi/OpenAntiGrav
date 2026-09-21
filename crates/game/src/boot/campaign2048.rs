//! Wipeout 2048's campaign map and this build's two extra tiles, as the boot
//! hands them to the front end.
//!
//! Its own file rather than a stretch of `boot.rs`, for the reason every
//! sibling here gives: that file is baselined by `scripts/check-file-size.py`
//! and may shrink but not grow. Both halves are 2048's alone - the map is
//! `SP.xml`'s (`docs/formats/2048-campaign.md`) and the tiles are this
//! build's own additions to its mode grid (`oag_ui::frontend::touch`'s
//! module docs) - and both are read here, where the archives and the
//! language are in hand, so the front end receives text and cells and never
//! opens a file.

use oag_ui::frontend::{ExtraTile, Launch, MapEvent};
use oag_ui::language::StringTable;

/// Every `SP.xml` event that has a map cell, as the map draws it.
///
/// **Filtered by what the file says, not by a save.** The 21 instances with
/// no `M_X`/`M_Y` are the multiplayer twins (`MP_*`, also carried by
/// `SP.xml`) and have no cell to draw at; the five `E3_*` instances do carry
/// cells and are left out too, by name - they are the E3 demo's races,
/// authored in the same file, and whether the shipped game ever shows them
/// is unmeasured, so they are named here as the one exclusion rather than
/// drawn as campaign events. Every event past that filter gets a
/// [`MapEvent::requires`] off `oag_2048::campaign::unlock_gates` - disc-only,
/// same as everything else here - but this function alone never decides
/// which are actually locked: that needs a save, which nothing in `boot`
/// reads, so `Frontend::refresh_campaign_progress` is what a caller with one
/// (`Session::finish_loading`) calls once these events reach the front end.
pub(super) fn map_events(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    report: &mut Vec<String>,
) -> Vec<MapEvent> {
    let text = match archives
        .read_name(oag_2048::campaign::SP_XML)
        .map_err(|e| e.to_string())
        .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
    {
        Ok(text) => text,
        Err(why) => {
            report.push(format!(
                "{}: {why} - the campaign map has no events",
                oag_2048::campaign::SP_XML
            ));
            return Vec::new();
        }
    };
    let doc = oag_2048::campaign::parse(&text);
    let events = oag_2048::campaign::events(&doc);
    // Every event's own gate, by name - disc-only (no save, no `Store`), so
    // this stays a plain read even though what unlocks a gate is not
    // decided until `Frontend::refresh_campaign_progress` runs, later, once
    // a save is in hand. See `oag_2048::campaign::unlock_gates`'s own doc.
    let gates = oag_2048::campaign::unlock_gates(&doc);
    let mut out = Vec::new();
    let mut without_cell = 0usize;
    let mut demo = 0usize;
    for event in &events {
        let (Some(x), Some(y)) = (event.x, event.y) else {
            without_cell += 1;
            continue;
        };
        if event.name.starts_with("E3_") {
            demo += 1;
            continue;
        }
        let requires = gates
            .iter()
            .find(|(name, _)| *name == event.name)
            .and_then(|(_, gate)| gate.clone());
        let circuit = event
            .track
            .and_then(|track| oag_2048::campaign::track_for(&doc, track))
            .map(|track| track.display_name)
            .unwrap_or_else(|| "no circuit".to_string());
        let class = event
            .speed_class
            .and_then(oag_2048::campaign::EClass::from_ordinal)
            .map(|class| class.as_str());
        let mode = oag_2048::campaign::engine_mode(event).unwrap_or("unknown mode");
        let mut detail = format!("{circuit} / {}", mode.replace('_', " "));
        if let Some(class) = class {
            detail.push_str(&format!(" / {class}"));
        }
        if let Some(laps) = event.laps.filter(|&laps| laps > 0) {
            detail.push_str(&format!(" / {laps} laps"));
        }
        // The description idstring is mostly empty on the disc
        // (`2048_EVENT_3` resolves to `""`); where it says something, it
        // is appended.
        if let Some(text) = event
            .description
            .as_deref()
            .and_then(|id| strings.get(id))
            .filter(|text| !text.trim().is_empty())
        {
            detail.push_str(&format!(" - {text}"));
        }
        out.push(MapEvent {
            name: event.name.clone(),
            x,
            y,
            detail,
            requires,
        });
    }
    report.push(format!(
        "{}: {} event(s) on the campaign map, {without_cell} with no cell skipped, {demo} E3 demo event(s) left out",
        oag_2048::campaign::SP_XML,
        out.len()
    ));
    out
}

/// This build's own two tiles, labelled through the project's own string
/// table - the ids `assets/ui/menu.toml` labels the same pages with.
///
/// A language with no project file of its own - German, today - falls back
/// to the English text, the way `menu.toml`'s rows fall back to their
/// literal `label`, rather than putting the bare id on a tile.
pub(super) fn extra_tiles(language: Option<&str>) -> Vec<ExtraTile> {
    let strings = oag_ui::strings::project_table(language);
    let english = oag_ui::strings::project_table(None);
    let label = |id: &str| {
        strings
            .get(id)
            .or_else(|| english.get(id))
            .unwrap_or(id)
            .to_string()
    };
    vec![
        ExtraTile {
            label: label("OAG_MENU_RACEBOX"),
            launch: Launch::RaceBox,
        },
        ExtraTile {
            label: label("OAG_MENU_REMIX"),
            launch: Launch::Remix,
        },
    ]
}
