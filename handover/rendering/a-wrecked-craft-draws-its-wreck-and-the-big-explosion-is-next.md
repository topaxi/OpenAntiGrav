# A wrecked craft draws its wreck and throws its sparks; the big explosion and the player's destroy camera are next

2026-10-01, lane `pulse-wreck`. Evidence page:
[ship-wreck-model.md](../../docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md).

**Landed.** `Ship_SetState` case 5 makes `shipwreck.vex` the live model; here that is
`CraftState::Eliminated`. `oag_game::race::scene::wreck` draws it (zonewreck in Zone) in
place of the hull, with the hull kept through `Destroyed`; `race::wreck_fx` spawns
`WO_SHIP_FXNODE_EXPLO` then `WO_SHIP_DEATH_SPARKS` at each wreck locator on that edge. Pulse
on a PSP disc only. Checked against PPSSPP at 480x272, same circuit (16_Track) and same craft (a
grid Piranha), `scripts/psp-wreck-capture.py`: the wreck's shape, pose, scorch and fire glints
agree frame for frame. `--force-wreck TICK:SLOT` puts a craft into the sequence for a headless
frame, `--no-hull-wreck` leaves the hull, and `wreck_ground_truth.rs` fails if the load, the
swap or the draw is dropped.

**Corrected.** The wreck carries **no** `0x2000` extra pass (its meshes read `0x1821`, the
hull's `0x3001`); the first note read the shared light-basis word. Bit `2` of the model flags
hides nothing: bit `4` is the visible bit and it moves hull to wreck.

## Open

- **`WO_SHIP_EXPLOSION`**, 1.5 s after the edge (state 6): seen live at that moment, not
  played. `FUN_088407b0` places it with a matrix (`local_370`) under a parent node, both unread.
  Its screen wash is already started by `race::craft_flash`.
- **The player's destroy camera** (`Camera_SetMode(.., 5)`): the original pulls the camera high
  and back; ours keeps the chase camera, so a player wreck frames differently. The player's
  own state-5 wash is skipped for the same reason (`craft_flash`).
- The wreck's two authored `Trail` nodes ([exhaust.md](../../docs/ghidra/functions/psp-pulse-usa/exhaust.md)):
  not looked for in the original's wreck frames.
- Under a wreck our blob shadow, shield shell and absorb overlays still draw as for a hull; the
  engine flare quad is hidden (seen, mechanism unread). `+0x2c` bit `0x1000000` appears on the wreck
  at state 6, unread.
- The fireballs in the first frames are whiter in ours than the original's warm orange; the
  scale given to the two effects (`1.0`) is chosen, not measured.
- PS2 Pulse, Pure and HD: none of their wrecks or `Ship_SetState` is read; Zone's `zonewreck.vex`
  loads but no Zone craft was wrecked on the original.

## Next Steps

1. Read `FUN_088407b0`'s matrix and parent for `WO_SHIP_EXPLOSION`, add it to
   `race::wreck_fx` at `craft_flash::BLOWUP_DELAY`, compare with a capture.
2. Port camera mode 5 so a player's wreck frames as the original's, then un-skip the player's
   state-5 wash.
3. `scripts/psp-wreck-capture.py --state 5` on a craft a race has eliminated in an Eliminator
   run, to confirm the same frames come out of a real shield depletion and not only the call.
