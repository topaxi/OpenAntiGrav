# The diffuse sun is its colour times `Lighting.Sun diffuse intensity`

2026-10-19, `omega-pink-road-sun`. `eboot.bin` (Omega, PS4, EU), Ghidra program
`/omega/eboot-ps4-omega-eu.bin`. **Static evidence only** (no PS4 emulator), so
confidence is capped at 84 by the [rubric](../../../reverse-engineering/confidence-rubric.md).
No function is renamed: the registrar and the consumer stay `FUN_015c1f20` and
`FUN_015c63a0`, so there is no `names.tsv` row.

## What reads it

`FUN_015c1f20` is the registration table `lightmap-prelit.md` describes. Three
of its rows matter here:

| Key | Destination | Registrar |
| --- | --- | --- |
| `Lighting.Sun diffuse colour` (`0x0182cd89`) | `0x01e3de70` | `FUN_01791910` (colour) |
| `Lighting.Sun diffuse intensity` (`0x0182999f`) | `0x019404b4` | `FUN_01791540` (scalar) |
| `Lighting.Sun specular intensity` | `0x019404b0` | `FUN_01791540` (scalar) |

Both consumers of the pair multiply them where the light is built:

```text
015cb49c  VBROADCASTSS XMM0, [0x019404b4]     ; diffuse intensity
015cb4a5  VMULPS       XMM0, XMM0, [0x01e3de70] ; times diffuse colour
013c2a5d  VBROADCASTSS XMM2, [0x019404b4]     ; the same pair in FrontendRoot_Construct's block
013c2a6a  VMULPS       XMM2, XMM2, [0x01e3de70]
013c2a7a  VBROADCASTSS XMM1, [0x019404b0]     ; specular intensity ...
013c2a83  VMULPS       XMM1, XMM1, [0x01e3de20] ; ... times specular colour
```

The static default of the scalar is `1.0` (`0x019404b4` and `0x019404b0` read
`0000803f`), so a file that omits the key draws its colour as written.

## What the files author

Over Omega's circuit `track.EnvSettings` the intensity is authored 0.3
(Sebenco Climb), 1 to 1.8, 2 to 3.5 (Altima 3, Tech De Ra 2.528), 5 (Mall,
Square), 6 (Tower), 8 (Sol) and 20 (Bridge). 2048's Vita files carry no such key
(`2.0 1.8 1.7` is the whole of Altima's sun there), so 2048 is unchanged:
**checked, differs**. Tech De Ra's `4 2.328 0.816` that `omega-status.md` quoted
as "the sun" was this colour alone; the sun the executable builds is
`[10.1, 5.9, 2.1]`.

## Wired

`oag_raceplay`'s `envsettings_light` multiplies the colour by the scalar on a PS4
source only (`oag_tables::envsettings::OMEGA_SUN_DIFFUSE_INTENSITY`).
`omega_lightmap_ground_truth` asserts Tech De Ra's `light.sun` equals colour times
2.528. The specular pair is left unwired for the shape reason `envsettings.md`
gives (an RGB specular colour against a scalar scale).

Confidence 80: both consumers are read at the instruction level, the destination
of the product is not followed to the shader constant, and the picture was judged
by eye only (before/after stills on seven circuits, tone map on).
