# ADR-0093: A custom campaign keeps a bounded run state, and its mod sets bounded modifiers at race start

**Challenged, being answered:** [`wasm-extensions/progression-challenge.md`](../wasm-extensions/progression-challenge.md) lists 25 numbered objections to this proposal (5 block a decision) and a smaller first slice; each answered one says so under it.

## Status

**Proposed** (2026-11-19). A design for the maintainer to accept, change or reject;
nothing is built. **This ADR stays editable until it is merged into `main`, and
becomes immutable only then.** Until that merge, a review may change a decision
here in place instead of superseding it.

**Narrows** these statements without editing them, as
[ADR-0080](0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md) narrowed
[ADR-0066](0066-the-menu-is-the-disc-plus-mods.md):

- **Persistence from the sim world** (decision 1, the central one): ADR-0079
  decision 6 ("no persistence except a host-mediated blob"), the
  [`wasm-extensions/README.md`](../wasm-extensions/README.md) section 3 row "No
  persistence from the sim or HUD world" with its line that the sim guest "never
  writes" the saved blob, the same page's `race-end` comment "Nothing persists past it
  in v1", and ADR-0080 decision 7 (a blob reaches a race at `race-start` only, and
  nothing flows back). A sim `race-end` now produces bytes that persist. Why this
  stays within the reason for the rule: the record is host-owned, listed and clearable
  on the MODS page, one per campaign, committed by the host (never written
  by the guest directly), capped at 4,096 bytes, never read by a second mod, and
  single-player.
- the "no replacing a recovered law or a disc table" row of the README's section 3
  and the same words in ADR-0079 decision 6 (decision 4): a bounded multiplier on the
  disc's number is not a replacement.

**Split** (maintainer, 2026-11-20): the choice screen and the mod's stats, which lean
on the frontend world, are [ADR-0094](0094-a-campaign-run-offers-a-choice-between-races-and-shows-the-mods-stats.md). This ADR needs no frontend
world and does not narrow ADR-0080 decisions 2 or 6.

**Builds on** [ADR-0077](0077-a-custom-campaign-compiles-to-the-titles-own-grid.md)
(custom campaigns; not edited, the `[run]` table is an optional addition under
[ADR-0064](0064-project-toml-files-declare-a-toml-schema.md)),
[ADR-0079](0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md)
(the worlds),
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
   **run**: opaque bytes of at most `MAX_RUN_BYTES` = 4,096 (raw, before base64; maintainer, 2026-11-20), a
   `u64` seed drawn when the run is created, an event ordinal, and, only after a
   faulted `race-end`, the latch it still owes (decision 6). It is a table of the records `Store` keyed like the campaign's
   progress rows (`<title slug>/mod:<mod id>/<campaign id>`), saved by
   `records::save_in_background`. It is not the ADR-0080 tune, which is one blob per
   mod and title.
2. **Binding.** `[run] module` names a module the mod lists in `[extension.rules]`.
   Only that module sees the run, and only in a race launched from that campaign;
   the start description carries the campaign and event keys. The mod declares
   `simulation = true`, so every race of the campaign is MODDED. **Only that module
   gets `race-start` and `race-end`**: another module that is on still ticks as usual
   but is never called at a race's start or end, and outside a run campaign no module
   is, so one writer fills the slot table (maintainer, 2026-11-20).
3. **Two optional guest exports.** `oag_race_start` and `oag_race_end`, with a
   second host-written buffer a guest names by `oag_run`, the pattern of
   `oag_wants_race` and `oag_track`. `race-start` reads the run state, fills the
   slot table (decision 4) and **sets the module's initial tick state** (the only way the run reaches
   `tick`, which never sees the run bytes); it cannot write the run. **A fault in
   `race-start`** (a trap, spent fuel, a malformed reply, a non-finite factor) **refuses
   the launch**: the player is back on the campaign page with a line naming the mod and
   the fault, and no stock race runs in its place (ADR-0077's "refuse, never
   substitute"; maintainer, 2026-11-20). An out-of-range factor still clamps (decision
   4). `race-end` reads
   the result (including the medal tier the host evaluated, the event's best medal
   before this race as `prior_tier`, the event's position in the campaign as
   `event_index`, and a `score`: zones reached in Zone, kills in Eliminator) and the final tick state,
   and writes the new run bytes. ABI revision 7.
4. **Modifiers.** At race start the mod fills a **per-slot table** in the run buffer
   (one row per grid slot: a factor per parameter of a closed list of `Handling`
   parameters, with spare columns for later ones, the pickup counts and durations;
   **zero means untouched**, so an all-zero table is a stock race; maintainer,
   2026-11-20). The host owns each range, scales the
   slot's `Ship::handling` once, and records the applied factors in `Extensions`
   state, which is hashed after the `slots.is_empty()` early return, so no vanilla
   hash moves and a replay on other bytes fails at tick 0. An out-of-range factor is
   **clamped with a named load-report warning**, not a fault (departing from the
   other commands; settled by the maintainer, `progression.md` question 1). This is a bounded
   multiplier on the disc's own number, never a replacement value, and so narrows
   the README's "no replacing a disc table" row without contradicting its reason.
   The engine is two parameters, named for the two arms of the measured force law
   (`T = 2 * min(throttle * amount, 0.5 * speed + accelcap)`): **`Launch`** scales
   `engine.accelcap` (acceleration from rest) and **`TopEnd`** scales `engine.amount`
   (the plateau, so terminal speed). Each is refused until a held-throttle ground
   truth shows its effect. Neither acts in a Zone event, whose speed path reads
   neither field; that is accepted, Zone being its own mode (maintainer, 2026-11-20).
   Factors are applied **before the speed plans are built**: each modified slot, AI or
   person, counts as its own hull and adapts the reference plan under the existing
   40,000-step budget, so the AI, the Autopilot and Pilot Assist fly a plan for the
   craft they drive (maintainer, 2026-11-20).
5. **Pickup parameters.** Per slot, at race start: `count` (rockets per press, mines
   per drop, cannon rounds), duration (Turbo, Shield, Autopilot) and strength (Turbo
   only, which is `Param::TurboStrength` of decision 4). A `count` only goes down:
   it is clamped to `1..=` the craft's own count (the authored `number_of_rockets` and
   `number_of_mines` on 2048 and Omega, the defaults 3 and 5 elsewhere), so a
   campaign can start a craft on one rocket and level it back up to two and three,
   and its progression ends at today's limit. Below the craft's own number it is a
   reduction of the disc's value, not a replacement past it (decision 4's rule).
   Durations (Turbo, Shield, Autopilot, all three kept; maintainer, 2026-11-20) and the
   cannon's rounds are per-slot overrides held **only in the extension state**, never
   fields of `World` or `Ship` and never edits to the race-wide weapon tables: every
   grant site goes through one helper that reads the slot's override when a module is
   loaded and the table's value otherwise (decision 12). A combined pickup is
   a `Weapon::Extension` whose `pickup-fire` reply sends `give-shield`, the new
   `give-autopilot` and `clear-pickup`. One kind per mod stays; the recipe is
   selected from state. `give-autopilot` works on any craft, a modified one included,
   flying the slot's adapted plan.
6. **Commit on exit.** An event ends at the two sites that record a medal today:
   the finish site for a lap-count mode, the escape site for Zone, Speed Lap and an
   abandoned Time Trial. The race goes on as it does today (the world steps on under
   the results; Zone runs until the player leaves), and `race-end`, the mod's own
   award and end-of-run logic, runs **once, when the player exits the race**, not at
   the finish moment (maintainer, 2026-11-20). What it reads is **latched** inside the
   sim on the tick the person's race ends (the result fields and a copy of the module's
   tick state, plain data, no guest call); Zone and Speed Lap latch at the leave. So
   the same race and inputs give the same run bytes, however long the player stays on
   the results. Its output is **committed at once**,
   ordinal + 1, in one records save; nothing is staged. The medal is recorded at its
   site as today, one code path for every campaign. A replay of a cleared event is the
   mod's to judge: `prior_tier` and `event_index` let it decide in one comparison
   whether a finish earns anything. RACE AGAIN is an exit: `race-end` runs and commits,
   then the event relaunches from the new run. A campaign may hide RACE AGAIN with
   `[run] race_again = false` (default `true`). RESTART RACE before the medal is
   captured writes nothing, and nor does quitting a lap-target race before its finish.
   A crash between the medal and the exit keeps the medal and leaves the run as it
   was. **A fault in `race-end`** keeps the medal and saves the latch in the run
   record as an owed end; the run bytes and ordinal are untouched. Until it is settled
   the campaign's events refuse to launch, and its page offers RETRY (run `race-end`
   on the saved latch again, for example after a mod update) or DISCARD (drop the
   latch; the run stays as it was). No reward is lost to a mod bug (maintainer,
   2026-11-20). A **tournament** event is one run event: `race-start` runs for every leg with
   the same committed bytes, and the tick state latched at the end of a leg is the
   next leg's initial tick state, which `race-start` sees and may keep or change, so
   a mod can count across legs. `race-end` runs once, at the exit after the last leg,
   with the final rank as `place`; quitting midway commits nothing. A per-leg
   `race-end` is a later addition (maintainer, 2026-11-20). Replay rebuilds, demos, unofficial runs and split screens never call `race-end`.
7. **A choice screen:** moved to [ADR-0094](0094-a-campaign-run-offers-a-choice-between-races-and-shows-the-mods-stats.md) decision 1. Until it is built,
   `race-end` picks the reward itself.
8. **Stats:** moved to ADR-0094 decision 2.
9. **Replays, ghosts, records, network.** The replay header's `ModUse` carries the run
   input bytes, seed, ordinal and keys (base64, capped on raw bytes), and on a
   tournament leg after the first the tick state carried in from the previous leg. Playback never
   reads the local store. There is no ghost (ADR-0091 decision 5). A race whose start
   applied any non-identity modifier, count or scale files no per-track `[[record]]`;
   loyalty and XP are paid as for any official race (maintainer, 2026-11-20). A
   modded records table, which would give such a race a personal best of its own, is
   later work outside this ADR.
   Run state is single-player: a lobby race carries none and the module declines it.
10. **Own unlocks** (maintainer, 2026-11-19). A campaign may opt out of the global
    ship and variant unlocks (the title's loyalty and `<Unlock>` rows,
    `oag_game::unlock`, that decide which teams, hulls and skins Team Selection
    offers) and define its own: `[unlocks] source = "global"` (default) or `"own"`.
    Under `"own"` Team Selection, for that campaign's events only, offers exactly
    what the campaign's rules open: data rows in the campaign file (a team or a
    variant, opened from the start, by an event's medal, or by a run flag), and the
    run's own unlock set, which the mod adds to at `race-end` (`unlock{team,
    variant}` into the run record, committed with the run bytes). The
    global unlocks are never written by a campaign directly (loyalty and XP earned by
    racing are paid as for any race, decision 9), and a campaign's own unlocks
    never leak into the Race Box or another campaign. Mechanism detail:
    `progression.md` section 7.1.
11. **Build order.** Prerequisite, owned outside this ADR: the records file is written
    atomically (temp file, then rename), a general rule for every profile file that the
    maintainer moved to its own ADR (2026-11-20). Then the run record and the ABI, then modifiers, then pickup
    parameters and `give-autopilot`. The choice and the stats follow under ADR-0094. A
    campaign ships with `race-end` choosing the reward by itself before they land.
12. **Original content pays nothing** (maintainer, 2026-11-20). A race with no module
    keeps `World`'s size, its snapshot, its hash and its per-tick work exactly as
    today. State only a mod needs (the applied factors, the pickup overrides, the
    latch) lives in the extension state, allocated (on the heap if need be) only when
    a module is loaded; the vanilla path costs at most a check that no override is
    there. A modded race may be slower or larger.

**Settled by the maintainer (2026-11-19):** an out-of-range modifier is clamped, never
a fault; whether a failed race ends a run is the mod's choice; whether the player may
skip an offer is the mod's choice; a campaign may scale the AI's craft as well as the
player's (`progression.md` section 12, questions 1, 3, 5 and 11); a campaign may opt
out of the global ship and variant unlocks and define its own (decision 10).
**Settled 2026-11-20:** a pickup `count` may only go down from the craft's own count
(decision 5); a campaign that levels a pickup up climbs back to today's limit and no
further (`progression-challenge.md` objection 12, answered). The draft's `Thrust` and
`TopSpeed` are replaced by `Launch` (`accelcap`) and `TopEnd` (`engine.amount`), each
gated on its own ground truth, and neither applies in Zone, at least for now (decision
4; objection 1, answered). An event of every mode ends where its medal is recorded
today (the finish site, or the escape site for Zone, Speed Lap and an abandoned Time
Trial); the race keeps running as today and `race-end` runs once, when the player exits
the race (decision 6; objection 2, answered). Nothing is staged: `race-end` gets `prior_tier` and
`event_index`, its output commits on exit, RACE AGAIN is an exit that commits, and a
campaign may hide RACE AGAIN with a `[run] race_again` flag (decision 6; objections 3
and 6, answered). The choice screen and the stats are split into ADR-0094; this ADR
needs no frontend world (objection 4 and question 15, answered). A modified craft adapts the
reference speed plan per slot before the plans are built, and `give-autopilot` runs on
any craft (decisions 4 and 5; objection 5, answered). An atomic records write is a
general rule for every profile file, decided in its own ADR outside this one, and a
prerequisite of the run record (decision 11; objection 7, answered). `race-end` gets a
`score` (zones or kills) and reads a result and tick state latched in the sim on the
tick the person's race ends (decisions 3 and 6; objection 8, answered). A tournament is one
run event whose tick state carries from leg to leg, with one `race-end` after the last
leg; a per-leg `race-end` is for later (decision 6; objection 9, answered). A fault in
`race-start` refuses the launch; the clamp stays (decision 3; objection 10, answered). A
fault in `race-end` keeps the medal and saves the latch as an owed end, settled by
RETRY or DISCARD before the campaign launches again (decision 6). A modified race
withholds only its per-track record; loyalty and XP are paid. The modded records table
is later work (decision 9; objection 11 and question 4, answered). All three pickup
durations stay, held as per-slot overrides in the extension state only; original
content never pays for mod features, mod content may be slower (decisions 5 and 12;
objection 13, answered). The guest SDK exposes `race_start` and `race_end` as default
methods on `Extension` with a `Run` blob type and craft handles, and clamps an
out-of-range factor as the host does, with a warning its test harness can assert on
(decisions 3 and 4; objection 16, answered). `race-start` fills a per-slot table, zero meaning
untouched, with spare columns, instead of sending commands; the run cap is 4,096
bytes in a run buffer of its own size (decisions 1 and 4; objection 17 and question
14, answered). Only the campaign's bound module is called at race start
and end (decision 2; objection 18, answered).

**Not decided here:** how a mod ends a run (the mechanism), the AI's use of a mod pickup,
a data-first modifier table, a per-owner damage scale, and every other item in
`progression.md` section 12.

## Alternatives considered

- **Reuse the ADR-0080 tune blob as the run state.** Keyed by mod and title, so two
  campaigns of one mod share it and NEW RUN would clear the mod's unrelated settings.
  Also written only by a screen. Rejected (decision 1).
- **Keep the run in the sim module's tick blob.** It dies with the race; persisting it
  would make the host snapshot a blob the guest cannot tell from scratch state.
  Rejected.
- **Stage the run bytes and the medal, commit on leaving, discard on RACE AGAIN** (the
  first draft). It closes RACE AGAIN but not re-entering a cleared event from the grid,
  and sends a `[run]` campaign down a second path through both `record_result` sites.
  Rejected for `prior_tier` and a commit on exit (decision 6).
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
- **A 1,024-byte run state in a 4 KiB run buffer** (the first draft). Fits, tightly,
  but a campaign with per-event histories outgrows it, and lowering a cap later breaks
  campaigns while raising one does not. Rejected for 4,096 bytes in a run buffer of
  its own size (16 KiB).
- **A race-start command list** (`set-modifier` and its siblings, 64 at most). A full
  grid of 16 crafts with 14 settings each is 224 commands. Rejected for the per-slot
  table.
- **The run in the tick buffer's state region.** At `race-start` the region is the
  module's own state and the output needs a second one. Rejected.
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

- **The README overstates the built ABI.** `race-start`, `race-end` and `setup.tune`
  are designs, not code; this ADR builds the first two.
- **The race exit calls a guest.** The results page's exits and the escape site in
  `oag-game` serve every campaign; for a `[run]` campaign they now call `race-end` and
  save the run. The medal path itself is unchanged.
- **Farming is the mod's job.** The host gives `prior_tier` and `event_index` and
  enforces nothing; a mod that ignores them pays out on every replay of a cleared
  event.
- **A clamp where the convention is a fault.** An author can ship a module whose
  rewards are silently weaker than intended; only the load report says so.
- **Per-track records are lost for a modified race.** No leaderboard pollution, and
  no personal best either, until the modded records table exists.
- **A boosted race still pays loyalty and XP.** Loyalty opens the title's teams and
  skins, so a campaign with strong modifiers is a faster way to the global unlocks.
  Accepted by the maintainer (2026-11-20).
- **A closed parameter list is a ceiling,** like the command set in ADR-0079. A
  parameter outside the list waits for a host change, and `Launch` and `TopEnd` are
  on the list and refused until their ground truths land.
- **No engine reward in Zone.** `Launch` and `TopEnd` scale `Handling` fields the Zone
  speed path never reads; a Zone event of a run campaign flies the stock engine.
- **Hard engine bounds remain.** Three rockets and five mines are the sizes of hashed
  arrays; more is an engine change with its own proof.
- **A mod can scale the AI** (allowed, maintainer 2026-11-19). The AI obeys the
  player's physics, so a slot's factor works on it too. Each distinct modified craft
  costs one plan adaptation at load.
- **An adapted plan is close, not optimal.** A modified craft's AI, Autopilot and Pilot
  Assist fly an adaptation of the reference plan, not a plan learned from scratch.
- **More weapon-lineage rows.** Each new per-ship pickup field and `give-autopilot`
  needs its row in `docs/gameplay/weapon-lineage.md` when built.
- **Run state is single-player.** Netplay needs a per-seat story, the same open
  question as the per-seat tune.
