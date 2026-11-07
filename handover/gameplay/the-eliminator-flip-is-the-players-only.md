---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-07
summary: "HD's Eliminator flip is wired for the player; the AI's flip, the camera through it, and 2048/Omega's per-craft flip time are open"
---

# The Eliminator flip: what is left

Landed: the player's flip on HD and Fury, `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.

## Open

- **The camera through a flip.** The original's hardly turns and then lands reversed; ours swings
  round the hull and can pass through a wall for `0.2 s`. Law unread.
- **The AI's flip.** The chain is read, on that page. Not wired; `WeaponAi_DecideFireOrAbsorb`'s
  absorb gate on the facing byte waits on it.
- **2048 and Omega.** `<Misc eliminator_flip_time>` per team and class, `race_eliminator_flip`, a
  second stick trigger at `0x811b369c`. Their `Title::eliminator_flip` is `None`.
- **Touch** has no flip button.
- **The stop.** Ours stops about `0.2 s` later than the original (page, "Ours").
- **`WRONG WAY`** is dropped off frames; trace `status+0x54`'s writer. The rumble still fires.
- **The bindings row** shows on every title, Pulse and Pure included. Scope it to titles with a flip.
- `Button::Flip` takes index 10: check `psp-pulse-usa/input.md` leaves it free before a PS2 L2/R2 wants it.
- **The flip's sound and effects** were not looked for.

## Next Steps

1. Find the reader of `PlayerInput+0x90` / `+0x94` in HD's camera update (`ps3-hdfury-eu`).
2. Read Omega's `PlayerInput_Update` (`0x012e8fb0`) against 2048's block.
3. Port the AI chain into `oag_ai::weapon_ai` once 1 is done, so an opponent's turn is seen right.
