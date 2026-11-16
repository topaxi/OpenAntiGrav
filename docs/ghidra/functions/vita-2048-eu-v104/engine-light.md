# The engine light on 2048: the same vertex light, in its own executable and its own shader

Read 2026-11-16, lane `engine-glow-re`, on `/2048/eboot-vita-2048-eu-v104.elf` and on the base
package's `data.psarc`. The law, the constants' meaning and the cross-title table are on
[Omega's page](../ps4-omega-eu/engine-light.md); this page holds what was read on 2048 itself.
Static only. **Checked, applies, not wired.**

## The functions

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x811b663a` | `Ship_LoadEngineLightData` | 85 | Formats `%s\EngineLightData.xml` (`0x81493988`) from `*(ship + 0x58e8) + 0x1b0`, or opens `Data\art\published\hdships\Zone\EngineLightData.xml` (`0x81493954`); reads `Distance` into `ship+0xfc` and `Radius` into `ship+0x100`. The same two offsets as HD's `Ship_LoadEngineLightData`. |
| `0x812726d6` | `EngineFlare_SubmitVertexLight` | 82 | Decompiled in full, below. The same branches and constants as Omega's `0x0173e350`; one caller, `0x811c69e8`. |
| `0x8103731c` | `VertexLight_AddCandidate` | 88 | `(range, &position, &colour, owner)`: returns 0 when the count `0x816aeee0` is 32 or `range <= 0`, else appends a `0x24` record at `0x816aea60`: position xyz, `1 / range`, colour rgb, `range`, owner. Omega inlines it into every producer with the same record. |

## `EngineFlare_SubmitVertexLight` (`0x812726d6`)

`flare+0x138` is the ship, `flare+0x34` the flare node's world matrix. Constants are immediates
(`movw`/`movt`), which is how the function was found: it is the only one in the flare's address range
that builds both `40.0` (`0x4220`) and `2.2` (`0x400c`).

```text
ship = flare+0x138
if ship+0x73fd:      base = (3.0, 0.5, 0.1); gain = 2.2; add = 5.0     ; Omega's ship+0x8ba1
elif ship+0x73fc:    base = (40, 10, 4);     gain = 10;  add = 40      ; Omega's ship+0x8ba0
else:                base = (4, 10, 40);     gain = 10;  add = 40

t     = flare+0x240;  tint = t > 0 || flare+0x2bc
strip = ship->vtable[0x3c](ship) && FUN_81000930(0x8153fc40, -1) == 0 && !tint
if strip:            base = (1.5, 0.1, 3.0); gain *= 0.2

if !tint:
    blend  = flare+0x148 + flare+0x154
    colour = base * (1 + gain * blend);  calm = 1 - blend
else:
    the transition tint, as on Omega;    calm = 1

anchor   = node.row3 - node.row2 * Distance          ; ship+0xfc
off      = flare+0x248                               ; vec4
VertexLight_AddCandidate(Radius + off.w * calm,      ; ship+0x100
                         anchor + off.xyz * calm, colour, -1)
```

Not read on 2048: who writes `ship+0x73fd` and `ship+0x73fc` (taken as Omega's two flags by their
place in the law, confidence 70), what `vtable+0x3c` and `FUN_81000930(0x8153fc40, -1)` return
(Omega tests the mag-strip predicate and the mode id there), and the jitter's ranges. 2048's flare
init (`FUN_81270228`) loads `data/Tex/EngineFlare/Engine_Flare_Rich.gxt` only; the executable names no
red sprite.

## The shader: a per-vertex Lambert light with a linear falloff (80)

`scripts/vita-gxp.py grep <data.psarc> DiffuseVertexLight0position` lists 24,291 programs: every
circuit's materials (up to 2,208 per circuit), the shared object library, and 469 under
`Materials/Ships`. The smallest, vertex program #4 of
`DLC1/environments/Anulpha_Pass/materials/cf_alpha4glow.rcsmaterial` (uniforms
`DiffuseVertexLight0diffuse` at resource 38 and `DiffuseVertexLight0position` at 40), disassembled
with `scripts/vita-usse.py disasm`:

```text
17  add  pa2.xyz, -pa0.xyz, sa40.xyz        L = light.position - world
18  dot  pa0.x, pa2.xyz, pa2.xyz            d2
19  rsq  pa4.x, pa0.x                       1 / d
25  mul  pa0.xyz, pa2.xyz, pa4.xxx          L / d
26  dot  pa0.x, pa6.xyz, pa0.xyz            N . L
27  max  pa0.xyz, 0, pa0.xxx
28  mul  pa2.xyz, pa0.xyz, sa38.xyz         * diffuse
29  rcp  pa0.x, pa4.x                       d
30  mad  pa0.xy, pa0.xx, -sa40.ww, sa46.zz  1 - d * position.w
31  max  pa0.xyz, 0, pa0.xxx
32  mul  pa0.xyz, pa2.xyz, pa0.xyz          the vertex light colour, written to o4/o6
```

`position.w` is the record's `1 / range`, so the light is
`diffuse * max(0, N . L) * max(0, 1 - d / range)` per vertex. `sa46.z` is read as `1.0`: the same
register clamps the fog factor with `min` at instruction 33. One program was read; that the
six-light programs repeat the block per light is taken from the uniform names, not disassembled.

HD's SPU law at exponent `1.0` is the same expression (`crates/mesh/shaders/spu_light.wesl`), so the
record and the falloff need no new term; what differs from HD is the colour, the range and the
receivers.
