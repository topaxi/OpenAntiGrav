# 2048's drawn-hull shake is HD's, constant for constant

`/2048/eboot-vita-2048-eu-v104.elf`, read from the decompile and disassembly 2026-10-26 (lane `2048-queens-offset`), then
switched off in the running game to see what it moves. HD's law and its derivation are on
[`hull-shake.md`](../ps3-hdfury-eu/hull-shake.md); this page is only what differs or was checked on 2048.

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x811b51bc` | `HullShake_Update` | 85 | `(obj, s0 = dt, s1 = f2)`, Thumb-2. Every constant of HD's `0x000d0c70` is here: the smoothing `0.99 * s + 0.01 * f2` (`0x3f7d70a4`, `0x3c23d70a`), `s = clamp((80 - s) * 0.02, 0, 1)` (`0x42a00000`, `0x3ca3d70a`), the uniform cubic B-spline basis (`0.16666667`, `3t^3 - 6t^2 + 4`), the 60-slot ring (`% 0x3c`), the roll/yaw/pitch terms `0.025`, `-0.01`, `0.025` (`0x3ccccccd`, `0x3c23d70a`), the offset `5.0 * (p - q)` and the `0.8 * s` scale; the new spline point is `(4.0, 1.8, 1.8) * (rand() * 4.656613e-10 - 0.5)` (`2^-31`: `SceLibc`'s `rand` is taken as a 31-bit value where HD's returns 30 bits, so the points are centred either way) |
| `0x811b5500` | `HullShake_Apply` | 70 | `(obj, node, invert)`: does nothing unless the byte at `obj+0x44c` is set (`Update` writes it as `s > 0`, the fade's own non-zero test), then builds three rotations from `obj+0x10` and composes them onto `node`. Three callers (below). With `invert = 1` it writes the node back inverted |

Object layout, from the offsets `Update` reads: spline points at `+0x40..+0x70`, parameter at `+0x80`, smoothed thrust at `+0x20`,
history ring from `+0x88` (HD: `+0x90`), ring index at `+0x448` (HD: `+0x450`), the active byte at `+0x44c`. The ship holds it
at `ship+0x6158`.

## Where it is called

| Call site | Function | What it shakes |
| --- | --- | --- |
| `0x812cb006` `Update`, `0x812cb010` `Apply` | `0x812caf1c`, the drawn hull's matrix (one caller, `0x812cb4da`) | the hull: `Update(obj, dt, *(*(ship+0x5f84)+0x428))`, then `Apply(obj, hull node, 0)`. `f2` is the body's `+0x428` where HD reads `+0x4c4` |
| `0x811c1580` `Update`, `0x811c15a0` `Apply` | a function Ghidra has not made one of | the hull again (the hull scale `DAT_8151fdd0` or `DAT_8151fdf4` is loaded right after, as after the first) |
| `0x811be926` `Apply` | `0x811be2b0` | the ship's `"internal tripod"` (`ship+0x5650`, the constructor's own label for the chase camera's node) with `invert = 1` |

Further `Update` callers exist (`0x810c93ac`, `0x810da8a6`, `0x8113a58c`); only the last was looked at: it runs an object at
`r11+0x110` with `f2 = 0`, not the ship's. The other two are not read.

**The constructor seeds the spline**: `rand() % 500` calls of `Update(obj, 0.01, ...)` in the ship constructor `0x811b8806`, and the
same loop again in `0x811c82dc` (not read further), so a ship's first shake position is a random point of the spline, not its start. HD's constructor was not read
for this (`hull-shake.md`, "What is not read"); ours draws four fresh points at a parameter of zero.

## What switching it off does (2026-10-26, Vita3K, Queens Mall)

The two hull `bl 0x811b5500` (`0x812cb010`, `0x811c15a0`) overwritten with `nop nop` in the running process, before the guest has
run them (`scripts/vita-2048-pair.py`, `patch_hull_shake`, bytes read back from the process: `eaf676fa` and `f3f7aeff`, the two
`bl` encodings, then `00bf00bf`). `Update` keeps running, so `rand` is drawn as before.

| | hull centroid range over the settled burst frames, `x` / `y` px |
| --- | --- |
| stock, two cold boots (`compare-page.md`, 2026-10-22) | 3 to 11 / 3 to 20 |
| patched, boot A | 5.1 / 8.3 |
| patched, boot B | 17.2 / 9.6 |

Boot B's 17 px is the yellow-pixel centroid picking up other yellow in the frame (the metric is not a hull measure on this circuit,
`compare-page.md`); read on the nose tip the hull moves less than that. The patch is shown live by the readback, not by this range.

The hull's motion in the original's countdown falls but does not vanish. The rest is the camera's own tripod shake (the same law on
the same object, the third call) or the hover; **it cannot be patched the same way**, because the tripod call has `invert = 1` and
is the step that turns the node into the view matrix: with it overwritten the HUD never came up after Retry (one boot, no
frame of the race).

## Omega

**Checked, applies, not wired.** Omega's base `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`, v1.00) carries the same function at
`0x012f7200`, found from the constructor's `"%s internal tripod"` string (`0x01814c35`, `0x0181533b`) and the 60-slot ring's `0x88888889`
divide: the same smoothing, fade, spline, ring and `0.025`, `-0.01`, `5.0`, `0.8` terms, with HD's own layout (history at `+0x90`, index at
`+0x450`, active byte at `+0x454`) and the points drawn as `(4.0, 1.8, 1.8) * (rand() * 2^-30 - 0.5)` as HD does. Six callers
(`0x0130ebdf`, `0x012ec8b8`, `0x0130a16f`, `0x014923f4`, `0x0157e1a7`, `0x01764c81`). Confidence 80 for "the same law" (read from the decompile,
nothing run). Not done: the `Apply` side, and a patched Omega pair, because the pair script races on the **patch v1.07**
executable, which is not imported in Ghidra (the base is), so the call-site addresses of the running build are unread, and
shadPS4's own patch mechanism was not tried.

## Ours

`oag_title::hull_shake::HullShake::HD` already carries the law, and 2048 and Omega inherit it (`RaceDefaults::hull_shake`,
`Inherited` from HD). This page moves that from "unread" to **read and identical on 2048 and on Omega's base executable**; the
provenance value in the code was left as it is (a change to it is a `.rs` edit that moves no behaviour). What ours does not do and the
original does: seed the spline with `rand() % 500` steps.
