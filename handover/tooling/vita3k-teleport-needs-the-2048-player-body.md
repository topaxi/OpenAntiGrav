# Vita3K teleport needs the 2048 player's body

2026-10-07. Memory access to a running Vita3K works (`/proc/<pid>/mem`, guest = host
- `0x400000000`, `SIGSTOP`/`SIGCONT`, Vita3K must be a child of the tool). The 2048
craft array (`0x818a6bf8`), role word (`ship+0x7060`) and body layout (`*(ship+0x5f84)`,
basis `+0x08`, position `+0x38`) are recorded in `docs/reverse-engineering/vita3k-capture.md`,
but the body of the ship with role 0 is not the craft the camera follows on the boot
measured, so `vita3k-drive.py place` is not built.

## Open

- Which body the camera's craft is driven by. Held accelerate did not move the visible
  craft on the last boot (copy delta 0), so first confirm input reaches the game.
- A name for Vineta K's DLC track in `oag-game` (`vineta`, `vineta_k`, `vinetak`,
  `vinetta` all missing), needed for any matched pair and for the coordinate transform.

## Next Steps

1. Boot, hold accelerate, and dump twice 0.4 s apart; keep triples that moved with the
   craft's own copy (`0x874a8a30` style), excluding the 35 render copies listed on the page.
2. Read `Ship_UpdateCameraRigs` (`0x811bd89a`) for what it reads as the craft's body.
3. Then write `place` as one run (boot, walk, place, photograph) mirroring `rpcs3-drive.py place`.
