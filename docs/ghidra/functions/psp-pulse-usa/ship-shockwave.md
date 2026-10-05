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
about fifty frames. **(Corrected 2026-10-01, pulse-fx-3: the broad white band with a hard lower edge on the side of the wreck
that this page first credited to the ring is the `Glow` template's quad, a different object - see
[particle-system.md](particle-system.md#a-sprite-templates-own-sprite-and-what-the-explosions-first-five-frames-are-2026-10-01-pulse-fx-3).
A software rasteriser run over the dump's own ring strips, matrices and textures gives a thin ellipse at `y 128-149`, nothing like a band.)**

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

**The ring's late dimness was the original's scheduler, not a render difference (closed 2026-10-01, pulse-fx-3).** The update steps each
ease `(int)(dt / 0.016666668)` times and **carries no remainder**, and the original's `dt` on PPSSPP is the emulated clock's own,
which jitters around `1/59.94 s`: of the 81 frames logged in `shockA`, **52 stepped** (`0.016593`, `0.016654`, `0.016543`... truncate to
`0`). So the original's ring reached step `14, 20, 26, 33` at frames `140, 150, 160, 170` where ours, one step per 60 Hz tick, had
reached `21, 31, 41, 51`: the logged alpha at frame 199 is `0.3569 = 0.98^51`, not `0.98^80`, and `geB`'s `0xb8` (`0.722`) is `0.98^16`
at 28 frames. At **equal step count** the horizon band `(300..480, 118..138)` reads, in the red channel, `185, 158, 110, 73` on ours
against `176, 148, 105, 76` on the original (steps `14, 20, 26, 33`): the draw agrees to within 7 % (`+5, +7, +5, -4 %`), so nothing in the strips'
state (additive `SRCALPHA` + `FIX 0xffffff`, `TFUNC 0x100`, alpha and colour tests on, no culling, `TLEVEL` slope mode) needs changing.
The textures decode identically (the dump's level-0 pixels equal ours for the ring's 64x64 `CLUT4`, error `0.0`), the model's scale
matches (`35.4` units per scale step on both: the dump's `world` row `72.15` at scale `2.04`), and the colours and alpha ride the same
vertices. **Not ported:** whether a real PSP, whose vblank is locked to `59.94 Hz`, truncates the same third of frames is unmeasured
(an emulated-time jitter of `0.1 %` flips the truncation, and a locked `16.683 ms` frame would always pass), so one step per tick is
what the code does for any `dt >= 1/60` and is kept. The same `(int)(dt/(1/60))` idiom is in `BombBlast_Update` (`0x0887250c`, its three eases, read 2026-10-01), so the Bomb's dome and ring also
ran about a third slow on PPSSPP in every capture taken so far. **`ParticleSystem_Update` does not do it**: it scales `dt` to a fractional tick
count (`60 * dt * the clock's scale`, clamped at `3.0`), so a particle's age follows the emulated clock proportionally and frame jitter
drops nothing there.

## The ring's texture time is seeded `0` at construction (2026-10-05, static only)

`ShipShockwave_Construct` stores `0.0` to `+0x88` and calls `Node_SetAnimTimeTree(0.0)` on the ring's
model at `0x0885ee44` (`f12 = f20 = 0`), the same seed `BombBlast_Construct` uses, so the ring's texture
should play on its age at rate 1 as the Bomb's does. **Confidence 80, below the 88 the Bomb and Repulser
carry:** this is the instruction read plus the shared model, not a live capture, and the ship explosion was
not among the five measured. `write_bomb_blasts` uploads it at the blast's age on that reading. See
[`anim-transform.md`](anim-transform.md#a-meshs-texture-time-seeded-per-spawn-so-object-age-2026-10-05).

## Not read, open

- ~~The ring's brightness~~: closed above (the scheduler for the late band, the `Glow` template for the first five frames).
- `DAT_08b34320` gates the whole object (`0x8c0f410` live); what clears it is unread, so every wreck
  here throws the ring.
- Whether `FUN_0885ecf0`'s other callers exist: one xref, `Ship_SpawnExplosionBig`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0885ecf0` | `ShipShockwave_Construct` | 85 |
| `0x0885efc4` | `ShipShockwave_Update` | 88 |
