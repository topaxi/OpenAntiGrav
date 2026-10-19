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
- An analog human stream with weapons (ADR-0069 condition 3) is as open as before; loss and jitter are measured above.
- The 100 ms shim is the only latency measured with weapons.

## Follow-up (2026-10-19, still Proposed)

**The replay window now shows what it reveals.** The quiet steps (a client's confirmed-state
advance and a correction's replay) journal the weapon cues, blasts and shield bulges they raise
(`oag_raceplay::catch_up`); drawn ticks log the same keys (what and whose, never where); once per
confirmed batch `Race::settle_presentation` matches the two and starts what the client had not
presented, at its age: the cue on the next drain (within 30 ticks, else dropped), a blast's particle
effect pre-aged by its age (`Stage::pre_age`, within 90 ticks), a shield bulge advanced by its age
(within 20). Matching tolerates 4 ticks. All of those limits are chosen, not measured. A remote fire
first shows in the confirmed step, not in the replay after it, which is why the journal spans both.
Screen flashes and the Plasma and Bomb blast models start fresh, not aged. Pad, lap, countdown and
magstrip cues stay dropped. A mine-trip blast and its hit sparks no longer play during a replay
(they were not gated on `draws()`). `--force-remote-correction TICK:SLOT[:WINDOW][:contact][:fire]`
has the remote person fire a Rocket inside the window: the late report was 3 cues and 3 blasts at
ages up to 28 ticks on Pulse circuit 01, and 0 with catch-up off. The frames differ by almost
nothing (an 8 by 108 pixel sliver at the horizon): the blasts were far ahead of the player's camera,
so the effect is chiefly the sound, which is checked by the unit tests (a fire first seen is started
once and never twice, an already presented one is not repeated, a second correction over the same window
adds nothing), not by ear.

**Packet loss and jitter, weapons on.** Pulse circuit 01, `Shim` at a 100 ms round trip, 5,400 ticks,
digest trigger, same setup as the table above; loss is per datagram each way, jitter is a draw from
0 to 30 ms added per message (datagrams may overtake, the reliable stream stays ordered). 0 bad
hashes in every run. Per client ranges:

| Clients | Loss | Jitter | Corrections | Replayed ticks | Wrong-weapon-state ticks | Largest pop-in (ticks) |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | 0 | 0 | 106 to 111 | 848 to 888 | 8 to 10 | 9 |
| 2 | 1 % | 0 | 102 to 111 | 918 to 999 | 4 to 10 | 10 |
| 2 | 5 % | 0 | 93 to 109 | 935 to 1,094 | 8 to 9 | 11 |
| 2 | 0 | 30 ms | 86 to 126 | 927 to 1,354 | 8 to 10 | 0 |
| 2 | 5 % | 30 ms | 114 to 128 | 1,426 to 1,619 | 4 to 11 | 18 |
| 4 | 0 | 0 | 163 to 176 | 1,304 to 1,408 | 34 to 40 | 9 |
| 4 | 1 % | 0 | 162 to 177 | 1,458 to 1,593 | 29 to 34 | 10 |
| 4 | 5 % | 0 | 156 to 173 | 1,568 to 1,737 | 15 to 22 | 11 (63 once) |
| 4 | 0 | 30 ms | 157 to 182 | 1,670 to 1,937 | 22 to 33 | 12 |
| 4 | 5 % | 30 ms | 179 to 185 | 2,074 to 2,149 | 21 to 34 | 12 to 18 |

Loss costs little: corrections stay near 1.8 a second, and the redundancy of the input datagrams
absorbs it. Jitter raises replayed ticks by 20 to 40 percent (a longer window per correction) and the
pop-in age by a few ticks. One client at 4 people, 5 % loss saw a missile first appear 63 ticks into
its flight (a resend wait, not seen again); the others stayed at or under 18. Wrong-weapon-state
ticks fall as loss rises, because a lost confirm delays the compared tick rather than changing it;
read that column as "ticks compared", not as an improvement. Late presentations in these runs were
22 to 97 cues and 16 to 62 blasts per client, none older than 15 ticks (63 pop-in aside). The sweep
is `weapons_ground_truth::loss_and_jitter_sweep` (`OAG_SWEEP=1`); `four_firing_clients_agree_through_5_percent_loss_and_jitter`
runs in `just test-data`.

## Consequences

- A network race's weapons are the room's setting (Omega's `Weapons` row, `RB_WEAP`, default on;
  Eliminator is always on). LAN hosts keep the originals' race: weapons on.
- The digest is client-side only; no wire change beyond the room's `weapons` byte (v1 edited in
  place, nothing is released).
- `SHIELD_STEP` (1 shield unit) is chosen: it separates a hit from contact wear.
