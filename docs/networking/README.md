# Networking

> **Not started in code.** Milestone M8, after single-player is complete. The
> shape is decided in outline (below); one design question is open and there
> is no ADR yet. `oag-net` is deliberately not created until the transport is
> decided, per [workspace layout](../architecture/workspace-layout.md).

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

**Status: measure first.** Nothing is decided and no ADR is written. The
options, their outcomes and the measurements are written up in the handover
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

Costs to measure before committing:

- The replay cost: up to N ticks times 8 craft plus weapons, per correction.
  No reconcile budget exists yet.
- A sim-only tick path. A replay must not re-fire camera shake, particles and
  audio cues; today `Race::tick` mixes view and sim side effects.
- `TickClock::advance` caps catch-up at 8 ticks and discards the surplus, which
  a replay burst exceeds above roughly 133 ms of round trip.

## Transport: WebTransport leaning, not decided

The web build runs in a browser tab, which cannot send raw UDP; WebSocket is TCP
and stalls every packet behind a lost one. WebTransport (QUIC) offers unreliable
datagrams and reliable streams, the two channel kinds the protocol needs.
Browser support per search excerpts: Chrome and Edge from 97, Firefox from 114,
Safari 26.4 (March 2026); not checked against caniuse or MDN.

Decided (maintainer, 2026-10-13): accepting self-signed certificates on a LAN is
acceptable. Native clients may trust a LAN host's certificate; a web client still
needs the host to advertise the certificate hash (`serverCertificateHashes`),
since a browser has no click-through for WebTransport (from memory, verify).

Constraints: it needs TLS, so a public server needs a normal certificate; a
browser tab can be a client but
never a listen server; networks that block UDP fail QUIC. The lean is a transport
trait with the two channel kinds, WebTransport implemented first for the dedicated
server and the web build, and native clients on it too if the LAN certificate
story proves workable (one wire protocol while the design is young). The Rust
crates (`wtransport`, `web-transport-quinn`) are unverified. Details and the proof
order are in the handover thread.

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
