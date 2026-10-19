# ADR-0071: Weapons are predicted simulation, with a discrete-state correction trigger

## Status

**Proposed (2026-10-19). Not accepted: the maintainer decides.** Answers condition 2 of
[ADR-0069](0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md)
("Weapons (option D, weapons as server-confirmed events)"); ADR-0069 is not edited. Evidence:
`crates/net/tests/weapons_ground_truth.rs` and the measurements below.

## Context

ADR-0069 held weapons off in network races and named option D, weapons as events the server
confirms, as the open question: a replayed input stream never fires, so a missile popping into view
after a rollback, or a dodge undone by one, was untested.

Reading the code first changed the question. **The whole simulation is predicted, not selected
features.** The client runs the same `RaceSim` as the server: projectiles, the Quake, the
LeachBeam, Repulsers, every craft's pickup and shield, each person's lock-on sight
(`RaceSim::locks`, advanced from the world with no camera) are `World` or `RaceSim` state, are
in `state_hash`, are restored by `restore_sim` and advanced by the replay (`tick_sim_only`). A
fire is the Square button edge in the input the wire already carries (`Held::buttons`, edges
re-derived by each end). Nothing about weapons needed a second mechanism to be predicted, and a
weapons-on race hashed identically on every confirmed tick from the first run.

What was missing is the **trigger**. A client predicts another person on their last held input,
which carries no edges, so a remote person's fire is never in this client's present until a
correction replays the confirmed tick; and corrections fired only on a craft position error over
1 unit. A missile that moved no craft by a unit left the present without it, for as long as it
took something else to trigger a correction.

## Decision (proposed)

1. **Weapons are predicted simulation (option B), not server-confirmed events (option D).** One
   timeline, one mechanism, the same rollback as a craft. No weapon-specific message exists on
   the wire; the room's `weapons` setting travels in `RaceParams` (already in the tick-0 hash).
2. **The correction trigger also compares discrete weapon state.** At every confirmed tick the
   client compares a digest of what a position threshold cannot see (`oag_net::weapon_sync::digest`:
   the projectiles as a set of kind, owner and target, the Quake, the LeachBeam, the Repulsers,
   which craft hold a lock, each craft's held pickup, shield in whole steps) between the present
   it predicted and the server's confirmed state, and corrects on a difference even with every
   craft inside the threshold. Continuous quantities keep ADR-0069's tolerance; discrete ones get
   none.
3. Everything else in ADR-0069 stands: the same replay, the same view-only smoothing.

## Why: measured

Pulse circuit 01, in one process over `Loopback` with a 100 ms round trip (`Shim`), 5,400 ticks,
every client flown by the player's driver (`Client::bot_input`, which fires what it holds), the
AI on the rest of the grid; 0 bad hashes in every run. "Position only" is the trigger before this
ADR (`Client::set_weapon_trigger(false)`).

| Clients | Trigger | Corrections per client | Ticks replayed per client | Confirmed ticks the present had wrong weapon state | Largest weapon pop-in (ticks into flight) | Largest pop (units) |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | position only | 95 to 100 | 760 to 800 | 44 to 88 | 29 | 10.7 |
| 2 | **digest** | 106 to 111 | 848 to 888 | 8 to 10 (all corrected) | 9 | 4.8 |
| 4 | position only | 150 to 159 | 1,198 to 1,271 | 283 to 438 | 95 | 11.9 |
| 4 | **digest** | 163 to 176 | 1,304 to 1,408 | 34 to 40 (all corrected) | 9 | 4.4 |

- **The gap was real.** With a position trigger the present kept a wrong weapon state for 44 to
  438 confirmed ticks (0.7 to 7 s) of a 90 s race, and a missile first appeared up to 95 ticks
  (1.6 s) into its flight.
- **The digest closes it for about 10 percent more corrections** (4 clients: 163 to 176 against
  150 to 159 in 5,400 ticks, about 1.8 per second, 8 replayed ticks each, 0.4 ms of a 16.7 ms
  frame by ADR-0069's own cost) and 6 to 8 percent more replayed ticks.
- **The pop-in is the round trip, not worse:** the oldest missile a correction shows is 9 ticks
  old, the prediction window (confirm time 165 ms). A missile cannot be shown earlier than the
  server's fire reaches this client. A person's *own* fire is predicted at once (their input is
  applied locally); only remote fires pop.
- **A dodge undone** (a hit appearing on the client's own craft after a correction): 0 to 1 per
  client per race with the digest on, 0 to 1 off. A predicted hit undone: 0 to 1. Rare at this
  hit rate (6 to 29 hits a race); not an argument either way.
- **Eliminator** over the wire, 2 people: the race ends on the server, 0 bad hashes, 12 and 9
  weapon mismatches, all corrected.
- **Across builds:** the lean `oag-server` (no `view` feature) against 2 and 4 `oag-game
  --connect --net-bot` clients (view on), 3,600 and 6,000 ticks: no `bad` in any client's log.
  The view-gated code in the fire path writes no simulation state.

## Not solved, named

- **The replay window draws nothing.** `tick_sim_only` drops cues and view effects, so a fire or
  a hit that first appears inside a correction's replayed ticks has no launch flash, no blast and
  no sound; the missile simply exists mid-flight. Smoothing covers only craft. Showing the effect
  late would be a view-side change (an event queue of what the replay newly raised); chosen not
  to be done here.
- Real packet loss and an analog human stream (ADR-0069 conditions 1, 3, 4) are as open as before.
- The 100 ms shim is the only latency measured with weapons.

## Consequences

- A network race's weapons are the room's setting (Omega's `Weapons` row, `RB_WEAP`, default on;
  Eliminator is always on). LAN hosts keep the originals' race: weapons on.
- The digest is client-side only; no wire change beyond the room's `weapons` byte (v1 edited in
  place, nothing is released).
- `SHIELD_STEP` (1 shield unit) is chosen: it separates a hit from contact wear.
