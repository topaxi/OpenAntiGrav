# The original resets a craft airborne for four seconds, and ours now does

2026-09-29. What is left after the sunk-craft port landed (branch
`sunk-craft-2`). The full write-up, with every number:
[docs/gameplay/leaving-the-track.md](../../docs/gameplay/leaving-the-track.md);
the reset state's reading:
[shield.md](../../docs/ghidra/functions/psp-pulse-usa/shield.md#state-3-is-the-reset-and-four-things-enter-it-2026-09-29).

**Landed**: the original's three sunk-craft mechanisms together (floor hull
contacts, `Collision_AddContact`'s projection gate, `Body_StepWorld`'s pass 1);
its **airborne reset** (four seconds with no hover probe touching,
`FUN_088418e0` at `0x08841d30`, watched firing in PPSSPP on `01_Track`), which is
the original's rescue for a craft on its back or flank and what stops the floor
contacts softlocking a player; and, chosen not measured (maintainer decision),
an AI that does not brake for a bend over a gap, so it flies `01_Track`'s lip
the way the original's field does (21 of 21 crossings at 69-111 u/s, logged
live). Same-pose trials at the lip match the original to the tick (reset at 371
against 372). Survey: 589 rescue events to 9.

**How the original was reached**: every circuit is selectable with the
dev-unlock byte (`*(u32 *)0x08b31774 + 0x45f = 1`), recipe in
[ppsspp-debugger.md](../../docs/reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29).
Basilico Black is `01_Track`.

## Open

- **Our reset lands differently.** `Race::respawn` puts the craft at rest on the
  racing line; the original's state-3 update (`FUN_0883ff6c`) puts it at the
  corridor midpoint five up via `AiTrack_LocatePosition` from `craft+0xaf0` and
  launches it at `FUN_0883db20(+0x78c * 0.25 + 50)` (53.5 u/s measured), and for
  the player charges `min(5, shield - 1)` of shield. Same for a `Reset` contact,
  which enters the same state. Confidence 50 on what the `+0x78c` term is.
- **The distance trigger** (`0x08841cec`: more than 200 units from `craft+0xaf0`
  for 0.5 s) is read, not ported; the invented `LostCircuit`/`OffTrack` dwells
  stand where it would. Porting it would retire two inventions and needs its
  own survey. The fourth trigger (`0x0884239c`, `FUN_0883191c(..) == 2`) is not
  read.
- **`05_Track` samples ~1605-1675**: the player's autopilot at every tier grinds
  a wall at 1-3 u/s until its shield runs out (lap 1 after 6,000 ticks); on
  `main` too (Novice and Skilled eliminated there). Opponents there are freed
  by the invented `Stalled` dwell. Not a lip, not airborne, so the airborne
  reset does not fire. A player would steer out; the autopilot does not.
- **Wall contact rose** on `13_Track` (RAPIER 515 to 1,453 contact ticks) and
  `07_Track` (VENOM clean-lap board shield per lap 19.9 to 22.7-25.9) with the
  port; `13_Track` PHANTOM no longer records a clean lap before it dies. Not
  attributed beyond "the hull port alone does the 13 PHANTOM one".
- **`06_Track`'s join at 1196-1200** (upper floor ending, 24-unit drop, no
  `Reset`) was not measured in the original; Vertica is reachable now.
- The original's `01_Track` grid at tick 0 is in the Single Race log
  (`orig01-single-hold.csv`, not committed): the
  left-side grid capture `grid.md` says does not exist.

## Next Steps

1. Port the reset state's relocation (corridor midpoint, five up, launch speed)
   and the player's shield charge, for `Reset` and airborne alike; re-run the
   lip same-pose trial, which should then match the original's relocation too.
2. Read `+0x78c` (`body+0x398` written in `FUN_088418e0`) to settle whether
   `FUN_0883db20`'s first argument is a launch speed.
3. Log a Vertica Single Race through the join at 1196 with the dev-unlock byte.
4. Compare the original's `01_Track` grid from the log above against
   `grid_poses`.
