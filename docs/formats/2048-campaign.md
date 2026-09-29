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
with `search_strings` in Ghidra over `eboot-vita-2048-eu-v104.elf`.

**2026-09-28: resolved, all four - the typedef ids are hashes of the type's
own name, the same convention `oag_formats::wad::hash_name` already
implements for this disc family.** Every one of the eight typedef ids
`SP.xml` carries - including the five already named in-file by their own
`type=`/`typedefid=` attribute - equals `oag_formats::wad::hash_name`
(case-folded CRC-32, `\` normalised to `/`) of the class's own name,
zero exceptions across all ten names checked:

| Name | `hash_name(name)` | Matches |
| --- | --- | --- |
| `GameModeBase` | `366306753` | the abstract typedef, already named in-file |
| `GameModeObjective` | `380278911` | already named in-file |
| `WOShipModelData` | `520725191` | already named in-file |
| `TrackDefinition` | `205052969` | already named in-file |
| `WeaponSetDefinition` | `-966434245` | already named in-file |
| `WOShipCreatorParams` | `-92967649` | already named in-file (a field's own declared type) |
| `GameMode_SpeedLapRace` | `-1915183557` | **[`typedef::RACE_A`]** |
| `GameMode_ArcadeRace` | `-1353052320` | **[`typedef::RACE_B`]** |
| `GameMode_EliminatorRace` | `1311982788` | [`typedef::ELIMINATION`], already suspected by instance names - now confirmed structurally |
| `GameMode_ZoneRace` | `1018671239` | [`typedef::ZONE`], likewise now confirmed structurally |

Confidence **92**: six already-named typedefs (in-file text) and four
previously-unnamed ones all land on their expected id through the identical,
already-implemented hash function, with zero collisions and zero misses -
not a single coincidental match, a closed cross-reference. The remaining two
class names, `GameMode_CheckPointRace` (hashes to `375161732`) and
`GameMode_ZombieRace` (hashes to `-1251488984`), match **no** typedef id
`SP.xml` carries - both classes ship in the executable but no campaign event
in this file instantiates either. Not chased into `MP.xml`, which carries
none of these four typedefs at all (see below) and is therefore not where
either would surface either.

**This settles the `M_PNEXTEVENT`-crosses-typedefs observation too, and
corrects the reading of it.** `"2048 - Event 3"` (`RACE_A` =
`GameMode_SpeedLapRace`) naming `"2048 - Event 4"` (`RACE_B` =
`GameMode_ArcadeRace`) as its own next event was read as evidence *against* a
mode split; it is not - it only shows the unlock graph chains across concrete
classes freely, which is unremarkable (finishing an ordinary race can
unlock a Speed Lap attraction and vice versa). The two typedefs really are
two different C++ classes, and the shape each carries in `SP.xml`
(`RACE_A`'s `M_MAXGHOSTSHIPS` and Speed Lap sentinel, `RACE_B`'s ordinary
1-plus-lap events) is exactly what a dedicated Speed Lap class versus a
generic Arcade Race class would author. `EventKind::from_typedef` still
merges both into `EventKind::Race` deliberately - `engine_mode` already
derives `"speed_lap"` vs `"single_race"` independently off the `laps ==
Some(0)` sentinel, so the concrete class name adds understanding, not new
gating logic this project needs to add.

**The technique is reusable**: any mjolnir typedef id can be checked against
a candidate name by hashing it with `oag_formats::wad::hash_name`, no Ghidra
session required once a candidate name is in hand (from a string search, a
header, or a guess worth testing) - useful for `MP.xml`'s own unnamed
typedefs or any future schema this project reads the same way.

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
| `Race` (13 instances, all `GameMode_SpeedLapRace`) | Total race time, in centiseconds | Lower is better (`elite < pass` on all 13) | 80 | `"2048 - Event 3"`: pass `13000` (130 s), elite `11000` (110 s) |
| `Zone` (10 instances) | The zone counter reached | Higher is better (`elite > pass` on all 10) | 80 | `"Zone Pass"`/`"Zone Elite"`: `10`/`20` |
| `Elimination` (15 instances) | **Points scored** - the original's own card words it `SCORE %d POINTS` (`FE_SCORE_POINTS`); what earns a point in the sim is not read | Higher is better (`elite > pass` on all 15) | 80 for the wording (2026-09-29: `GameModeObjective_FormatText`'s fall-through for a class with no override, `campaign-event-card.md`); the gameplay metric is still unimplemented |

**Why Elimination's own `BEAT_VALUE` metric is still not graded.** (2026-09-29: the wording says points, above.) Its targets
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
  `oag_2048::campaign::craft::refused_craft` resolves that mask onto the
  twenty native craft it actually forbids, including a prototype craft's own
  substitution (see below). Confidence 90 for the flags themselves
  (structural: four boolean fields, offsets `0x80`-`0x83`, authored on a
  small, coherent subset of events).

  **2026-09-28: the enforcement site is `GameModeBase_IsShipTypeAllowed`
  (`0x812b41da`), not `TeamSelection_Screen.cpp`.** The first pass's search
  focused on `TeamSelection_Screen.cpp` because the restriction reads as a
  Team-screen concern; the real caller is a different, previously
  undocumented native screen - the campaign map's own per-event card
  (`docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`), the
  same three-button, photo-backed screen `2048-frontend.md`'s own module doc
  photographed live but could not locate any authoring XML for. Its Launch
  button refuses (silently, no redirect) when the player's own current craft
  fails `GameModeBase_IsShipTypeAllowed`; its Change Craft button (drawn only
  when the event does **not** force a craft) redirects to `Team_Definition.xml`'s
  `team` screen - the same one `oag_ui::frontend::team` already implements,
  and **unfiltered**: no read of `GameModeBase+0x80`..`0x83` was found on
  that redirect, so the original's own Team screen offers every craft
  regardless of the pending event's restriction, exactly matching
  `oag_ui::frontend::team`'s existing shape. This project now enforces the
  same gate at the same layer: `oag_ui::frontend::campaign_map::Frontend::launch_selected_event`
  refuses the tap when the player's own current craft (`Frontend::team_choice`,
  or a `settings.race.team`/`variant` seed before the player has touched
  `Team` this session - see `CampaignMap::craft_seed`'s own doc) is in the
  event's own `MapEvent::refused_craft` set, precomputed at boot
  (`crates/game/src/boot/campaign2048.rs`) from `refused_craft`. A forced
  event never consults this set at all, the same precedence the original's
  own card applies (its Change Craft button does not even draw on one).
  `race::load_event` itself still only reports the forbidden categories on
  `Loaded::report` - the refusal lives at the map, not the loader, since
  `--event` is also the loader's own entry point from a headless capture and
  the ground-truth suite, neither of which has a map to refuse on.

  **A prototype craft is not a fifth, independent class.** `M_PROTOTYPELIVERY`
  (`WOShipModelData+0x25`, one field past `M_LIVERY`) names the class a
  team's own prototype craft "counts as" for this check alone: measured off
  the real `SP.xml`, `AG_System_Proto`→Agility, `Auricom_Proto`→Combat,
  `Feisar_Proto`→Speed, `Piranha_Proto`→Speed, `Qirex_Proto`→Combat. So
  Qirex's own prototype is refused by `"2050 - Event 7"` (no Combat, no
  Speed) exactly as `Qirex_Combat` is, and allowed by `"2050 - Event 5"` (no
  Agility, no Speed) exactly as `Qirex_Combat` is. Confidence 85 - the
  decompile is literal (`docs/ghidra/functions/vita-2048-eu-v104/game-mode-base-fields.md`'s
  own `GameModeBase_IsShipTypeAllowed` section) but this was not watched
  running against a live restricted event; the six restricted events all sit
  in the "2049"/"2050" story arcs and are locked on a fresh save, so the
  in-window refusal itself was verified through `oag-ui`'s own unit tests
  (`crates/ui/src/frontend/tests/wipeout2048.rs`) rather than a live capture.
- **`M_PGRIDSHIPMODELDATA`** (capacity 7) sizes the AI grid explicitly on most
  numbered events. **2026-09-28: wired.** `oag_2048::campaign::craft::grid_craft`
  resolves the field slot-by-slot (not through `Field::references`, which
  drops an unauthored slot's own position along with its empty value) onto
  `race::Options::grid_teams`, a **per-slot** override
  `crates/game/src/race/load/roster.rs::apply_grid_teams` overlays onto
  `oag_game::livery::teams_for_slots`' own "chosen, not measured" placement -
  slot by slot, not all-or-nothing: a slot this does not resolve (unauthored,
  or a dangling `WOShipModelData` reference - neither observed on the real
  file) keeps whatever `teams_for_slots` already gave it, and the loader's own
  report (`"grid: N of 7 AI slot(s) authored by the event's own
  M_PGRIDSHIPMODELDATA; ... left on the chosen, not measured, fallback"`)
  says which. Measured directly against the real EU v1.04 file: 55 of
  `SP.xml`'s 141 events author all 7 slots (every reference resolving to a
  real craft), 2 more (`"2050 - Event 3-4"`/`"2050 - Event 6-4"`) author only
  their own first slot, and 84 author no slot that resolves at all (most name
  no `M_PGRIDSHIPMODELDATA` field whatsoever, and `grid_craft` returns `[]`
  for those; one, `"Pirhana P Ship Challenge"` below, authors the field with
  a single empty slot instead) - either shape changes nothing at the overlay.
  Verified live: `--race --event "2048 - Event 6"` (all 7 slots authored)
  reports `"grid: 7 of 7 AI slot(s) authored..."` and `"grid liveries:
  feisar2048\3, Feisar2048\1, Feisar2048\1, Feisar2048\1, Auricom2048\1,
  Auricom2048\1, AG_Systems2048\1, AG_Systems2048\1"` - three Feisar, two
  Auricom, two AG_Systems, exactly `SP.xml`'s own authored roster; a
  `--screenshot` of the same run shows `POS 1/8` and multiple opponent craft
  on Mall's own grid. Pinned end to end by
  `crates/game/tests/vita_2048_campaign_grid_ground_truth.rs` (disc-backed,
  `#[ignore]`d) and by synthetic-fixture unit tests in
  `crates/2048/src/campaign/tests.rs` and
  `crates/game/src/race/load/roster.rs`. One event, `"Pirhana P Ship
  Challenge"` (`RACE_A`, 2 laps), authors an **empty** grid alongside its
  forced player craft - a solo or ghost run, not measured further this pass.
  **Not the same shape as an unauthored field**: its own `M_PGRIDSHIPMODELDATA`
  is present (`length="7"`) but carries a single `<ARRAY value=""/>` child, so
  `grid_craft` returns `[None]` rather than `[]` - `apply_grid_teams` still
  overrides nothing (a `None` never overwrites a slot), but the loader's own
  report now names this event too (`"grid: 0 of 7 AI slot(s) authored..."`)
  where a genuinely field-free event stays silent about the grid entirely.
  The race itself plays exactly as it did before this pass either way, on
  `teams_for_slots`' own fallback.

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

## The event card (2026-09-29)

Tapping an unlocked node on the campaign map opens the original's per-event
card before anything launches. **Evidence and every measured rectangle:
[campaign-event-card.md](../ghidra/functions/vita-2048-eu-v104/campaign-event-card.md)**,
checked against `data/reference/2048-frontend/14-campaign-map-event-card-unity-square.png`.
`oag_ui::frontend::event_card` builds it; `oag_game::boot::campaign2048`
resolves what it says. Every word and picture comes off the disc:

| On the card | From | Confidence |
| --- | --- | --- |
| Circuit name (`UNITY SQUARE`) | `TrackDefinition`'s `M_DISPLAYNAME` | 90 |
| Photo | `NewImages\trackscreens\<Name>.gxt`, `Name` the capitalised `M_TRACKNAME` (10 base circuits); decompile table `0x8151cc68` | 85 |
| Emblem | `NewImages\tracks\<track>.gxt` | 80 |
| `PASS` + objective line | `FE_PASS` and `M_PASSOBJECTIVE`'s type/target through `GameModeObjective_FormatText`: `FINISH` -> `SP_Objective_Finish`, `POSITION` -> `ER_FINISH_1ST/2ND/3RD/IN_POS`, `KILLS` -> `FE_ELIMINATE_OPP` | 85 |
| Lap glyph with the count | `callout/num_laps.gxt`, `M_NUMOFLAPS` | 80 |
| Class glyph | `speedclass/{d,c,b,a,ap}_class.gxt` by `M_SPEEDCLASS` ordinal `0`-`4` | 85 (2026-09-29: `FUN_81061274` maps ordinals `0`-`4` to the handles `FrontEnd_LoadCardTextures` fills with exactly `d`, `c`, `b`, `a`, `ap`; the caption `FUN_812b26cc` reads ordinals `0` and `1` both as `C CLASS`) |
| Three buttons | Launch, Back, Change craft (only when `M_PPLAYERSHIPMODELDATA` is unset) | 75 |

**What is drawn (2026-09-29, second pass):** the objective page (kind `0`), the
leaderboard page (kind `1`: three tabs and the no-network Personal panel,
`FUN_810540c4`) and the rules page (kind `4`: class, laps, forced craft and
the allowed craft classes, `FUN_810535fe`), in the order
`CampaignEventCard_BuildPageList` puts them, so a card with an objective, a
class and laps has the three dots of frame 14. The objective line is worded for
all 80 events that author one (`BEAT M:SS` for `GameMode_SpeedLapRace`,
`ZONE TARGET : n` for Zone, `SCORE n POINTS` for the rest - see
`campaign-event-card.md`'s `GameModeObjective_FormatText`). The kind line
under the title is the executable's (`GameMode_GetKindLabel`): `ZONE`,
`COMBAT` for Elimination, `RACE`, and for the timed class `SPEED LAP` on
button shapes `5`/`6` and `TIME TRIAL` otherwise - the 13 events with laps and
a `BEAT M:SS` pass are `TIME TRIAL` with the stopwatch icon (the map draws
`speed_mode` for the whole class, 53 events). Zone events draw the
`Zone<Name>` photo of their circuit: they do author `M_TRACKDEF` (all ten
resolve to a base circuit), so the earlier "Zone events author no circuit"
claim was wrong. The card opens on the rules page when the current craft is
refused.

**Not drawn, on purpose, with where to pick it up:** page kind `2` (trophy
and cup art: the image handles behind `FUN_81052fb4` are not located; button
shapes `1`, `2`, `9`-`11` and ten named events) and kind `3` (`FUN_81052810`,
`FE_PASS_TO_UNLOCK`; its `event+0x7c` is a runtime field, probably
unreachable); the weapon callout on the rules page (`FUN_810626ce`, so an
event with weapons has its icons laid out a row short); the elite-pass row
(`FUN_81055150`, event modes `3`/`4`) and the personal record row of kind `1`
(no per-event result is kept beyond the medal). The `<` / `>` arrows are page
arrows, not lap arrows.

**Panel opacity and the PASS medal need no change.** Frame 14's body panel
over the map's green blur reads `(219, 246, 212)` at `x=436`: the panels are
translucent white (`Transparent2048`, `0xc0ffffff`) in the original too, and
look opaque because the map behind them is blurred and paler than ours. The
first dot is `Orange2048` `(221, 88, 11)` and the others `Blue2048` on both. The
medal of an event not yet passed is the grey `Icon_no_pass_medal`; an earned
one draws the pass or elite medal.

**Locked nodes:** frame 14's README note says tapping a locked node opens its
card, and `CampaignEventCard_HandleInput`'s map-tap branch gates only on
`event+0x2d8 != 0` (`0x810f2164`); this build keeps refusing a locked event at
the map. Reading what `event+0x2d8` is would settle which is right.

**Chosen, not measured (no confidence):** the mode icon and kind line follow
the class ordinal recovered from the constructors, but which of the two
timed-class words shows is `M_BUTTONSHAPE`-driven and was only read, not
watched; which medal glyph an earned tier draws; square as the pad's Change
craft key; where the forced craft's team logo and class icon sit relative to
the item centre; the `--` on the personal panel of an event with no result
yet (drawn for every open or locked event, and not for a passed one, whose
best result this build does not keep).

**Capture a card:** `just play 2048 --until card:N:EVENT NAME --press cross
--ticks 2 --screenshot out.png` opens the named event's card on page `N`
(`--until card` alone is the selected event's first page).

## The launch: `crates/game/src/race/load/campaign.rs`

`race::load_event(options, event_name)` resolves `event_name` against
`SP.xml`, overrides `options.track`/`class`/`mode`/`laps_override` on a clone
of `options`, and calls the existing `race::load` unchanged. **A second entry
point rather than a new required `Options` field for any of those** - most
call sites across this workspace build `Options` through `..Options::default()`
rather than exhaustively today, but a handful (measured 2026-09-28: one,
`crates/game/src/main/prepare.rs`) still name every field, and a *required*
new one would have touched every one of them for a feature only this
campaign needs. `Options::grid_teams` (below) is the one field this pass did
add - safe as an addition precisely because it defaults to "no override" and
so only that one exhaustive call site needed a line naming it explicitly, not
dozens. `oag-game`'s own `--event <NAME>` flag drives it through the one call
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

## The weapon set gate: `WeaponType`'s bit layout, decoded where the data pins it

2026-09-28, updated same day. `WeaponSetDefinition`'s own `M_WEAPONAVAILABLEBITS` (typedef
`2139957613`, named `WeaponType` in-file, confidence 95) carries one raw
value per instance - 20 instances in `SP.xml`, each with a descriptive
`name=` (`"Rockets Only"`, `"Cannons, Missile, Plasma"`, `"EliminatorWeapons"`,
...). Which bit is which weapon was chased from the data alone first, per
this project's own methodology, and only the bits the data alone could not
settle went to `eboot.elf`.

**Step 1: what the 20 names, tabulated against their bits, force by
themselves.**

| Bits | Name(s) |
| ---: | --- |
| 1 | Rockets Only |
| 2 | Missile Only |
| 4 | Quake Only |
| 8 | Turbo Only |
| 32 | Cannons Only, NoQuake |
| 34 | Cannons and Missile |
| 64 | Autopilot only |
| 128 | Plasma Only |
| 162 | Cannons, Missile, Plasma |
| 170 | Can, Mis, Plas, Turbo |
| 768 | Mines Only |
| 800 | Cannons and Mines |
| 929 | TEST, CombatNoLeechQuakeMissile |
| 931 | CombatNoLeechQuake |
| 1023 | DemoWeapons |
| 1024 | Leech Beam Only |
| 1056 | Leech and Cannons |
| 1959 | EliminatorWeapons |

Eight bits are pinned at **confidence 95** by the "Only"-named singletons
alone, cross-checked against every multi-weapon value with no residue left
over (`"Cannons and Missile"` = `34` = `32 + 2`, `"Cannons, Missile,
Plasma"` = `162` = `128 + 32 + 2`, `"Can, Mis, Plas, Turbo"` = `170` = `128 +
32 + 8 + 2`, `"Leech and Cannons"` = `1056` = `1024 + 32`,
`"CombatNoLeechQuakeMissile"`/`"TEST"` = `929` = `1 + 32 + 128 + 768`,
`"CombatNoLeechQuake"` = `931` = `929 + 2`, `"EliminatorWeapons"` = `1959` =
`1024 + 512 + 256 + 128 + 32 + 4 + 2 + 1`):

| Bit | Weapon | Confidence |
| ---: | --- | ---: |
| 0 (`1`) | Rocket | 95 |
| 1 (`2`) | Missile | 95 |
| 2 (`4`) | Quake | 95 |
| 3 (`8`) | Turbo | 95 |
| 5 (`32`) | Cannon | 95 |
| 6 (`64`) | Autopilot | 95 |
| 7 (`128`) | Plasma | 95 |
| 10 (`1024`) | LeachBeam | 95 |

**Bits 8 and 9 (`256`/`512`) travel together in every instance, confidence 75
as a pair from the data alone.** Across all 20 instances the two are always
both set or both clear - `"Mines Only"` = `768` = `256 + 512`, and every
composite that includes it (`"Cannons and Mines"` = `800`, `"DemoWeapons"` =
`1023`, `"EliminatorWeapons"` = `1959`) carries the full pair, never one
half. Data alone could not split them into two independent bits. **Resolved
below**: `WeaponType`'s own declaration names them individually, `BOMB` = 8
and `MINE` = 9.

**Bit 4 (`16`) was not pinned by data alone, and stayed unnamed.** It is set
on `"DemoWeapons"` (`1023`) and clear on every one of the eight
singleton-named instances and on `"EliminatorWeapons"`/both `"Combat*"`
sets - a pattern that would fit a third non-damaging pickup alongside Turbo
(bit 3) and Autopilot (bit 6), and `Weapon::Shield` is specifically the only
one of 2048's eleven real, distinct pickups (see below) this reading's eight
confirmed bits plus the mines pair do not already account for. That was
suggestive, not a read of the enum's own declaration, so it stayed under the
70 threshold. **Resolved below**: bit 4 is `SHIELD`.

**`"NoQuake"` (`32`) is an authored anomaly, not evidence either way.** Its
value is identical to `"Cannons Only"`'s, which contradicts what the name
promises ("every weapon except Quake" would need most of the other bits
set too). The bit table above is read off the *value*, not the name, so
this project enforces what `"NoQuake"` actually carries (`Cannon` alone) and
flags the mismatch here rather than guessing at repair.

**No `WeaponSetDefinition` instance in `SP.xml` ever sets a bit above 10**
(the highest observed raw value is `1959`), so `Weapon::Repulser` and
`Weapon::Shuriken` have no bit at all in this reading - consistent with
`docs/gameplay/pickups.md` recording neither as implemented on any title.

**Step 2: `WeaponType`'s own declaration in `eboot.elf`, chased once data
alone hit its ceiling on bit 4 and the 8/9 split.** Rather than a runtime
consumer of `m_weaponAvailableBits` (none was found - see the wiring
paragraph below), the field's own reflection metadata gives a stronger
source: `WeaponType_RegisterValues` (`0x8100e650`,
[weapon-type-bits.md](../ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md))
is the enum's own name/ordinal registration, called from
`WeaponType_RegisterEnum` (`0x8101291a`), and `m_weaponAvailableBits`'s own
field registration (`FUN_812b4f00`) declares its type as literally
`"WeaponType"`. The eleven pairs it registers:

| Bit | Weapon | Confidence |
| ---: | --- | ---: |
| 0 | Rocket | 95 |
| 1 | Missile | 95 |
| 2 | Quake | 95 |
| 3 | Turbo | 95 |
| 4 | Shield | 95 |
| 5 | Cannon | 95 |
| 6 | Autopilot | 95 |
| 7 | Plasma | 95 |
| 8 | Bomb | 90 |
| 9 | Mine | 90 |
| 10 | LeachBeam | 95 |

All eight bits `SP.xml`'s own data already pinned at confidence 95 match
these ordinals exactly, zero exceptions - the cross-check that licenses
reading the remaining three (`Shield`, `Bomb`, `Mine`) at the same tier as
the eight, modulo `Bomb`/`Mine` sitting one tier lower (90, not 95) because
the shipped `SP.xml` data never actually exercises the two apart, so the
split is read directly off the enum but not independently corroborated by
data. **`Weapon::Shield` is bit 4** - resolving the suggestive-only reading
above to a direct declaration read. **`Weapon::Bomb` is bit 8, `Weapon::Mine`
is bit 9** - the *opposite* order from `pickup-icon-uv-table.md`'s
held-weapon id table (id 8 `FE_MINES`, id 9 `FE_BOMB`), a transposition trap
this page's own doc comment flags explicitly. See
[weapon-type-bits.md](../ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md)
for the full decompile and reasoning.

**Step 3, corroboration rather than resolution: `eboot.elf`'s own
held-weapon id order is a *different* enum from this one.**
[`pickup-icon-uv-table.md`](../ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md)'s
`Hud_UpdatePickupIcon` reading already recovered 2048's *held-weapon* id
order at confidence 80-90: `1` Rocket, `2` Missile, `3` Quake, `4` Turbo,
`5` Shield, `6` Autopilot, `7` Plasma, `8` Mines, `9` Bomb, `10` Cannon,
`11` LeachBeam - eleven real, distinct pickups, no Repulser or Shuriken.
That order **disagrees with `WeaponType`'s own bit order from id/bit 5
onward** (held-id `5` is Shield, `WeaponType` bit `5` is Cannon; held-id
`6` is Autopilot, `WeaponType` bit `6` is Autopilot too, but the two only
agree there by coincidence, since held-id `10` is Cannon, not bit 10's
LeachBeam) - so this is independent confirmation that 2048 ships these
eleven weapons as real, separately-tracked pickups, and nothing more; **it
is not what resolves `WeaponType`'s own bit assignment for bit 4 or the
Mine/Bomb split** - Step 2 above does that, off the enum's own declaration,
precisely because this table's ordering cannot be trusted to carry over (the
two enums already disagree from position 5 on). Checked directly:
`Data\XML\weaponstats_Race_2048.xml` weights all thirteen of Pulse's pool
(Shield included) for all four classes, so `Weapon::Shield` was already a
real, weighted 2048 pickup regardless of whether `WeaponType` bit 4 was
ever confirmed to be its flag - Step 2 now confirms it is.

**Wiring**: `WeaponSet::allowed_weapons` (`crates/tables/src/mjolnir/
campaign.rs`) decodes all eleven bits above (`WEAPON_BITS`) and nothing
else - `oag_gameplay::pickup::draw`'s own `allowed` parameter gates a
`Weapon Pad`'s draw to that list, wired from the event's own `M_WEAPONSET`
in `race::load_event` (`crates/game/src/race/load/campaign.rs`) onto
`Setup::allowed_weapons`. An event whose weapon set decodes to nothing
recognised (no weapon set authored at all, or `available_bits` sets no bit
in `WEAPON_BITS`) races unrestricted rather than this project guessing - the
report says which. **AI slots are gated the same as the player's - chosen,
not measured**, since no runtime consumer of this mask was found in
`eboot.elf` to read otherwise (see the wiring note in Step 2's own page,
[weapon-type-bits.md](../ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md):
the only runtime reference to `m_weaponAvailableBits`'s own field
registration found by string cross-reference is the schema registration
itself, and the enum's own declaration - the field/type reflection link -
is what settles the bits instead, the same "string only at registration,
reader unfound" wall `frontend-campaign-map.md`'s own `m_x`/`m_y` chase
already hit).

**How far bit 4 actually reaches the campaign**: of the 20 named
`WeaponSetDefinition` instances, `"DemoWeapons"` (`1023`) is the only one
that sets it. Of `SP.xml`'s 141 events, 36 carry a non-empty `M_WEAPONSET`,
and exactly 5 of those point at `"DemoWeapons"` - measured directly off the
real file, not estimated. Those five events are the only ones whose gate
changes behaviour from this fix; every other restricted event
(`"EliminatorWeapons"` at 17 events, both `"Combat*"` sets, etc.) already
excluded `Shield` and still does, since none of them set bit 4.

## What is not determined

- **`GameModeObjective`'s own semantics are mostly resolved - see "The
  objective law" above.** What remains open there: Elimination's own
  `BEAT_VALUE` metric (confidence under 50, not guessed), and three
  orphaned objective instances (`"Objective - Finish"`, `"MostDamage"`,
  `"BeatPersonalBestLap"`) that no event references at all.
- **Which `GameMode_*` C++ class each of the four event typedefs
  instantiates.** See [Four more typedefs carry no name in the file at
  all](#four-more-typedefs-carry-no-name-in-the-file-at-all).
- **2026-09-28: closed.** `WeaponType` bit 4 (`Shield`) and the `Mine`/`Bomb`
  split (bit 8 `Bomb`, bit 9 `Mine`) are resolved off `WeaponType`'s own enum
  declaration in `eboot.elf` - see "The weapon set gate" above, Step 2, and
  [weapon-type-bits.md](../ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md).
  All eleven bits are now decoded and wired.
- **`WOShipCreatorParams`.** Referenced by every event's
  `M_PGridShipCreatorParams`/`M_PPlayerShipCreatorParams`, never itself seen
  as an instance in `SP.xml` - confirmed **zero** authored references across
  all 141 events, not a sample - so its own shape stays unread; it is not
  the craft-restriction mechanism (see "Craft choice" above,
  `M_PPLAYERSHIPMODELDATA`/`M_bPrevent*Ships` are).
- **2026-09-28: closed** - `GameModeBase_IsShipTypeAllowed`, called from the
  campaign map's own event card. See "Craft choice" above and
  `docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`.
- **The `M_X`/`M_Y` -> screen projection for a native event.** See
  [above](#what-m_xm_y-project-to-on-screen-is-not-in-spxml-and-is-left-open).
- **`MP.xml`'s season/level ladder.** See [above](#dataxmlmpxml-is-not-spxmls-schema).
- **2026-09-28: closed** - `M_PGRIDSHIPMODELDATA` is wired into the AI grid
  assignment. See "Craft choice" above.

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
