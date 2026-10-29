# Custom campaign (`campaigns/<id>.toml` in a mod)

**Status: built (v1.0) for Wipeout Pulse, HD/Fury and Omega. Not 2048 or Pure.**
The project's own format, not a disc's. Decision:
[ADR-0077](../architecture/adr/0077-a-custom-campaign-compiles-to-the-titles-own-grid.md).
Schema: [`schemas/v1/campaign.tosd`](../../schemas/v1/campaign.tosd). Reader:
`oag_mod::campaign` (shape). Compiler: `oag_game::campaign::custom` (names, units,
locks). Worked example: `example-campaign/` in [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples) (mounted at `assets/mods/examples`).

A mod carries any number of campaign files under `campaigns/`, pilot files under
`pilots/`, and track files of this project's own format under `tracks/`. An event may
also race a track another installed mod carries. An installed campaign is listed by name on the page RACE CAMPAIGN opens
once at least one is installed. The page's first row is the disc's own campaign. With
none installed the menu is unchanged.

## A file

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/campaign.tosd", version = "1.0.0" }

[campaign]
id = "night-run"          # unique in the mod: lowercase letters, digits, '.', '-', '_'
name = "NIGHT RUN"        # the picker row
description = "..."       # optional: the row's second line
titles = ["wipeout-hd"]   # optional: narrows the mod's own `titles`
reveal = "hidden"         # "locked" (default: drawn with a lock) or "hidden" (absent until open);
                          # a [[grid]] may override it

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
```

## Rules and units

| What | Rule |
| --- | --- |
| Slot | Cell `gridN_x_y`, the name the title's screens place a hex by (`Cell::grid_coords`). Pulse, HD and Omega all draw the same field: x `0`-`6`, y `0`-`4` on even columns and `0`-`3` on odd ones, odd columns half a hex lower. A neighbour is one of the six hexes around a cell. An event in a slot the layout lacks is logged as unreachable when the campaign opens. |
| Track file | `file = "tracks/<name>.track.toml"` races a [track file](track-file.md) the mod carries (the manifest and the `.glb` beside it), the way `--track-file` and RACE REMIX's CUSTOM do. `file = "<mod id>:<path>"` races one in another installed mod, which must be for the same title; a mod that is not installed leaves the campaign out, and the log names it. The reference is the dependency: nothing in the manifest declares it yet. Cell Selection names it by the manifest's `track.title`; its records and ghosts key as `foreign:<track.id>`. **Folder mods only** (the reader takes a path on disk): in a `.zip` or `.oagmod` the campaign is not offered and the log says why. Not a tournament leg and not Zone. |
| Medal targets | A finishing place (race, tournament, head2head); **seconds** (time_trial, speed_lap; stored as centiseconds); a count (zone, elimination; more is better). |
| Absent tier | Never awarded: stored as `0` on a lower-is-better mode and `i64::MAX` on a counting one, values the evaluator can never meet. No `[medals]` awards nothing. |
| Rungs | Any of `easy`/`medium`/`hard` fills the cell's per-rung targets (HD's shape); an absent rung takes the flat triple. On Pulse the screen judges at medium. |
| Opening | `open = true`: open from the start. `requires`: every named event has at least `medal` (`any`, the default, `bronze`, `silver`, `gold`). Neither: the title's neighbour rule (a medal on this cell or a hex neighbour). A tier with no open event opens its first event without requirements. |
| Hidden | `reveal = "hidden"` leaves every unopened cell out of the screen: it cannot be drawn, selected, pointed at or confirmed. Per tier: a `[[grid]]`'s `reveal` overrides the campaign's. |
| Laps | Absent: what Pulse's own 236 cells author for the mode and class (3 Venom, 4 Flash or Rapier, 5 Phantom on a lap race, 7 on a speed lap; none on Zone or elimination). **Chosen** for a mod cell, so the screen never shows a lap mode as endless. |
| Class | Matched case-insensitively and stored as a disc cell writes it (`Venom`), so the screens show the title's own word. |
| Tier | `unlock_grid` and `required_points` are the original's own law (`grid_points_met`); a tier with neither is open. |
| Preset craft | Team Selection is skipped and the team flies its base hull. Cell `ship` holds the team and `ShipChoice` is `false`: **chosen**, those fields were never read on a disc cell. |
| AI | `original`: the recovered driver, at `difficulty` (its rung, else the screen's) and `skill`. `ours`: this project's driver at `difficulty`, flying `pilot` in every opponent slot. `--ai ours` still overrides. |
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
- Opponent craft variants (`teams` entries with a variant), AI craft overrides (top
  speed and so on), a pilot per slot: later minors, compatible additions to `field`.
- A flyer card shipped in the mod: only the disc's own cards are named.
- The in-game editor: the grid itself, saving into a slot (the plan's step 9).
- A track file in a zipped mod: the track reader takes a path on disk.

## Cross-check: 2048 and Omega

**Checked, differs.** Omega draws HD's grid screens and takes mod campaigns.
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
| Final (6 points on Opening) | hidden | Night Tour (tournament, our driver with the roster's `aggressive`), Last Stand (elimination, inline pilot), Octagon Trial (time trial on the bundled `tracks/practice-octagon.track.toml`, a bare loop with no art), Rapier Duel (head to head), Finale (needs a silver on tier 1's Hillside Sprint and the rapier duel) |

Hillside Sprint's medal times come from the autopilot on Pulse at Phantom (best lap
30.23 s on the padded Hillside); Octagon Trial's from its run at Venom (1:38.86 for three laps). Circuits
and teams are on Pulse's base disc (Pulse's Auricom comes with a DLC pack, so it is not
used).

`--menu-page grid-select=<mod id>/<campaign id>` and `cell-select@<tier>=<mod id>/<campaign id>`
draw a mod campaign's screens headless, as they draw the disc's.
`crates/mod/src/campaign/tests.rs` checks that the schema and the reader agree.
