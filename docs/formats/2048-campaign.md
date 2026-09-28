# 2048's own campaign: the "mjolnir" typed-instance database, `Data\xml\SP.xml`

**Status: partial.** Implemented in
[`oag_tables::mjolnir`](../../crates/tables/src/mjolnir.rs) (generic reader)
and [`oag_tables::mjolnir::campaign`](../../crates/tables/src/mjolnir/campaign.rs)
(typed view: event kinds, `TrackDefinition`, `WeaponSetDefinition`), with the
entry names and 2048-specific facts in
[`oag_2048::campaign`](../../crates/2048/src/campaign.rs), per
[ADR-0022](../architecture/adr/0022-title-packages.md). Wired into a headless
launch: `crates/game/src/race/load/campaign.rs`'s `race::load_event` resolves
an event by name onto `race::Options` before calling the existing `load`, and
`oag-game --event "<name>"` drives it end to end - see
[the launch section](#the-launch-cratesgamesrcraceloadcampaignrs) below.

Wipeout 2048 has no `CellMode_Definition.xml` and no racebox
([`race-setup.md`](race-setup.md): "2048 has no such flow at all"). Its own
campaign - not the two DLC-gated `PI_Grid` tiers `data/plugins/grids/grid_NN.xml`
author (`Campaign="HD"`/`Campaign="Fury"`, already reachable through
[`oag_tables::race_campaign`](race-campaign.md) and
[`oag_hd::campaign`](../../crates/hd/src/campaign.rs)'s own schema) - lives in
`Data\xml\SP.xml`: 1.3 MB, single-player, discovered by extracting
`data/xml/SP.xml` out of `PSP2/data.psarc` and finding it is not `PI_Grid`
shaped at all.

## The format: `<mjolnir><instance typedefid="..." name="...">`

```xml
<mjolnir>
  <instance instanceid="-1143582041" typedefid="-1915183557" name="2048 - Event 3">
    <DATA>
      <M_TRACKDEF name="m_trackDef" type="TrackDefinition" length="1" typedefid="205052969">
        <ARRAY value="-1892961298" typedefid="205052969"/>
      </M_TRACKDEF>
      <M_PNEXTEVENT name="m_pNextEvent" type="GameModeBase" length="1" typedefid="366306753">
        <ARRAY value="-484309551" typedefid="-1353052320"/>
      </M_PNEXTEVENT>
      ...
    </DATA>
    <BASE>...</BASE>
    <USER>...</USER>
  </instance>
  ...
</mjolnir>
```

Every `<instance>` is a typed record; `typedefid` names which shape. Every
`<M_FOO>` child under `<DATA>` is one field, itself carrying a declared
`type=`/`typedefid=` and one or more `<ARRAY>` children holding the value(s).
Plain UTF-8 XML, not [fexml](fexml.md)-shortened - no `<code>` dictionary -
but read through [`oag_tables::fexml::parse`](../../crates/tables/src/fexml.rs)
regardless, since that reader's tag/attribute scanner is format-agnostic and
already tolerates this disc's malformed tags.

### The strongest evidence: `type=`/`typedefid=` is an in-file name table

Every field that references another instance carries that instance's own
type name right on the field element - `M_TRACKDEF`'s `type="TrackDefinition"
typedefid="205052969"` is the file *telling* a reader what `205052969` is, not
a reader guessing from field shape. Collecting every `(typedefid, type)` pair
across all 288 `SP.xml` instances closes the name table with no ambiguity for
eight of the ids in play:

| `typedefid` | name | instances | confidence |
| --- | --- | --- | --- |
| `380278911` | `GameModeObjective` | 96 | 95 |
| `366306753` | `GameModeBase` | 0 direct (abstract base - see below) | 95 |
| `520725191` | `WOShipModelData` | 21 | 95 |
| `205052969` | `TrackDefinition` | 10 | 95 |
| `-966434245` | `WeaponSetDefinition` | 20 | 95 |

Confidence 95 for all five: this is the file's own authored text,
cross-checked against every one of the dozens of fields that names each
typedef, not a structural read. Everything else the file spells this way
(`eClass`, `bool`, `u32`, `int`, `float`, `char`, `GameModeOptions`,
`CanvasButtonShape`, `WOShipCreatorParams`, `ObjectiveValue`, `I_UnlockData`,
`ActiveWeaponPads`, `WeaponType`, `ObjectiveOptions`) is a scalar or nested
type, never itself an instance.

**This corrects a hypothesis, not just fills a gap.** Before this pass read
the field-type pairs, `-966434245`'s twenty single-field instances (each one
`M_WEAPONAVAILABLEBITS`) were guessed to be `TrackDefinition`, reasoning "2048
ships 20 track profiles." Two things were wrong with that guess at once: 2048
ships **ten** circuits, not twenty (`data/plugins/tracks/Definition.xml` lists
`square`, `park`, `tower`, `mall`, `bridge`, `arena`, `subway`, `cathedral`,
`sol`, `altima` - exactly `205052969`'s own instance count), and the file's
own text names `-966434245` `WeaponSetDefinition` outright - its twenty
instances are named `"Rockets Only"`, `"Missile Only"`, `"Cannons, Missile,
Plasma"`, and so on, weapon-loadout presets, not circuits. `205052969`'s own
`M_TRACKNAME` field matches the plugin's ten stems exactly, confirming the
correction from the other direction too. Anyone who read the earlier
hypothesis should re-check against this table.

### Four more typedefs carry no name in the file at all

`-1915183557` (53 instances), `-1353052320` (52), `1311982788` (26) and
`1018671239` (10) - the four concrete event shapes - are never a field's
*declared* type anywhere in `SP.xml`. Every field that points at one of them
(`M_PNEXTEVENT`, `M_PBRANCHEVENT`, `M_PEVENTREQUIRED`, and
`WOShipModelData`'s own `M_PCAMPAIGNUNLOCK`) declares the abstract
`GameModeBase` (`366306753`) as its *static* type instead - normal C++
polymorphism, serialised. **The `<ARRAY>` element itself still carries the
concrete pointee's own `typedefid`**: `2048 - Event 3`'s own `M_PNEXTEVENT`
reads `<ARRAY value="-484309551" typedefid="-1353052320"/>`, naming the
referenced instance's real shape even though the field's own declaration
cannot. `oag_tables::mjolnir::Reference::typedef_id` recovers this for every
edge in the event graph without a second lookup.

Since nothing in the file names these four, [`EventKind`] classifies them by
measured field shape instead, at confidence 78-82 (structural fit plus
corroborating instance names, still short of a runtime trace):

| typedef | count | distinguishing fields | instance names | `EventKind` |
| --- | --- | --- | --- | --- |
| `1018671239` | 10 | `M_ZONETIMECOUNTER`/`M_ZONETOADDMINES`/`M_STARTZONENUMBER`/`M_ENDZONENUMBER`/`M_NUMBEROFMINES`; `M_SPEEDCLASS` always empty | generic `"20XX - Event N"` | `Zone` |
| `1311982788` | 26 | `M_ELIMINATENUMOFOPPONENTS`/`M_SCORETARGET`/`M_SURVIVEFORNUMOFLAPS`/`M_TIMELIMIT`/`M_BSEEKANDDESTROYTARGETID`/`M_BSOLOSCORING` | `"MPElimination"`, `"MP_Arena_Eliminator_flash"`, generic ones | `Elimination` |
| `-1915183557` | 53 | the only typedef with a non-empty `M_MAXGHOSTSHIPS`; the only one carrying `laps == 0` (all 40 Speed Lap events) | `"<Track> Speed Lap - <Class>"` (40) plus 13 generic | `Race` |
| `-1353052320` | 52 | no `M_MAXGHOSTSHIPS` at all; `laps` always `>= 1` | `E3_Demo_*`, `MP_*_Race_flash`, `"* Ship Challenge"` (10-lap), generic ones | `Race` |

**`eboot.elf`'s own string table names six `GameMode_*` C++ classes**
(`GameMode_ArcadeRace`, `GameMode_CheckPointRace`, `GameMode_EliminatorRace`,
`GameMode_SpeedLapRace`, `GameMode_ZombieRace`, `GameMode_ZoneRace`), found
with plain `strings` over `data/extracted/vita/PCSF00007/patch-v104/eboot.elf`
(no Ghidra opened for this pass, per this lane's own scope). **None of the
six is bound to a typedef here, and that is deliberate rather than an
oversight**: `M_PNEXTEVENT` chains cross the two `Race` typedefs freely -
`"2048 - Event 3"` (`-1915183557`) names `"2048 - Event 4"` (`-1353052320`)
as its own next event - which argues against the two typedefs being a mode
distinction at all, and confidence for any specific class-to-typedef pairing
would sit under 50 (a guess) with the evidence gathered so far. Recorded as
open, not resolved by the shape of the six names alone.

## The event fields a caller needs to launch one

[`Event`](../../crates/tables/src/mjolnir/campaign.rs) carries the subset of
each event's ~35-40 fields a launcher needs: `track` (a reference into
`TrackDefinition`), `speed_class` (raw `eClass` ordinal), `laps`,
`weapon_set`, the `next_event`/`branch_event`/`required_event` unlock-graph
edges (each a [`Reference`] carrying its own concrete typedef, per the
section above), `pass_objective`/`elite_objective` (references into
`GameModeObjective`, itself unread beyond its own field census - see
[What is not determined](#what-is-not-determined)), `x`/`y` and
`description` (an idstring like `"2048_EVENT_3"`, a language-table key this
crate does not resolve).

### `TrackDefinition` resolves to a circuit by its own `M_TRACKNAME`

All ten instances, exact match against `data/plugins/tracks/Definition.xml`'s
own `<PI_Track name="...">` stems:

| `M_TRACKNAME` | `M_DISPLAYNAME` |
| --- | --- |
| `square` | UNITY SQUARE |
| `mall` | QUEENS MALL |
| `park` | METRO PARK |
| `bridge` | CAPITAL REACH |
| `tower` | EMPIRE CLIMB |
| `arena` | ROCKWAY STADIUM |
| `subway` | SUBWAY |
| `cathedral` | DOWNTOWN |
| `sol` | SOL |
| `altima` | ALTIMA |

`oag_2048::campaign::track_vex_entry` joins `M_TRACKNAME` into
`Data\art\published\environments\<stem>\track.vex`, the same spelling
[`oag_2048::race::DEFAULT_TRACK`](../../crates/2048/src/race.rs) already
uses for `altima`. Confidence 90: the stem match is exact and every stem
names a real archive directory, but this is convention rather than a runtime
read of `track_plugin_definition`'s own `location=` attribute.

### `eClass` is a raw `0`-`4` ordinal, and 2048 has five classes

2048's native `handlingstats.xml` authors `VENOM`, `FLASH`, `RAPIER`,
`PHANTOM`, `SUPERPHANTOM` in that document order (`data/HandlingStats/feisar2048/3/handlingstats.xml`,
checked directly) - a fifth rung [`oag_tables::handling::SpeedClass`]
deliberately does not carry (see [handling-stats.md](handling-stats.md)'s
"the enum was deliberately not widened" section), so `oag_2048::campaign::EClass`
is 2048's own five-variant type rather than a reuse.

`M_SPEEDCLASS`'s ordinals `1`-`4` are measured directly, at confidence 85:
forty `SP.xml` events are named `"<Track> Speed Lap - <Class>"`, one per
track per class, and every one's own `M_SPEEDCLASS` agrees with its name -
`1` on every `- Flash`, `2` on every `- Rapier`, `3` on every `- Phantom`, `4`
on every `- Super P` (SuperPhantom) - across all ten circuits, no exception.
**`0` (Venom) is not directly observed**: no event in either `SP.xml` or
`MP.xml` authors `M_SPEEDCLASS="0"`, and no campaign event is named for
Venom at all (Speed Lap events skip it entirely - only Flash/Rapier/Phantom/
SuperPhantom get a per-track Speed Lap attraction). `0 => Venom` is
elimination over the other four confirmed ordinals, corroborated by - not
measured against - `handlingstats.xml`'s own document order. Confidence 70
for that one rung; a direct `M_SPEEDCLASS="0"` observation would raise it.

### `laps == 0` is Speed Lap's own sentinel

Every one of the 40 `"<Track> Speed Lap - <Class>"` events carries
`M_NUMOFLAPS` absent or `0`; every other lap-race event (the 13 remaining
`-1915183557` instances plus all 52 of `-1353052320`) carries `1` or more.
`oag_2048::campaign::engine_mode` reads `laps == Some(0)` as the mode
`"speed_lap"` and leaves `Options::laps_override` unset so
`oag_race::Mode::laps_target` answers the real count, rather than racing a
literal zero-lap Speed Lap.

## The unlock graph

`M_PNEXTEVENT`/`M_PBRANCHEVENT`/`M_PEVENTREQUIRED` are the whole of the
authored ordering; nothing in the schema authors a linear play sequence or
names a "first" event. `oag_tables::mjolnir::campaign::events` returns
document order only - a caller wanting campaign order has to walk the edges
from wherever it already knows is the start, the same way this project reads
[`race-campaign.md`](race-campaign.md#two-open-questions-this-pass-did-not-resolve)'s own `PI_Grid`
chain.

**Resolved 2026-09-21: `oag_2048::campaign::unlock_gates` walks it.** Every
event's own gate collapses to a single optional name - the event that has to
be completed before this one opens - by folding two authored sources into
one: an incoming `M_PNEXTEVENT`/`M_PBRANCHEVENT` edge (some other event
names this one as its own "what comes next") and this event's own
`M_PEVENTREQUIRED`. Measured directly against every instance in the EU
v1.04 `SP.xml`, not assumed: no event is ever the target of more than one
incoming chain edge, and the two events that carry *both* an incoming edge
and their own `M_PEVENTREQUIRED` (`"2049 - Event 1"`, `"2050 - Event 1"`)
name the identical prerequisite either way - so a single name loses no case
this file authors. Confidence 90.

Of the map's own 115 cell-bearing, non-`E3_*` events (see
[2048-frontend.md](2048-frontend.md)'s own filter), 16 carry no gate at all
and are open on a fresh save: `"2048 - Event 1"` and 15 side events with no
authored prerequisite - a per-team `"* P Ship Challenge"` (five) and a
per-track `"* - S Phantom Challenge"` (ten). `M_bForceAlwaysUnlocked` is
authored on **no event at all** (measured: zero non-empty/non-zero
instances across all 141), so it is not what opens these; the absence of
any incoming edge or `M_PEVENTREQUIRED` is. `M_RankRequired` is likewise
never authored on the numbered campaign's own events, so a player rank gate
is not part of this title's unlock law either, at least not through this
field.

Confirmed live, not only by the field census: `crates/game/tests/
vita_2048_campaign_progress_ground_truth.rs` boots the real package, drives
the pad to `newFEshell`, folds an in-memory `records::Store` in through
`oag_ui::frontend::Frontend::refresh_campaign_progress`, and checks that
recording a bronze on `"2048 - Event 1"` is what turns `"2048 - Event 2"`
from `Locked` to `Open` - `"2048 - Event 1"`'s own `M_PNEXTEVENT` names it.

**What is still chosen, not measured: which tier unlocks the next event.**
Nothing in `SP.xml` says whether *any* result (even one with no medal) or
only a Pass-or-better result opens whatever names this event as a
prerequisite. `oag_ui::frontend::Frontend::refresh_campaign_progress`'s own
caller decides this by what it feeds the closure - today, `Session::
finish_loading` feeds exactly `Store::campaign_medal`'s own `best_medal`,
so a result with no medal at all (the event was left unfinished, or its own
objective law never grades - see "The objective law" below) does not open
anything. No confidence score: this is a design choice on top of a measured
graph, not a reading of the graph itself.

## The objective law

**Measured 2026-09-21**, against every `(pass, elite)` `M_OBJECTIVETYPE`
pair the real file's 80 fully-authored events carry -
`oag_tables::mjolnir::campaign::Objective` reads `M_OBJECTIVETYPE`/
`M_OBJECTIVETARGET` generically; `oag_2048::campaign::{objective_type,
EventObjectives, EventOutcome, Tier, evaluate_tier}` give them a meaning.

**Every event authors both `M_PASSOBJECTIVE` and `M_ELITEOBJECTIVE`, or
neither - never one alone.** Measured across all 141 event instances: 80
with both, 61 with neither (every Speed Lap, every `MP_*` template, and a
handful of generic unlinked nodes). An event with neither authors no medal
at all, though it can still be finished and still opens whatever names it
as a prerequisite.

`M_OBJECTIVETYPE`'s own `ObjectiveValue` ordinal, by the `(pass, elite)`
pair it appears in and the [`EventKind`](#the-strongest-evidence-typetypedefid-is-an-in-file-name-table)
of the 80 events that carry it:

| Ordinal | Name | Meaning | Sample instances |
| --- | --- | --- | --- |
| `1` | `FINISH` | Boolean: met by finishing at all. The two instances that carry it (`FinishRaceAnyPosition`) author no target. | `"2048 - Event 1"`/`"2048 - Event 2"`'s own pass |
| `4` | `POSITION` | The target is a finishing place, `1`-based; met when the player's own place is at most the target ("Nth or better", `1` = win). | `"Top5"` (target `5`), `"Win"` (target `1`), 38 `(POSITION, POSITION)` pairs across the numbered campaign |
| `7` | `KILLS` | The target is an opponent kill count; met when the player's own count is at least the target. Small numbers only (`1` observed) - never confused with `BEAT_VALUE`'s own large Elimination targets below. | `"Kill1Opponent"` (target `1`), the elite half of `"2050 - Event 3-4"`/`"6-4"` |
| `2` | `BEAT_VALUE` | A numeric threshold whose *metric* is not named by the type alone - picked from the event's own `EventKind`, below. | see next |

`BEAT_VALUE`'s metric, read off which `EventKind` carries it and confirmed
by the measured comparison direction (`elite` against `pass`, on every one
of the 80 fully-authored events, no exception found either way):

| `EventKind` | Metric | Direction | Confidence | Sample |
| --- | --- | --- | --- | --- |
| `Race` (13 instances) | Total race time, in centiseconds | Lower is better (`elite < pass` on all 13) | 80 | `"2048 - Event 3"`: pass `13000` (130 s), elite `11000` (110 s) |
| `Zone` (10 instances) | The zone counter reached | Higher is better (`elite > pass` on all 10) | 80 | `"Zone Pass"`/`"Zone Elite"`: `10`/`20` |
| `Elimination` (15 instances) | **Not identified** | Higher is better (`elite > pass` on all 15), but by what metric is unknown | Under 50 - not guessed |

**Why Elimination's own `BEAT_VALUE` metric is left unread.** Its targets
run `25`-`100`; `oag_race::Standing::kills` (the only per-race count this
engine's own Eliminator mode tracks) never approaches those numbers in a
real match, and nothing else - a damage total, a points score - is tracked
at all. `evaluate_tier` returns `None` for this one case rather than
grading against a metric with no evidence behind it - see that function's
own doc comment. An Elimination event still plays and still unlocks
whatever names it as a prerequisite once finished; it simply never carries
a medal on this build, which is an honest, visible gap rather than a wrong
number.

Two objective instances go unused by any of the 141 event instances this
pass checked (`"Objective - Finish"`, `"MostDamage"` - the latter authoring
neither a type nor a target at all) and `"BeatPersonalBestLap"` (type `2`,
target `0`) is likewise never referenced by a `RACE_A`/`RACE_B`/`ELIM`/`ZONE`
instance's own `M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE` - three orphaned
objectives, left unread rather than guessed at.

**`HardcorePass2048` is not a third medal tier.** `NEWGUI/Skin.xml`
declares three pass-tier colour globals - `Pass2048` (green), `ElitePass2048`
(yellow) and `HardcorePass2048` (purple) - which reads at first like a
three-rung ladder. It is not: `M_PSECONDARYOBJECTIVE`/`M_PTERTIARYOBJECTIVE`
are real field names in the schema (found by grepping every distinct
`M_*OBJECTIVE*`/`M_*PASS*` field name in the raw file) but appear on **zero**
of the 141 event instances - measured, not merely unobserved in a sample.
2048's own authored medal law is genuinely two-tier; `HardcorePass2048`
most plausibly names a Hardcore *difficulty* flag this pass found no
authored data for, not a rung this build's own map or persistence ever
needs to draw. See `oag_ui::frontend::campaign_map`'s own "Progression"
section for where this lands in the draw code.

## `Data\xml\MP.xml` is not `SP.xml`'s schema

289 instances, but only `GameModeObjective` (59 of them) is shared with
`SP.xml`. The other 230 are two typedefs `SP.xml` carries none of:

| `typedefid` | count | fields | names |
| --- | --- | --- | --- |
| `1114956821` | 210 | `M_BASEOBJECTIVE`/`M_HARDOBJECTIVE`/`M_MEDIUMOBJECTIVE`/`M_PHARDLEVEL`/`M_PMEDIUMLEVEL`/`M_PNEXTLEVEL`/`M_PAR`/`M_X`/`M_Y` | `"MP_S12_E02"` and siblings |
| `425681076` | 20 | `M_PNEXTSEASON`/`M_PFIRSTLEVEL` | `"MP_Season_01"` and siblings |

A "level" shape with three difficulty-tiered objectives and a par time,
grouped into twenty season containers - multiplayer's own progression, not
read further this pass. `events`/`tracks`/`weapon_sets` find nothing in an
`MP.xml` document (they filter by `SP.xml`'s own typedef ids), rather than
misreading one shape as another.

## `Data\xml\MjolnirData.xml`: no schema, just a workspace list

212 bytes:

```xml
<mjolnir version="2634955210">
  <WORKSPACES>
    <WORKSPACE name="SP.xml"/>
    <WORKSPACE name="MP.xml"/>
    <WORKSPACE name="Profile.xml"/>
  </WORKSPACES>
</mjolnir>
```

The editing tool's own file list, not a type table. `Profile.xml` is not in
the base package - not chased this pass.

## The `CanvasLabel linkedevent` cross-reference

`data/plugins/frontend/NEWGUI/Definition.xml` (the campaign map's own front
end, see [2048-frontend.md](2048-frontend.md)) carries **68** `<CanvasLabel>`
elements. **47** of them carry a `linkedevent` attribute on their own
`<Values>` child (the `Values`-as-attribute-carrier convention
[fexml.md](fexml.md) documents), naming **45** unique event names, and
**all 47 resolve** to an `SP.xml` instance name - zero unresolved. Measured
with `oag_tables::fexml`/`oag_tables::mjolnir` directly (not a text search):
walk every `CanvasLabel` node, read `.value("linkedevent")`, look each
non-empty value up in the parsed `SP.xml` `Document`. The other 21
`CanvasLabel`s carry no `linkedevent` at all - other UI text, not event
tiles.

## What `M_X`/`M_Y` project to on screen is not in `SP.xml`, and is left open

`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md` recovered a
scale/bias/offset table (`X scale 3.0, Y scale 2.0, X bias 55.0, Y bias
25.0`, plus an 8-entry per-type pixel offset, into a 401x120 bounded area) -
but that table belongs to the **DLC** `HD CAMPAIGN`/`FURY CAMPAIGN` tiers'
own `FE3DCanvas` hotspots (`TouchCampaignHD_Item`/`TouchCampaignFury_Item`),
an explicitly different node set from this campaign's native events. Neither
`MjolnirData.xml`, `NEWGUI/Definition.xml` nor anything else checked this
pass authors an equivalent projection for a native `M_X`/`M_Y` pair - the
`CanvasLabel`'s own `Values x=".." y=".."` (front-end pixel-ish coordinates)
and the linked event's own `M_X`/`M_Y` (campaign-grid coordinates, e.g.
`"2048 - Event 3"` is `x="5" y="5"`) are visibly different number spaces with
no authored formula connecting them found in the data. Left open rather than
fitted; per this lane's own scope, no Ghidra session was opened to chase it
further.

## Craft choice: forced on some events, restricted on others, never through `WOShipCreatorParams`

**2026-09-28, answering the user's own play observation directly**: 2048 is
more restrictive about craft than the earlier "which is not the mechanism"
line below assumed. `WOShipCreatorParams` (referenced by every event's
`M_PPlayerShipCreatorParams`/`M_PGridShipCreatorParams`) is authored on
**zero** of `SP.xml`'s 141 events, player or grid - confirmed exhaustively
this pass, not just "every observed reference is empty" on a sample. The real
mechanism is two fields `GameModeBase_RegisterFields`
(`docs/ghidra/functions/vita-2048-eu-v104/game-mode-base-fields.md`) already
names but no prior pass had read at runtime:

- **`M_PPLAYERSHIPMODELDATA` forces one specific craft**, authored on 14 of
  the 141 events: all five per-team `"* P Ship Challenge"` events (the team's
  own craft - `"AG-Systems P Ship Challenge"` forces `AG_System_Proto`,
  `"Qirex P Ship Challenge"` forces `Qirex_Proto`, and so on), all four
  `E3_Demo*` builds (Feisar Speed) and five ordinary numbered events
  (`"2048 - Event 4-2"`, `"2049 - Event 3-3"`, `"2050 - Event 3-4"`, `"2050 -
  Event 6-1"`). `WOShipModelData`'s own `M_TEAM`/`M_LIVERY` resolve onto a
  `race::Options::team` id through `oag_2048::campaign::craft::{team_id,
  variant_index}`: `M_TEAM` matches `oag_2048::race::NATIVE_TEAMS` exactly,
  and `M_LIVERY`'s four values (`"combat"`, `"agility"`, `"speed"`,
  `"prototype"`) map onto `oag_2048::race::SHIP_TYPES`' own slots in that
  order - **not a string match**: `"combat"` is `SHIP_TYPES`' `"fighter"`
  slot (Feisar and Qirex spell the slot differently, see that constant's own
  doc comment), corroborated by `GameModeBase_RegisterFields`' own four
  prevent-flag offsets below landing in the identical order. `race::load_event`
  applies this unconditionally, overriding the caller's own team pick -
  confirmed live: `--race --event "2048 - Event 4-2"` reports `"2048 - Event
  4-2" forces the player craft: Qirex2048\1`, `--event "AG-Systems P Ship
  Challenge"` reports `AG_Systems2048\4`, and `--event "2048 - Event 1"`
  (no `M_PPLAYERSHIPMODELDATA`) reports nothing, team untouched.
- **`M_bPreventCombatShips`/`M_bPreventAgilityShips`/`M_bPreventSpeedShips`/`M_bPreventProtoShips`
  restrict choice to a subset of `SHIP_TYPES`** without forcing one, authored
  `true` on 6 events - none of which also forces a craft: `"2050 - Event
  3"`/`"2050 - Event 3-2"` (no Combat/Agility), `"2050 - Event 5"`/`"2050 -
  Event 5-4"`/`"2049 - Event 2-3"` (no Agility/Speed) and `"2050 - Event 7"`
  (no Combat/Speed) - all five bar one in the "2050" story arc.
  `oag_2048::campaign::craft::restriction` reads the four flags into a mask;
  `race::load_event` reports which categories an event forbids but **does
  not enforce the restriction**, because which native screen applies it
  (greying a tile, clamping the cursor, refusing the launch) was not found in
  `eboot.elf` this pass - `Frontend/Screens/TeamSelection_Screen.cpp`'s own
  constructor and the two screens that redirect into it
  (`TeamSelectRedirectPlayer1`) were read and neither touches these four
  offsets or `+0x3c`; the actual read site is elsewhere in that screen's own
  method table and was not chased further. Confidence 90 for the flags
  themselves (structural: four boolean fields, offsets `0x80`-`0x83`,
  authored on a small, coherent subset of events); confidence under 50 for
  how the original enforces them, so nothing is guessed there.
- **`M_PGRIDSHIPMODELDATA`** (capacity 7) sizes the AI grid explicitly on most
  numbered events - a second, larger finding this pass surfaced but did not
  wire: it would replace `oag_game::livery::teams_for_slots`' own "chosen, not
  measured" grid assignment with authored data for every event that carries
  it. Left as a next step. One event, `"Pirhana P Ship Challenge"`
  (`RACE_A`, 2 laps), authors an **empty** grid alongside its forced player
  craft - a solo or ghost run, not measured further this pass.

This falsifies neither half of the user's own observation: the disc really
does both force a specific craft on some events and restrict the category on
others, measured directly off `SP.xml` rather than assumed from
`WOShipCreatorParams`'s existence.

**Correction to "The unlock graph" above**: all five `"* P Ship Challenge"`
and all ten `"* - S Phantom Challenge"` side events - named there as "open on
a fresh save" - also carry `M_RankRequired` (`10`-`50` for the Ship
Challenges, `25`-`34` for the Phantom Challenges), unread and unenforced by
this engine (there is no player rank at all in this build). "Open on a fresh
save" is still correct for the unlock-graph gate specifically - nothing
incoming or `M_PEventRequired` blocks them - but the original likely also
rank-gates them, a second, independent gate this project does not model.
Live verification of the rank gate was not attempted:
`data/extracted/vita/PCSF00007/base/savedata` ships empty (no progressed
profile), and reaching rank 10 (Feisar's own Ship Challenge, the lowest of
the five) needs playing through the campaign, out of this pass's scope.

## The launch: `crates/game/src/race/load/campaign.rs`

`race::load_event(options, event_name)` resolves `event_name` against
`SP.xml`, overrides `options.track`/`class`/`mode`/`laps_override` on a clone
of `options`, and calls the existing `race::load` unchanged. **A second entry
point rather than a new `Options` field** - `Options` is built exhaustively
(every field named, no `..Default::default()`) at dozens of call sites across
this workspace, and a new required field would have touched every one of
them. `oag-game`'s own `--event <NAME>` flag drives it through the one call
site that runs `--race`'s headless load
(`crates/game/src/main/headless.rs::run_race`).

Verified against the real EU v1.04 package:

- `--event "2048 - Event 3" --ticks 300 --screenshot` resolves to **Park**
  (`M_TRACKDEF` `-1892961298`, `METRO PARK`), **Flash** (`eClass` `1`), **2
  laps**, mode **`single_race`**, a full AI grid (`Arcade_HUD.xml` loads,
  matching the mode) - the screenshot shows the player's yellow Feisar craft
  on Park's own start grid, opponents visible ahead.
- `--event "2048 - Event 4-2"` (an `Elimination` event) loads
  `Elimination_HUD.xml` and a real course end to end.
  **`oag_race::Mode::Eliminator` is a real, implemented mode** -
  `crates/game/src/race/eliminator.rs`, a kill-count race end in `tick.rs` -
  and 2048's own `TITLE.weapons.elimination` already names
  `Data\XML\weaponstats_Elimination_2048.xml`, so none of `SP.xml`'s four
  event kinds is refused as unsupported; the CLI's own `--mode` help text
  omitting `eliminator` from its list is a stale string, not a real
  restriction.
- `--event "Nonexistent Event Name"` refuses cleanly: exit 1, `"Nonexistent
  Event Name" names no instance in Data\xml\SP.xml`.

**2026-09-21: `load_event` also resolves the event's own medal law.**
`race::Loaded::campaign_2048_event` carries the event's own name plus
`oag_2048::campaign::event_objectives(&doc, &event)` - `None` for an event
that authors neither `M_PASSOBJECTIVE` nor `M_ELITEOBJECTIVE` (see "The
objective law" above) - resolved once here, at load, rather than re-parsed
by `RaceStage::observation` at grading time. `crates/game/tests/
vita_2048_campaign_progress_ground_truth.rs` pins `"2048 - Event 1"`'s own
resolved pair (`FinishRaceAnyPosition`/`Win`) end to end against the real
package.

## What is not determined

- **`GameModeObjective`'s own semantics are mostly resolved - see "The
  objective law" above.** What remains open there: Elimination's own
  `BEAT_VALUE` metric (confidence under 50, not guessed), and three
  orphaned objective instances (`"Objective - Finish"`, `"MostDamage"`,
  `"BeatPersonalBestLap"`) that no event references at all.
- **Which `GameMode_*` C++ class each of the four event typedefs
  instantiates.** See [Four more typedefs carry no name in the file at
  all](#four-more-typedefs-carry-no-name-in-the-file-at-all).
- **`WeaponType`'s bit layout.** `WeaponSetDefinition`'s own
  `M_WEAPONAVAILABLEBITS` is a raw value per instance (`"Rockets Only"` is
  `1`); which bit is which weapon is not chased.
- **`WOShipCreatorParams`.** Referenced by every event's
  `M_PGridShipCreatorParams`/`M_PPlayerShipCreatorParams`, never itself seen
  as an instance in `SP.xml` - confirmed **zero** authored references across
  all 141 events, not a sample - so its own shape stays unread; it is not
  the craft-restriction mechanism (see "Craft choice" above,
  `M_PPLAYERSHIPMODELDATA`/`M_bPrevent*Ships` are).
- **Which native screen enforces `M_bPrevent*Ships`.** See "Craft choice"
  above - the flags themselves are measured, how the original applies them at
  the Team Selection screen is not.
- **The `M_X`/`M_Y` -> screen projection for a native event.** See
  [above](#what-m_xm_y-project-to-on-screen-is-not-in-spxml-and-is-left-open).
- **`MP.xml`'s season/level ladder.** See [above](#dataxmlmpxml-is-not-spxmls-schema).
- **`M_PGRIDSHIPMODELDATA`'s own authored grid, against `teams_for_slots`.**
  See "Craft choice" above - not wired into the AI grid assignment this pass.

## See also

- [Race campaign](race-campaign.md) - the `PI_Grid`/`PI_Cell` schema the DLC
  `HD CAMPAIGN`/`FURY CAMPAIGN` tiers use instead, on this same title.
- [2048 frontend](2048-frontend.md) - `NEWGUI/Definition.xml`'s own campaign
  map screen, `FE3DCanvas`, and the `CanvasLabel linkedevent` cross-reference
  measured above.
- [`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md) -
  the DLC tiers' own scale/bias projection, and why it does not apply here.
- [Handling stats](handling-stats.md) - `SpeedClass`'s four-wide ladder and
  why it was not widened for 2048's fifth rung.
