# Pause in multiplayer: a local timeout, and what online does instead

2026-10-13, maintainer's request. A design note, nothing built beyond the local
rule below. Network play itself is unbuilt (see
[the split-screen and network thread](split-screen-multi-window-network-play-in-order.md),
sections 3 to 3b, and [`docs/networking/README.md`](../../docs/networking/README.md)).

## What exists (local split screen)

Any seated player's Start pauses the race, and the pause belongs to them: only
their Start resumes and only their Circle leaves the race. A pause nobody pressed
for (the window losing focus) is anyone's to lift. Code: `session::pause` in
`crates/game/src/main/session/pause.rs`, with `Controls::take_presses`. Written up
in [`docs/rendering/split-screen.md`](../../docs/rendering/split-screen.md),
"Who plays which slot".

## Open: a local timeout

The owner rule can strand the other players: the owner walks away, or their pad's
battery dies. Proposal: **after a timeout, the pause becomes anyone's** (the
owner becomes `None` in `pause::Paused`), so any seated player can resume or leave.

Decisions still owed, none measured:

- **The timeout**: a fixed value (somewhere around 30 to 60 s), a setting, or both.
  Leaning: a fixed default, no setting until someone asks.
- **What starts the clock**: the pause itself, or the owner's last input. The first
  is simpler and predictable.
- **Whether to show it**: a line under the pause ("Player 2 paused - anyone can
  resume in 0:25"). Without it the rule looks like a bug when another player's Start
  suddenly works.
- **A disconnected owner**: their pad dropping (`split_screen::follow_devices` already
  turns a deviceless seat to AI) could lift the owner at once rather than waiting.
- Wall clock, not ticks: the race is frozen, so the timer cannot count simulation
  ticks; it lives in the session, outside `RaceSim`, and is not hashed.

Small: the pure function gains a "now" and a "paused at", plus tests. Half a day
with the on-screen line.

## Open: online

Online is server-authoritative ([netcode trust model](../../docs/networking/README.md)),
and one player freezing a race for seven others is the problem online games avoid.
What comparable games do (from general knowledge, not researched for this note):
public online races usually have **no pause**; the pause button opens the player's
own menu while their craft keeps racing. Private lobbies sometimes allow a limited
"technical pause".

Options, to decide before the protocol ADR:

1. **No race pause online.** Start opens a local overlay (settings, leave race); the
   race and the player's craft keep running. While the overlay is up the craft
   either coasts on neutral input, or the autopilot flies it (decide which; the
   autopilot already flies any seat, `Race::set_autopilot`). Simplest, and nothing in
   the protocol changes.
2. **Lobby pause for private races.** The host or a majority vote pauses; a pause has a
   budget per player (count and length) and a hard timeout after which the race
   resumes on its own. Needs a server message, a resume countdown so nobody is
   caught out, and the lobby settings to configure it.
3. **LAN client-hosted games** (trusted, per the trust model) can use the local rule
   plus the timeout above, carried as a server event so every client freezes on the
   same tick.

Constraints whatever is chosen:

- **A pause is an event in the input stream**, stamped with the tick it takes effect
  on, never a client freezing its own simulation: every client and the server must
  freeze and resume on the same tick or prediction diverges. The replay records it the
  same way (`oag-replay` treats inputs as the truth), and so does a seat change,
  which is the related open lane `seat-changes-as-input`.
- A resume should have a short countdown (the race start's own, or a shorter one) so
  the players who did not resume are not crashed into a wall on the first tick.
- Leaning (Claude's, to be challenged): option 1 for public online races, option 2
  only once private lobbies exist, option 3 for LAN.

## Next Steps

- Local timeout: decide the value and whether to show it, then extend
  `session::pause` with tests.
- Online: pick between the options above when the protocol ADR is written; until
  then nothing online should assume a race can be paused.
