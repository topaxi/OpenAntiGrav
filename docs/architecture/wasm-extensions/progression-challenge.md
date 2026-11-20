# WebAssembly extensions: a challenge of the campaign run draft

**Status: a design review, 2026-11-19.** It challenges
[ADR-0093](../adr/0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
(Proposed) and [progression.md](progression.md) as they stand at commit `fd75340aa`.
It changes neither. **Resolved:** the maintainer answered every objection on
2026-11-20; each answer stands under its objection. Nothing here is built or
measured by this review; where a number is derived from a documented law it says so.

**The claim tested:** the draft is a sound, minimal, buildable design that fits this
engine's rules.

**Result: false as written, true for a smaller core.** The run state with two
sim-world exports (capability 1) is sound after four changes and is buildable today.
The handling parameter table (capability 2) contradicts the measured force law. The
pickup parameters (capability 3) are mostly redundant or go the wrong way. The choice
screen and the stats (capabilities 4 and 5) rest on an undecided ADR and have a
simpler shape that does not.

| Severity | Count | Numbers |
| --- | ---: | --- |
| blocks | 5 | 1 to 5 |
| should change | 13 | 6 to 18 |
| nit | 7 | 19 to 25 |

"Blocks" means the decision it names cannot be accepted or built as written. Each
one says what it blocks; none blocks the whole draft.

## Verdict per capability

| Capability | Verdict | In one line |
| --- | --- | --- |
| 1. Run state carried race to race | **keep, change** | Commit where the medal commits, cover Zone and Speed Lap, give `race-end` the prior medal (objections 2, 3, 6, 7, 8, 9) |
| 2. Race-start handling modifiers | **change, defer to a later slice** | Rebuild the engine parameters from the force law, person slots only, plan the Autopilot from the modified craft (1, 5, 10, 11, 16) |
| 3. Pickup parameters and combined pickups | **drop counts and the Shield and Turbo durations, defer the Autopilot pieces, keep the combined pickup** | The combined pickup is expressible today; counts replace an authored value and can only go down on Pulse and HD (12, 13) |
| 4. Between-races choice screen | **change** | A host-drawn offer page and a third sim export, no frontend world needed (4, 14) |
| 5. Mod-defined stats | **change, defer the Ship Select rows** | The guest reports rows; the host does not read offsets out of opaque bytes (4, 15) |

## What held

These claims of the draft were checked and are true. They are listed so the
objections are not read as "everything is wrong".

- ABI revision 6 exports `tick`, `oag_wants_race`, `oag_track`, `oag_state_len` and
  nothing for a race start or end (`crates/ext-abi/src/lib.rs` header, `REVISION`).
  No `[[extension.screen]]` in `crates/mod/src/manifest.rs`, no `open-mod-screen`
  in any crate. ADR-0080 is Proposed.
- `Ship::handling` and `Ship::weapon_counts` are deliberately unhashed
  (`crates/gameplay/src/hash.rs`, the destructure in the ship hash).
- `Extensions` hashes nothing when no module is loaded (`On::hash_into` in
  `crates/raceplay/src/extensions/on.rs` returns on `slots.is_empty()`).
- A replay stores an initial hash and one hash per interval
  (`crates/replay/src/verify.rs`), so an applied record hashed before the first tick
  does refuse other run bytes at tick 0.
- Pulse and HD fly one `Handling` struct under one force law: HD's thrust from rest
  matches Pulse's law on HD's table within 0.7 % (confidence 90,
  [hd-handling-ground-truth.md](../../physics/hd-handling-ground-truth.md)). A
  factor on a field means the same on both.
- No network race runs a guest today ([hooks.md](hooks.md), "Online").
- The second buffer's arithmetic fits. With `MAX_CRAFTS` = 16 the layout ends at
  3,996 of 4,096 bytes (116 header, 1,024 in, 32 result, 4 length, 1,024 out, 1,024
  tick state, 772 reply).
- Command codes 39 and up are free (the highest is 38), and craft flag bit 32 is
  free (`crates/ext-abi/src/craft.rs` uses 1 to 16).
- A run is per title: progress rows are keyed
  `<title slug>/mod:<mod id>/<campaign id>`, so a campaign offered on Pulse and HD
  has two runs and a run cannot continue on another title.

## Blocks

### 1. `Thrust` and `TopSpeed` are mapped against the measured force law (blocks the capability 2 table)

**Evidence.** The engine law is
`T = 2 * min(throttle * amount, 0.5 * speed + accelcap)`
(`oag_physics::engine::engine`, `crates/physics/src/engine.rs`; the cap arm at
confidence 95 in
[force-balance-ground-truth.md](../../physics/force-balance-ground-truth.md)). From
rest the cap arm binds, and the launch is `2 * accelcap`: the same page fits a launch
to `net = fs + 2 * accelcap - 2.0 - 0.005 * fs^2` with rms 0.127, and
[hd-handling-ground-truth.md](../../physics/hd-handling-ground-truth.md) records HD
looking 14 % quicker off the line on a team whose `accelcap` is 21 against 18.5
(`21 / 18.5 = 1.135`). `engine.amount` is the other arm: the plateau thrust, which
sets terminal speed.

Derived from that law, not measured (HD Feisar, `accelcap` 18.5, `amount` 0.412
after the load scale, drag `0.005 * v^2 + 0.01 * v + 2`):

| Factor on `engine.amount` | Cap arm binds until | Terminal speed |
| --- | --- | --- |
| 1.0 | 45 units/s | about 126 units/s |
| 1.1 | 54 units/s | about 132 (+5 %) |
| 2.0 | 128 units/s | about 179 (+43 %) |

So the draft's `Thrust` (section 4.1, "engine.amount") changes nothing off the line.
At 2.0 the craft accelerates exactly as stock through the whole stock speed range
and only gains top speed. It is a top-speed parameter that grows with the square
root of the factor.

The draft's `TopSpeed` (section 4.2) names scaling `accelcap` as a candidate and
calls its effect on terminal speed unmeasured. The documented law already answers
it: `accelcap` leaves the law once the plateau arm binds, so it moves the launch and
not the terminal speed.

`Thrust` also does nothing in a Zone event: the `auto_speed` branch replaces the
whole throttle path and never reads `engine.amount` (`engine.rs`, "The four-corner
branch"). Custom campaigns have Zone events
([custom-campaign.md](../../formats/custom-campaign.md), `mode`).

Section 4.1 says `Thrust` "composes with the per-tick `scale-thrust`" with a product
of at most 4.0. They are not the same lever. `scale-thrust` multiplies the finished
engine force, after the cap and the turbo add (`Environment::extension_thrust`,
`crates/physics/src/forces.rs`), and it works in Zone.

**Proposed change.** Remove `Thrust` and `TopSpeed` as defined. Either:

- (preferred) offer one engine parameter, `EnginePower`: a per-slot base factor on
  the same post-law point `scale-thrust` uses, bounded together with it by
  `MAX_THRUST_FACTOR`. One meaning on every mode. It needs no `Handling` write; or
- name the two arms for what they do: `Launch` scales `accelcap`, `TopEnd` scales
  `engine.amount`, with the table above in the docs and a held-throttle ground truth
  for each before either ships.

A reward that makes the engine stronger is already expressible with `scale-thrust`
re-sent each tick from the tick state ([hooks.md](hooks.md): "`scale-thrust` above
1.0 is the mod's engine scale"). The first slice needs no engine parameter at all.

**Answered by the maintainer (2026-11-20): `Launch` and `TopEnd`.** The second option:
`Launch` scales `accelcap`, `TopEnd` scales `engine.amount`, each refused until its
held-throttle ground truth lands. Neither applies in Zone, which is its own mode, at
least for now (ADR-0093 decision 4, progression.md section 4.2).

### 2. Zone and Speed Lap events never reach `race-end` (blocks capability 1 for two of seven modes)

**Evidence.** `RaceSim::finished` never turns true for Speed Lap or Zone;
their result and medal are captured only when the player leaves
(`crates/game/src/records.rs` module doc, "Where this is captured";
`Session::escape` in `crates/game/src/app/session/escape.rs`, "The other capture
site"). The draft's lifecycle table (section 5) says "QUIT RACE or escape before the
finish: no `race-end`, nothing staged". Under that rule a Zone or Speed Lap event of
a run campaign never calls `race-end`, never commits and never offers a reward.
Leaving is how those modes end.

**Proposed change.** Define the end of an event as the two sites that already record
the medal: the finish site for a lap-count mode, the escape site for Zone, Speed Lap
and an abandoned Time Trial. `race-end` is called wherever `Launch::record_result`
is called and nowhere else. The "quit stages nothing" row then applies only to modes
with a lap target.

**Answered by the maintainer (2026-11-20): both medal sites.** An event ends where its
medal is recorded today, the finish site or the escape site. The race goes on as it
does today, and `race-end` (the mod's award and end-of-run logic) runs once, when the
player exits the race (ADR-0093 decision 6, progression.md section 5).

### 3. Replaying a committed event farms rewards, and the staging does not stop it (blocks decision 6 as the answer to "a retry must not farm")

**Evidence.** The lifecycle table has no row for launching an event again after its
commit. A cell stays enterable after it has a medal (the campaign keeps the best
medal per cell). Each such finish calls `race-end`, commits, raises the ordinal and so
rolls a fresh offer. `RunResult` (section 3) carries this race's `tier` and not the
medal the cell already had, so a guest cannot tell a first clear from the tenth
without keeping its own table. The only event identity it gets is a 32-bit
`channel_key`, 4 bytes per cleared event out of 1,024.

So decision 6 (stage the bytes and the medal, commit on leaving, a commit at next
launch after a crash, a changed path through both `record_result` call sites) closes
RACE AGAIN and leaves the equivalent path through the grid open.

**Proposed change.** Add to the result `prior_tier` (the cell's best before this
race, 255 for none; the host has it, `Store::campaign_medal`) and `event_index` (the
event's position in the campaign file, so a cleared set is a bitmask). With those the
guest decides in one comparison whether a finish earns anything. Then see objection 6:
the staging can go.

**Answered by the maintainer (2026-11-20): `prior_tier`, no staging.** `race-end` gets
`prior_tier` and `event_index` and decides itself whether a replay earns anything
(ADR-0093 decision 6, progression.md section 5). Answered together with objection 6.

### 4. Capabilities 4 and 5 are decided inside an ADR whose base is undecided (blocks accepting decisions 7 and 8)

**Evidence.** ADR-0080 has been Proposed since 2026-10-29 and nothing of the
frontend world exists. ADR-0093 decision 7 narrows ADR-0080 decision 6, and its
Status narrows ADR-0080 decision 7. One Proposed ADR is amending another. Once
ADR-0093 merges it is immutable, while the ADR it amends can still change or be
rejected (the draft's own open question 15).

The draft is also one ADR with ten decisions over five capabilities of very
different readiness. Accepting it accepts the frontend-dependent ones.

**Proposed change.** Split it. One ADR for the run state and the two exports (sim
world and records only). A second for start-time modifiers, after objection 1. The
choice and the stats get their own ADR once objections 14 and 15 are answered; as
proposed there they need no frontend world, so they stop depending on ADR-0080.

**Answered by the maintainer (2026-11-20): two ADRs.** ADR-0093 keeps everything that
needs no frontend world (the run, the exports, modifiers, pickup parameters, own
unlocks, replays and records). The choice screen and the stats moved to ADR-0094,
Proposed, to be settled with objections 14 and 15.

### 5. A modified craft invalidates the learned speed plan (blocks modifiers on AI slots and `give-autopilot` on a modified craft)

**Evidence.** The AI, the Autopilot pickup and Pilot Assist drive a speed plan
learned per hull at race load by simulating that hull's `Handling`
(`crates/raceplay/src/speed_plan.rs`: `HullPlans`, `other_hulls`, `flown_by`;
`SimRace::plans` is learned on a thread and "adopted only if the race still matches
what it was learned from"). A search from scratch is measured there at 12 to 24
times the CPU of a race start, which is why other hulls adapt the reference's plan
under a 40,000 step budget.

The draft applies factors "after the grid is placed" and never mentions the plan.
Decision 4 accepts any grid slot, so every AI slot can become a distinct hull. The
worked example (section 9) gives the player `Thrust` and then `give-autopilot`: the
Autopilot would fly a plan learned for the stock craft.

**Proposed change.** In the first modifier slice, person-flown slots only (this also
answers open question 11 and the "a mod can scale the AI" consequence). Apply the
factors before `begin_speed_plans`, and learn the person's Autopilot and Pilot Assist
plan from the modified `Handling`. Defer `give-autopilot` until a ground truth shows
a modified craft's Autopilot laps without wall contact.

**Answered by the maintainer (2026-11-20): adapt per slot, allow `give-autopilot`.**
AI slots stay allowed (settled 2026-11-19). Factors are applied before the speed plans
are built, and each modified slot adapts the reference plan under the 40,000-step
budget. `give-autopilot` runs on any craft: the maintainer has seen the pickup
Autopilot hold the spline hard without a crash, even on a poor plan (an observation,
not a measurement). ADR-0093 decisions 4 and 5, progression.md sections 4.1 and 6.

## Should change

### 6. Commit where the medal commits; drop the staging

**Evidence.** Objection 3: the staging does not close farming. Its costs are real:
a `pending` copy and an `offer_pending` flag in the record, a commit at the next
launch, RACE AGAIN and RESTART RACE taking a different path in a `[run]` campaign,
and both `record_result` sites changing behaviour for one kind of campaign (the
draft's own "regression surface for the disc campaigns").

**Proposed change.** `race-end` runs at the medal's two capture sites and its output
is the committed run, saved in the same `save_in_background` call as the medal. In a
`[run]` campaign the results page offers no RACE AGAIN once the result is recorded;
the player re-enters from the grid and the guest sees `prior_tier`. RESTART RACE
before the end is unchanged (nothing was written). One code path for every campaign.

**Answered by the maintainer (2026-11-20): drop the staging, keep RACE AGAIN.**
`race-end` runs once when the player exits the race (objection 2's answer) and commits
at once. RACE AGAIN is an exit: it commits, then relaunches from the new run. A mod
that wants no RACE AGAIN sets `[run] race_again = false`.

### 7. Make the records write atomic before a run lives in it

**Evidence.** `records::write_to` calls `profile::write`, which on native is
`std::fs::write` (`crates/game/src/profile.rs`): truncate, then write. The draft's
crash row says the staged pair "never splits". True, they are one file, and a crash
inside that write can leave the file cut short. A medal can be earned again; a run
cannot be rebuilt.

**Proposed change.** Write to a sibling file and `rename` (the module already
re-exports `rename`). It is a small change that stands alone and should land first.

**Answered by the maintainer (2026-11-20): yes, as a general rule outside mods.**
`rejoin_record.toml`, `music.toml` and the identity file are already written temp then
rename; `records.toml` and the settings are not, and an unparsable `records.toml` is
moved aside so the player starts with no records. Every profile file gets the atomic
write through one helper, under its own general ADR and thread (handed to the drive's
orchestrator). ADR-0093 decision 11 names it as a prerequisite of the run record.

### 8. `RunResult` misses what two modes are scored on, and its tick state depends on frame pacing

**Evidence.** Medal targets are a place, seconds, or a count for Zone and
Eliminator ([custom-campaign.md](../../formats/custom-campaign.md), "Medal
targets"). `RunResult` has place, laps and ticks and no count, so a guest cannot
reward zones reached or kills.

The finish site runs after the frame's tick loop (`Session::frame`, "The one moment
a finished race's result is captured"; the loop steps a variable number of ticks),
and the world goes on being stepped under the results (`RaceSim` in `results.rs`).
The "final tick state" and `shield` handed to `race-end` are therefore from one to
several ticks after the finish, by frame rate. Two machines with the same inputs can
write different run bytes. No hash breaks, since `race-end` is outside a replay, but
"a run is a function of its inputs" fails, and with it any later re-simulation of a
run.

**Proposed change.** Add `score: u32` (zones or kills). Latch the result fields and
the module's blob inside the sim on the tick the person's race ends (plain data, no
guest call), and build `RunResult` from the latch. The escape-site modes use the
state at the leave, which is the player's own timing.

**Answered by the maintainer (2026-11-20): score and latch, as proposed.** ADR-0093
decisions 3 and 6, progression.md sections 3 and 5.

### 9. A tournament event is unspecified

**Evidence.** A campaign event may be a tournament with several legs
([custom-campaign.md](../../formats/custom-campaign.md), `tracks`). Each leg reaches
the finish site, where `record_finished_leg` folds it into the standings, and the
medal is known only after the last leg (`crates/game/src/app/session/tournament.rs`).
The draft never says whether `race-end` runs per leg or per event, or what the
ordinal counts.

**Proposed change.** `race-start` runs for every leg with the same committed bytes.
`race-end` runs once, after the last leg, with the final rank as `place`. The tick
state does not carry between legs.

**Answered by the maintainer (2026-11-20): once per event, and the state carries.**
`race-start` runs per leg with the same committed bytes and the previous leg's latched
tick state; `race-end` runs once after the last leg. A per-leg `race-end` is for the
future (ADR-0093 decision 6, progression.md section 5).

### 10. A fault at `race-start` must refuse the launch, and then the clamp question disappears

**Evidence.** The draft clamps an out-of-range factor "because a fault would turn
the module off for the race" and the player would fly stock with no hint (open
question 1). A trap, spent fuel or a malformed record at `race-start` still does
exactly that, and the stock race then earns a medal in a campaign whose module is
the point. ADR-0077 decision 6 is "refuse, never substitute".

**Proposed change.** Any `race-start` fault refuses the launch: back to the campaign
page with a line naming the mod and the fault. Then an out-of-range factor can be a
fault like every other command, the README's "a fault, never a clamp" holds, and the
"clamp where the convention is a fault" consequence goes away.

**Answered by the maintainer (2026-11-20): refuse the launch; the clamp stays.** Any
`race-start` fault refuses the launch with a line naming the mod. An out-of-range factor
still clamps with a load-report warning (settled 2026-11-19), so only the first half of
the proposed change is taken (ADR-0093 decision 3, progression.md section 4.1).
Asked beside it, the same day: a fault in `race-end` keeps the medal and saves the
latch as an owed end; the campaign refuses to launch until RETRY succeeds or the
player picks DISCARD, so a mod bug loses no reward (ADR-0093 decision 6,
progression.md section 5).

### 11. The no-record rule leaves loyalty and XP paying for a boosted race

**Evidence.** The finish site pays loyalty for any official race
(`Session::frame`, the `record_loyalty` block) with no test of `is_modded`. Loyalty
is kept per team. Decision 9 withholds only the per-track
`[[record]]`.

**Proposed change.** State one rule for a race whose start applied a non-identity
modifier: no per-track record, no loyalty, no XP. Or decide that loyalty is paid and
write that down. The engine-power path through `scale-thrust` has the same question
today, so the rule belongs to MODDED races in general.

**Answered by the maintainer (2026-11-20): loyalty and XP are paid.** A modified race
withholds only its per-track record, as the draft had it. The modded records table is
later work, outside ADR-0093 (decision 9; progression.md section 5.1, question 4).

### 12. A pickup `count` replaces an authored value, and on Pulse and HD it can only go down

**Evidence.** `WeaponCounts::default()` is 3 rockets and 5 mines, which are
`MAX_ROCKETS` and `MAX_MINES` (`crates/weapons/src/counts.rs`). Pulse and HD author
none (the draft's own section 10), so on two of the three titles with custom
campaigns the range 1 to 3 and 1 to 5 can only take away. On Omega and 2048 the
count is authored per craft, and `set-pickup-count{count}` writes an absolute value
over it. Decision 4 defends modifiers as "a bounded multiplier on the disc's own
number, never a replacement value"; the count is a replacement, and the README's
section 3 row names "a weapon pad's grant".

**Proposed change.** Drop `set-pickup-count` from this design. If a later design
wants it, make it a signed delta clamped to the engine bounds and list it among the
narrowed statements.

**Answered by the maintainer (2026-11-20): keep counts, down only.** Going only down
is the point: a campaign may start a craft on one rocket and level the pickup up to
two and three, ending at today's limit. The count is clamped to `1..=` the craft's
own count (ADR-0093 decision 5), which keeps decision 4's rule: it reduces the
disc's number and never replaces it past it.

### 13. `set-pickup-duration` duplicates built commands for two of its three weapons

**Evidence.** A guest can already hold a Shield past its time with `hold-shield`
each tick and end it with `clear-shield` (`crates/ext/src/reply.rs`), and a boost of
any length is `scale-thrust` on a timer in the tick state (the Energy Turbo example
in the [README](README.md)). The draft adds three per-ship fields, their hash record
and three weapon-lineage rows for this. Only the Autopilot has no existing lever.

**Proposed change.** Drop the Shield and Turbo durations. Defer the Autopilot's with
`give-autopilot` (objection 5). The combined pickup stays: shield plus boost is
`give-shield`, `scale-thrust` and `clear-pickup` today.

**Answered by the maintainer (2026-11-20): keep all three, outside `World`.** The
built commands only approximate a longer pad Shield or Turbo, so the Turbo, Shield and
Autopilot scales stay, as per-slot overrides in the extension state behind one grant
helper. The rule behind it: original content must not regress, mod content may be
slower (ADR-0093 decisions 5 and 12, progression.md section 6).

### 14. The choice does not need the frontend world, and should not live in it

**Evidence.** In section 8.1 a frontend guest parses the run bytes, applies the pick
and returns new bytes that become simulation input. That puts a state transition in
the world with "no determinism requirement" (ADR-0080 decision 1), needs a second
module that carries the same byte layout as the rules module, and cannot be tested
with the SDK's native race harness. The draft rejects "the screen returns only the
pick" because Ship Select rows must show the pick early and because the pick would
be a second replay input. The first reason falls with objection 15. The second is
one byte in a header record that already carries up to 1,024.

**Proposed change.** `race-end` replies with up to four offers, each a string id of
the mod. The host draws a standard pick page in the title's skin (pad and pointer
come from the host page, so the pointer rule is met once). On a pick the host calls
a third sim-world export, `oag_run_pick`: a pure function of (run bytes, seed,
ordinal, pick index) that returns the new run bytes. All run logic stays in one
deterministic module with one layout. No dependency on ADR-0080 and no narrowing of
its decision 6.

**Answered by the maintainer (2026-11-20): the mod screen, as drafted.** The choice
stays an ADR-0080 screen that returns the new run bytes, so a campaign owns its
reward screen; it waits for the frontend world (ADR-0094 decision 1, progression.md
section 8.1).

### 15. Stats by byte offset make the campaign file a second copy of the guest's layout

**Evidence.** `[[run.stat]]` (section 7) has the host read `u8` or `u16` values at
offsets in bytes the draft elsewhere calls opaque. The layout then lives in the
rules module, the screen module and the campaign TOML. After a mod update that moves
a field, the host draws wrong numbers until the next write, and `race-start` cannot
write, so nothing can migrate the bytes before Ship Select shows them.

The Ship Select allowance also means three screen integrations (Pulse's Team
Selection, HD's and Omega's Ship Select), a fourth place for preset craft (open
question 10) and a narrowing of "a disc screen gains no mod behaviour".

**Proposed change.** The guest reports its rows: `race-end` and `oag_run_pick` may
reply with up to eight `stat{string id, value, max}` records. The host stores them
beside the run as a display cache and draws them on pages it owns for the campaign
(the offer page, and a line on the campaign page). A stale cache shows the last true
values, never misread bytes. Defer rows on a title's Ship Select until a campaign
exists that needs them.

**Answered by the maintainer (2026-11-20): the guest reports rows now, Ship Select
rows as a follow-up.** Up to 8 `stat` rows from `race-end` and the choice screen,
cached beside the run, drawn on the campaign page (ADR-0094 decision 2, progression.md
section 8.2). Nit 24 falls with it.

### 16. The SDK draft does not match `crates/ext`

**Evidence, each against `crates/ext/src/lib.rs` or `reply.rs`.**

- `trait Run: Extension`, with `export!` gaining three exports "when the extension
  implements `Run`". `export!` is `macro_rules!` and cannot see a trait impl. The
  existing optional call, `wants_race`, is a default method on `Extension`, and
  `export!` always emits `oag_wants_race`.
- `run_state!` with `Haul::read` and `h.bytes()` duplicates `blob!`, whose
  `from_bytes` already reads missing bytes as zero. `MAX_RUN_BYTES` equals
  `MAX_STATE_BYTES`, so `blob!`'s size assert already fits.
- `StartReply::modifier(slot: Slot, ...)` takes a raw slot. Every `Reply` method
  takes a `&Craft` the observation handed out, "so a command cannot name an empty
  slot".
- The example writes `let _ = out.modifier(...)`. The SDK refuses a factor past the
  range, so at engine level 11 the reward is dropped whole, in silence. That is worse
  than the host's clamp and is the outcome the clamp was chosen to prevent.

**Proposed change.** Default methods `race_start`, `race_end` (and `run_pick`) on
`Extension`, with `type Run: Blob` beside `type State`. `Start::crafts()` hands out
a grid craft (slot, person flag, team key) that the reply methods take. Under
objection 10 a refused value is the author's bug, found by `cargo test`; the example
should use `?` or handle the error, not discard it.

**Answered by the maintainer (2026-11-20): default methods, and the SDK clamps.**
`race_start` and `race_end` are default methods on `Extension` with `type Run: Blob`,
and the reply methods take a grid craft handle. An out-of-range factor clamps in the
SDK exactly as on the host, and the test harness records a warning a test can assert
on (progression.md section 3, "The guest SDK"; the worked example in section 9 follows
it, and also fixes the duplicate offer and the made-up `mix`).

### 17. The caps are argued from the wrong grid size, and the buffer is full on day one

**Evidence.** `MAX_START_COMMANDS` = 64 is justified as "8 slots x 8 parameters,
with room". `MAX_CRAFTS` is 16 (`crates/ext-abi/src/lib.rs`), and one slot can take
8 modifiers, 3 counts and 3 durations: 14 a slot, 224 for a full grid. The layout
uses 3,996 of 4,096 bytes, so objection 3's and 8's fields have 100 bytes left and
the next addition forces a squeeze.

**Proposed change.** Person slots only (objection 5), so 32 start commands is ample.
Overlap the regions by call: `race-start` never uses the result or the final tick
state, `race-end` never uses the command reply. Start `MAX_RUN_BYTES` at 256 (the
example uses 6); raising a cap later breaks nobody, lowering one does.

**Answered by the maintainer (2026-11-20): a per-slot table, and 4 KiB.** AI slots
are allowed (settled), so the premise "person slots only" is gone. `race-start` fills
a fixed table, a row per grid slot (16 factor columns with 8 spare, 4 counts, 3
durations, 80 bytes), zero meaning untouched; no command cap. The run cap goes up, not
down: 4,096 bytes, in a run buffer of its own size (16 KiB) with regions overlapped by
call (progression.md section 3).

### 18. Two modules may both send start commands, and the draft does not say who wins

**Evidence.** Any module exporting `oag_race_start` gets the call (section 7,
"Binding": the unbound module "sees an empty run"). A second rules mod that is on
runs in the campaign's races too. "Two commands for one slot and parameter are a
fault" does not say whose. The precedent for a shared factor is `scale-thrust`:
"multiplied across modules and capped" ([hooks.md](hooks.md)).

**Proposed change.** In the first modifier slice only the campaign's bound module
may send start commands; another module's is a fault of that module. Revisit with
the composition question (README open question 8).

**Answered by the maintainer (2026-11-20): the bound module only.** Stricter than
proposed: no other module is called at race start or end at all, so there is nothing
to fault (ADR-0093 decision 2, progression.md section 7). Composition stays README
open question 8.

## Nits

19. **Open question 9 is already answered.** The manifest key `drives_ai_pickup`
    lets the module decide when an AI craft uses the mod's pickup
    (`crates/mod/src/manifest.rs`; `On::guest_drives_ai_pickup`). The Turbo gate is
    only the default.
20. **`TurboStrength` is more than the Turbo pickup.** `engine.turbo` is also added
    while a barrel roll's landing payout runs, in Zone too (`engine.rs`:
    `turbo_timer > 0.0 || roll_payout_timer > 0.0`). Say so in the table, or the
    name misleads. `ShieldMax` likewise scales the barrel roll's cost and the
    Eliminator's lap refill, both fractions of `dimensions.shield`.
21. **`RunResult` is not 32 bytes as listed.** Six `u8`, two `u32`, two `f32` and 4
    spare bytes are 26 without padding. Give it offset constants like every other
    record in `oag-ext-abi`.
22. **Three statements about a new run disagree.** Section 5 creates it with "empty
    bytes", the example comments "zeroed when the run is new", and `[run] run_bytes`
    declares a length in the campaign file. The precedent for a length is an export
    (`oag_state_len`), checked against the manifest.
23. **Section 1's table says `ModUse` carries "state bytes".** It carries the
    blob's length (`state_bytes: u32`), not the bytes.
24. **`bonus_of` hard-codes bar names in a generic schema.** Which bars a title
    draws is title data (ADR-0058); `firepower` and `length` exist on two titles.
    Moot if objection 15 is taken.
25. **`Grip` has no stability check behind its range.** The lateral force is
    explicit: `grip * lateral velocity * k`, times 1.5 during a roll payout
    (`crates/physics/src/airbrake.rs`). It overshoots when
    `grip * 1.5 * factor / 60` passes 1. One measured `grip_ground` is about 10
    ([cornering-ground-truth.md](../../physics/cornering-ground-truth.md)), which
    gives 0.5 at factor 2.0. **Unsettled** for other craft: a census of the largest
    authored `grip_ground` and `grip_air` per title settles the range.
    **Answered by the maintainer (2026-11-20): a per-craft safety clamp.** The host
    clamps the factor to `0.9 * 60 / (grip * 1.5)` for each craft, with a load-report
    warning; the census becomes documentation (ADR-0093 decision 4, progression.md
    section 4.1).

The worked example has small faults of its own: both offers can be the same reward
(`roll % 3` twice), `mix` is not an SDK function, and a failed race writes nothing
without the draft saying what "no write" does to the ordinal.

**Answered by the maintainer (2026-11-20): nits 19 to 23 as proposed** (question 9
answered by `drives_ai_pickup`; the side effects named in the table; `RunResult`
offsets; `oag_run_len` instead of `[run] run_bytes`; the `ModUse` wording). Nit 24 fell
with objection 15. Nit 25 is taken separately. The worked example's faults were fixed
with objection 16. The ordinal on a run that writes nothing was answered with "how a
mod ends a run" the same day: every committed `race-end` advances it (ADR-0093
decision 1).

## Edge cases asked about, and where each stands

| Case | Finding |
| --- | --- |
| Retry farming | Open through the grid: objection 3. |
| Quit mid-race | Sound for lap-count modes. Wrong for Zone and Speed Lap: objection 2. A failed run cannot be made to count, because quitting before the end writes nothing; that is every campaign's rule today and the draft's open question 3. |
| Crash during save | The pair shares one file, but the write is not atomic: objection 7. |
| A mod updated mid-run | The guest's own version byte handles the bytes. The host's offset-based stats cannot: objection 15. The replay side is covered, since `ModUse` pins the module's SHA-256. |
| Two mods both wanting run state | One run per campaign and one bound module, so two runs never mix. Two modules sending start commands: objection 18. |
| A run started on one title, continued on another | Cannot happen: the key carries the title slug. Checked, holds. |
| Saves synced between machines | `just sync-saves` copies `data/saves/` (emulator save states) and never the player's `records.toml`. The run and its medals share that one file, so any whole-file copy keeps them consistent. Nothing merges two machines' records, as today. No objection. |
| Replay, rollback | Sound once the factors are applied before the initial hash. `Ship::handling` is in the snapshot. The run-to-run purity gap is objection 8. |
| Network, lobby fingerprint | No network race runs a guest, so nothing to break today. When one does, the start commands' inputs must travel in `RaceParams` like a `sim` pick; the hashed applied record already refuses a mismatch at tick 0. |

## The smallest first slice

Buildable now, with no frontend world, no `Handling` write, no new hashed engine
state and no change to a disc screen.

1. **Atomic records write** (objection 7). Stands alone.
2. **The run record**: `[run] module` in the campaign file; bytes of at most 256, a
   seed and an ordinal in the records `Store` under the campaign's key.
3. **Two exports, ABI revision 7.** `oag_race_start` reads the run and sets the
   module's initial tick state. It sends no commands. `oag_race_end` reads the
   result (with `prior_tier`, `event_index`, `score`, latched at the end tick) and
   writes the run. It is called at the medal's two capture sites and committed with
   the medal (objections 2, 3, 6, 8, 9).
4. **A launch that refuses on a `race-start` fault** (objection 10).
5. **The replay header's `run` input**, as the draft has it.
6. **Rewards through built commands only**: engine power by `scale-thrust` each tick
   from the tick state, stronger pads by `scale-slot-pads`, a longer Shield by
   `hold-shield`, the mod's own combined pickup by `give-shield` and `scale-thrust`,
   AI use by `drives_ai_pickup`. `race-end` picks the reward itself, which the draft's
   section 11 already allows.

The vanilla proof is trivial: the tick blob is already hashed and nothing else
changes. The slice proves the one new principle (a sim-world call whose output
persists), the lifecycle and the replay path, and it ships a playable campaign whose
craft changes from race to race.

Next, in order: the host-drawn offer page with `oag_run_pick` and guest-reported
stat rows (objections 14, 15); then start-time modifiers for person slots with the
parameter table rebuilt (objections 1, 5, 11, 18); then the Autopilot pieces.
Deferred without a date: pickup counts, AI-slot modifiers, rows on a title's Ship
Select, a frontend-world choice screen.

## Not checked

- No Ghidra or emulator work, and nothing was run. Every engine claim is a reading
  of the cited function or page.
- The terminal-speed table in objection 1 is arithmetic on the documented law. A
  held-throttle ground truth at factors 1.0 and 2.0 would confirm it.
- The authored rocket and mine counts on 2048 and Omega were not read from the
  discs; objection 12 relies on the draft's own statement that they are per craft.
- Whether the end-of-race screens of each title can hide RACE AGAIN for one kind of
  campaign (objection 6) was not read screen by screen.
- 2048 and Omega: this review recovers nothing for either, so the cross-title check
  has nothing to record beyond the draft's own.
