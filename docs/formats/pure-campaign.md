# Wipeout Pure's campaign

**Status: read and built (2026-10-12).** `RACE CAMPAIGN` on a Pure source opens a
list of events for the class being played; each event launches as a race, its medal
is recorded in the profile, and the gates on tournaments, leagues, circuits, Zone
circuits and two teams read those medals. The reader is
`oag_tables::pure_campaign`, the rows are `oag_game::campaign::pure_list`, the screen
is `oag_ui_screens::pure_campaign`, and the code that judges a result is
[`campaign.md`](../ghidra/functions/psp-pure-usa/campaign.md).

Pure's campaign is **a list, not a grid**. Pulse and HD author grids of cells; Pure
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
that is on the disc (`pure_campaign_walk_ground_truth`).

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

A locked event is drawn dim with its requirement in the footer; the Team Selection
screen drops a gated team until it opens. `--unlock-all` lifts every gate (ours).

## The screen

A scrolling list under mode headings, a class line (`CLASS: VENOM`) stepped with
left and right, a medal word at each event's right edge (gold, silver and bronze
inked), `LOCKED` on a gated one. Pad, wheel, hover and click all work: a click on a
row confirms it, a click on the class line steps the class (the left third back), the
secondary button backs out. **The screen is this project's own**: the original walks
a chain of one-choice screens (class, league or tournament, circuit, team) and no
capture of a list exists to match. A confirmed event opens `Team Selection`
(Zone flies its own craft and skips it), then loads the race as a campaign cell, so a
later lane can fly Pure's original opponent in it the way Pulse and HD do.

`--menu-page main --menu-prompt pure-campaign[:N]` draws the list headless, `N`
indexing the class.

**After a race.** Pure ships no EndRace screens this build reads, so a finished race
shows the built-in results table, and its Cross, Start or a click leaves it: a single
event returns to the list with the cursor on it and its medal shown; a tournament with
a leg to go starts that leg instead (the medal is judged after the last). Escape
abandons a tournament. Walked on an Xvfb with `--autopilot`: a Vineta K Vector time
trial flown to a bronze (2:09.53 against 2:04 gold), recorded as
`timetrial/vineta k/vector`, shown on the reopened list; and a Vector Alpha
tournament's first leg handing on to Modesto Heights.

## Chosen, not measured

The screen, the colours and the row pitch; showing every class from the start (the
original's fresh profile offers Vector and Venom and gates the rest in code, not
data); counting each event's best medal once toward `MedalCount`; racing a time trial
at the mode's default laps; the five classes in `PureClass::ALL` order. See
[`campaign.md`](../ghidra/functions/psp-pure-usa/campaign.md#what-is-not-determined).

## 2048 and Omega

**Not checkable, and not the same shape.** 2048 and Omega share an asset lineage with
each other, not with Pure; their campaign is a grid (2048's `mjolnir` database,
[`2048-campaign.md`](2048-campaign.md)) and their front end is HD's. Nothing here
carries over, and nothing there reads a `PI_League`.
