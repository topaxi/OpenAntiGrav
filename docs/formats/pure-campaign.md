# Wipeout Pure's campaign

**Status: read and built (2026-10-12), walked the way the disc authors it
(2026-10-12, `pure-campaign-authored`).** The main menu's top row on a Pure source
is **SINGLE PLAYER**, and it opens the disc's own chain of one-choice screens (mode,
class, league or tournament, circuit, team); the last screen before the team names an
event, which launches as a race, its medal is recorded in the profile, and the gates on
tournaments, leagues, circuits, Zone circuits and two teams read those medals. The
chain's order is read off the screens' own redirects (`oag_tables::pure_flow`), the
events are `oag_tables::pure_campaign`'s, the state machine is
`oag_game::campaign::pure_chain`, the screen is `oag_ui_screens::pure_chain`, and the
code that judges a result is [`campaign.md`](../ghidra/functions/psp-pure-usa/campaign.md).
An earlier build (same day) offered the events as one chosen list panel from a
`RACE CAMPAIGN` row; that panel is gone.

Pure's campaign is **a list of events, not a grid**. Pulse and HD author grids of cells; Pure
authors no `grid_NN.xml` at all. What the disc carries is two files, identical on the
EU and USA pressings (`pure_campaign_ground_truth`):

| File | Holds |
| --- | --- |
| `Data\Plugins\PI001\Definition.xml` (`Data.wad`) | 16 `PI_Track` (type `Race` 8, `Classic` 4, `Zone` 4), 3 `PI_League`, 4 `PI_Tournament`, 11 `PI_Team`, and the 17 `<Unlock>` rows that gate them |
| `<location>\stats.xml` per circuit (`FEData.wad`) | `RaceTimes` (a gold time per class, plus `Silver` and `Bronze` offsets), `ZoneMedals` (three counts), and `AIStats` |

`oag-wad cat <image>:PSP_GAME/USRDIR/Data.wad 'Data\Plugins\PI001\Definition.xml'`
prints the first. The 16 circuits' `stats.xml` all resolve at `<location>\stats.xml`
in `FEData.wad`; no circuit ships a `stats_reversed.xml`.

## What an event is

Per class there are 32 events, in this order (the order the front end offers the
modes): four **tournaments** (Alpha, Beta, Classic, Championship, which the language
table names Ascension), twelve **single races** (the three leagues' circuits), twelve
**time trials** (the same circuits) and four **Zone** circuits. Zone has no class, so
its four events repeat in every class's list and are one event each in the store. 160
events across the five classes, every one of which plans a race and names a circuit
that is on the disc (`pure_chain_walk_ground_truth`).

Each is an `oag_tables::race_campaign::Cell`, the type Pulse's grids are made of, so
the launch, the medal evaluation and the record are the campaign's own. The cell's
targets are the executable's law over the disc's figures:

| Event | Targets (what `Cell::evaluate_medal` compares) |
| --- | --- |
| Single race | place 1, 2, 3: gold, silver, bronze |
| Tournament | final standings rank 1, 2, 3, after every circuit, on the shared points table (8, 6, 5, 4, 3, 2, 1, 0) |
| Time trial | centiseconds: `RaceTimes[class]`, `+ Silver`, `+ Bronze` (seconds, times 100) |
| Zone | zone count, highest first: `ZoneMedals` gold, silver, bronze |

## Unlocks

Seventeen `<Unlock>` rows in three shapes, all read and evaluated
(`oag_tables::pure_campaign::Unlock::met`); a row passes when every condition it
names does:

| Row | Gates | Passes when |
| --- | --- | --- |
| `MedalCount="10"`, `25`, `40`, `60` | the four Classic circuits; the Classic league (10) and tournament (60) | the profile holds at least that many medals |
| `MedalCount="70"` | the `Medievil` team | the same |
| `Track="Zone 1" medal="bronze"`, and `Zone 2` silver, `Zone 3` gold | Zone 2, 3, 4 | the Zone event on that circuit holds that medal or better |
| `Track="Zone N" medal="Gold"`, N = 1..4 | the `Zone` team | all four |
| `Tournament="Alpha" class="asSelected" medal="Any"` | the Beta league and tournament; Beta (tournament) opens the Championship | any medal in that tournament **at the class being played** |

Two things the file does that a reader keyed on name would get wrong, both pinned by
the ground truth: a league and a tournament of one name carry different gates
(`Classic` is 10 as a league, 60 as a tournament), and `medal` is cased `bronze`,
`Gold` and `Any` in one file. A single race or a time trial on a circuit is gated by
its own rows **and** its league's.

A gated event is not listed until it opens (the original lists none it has not opened);
the Team Selection screen drops a gated team the same way. `--unlock-all` lifts every gate
(ours).

## The chain

Read off `Skin.xml` (`Main Menu`, `Single Player`) and `Selection_Definition.xml`
(`Class Selection`, `League Selection`, `Tournament Selection`, `Track Selection`,
`Zone Track Selection`, `Team Selection`), identical on the EU and USA pressings (`pure_flow_ground_truth`; the one difference is `Main Menu` itself: the USA disc has no `Teasers` row, `main_menu_ground_truth.rs`)
(`pure_flow_ground_truth`):

| Screen | Offers | Goes to |
| --- | --- | --- |
| `Main Menu` | Single Player, Multiplayer, Profile, On Line, Teasers | `Single Player` on the first |
| `Single Player` | Arcade (the disc's English "Single Race"), Tournament, Time Trial, Zone, Free Play | Arcade, Tournament, Time Trial and Free Play: `Class Selection`; Zone: `Zone Track Selection` |
| `Class Selection` | Vector, Venom, Flash, Rapier, Phantom | Tournament: `Tournament Selection`; every other mode: `League Selection` |
| `League Selection` | the three leagues | `Track Selection` |
| `Tournament Selection` | the four tournaments | `Team Selection` |
| `Track Selection` | the league's circuits | `Team Selection` (its back goes to `League Selection`) |
| `Zone Track Selection` | the four Zone circuits | `Launch Game`: no class, league or team |
| `Team Selection` | the teams | `Launch Game` |

Multiplayer branches in the same redirects (`Multiplayer Redirect`, `Network Check`)
are not followed. The Zone events are one each in the store, so Zone asks no class.

**What the original shows, captured on PPSSPP (fresh profile, 2026-10-12, `pure-psp-eu.chd`).**
Every screen is drawn on the white skin with the title pink at top left, the rows below it
**18 grid lines apart** (36 px in a 2x window, `FlowFiles::row_pitch`), the selected row in
the darker blue, an amber help line, and a `Confirm` / `Back` bar. Gated entries are
**not listed**, they are not drawn locked: `Class Selection` offers Vector and Venom (with the
class's stat graph on the right), `League Selection` and `Tournament Selection` offer Alpha
alone (with the league's emblem and its four circuits on the right), `Track Selection` lists
the four Alpha circuits (a map of the highlighted one, its race and lap record, length and
height), and `Team Selection` lists eight teams (Feisar to Harimau) with its four ratings.
**This build lists what the profile has opened and nothing else** (`Chain::view`), draws the rows
on the skin as Pure's own menu pages are drawn (the same frame, title and selected ink,
`oag_ui_screens::pure_chain::draw_list`), draws the right-hand art the disc authors for the focused row
([below](#the-art-on-the-right-hand-side)) and does not draw the help line. Chosen, not measured: the medal word and its inks at a row's right
edge, the context line (`SINGLE RACE - VENOM - ALPHA`) between the title and the first row,
and (not captured) what `Zone Track Selection` hides on a fresh profile: Zone 2, 3 and 4
are gated by medals and are hidden by the same rule. `Team Selection` is the race box's own
screen (`oag_ui_screens::picker`), with the disc's two gated teams dropped until they open;
a fresh profile there lists 16 teams here against the original's 8, because the other
eight are gated in the original's code, not in `Definition.xml` (unread).

**The modes.** Arcade is a single race, Tournament a tournament, Time Trial the time
trial and Zone a Zone event: each ends in one `Cell` (the tables above) that the
campaign's own launch plays and records. **Free Play** is the one mode that is not a
medal event: the disc names it and a "Free play session results" string and no medal law
reaches it, so this build races the chosen circuit and class as a single race whose cell
cannot award a medal (`oag_tables::pure_campaign::free_play_event`, targets 0/0/0) and is
never written to `[[campaign]]` (chosen, not measured). `Team Selection`'s Back, and the
end of any chain race, return to the screen that picked the event with the cursor on it.

## The art on the right-hand side

Each chain screen's picture is read off the disc and drawn beside the rows, following the
cursor (pad and pointer): `oag_game::campaign::pure_art` reads it, `Chain::art_key` names
it, the focused row decides which. **A census of every definition file Pure's `Skin.xml`
loads** (`Selection`, `Teaser`, `Options`, `Multiplayer`, `RecordGrid`, `Controls`, `Stats`,
`StartProfile`, `Demo`, `InGame`, plus `Skin.xml` itself; `Credits` and `Online` are localised and were not read) found a texture on the right-hand side in only these places:

| Screen | What it shows | Authored in | Drawn here |
| --- | --- | --- | --- |
| Main Menu, Single Player | **nothing** (PPSSPP capture; the top-right mark is the frame's) | - | nothing |
| Class Selection | the class's stat graph: `class_background.mip` at (250, 54) tinted `DesignColor2`, then `classgraph.mip` (`DesignColor`), `<class>graph3.mip` (`CrosshatchColor`) and `<class>graph2.mip` (`GraphColor`) at (252, 47), one `<Watch watch="Class" value="Vector">` per class | `Selection_Definition.xml` | yes |
| League Selection, Tournament Selection | the league's emblem, `Data\Tournaments\<name>\Images\<name>league.mip` at (246, 45) tinted `AlphaColor`/`BetaColor`/`ClassicColor`/`ChampionshipColor`; the same entry serves both screens | the entry's own `screen.xml`, named by `PI_League`/`PI_Tournament` `location` | yes |
| Track Selection, Zone Track Selection | the circuit's map (`Track_1`..`Track_3.mip`, two seconds each, in the slideshow `Team Selection` already plays) and its record panel | the circuit's `screen.xml` (`screen_z.xml` in Zone mode), named by `PI_Track` `location` | yes |
| Team Selection | the craft's still and its four ratings | the ship's `screen.xml` | the race box's picker, unchanged |
| Show Teasers | one of three 248x140 teaser stills at (220, 55) in a rule box, a `<Watch watch="Teaser">` per row | `Teaser_Definition.xml` | **no screen**: the main menu has no Teasers row here (`docs/architecture/menus.md`) |
| Options, Records, Controls, Stats, Profile, Multiplayer | none (only rules, scroll arrows and the tab bars) | - | nothing |

**`<Watch>` is a conditional group.** `watch` names a front-end global, `value` what it has
to hold: the global is the focused row of the `<Menu global="Class">`, the value an entry's
disc idstring (`Vector`, not the localised label). `oag_ui::screen::Screen::watches` holds
them; the reader keeps the images they contain. `enabletransition` and `transition` (Phantom's
spelling) are read as the seconds the picture takes to fade in, **chosen, not measured**;
which of fade or wipe the original does is unmeasured. Paint order is the file's.

**The panel's placeholders are the engine's to fill.** The circuit files author `Race Record`
and `Lap Record` as `00' 00' 00` and the Zone panel's counters as `00`, then name
`<DisplayRecords track="...">`. PPSSPP on a fresh profile (2026-10-12, `pure-psp-eu.chd`)
reads `0.00.00` and `0.00.00` for the two records, `25`, `20`, `15` for Zone's gold,
silver and bronze (the circuit's `<ZoneMedals>`, equal in `stats.xml` on both pressings), and
`0` for `Best Zone` and `Best Score`. `oag_game::campaign::pure_art::Fills` writes those:
a record is `oag_hud::format_lap_time` of the profile's quickest finished race and quickest lap
on that circuit, **chosen, not measured** (any mode and class: which rows the original reads
was not captured), and `Best Zone`/`Best Score` are left blank once a Zone result is stored
(how the original words them is not captured). Compared by eye at 960x544 against the same screens on PPSSPP
(software renderer), the class graph, the emblem, the map and the panel sit where the original's
do; the tests pin the authored positions on both pressings.

**Open, in the same capture.** The panel's labels and the help line are set in Pure's `Scroll`
and `Stats` roles, which both resolve to `LTe50325.fnt` (`PI000`'s `<Font>` blocks); this build
draws them in the menu face, so they are larger than the original's and crowd at the panel's
columns. The renderer's third glyph slot is unused on Pure and is where that face goes; the
same face is the `Confirm`/`Back` legend's, whose `Back` reads `BACK BUTTON` here.

## The class gate

A fresh profile's `Class Selection` lists **Vector and Venom** only (PPSSPP capture,
2026-09-10); the menu authors all five and code hides the rest. This build reads that as
data (`oag_title::Campaign::classes_at_start`, Pure: `["Vector", "Venom"]`) and hides the
other three. **What opens Flash, Rapier and Phantom is not read.** The original's reward
reveal names `Unlock Flash Class`, `Unlock Rapier Class` and `Unlock Phantom Class`
(`0x08a7c820`.., the strings after `Unlock Track` and `Unlock Liveries` in a table at
`0x08ab4a0c` the `Show Unlocks` reveal indexes at `0x08951538`), so the classes are unlocked
as rewards, but the code that decides a reward was not found (`DisableEntriesBitField`,
the menu attribute that hides entries, is parsed at `0x088c76a0`; who sets it for the class
menu was not traced). **Chosen, not measured:** a later class opens once the class before it
holds any tournament medal, the shape of the disc's own
`Tournament="Alpha" class="asSelected" medal="Any"` rows. `--unlock-all` offers all five.

**After a race.** Pure ships no EndRace screens this build reads, so a finished race
shows the built-in results table, and its Cross, Start or a click leaves it: a single
event returns to the screen that picked it (circuit, tournament or Zone circuit) with the
cursor on it and its medal shown; a tournament with a leg to go starts that leg instead
(the medal is judged after the last). Escape abandons a tournament.

**Walked windowed, 2026-10-12** (Xvfb, software Vulkan, `--autopilot --autopilot-skill ace`,
mouse only, Back by the secondary button): SINGLE PLAYER, Time Trial, Venom, Alpha, Vineta K,
`Team Selection`, a flown race, the built-in results table, a click out of it, and the chain
back on `Track Selection` with the cursor on Vineta K. Also a Vector time trial and a Vector
single race, each through the same path. The three runs **earned no medal**, each just short
(a Venom time trial 111.43 s against the 111 s bronze limit, Vector 130.47 s and 130.75 s
against 130 s, a single race finished fourth), so each wrote an empty `[[campaign]]` row; the
medal itself is walked headless (`pure_chain_walk_ground_truth`): SINGLE PLAYER answered to
Alpha's tournament at Venom, its four legs flown by the autopilot (places 5, 2, 3, 4; 18 points,
rank 3), the **bronze recorded** in the store, and the chain read again with Beta opened and Flash's
Beta still unlisted; and the same walk to a Vineta K Venom time trial (111.78 s against the 111 s bronze
limit: judged, no medal).

## Chosen, not measured

The medal word, its inks and the context line; what the screens omit (help line, art,
map); the opening rule for Flash, Rapier and Phantom; Free Play's plain race;
counting each event's best medal once toward `MedalCount`; racing a time trial at the
mode's default laps. See
[`campaign.md`](../ghidra/functions/psp-pure-usa/campaign.md#what-is-not-determined).

## 2048 and Omega

**Not checkable, and not the same shape.** 2048 and Omega share an asset lineage with
each other, not with Pure; their campaign is a grid (2048's `mjolnir` database,
[`2048-campaign.md`](2048-campaign.md)) and their front end is HD's. Nothing here
carries over, and nothing there reads a `PI_League`.
