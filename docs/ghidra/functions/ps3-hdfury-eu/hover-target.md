# HD's hover target: lowered on the grid, released two and a half seconds after GO

2026-10-08, lane `hd-flyby-hover`. Binary `EBOOT.elf` (BCES-00664, TOC `0x008ad4d8`). The
question: why does the original's craft sit 1.84 units lower than ours in the pre-race flyby
([`hd-ride-height.md`](../../../physics/hd-ride-height.md)), and what lifts it. Read from the
disassembly (capstone through `data/scratch/hd-weapon-blasts/ppcdis.py`), then checked against
three live RPCS3 boots (`scripts/rpcs3-height.py --count-secs`), so every number below has a
static source and a live one.

| Address | Name | What | Confidence |
| --- | --- | --- | --- |
| `0x000f0630` | `Craft_SetState` | `(entry, state)`: writes `entry+0x2f8`; state 0 sets `entry+0x264` bit 1 (value 2), state 1 clears it and zeroes `+0x310/+0x314/+0x318/+0x31c`, state 3 runs two cleanups | 85 |
| `0x000f1958` | `Craft_Update` | the per-frame craft update `(dt, entry, ctx)`; the hover target block is at `0x000f1c20-0x000f1d00` | 75 (the name: it calls `Craft_UpdateAirbrakes`, `Craft_UpdatePitch`, the hull and probe code) |

## The law (confidence 90)

`entry` is the `0x600` object at `ship+0x5fac`. In `Craft_Update`, after the hover base
(`*(entry+0x7c) + entry+0x80`, the handling row's ride height; slowdown subtracted via `+0x334`) and
the magnet-lock gain `x (1 + 0.2 * entry+0x2c0)` (`0x000f1c80-0x000f1c9c`):

1. **Grid state** (`entry+0x264 & 2` set, which is `entry+0x2f8 == 0`): `0x000f1cb4-0x000f1cc4`
   stores `entry+0x348` into `entry+0x344`, replacing the base. `entry+0x34c` counts down by `dt`;
   below zero (`0x000f27e8-0x000f284c`) it is re-rolled to `rand8 * 0.002` and `entry+0x348` to
   `3.0 + rand8 * 0.0003` (`rand8 = Libc_Rand() & 0xff`; constants at `TOC-0x42ac` and `TOC-0x42a8`,
   `3.0` is the literal `0x40400000` stored to `+0x2f4` at `0x000f27e8`). So `+0x348` is a
   jittering `3.000 .. 3.0765` and `+0x34c` is `0 .. 0.51` s, which is what the live reads show
   (`3.00 .. 3.08`, `0.10 .. 0.41`).
2. **Raced** (bit clear): `0x000f23bc` adds `dt` to `entry+0x348` every frame (a timer in seconds),
   and `0x000f1cd8-0x000f1ce4` takes `entry+0x344 = min(base, entry+0x348)`.
3. Both paths end at `0x000f1ce8-0x000f1cfc`: `entry+0x344 = (that) * TARGET_GLOBAL_SCALE` (the
   `0.75` global `oag_physics::hover::TARGET_GLOBAL_SCALE`; the pointer at `TOC-0x42fc` reads `1.0` in
   the file and is set to `0.75` by the craft constructor, as on PSP).

So the target is `0.75 * 3.0x = 2.25 .. 2.31` while the craft is in state 0, and after state 1 it is
`0.75 * min(5.5, 3.0x + t)`: a linear ramp at `0.75` units a second that meets the race value
`0.75 * 5.5 = 4.125` after `(5.5 - 3.04) = 2.46` s. The craft follows its target to within `0.04`:
height `+0x260` = target `- 0.16`.

Live (three boots, Talons Junction, Venom, Time Trial, `data/scratch/hd-flyby-hover/c1..c3`):
`0.75 * entry+0x348 == entry+0x344` to three decimals on every ramp sample (c2: `4.484 -> 3.363`;
c1: `5.346 -> 4.010`; c3: `4.810 -> 3.608`), `+0x348` advances `1.00` per game second, the entry's state
reads 0 and the flag word `3` through the flyby **and the whole countdown**, and flips to state 1
(flag word `1`) on the sample where the HUD shows `GO` with the race clock at `0.00.0` (c2 frame
`count-008`, the sample before reads `2`): **the flip is at GO**, not at the cross tap. Heights:
2.13-2.19 through flyby and countdown, `3.20 -> 3.27` 1.4 s after GO, `4.00` by about 2.5 s.

## Who flips the state

`Craft_SetState(entry, 1)` has these callers (scan of `bl 0x000f0630`): `0x00054838` and
`0x00057940` (both a loop over the manager's craft array at `manager+0xe8`, for ships whose
`ship+0x628c` is 0 or 2, called from the race manager's constructor path and from ten mode managers),
`0x000e3f28` (sets `ship+0x628c = 2` and the craft to state 1, reached from `0x000e6060`), `0x000e1e8c`, `0x000e40f8`,
`0x000eaf2c`, and `0x00058728` (state 0). **Which of them runs at GO is not read**: the timing is
measured, the caller is not named. AI craft run the same loop, so the same law covers them (read,
not measured: Time Trial had no AI).

## Not read

- Modes where `*(0x009384e1) == 0` and the game state `+0xe0` is 6, 0xd or 0xe skip the grid-state
  branch (`0x000f2390-0x000f23a8`): which modes those are is unread.
- `0x00058728` (state 0) and the other callers may lower a craft again after a respawn.
- The `+0x348` jitter is random per craft and per re-roll; its mean is the only thing ported.

Other titles: Pulse's craft update is the PSP/PS2 `Ship_UpdateCraft`; whether it carries the same
grid-state branch is **not read** (Pulse measured no lowered flyby craft; its target block is
unchecked). Omega and 2048: not checkable here (no PS4 emulator; 2048's craft is a separate build).
