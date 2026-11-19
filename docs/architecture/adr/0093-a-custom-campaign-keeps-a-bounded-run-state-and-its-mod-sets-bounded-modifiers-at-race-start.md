# ADR-0093: A custom campaign keeps a bounded run state, and its mod sets bounded modifiers at race start

## Status

**Proposed** (2026-11-19). A design for the maintainer to accept, change or reject;
nothing is built. **This ADR stays editable until it is merged into `main`, and
becomes immutable only then.** Until that merge, a review may change a decision
here in place instead of superseding it.

**Narrows** three statements without editing them, as
[ADR-0080](0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md) narrowed
[ADR-0066](0066-the-menu-is-the-disc-plus-mods.md):

- the "no replacing a recovered law or a disc table" row of
  [`wasm-extensions/README.md`](../wasm-extensions/README.md) section 3 (decision 4);
- the "a disc screen never gains behaviour a mod invented" clause of ADR-0066
  decision 4, ADR-0079 decision 6 and ADR-0080 decision 2 (decision 8);
- ADR-0080 decision 6, "the only output is one blob" of a screen (decision 7).

**Builds on** [ADR-0077](0077-a-custom-campaign-compiles-to-the-titles-own-grid.md)
(custom campaigns; not edited, the `[run]` table is an optional addition under
[ADR-0064](0064-project-toml-files-declare-a-toml-schema.md)),
[ADR-0079](0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md)
(the worlds), ADR-0080 (the frontend world, itself *Proposed*),
[ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md) and
[ADR-0091](0091-a-modded-race-records-its-inputs-and-its-header-names-the-modules.md)
(replays), and [ADR-0070](0070-the-network-transport-is-webtransport.md) (the
lobby). The surface, with signatures, ranges and the open questions, is
[`wasm-extensions/progression.md`](../wasm-extensions/progression.md).

## Context

A custom campaign today is a grid of independent events. Each race starts from the
same craft and the same pickups, and what a campaign remembers is the best medal per
cell, which is never downgraded. A mod author who wants a campaign whose races
change the craft and the pickups from race to race (a reward picked after one race
making the next race's engine stronger, a pickup that now does two things) has
nowhere to put the state and no way to act on it:

- the sim world's only state is a tick blob that dies with the race
  (`race-end` "Nothing persists past it in v1");
- `race-start`, `race-end` and `setup.tune` are in the README's sketch and not in the
  ABI (revision 6 exports `tick`, `oag_wants_race`, `oag_track`);
- `Ship::handling` and `Ship::weapon_counts` are load-time data the state hash
  deliberately skips, so a change to them is invisible to a replay's tick-0 check;
- the frontend world, the only place a player could choose a reward, is a Proposed
  design;
- the only per-tick levers are `scale-thrust` and the pickup commands.

Four facts shape any answer. The simulation must stay a function of the replay
header plus the inputs. A retry must not farm rewards. A mod must not be able to
write a value the disc authors, only scale it. A title's own bars are authored
ratings, not computed from handling.

## Decision

1. **A run.** A custom campaign that declares a `[run]` table has, per profile, one
   **run**: opaque bytes of at most `MAX_RUN_BYTES` = 1,024 (raw, before base64), a
   `u64` seed drawn when the run is created, an event ordinal, and a staged copy
   (`pending`). It is a table of the records `Store` keyed like the campaign's
   progress rows (`<title slug>/mod:<mod id>/<campaign id>`), saved by
   `records::save_in_background`. It is not the ADR-0080 tune, which is one blob per
   mod and title.
2. **Binding.** `[run] module` names a module the mod lists in `[extension.rules]`.
   Only that module sees the run, and only in a race launched from that campaign;
   the start description carries the campaign and event keys. The mod declares
   `simulation = true`, so every race of the campaign is MODDED.
3. **Two optional guest exports.** `oag_race_start` and `oag_race_end`, with a
   second host-written buffer a guest names by `oag_run`, the pattern of
   `oag_wants_race` and `oag_track`. `race-start` reads the run state and replies
   with commands; it cannot write the run. `race-end` reads the result (including
   the medal tier the host evaluated) and writes the new run bytes. ABI revision 7.
4. **Modifiers.** At race start the mod may send `set-modifier{slot, param, factor}`
   for a closed list of `Handling` parameters. The host owns each range, scales the
   slot's `Ship::handling` once, and records the applied factors in `Extensions`
   state, which is hashed after the `slots.is_empty()` early return, so no vanilla
   hash moves and a replay on other bytes fails at tick 0. An out-of-range factor is
   **clamped with a named load-report warning**, not a fault (departing from the
   other commands; open question 1 of `progression.md`). This is a bounded
   multiplier on the disc's own number, never a replacement value, and so narrows
   the README's "no replacing a disc table" row without contradicting its reason.
5. **Pickup parameters.** Per slot, at race start: `count` (rockets per press, mines
   per drop, cannon rounds), `Duration` (Turbo, Shield, Autopilot) and `Strength`
   (Turbo, which is `Param::TurboStrength`). Durations and the cannon's rounds are
   per-ship fields, never edits to the race-wide weapon tables. A combined pickup is
   a `Weapon::Extension` whose `pickup-fire` reply sends `give-shield`, the new
   `give-autopilot` and `clear-pickup`. One kind per mod stays; the recipe is
   selected from state.
6. **Stage and commit.** The finish site calls `race-end` once, and its output is
   staged together with the medal the race earned. It is committed when the player
   leaves the results for the campaign screens. RACE AGAIN and RESTART RACE discard it
   and restart from the committed bytes. Quitting before the finish stages nothing.
   Replay rebuilds, demos, unofficial runs and split screens never call `race-end`.
7. **A choice screen** is an ADR-0080 screen named by `[run] choice_screen`, opened by
   the host after a finish whose `race-end` asked for it. It receives the run bytes and
   may return replacement bytes, as a second allowed output beside `save`
   (narrowing ADR-0080 decision 6). The offers are rolled in `race-end` from the run's
   seed and ordinal, because the frontend world has no seeded RNG.
8. **Stats.** `[[run.stat]]` rows name a byte offset, a kind and a style in the run
   bytes. The host draws them and interprets nothing else. A campaign that declares
   them may add **display-only** rows and bonus segments to the stat panel of the
   Ship Select it launches, for that campaign only (the one allowance that narrows
   "a disc screen gains no mod behaviour"). A title bar is never recomputed from scaled
   handling.
9. **Replays, ghosts, records, network.** The replay header's `ModUse` carries the run
   input bytes, seed, ordinal and keys (base64, capped on raw bytes). Playback never
   reads the local store. There is no ghost (ADR-0091 decision 5). A race whose start
   applied any non-identity modifier, count or scale files no per-track `[[record]]`.
   Run state is single-player: a lobby race carries none and the module declines it.
10. **Build order.** The run record and the ABI first, then modifiers, then pickup
    parameters and `give-autopilot`, then the frontend world and the screen. A campaign
    can ship with `race-end` choosing the reward by itself before the screen lands.

**Not decided here:** whether a failed race ends a run, the AI's use of a mod pickup,
`TopSpeed` as a parameter, a data-first modifier table, a per-owner damage scale, the
page that shows stats when the craft is preset, and every other item in
`progression.md` section 12.

## Alternatives considered

- **Reuse the ADR-0080 tune blob as the run state.** Keyed by mod and title, so two
  campaigns of one mod share it and NEW RUN would clear the mod's unrelated settings.
  Also written only by a screen. Rejected (decision 1).
- **Keep the run in the sim module's tick blob.** It dies with the race; persisting it
  would make the host snapshot a blob the guest cannot tell from scratch state.
  Rejected.
- **Commit at the finish.** It is how medals work today, and it lets a retry farm
  rewards. Rejected for decision 6, at the cost of touching the two `record_result`
  call sites in `oag-game`.
- **Let `race-start` write the run state.** A retry re-applies a start-time write on
  top of itself. Rejected: `race-start` stays a pure function of (bytes, seed,
  ordinal, race).
- **Write modifiers straight into `Handling` and let the hash skip them.** Matches the
  current "load-time data" rule and lets a replay on other bytes drift unnoticed.
  Rejected (decision 4).
- **A raw "field index plus factor" command.** The host cannot say what a field means
  or choose a range, and an engine version can re-point an index. Rejected for a
  closed `Param` list.
- **A per-tick modifier command.** Needs a timer or re-sending and keeps the factor out
  of the applied record. `scale-thrust` already covers per-tick thrust. Rejected.
- **A 4 KiB run state.** Needs a buffer larger than `IO_LEN`, which every built guest
  assumes. Rejected for 1,024 and a second buffer.
- **The run in the tick buffer's state region.** At `race-start` the region is the
  module's own state and the output needs a second one. Rejected.
- **A screen that returns only the pick, with `race-start` applying it.** Keeps the
  transition in deterministic code, but the Ship Select rows must show the pick before
  the next race starts, and the pick would be a second replay input. Rejected.
- **A screen that rolls the offers.** No seeded RNG on the UI side. Rejected.
- **Stats on a mod screen's canvas only.** The player would see the craft and the
  title's bars on one screen and the mod's stats on another, and the canvas would need
  disc data a mod may not yet read (README open question 15). Rejected as the only
  path; kept as the fallback if the maintainer refuses the allowance in decision 8.
- **More pickup kinds per mod for several combined rewards.** Changes what the cap of
  32 means and renumbers the craft record codes. Rejected for one kind selected by
  state.
- **A host-typed run schema.** The host would own a schema mods must fit; the brief is
  numbers the mod interprets itself. Rejected.
- **A declarative modifier table as the only path.** The shape of a reward is the
  mod's point and a formula table grows into a poor scripting language. Rejected as
  the only path; possible on top (open question 7).

## Consequences

**Good.** A campaign can change the craft and the pickups from race to race with no
engine knowledge of what a reward is. A race with no run module hashes exactly as
before. A replay carries what it needs and fails at tick 0 on wrong bytes. A retry is
the same race. Every disc number stays the source of truth.

**Bad.**

- **The README overstates the built ABI.** Three of the pieces this depends on
  (`race-start`/`race-end`, `setup.tune`, the frontend world) are designs, not code.
  Capabilities 4 and 5 are blocked until ADR-0080 is accepted and built; if it is
  rejected they have no home.
- **Staging the medal changes shared code.** The finish site and the escape site in
  `oag-game` serve every campaign; a `[run]` campaign takes a different path through
  them, which is a regression surface for the disc campaigns.
- **A clamp where the convention is a fault.** An author can ship a module whose
  rewards are silently weaker than intended; only the load report says so.
- **Per-track records are lost for a modified race.** No leaderboard pollution, and
  no personal best either, until the modded records table exists.
- **A closed parameter list is a ceiling,** like the command set in ADR-0079. A
  parameter outside the list waits for a host change, and `TopSpeed` is on the list
  and refused until it is measured.
- **Hard engine bounds remain.** Three rockets and five mines are the sizes of hashed
  arrays; more is an engine change with its own proof.
- **A mod can scale the AI.** The AI obeys the player's physics, so a slot's factor
  works on it too; whether to allow that is open (question 11).
- **More weapon-lineage rows.** Each new per-ship pickup field and `give-autopilot`
  needs its row in `docs/gameplay/weapon-lineage.md` when built.
- **Run state is single-player.** Netplay needs a per-seat story, the same open
  question as the per-seat tune.
