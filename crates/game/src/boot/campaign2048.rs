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

use oag_ui::frontend::{EventCard, EventIcon, ExtraTile, Launch, MapEvent};
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
        // The disc's own four mode icons - see `EventIcon`'s own doc
        // comment. Chosen by the event's class the way the executable does:
        // each `GameMode_*` constructor writes its own icon ordinal
        // (`FUN_81061040`), `GameMode_SpeedLapRace` `3` - the stopwatch. That
        // takes all 53 of its events, the 13 timed races (`"2048 - Event 3"`,
        // laps and a `BEAT M:SS` pass) as well as the 40 with `laps == 0`.
        let kind = match event.typedef_id {
            oag_tables::mjolnir::campaign::typedef::RACE_A => EventIcon::SpeedLap,
            _ => match event.kind {
                oag_2048::campaign::EventKind::Zone => EventIcon::Zone,
                oag_2048::campaign::EventKind::Elimination => EventIcon::Elimination,
                oag_2048::campaign::EventKind::Race => EventIcon::Race,
            },
        };
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
        // Both read off the same `Instance` this event's own fields already
        // came from - `oag_2048::campaign::craft`'s own doc comment for what
        // each means and why they never coexist on the real file.
        let (forced_craft, refused_craft) = match doc.instance(event.instance_id) {
            Some(instance) => (
                oag_2048::campaign::craft::forced_craft(&doc, instance),
                oag_2048::campaign::craft::refused_craft(&doc, instance),
            ),
            None => (None, Vec::new()),
        };
        out.push(MapEvent {
            name: event.name.clone(),
            x,
            y,
            detail,
            requires,
            kind,
            forced_craft,
            refused_craft,
            card: event_card(&doc, event, kind, strings),
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

/// The ten circuits `FUN_81060744` (`0x810f1196`'s photo call) and
/// `FUN_81060584` (the emblem) name-match `M_TRACKNAME` against. The
/// executable's own table also carries the DLC circuits and their `_R`
/// reverses, none of which `SP.xml` authors an event on.
const CARD_TRACKS: [&str; 10] = [
    "square",
    "park",
    "tower",
    "mall",
    "bridge",
    "arena",
    "subway",
    "cathedral",
    "sol",
    "altima",
];

/// `GameModeObjective_FormatText` (`FUN_812b671a`)'s ordinal-to-wording table
/// plus the per-mode override it offers first (`vtable+0x6c`; eight
/// `GameMode_*` vtables share the slot). The override matters for
/// `BEAT_VALUE` (`2`) only. Of the eight, `GameMode_ZoneRace`'s (mode icon
/// ordinal `0`) is `FUN_812c4a2a` (`"%s : %d"` over `FE_ZONE_TARGET`),
/// `GameMode_SpeedLapRace`'s (ordinal `3`) is `FUN_812c1011`
/// (`MP-Objective_Beat_1` over the target as `M:SS`, `FUN_8114f252`), and the
/// other six keep the base class's `return 0` (`0x813ea238`), so their
/// `BEAT_VALUE` falls through to `FE_SCORE_POINTS` (`SCORE %d POINTS`) -
/// Elimination (ordinal `1`) among them. The class of an event is its
/// typedef, not its [`EventKind`], which merges the two race typedefs.
pub(crate) fn objective_text(
    strings: &StringTable,
    typedef_id: i64,
    objective_type: i64,
    target: i64,
) -> Option<String> {
    use oag_tables::mjolnir::campaign::typedef;
    if objective_type == 2 {
        if typedef_id == typedef::ZONE {
            let head = strings.get("FE_ZONE_TARGET")?;
            return Some(format!("{head} : {target}"));
        }
        if typedef_id == typedef::RACE_A {
            let beat = strings.get("MP-Objective_Beat_1")?;
            return Some(beat.replace("%s", &centiseconds_as_clock(target)));
        }
    }
    let id = match (objective_type, target) {
        (1, _) => "SP_Objective_Finish",
        (2, _) => "FE_SCORE_POINTS",
        (4, 1) => "ER_FINISH_1ST",
        (4, 2) => "ER_FINISH_2ND",
        (4, 3) => "ER_FINISH_3RD",
        (4, _) => "ER_FINISH_IN_POS",
        (7, _) => "FE_ELIMINATE_OPP",
        _ => return None,
    };
    let text = strings.get(id)?;
    Some(text.replace("%d", &target.to_string()))
}

/// `FUN_8114f252`: hundredths of a second as `M:SS`, saturating at `9:59`,
/// with `0xffff` (the "no time" sentinel) reading as zero.
fn centiseconds_as_clock(centiseconds: i64) -> String {
    let centiseconds = if centiseconds == 0xffff {
        0
    } else {
        centiseconds.max(0)
    };
    let seconds = centiseconds / 100;
    let (minutes, seconds) = if seconds / 60 > 9 {
        (9, 59)
    } else {
        (seconds / 60, seconds % 60)
    };
    format!("{minutes}:{seconds:02}")
}

/// `FUN_8105dcb8` loads one `Team_Logos` texture per native team; `FUN_81061808`
/// picks it by the team's own string. Piranha's file is spelt `Pirhana` on the
/// disc, and AG Systems' `AG-SYS`.
fn team_logo(team: &str) -> Option<&'static str> {
    Some(match team {
        "Feisar2048" => r"Data\FE\NewImages\Team_Logos\Icon_Team_Feisar.gtf",
        "AG_Systems2048" => r"Data\FE\NewImages\Team_Logos\Icon_Team_AG-SYS.gtf",
        "Qirex2048" => r"Data\FE\NewImages\Team_Logos\Icon_Team_Qirex.gtf",
        "Auricom2048" => r"Data\FE\NewImages\Team_Logos\Icon_Team_Auricom.gtf",
        "Piranha2048" => r"Data\FE\NewImages\Team_Logos\Icon_Team_Pirhana.gtf",
        _ => return None,
    })
}

/// `FUN_81061808`'s class icon for a `(team, variant)` id: variants `1`-`3`
/// are combat, agility and speed (`Icon_Ship_Combat`, `_Agility`, `_Racer`),
/// `4` is the prototype, whose icon depends on which class the team's
/// prototype is (AG Systems agility, Qirex and Auricom combat, the rest
/// speed).
fn ship_class_icon(team: &str, variant: &str) -> Option<&'static str> {
    Some(match (variant, team) {
        ("1", _) => r"Data\FE\NewImages\Team_Logos\Icon_Ship_Combat.gtf",
        ("2", _) => r"Data\FE\NewImages\Team_Logos\Icon_Ship_Agility.gtf",
        ("3", _) => r"Data\FE\NewImages\Team_Logos\Icon_Ship_Racer.gtf",
        ("4", "AG_Systems2048") => r"Data\FE\NewImages\Team_Logos\Icon_Ship_Agility_proto.gtf",
        ("4", "Qirex2048" | "Auricom2048") => {
            r"Data\FE\NewImages\Team_Logos\Icon_Ship_Combat_proto.gtf"
        }
        ("4", _) => r"Data\FE\NewImages\Team_Logos\Icon_Ship_Racer_proto.gtf",
        _ => return None,
    })
}

/// What one event's card says, off the same instances the map already read.
fn event_card(
    doc: &oag_2048::campaign::Document,
    event: &oag_2048::campaign::Event,
    kind: EventIcon,
    strings: &StringTable,
) -> EventCard {
    let track = event
        .track
        .and_then(|track| oag_2048::campaign::track_for(doc, track));
    let known = track
        .as_ref()
        .map(|track| track.track_name.as_str())
        .filter(|name| CARD_TRACKS.contains(name));
    let photo = known.map(|name| {
        let mut chars = name.chars();
        let head: String = chars
            .next()
            .into_iter()
            .flat_map(char::to_uppercase)
            .collect();
        // `FUN_81060744`'s `isZone` (`event+0x190 == 0`, the Zone class) picks
        // the `Zone<Name>` file of the same circuit.
        let zone = if kind == EventIcon::Zone { "Zone" } else { "" };
        format!(
            r"Data\FE\NewImages\trackscreens\{zone}{head}{}.gtf",
            chars.as_str()
        )
    });
    let emblem = known.map(|name| format!(r"Data\FE\NewImages\tracks\{name}.gtf"));
    let objective = event
        .pass_objective
        .and_then(|reference| oag_2048::campaign::objective_for(doc, reference));
    // `FUN_810535fe` and `FUN_81055150` draw the class glyph only when the
    // mode ordinal `+0x190` is not `0`: a Zone event has none.
    let has_class = kind != EventIcon::Zone;
    let class_icon = event
        .speed_class
        .filter(|_| has_class)
        .and_then(|class| usize::try_from(class).ok())
        .and_then(|class| {
            oag_ui::frontend::CARD_TEXTURES
                .iter()
                .copied()
                .filter(|t| t.contains(r"speedclass\"))
                .nth(class)
        })
        .map(str::to_string);
    // `FUN_812b021a`: the line under the title, by the same ordinal. The
    // timed class says `SPEED LAP` when `M_BUTTONSHAPE` is `5` or `6` and
    // `TIME TRIAL` otherwise; Elimination's own word is `FE_GAMEMODE_ELIM`
    // (`COMBAT`).
    let shape = doc
        .instance(event.instance_id)
        .and_then(|instance| instance.field("M_BUTTONSHAPE"))
        .and_then(oag_2048::campaign::Field::int);
    let kind_id = match kind {
        EventIcon::Race => "IG_HUD_RACE",
        EventIcon::SpeedLap if matches!(shape, Some(5 | 6)) => "Speed Lap",
        EventIcon::SpeedLap => "Time Trial",
        EventIcon::Zone => "Zone",
        EventIcon::Elimination => "FE_GAMEMODE_ELIM",
    };
    let instance = doc.instance(event.instance_id);
    let class_label = event.speed_class.filter(|_| has_class).and_then(|class| {
        // `FUN_812b26cc`: ordinals `0` and `1` both read `Speed_Class_C_0`.
        let id = match class {
            0 | 1 => "Speed_Class_C_0",
            2 => "Speed_Class_B_0",
            3 => "Speed_Class_A_0",
            4 => "Speed_Class_A_Plus_0",
            _ => return None,
        };
        strings.get(id).map(str::to_string)
    });
    let lap_label = event.laps.filter(|&laps| laps > 0).and_then(|laps| {
        let id = if laps == 1 {
            "Callout_Lap"
        } else {
            "Callout_Laps"
        };
        Some(strings.get(id)?.replace("%d", &laps.to_string()))
    });
    let forced_craft = instance
        .and_then(|instance| oag_2048::campaign::craft::forced_craft(doc, instance))
        .map(|id| {
            let (team, variant) = id.split_once(['\\', '/']).unwrap_or((&id, ""));
            let team = team.to_string();
            oag_ui::frontend::CardCraft {
                logo: team_logo(&team).map(str::to_string),
                type_icon: ship_class_icon(&team, variant).map(str::to_string),
                caption: craft_caption(strings, &team, variant),
            }
        });
    // `FUN_810535fe` draws a glyph for each class `GameModeBase_IsShipTypeAllowed`
    // still allows, but only when any of the four prevent flags is set.
    let restriction = instance.map(oag_2048::campaign::craft::restriction);
    let allowed_classes = match restriction {
        Some(flags) if flags.iter().any(|&flag| flag) => [
            ("Icon_Ship_Combat_1col", "FE_SHIP_COMBAT_ONLY", flags[0]),
            ("Icon_Ship_Agility_1col", "FE_SHIP_AGILITY_ONLY", flags[1]),
            ("Icon_Ship_Racer_1col", "FE_SHIP_SPEED_ONLY", flags[2]),
        ]
        .into_iter()
        .filter(|(_, _, prevented)| !prevented)
        .filter_map(|(icon, id, _)| {
            Some(oag_ui::frontend::CardRestriction {
                icon: format!(r"Data\FE\NewImages\Team_Logos\{icon}.gtf"),
                label: strings.get(id)?.to_string(),
            })
        })
        .collect(),
        _ => Vec::new(),
    };
    let word = |id: &str| strings.get(id).map(str::to_string).unwrap_or_default();
    let trophy = oag_2048::campaign::trophy::art_for(&event.name, shape).map(|art| {
        oag_ui::frontend::CardTrophy {
            texture: art.texture,
            header: word(&art.header_id),
            callout: word(&art.callout_id),
        }
    });
    let weapons = oag_2048::campaign::callout::for_event(doc, event)
        .and_then(|callout| card_weapons(strings, callout));
    let elite = event
        .elite_objective
        .and_then(|reference| oag_2048::campaign::objective_for(doc, reference));
    EventCard {
        weapons,
        elite_label: strings.get("FE_ELITE_PASS").map(str::to_string),
        elite_objective: elite.and_then(|objective| {
            objective_text(
                strings,
                event.typedef_id,
                objective.objective_type?,
                objective.target.unwrap_or(0),
            )
        }),
        has_trophy_page: oag_2048::campaign::trophy::has_page(shape),
        trophy,
        class_label,
        lap_label,
        forced_craft,
        allowed_classes,
        tabs: oag_ui::frontend::CardTabs {
            tabs: [word("FE_PERSONAL"), word("FE_FRIENDS"), word("FE_GLOBAL")],
            current_best: word("FE_CURRENT_BEST"),
        },
        title: track.map(|track| track.display_name).unwrap_or_default(),
        kind_label: strings.get(kind_id).map(str::to_string),
        pass_label: strings.get("FE_PASS").map(str::to_string),
        has_objective: objective.is_some(),
        objective: objective.and_then(|objective| {
            objective_text(
                strings,
                event.typedef_id,
                objective.objective_type?,
                objective.target.unwrap_or(0),
            )
        }),
        laps: event.laps.filter(|&laps| laps > 0),
        class_icon,
        photo,
        emblem,
    }
}

/// `FUN_81061808`'s caption: `"%s %s"` of the team's label and the craft
/// class's (`WOShipModelData_LiveryLabelId`: variants `1`-`3` are combat,
/// agility and speed, `4` the prototype).
fn craft_caption(strings: &StringTable, team: &str, variant: &str) -> String {
    let team_label = strings
        .get(team)
        .map_or_else(|| team.to_string(), str::to_string);
    let class_id = match variant {
        "1" => "FE_SHIP_COMBAT",
        "2" => "FE_SHIP_AGILITY",
        "3" => "FE_SHIP_SPEED",
        "4" => "FE_SHIP_PROTO",
        _ => return team_label,
    };
    match strings.get(class_id) {
        Some(class) => format!("{team_label} {class}"),
        None => team_label,
    }
}

/// What the weapon callout draws for `callout`, `None` when it draws nothing.
fn card_weapons(
    strings: &StringTable,
    callout: oag_2048::campaign::callout::Callout,
) -> Option<oag_ui::frontend::CardWeapons> {
    use oag_2048::campaign::callout::{Callout, ICONS, NAME_IDS, SEPARATOR, texture};
    let single = |stem: &str, id: &str| oag_ui::frontend::CardWeapons {
        icons: vec![texture(stem)],
        caption: strings.get(id).map(str::to_string).unwrap_or_default(),
    };
    Some(match callout {
        Callout::Nothing => return None,
        Callout::WeaponsOff => single("weapons_off", "Event_Variable_Weapons_Off_0"),
        Callout::OffensiveOff => single("offensive_off", "Event_Variable_Offensive_1"),
        Callout::DefensiveOff => single("defensive_off", "Event_Variable_Defensive_1"),
        Callout::Weapons(bits) => oag_ui::frontend::CardWeapons {
            icons: bits.iter().map(|&bit| texture(ICONS[bit])).collect(),
            caption: bits
                .iter()
                .filter_map(|&bit| strings.get(NAME_IDS[bit]))
                .collect::<Vec<_>>()
                .join(SEPARATOR),
        },
    })
}

/// Every texture the cards of `events` ask for beyond the fixed set.
pub(super) fn card_textures(events: &[MapEvent]) -> Vec<&str> {
    let mut names: Vec<&str> = oag_ui::frontend::CARD_TEXTURES.to_vec();
    for card in events.iter().map(|event| &event.card) {
        let craft = card.forced_craft.as_ref();
        let logo = craft.map(|c| &c.logo).unwrap_or(&None);
        let icon = craft.map(|c| &c.type_icon).unwrap_or(&None);
        let art = card.trophy.as_ref().map(|t| &t.texture);
        let weapon_icons = card.weapons.iter().flat_map(|w| w.icons.iter());
        for name in weapon_icons {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
        for name in [
            card.photo.as_ref(),
            card.emblem.as_ref(),
            logo.as_ref(),
            icon.as_ref(),
            art,
        ]
        .into_iter()
        .flatten()
        {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
    }
    names
}
