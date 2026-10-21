# HD's drawn hull shakes off its body every frame; ours draws the body, so every HD pair's craft sits 3-13 px off

2026-10-22 (`hd-hull-bay`). The largest craft difference left on the 13 HD compare pairs is placement,
not material: our craft sits 3 to 13 px from the original's (Talon's Junction `(0, -9)`, Sol 2
`(+13, -6)`, Modesto Heights `(-12, +1)`), while the floor beside it aligns within 1 px. Ours draws
exactly the RAM body; the original does not. Evidence and numbers:
[hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "The drawn hull is not the body";
the code site: [ship-lean.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md), "Not the last step".

What is measured (RPCS3, interpreter):

- The hull's `EdgeGeom` matrix (`0x005d8488`, `r11`, scale 0.75) equals `ship+0x40` (copy at `+0x80`)
  and is 1.1-1.8 degrees and up to 0.09 units off the body rows at `body+0x1d0`, about an axis that
  differs per read. Lag and the camera are ruled out (the original's craft is still between frames
  0.12 s apart; the near floor aligns).
- In `Ship_UpdateEntity` (`0x000eadb8`) the node matrix equals the body at `0x000ebc78` and carries the
  offset at `0x000ebc88`, different on consecutive frames. Between them:
  `FUN_000d0c70(ship+0x6b00, f2 = *(r9+0x4c4))` and `FUN_000d39d0(ship+0x6b00, node, 0)`.
- Posing ours with one read halves Talon's craft difference (32.37 -> 16.04 at zero shift). That was a
  temporary override to prove the cause, not a fix; do not land a per-pair pose as accuracy.

## Open

- The law of the `ship+0x6b00` object: what drives it (time, speed, hover, engine), its amplitude and
  rate, and whether it runs for opponents too. Both functions defeat Ghidra's decompiler (VMX,
  0x258 and 0x4a4 bytes).
- Cross-title: Omega carries HD's engine (expected, not checkable, no PS4 capture); 2048 not compared;
  whether Pulse/Pure shake their hull is unread.

## Next Steps

1. Read `FUN_000d0c70` and `FUN_000d39d0` from the disassembly (`mcp__ghidra-mcp__disassemble_bytes`
   on `/hdfury/EBOOT-ps3-hdfury-eu.elf`); name them only once the reading reaches 50.
2. Sample live: restore a Time Trial state with `scripts/emu-restore-state.sh <lane> <state> --interpreter`,
   read `ship+0x6b00..` and the `0x005d8488` matrix against `body+0x1d0` over a few seconds at rest and at
   speed. Restore fresh for each attach: the GDB stub wedged on a second attach three times this session.
   A probe is a few lines over `scripts/rpcs3-hd-hull-light.py`'s helpers (`stop_at`, `place.find_player`,
   `place.read_pose`): break at `0x005d8488`, keep the matrix with scale 0.75 within 0.1 of the body, or stop
   at `0x000ebc78`/`0x000ebc88` with `r31` the ship and read the matrix at `r28`. Land it under `scripts/`.
3. Port the law to the drawn model matrix in `oag_raceplay` (view only, unhashed, its own seeded `Rng` if
   it is random), then `just compare-rerender-ours --only compare-page/hd` and measure each pair's craft
   offset with a craft mask (render once with the craft lifted out of view, `--pose` y + 300).
