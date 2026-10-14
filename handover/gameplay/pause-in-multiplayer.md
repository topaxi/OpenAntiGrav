# Pause in multiplayer: a local timeout, and what online does instead

2026-10-13, maintainer's request. A design note; the local rule is built, the online part is open. Network play itself is unbuilt (see
[the split-screen and network thread](split-screen-multi-window-network-play-in-order.md),
sections 3 to 3b, and [`docs/networking/README.md`](../../docs/networking/README.md)).

## What exists (local split screen)

Any seated player's Start pauses the race, and the pause belongs to them: only
their Start (or a click, which is slot 0's Start) resumes and only their Circle
leaves the race. A pause nobody pressed for (the window losing focus) is anyone's
to lift. Code: `oag_game::pause` (the pure rule) and `session::pause` in
`crates/game/src/main/session/pause.rs` (the session glue and the line). Written
up in [`docs/rendering/split-screen.md`](../../docs/rendering/split-screen.md),
"Who plays which slot".

Landed 2026-10-14 (`pause-timeout`, all chosen, not measured):

- ~~**A local timeout**~~: the owner's claim lapses 30 s after the pause itself,
  a fixed value, no setting, wall clock in the session and not hashed.
- ~~**A disconnected owner**~~ lifts the claim at once (`present_seats`).
- ~~**Showing it**~~: a line under the pause, `PLAYER 2 PAUSED - ANYONE CAN RESUME
  IN 0:25`, then `ANYONE CAN RESUME`.
- ~~**A resume countdown**~~: `3`, `2`, `1`, `GO` for 3 s in a race with more than
  one person, the race frozen through it (hash test), Start during it re-pauses.
  Single-player resume is unchanged; widening it is one flag (`Table::countdown`).

## Open: the local rule's edges

- The countdown art is Pulse's `Cockpit_321GO.vex` only. HD, Pure, 2048 and Omega
  author no screen-space countdown, so they show the line `RESUMING` and nothing
  else; a countdown drawn off HD's `321go` billboards or 2048's `321Go_2048.vex`
  would be new art placement work. 2048 and Omega were not run.
- The art sits at the widget's authored spot (63% down, centred), per view of a
  split, so it lands over the craft in a short view. Not tuned.
- No resume row exists (the pause draws no menu), so "click the resume row" is
  "click anywhere on a paused race".

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
  **Decided 2026-10-14 (maintainer):** once a resume is triggered, an on-screen
  `3`, `2`, `1`, `GO` countdown runs, then the race resumes. This is ours (the
  original is single-player and has no resume countdown), so it is **chosen, not
  measured**. The race stays frozen through the countdown.
- Leaning (Claude's, to be challenged): option 1 for public online races, option 2
  only once private lobbies exist, option 3 for LAN.

## Next Steps

- Online: pick between the options above when the protocol ADR is written; until
  then nothing online should assume a race can be paused. A resume countdown
  online is a server event on the same tick for every client, like the pause.
