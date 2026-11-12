# Custom campaign (`campaigns/<id>.toml` in a mod)

**Status: built (v1.0) for Wipeout Pulse, HD/Fury and Omega. Not 2048 or Pure.** The in-game editor: Pulse only (below).
The project's own format, not a disc's. Decision:
[ADR-0077](../architecture/adr/0077-a-custom-campaign-compiles-to-the-titles-own-grid.md).
Schema: [`schemas/v1/campaign.tosd`](../../schemas/v1/campaign.tosd). Reader:
`oag_mod::campaign` (shape). Compiler: `oag_game::campaign::custom` (names, units,
locks). Worked example: `example-campaign/` in [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples) (mounted at `assets/mods/examples`).

A mod carries any number of campaign files under `campaigns/`, pilot files under
`pilots/`, and track files of this project's own format under `tracks/`. An event may
also race a track another installed mod carries. An installed campaign is listed by name on the page RACE CAMPAIGN opens
once at least one is installed; on Pulse that page ends with a NEW CAMPAIGN row (the editor,
below). The page's first row is the disc's own campaign. With none installed RACE CAMPAIGN
stays as it was, one press into the disc's campaign; on Pulse its SQUARE opens the editor
instead, named in the footer's Square prompt (NEW CAMPAIGN) while the row is selected.

## A file

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/campaign.tosd", version = "1.0.0" }

[[medal-tier]]            # optional, best first: the medals this campaign awards
name = "platinum"         # lowercase letters, digits, '-', '_': what targets, requirements and records name it by
points = 4                # what a cell earned at this tier adds to its grid's points
colour = "#9fe3ff"        # the swatch colour, #RRGGBB
string_id = "MY_PLATINUM" # optional: the tier's word, from the mod's strings/english.toml; absent shows the name in capitals

[campaign]
id = "night-run"          # unique in the mod: lowercase letters, digits, '.', '-', '_'
name = "NIGHT RUN"        # the picker row
description = "..."       # optional: the row's second line
titles = ["wipeout-hd"]   # optional: narrows the mod's own `titles`
reveal = "hidden"         # "locked" (default: drawn with a lock) or "hidden" (absent until open);
                          # a [[grid]] may override it
difficulty = "medium"     # optional: the opponents' base difficulty for every cell that names none
progression = "ramp"      # optional: "flat" (default) or "ramp"; see "Difficulty" below

[[grid]]                  # a tier: one page of the title's grid, at most 16
id = "opening"            # unique in the campaign
name = "Opening"          # optional, for the report
required_points = 4       # optional: points on `unlock_grid` needed to open this tier
unlock_grid = "warmup"    # optional: absent opens the tier from the start
reveal = "locked"         # optional: this tier's own, in place of the campaign's
flyer = "01_uplift"       # optional, HD/Omega: one of the disc's own flyer cards

[[grid.event]]
slot = [2, 1]             # the hex slot [x, y], 0..=9 each, one event per slot
id = "gate"               # optional: what `requires` names; default "<grid id>/<x>-<y>"
name = "Gate Run"         # optional, for the report
mode = "race"             # race | time_trial | speed_lap | zone | elimination | tournament | head2head
track = "02_Track"        # a disc circuit id; "<title slug>:<id>" for another disc's
# tracks = ["01_Track", "02_Track"]   # a tournament's legs instead of `track`, one disc
# file = "tracks/lagoon.track.toml"   # a track file the mod carries, instead of `track`
# file = "oag.hillside:tracks/hillside.track.toml"  # or one another installed mod carries
class = "venom"           # a speed class of this title; optional on zone
laps = 3                  # optional: the disc's own count for the mode and class when absent
weapons = false           # optional: the mode's own default when absent
craft = { team = "Feisar" }           # optional: preset craft, Team Selection is skipped
unlock = { requires = [{ event = "first", medal = "silver" }] }  # or { open = true }

[grid.event.field]        # optional
ai = "original"           # "original" (default) or "ours"
difficulty = "hard"       # original: easy | medium | hard; ours: novice | skilled | elite | ace
skill = 1.75              # original only: the track's skill-curve position, as Pulse's `skill`
pilot = "rival"           # ours only: the mod's `pilots/rival.toml`, else the roster's
teams = ["Qirex", "Piranha"]          # the opponents' teams, grid slot 1 first

[grid.event.medals]       # optional; a tier left out is never awarded
gold = 1                  # flat: every rung
silver = 2
easy = { gold = 2, silver = 3, bronze = 5 }   # optional per-rung overrides
extra = [{ tier = "platinum", target = 1 }]   # a tier of your own: one row per tier other than gold, silver, bronze
```

## Rules and units

| What | Rule |
| --- | --- |
| Slot | Cell `gridN_x_y`, the name the title's screens place a hex by (`Cell::grid_coords`). Pulse, HD and Omega all draw the same field: x `0`-`6`, y `0`-`4` on even columns and `0`-`3` on odd ones, odd columns half a hex lower. A neighbour is one of the six hexes around a cell. An event in a slot the layout lacks is logged as unreachable when the campaign opens. |
| Track file | `file = "tracks/<name>.track.toml"` races a [track file](track-file.md) the mod carries (the manifest and the `.glb` beside it), the way `--track-file` and RACE REMIX's CUSTOM do. `file = "<mod id>:<path>"` races one in another installed mod, which must be for the same title; a mod that is not installed leaves the campaign out, and the log names it. The reference is the dependency: nothing in the manifest declares it yet. Cell Selection names it by the manifest's `track.title`; its records and ghosts key as `foreign:<track.id>`. **Folder mods only** (the reader takes a path on disk): in a `.zip` or `.oagmod` the campaign is not offered and the log says why. Not a tournament leg. A `zone` event takes one too: the file's loop is the Zone circuit, flown with the title's Zone craft, speed ramp, HUD and sound ([track-file.md](track-file.md#zone-mode-on-a-track-file)). |
| Medal targets | A finishing place (race, tournament, head2head); **seconds** (time_trial, speed_lap; stored as centiseconds); a count (zone, elimination; more is better). |
| Medal tiers | No `[[medal-tier]]`: the title's gold 3, silver 2, bronze 1, exactly as before. A list replaces it (a tier may be named `gold`, `silver` or `bronze` too; those three keep the title's words, its trophy and its jingle). The list is the order of quality: the first tier the result meets wins, and a better tier counts wherever a lesser one is asked for (a `requires` of `medal = "gold"` is met by `platinum`; a grid's gold-medal count includes every tier above gold; **chosen, not measured**). Targets name tiers by the `gold`, `silver`, `bronze` keys and `extra = [{ tier, target }]` rows (a table cannot take a key the schema does not list); a name the list lacks refuses the campaign by name. A grid's maximum points are, per event, the points of the best tier that event awards (an event with no platinum target is worth gold's 3 in a campaign whose platinum is 4), and the screen's points line is out of the same figure; Grid Selection's tier hex keeps the title's rule by position (everything earned: the best colour, halfway: the second, the required points: the third) (**chosen, not measured**). A tier with no gold, silver or bronze name stands in for the nearest standard tier below it in the list, else the last above it, for the end-of-race trophy and jingle (**chosen, not measured**). Cell Selection's three target rows list the three best tiers the event awards, each in its tier's colour; an event that awards no platinum shows gold, silver and bronze. A record keeps the tier by name (`best_tier` in `[[campaign]]`, only when it is not gold, silver or bronze; `best_medal` keeps the class that stands in for it), so a file written before tiers had names reads as before. A mod's own `strings/english.toml` gives the words (`string_id`); English only. |
| Absent tier | Never awarded: stored as `0` on a lower-is-better mode and `i64::MAX` on a counting one, values the evaluator can never meet. No `[medals]` awards nothing. |
| Rungs | Any of `easy`/`medium`/`hard` fills the cell's per-rung targets (HD's shape); an absent rung takes the flat triple. On Pulse the screen judges at medium. |
| Opening | `open = true`: open from the start. `requires`: every named event has at least `medal` (`any`, the default, or a tier of the campaign: `bronze`, `silver`, `gold` unless it declares its own). Neither: the title's neighbour rule (a medal on this cell or a hex neighbour). A tier with no open event opens its first event without requirements. |
| Hidden | `reveal = "hidden"` leaves every unopened cell out of the screen: it cannot be drawn, selected, pointed at or confirmed. Per tier: a `[[grid]]`'s `reveal` overrides the campaign's. |
| Laps | Absent: what Pulse's own 236 cells author for the mode and class (3 Venom, 4 Flash or Rapier, 5 Phantom on a lap race, 7 on a speed lap; none on Zone or elimination). **Chosen** for a mod cell, so the screen never shows a lap mode as endless. |
| Class | Matched case-insensitively and stored as a disc cell writes it (`Venom`), so the screens show the title's own word. |
| Tier | `unlock_grid` and `required_points` are the original's own law (`grid_points_met`); a tier with neither is open. |
| Preset craft | Team Selection is skipped and the team flies its base hull. Cell `ship` holds the team and `ShipChoice` is `false`: **chosen**, those fields were never read on a disc cell. |
| AI | `original`: the recovered driver, at `difficulty` (its rung, else the campaign's, else the screen's) and `skill`. `ours`: this project's driver at `difficulty` (its level, else the campaign's), flying `pilot` in every opponent slot. `--ai ours` still overrides. |
| Difficulty | `[campaign] difficulty` is the base for every event that names no `difficulty` of its own in `[field]`; one that does keeps its own, whatever the campaign says. The word is a rung of the original driver (`easy`, `medium`, `hard`) or a level of ours (`novice`, `skilled`, `elite`, `ace`); an event takes the rung of the driver it flies, and a word from the other ladder keeps its relative place, **chosen**, rounding down on a tie: `easy` is `novice`, `medium` is `skilled`, `hard` is `ace`; `skilled` and `elite` are `medium`. An event with no `[field]` at all follows it too. Absent, with `progression = "flat"`: nothing is set, and the screen's rung applies as it always did. |
| Progression | `flat`: every cell at the base. `ramp`: the first cell at the base, the last at the top of its driver's ladder (`hard`, `ace`), the cells between stepping up evenly and never down. **Cell order** is the tier's position in the file, then the event's position within the tier; every event counts, whatever its mode. A ramp with no `difficulty` starts at the bottom of the ladder. Per-tier and per-cell steps are not in v1.0: a cell that must differ names its own `difficulty`. |
| Pilot | A name looks in the mod's `pilots/<name>.toml` first, then the player's roster. A table written inline uses the pilot file's keys ([`pilot.tosd`](../../schemas/v1/pilot.tosd)). A pilot needs `ai = "ours"`. |
| Head to head | `head2head` fields one opponent: name its team in `teams` and its pilot in `pilot`. |
| Cross disc | `track = "wipeout-hd:03_Track"` races another title's circuit with this title's craft, the way RACE REMIX does. A campaign naming a disc this machine cannot open is **not offered**, and the log names it. |
| Progress | `[[campaign]]` rows under the title `<title slug>/mod:<mod id>/<campaign id>`. The disc campaign's rows are never touched. The per-track `[[record]]` keeps the time, not the medal. |

## Errors

The shape (header, unknown keys, ids, slots, references) is checked when the mod is
read. Names (mode, class, difficulty, AI, pilot keys) are checked when the campaign
compiles for the booted title. Either error names the mod, campaign and event, and the
campaign is offered nowhere. The log says why. A team or roster pilot the disc or
player lacks is checked when the event is confirmed: the event refuses and the screen
says so, and no other team or pilot is substituted.

## Not in v1.0

- **2048**: no hex screens to draw on (the plan's step 7). **Pure**: a list, not a grid.
- A progression other than `flat` and `ramp` (a step per tier, a ceiling below the top of the ladder).
- Opponent craft variants (`teams` entries with a variant), AI craft overrides (top
  speed and so on), a pilot per slot: later minors, compatible additions to `field`.
- A flyer card shipped in the mod: only the disc's own cards are named.
- The in-game editor on **HD/Fury and Omega**, and the editor's own limits (tournaments,
  editing a saved campaign, several tiers, unlock rules, laps, weapons, pilots): see below.
- A track file in a zipped mod: the track reader takes a path on disk.
- A track file as a tournament leg (a Zone `file` event is supported).

## The in-game editor (Pulse)

**Status: built for Wipeout Pulse (PSP and PS2). Open: HD/Fury, Omega** (their hex screens
draw flyer cards and per-rung targets; `oag_game::campaign::custom::editor_offered` is the
switch). The originals have no editor, and HD's `Racebox` is custom races, not this.

RACE CAMPAIGN, then NEW CAMPAIGN (the picker's last row), or SQUARE on RACE CAMPAIGN while no
campaign is installed, opens the title's own `Cell
Selection` with **every hex slot** drawn: a placed event filled, an empty slot dimmed. The
pad moves the cursor (CROSS places an event and opens its fields; LEFT and RIGHT or CROSS step
the selected field; UP and DOWN pick the field; TRIANGLE clears the slot; SQUARE names the
campaign; START saves; CIRCLE closes the fields, then leaves without saving). The mouse or a
finger clicks a hex to select it and the selected hex again to confirm it, hovers a field to move
the panel's row and clicks a row to step it or act, and the secondary button backs out. Hovering
a hex does not move the cursor, so crossing hexes on the way to the panel keeps the event.

| Field | Values |
| --- | --- |
| Mode | race, time trial, speed lap, Zone, Eliminator, head to head (a tournament names legs of its own and is left to the file) |
| Circuit | the booted disc's circuits |
| Speed class | the title's ladder (no class on Zone) |
| Team | the player chooses, or one of the roster's teams (a preset craft) |
| Opponents | the original driver, or ours (`ai = "ours"`); the cell names no `difficulty`, the campaign's governs |

SELECT (or the row SETTINGS) switches the details panel to the **campaign page**, which holds the
settings no one event owns:

| Row | Values |
| --- | --- |
| Difficulty | easy, medium (the default), hard: `[campaign] difficulty` |
| Progression | flat (the default), ramp: `[campaign] progression` |
| Seed | a number, 1 to start; LEFT and RIGHT step it, CROSS or a click types one (up to nine digits) |
| Class from, class to | the slowest and fastest speed class a filled cell may draw (the whole ladder to start with); a bound that passes the other moves it along |

FILL GRID (TRIANGLE on the campaign page) replaces **every** hex slot with an event drawn from the seed:
each cell draws its mode (the six above), its circuit and its speed class, in that order, from `oag_core::Rng`
seeded with the seed, a class per cell inside the bounds and none on a Zone cell. The same seed, bounds and
disc give the same grid, whatever was placed before. Every filled cell has the player choosing the team and
the original driver, is open from the start, and follows the campaign's difficulty (the editor writes no
`difficulty` on a cell, **including the `skilled` it used to write on a cell flown by our driver**). Clicking a
hex or CIRCLE returns to the event page, where a filled cell is edited like any other. The seed is not saved in
the file: the grid is.

SAVE writes the campaign (`CampaignDef::write`, which `parse` reads
back equal) as a **fresh folder mod** `<data dir>/oag/mods/<campaign id>[-n]/` with a
generated `oag-mod.toml` (id `player.<campaign id>`, `titles` this title, no licence) and
`campaigns/<campaign id>.toml`, on a worker thread. An existing folder is never touched: a
second save of the same name takes the next `-n`. The menus are then rebuilt the way a MODS
page switch rebuilds them, and the menus open on the picker, which lists the campaign at once. One tier `grid0`; every
event is `unlock = { open = true }`. Medals: the disc's own cell for the same mode, circuit
and class when it has one (seconds on a timed mode); for a finishing place (race, head to
head) 1, 2, 3 when it has none; none for a timed event with no cell of its own (its screen
reads `0:00.00` and it is never awarded, since a time exists per circuit and one invented
would be a guess), and none for Zone and Eliminator without a same-class cell. Laps and
weapons are the mode's own.

The compile path is ADR-0077's. The difficulty setting is a `[campaign]` addition to v1.0 (nothing is released, so
the format was edited in place), and the mod author skill (`oag-mod-author`) documents it. The editor cannot write
what the format cannot read. A disc cell's medal tier at `0` is left out of a placed event (a tier the format refuses
as a target and the original never awards), and a cell with all three at `0` gets no medals. Ground truth for the fill:
`custom_campaign_ground_truth::a_grid_filled_from_a_seed_launches_on_pulse` (and `_on_ps2_pulse`: three seeds,
every cell compiles and plans, the ramp never steps down) and
`campaign_cursor_ground_truth::a_grid_filled_from_a_seed_in_the_editor_is_saved_with_its_setting` (the pad through
the real session: the same seed twice writes the same grid, another seed another). Ground truth:
`custom_campaign_ground_truth::a_campaign_built_in_the_editor_launches_on_pulse` (model to
disc-compiled `grid0_x_y` cells that plan) and
`campaign_cursor_ground_truth::a_campaign_made_in_the_editor_is_listed_and_its_event_launches`
(the real session, headless: place, save, picker row, launch).

## Cross-check: 2048 and Omega

**Checked, differs.** The difficulty setting resolves in the title-independent compiler
(`oag_game::campaign::custom`), so an HD or Omega campaign takes it with the same rung mapping: **checked, applies,
not wired** for the editor (Pulse only) and without an HD or Omega disc-backed test (the unit tests compile on Pulse). Omega draws HD's grid screens and takes mod campaigns.
2048 has no `CellMode_Definition.xml` (`docs/formats/2048-campaign.md`) and takes none
yet. Its own event map is not reused. When it is covered it will draw the same compiled
grid, with hidden reveal standing in for its events appearing.

## Ground truth

`crates/game/tests/custom_campaign_ground_truth.rs` compiles the example, with
`oag.hillside` installed beside it, on Pulse's PSP and PS2 discs and checks that HD,
Omega, Pure and 2048 do not offer it. Every disc circuit must resolve on the disc, the
two track-file events must plan against their own files, every named team must be on the base disc's roster, every slot
must be one the title's Cell Selection draws, every cell must plan, and the tournament
must keep its three legs. HD's and Omega's path (flyer cards, their slot field) has no
disc-backed example since the example became Pulse-only; the compiler's unit tests cover
it. These tests, and the unit tests that read the example, return early and say so when
the `mod-examples` submodule is not checked out; CI checks it out with its LFS files.

## The worked example

`example-campaign/` in [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples) holds NIGHT RUN: two tiers, six
events and five, **for Pulse only** (the manifest's `titles`), beside `hillside/`, a
track-only mod (`oag.hillside`). The game run from the checkout (`just play`) finds
both, since it searches `assets/mods/examples` relative to the working directory, where
the repository mounts `mod-examples` as a submodule. An installed game does not.

| Tier | Reveal | Events |
| --- | --- | --- |
| Opening | locked | Warm-up (time trial, open), Gate Run (race, preset Feisar, named opponents, per-rung medals), Hot Lap (speed lap, neighbour rule), Duel (head to head against the mod's `rival` pilot, needs silver on the gate), Hillside Sprint (time trial on `oag.hillside:tracks/hillside.track.toml`, open from the start), Clean Race (weapons off, original driver on hard) |
| Final (6 points on Opening) | hidden | Night Tour (tournament, our driver with the roster's `aggressive`), Last Stand (elimination, inline pilot), Octagon Zone (a Zone event on the bundled `tracks/practice-octagon.track.toml`, a bare loop with no art), Rapier Duel (head to head), Finale (needs a silver on tier 1's Hillside Sprint and the rapier duel) |

Hillside Sprint's medal times come from the autopilot on Pulse at Phantom (best lap
30.23 s on the padded Hillside); Octagon Zone's medals are 24 / 18 / 12 zones, **chosen, not measured**: inside the 18 to 24 gold range Pulse's own 16 Zone cells author. The autopilot on the Octagon clears zone 24 in a little over four minutes and does not wreck until zone 626 (about 104 minutes, 2026-10-29), so it is a ceiling and not a target. Circuits
and teams are on Pulse's base disc (Pulse's Auricom comes with a DLC pack, so it is not
used).

`--menu-page grid-select=<mod id>/<campaign id>` and `cell-select@<tier>=<mod id>/<campaign id>`
draw a mod campaign's screens headless, as they draw the disc's.
`crates/mod/src/campaign/tests.rs` checks that the schema and the reader agree.
