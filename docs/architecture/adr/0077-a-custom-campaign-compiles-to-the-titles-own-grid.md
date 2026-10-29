# ADR-0077: A custom campaign is a mod file that compiles to the title's own grid

## Status

Accepted (2026-10-29). Builds on [ADR-0066](0066-the-menu-is-the-disc-plus-mods.md)
(mods, and the menu as the disc plus mods), [ADR-0064](0064-project-toml-files-declare-a-toml-schema.md)
(every TOML file of ours declares a schema) and [ADR-0058](0058-per-title-behaviour-is-title-data-with-provenance.md)
(per-title behaviour is `Title` data).

## Context

Every campaign this engine plays is a disc's own. The maintainer wants a campaign
format of the project's own, carried by mods, that covers what the Pulse, HD/Fury,
Omega and 2048 campaigns do between them: events opening after earlier ones,
a preset or free craft, medals per difficulty rung, tournaments, a chosen field and
AI. It uses race modes the engine already runs and nothing else. Pulse, HD/Fury and
Omega draw their campaign as a hex grid of `PI_Cell`s (`oag_tables::race_campaign`),
which the engine already reads, draws, launches and records. Pure draws a list.
2048 has the hex art but no hex screens.

## Decision

1. **A campaign is `campaigns/<id>.toml` in a mod**, kind `campaign`, checked
   against [`schemas/v1/campaign.tosd`](../../../schemas/v1/campaign.tosd). The
   format is [`docs/formats/custom-campaign.md`](../../formats/custom-campaign.md).
   `oag-mod` reads its shape and knows no title, mode or team
   (`oag_mod::campaign`). The game compiles it (`oag_game::campaign::custom`).
2. **It compiles to the title's own `Grid`/`Cell`**, slot-addressed: an event in
   slot `[x, y]` of tier `N` is the cell `gridN_x_y`, the name the screens already
   place by. The title's own screens, layouts and art draw it. What a cell has no
   field for (a preset team, the AI, the opponents, the requirements, a
   cross-disc circuit) rides beside the cell. One event per slot keeps the format
   writable by the grid editor that comes next.
3. **Titles are data.** `oag_title::Campaign::custom_campaigns` is `true` on
   Pulse, HD/Fury and Omega. A mod's manifest `titles` now filters (by
   `Title::slug`, e.g. `wipeout-omega-collection`), and so does a campaign's own
   `titles`.
4. **The menu only changes when a campaign is installed.** RACE CAMPAIGN then opens
   a page whose first row is the disc's campaign and whose other rows are the
   installed ones (`Definition::add_campaign_picker`, after the title's own
   trimming). With none installed the tree is byte-identical.
5. **Progress is namespaced, no schema change.** A mod campaign's medals are
   `[[campaign]]` rows under the title `<title slug>/mod:<mod id>/<campaign id>`.
   The per-track `[[record]]` row of a mod event keeps its time but not its medal.
6. **Refuse, never substitute.** A name the engine does not know (mode, class,
   difficulty, pilot key) fails compilation, naming the mod, campaign and event.
   A campaign that fails, or whose event names a disc this machine cannot open,
   is offered nowhere and named in the log. A team or roster pilot the source
   lacks refuses the event when it is confirmed, with a line on the screen.
7. **Opening rules.** `open = true` opens from the start; `requires` needs medals
   on named events. Both replace the title's neighbour rule for that cell. An
   event naming neither keeps the neighbour rule. `reveal = "hidden"` (per
   campaign, overridable per tier) drops unopened cells from the screen model,
   so the draw, pad, pointer and Confirm cannot reach them. A tier's points
   still count every cell.
8. **Circuits are the disc's, another disc's, or the mod's own.** `track`
   names a disc circuit (`<title slug>:<id>` for another disc's). `file` names
   a [track file](../../formats/track-file.md) the mod carries, raced through
   the same route as `--track-file`, or `<mod id>:<path>` for one another
   installed mod carries (a track-only mod). A track file is raced from a
   folder mod only, since its reader takes a path on disk.
9. **The example mods are found from a checkout.** The search path
   includes `assets/mods/examples`, relative to the working directory, as
   `data/mods` is; the repository mounts github.com/OpenAntiGrav/mod-examples
   there as a submodule, so no track or model file enters this repository's
   history. `just play` offers the example campaign, and an installed
   game does not. The examples are left out of the LAN content fingerprint.

Rejected:

- **A list screen like Pure's for every title.** It would leave the titles'
  own campaign screens unused and need art the discs do not ship for HD and Omega.
- **A separate records table.** A namespaced title in the existing rows is
  invisible to the original campaign's readers and needs no migration.
- **Resolving teams and circuits when the campaign compiles.** The menu is built
  before the race box's catalogue is complete, and a mod cannot know a DLC pack is
  mounted. Teams and circuits are checked at launch. Discs are checked at build,
  because a campaign the player could never finish should not be offered.

## Consequences

- 2048 is not covered: no hex screens to draw on. It needs a layout of the
  project's own (the plan's step 7).
- HD's `Campaign Selection` is skipped for a mod campaign, and its grid pages
  one tier at a time. A tier shows a flyer card only when it names one of the
  disc's (`flyer`). A card shipped in the mod is not supported.
- An absent `laps` takes the count Pulse's own cells author for the mode and
  class (3/4/4/5 by class, 7 on a speed lap). That keeps the screen from showing
  a lap mode as endless. It is chosen for a mod cell, not measured on HD or
  Omega.
- A track file event cannot be a tournament leg or Zone, and a zipped mod that
  carries one is not offered. Unzipping a mod to play it is the workaround.
- The cell's `Locked` and `ShipChoice` fields carry mod data the original never
  authored (`locked` from `open`/`requires`, `ship_choice = false` for a preset
  craft). Both are labelled as chosen in the format page.
- An absent medal tier is filled with a value the evaluator can never meet
  (`0` lower-is-better, `i64::MAX` counting) rather than a new evaluator branch.
  A reader of the raw cell sees those numbers.
- A cross-disc event swaps `race::Options::source`/`craft_source` as RACE REMIX
  does, and `Session::open_menus` puts them back. A path that left the race
  without reaching the menus would keep the other disc's source.
- An event's pilot flies every opponent: our driver only, set once after the
  race starts. Per-slot pilots, opponent craft variants and AI craft overrides
  are later minors.
