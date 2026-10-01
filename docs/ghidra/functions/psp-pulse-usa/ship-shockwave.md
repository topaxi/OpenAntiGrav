# The ship explosion's shockwave: `FUN_0885ecf0` and `FUN_0885efc4`

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | weapons and effects, ship destruction |
| **Related** | [`screen-flash-callers.md`](screen-flash-callers.md) (`Ship_SpawnExplosionBig`), [`ship-wreck-model.md`](ship-wreck-model.md), [`mine.md`](mine.md) (`BombBlast_Update`, the Bomb's own use of the same `.vex`), [`shield-pickup.md`](shield-pickup.md) (`mesh+0x6c`) |

Read 2026-10-01 (headless Ghidra on a scratch copy of the project) and logged on a running
original (PPSSPP v1.20.4, Talon's Junction, a grid opponent put into `Ship_SetState(entity, 4)`,
`scripts/psp-wreck-capture.py --hits`). Question: `Ship_SpawnExplosionBig` builds a
`Data\Weapons\Bomb_Shockwave.vex` object that `screen-flash-callers.md` called "read, not drawn" -
what is it, and is it seen?

## What it is

`Ship_SpawnExplosionBig` (`0x088407b0`) calls `FUN_0885ecf0(object, matrix)` with the hull's world
matrix, before it moves that matrix's translation for `WO_SHIP_EXPLOSION`. The constructor:

- copies the 4x4 into `+0x40..+0x7c` (rows: `+0x40` right, `+0x50` up, `+0x60` forward, `+0x70` position),
- sets the vtable at `+0x38` to `0x08aca678` (its `+0x24` entry, `0x0885efc4`, is the per-tick update;
  the rest are the generic node methods),
- loads `Data\Weapons\Bomb_Shockwave.vex` into a `0x1d0`-byte node under `DAT_08b32c88`
  (the same file `BombBlast_Construct` loads for a Bomb),
- writes the eases: scale `+0x8c = 0.1` toward `+0x90 = DAT_08ab0f30 = 20.0` at rate
  `+0x94 = DAT_08ab0f2c = 0.05`; alpha `+0x98 = 1.0` toward `+0x9c = 0.0` at rate
  `+0xa0 = DAT_08ab0f28 = 0.02`; age `+0x88 = 0`.

The update `FUN_0885efc4(dt, object)` steps each ease `(int)(dt / (1/60))` times
(`x += (target - x) * rate`), adds `dt` to the age, builds the model's matrix from the object's:
`n = normalise(up row)`, `t = normalise(ref - n (n . ref))` with `ref = (DAT_08a907c0..c8)`, the same
`(0, 0, 1)` `BombBlast`'s basis uses, then rows `(n x t) s`, `n s`, `t s` and the position -
**a uniform scale `s`**, where the Bomb's ring widens in its own plane only. It calls
`Image_SetVertexColours(model, (alpha * 255) << 24 | 0xffffff)` and, at `age >= 1.5`, clears bit `4`,
sets `0xa` and queues the node for destruction (`g_pending_destroy_nodes`).

## Logged live (one boot, 81 hits over frames 119 to 199 after the call)

| Frame (after the call) | dt | age | scale | alpha |
| ---: | ---: | ---: | ---: | ---: |
| 119.16 | 0.0166 | 0.0000 | 0.100 | 1.0000 |
| 121.12 | 0.0167 | 0.0333 | 1.095 | 0.9800 |
| 122.12 | 0.0167 | 0.0500 | 2.040 | 0.9604 |
| 123.12 | 0.0167 | 0.0667 | 2.938 | 0.9412 |
| 124.13 | 0.0167 | 0.0834 | 3.791 | 0.9224 |
| 125.12 | 0.0167 | 0.1001 | 4.602 | 0.9039 |
| 199.09 | 0.0167 | 1.3343 | 18.545 | 0.3569 |

`0.1 + (20 - 0.1) * 0.05 = 1.095`, and so on to every digit. Position `(-112.61, -49.85, -195.41)`,
the craft's own and not the explosion effect's point `2.9995` units lower; up row
`(0.002, 0.75, -0.009)`. Confidence **90** for the law (decompile and the log agree); **88** for the
update's name, **85** for the constructor's.

## Is it seen: yes, as geometry

A GE dump (`--ge-dump-k 121` and `147`, `scripts/psp-ge-dump.py`'s reader) shows three additive strips
that a dump of the same scene 20 frames before the call does not: **43, 97 and 47 vertices**, vertex type
`0x13f`, blend `SRCALPHA + FIX(0xffffff)` - the three submeshes of `Bomb_Shockwave.vex`
(`shockwaveShape`, `edgeShape`, `edge1Shape`, 187 vertices; textures `pulse_ship_shock_ADD` and
`hemisphere_disperse4_ADD`). The ring is `5.365` across its radius at scale `1`, so by the fifth tick
(scale `4.6`) it is `25` units out, flat at the craft's height, and by the thirtieth (`15.8`) it is `85`.

**What it looks like** (seen once, frames `opA`/`opB` against ours): from a chase camera at about the same
height the ring is edge-on, a thin orange line along the horizon across the whole width that decays over
about fifty frames, and for the first five frames a broad white band with a hard lower edge on the
side of the wreck. Ours had neither before this change.

## The alpha reaches the draw, as the ambient light's alpha

`Image_SetVertexColours` (`0x089122b4`) stamps its word at `mesh+0x6c` of every `Mesh`-class node under the
object (up to ten). The GE reads it as the **ambient light** (`shield-pickup.md`, third pass: scene ambient,
the vertex colours the material). In the two dumps (`geA`, `geB`) the ring's six strips are drawn with lighting
on (`0x17` = 1), no lights enabled, `MATERIALUPDATE` (`0x53`) `7` so the vertex colours are the material, and the
**ambient light colour/alpha `0x5c`/`0x5d` written in the command list just before each strip**:

| Dump | `0x5c` | `0x5d` | The ease `0.98^n` at |
| --- | --- | --- | ---: |
| `geA` strips 6691, 6701, 6711 | `ffffff` | `f4` | `0.957` |
| `geB` strips 6694, 6704, 6714 | `ffffff` | `b8` | `0.722` |

Other model draws in the same frame leave `0x5d` at `0xff` and lighting off. So the drawn alpha is the vertex alpha times
the ease - `Drawable::tint([1, 1, 1, alpha])` - and the Bomb's own shockwave, whose `+0xf4` ease writes the same call,
fades the same way (`0.1` a tick). Confidence **88** (two dumps, six strips, the register written just before each).
The ring's vertex words in the buffer are the authored ones, as a lit material leaves them: that was first
misread as "the fade does not reach the draw", from the *material* registers `0x55`/`0x58`, which stay `ffffff`/`ff`.

**The ring is still too dim in ours.** With the fade the horizon band at `(300..480, 118..138)` reads, in the red channel at
140, 150, 160 and 170 frames after the call, `176, 148, 105, 76` on the original and `164, 82, 66, 59` on ours (`182, 104, 82, 74`
before the fade was wired): the original's band stays brighter than ours either way, which is the same gap as the first five
frames' white. The strips' state (additive `SRCALPHA` + `FIX 0xffffff`, `TFUNC 0x100`, alpha and colour tests on, no culling,
`TLEVEL` slope mode) was read, and the cause is open.

## Not read, open

- The ring's brightness (above): the first five frames and the horizon band read about `1.5` to `2` times brighter on the
  original. The textures decode plausibly (`pulse_ship_shock_ADD` a fire noise, the edge texture a white-to-brown gradient); the
  blend, the colour test (`NOTEQUAL` black), `TLEVEL` (`c8:050002`, bias 5, slope mode) and the filter were not compared
  against ours state by state.
- `DAT_08b34320` gates the whole object (`0x8c0f410` live); what clears it is unread, so every wreck
  here throws the ring.
- Whether `FUN_0885ecf0`'s other callers exist: one xref, `Ship_SpawnExplosionBig`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0885ecf0` | `ShipShockwave_Construct` | 85 |
| `0x0885efc4` | `ShipShockwave_Update` | 88 |
