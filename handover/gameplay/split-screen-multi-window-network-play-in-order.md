---
categories: [rendering, frontend]
---

# Split screen, multi-window and network play: one shared prerequisite, then three efforts in dependency order

2026-09-16. Not an RE finding - a planning thread, opened on request, laying
out how split screen, multiple OS windows and network multiplayer relate to
each other and to the engine as it stands today, in the order that minimizes
wasted work. No code changed. No ADR exists yet for any of this; whichever
design is actually chosen for the shared prerequisite below is ADR-shaped and
should become one before implementation starts, the same way [ADR-0003](../../docs/architecture/adr/0003-no-ecs.md)
and [ADR-0007](../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)
already pin the decisions this plan leans on.

**Decided, on request, and stated up front so the rest of this thread reads
consistently: the network protocol is ours, not a port.** Unlike every other
subsystem this project reimplements, network play is not observed from the
original and is not trying to be compatible with it - see the "no source to
observe" point in section 3 for why there is nothing to port from even if
compatibility were wanted. It is designed state-of-the-art from scratch:
server-side authority (the server, not any client, is the only source of
truth, specifically to close off client-side cheating) with client-side
prediction for responsiveness, reconciled by replaying inputs against
corrected server state when a prediction turns out wrong. Full shape in
section 3. Split screen, multi-window and network play all target **up to 8
players** - the existing `MAX_SHIPS = 8` grid cap, so a full-player race
needs zero AI-filled slots and a networked race can never exceed what a
local race already handles.

**2026-09-16, revised same day**: an external review pass caught that the
first version of this thread named the wrong tick function throughout.
`World::tick` does not exist. The real 60 Hz step is `Race::tick`
(`crates/game/src/race/tick.rs`), living in **`oag-game`**, the composition
root, not in `oag-gameplay`. It advances `RaceSim`
(`crates/game/src/race/sim.rs`), which bundles `World` together with
`CollisionWorld`, `Spline`, `ai_order: Vec<u32>`, `ai_tuning`,
`weapon_pad_refresh_left` and a per-tick `cues: Vec<CueEvent>` audio-output
accumulator (`self.sim.cues`, [ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md)'s
"cues are a per-tick output, never `World` state") - `World` alone is not the
simulation, and `RaceSim::state_hash` (`crates/game/src/race/hash.rs`) hashes
the extras precisely because of that. Every "`World`"/"`World::tick`"
reference below that actually meant the whole tick step has been corrected
to `RaceSim`/`Race::tick`; see the "Corrected: what actually has to move"
callout in section 3 for what changes as a result. `oag_gameplay::World`
itself is still exactly as described - a plain, fixed-array,
`memcpy`-shaped struct per ADR-0003 - it's just not the whole story on its
own.

None of the three is scoped as a roadmap item on its own. `docs/overview/roadmap.md`
carries a single unchecked `- [ ] Networking` line under **M8 - Beyond Pulse**,
the last milestone, gated on single-player completeness across every title.
Split screen and multi-window aren't named in the roadmap at all - the only
existing trace of split screen is two frontend threads noting the PS2 menu
XML already authors split-screen widgets (`Team SelectionSplit`, `Track
CreationSplit`) that this build currently strips to single-player
(`handover/frontend/pulses-race-box-is-read-both-pressings.md`). This thread
does not change that prioritization; it exists so that whenever any of the
three is picked up, it starts from an accurate picture instead of rediscovering
the same constraints.

## Why one prerequisite gates all three

The engine today is **hard single-player**, not "single-player by default":

- `World` (`crates/gameplay/src/world.rs:203`) already carries
  `ships: [Ship; MAX_SHIPS]` with `MAX_SHIPS = 8` - room for a full grid - but
  `race: RaceState` is a *single* field. Its own doc comment says why:
  "Single-ship for now: the modes that need it - time trial, speed lap, Zone -
  have one ship on the track, and per-opponent timing arrives with the grid
  rather than before it." Only `ships[0]` is ever the timed, human-controlled
  ship; `crates/game/src/main/race_stage.rs` reads that index directly for
  every HUD and lap-timing field. **Not everything per-ship is missing,
  though**: `ShipState::standing: oag_race::Standing` (lap, circuit place,
  finished) is already tracked "every craft, the player included" - it's
  specifically `World::race` (the clock, Zone counters, best lap) that's
  singular. Widening `race` to per-player reuses `standing`'s existing shape
  rather than inventing per-ship tracking from nothing.
- `InputSnapshot` (`crates/gameplay/src/input.rs:253`) is one struct, no
  player index, doc-commented as "the whole of what the simulation is allowed
  to know about input" - by design, a replay records exactly this and nothing
  else. `Race::tick` takes exactly one per call.
- `oag-input`'s `Pad` explicitly **merges every attached gamepad into one
  logical stream** rather than assigning devices to players:
  "Pads are merged rather than assigned to players: there is one ship, and a
  Deck with a pad plugged into its dock should steer from either"
  (`crates/input/src/pad.rs:366`). There is no `GamepadId` -> player-slot
  mapping anywhere, even as a stub.
- `App` (`crates/game/src/main/app.rs:95`) is documented as "one of these per
  run, not one per window" - the composition root assumes exactly one winit
  window for the process's whole life.

All three features - split screen, multi-window, and network play - need the
simulation to think in terms of **N players feeding N inputs into one
`RaceSim` per tick**, instead of one. That's a single, shared, foundational
change:

1. `RaceState` becomes per-player (array or `[RaceState; MAX_PLAYERS]`
   alongside the existing `[Ship; MAX_SHIPS]`). **`MAX_PLAYERS = MAX_SHIPS =
   8`**, decided on request rather than left open: split screen, multi-window
   and network play all target up to 8 players, so there's no case where the
   player cap needs to be smaller than the grid it fills. A slot not driven
   by a human is still AI-driven exactly as today, so this needs an explicit
   notion of "which grid slots are locally/remotely human" alongside the
   per-player `RaceState`, not instead of it.
2. `Race::tick` takes N `InputSnapshot`s (or a `[InputSnapshot; MAX_PLAYERS]`)
   instead of one, applied to N ship slots instead of `ships[0]` alone.
3. `oag-input` gains a device -> player-slot assignment (which `GamepadId`
   or keyboard drives which slot), replacing the current merge-everything
   `Pad`.

`RaceState` and `InputSnapshot` are `oag-gameplay` types, and the device
assignment is `oag-input`'s - no rule in `docs/architecture/workspace-layout.md`
is threatened by widening either. But `Race::tick` itself, the function that
actually consumes N inputs per tick, lives in `oag-game`, so this
prerequisite touches the composition root too, not only the two gameplay-side
crates - worth knowing going in, since it means "land the prerequisite" is
partly an `oag-game` change, not a self-contained `oag-gameplay`/`oag-input`
one. It's still the one piece of work on the critical path for *every* one
of the three features below, plus the already-placeholdered `oag-replay`
(M7, input recording/playback, per `workspace-layout.md`'s "crates that do
not exist yet" table). Worth landing as its own change before any of split
screen, multi-window or networking starts, rather than growing it ad hoc
inside the first feature that needs it.

`ADR-0003` (no ECS) is the reason the `World` portion of this is tractable
at all: `World` is a plain struct with fixed-size arrays, chosen specifically
so "snapshotting the whole world is one memcpy-shaped operation" for replays
and golden tests. Widening it to carry N `RaceState`s is a shape change
within that same design, not a departure from it - though the ADR's own
"revisit if" clause ("a later title's entity model does not fit fixed
arrays") is worth rereading once the per-player shape is nailed down, in
case N independently-timed players turns out to want something ADR-0003
didn't anticipate. **`RaceSim` as a whole is not memcpy-shaped, though** -
`ai_order: Vec<u32>` and `cues: Vec<CueEvent>` are heap-allocated - which
matters most for network reconciliation; see the corrected callout in
section 3.

## 1. Split screen - lowest complexity, do this first

Same process, same machine, one `RaceSim`, one fixed 60 Hz tick, up to 8
viewports in **one** window. This is the cheapest of the three because it changes
nothing about process/window lifecycle (`App` stays "one per run") and
nothing about transport - it's purely: multiple local inputs in, multiple
local views out, same tick.

What's missing beyond the shared prerequisite:

- **Viewport/multi-camera support in `oag-render`.** Today there is, by its
  own doc comment, "one track-ribbon builder and one camera convention in
  the repository" (`crates/render/src/lib.rs:4`) - no `Viewport` struct, no
  split-region compositing. This is new surface area, not a variant of
  something that exists. `oag-render` owning no window
  (`crates/render/src/lib.rs:12`, permanent by design) is actually favorable
  here: the surface/window count stays 1, and split screen is purely "render
  N times into N sub-rects of the one surface," which doesn't touch the
  window-ownership boundary at all.
- **Per-player HUD** in `oag-ui`, laid out per-viewport rather than
  full-screen.
- **Per-player pad assignment**, covered by the shared prerequisite above.
- **A layout that scales to 8**, not just the original's 2/4-way splits -
  viewport count and arrangement (2-up, 4-up, up to 8-up) becomes a variable
  the viewport/camera-region support above needs to handle, not a fixed
  quadrant layout.

Split screen is also the one feature with existing authored data to check
against: the PS2 build's menu XML already has `Team SelectionSplit`/`Track
CreationSplit` screens and a `MP_Screen.xml` slideshow that this project's
picker currently strips (`picker::strip_player_suffix`,
`handover/frontend/pulses-race-box-is-read-both-pressings.md:108`). Worth
reading that thread before building the front-end leg, since the original's
screen flow for enabling split screen is already partly RE'd.

## 2. Multi-window - same simulation, different rendering/app-lifecycle risk

Same process, same machine, one `RaceSim`, one tick - but up to 8 *OS windows*
instead of up to 8 viewports in one window (each player's view full-screen in
their own window, e.g. for a multi-monitor local setup). Simulation-side,
this needs nothing split screen doesn't already need - it reuses the shared
prerequisite and, if built second, split screen's per-player input/HUD work
directly.

The complexity here is entirely on the app/render plumbing, and it's a
different shape of work than split screen's, not a strict superset:

- `App`'s "one of these per run, not one per window" assumption
  (`crates/game/src/main/app.rs:95`) has to go. winit's `ApplicationHandler`
  needs to route resize/redraw/input events per-window, and `Session`
  (currently singular) needs to become one-per-window or gain an explicit
  window index.
- Each window needs its own wgpu surface; `oag-render`'s device/queue can be
  shared across them (it owns no window and never will,
  `crates/render/src/lib.rs:12`), but the surface-per-window bookkeeping
  and per-window resize handling is new.
- No new viewport/camera-region concept is needed here (each window draws
  one full-frame camera) - which is why, camera-wise, this is *simpler* than
  split screen even though it's listed second: the ordering is driven by
  `App`'s lifecycle assumption being the harder thing to unwind, not by
  render complexity.

Split screen and multi-window could in principle be built in parallel once
the shared prerequisite lands, since they touch different parts of
`oag-render`/`oag-game` (viewport compositing vs. multi-surface window
lifecycle). Doing split screen first is recommended anyway: it ships local
multiplayer sooner, with zero risk to `App`'s existing single-window
lifecycle, and multi-window can then reuse its per-player input/HUD work
wholesale.

## 3. Network multiplayer - highest complexity, ours by design, and the one with no source to observe

Different processes, possibly different machines, up to 8 players. **This is
original engineering, not a port**: the design is server-authoritative with
client-side prediction and replay-based reconciliation, chosen on request
rather than derived from the original, because there is no original to
derive it from (see below). This is qualitatively harder than the first two,
for reasons beyond "more code":

- **The protocol, stated explicitly since it's the one piece of this thread
  that's a design decision rather than an architecture survey:**
  - **The server is the sole authority.** It runs the canonical `RaceSim`,
    accepts only `InputSnapshot`s from clients (never client-reported
    state), and ticks the simulation itself. A client cannot report "I am at
    this position" or "I hit that ship" and have it believed - the server
    derives both from its own tick. **Validation needs to be more than bounds
    checking, though**: `Input` (`crates/gameplay/src/input.rs:157`) tracks
    `pressed`/`released` as *edges*, derived internally from a held-mask
    history (`pressed = held & !held_last`) - a client sending its own
    precomputed edge bits could claim a press that never transitioned
    through held state. The server has to re-derive edges from a held-state
    stream it advances itself, the same way `Input::advance` already does
    locally, never trust a client's `pressed`/`released` bits directly. Also
    worth being honest about scope: server authority bounds *state* cheats
    (position, damage, "I won") - it does nothing against input-level cheats
    like frame-perfect timing or a client that withholds inputs to see
    others' positions first. Different problem, not solved by this design.
  - **Each client predicts its own ship locally**, running the same
    deterministic `Race::tick` ahead of the server's acknowledged state
    using its own just-captured input, so the local player's controls feel
    immediate rather than waiting a round trip. This is the same tick
    function every other mode already uses - prediction is "run the sim
    client-side too," not a second implementation of it.
  - **Reconciliation is a snapshot-and-replay, not a smooth correction.**
    When an authoritative snapshot arrives from the server (tagged with the
    input sequence number it was computed from), the client restores its
    `RaceSim` to that snapshot and **replays its own buffered local inputs**
    since that point forward, exactly as recorded, to arrive back at a
    corrected present. If the server's result matches what the client had
    already predicted, this is invisible; if not, the client's view snaps to
    the corrected trajectory. **Corrected: what actually has to move isn't
    the memcpy-shaped `World` alone, it's the whole `RaceSim`** - which
    carries a `Vec` (`ai_order`) to clone on restore, cheap but not literally
    a memcpy, and a `cues: Vec<CueEvent>` audio-output accumulator
    ([ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md))
    that a reconciliation replay must not let reach the audio layer a second
    time - a tick that already played its cues once must not play them again
    just because it got re-run to reconcile a misprediction. `InputSnapshot`
    and the already-placeholdered `oag-replay` for M7 are still the right
    two primitives to build on; the correction is about what gets
    snapshotted, not whether snapshot-and-replay is the right shape.
  - **Remote players are not predicted the same way** - the working plan is
    interpolation/extrapolation between authoritative server updates, since a
    client has no early knowledge of another player's next input. **Flagged
    as a real disagreement worth resolving before the ADR, not settled
    here**: interpolation/extrapolation is the standard compromise in
    genres where players rarely alter each other's motion frame to frame. A
    racing pack does not have that property - ship-ship contact, and
    Leech/Quake/Disruptor hits, change a craft's motion on contact, every
    tick, and with the local ship predicted ahead and remote ships shown
    behind, a contact resolves against a remote ship that's stale by close
    to a full round trip, which is exactly when the interpolated guess is
    most likely to be wrong. Given the sim is already deterministic and
    fixed-step (ADR-0007), rollback with predicted remote inputs
    (repeat-last-known-input until a real one arrives, GGPO-shaped) is worth
    weighing against interpolation specifically for this game's tighter
    inter-ship coupling before the protocol ADR is written - server
    authority can still sit on top of either choice.
- **It needs a transport that doesn't exist as a concept anywhere in the
  codebase yet.** `docs/architecture/workspace-layout.md` lists `oag-net`
  only as a not-yet-created placeholder for M8, with the explicit warning
  that "a crate created before its shape is understood tends to get the
  wrong shape" - and the same doc's own precedent (`oag-weapons`, planned
  but never created because the work fit existing crates) means `oag-net`
  shouldn't be scaffolded until the transport and message shape (tick rate
  vs. send rate, reliable-vs-unreliable channels, snapshot delta encoding)
  is actually decided.
- **Determinism is what makes client-side prediction trustworthy**, and
  it's exactly what `ADR-0007` bought deliberately: the engine keeps a fixed
  60 Hz timestep and diverges from the original's variable-`dt` simulation
  specifically so that "replays, golden tests and cross-platform comparison
  all still work." The same property that makes a golden test reproducible
  is what makes a client's predicted tick and the server's authoritative
  tick agree often enough that reconciliation is the exception, not the
  norm - a nondeterministic sim would make every server correction visible
  as a snap, every tick.
- **`oag-core`'s existing state hash** (`crates/core/src/hash.rs`, used today
  by CI's cross-platform `determinism` job) is directly reusable to confirm
  a client and server agree on state after reconciliation, and to flag a
  genuine desync (a client running a different build, a platform-specific
  float divergence `ADR-0002`/`ADR-0007` don't fully rule out) rather than
  silently drifting.
- **There is no original implementation to reverse-engineer this from, and
  none is wanted.** `docs/networking/README.md` puts confidence at 90 that
  the PSP Pulse build ships no `pspnet` stack and no `libhttp` at all (three
  PRX modules against Pure's 26, which has the full network stack) - meaning
  Pulse likely never had online play to observe in the first place. That's a
  real break from this project's usual "observe -> hypothesize -> verify ->
  document -> implement" methodology, and it's intentional here rather than
  a gap: the protocol above is designed fresh, not ported, so there's
  nothing to verify it against except itself (tests, and eventually live
  play). `docs/networking/README.md`'s existing scope (ad-hoc PSP local
  mode, game sharing via `GSHARE/SHARE.BIN`, DLC) is a **different, narrower
  question** - recreating what the original's local/ad-hoc mode did, if
  anything - and is not what this section is planning; that scope stays
  wherever `docs/networking/README.md` already put it, unaffected by this
  thread's server-authoritative online design.

Network play only strictly needs the shared prerequisite (N inputs, up to 8
players, one `RaceSim`) - it doesn't require split screen or multi-window to
exist first, since a networked race could run one player per machine, one
window each, no local split at all. But it benefits from whichever local
multi-player work already forced the `World`/`InputSnapshot`/input-mapping
generalization to be real and tested against actual multi-player input,
rather than reasoning about it in the abstract for the first time under
network latency.

**Corrected: the server side is not already free.** The first version of
this thread claimed the headless server "already supports [running the sim
without a renderer] without change" via `oag-gameplay`'s render-free design.
That's wrong - `Race::tick`/`RaceSim` live in `oag-game`, and per
`workspace-layout.md`'s dependency rule 2, "no crate may depend on
`oag-game`," so a server can't simply be some other crate importing
`oag_game::race`. Two real options, not one free one:
1. A second `[[bin]]` target inside the `oag-game` crate itself (its
   `Cargo.toml` already has `[lib] oag_game` plus `[[bin]] oag-game` - "a
   thin `[[bin]]` over `[lib]` so boot logic is testable headlessly," per
   `CLAUDE.md`'s crate table). A headless-server binary in the same crate
   reuses `Race`/`RaceSim`/`tick` through the lib directly, with no new
   dependency edge and no rule violation - this is the cheap path.
2. Hoisting `RaceSim` and `Race::tick` out of `oag-game` into a
   gameplay-side crate, so anything (a server, `oag-trace`, a future
   `oag-net`) can depend on it directly. Architecturally cleaner, but real
   work, and not obviously worth it just for the server - `oag-trace`
   already hit this exact wall and chose to duplicate the logic it needed
   rather than depend on `oag-game`
   (`crates/trace/src/replay.rs`'s `locate`, whose own doc comment states
   "`oag-trace` cannot depend on `oag-game`... so the resampling is
   duplicated crate to crate on purpose"), which is a working precedent for
   option 1's same-crate reuse being preferred over a wider refactor.

## Recommended order

1. ~~**Shared prerequisite**: per-player `RaceState`, N-input `Race::tick`,
   device-to-player mapping in `oag-input`. Touches `oag-game` as well as
   the two gameplay-side crates (see the corrected "Why one prerequisite
   gates all three" section). Blocks all three below and `oag-replay` (M7).~~
   **Done, 2026-09-16.** All three landed, plus the headless-server binary
   section 3 asked for, in four commits:
   - `World::race` is `[RaceState; MAX_PLAYERS]` with `MAX_PLAYERS = MAX_SHIPS
     = 8`, and `World::controllers: [Controller; MAX_PLAYERS]` beside it -
     `Ai`, `Local` or `Remote`, both hashed. `World::SINGLE_PLAYER` is the one
     `Local` in slot 0 the engine already had hard-coded, and
     `human_slots`/`primary_slot`/`primary_race`/`mode`/`laps_target` are what
     replaced the literal `0`s. `crates/gameplay/tests/determinism.rs`'s
     constants moved, isolated the documented way - see that file's own
     2026-09-16 history entry.
   - `Race::tick` takes `oag_gameplay::PlayerInputs`, a fixed
     `[InputSnapshot; MAX_PLAYERS]`. The player slot is threaded into
     `flown_for_the_player`, `autopilot_controls`, `grant_free_turbo`,
     `spend_pickup`, `tick_autopilot` and `lost_off_the_track`;
     `advance_cannons` asks `World::controllers` instead of `slot == 0`.
     **The player step still runs once** and returns one `Evaluated` - N
     cameras and N HUDs are step 2's work, and `Race::tick`'s doc comment says
     so.
   - `oag_input::pad::Assignment` maps `GamepadId` and the keyboard to slots;
     `Pad::poll_players` and `Controls::player_snapshots` read per slot, and
     `Controls`' button state is one `Input` per slot because an edge is per
     pilot. Default: every device on slot 0, which is the old merged stream.
   - `oag-headless-sim`, a second `[[bin]]` in `oag-game` over
     `Setup::headless`, ticks with no renderer and prints its state hash -
     section 3's option 1, taken. `just headless-sim` runs it; its claims are
     also in `crates/game/src/race/tests/headless.rs` so the gate holds them.

   Behaviour is unchanged throughout: a single-player race still reads exactly
   `ships[0]`. **No ADR was written** - this thread asks for one and the work
   was scoped to the refactor, so the design lives in doc comments on
   `World::race`, `World::controllers`, `Controller` and `PlayerInputs`.
   Writing the ADR from those is still open.
2. **Split screen**: viewport/camera-region support in `oag-render`,
   per-player HUD in `oag-ui`. Lowest risk - no window-lifecycle changes.
3. **Multi-window**: unwind `App`'s one-window-per-run assumption, per-window
   wgpu surfaces. Can run in parallel with (2) once (1) lands; ordered after
   split screen here only because it's more invasive to `oag-game`'s current
   structure.
4. **Network multiplayer**: server-authoritative with client-side prediction
   and replay-based reconciliation, up to 8 players, decided per section 3 -
   original design, not a port. Before a transport or `oag-net` exists,
   prove the shape cheaply with an **in-process loopback client/server**:
   two `RaceSim`s in one test binary, inputs passed directly (no socket),
   asserting the client's post-reconciliation `state_hash` matches the
   server's. That's a test of the snapshot/replay/cue-suppression logic in
   isolation from anything network-shaped, and it's buildable the moment the
   headless-server binary (a second `[[bin]]` in `oag-game`, see section 3)
   exists. Only after that passes does the transport and message-format
   decision (needed before `oag-net` is created) become worth making.

## Open

- ~~Whether `RaceState` should become a fixed `[RaceState; MAX_PLAYERS]` or
  gain an explicit human/AI-slot distinction on top of the existing
  `[Ship; MAX_SHIPS]` grid - not decided here, needs a design pass against
  how AI opponents and human players currently share the same `ships` array.~~
  **Done, 2026-09-16. Both, not either.** `race` is the fixed array, indexed
  by grid slot so `race[i]` and `ships[i]` are the same racer by construction,
  *and* `controllers` is the human/AI distinction beside it. Collapsing them
  would have made "has a `RaceState`" mean "is human", which destroys the one
  thing the array is for - ranking eight timed craft against each other.
  `RaceState::mode`/`laps_target` are replicated eight times with only slot
  0's authoritative, read through `World::mode`/`World::laps_target`; that
  redundancy is recorded on the field rather than hoisted out, because
  splitting `RaceState` is a change to `oag-race`'s own recovered type.
- **An ADR for the per-player shape is still owed.** This thread said the
  design was ADR-shaped and should become one before implementation; the
  implementation landed first, on a scoped refactor brief, with the reasoning
  in doc comments instead. Writing it up is a docs-only change now, not a
  design question - the decisions are made and tested.
- Transport choice for `oag-net` (framing, reliable-vs-unreliable channel
  split for inputs vs. snapshots, send rate relative to the fixed 60 Hz
  tick) - not decided, needed before the crate is created.
- Reconciliation tuning: how many ticks of local input the client buffers
  for replay, how snapshots are diffed/compressed for the wire, and how a
  genuine desync (caught by the state hash) is surfaced to the player - none
  of this is decided, all of it sits inside the server-authoritative/
  client-prediction shape section 3 already commits to.
- Whether the original's ad-hoc/game-sharing scope
  (`docs/networking/README.md`) is worth recreating separately from this
  online design, purely as its own RE-and-reimplement exercise - not a
  blocker for anything here, since the two are explicitly independent per
  section 3.
- **`docs/networking/README.md`'s "Pulse likely has no online play"
  conclusion rests on absent PRX files on disc alone, and that may not be
  the whole story.** `sceUtilityLoadNetModule` is a real PSP API for
  loading network modules from firmware flash0 at runtime, not from the
  disc - a 2007-era title could use it without shipping its own net PRXs the
  way 2005's Pure did. Neither this thread nor the review that caught the
  `RaceSim` issue above checked `BOOT.BIN`'s own import table for
  `sceUtilityLoadNetModule`/`sceNetInet*`/`sceNetAdhoc*` NIDs (this session
  had no populated `data/images/`, and reproducing that check needs the
  Ghidra bridge or a PPSSPP import dump against the real disc). Worth
  checking before leaning on "no source to observe" as load-bearing
  justification for the design in section 3 being unverifiable-by-RE - the
  design stands either way (it's chosen on request, not derived from the
  original), but the specific claim that there's nothing to observe should
  be confirmed, not assumed.
- **8-way split screen is 8 full scene renders per frame**, not a cost this
  thread has estimated. Worth a frame-budget pass once the viewport work in
  section 1 exists, before committing to 8-up as a shipped configuration
  rather than a tested ceiling.
- **Zone, Speed Lap and (per the shared-prerequisite section) every mode
  that currently reads `World::race` as a single field is single-ship by
  the original's own design**, not just by this codebase's current
  limitation - `World::race`'s doc comment names Time Trial, Speed Lap and
  Zone specifically as one-ship-on-track modes. Whether any of those modes
  gets a genuine multi-player treatment (a multi-player Zone, say) or stays
  single-ship even after the shared prerequisite lands is a design question
  the prerequisite work doesn't answer by itself.
- Snapshot/delta sizing for the wire (8 ships plus projectiles plus
  `RaceSim`'s non-`World` extras) is unestimated - relevant to the
  transport decision above but not computed here.

## Next Steps

- ~~Land the shared prerequisite (per-player `RaceState`, N-input
  `World::tick`, per-player device mapping in `oag-input`) as its own
  change, before starting split screen.~~ **Done, 2026-09-16** - see
  "Recommended order" step 1 for what landed, and note that the function is
  `Race::tick`, not `World::tick`, which this line still had wrong. Split
  screen is unblocked; its first move is the viewport work in section 1.
- Read `handover/frontend/pulses-race-box-is-read-both-pressings.md` for the
  existing split-screen menu-XML findings before building split screen's
  front-end leg.
- Write the ADR for the server-authoritative/client-prediction/replay-
  reconciliation design in section 3 before creating `oag-net` - the shape
  is decided, the ADR just needs to exist. Not before M8 opens per the
  roadmap, but the decision itself doesn't need to wait for that milestone
  to start.
- Decide the transport (framing, channel split, send rate) as the next open
  question once the ADR is written.
- Before writing that ADR, resolve the remote-player prediction question
  flagged in section 3 (interpolation vs. rollback-with-predicted-inputs) -
  it changes the protocol shape, not just a tuning parameter.
- Check `BOOT.BIN`'s import table for `sceUtilityLoadNetModule`/
  `sceNetInet*`/`sceNetAdhoc*` NIDs to confirm or correct
  `docs/networking/README.md`'s "no online play" reading before citing it
  further - flagged above, not yet done.
