# Networking

> **A first server and client exist (2026-10-14)**: `oag-server` and
> `oag-game --connect`, over WebTransport, on localhost (below, "Running a local
> server"). Milestone M8 is otherwise ahead: no web client. Weapons are predicted simulation (ADR-0071). The transport is
> [ADR-0070](../architecture/adr/0070-the-network-transport-is-webtransport.md); the
> prediction is [ADR-0069](../architecture/adr/0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md),
> still Proposed.

## Scope: two separate things

1. **Multiplayer for this engine, up to 8 players.** Ours, designed from
   scratch. It is not a port and does not try to be compatible with any
   original.
2. **The originals' own local features**: ad-hoc wireless multiplayer, game
   sharing (`GSHARE/SHARE.BIN`) and downloadable content. Recreating those is a
   separate reverse-engineering exercise that nothing in (1) waits on.

## What the originals did

- **Pulse (PSP) links the full network stack.** `BOOT.BIN`'s import table
  resolves `sceNetInet` 18 of 18, `sceNetAdhoc` 7 of 7, `sceNetApctl` 6 of 6,
  `sceHttp` 5 of 7 and `sceSsl`, plus the `sceNp` trio (unresolved only
  because NP uses a different NID scheme). Evidence and caveats:
  [`imports.md`](../ghidra/functions/psp-pulse-usa/imports.md), confidence
  **99**. That page reads UCUS-98712 (USA), while the PRX list below is
  UCES00465 (EU); `just resolve-imports` against the EU import would close it.
- **This page used to say the opposite**, at confidence 90, from the disc's
  PRX list alone (three modules: `libfont`, `libmp3`, `pspnet_ap_dialog_dummy`;
  Pure ships 26). That inference was wrong: a module loaded from firmware
  leaves no file on the disc. The PRX list is evidence about what is on the
  disc, not about what the binary calls.
- The disc carries `UCES00465/GSHARE/SHARE.BIN`, a 6.6 MiB PBP, almost certainly
  the game-sharing payload. See [PSP disc layout](../psp/pulse-disc-layout.md).
- **Nobody has read how any original synchronised craft over the wire**, and
  nothing identified so far is worth reading. Reverse-engineering in this area
  is justified only if it can teach something important; our protocol is not
  constrained by theirs.
- **The originals cannot be a model for the sim side.** None runs a fixed
  tick: Pulse and Pure integrate a measured `dt`, HD sub-steps at 60 Hz inside
  a variable frame, 2048 races at 30 fps. See
  [determinism](../architecture/determinism.md#the-tick-rate-and-how-the-original-differs).
  A variable `dt` cannot be run in lockstep, so any sync they did was almost
  certainly state-based, with each console simulating its own craft. That is an
  inference, confidence **40**, not a measurement.

## Decided

Maintainer decisions, 2026-09-16 (the protocol) and 2026-10-13 (the trust
model):

1. **Tick-based, not time-based.** The fixed 60 Hz step of
   [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md). Inputs
   are tagged with a tick number. Determinism ([ADR-0002](../architecture/adr/0002-determinism-model.md))
   is what makes prediction and replay trustworthy.
2. **Online play is server-authoritative.** The server runs the canonical
   `RaceSim`, accepts `InputSnapshot`s only (never client-reported state) and
   re-derives press and release edges from a held-state stream it advances
   itself.
3. **One server role, two deployments.** A headless dedicated server for
   online, and a listen server inside a client for LAN. A LAN host is trusted
   and gets no cheat defence: cheating there is a social problem.
4. **Clients predict and reconcile by replay.** A client runs the same tick
   ahead of the server's acknowledged state, and on an authoritative snapshot
   restores to it and replays its buffered inputs. The `RaceSim` snapshot and
   restore pair and a loopback proof exist (`crates/raceplay/src/reconcile.rs`).
5. **Up to 8 players**, the existing `MAX_SHIPS` grid, so a networked race never
   exceeds what a local one handles ([ADR-0052](../architecture/adr/0052-world-and-race-tick-widen-to-n-players.md)).
   `Race::tick` steps every person-flown slot (`World::person_flown`) since
   2026-10-13, so a remote slot's input reaches its own craft; ADR-0052's
   "the player step still runs once" consequence no longer holds.
6. **Mods may change anything**, simulation and race modes included. The
   server advertises its rules and content fingerprint and the client shows it
   on join. Mod code arriving through a lobby is never auto-run.

## Open, gated on measurement: remote craft, rolled back or interpolated

The one open design question. Interpolating remote craft in the past is the
usual shooter answer, but it fits games where players rarely change each
other's motion tick to tick. A racing pack does: ship contact and weapon hits
change a craft's motion on contact, and a predicted local craft meeting an
interpolated remote one resolves against a state close to a round trip stale.

**Status: measured on the desktop, 2026-10-14; the maintainer decides.** M1 to M4
are in ("Reconciliation cost", "Remote-input predictability", "What a correction
looks like" below) and favour the recommendation, with two conditions still unmet
(the Galaxy S24 cost and weapons). [ADR-0069](../architecture/adr/0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md)
is drafted as Proposed. The options, outcomes and measurements are in the handover
thread for this work.

**Current recommendation, open to challenge:** predict every craft in one
present timeline, the local one with its real input and each remote one with its
last known input, and roll back and replay only when an authoritative state
differs by a large amount. The server buffers each client's input and consumes
one per tick. No client input delay by default. Draw the correction smoothed,
renderer-side only. This is the same machinery as decision 4 applied to every
craft, so it adds a policy, not a second mechanism. Interpolation stays as the
fallback for a stalled link. Server authority sits on top of either choice.

The best evidence is Rocket League's own GDC 2018 slides (below). They
interpolated the ball, found the car and the ball then lived in two timelines
that disagreed on a hit, and shipped predict-everything. The same slides call
remote cars the weak case and price a correction at 24 replayed frames for 200 ms
of ping at 120 Hz. Rocket League has one ball, no weapons and simple bodies.
Nothing here has been measured on this engine.

Costs to measure before committing, with what is known (2026-10-14; M1 and M4
measured, see below):

- The replay cost: **measured on the desktop, below**. The Galaxy S24 half is
  open.
- A sim-only tick path: **landed**, `Race::tick_sim_only`
  (`crates/raceplay/src/reconcile.rs`). See "The sim-only replay tick".
- `TickClock::advance` caps catch-up at 8 ticks and discards the surplus, which
  a replay burst exceeds above roughly 133 ms of round trip. **It does not touch
  the sim-only path**: a replay is a count of ticks owed, not elapsed time, so
  it is a plain loop and never goes through the clock. See below.

## The sim-only replay tick

`Race::tick_sim_only(&inputs)` advances `RaceSim` exactly as `Race::tick` does
and leaves the picture and sound alone: no camera, shake, particle or exhaust
step, no view RNG draw (`sparks_rng`, `shake_rng`, `stage_rng`, `exhaust_rng`,
`engine_light_rng`, `leach_beam_rng`), no HUD, no recording, and the cues,
announcements and class announcements it raised are dropped. The hash after N of
these equals the hash after N normal ticks from the same snapshot, on every
tick and every slot, in every scenario pinned
(`tests::reconcile::a_quiet_replay_reaches_the_hash_of_a_normal_one`, which also
compares the whole `RaceSim` as text outside the hash, and on a real circuit with
eight craft, two of them people, weapons in every hand, `quiet_replay_ground_truth`),
**with no exception**: the lock a Missile or LeachBeam fires at is simulation
state, below.

**A mode on the one body, not a second entry point.** View writes sit inside the
simulation's helpers (a craft's step flashes its shield, a pad crossing lights
the plume, a respawn clears the trail), so a second entry point would be a
700-line copy of `tick` or a split of a dozen helpers, and `Race::tick` with the
mode unset runs the same instructions in the same order as before. The three cue
queues are truncated to their entry length rather than gating twenty-odd push
sites, because a cue's bookkeeping (`contact_cue_cooldown`) is sim state.

**The lock is simulation state (2026-10-14, lane `sim-side-lock`).** A Missile
or LeachBeam fires at the craft its holder has *locked*, and a lock is a reticle
held on a target for a time. It used to be the primary's on-screen reticle, advanced
from the chase camera, so the simulation read the view: a replay was exact only as
far as the reticle it started from, and a server with no camera could disagree with
its client about a lock with no misprediction at all. Now `RaceSim::locks` holds one
`oag_race::sight::Sight` per slot, advanced by `Race::advance_locks`
(`crates/raceplay/src/lock.rs`) for every person-flown slot (the primary and any
second pad alike) from the world alone, and hashed while the slot holds a Missile or
a LeachBeam (a race holding neither hashes as before). The law is the recovered
reticle law, unchanged (the Missile's `0.8` s hold and arrival, the Pulse
LeachBeam's `0.34` s closing); what it is *fed* is **chosen, not measured**: a fixed
virtual camera, the team's far chase rig at rest at the authored field of view plus
the speed widen, in the title's HUD grid - no spring lag, shake, boost kick,
field-of-view setting or cockpit view, which is what the picture's reticle still has
(it advances only on a drawn tick, through the real camera, and is told the lock by
`Sight::follow`). Timing: at rest the first lock is tick 48, 20 and 48 for the
Missile, the Pulse LeachBeam and Wipeout HD's LeachBeam, as before the change; over
3600 ticks of an autopilot race the LeachBeam lock is identical tick for tick and the
Missile's agrees on 96% of ticks, differing in twelve windows of 1 to 37 ticks (the
sim lock is held 146 ticks more and re-forms less often, since the real camera's eye
lag makes the old reticle drop after a target switch or in a bend).
The grid and the law are configuration the lock reads. Since 2026-10-14 they are
`Setup::sight` (`oag_raceplay::Sighting`), taken from the title's HUD at load and
applied by `Race::start`, so a server or a replay player is correct with no extra
call; the former `set_sight_screen` and `set_sight_dialect` setters are gone (a
race started without them ran Pulse's LeachBeam under the Missile's law, locking
at 48 ticks instead of 20). The simulation's locks are in the title's whole HUD
space (`Sighting::lock_screen`) and never in a split view: the view grid
(`view_screen`) is a fact about this machine's display, and the locks are hashed.
That moves one thing: a race loaded for an authored two-view split used to run its
locks in one view's grid and now runs them in the whole space (single-view races
are identical, by construction and by every lock and race ground truth). Opponents have no lock (their shot takes the
window's candidate at once), unchanged. Pinned by `tests::lock` (the camera cannot
move the lock, a quiet replay locks exactly as a normal tick does) and the
`quiet_replay_ground_truth` tests, which hold with no exception.

**Not covered by the sim-only tick:** a recording in progress does not hear the
replayed ticks, so it keeps the inputs as first predicted; `restore_sim` puts
back whatever cues were undrained at the snapshot, which a client that drained
them since would hear twice; and `capture_results` and `wreck_ended_tick` still
run, because a caller reads them back.

## Reconciliation cost (2026-10-14, desktop)

Section 3a's M2 (replay cost per tick) and M3 (snapshot cost). The Galaxy S24
half is **open**.

Method. `tests::reconcile_cost::reconcile_cost_sweep` (`crates/raceplay/src/tests/reconcile_cost.rs`,
`#[ignore]`d, runs only with `OAG_SWEEP=1`). Pulse, Single Race, VENOM, all 12
forward circuits, 8 craft (one person on the throttle, seven AI), weapons on,
timed after the countdown plus 600 ticks. Each timing is the median of 7
repeats of 240 ticks, normal and sim-only alternating which goes first; the
whole sweep was run 3 times, one fresh process per circuit, and the table is the
median over circuits of the median over runs. Profile `dev`, where this
workspace's crates are `opt-level 2` and not LTO; the shipping `dist` profile
was not measured. Machine: AMD Ryzen 9 7900, 24 threads, shared; 1-minute load
average 10.8 to 14.1 at the end of the three passes (4.9 to 8 on the
single-process spot runs before them), so read every figure as inflated by
contention. Heap bytes are the resident-set delta of 64 copies held at once in a
fresh process, so they are approximate.

| | median over circuits | range over circuits |
| --- | --- | --- |
| Tick, `Race::tick` | 65 us | 60 - 87 us |
| Tick, `Race::tick_sim_only` | 60 us | 54 - 75 us |
| Rollback of 6 ticks (100 ms), restore plus sim-only replay | 0.42 ms | 0.39 - 0.53 ms |
| Rollback of 12 ticks (200 ms) | 0.77 ms | 0.71 - 0.98 ms |
| Rollback of 24 ticks (400 ms) | 1.50 ms | 1.37 - 1.87 ms |
| The same 6 / 12 / 24 through `Race::tick` | 0.44 / 0.82 / 1.62 ms | 0.42 - 0.60 / 0.77 - 1.09 / 1.50 - 2.11 ms |
| `snapshot_sim()` (whole `RaceSim` clone) | 63 us, 1.7 MiB | 48 - 86 us, 1.4 - 2.6 MiB |
| The immutable track half of it | 47 us, 0.74 MiB | 35 - 52 us, 0.58 - 0.84 MiB |

What it says.

- **The sim dominates the tick.** Skipping the view's work saves 5 us of 65
  (7.5 percent) on a typical circuit and up to 23 percent on `07_Track`. The
  sim-only path is correct for the camera, particles and sound, not a large
  speed-up.
- **A rollback is cheap on the desktop.** 6 ticks cost 0.42 ms, 2.5 percent of a
  16.7 ms frame; 12 ticks 4.6 percent; 24 ticks 9 percent. Section 3a's proposed
  pass condition (a 100 ms round trip costs at most 25 percent of a frame on the
  S24) holds on the S24 if that phone is no more than about 10 times slower than
  this desktop for this code. **Unmeasured.**
- **Worst-case rollback rate.** Section 3a's expectation of about 60 rollbacks a
  second of 6 ticks is 360 replayed ticks a second, about 22 ms of CPU per
  second, 2.2 percent of one core. M1 (how often a real stick forces a rollback)
  is still unmeasured.
- **A snapshot costs as much as a tick** (63 us against 65 us). A client that
  snapshots every tick doubles its simulation CPU; one that restores from the
  server's snapshot and snapshots only on demand does not. The track half is 75
  percent of the clone's time and 44 percent of its bytes, so splitting it would
  take a snapshot from 63 to about 16 us. That is the lever to reach for if a
  design snapshots per tick; the whole-struct clone stays until one does. A
  24-tick window of per-tick snapshots would hold about 41 MiB.

Reproduce: `OAG_SWEEP=1 OAG_SWEEP_TRACKS=09_Track OAG_REQUIRE_GAME_DATA=1 cargo
nextest run -p oag-raceplay --run-ignored all --no-capture -E 'test(/reconcile_cost/)'`
(one circuit per process; omit `OAG_SWEEP_TRACKS` for all twelve, with heap
figures that then read low after the first).

## Remote-input predictability (2026-10-14, desktop)

Section 3a's M1. Counts are deterministic, shown rather than assumed: the full
AI sweep and the full human sweep were each run twice and the two tables are
byte-identical; only timings, which this sweep does not take, would need repeats.
One-minute load average at the start of each circuit, from the logs: 12.9 to 66.2
(AI sweep, both runs), 12.9 to 43.1 (human sweep, both runs); the other members'
builds and tests were the load.

**Method.** `tests::reconcile_predict` (`crates/raceplay/src/tests/reconcile_predict.rs`,
`#[ignore]`d, `OAG_SWEEP=1`; `OAG_SWEEP_GHOSTS` names a directory of `.oagr` ghost
files for the human half). A server runs the race on the true inputs and a client
runs it ahead: its own craft on the true input, every remote craft on that craft's
last input it has heard of, which is `window` ticks old (3, 6 and 12 ticks: 50, 100
and 200 ms). Every tick a packet carries the server's state after tick `t - window +
1`; the client compares it with what it had and, under the policy, restores the
server's state and replays `window` ticks through `Race::tick_sim_only`. In-process,
lossless; the latency is the only network. Policies: `every packet` corrects when
the state hash differs at all; `threshold d` when some craft is more than `d` world
units from the server's (**chosen, not measured**: 0.25, 1 and 4; a craft is 13
units long and 5.5 wide). Scored from the countdown's release, 1,500 ticks on each of
the 12 Pulse circuits (VENOM, Single Race, 8 craft). A correction is checked to land
on the server's exact hash (`bad corrections`: 0 in every row).

**Where the input comes from, and what that means.**

- **AI-derived streams** (`ai7`: 1 own craft and 7 remote; `ai1`: 7 own, 1 remote).
  The driver's controls recorded off an all-autopilot run and replayed as
  `InputSnapshot`s, thrust binarised the way a button is (26 to 41 percent of
  ticks, by circuit, had a fractional thrust). Continuous and reactive: about 10,000 distinct `stick_x`
  values over the 10,500 scored remote ticks of a circuit, 1 to 5 percent of them at
  full deflection, and the next input differs from the last on 96 to 99 percent of
  ticks (the next 3, 6 and 12: 97 to 100 percent). This is **a proxy for an analog stick, not a
  recording of one**; no recording of eight people on one circuit exists.
- **The maintainer's four ghost files** (Pulse, three circuits, 1,878 to 3,997
  scored ticks each). A census of `stick_x` finds **three distinct values in every
  file (-1, 0, +1) and 33 to 39 percent of ticks at full deflection: these were
  flown on a keyboard or d-pad, not an analog stick.** The next input differs from
  the last on 4 to 8 percent of ticks; the next 3, 6 and 12 differ on 13 to 22, 25 to
  40 and 45 to 63 percent. They are replayed alone as recorded and as slot 0 of a
  pack whose other seven craft run the AI-derived streams.
- **Weapons are absent**: a replayed stream never presses fire, so a pack here has
  walls and contact but no missiles (option D is untouched). No packet loss, jitter
  or server input buffer.

**Results** (pooled over the 12 circuits for `ai`; a range over the 4 files for
`human`). Corrections per second of racing, then the sim cost
of the replays as a multiple of the forward simulation (`1 + replayed ticks / 60`),
for windows 3 / 6 / 12.

| stream | policy | corrections per second | sim cost x |
| --- | --- | --- | --- |
| ai7 (7 remote) | every packet | 53 / 53 / 53 | 3.7 / 6.3 / 11.6 |
| | threshold 0.25 | 2.7 / 3.0 / 3.2 | 1.13 / 1.30 / 1.65 |
| | threshold 1 | 1.4 / 1.6 / 1.8 | 1.07 / 1.15 / 1.36 |
| | threshold 4 | 0.65 / 0.78 / 0.99 | 1.03 / 1.08 / 1.20 |
| ai1 (1 remote) | every packet | 25 / 25 / 25 | 2.3 / 3.5 / 6.1 |
| | threshold 1 | 0.59 / 0.67 / 0.82 | 1.03 / 1.07 / 1.16 |
| human (keyboard), alone | every packet | 2.6 to 4.8, all windows | 1.1 to 1.2 / 1.3 to 1.5 / 1.5 to 2.0 |
| | threshold 1 | 0.4 to 1.5 / 0.5 to 1.6 / 0.8 to 1.8 | at most 1.4 |
| human (keyboard), in a pack | every packet | 2.6 to 4.8, all windows | as alone |

Position error against the server, in world units (a craft covers about 2 units a
tick at 120 units/s, so 2 units is one tick of travel). Remote craft, p95 over ticks,
free-running (not in contact within the window), for windows 3 / 6 / 12:

| stream | every packet | threshold 1 | threshold 4 |
| --- | --- | --- | --- |
| ai7 | 0.01 / 0.04 / 0.21 | 0.66 / 0.82 / 1.46 | 2.6 / 2.8 / 3.7 |
| ai1 | 0.01 / 0.04 / 0.23 | 0.94 / 1.08 / 1.82 | 3.4 / 3.8 / 4.9 |
| human, alone | 0.00 to 0.01 / 0.01 to 0.08 / 0.14 to 0.41 | 0.86 to 1.12 / 0.99 to 1.84 / 1.7 to 4.8 | 3.0 to 3.9 / 3.3 to 5.0 / 4.3 to 9.8 |

The size of the correction a player would see (the pop, p95): every packet 0.16 /
0.49 / 1.65 units (ai7), threshold 1 1.7 / 2.5 / 5.3 (ai7), threshold 4 5.9 / 7.9 /
13.8. The player's own craft, shoved only through contact: p95 at most 0.07 unit in `ai7`
under every policy and 0.0 to 0.6 in `ai1` (the 0.4 to 0.6 under threshold 4); the
maxima of 3 to 17 units are contact. Contact ticks (a craft pair resolved on the server's tick) are
45 percent of the scored ticks in the AI packs.

**What it says.**

- **Last-input prediction is nearly free for digital input and a storm for analog
  input.** The keyboard ghosts need 2.6 to 4.8 corrections a second under the strict
  policy at every window, 1.1 to 2.0 times the simulation. The AI-derived streams,
  which change every tick, need 53 a second for seven remote craft (11.6 times the
  simulation at 200 ms). Section 3a's risk (1), "an analog stick changes almost every
  tick", is real **for an analog stick**, and these ghosts cannot say how often that
  is: the proxy is a driver, which is more reactive than a hand.
- **A state threshold removes the storm.** At 1 unit (0.08 of a craft's length, half
  a tick of travel) the seven-remote pack needs 1.4 to 1.8 corrections a second and
  1.07 to 1.36 times the simulation, with a free-running remote error p95 of 0.7 to
  1.5 units. The error the threshold leaves is below one tick of travel; the
  correction pop is 1.7 to 5 units. This is the policy the recommendation already
  names ("a large difference requires correction"), now with a number to argue about.
- **The cost of the strict policy against the proposed pass condition.** 6.3 times
  the simulation at 100 ms is 0.41 ms a frame on this desktop (65 us a tick), 2.5
  percent of a frame; on a phone 10 times slower it is 25 percent, exactly the
  proposed budget, and 46 percent at 200 ms. Under a threshold it is 1.1 to 1.4 times,
  negligible. **The S24 itself is still unmeasured.**
- **Outliers.** The maxima of 257 to 289 units under thresholds 1 and 4 are craft
  respawned on one timeline only, drawn wrong for up to one window until the next
  packet crosses the threshold. They are what `correction::SNAP_DISTANCE` (20 units)
  is for: a correction that large is drawn where it lands. They are rare (the p95 is
  below 4 units) and are a discrete-event problem, not an input-prediction one.
- **Only discrete events make a big error.** A sideshift moves a craft about 4.5
  units; contact, a respawn and a wreck move it more. A stick or a thrust held wrong
  for 200 ms moves it under 1 unit.

Reproduce: `OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay
--run-ignored all --no-capture -E 'test(/reconcile_predict_ai_sweep/)'` (about 11
minutes on a loaded machine; `OAG_SWEEP_TRACKS=09_Track` for one circuit), and the same
with `reconcile_predict_human_sweep` and `OAG_SWEEP_GHOSTS=<a directory of .oagr
files saved by the game's ghost recording>`.

**Not covered:** a recorded human analog stream (answered by a proxy, "An analog
stick, 2026-10-19" below, until one is flown), weapons (ADR-0071), loss and jitter
(ADR-0071's follow-up), the S24, and a threshold in effect rather than in units
(section 3a's last question).

## What a correction looks like (2026-10-14)

Section 3a's M4. `Race::reconcile` (`crates/raceplay/src/correction.rs`) is the
client's correction: restore the server's state and replay the ticks since through
`tick_sim_only`. When the guess was wrong a craft ends the replay somewhere else, and
drawn as it is that is a pop. The smoothing is **a view-only offset that decays**:
the craft stays drawn where it was (`before - after`) and the offset loses half
every 6 ticks (100 ms, **chosen, not measured**; the originals have no rollback),
added where a craft is drawn (`drawn_model`, `drawn_position`) and nowhere the
simulation reads. A correction over 20 units (**chosen**) is drawn where it lands.
**A person's craft glides, this machine's own included** (since 2026-10-19; see "The
own craft's correction" below): the camera follows the same offset, because offsetting
the hull alone draws it off its own chase camera (seen: the hull slid left of the
camera's centre for the length of the glide in the first render). The hash is identical with
smoothing on or off (`tests::correction`), and a replay is not a drawn tick.

**The scene.** `oag-game --race --opponents --hold cross --scheme veteran --size 960x540
--ticks N --force-remote-correction TICK:SLOT[:WINDOW[:contact][:fire][:own[=UNITS]]] [--no-correction-smoothing]
--screenshot out.png`. At `TICK` craft `SLOT` is handed to a scripted remote person
who double-taps its airbrake toward the player (a **sideshift**, the input a last-input
guess cannot see coming: a pressed edge, two within a quarter second); the client
guesses it straight on for `WINDOW` ticks (default 6), then the server's state arrives
and the client corrects. With `contact` the player dashes at it too, so the two touch
on the true timeline and not on the guessed one. A frame is a function of the tick
count, so a sequence is one run per `--ticks`. **No capture of the original shows a
correction (it has none); there is no reference frame.**

- **A, the rival pops** (`325:7:12`, Pulse circuit 01, slot 7 beside the player at
  the left edge of the chase view): the rival moves 4.5 units at the correction.
  Without smoothing it jumps about 70 px right in one frame at 960x540 (measured as
  the centroid of its purple hull in the crop, approximate: it moves 60 to 78 px
  between frames 337 and 338 and the centroid is noisy); with it, it stays where the
  last frame drew it (the same crop at 337 and 338 shows a 10 to 20 px step) and
  glides across over about 8 frames, about 45 px short of the unsmoothed one still at
  frame 345. Read by eye on a contact sheet of the crops (the left column pops, the
  right column glides). A 1 unit pop is about 15 px at that distance and would not be
  noticed against 2 units of travel a tick.
- **B, contact** (`--mode single_race 480:7:16:contact`, window 16 is the shortest of 12/16/20 where
  the touch exists: own craft 2.1 units at 16, 3.6 at 20): the rival is **out of shot**
  behind a trackside barrier, so the picture shows only the player's own craft and the
  camera jerked by 2.1 units, which is not smoothed by the rule above and which the
  player's own dash makes impossible to separate by eye from the contact. The contact
  case is judged by the numbers (own craft 2 to 5 units) and not the picture.

**Reading.** The smoothing is worth having for a remote craft near the camera and
costs a `Vec3` per craft; below about 2 units nothing needs it. The own craft was the
open case: its correction moves the camera. Built 2026-10-19, below.

### The own craft's correction (2026-10-19)

**Condition 4 of ADR-0069.** What was wrong. A correction that moves the player's own
craft moved the camera's *aim* in one tick: the chase camera's eye is a sprung point
that stays, and it looks at the craft's new position, so a 3-unit sideways correction
swung the view **12.1 degrees** in one frame (`tests::correction`, the measure is the
angle between the camera's forward before and after the correction; the eye itself
moves by about 1e-6). 12 degrees is a quarter of the screen's width at 960x540.

What was built. `Race::camera_target` is the one place the chase camera reads its craft
from; it is the craft's **drawn** pose (the simulation's plus the correction glide), so
the hull and the camera that frames it take the glide together and the old failure (the
hull off its own camera's centre) cannot come back. `reconcile` now accumulates the
offset for the seat's own craft too and decays it on the same half-life of 6 ticks
(**chosen**), with the same 20-unit snap: a correction past it (a respawn, a wreck) is
drawn where it lands, so a real knock-out is never glided. The hash is identical with
smoothing on and off.

**A hit is not softened.** The own craft's input is known exactly, so what moves it in a
correction is an interaction (a contact the guess missed, a weapon), and a hit first
shown by the correction must not glide. A correction that takes more than 1 shield unit
off the seat's own craft (`correction::HIT_SHIELD`, ADR-0071's `SHIELD_STEP`,
**chosen**) draws the craft where it lands, and a smaller loss (contact wear) glides
(`a_hit_that_first_shows_in_a_correction_is_not_softened`). In the wire runs below such
corrections are rare: `own hit added` was 2 in 16 client runs of 4 clients (and 0 in the
four 2-client runs), against 1 to 27 own-craft moves per client. A hit first
seen inside a correction's replay arms the camera shake once, at the correction, where the hit is
drawn (2026-10-19, `reconcile`: the replay is quiet and `settle_presentation` covers cues, blasts
and bulges only, so the shake had no other way in). The shake is a weapon hit's (`0.6`, elsewhere),
**chosen, not measured**: a correction cannot tell what struck. The same loss in a second
correction arms nothing (`a_hit_seen_in_a_correction_shakes_once`).

**What follows the drawn pose.** Read in the view modules, then looked at in the frames:
the hull (`drawn_model`), the shadow (`body_between`), the exhaust nozzle and its trail
(`nozzle_of` reads the drawn model matrix) and the camera. `hull_reach` is now measured on
the drawn pose too. **Still on the simulation's pose, listed rather than fixed**:
nothing now: `craft_flash`'s and `damage_fx`'s positions (the smoke following a damaged hull, the
leach spark, the wreck flash and blast) moved to the drawn pose on 2026-10-19
(`the_smoke_rides_the_drawn_pose` fails with the sim pose put back; the wreck flash has no test of
its own).
The frames at ticks 501 and 504 (read at 960x540) show the hull, its shadow and its engine
bloom together with no visible separation; a 3-unit offset is a quarter of a craft's width.

**A bug found on the way, fixed in the same change.** The chase camera was advanced
toward `ships[0]` (and snapped to it) whatever seat the client had, while its view was
built from the seat's craft. A client in seat 1 had its camera spring toward slot 0.
On the test grid the eye stood 555 units from seat 1's craft before and 4.5 units after
(`a_seat_one_camera_follows_seat_one`); `set_seat` now snaps the camera to the seat when the craft it follows changes (a seat that
names the craft already followed is no cut). View only, no hash moves. **A real online seat 1 did show it** (2026-10-19): an `oag-server --people 2` and two
`oag-game --net-bot` clients on loopback, the seat-1 client's still at tick 700 on the
parent of the first netcode-conditions commit has the craft small and far from the framing
(the camera chases slot 0's craft, one place ahead), and on this branch it is framed as seat 0's
is. The error is the distance between the two craft, so it is largest when they are far apart
and small on the grid; `every_seat_camera_follows_its_own_craft` (both seats, close, far and
cockpit views) and `a_seat_one_camera_follows_seat_one` fail if the camera follows another slot.
The same sweep found `visibility_sections` placing the camera by slot 0's respawn (a seat-1 shot
went to "draw everything" while a rival recovered in slot 0, and a seat-1 craft's own recovery was
not seen), the three audio levels (`shield_is_up`, `craft_is_exploding`, `autopilot_is_active`)
reading slot 0, and the view-only readers of `primary_slot` (the first person, which is slot 0 on every
network client): the weapon-hit shake, the destroyed-craft shake cancel and blast shake, the
PLAYER_DESTROYED flash, the reset flash, the destroy camera (a seat-1 client's wreck camera never
started) and the speed-pad boost pulse; all now read the seat (`a_seated_view_is_placed_by_its_own_respawn`, `a_seated_hit_shakes_the_seats_camera`; each fails with the call put back). Split screen and local windows are unchanged: `player_slot` is the primary slot there.
**The wall-contact effects follow the seat too** (2026-10-19, `seat1-contact-fx`). `advance_contact_fx`
(collision sparks, wall shake, the shield-up edge) ran on `primary_slot`'s step and read `ships[0]`, a literal
`ship_model_matrix()` and a literal slot 0 in its cues; it now takes the seat's slot and step
(`player_slot`, falling back to the primary when the seat is not person-flown). The contact *sound* is
raised for every person-flown slot in slot order, drawn tick or replayed tick alike
(`raise_contact_cue(slot, ..)`, each with its own `shielded`), so which seat a machine holds cannot change
which cooldowns tick; `contact_cue_cooldown` is **not** in `state_hash` (`RaceSim::contact_cue_cooldown`'s
doc comment, and `hash.rs` does not name it), so no hash can move, and a one-person race flies slot 0 alone
and runs as before. A second local person (split screen, multi-window) now also sounds their own wall hits,
which they did not before; their sparks and shake stay unwired (local seats draw through `set_viewpoint`
and the contact fx there still follow the primary view, open). Also moved to the seat: the countdown voice's
cue slot, the Missile and Leach reticle (`update_sight`, `sight_screen_of`, the sight screen fit; a seated
client skips the per-slot split-screen reticles), the zone announcements and run statistics gate
(`advance_player_rules`), the wreck-ended tick, the boost FOV kick, the airbrake flaps, and the engine
crossfade's throttle in `oag_game::sound::race_tick` (it read `ships.first()`). Tests, each failing with its
read put back: `a_seated_client_gets_its_own_wall_sparks_shake_and_sound` (seats 1 to 3 of four people),
`a_seated_client_is_not_shaken_by_another_slots_wall`, `the_contact_cue_cadence_does_not_depend_on_the_seat`,
`the_start_voice_is_the_seats`, `a_seated_reticle_is_drawn_for_the_seats_weapon`,
`the_boost_kick_and_the_flaps_follow_the_seats_craft` (`tests/seat_view.rs`). Not tested: the zone
announcement gate, the wreck-ended tick and the engine throttle. On a real `--people 2` loopback run the
seat-1 client's still two ticks after its own wall hit shows the sparks at its own craft's wing, the view
tilted by the shake and the shield bar red; the tick before shows neither.

Frames: `oag-game --race --opponents --hold cross --scheme veteran --size 960x540
--ticks N --force-remote-correction 480:7:20:own=3 [--no-correction-smoothing]
--screenshot out.png`, with `own[=UNITS]` new: the server's copy of the player's craft
is that many units to its right when the correction arrives (the older `contact` scene stopped moving the own craft on 2026-10-14, at the commit that made
Time Trial with two or more people Pulse's Multiplayer Time Trial: the scene hands the rival to
a "remote person", which makes two people, and craft do not collide in that mode. Found by
bisecting `480:7:16:contact`: own craft 2.11 before, 0.00 from that commit. Not a bug: run the
scene with `--mode single_race` and it moves 2.11 units again at window 16 exactly as recorded, and
the scene now warns when its mode has no collisions). The correction is drawn at tick 501. Ticks 500 to 504,
smoothing off (top row) against on (bottom row): off, the view lurches at 501 (the track
edge, the horizon and the hull all jump sideways and the rival appears at the left edge);
on, the four frames are one continuous sweep and the hull stays centred. No reference
frame exists (the originals have no rollback). 3 units is **chosen**: it is the size of
the typical own-craft pops measured below (most under 2 units, the largest 8.7).

## An analog stick (2026-10-19)

**Condition 3 of ADR-0069.** The proxy, its shape, what it costs, against the keyboard
ghosts and the driver-derived streams above. **The stream is chosen, not measured:** no
recording of an analog stick exists, and no pad is attached to the machine this was run
on (`/proc/bus/input/devices`, read only), so a recorded one is still owed: a controller
player flying a pack through `--record-ghost`, replayed by the same sweeps.

**The hand** (`oag_raceplay::hand::Hand`, the one definition both sweeps use). The
driver's wants through: a **second-order stick** (4 Hz natural frequency, damping 0.6,
about 10 percent overshoot, the thumb's mass and spring), **tremor** (a seeded 10 Hz sine
plus white noise, 1.2 percent of the range each), the pad layer's **0.1 deadzone** with
the rest rescaled, **8-bit axes** (1/127 steps), airbrakes as **analog triggers** (a first
order lag of 3 ticks) and thrust as Cross held through the countdown. It is closed loop:
the stream is recorded from a race the filtered stick *flew*, not filtered afterwards, so
the craft still drives the circuit (craft 0 travelled 3,094 units in 1,500 ticks of
circuit 01 against 3,097 on the driver's raw wants).

**What was tried and dropped: a reaction delay.** The first model had a pure delay of 8
ticks (133 ms, the motor loop). Closed around this driver, which is designed with no
delay, it limit-cycles against the stops: 50 percent of ticks in the top tenth of the
stick's range at 8 ticks, 35 percent at 4, 19 percent at 2, against 3 percent for the
driver's own wants and for the shipped model. A person's reaction time is absorbed by
looking ahead, which the driver already does, so the delay is out; this is a result about
the driver as much as about hands. Census of the shipped stream, circuit 01, 7 remote:
256 distinct `stick_x` values over 10,500 ticks (the AI-derived stream: 10,315, the
keyboard ghosts: 3), 1.3 percent of ticks at full deflection (AI 1.8, keyboard 33 to 39),
the next input differs from the last on 87 percent of ticks (AI 99, keyboard 4 to 8), mean
change of the stick per tick 0.046 (AI 0.069, which the hand halves). **So the hand is a
smoothed driver, smoother than the AI-derived stream and far busier than the keyboard**:
a stick in a real hand can be calmer than this (long holds in a bend) or busier (jabs),
and this proxy cannot say which.

**Prediction sweep** (`reconcile_predict_ai_sweep` with `OAG_SWEEP_HAND=only`; the same
harness as above, all 12 circuits, 1,500 ticks, 8 craft, no weapons, a lossless link).
Corrections per second, pooled over 12 circuits, windows 3 / 6 / 12 (50, 100, 200 ms);
the ADR's AI-derived figures beside, from "Remote-input predictability":

| stream | policy | corrections per second | sim cost x | the AI-derived stream |
| --- | --- | --- | --- | --- |
| hand7 (7 remote) | every packet | 52.2 / 52.2 / 52.2 | 3.6 / 6.2 / 11.4 | 53 / 53 / 53, 3.7 / 6.3 / 11.6 |
| | threshold 0.25 | 3.5 / 3.9 / 4.0 | 1.18 / 1.39 / 1.80 | 2.7 / 3.0 / 3.2 |
| | threshold 1 | 1.7 / 2.2 / 2.5 | 1.08 / 1.22 / 1.50 | 1.4 / 1.6 / 1.8, 1.07 / 1.15 / 1.36 |
| | threshold 4 | 0.80 / 1.0 / 1.3 | 1.04 / 1.10 / 1.26 | 0.65 / 0.78 / 0.99 |
| hand1 (1 remote) | every packet | 26 / 26 / 26 | 2.3 / 3.6 / 6.2 | 25 / 25 / 25 |
| | threshold 1 | 0.87 / 1.1 / 1.4 | 1.04 / 1.11 / 1.27 | 0.59 / 0.67 / 0.82 |

Free-running remote error p95 at threshold 1: 0.73 / 0.95 / 1.9 units (7 remote; AI
0.66 / 0.82 / 1.5), the correction pop p95 1.6 / 2.5 / 5.4. 0 bad corrections in every
cell. The own craft's error p95 is 0.08 to 0.26 at threshold 1. Contact ticks are 46
percent of the scored ticks (the AI pack's 67 percent on circuit 01), because a hand
bunches the field less than the driver does.

**Reading.** The 53 a second was an upper bound on *strictness*, not on activity: a
stick that is smoother than the driver still needs 52 corrections a second under the
strict policy, because a strict policy corrects on any bit of difference. The threshold
policy is what removes it, and it removes it for the hand as for the driver, by a factor
of 21 to 31. The hand needs **1.2 to 1.4 times the AI-derived stream's corrections** at a
threshold of 1 (1.7 to 2.5 a second against 1.4 to 1.8, seven remote craft), because the
driver's own errors are small and symmetric where a lagging thumb leaves the craft a
little behind its line for longer. ADR-0069's recommendation (threshold 1) holds: at 200
ms it is 1.5 times the simulation, at 100 ms 1.22 times, a remote error p95 of 1 unit,
half a tick of travel.

**Over the wire** (`weapons_ground_truth::sweep_*`, `OAG_SWEEP=1`, the loopback harness
of ADR-0071's follow-up: Pulse circuit 01, 100 ms round trip, 5,400 ticks, weapons on,
every client flown by the driver (`bot`) or the driver through the hand (`hand`), both
in one session on one machine, 0 bad hashes in all 16 client runs). In this harness only
the *people* are predicted from their last input (the AI is the same on both ends), so
the 2-client case has one remote craft. Per client ranges:

| Clients | Loss, jitter | Flown by | Corrections | Replayed ticks | Mean error at a confirmed tick | Largest pop | Own craft moved, own pop max | Largest pop-in age (ticks) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | 0, 0 | bot | 103 to 115 | 820 to 913 | 0.23 to 0.24 | 4.1 to 4.8 | 2 to 7, 1.1 to 1.3 | 9 |
| 2 | 0, 0 | hand | 106 to 108 | 846 to 856 | 0.23 to 0.27 | 3.3 to 4.2 | 7 to 8, 0.7 to 2.0 | 9 |
| 2 | 5 %, 30 ms | bot | 107 to 127 | 1,214 to 1,424 | 0.22 to 0.25 | 6.9 to 7.2 | 5, 0.4 to 2.7 | 0 |
| 2 | 5 %, 30 ms | hand | 120 to 122 | 1,322 to 1,335 | 0.21 to 0.25 | 5.9 to 8.3 | 10 to 11, 2.3 to 2.7 | 12 (137 once) |
| 4 | 0, 0 | bot | 158 to 167 | 1,186 to 1,242 | 0.24 to 0.25 | 4.1 to 7.0 | 1 to 14, 0.2 to 1.8 | 9 |
| 4 | 0, 0 | hand | 154 to 168 | 1,019 to 1,101 | 0.26 to 0.27 | 3.2 to 4.4 | 2 to 18, 0.2 to 1.7 | 8 (30 once) |
| 4 | 5 %, 30 ms | bot | 164 to 179 | 1,604 to 1,788 | 0.25 to 0.26 | 6.1 to 7.5 | 6 to 15, 0.7 to 2.5 | 12 (152 once) |
| 4 | 5 %, 30 ms | hand | 164 to 182 | 1,549 to 1,744 | 0.24 to 0.26 | 8.7 to 11.8 | 4 to 25, 0.8 to 8.7 | 12 (62 once) |

Two things. **The hand barely moves the network numbers** (corrections a second 1.1 to
1.4 with two clients and 1.7 to 2.0 with four, whoever flies): in this harness the
corrections are chiefly the discrete events (a weapon, a contact, a sideshift) and the
remote people's last-input guess, and a smooth hand has fewer big pops (largest pop 3.2
to 4.4 against 4.1 to 7.0 with four clean clients) where it replays fewer ticks (1,019 to
1,101 against 1,186 to 1,242). **Cost per correction** is the replayed ticks over the
corrections: 7 to 11 ticks, a 100 ms round trip plus the 2 to 5 ticks the server holds.
**Outliers**: one client saw a shot (the counter does not say which weapon) first appear **137 ticks** (2.3 s) into its flight
with the hand at 5 percent loss and 30 ms jitter (2 clients), **152** (4 clients, the
driver) and **62** (4 clients, the hand): resend waits, the 63 of ADR-0071's table seen
again and larger. They are the worst late presentations measured: 3 of the 12 client runs
at 5 percent loss and 30 ms jitter, none at 0 percent loss (30 once, clean), so ADR-0071's
"none older than 15 ticks" has a tail that loss and jitter reach; nothing here caps it.

**Own craft.** "Own craft moved" counts corrections that moved the player's own craft by
more than 0.05 units: 1 to 25 of 100 to 180 corrections per client, so about one in ten,
by up to 2.7 units in clean runs and **8.7 units** at 5 percent loss and 30 ms jitter
with four clients (`own pop max`, twice; a second run of the same cells, wall clock and so
not identical, gave 5.5 clean and 4.4 lossy at most, with 1 to 27 own moves per client). Those are the ones the smoothing above is for:
3 units is a 12-degree swing without it, and 8.7 is under the 20-unit snap, so it glides.

**What this does not answer.** Whether a real hand on a real pad needs more or fewer
corrections than a smoothed driver. The two ends of the range are measured (keyboard
ghosts 2.6 to 4.8 strict corrections a second, the hand 26 to 52, the driver 22 to 53),
and the *threshold* policy that the ADR recommends costs 0.4 to 1.8 a second for
keyboard, 0.9 to 2.5 for the hand and 0.6 to 1.8 for the driver, so the choice of
policy, not the stream, decides the cost, and any plausible stream lands inside 2.5 a
second at a 1-unit threshold. A recorded pad stream would pin it.

## Replay cost on this machine (2026-10-19)

The same sweep as "Reconciliation cost" above, one circuit (09_Track), profile `dev`,
AMD Ryzen 9 7900, 1-minute load average 9.0 (21.97 and 20.47 over five and fifteen
minutes), so read as inflated: a tick 63 us (`Race::tick`) and 59 us (`tick_sim_only`),
a 6-tick rollback 0.446 ms, 12 ticks 0.77 ms, 24 ticks 1.48 ms, a snapshot 62 us. The
2026-10-14 figures (65 / 60 us, 0.42 / 0.77 / 1.50 ms) hold. **Per correction**, the
network runs above replay 7 to 11 ticks a correction, so about **0.4 to 0.65 ms a
correction and 0.05 to 0.10 ms of CPU a second** averaged (the 4-client run's 1,019 to
1,788 replayed ticks over 90 s is 11 to 20 ticks a second, 0.7 to 1.2 ms a second at 59
us a tick, 0.07 to 0.12 percent of one core). **Condition 1 (the Galaxy S24) stays open:
the maintainer deferred it on 2026-10-19; no APK or phone steps were prepared.** The
pass condition (a 100 ms round trip at most 25 percent of a frame) holds on a phone up
to 10 times slower than this desktop, unmeasured, as before.

## Transport: WebTransport (2026-10-14)

**Decided by the maintainer (2026-10-14): WebTransport, plain UDP only if it fails.**
It did not fail. [ADR-0070](../architecture/adr/0070-the-network-transport-is-webtransport.md)
records it; what follows the decision there (the crate, the certificate scheme, the
wire shape) is Proposed.

Why WebTransport at all: the web build runs in a browser tab, which cannot send raw
UDP; WebSocket is TCP and stalls every packet behind a lost one. WebTransport (QUIC)
has unreliable datagrams and reliable streams, the two channel kinds the protocol
needs. Browser support per search excerpts: Chrome and Edge from 97, Firefox from
114, Safari 26.4 (March 2026); not checked against caniuse or MDN. A browser tab
cannot open sockets itself, so a web client would drive the browser's own
`WebTransport` object through `wasm-bindgen` behind the same `Link` trait (not built).

**The spike (2026-10-14, half a day).** Both Rust crates were tried in a scratch
crate on this workspace's MSRV (1.97.1), localhost, a self-signed certificate:

| | `wtransport` 0.7.2 | `web-transport-quinn` 0.13.2 |
| --- | --- | --- |
| Session, one bidirectional stream both ways, datagrams both ways | works | works |
| Pinned by certificate hash | works | works; a wrong hash fails the handshake with `UnknownIssuer` in 2.4 ms |
| Largest datagram on localhost | 1,413 bytes | 1,413 bytes |
| 200 datagram echoes, median round trip (debug build, loaded machine) | 379 us | 189 to 269 us |
| Self-signed certificate | built in | minted with `rcgen` (a few lines) |
| Crypto | `ring` by default | `aws-lc-rs` by default (C, cmake); `ring` as a feature, used |
| Browser client in the same family | no (a third-party `xwt`) | `web-transport-wasm`, and `web-transport` over both |
| Last release (crates.io, 2026-10-14) | 2026-08-11 | 2026-10-07 |
| Crates in a spike's normal dependency tree | 135 | 143 (`ring`) |

`web-transport-quinn` was taken, for the browser path. An earlier run that seemed to
show both refusing a wrong hash with a timeout was a test bug (the server accepted
one session only); with an accept loop the refusal is the TLS error.

**The certificate.** `oag-server` mints an ECDSA P-256 certificate per run, valid 13
days (a browser's `serverCertificateHashes` accepts at most 14; from memory, verify),
for `localhost`, `127.0.0.1`, `::1` and any `--cert-name`, and prints its SHA-256. A
client accepts that certificate only (`--cert-hash`). Self-signed on a LAN is the
maintainer's decision (2026-10-13); a public server would present a CA-signed one
(not built).

**What crosses the wire** (`oag_net::wire`, version 1):

- Reliable stream: hello (protocol, name), welcome (seat, the race's options as
  names, the tick-0 state hash), ready (the client's tick-0 hash), start, resend,
  confirmed ticks, end, reject. Length-prefixed.
- Datagrams up: a seat's held buttons and four axes for its newest eight ticks
  (20 bytes a tick; **chosen**). The server derives press and release from the held
  stream itself (decision 2).
- Datagrams down: per tick, the input the server applied to every person and the
  state hash after the tick, the newest sixteen ticks (**chosen**; 29 bytes a tick
  for one person, 169 for eight: the hash, the retired-seat mask, 20 bytes a person), and the receiver's lead: how many ticks of its
  input the server holds.
- **No state.** A client re-runs the server's inputs on its own copy of the
  server's state and checks the hash every tick; a mismatch is a determinism bug or
  different content, logged as an error, never a misprediction. The join refuses a
  client whose race starts at a different tick-0 hash.

## Hosting model

**Chosen, not measured.** Direction from the maintainer discussion of 2026-10-14.
`oag-server` already runs one race per process; the online lobby (item 3's "separate small lobby") was built as a first spike on 2026-10-16 ([online-lobby.md](online-lobby.md), part 5); the warm loop is not.

1. **One race per `oag-server` process.** Each process runs one match: one canonical
   `RaceSim`, one 60 Hz tick loop, one set of clients. This is the usual shape for
   authoritative real-time games (Unreal and Source dedicated servers, Rocket League),
   and fleet managers such as Agones, AWS GameLift, Edgegap and Hathora allocate one
   process per match from a warm pool. Why:
   - A crash or desync ends one match, not every match on the host.
   - Matches share no state, so nothing has to be locked or cleaned up between them.
   - Capacity grows by adding processes.
   - A rolling update drains the old matches and starts new ones on the new build.
   - The OS schedules each match's tick as its own work, with no shared loop.
2. **The alternative, not chosen: many rooms in one process.** Colyseus, Nakama and MMO
   shards work this way. It loads shared assets once, but the isolation between rooms
   has to be built by hand. Our tick is cheap (6 rollback ticks cost 0.42 ms on the
   desktop, see above), so the cost that matters is the disc image and track load.
   A warm pooled process pays that once, which removes most of the reason to share.
3. **Planned shape, none of it built.**
   - A warm process loads the image and the track at start, then waits for a match.
   - A reset-and-wait loop returns a finished race to waiting instead of exiting. This
     gives pooling without a fleet manager.
   - A separate small lobby or matchmaker starts or reuses servers and gives each client
     the address and the certificate hash. **Built as a spike (2026-10-16):** `oag-server lobby`
     (rooms, one game slot with a queue, a forked `oag-server race` child, the clients
     reconnecting by seat token), `oag-game --online`; the flow Omega used, the wire messages,
     the ports (UDP 60000 lobby, UDP 60001 race), how to run it and the deploy steps are in
     [online-lobby.md](online-lobby.md).
   - LAN stays a listen server inside the client (decision 3 above).

## Running a local server

From a checkout with a Pulse image in `data/images/` (any title works; the server
resolves the image as the game does when `--image` is left out):

```sh
# 1. The server: one person, the AI on the rest of the grid, Pulse's circuit 01.
cargo run --release -p oag-net --bin oag-server -- --image data/images/pulse-psp-eu.chd \
  --track 'Data\Environments\01_Track\track.vex' --people 1 --hash-out /tmp/oag.hash
# prints: listening on 127.0.0.1:4433, then: cert-hash <64 hex digits>

# 2. The client, in a second terminal: the game, straight into the server's race,
#    from the same image (a different pressing is refused at the join).
just play --connect 127.0.0.1:4433 --cert-hash @/tmp/oag.hash
```

Both are release builds, as `just play` is: a debug build (`cargo run` without
`--release`) turns on wgpu's Vulkan validation, which prints loader and
`OutputNotConsumed` messages in any race, networked or not, and runs far slower.

`--cert-hash` takes the 64 hex digits the server printed, or `@FILE` for the file
`--hash-out` wrote. Server flags: `--people N` (each client takes the next seat, grid
slot `0..N`), `--team NAME` (repeat it per seat in order, `--team a --team b`; a seat past the list flies the last one), `--no-opponents`, `--class`, `--mode`, `--difficulty`, `--scheme`
(one control scheme per race; every client flies the server's), `--seed`,
`--listen ADDR`, `--cert-name NAME` (another name the certificate is valid for),
`--max-ticks N`, `--lead-ticks N`. Client flags: `--net-bot` (the player's own driver
flies this client's craft, sent as input like a pad's: a whole race with nobody at
the keys), `--net-shim RTT_MS[:LOSS_PERCENT]` (a round trip and a datagram loss rate
added on the client's end, **chosen** test conditions), and `--screenshot out.png
--ticks N` for a headless client paced on the wall clock. Both print one line a
second: the server per seat (inputs in time, repeated, late, lead), the client its
corrections, the ticks they replayed, the largest pop, the hash agreeing and its lead
and prediction window.

The client's per-second line also carries `quic rtt`, `confirm`, `jitter`, wire
`rx`/`tx` and `loss`, and the same figures are the **network block of the `dev`
performance overlay** in the window (`[graphics] perf_overlay = "dev"`), or in a
headless `--screenshot` client with `--overlay` (details in
[the overlay's section of the menus page](../architecture/menus.md#the-network-block-dev-tier-network-races-only)):

- `quic rtt` is QUIC's smoothed round trip and `confirm` is the time from sending a
  tick's input to that tick coming back confirmed (the round trip plus the server's
  input buffer, so it reads about 50 ms on localhost). **Under `--net-shim` only
  `confirm` moves**: the shim sits above the socket, so `quic rtt` and `loss` stay
  at loopback and the overlay shows the shim's configured figures and drop count
  on their own amber rows. Measured on localhost with `--net-shim 100:2`: `confirm`
  about 190 ms, the prediction window 10 to 11 ticks (about 180 ms), `quic rtt`
  0.1 to 0.3 ms.
- `jitter` is the mean change between consecutive `confirm` samples; `lead` is the
  server's input buffer depth for this seat.

**Held fixed in a network race** (`oag_net::race`, all **chosen**): this project's AI
driver flies the opponents (the original driver with several people is unread),
pilot assist is off, weapons are the room's (on by default; predicted simulation, [ADR-0071](../architecture/adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md)), and every person's slot
is `Controller::Remote` on every peer, the client's own included, because the
controllers are hashed; which craft is this machine's is view-side
(`Race::set_seat`). The menus' ONLINE row opens the online lobby flow ([online lobby](online-lobby.md#8-the-online-menus-2026-10-16); LAN is [LAN play](#lan-play-2026-10-14) below). Start and focus loss
do not pause a network race, and it writes no record and no ghost (its inputs are
predictions corrected by a server, under the server's settings).

### Measured on localhost (2026-10-14)

Pulse EU, circuit 01, VENOM, seed 1, two people (bot clients, each headless at
960x540) and six AI, weapons off, 1,500 ticks; debug build on the shared desktop.
Each line is one client over the whole run; "hash ok" counts confirmed ticks whose
state hash matched the server's.

| Round trip, loss (`--net-shim`) | Client | Hash ok / bad | Corrections | Ticks replayed | Largest pop | Largest error at a confirmed tick | Prediction window |
| --- | --- | --- | --- | --- | --- | --- | --- |
| none | 0 | 1,459 / 0 | 18 (0.7 a second) | 36 | 1.27 | 1.07 | 2 ticks |
| none | 1 | 1,463 / 0 | 16 (0.6 a second) | 32 | 1.62 | 1.13 | 2 ticks |
| 100 ms | 0 | 1,461 / 0 | 25 (1.0 a second) | 226 | 3.91 | 1.28 | 10 ticks |
| 100 ms | 1 | 1,463 / 0 | 25 (1.0 a second) | 256 | 5.98 | 1.33 | 11 ticks |
| 100 ms, 5% | 0 | 1,462 / 0 | 26 (1.0 a second) | 287 | 4.29 | 1.31 | 11 ticks |
| 100 ms, 5% | 1 | 1,465 / 0 | 27 (1.1 a second) | 297 | 7.12 | 1.58 | 11 ticks |

**A whole race** (3 laps, both people and the AI over the line, the server ending
on "every person has finished" at tick 6,617 and 6,607):

| Round trip | Client | Hash ok / bad | Corrections | Ticks replayed | Largest pop | Final board |
| --- | --- | --- | --- | --- | --- | --- |
| none | 0 | 6,600 / 0 | 100 (0.9 a second) | 300 | 1.49 | the server's, line for line |
| none | 1 | 6,613 / 0 | 105 (1.0 a second) | 282 | 1.85 | the server's, line for line |
| 100 ms | 0 | 6,553 / 0 | 160 (1.5 a second) | 1,462 | 4.37 | the server's, line for line |
| 100 ms | 1 | 6,555 / 0 | 121 (1.1 a second) | 1,146 | 7.23 | the server's, line for line |

**The board is the server's.** A client's prediction finishes a few ticks before its
confirmed copy of the server's state does, and the board used to be taken there: on
the first whole race the two clients' boards differed from each other by one tick
in a person's time. The board is now taken again from the confirmed state when that
finishes (`Race::recapture_results`), and both clients print the server's board
exactly (compared line by line against `oag-server`'s own `board` lines).

At 100 ms the first two seconds are not clean: the clients start 50 ms after the
server's start and their first inputs arrive late (the server repeats the last
input, 36 to 44 late inputs a second for the first two seconds) until the lead
controller has run them ahead; that is inside the 272-tick countdown, where only
the held throttle matters. From then on every input arrived in time.

The server's side of the shorter runs: each seat's input arrived in time on every tick
(none repeated, none late) with a lead of 2 to 5 ticks; at 5 percent loss the
eight-tick redundancy covered every gap and no client needed a resend. One person
alone needs no corrections at all (900 ticks, 0 corrections): the AI is the same
on both ends and the client knows its own input. A windowed client on Xvfb with
software Vulkan (20 to 25 frames a second) kept pace: 2,540 confirmed ticks, hash ok on
every one. A windowed client with a copy of the maintainer's own settings (render
1280x720, motion blur, the performance overlay) ran at 10 frames a second there,
fell behind while its pipelines compiled, took the server's state (the client's
"server ahead" path) and caught up, and then ran slightly slower than real time: the
server repeated 10 to 16 of its inputs a second. Software Vulkan, not the protocol;
a real GPU is the test. The corrections are the other person's craft guessed on its last input
for a window of 2 to 11 ticks, crossing the 1-unit threshold about once a second;
they agree with section "Remote-input predictability" (ai1, threshold 1: 0.6 to 0.8
a second).

Reproduce: the two commands above, with `--people 2`, two clients with `--net-bot
--screenshot cN.png --ticks 1500 [--net-shim 100[:5]]`, and the per-second lines
summed. The disc-backed test `net_ground_truth` (`crates/net/tests/`) runs a server
and two bot clients in one process at a 100 ms round trip and asserts every hash
agrees and that corrections happen.

**The loopback harness runs on a virtual clock (2026-10-19).** The disc-backed
`weapons_ground_truth` and `net_ground_truth` races used to delay packets with a `Shim` on
the wall clock while the harness slept 16.7 ms a round, so a 100 ms round trip was a different
number of ticks on a loaded machine and the Eliminator race (`an_eliminator_race_is_flown_over_the_wire_by_two`)
ended differently: it failed 4 of 6 full `just test-data` runs. Measured with the sleep alone
changed (`finished` is read after the run): sleep 0 gave `fires: 0` and failed the weapons
assertion in 6 s, 16.7 ms gave `fires: 3, hits: 64`, 40 ms gave `fires: 1, hits: 106`, all on
one tree with `hash_mismatches: 0` - the ends agreed in every case, because the difference was
which inputs reached the server on which tick. `Shim::with_clock` reads its delays off a
shared `oag_net::link::Clock` the harness advances one tick per round (`Pace::Virtual`), no
sleeping anywhere: the same race (`fires: 4, hits: 46`, peak 23 in the air) comes out on
a quiet machine and under 48 busy loops, in 5.5 s against 98 s. The server's
`SeatCounts` (`repeated`, `late`) are now fixed numbers of the harness too. The
`confirm_sum_ms` stat still reads the wall clock, so in these runs it is the harness's
own speed and not a round trip.

**Not covered here** (weapons, loss, jitter, four clients and the own craft's correction
are measured in ADR-0071's follow-up and "An analog stick, 2026-10-19" below): the
Galaxy S24 (deferred by the maintainer), a recorded human analog hand, a client joining
late, a real network.

## LAN play (2026-10-14)

**Chosen, not measured**, except where a number is quoted. The maintainer approved the
lane on 2026-10-14; the decisions it builds on are 3 (a LAN host is a listen server inside a
client, trusted) and the transport of ADR-0070. Code: `oag-net` (`discovery`, `lobby`, `serve`,
`lan`), `oag-game` (`session::lan`, `net_play::Pending`), the LAN page of `assets/ui/menu.toml`.

### Two machines, step by step

Both machines run the same title and pressing (a different one is refused by name; a different
pressing of the same title is refused at the tick-0 hash), on one network where UDP broadcast and
UDP/QUIC on ports 4434 and 4435 reach each other (a host firewall must allow them, **this lane
changed no firewall**).

1. **Host.** MULTIPLAYER, LAN, HOST GAME. The RACE page opens (the MODE row shows that a
   network race is a Single Race); pick class, AI difficulty and the rest, START; pick the circuit
   in Track Select and the ship in Ship Select. The LAN LOBBY opens: PLAYER 1 is you.
2. **Joiner.** MULTIPLAYER, LAN, JOIN GAME. The list shows each host found, its name and circuit and
   how many are in. Choose one and wait; the host's lobby now lists you as the next player.
   A host on another title shows that title and refuses the choice ("THIS PLAYS ... NOT THIS DISC").
3. **Host.** START RACE (the pad's or key's START works anywhere on the lobby). The race loads on
   every machine, starts when all have loaded, and ends on the end screens; RETURN TO MENU or
   Escape leaves the network race (a joiner whose host leaves is returned to the menus after a second).
4. **If the list stays empty** (a network that drops broadcast, another subnet): TYPE AN ADDRESS
   on the joiner and enter the host's IPv4 address or name (the pad's or the desk keyboard's, digits,
   dot, hyphen, letters); the joiner then asks that address directly and the host appears in the list.

Pointer and touch work on every one of these screens (rows click like the join screen's). In step 2 the joiner first picks a ship (Ship Select), then the list.

### How it works

- **One host loop.** `oag_net::serve::run` is the race phase (start, 60 Hz on the wall clock, late
  connections refused by name, end, linger). `oag-server` and the hosting game both call it; they differ
  in how the seats fill and where the log goes.
- **The host is a client of its own server.** `oag_net::lan::Hosted::spawn` binds the game port (4435,
  every interface), answers discovery, and runs the lobby and then the host on a thread. The game's own
  player is the host's first client over an in-process `Loopback` link that is already in the lobby when the
  thread starts, so it is seat 0 by construction and goes through the same `Client` a joiner uses. (A
  WebTransport connection to itself would be the same code with TLS in the way, and a fast joiner
  could take seat 0.)
- **The lobby holds the welcomes.** The welcome carries the race and the head count, and START fixes the
  count. `oag_net::lobby::Lobby` collects hellos (names) and waits; at START the host thread fixes
  `people` at the number waiting (the host alone is allowed: a race against the AI over the network code),
  loads the race (`oag_net::race::load`, the server's path), seats the links in arrival order with
  `Host::seat` and waits for every seat's tick-0 hash. A seat that is refused or leaves meanwhile ends the
  start (`Host::departed`); nobody waits on a seat that will not come. Joiners wait on the reliable
  stream, with `Pending::poll` a frame at a time in the game (no blocking wait on the window thread).
- **Discovery** (`oag_net::discovery`): a joiner asks (UDP datagram `OAGQ`, to the limited broadcast
  address, to loopback, and to any typed address, every second), the host answers the asker with
  protocol, game port, certificate hash, host name, title, circuit label and head count (`OAGA`).
  Only the host binds the well-known port (4434), so a host and a joiner on one machine do not
  fight over it; the host's address is the answer's source address. A host heard on two legs is one
  host (same hash and port). LAN hosts are trusted (decision 3), so the hash travels in the clear; a
  WAN server's hash would not.
- **Keep-alive.** A QUIC connection closes after its idle time (30 s by default), and a lobby can
  wait minutes. `WtLink` sends an empty datagram every 5 s when it has nothing else to send; the
  other end drops it. (First run without it: the joiner timed out at about 30 s.)
- **A stopped host frees its port.** The accept loop ends and closes the endpoint when the `Listener` is
  dropped; a second HOST GAME in the same game binds the same port.

**One local person per machine** in this cut: no split screen on a LAN client. **Every person flies the ship they
picked**, wire v1 changed in place (nothing is released): the hello carries the team its person wants (`Hello.team`),
the welcome's `RaceParams.teams` lists every seat's team in seat order (an empty entry is `RaceParams.team`, the
default), and `oag_net::race::options` maps seat 0 to `Options::team` and seats 1 and up to `Options::seat_picks`,
on both ends, so the tick-0 hash still agrees. A joiner picks in Ship Select **before** the host list (JOIN GAME opens
it first); the host picks in its own Ship Select. `oag-server --team a --team b` fills the seats in order (a seat past the
list flies the last one); `--team a` alone is the old behaviour. Held fixed as in any network race:
weapons on (ADR-0071), our AI flies the opponents, pilot assist off, no ghost, no record. The host's race is a Single Race
whatever the pages say. Race again is not offered on a network race (the end screen's RACE AGAIN returns
to the menus; a rematch is a new lobby).

### The join list says whether you can join (maintainer's requirement)

Every host the browser finds stays in the list. The advert carries the title, the platform of its image, the build
(`BUILD_VERSION`), the wire protocol, a content fingerprint (the player mods loaded, FNV-1a of their ids and releases;
mods may change the sim), the circuit, the mode, and the people in out of the seats. `oag_net::discovery::compat` decides,
on the joiner, before connecting, in this order: **OtherTitle** (another title, another platform's pressing, or this
client lacks that image), **OtherVersion** (another protocol or build), **OtherContent** (other mods), **Full**, else
**Joinable**. A row reads `NAME   TITLE (people/max)`; a row that cannot be joined is dimmed, cannot be selected by a
confirm or a click, and says why as the help line while the cursor is on it (`oag_game::lan_list`, strings
`OAG_LAN_ROW_*`); a joinable row says its circuit there. A different pressing of one platform's title (EU against USA)
is not told apart by the advert and is refused at the tick-0 hash.

### Proof (2026-10-14, one machine, two game instances, Xvfb `:93` and `:94`, lavapipe, Pulse EU, `--net-bot` on both)

Both walked the menus with scripted key presses. Host: MULTIPLAYER, LAN, HOST GAME, RACE page, Track Select, Ship
Select, lobby. Joiner: MULTIPLAYER, LAN, JOIN GAME, found the host by discovery (loopback leg and broadcast
leg, one row), chose it, waited **over 50 seconds** (keep-alive), the host pressed START. Whole three-lap race
to the end screens: host log `tick 8880: every person has finished`; **hash ok 8,880 bad 0 on both
machines**; the eight `board` lines identical on the server and the joiner; the joiner's prediction needed 133
corrections and 0 stalls over the race (bots, M1 territory). Also proved live: a typed address (`127.0.0.1`
through the desk keyboard) reaches the asking path; a host that leaves a race returns the joiner to the menus;
a second HOST GAME in the same game after the race binds the port again; a Pure joiner is refused by name
when it picks a Pulse host. A ground-truth test (`net_ground_truth::a_lan_host_is_found_joined_started_and_raced_by_two_over_real_sockets`)
runs the same chain over real sockets without a window.

**Not covered:** two physical machines (one network, a real broadcast), a firewall in the way, IPv6, a
different pressing of the same title live (the tick-0 refusal is a loopback test), more than two people,
a human at the keys, Android and iOS, the web (the rows decline there).
||||||| c83eaf38e

## Running a server on another machine

The server is one binary and one disc image; the client is the game. Nothing is
installed on the server's host.

1. **Build a binary the host can run.** Built on its own, `oag-server` links the
   simulation half of `oag-raceplay` and no renderer, window, audio or UI crate at
   all (the `view` feature is off; rule 6 of `just check-deps` holds it, see
   [the lean race](../architecture/workspace-layout.md#the-lean-race-2026-10-14)).
   What does not travel is the build machine's libc, so build it for the host:

   ```sh
   scripts/build-server-portable.sh musl target/portable/oag-server   # fully static
   scripts/build-server-portable.sh 2.35 target/portable/oag-server   # glibc 2.35 and up
   ```

   **`musl`** links a fully static binary (`ldd`: "statically linked", about 9 MB
   stripped) that runs on any x86-64 Linux whatever its libc. It needs the
   `x86_64-unknown-linux-musl` Rust target (`rustup target add`) and `zig` on PATH,
   which compiles `ring`'s C for musl; nothing on the host. The **glibc** form links
   through `zig cc` against a named glibc (default 2.35, Ubuntu 22.04) and needs
   `libc` and `libm` on the host (`crates/net/tests/server_libs.rs` fails if the
   server asks for any other shared library). Measured 2026-10-14: a static musl
   server against the game as a `--net-bot` client, 1,039 confirmed ticks at a
   100 ms `--net-shim`, 0 bad hashes, and the same tick-0 fingerprint as the glibc
   build.
2. **Copy the binary and the one extraction** the race is on, to a directory of its own
   (`oag-server extract --image data/images/pulse-psp-eu.chd --out data/server` keeps only what
   the server's races read, 183 MiB of Pulse's 308; a whole image works too; see
   [the online lobby, part 6](online-lobby.md#6-the-server-extraction-and-the-build-string-2026-10-16)), and give the server that directory for its settings
   and logs (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`).
3. **Run it listening on every address**, on a UDP port the host lets through:

   ```sh
   ./oag-server --image pulse-psp-eu.chd --track 'Data\Environments\01_Track\track.vex' \
     --people 1 --listen '[::]:PORT' --hash-out hash.txt
   ```

   `[::]` accepts IPv4 too on Linux (dual stack). A host whose name has only an AAAA
   record needs it: the client resolved the name to IPv6 and the first attempt, bound
   to `0.0.0.0`, was never answered. The default `127.0.0.1` accepts nobody else.
4. **The certificate hash is the handoff.** The server mints a self-signed
   certificate per run and prints `cert-hash <64 hex digits>`; the client pins that
   hash and checks nothing else (not the name), so the digits must come from this
   run of the server, over a channel you trust (the shell session that started
   it). Read `hash.txt` over the same ssh connection and pass it on:

   ```sh
   oag-game pulse-psp-eu.chd --connect HOST:PORT --cert-hash <digits> [--net-bot]
   ```

   A server restart is a new certificate and a new hash.
5. **When it is over**, the server exits by itself once every person has finished;
   stop it by its process id otherwise, and delete the binary and the image.

**Blocked ports.** UDP is filtered on many hosts. The test host let through port
60000 and dropped 4433, 4434, 8443, 27015 and 44330 (checked with a one-line UDP
echo before any server ran; ICMP was dropped too, so `ping` shows nothing). Probe
a few high ports the same way; do not change the host's firewall for a test.

### Measured against a remote host (2026-10-14)

A server on a small Linux host over the internet (Ubuntu 22.04, glibc 2.35, one
vCPU, release build, no GPU), the client on the shared desktop (debug game
build), Pulse EU, circuit 01, VENOM, seed 1, one person with the AI on the other
seven, weapons off, three laps. **Round trip** by a 100-packet UDP echo, 50 ms
apart: minimum 11.4 ms, median 11.9, 95th percentile 14.7, maximum 19.6. No
`--net-shim`.

| Client | Ticks | Hash ok / bad | Corrections | Ticks replayed | Largest pop | Stalls | Resends | Final board |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Headless bot, `--screenshot` | 6,308 | 6,248 / 0 | 1 | 3 | 1.26 | 118, all in the first 200 ticks | 0 | the server's, line for line |
| Windowed bot, Xvfb, software Vulkan | 6,326 | 6,326 / 0 | 0 | 0 | none | 54 | 0 | the server's (race complete) |

("Hash ok" sums the per-second lines, which start counting once the first tick is
confirmed, so it trails the ticks by the first second.) In both runs the server
ran 60 ticks a second with every seat's input fresh and a lead of about 5 ticks,
except in the first 180 ticks inside the countdown, where the client was still
starting and the server repeated its last input (31 and 60 of 60 in the first
two seconds, a negative lead, then caught up), and in the windowed run for about
300 ticks around tick 4,200, where 2 to 13 inputs a second were repeated and the
lead fell to -13 once. That window is when two screenshots were being taken of
the software-rendered window on the same machine; the cause was not isolated.
One person alone needs no corrections, as on localhost (the one correction in the
headless run was a single 3-tick replay, pop 1.26, at tick 441).

**Against the local shim.** At 12 ms the numbers sit at the "none" row of the
localhost table (0.9 to 1.0 corrections a second there with two people; none to
one here with one), not at the 100 ms rows: the real round trip is a ninth of the
shim's, and the prediction window stayed at 3 to 6 ticks (it was 10 to 11 at 100
ms). A lossy or jittery link, a second person and a real GPU are still unmeasured.

## Cheating: what is and is not worth defending

Ranked by how bad it is for a Wipeout lobby, given decisions 2 and 3.

| Cheat | Verdict |
| --- | --- |
| Oracle bot or assist: a program reads the screen or the state and emits legal input, possibly through a faked input device | **Accepted.** Undetectable client-side, and every peer holds the full sim state anyway. Racing decisions are narrowly scoped, so machines do well at it (Sony AI's Gran Turismo Sophy, 2022, from memory, not checked here). No client anti-cheat work |
| Late or look-ahead inputs (waiting to see others' inputs first) | The server buffers each client's input and consumes one per tick, as Rocket League does (which also stops speed cheats); an empty buffer repeats the previous input; the rollback window is capped |
| A client running a modified sim | Harmless online: the server's result is the truth and the client only mispredicts |
| Forged records | Replays are input streams ([ADR-0055](../architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)), so a server can re-simulate a run. A modified-sim run goes to a separate board, tagged with its fingerprint |
| A malicious server or LAN host | Out of scope, possible because the project is open source. The fingerprint shown on join is consent, not defence |

The seeded `Rng` is public to every peer in an input-based design, so a
modified client can look ahead to a pickup's roll. If that ever matters, the
server supplies gameplay-random outcomes as confirmed events.

**Interest management (culling hidden players) does not apply.** It saves
bandwidth and blocks wallhacks in 64-player shooters that send state. Here every
client predicts all 8 craft and so needs all 8 inputs, and an input is a handful
of bytes per tick.

## Research

Except for the Rocket League slides below, read from search excerpts only; none
of the papers were read in full, so treat the summaries as pointers.

- [Cone, It IS Rocket Science! (GDC 2018 slides)](https://media.gdcvault.com/gdc2018/presentations/Cone_Jared_It_Is_Rocket.pdf),
  also on [GDC Vault](https://www.gdcvault.com/play/1024972/It-IS-Rocket-Science-The).
  The only primary source found on a vehicle game with collisions, read as
  extracted slide text (the animations and the spoken talk are not covered).
  Fixed 120 Hz tick, 100% server authoritative, "input delay is not an option".
  The server buffers client inputs and throttles the client faster or slower;
  an empty buffer runs physics on the previous input. The client predicts
  everything, and on a large difference from the server's state it reverts all
  physics actors to that frame and re-runs the frames. Interpolating the ball
  left the car and the ball "in two different timelines"; FPS-style lag
  compensation was weighed and not shipped. Predict-everything "works well with
  ball (predictable)" and "not as well with cars (unpredictable)", and 200 ms of
  ping at 120 Hz is 24 correction frames. The slide text does not say what input
  is assumed for remote cars.
- [Local-lag and Timewarp](https://publications.cs.hhu.de/Mauve2004a.html)
  (Mauve, Vogel, Hilt, Effelsberg, IEEE Trans. Multimedia, 2004). Local lag is
  deliberate input delay and timewarp is rollback and re-simulation. It shows
  a trade-off between responsiveness and short-term inconsistency and compares
  timewarp with dead reckoning, concluding the choice is application-dependent.
  The formal basis of the recommendation above.
- [Fiedler, Introduction to Networked Physics](https://www.gafferongames.com/post/introduction_to_networked_physics):
  deterministic lockstep, snapshot interpolation, state synchronization.
  Lockstep needs exact determinism and every frame waits for all players, so he
  recommends 2 to 4 players at most.
- [A Survey and Taxonomy of Latency Compensation Techniques for Network Computer Games](https://dl.acm.org/doi/10.1145/3519023)
  (ACM, 2022): the map of the field, with a table of racing-game studies.
- [Pantel and Wolf, racing-car collisions](https://www.cs.ubc.ca/~krasic/cpsc538a/papers/p23-pantel.pdf):
  two racing games tested handled delay-induced collisions poorly, and without
  compensation opponents' positions go stale.
- [Latency and packet loss in networked racing games](https://dl.acm.org/doi/10.1145/1103599.1103622)
  (ACM, 2005) and [Predictive Dead Reckoning for P2P games](https://ece.uwaterloo.ca/~sl2smith/papers/2023TOG-Predictive_Dead_Reckoning.pdf)
  (Waterloo, 2023): later reading.

Secondhand and unverified: Rocket League sends server updates at about 60 Hz
against its 120 Hz physics (a Steam forum post relaying a Psyonix developer,
disputed in the same thread). Trackmania runs online matchmaking without car
collisions, per a review; its sync mechanism was not confirmed. For iRacing,
Forza, Mario Kart 8, Gran Turismo 7, Codemasters F1 and Assetto Corsa
Competizione no technical description was found, only player reports and press
attribution; the survey is in the handover thread.

## Approach

Deliberately last. Multiplayer built on an unverified simulation inherits every
one of its bugs and adds desync on top. The state hash of `oag-core` and the
input-stream replays of M7 are what a desync check and a record check are built
from.
