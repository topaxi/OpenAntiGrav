# HD/Fury: `zoneTexVis` is sampled with the sRGB decode on (2026-11-23, `det-floor-gain-re`)

**The question.** Fed the original's own held bands, ours' Detonator floor (the `zoneTexVis` visualiser glow,
[zone-shader.md](zone-shader.md)) is 1.7 to 2.3 times the original's blue after the near-plane fix
([zone-shader.md](zone-shader.md), "The glow's depth factor"). Is there a gain on the strip itself?

**The answer.** The strip's texture unit decodes R, G and B with the sRGB curve before the program sees them.
Ours samples it raw and adds the raw texel into HD's linear scene. That is a real, read difference
(confidence **75**), but it is **not the brief's factor of about 0.5**: applying it alone overshoots: ours goes from 1.7-2.3 times the original's blue to
0.38-0.89 of it. **Read, not sufficient.** A second, unread factor makes the original's floor brighter than
the decode alone predicts (candidates below).

## The law

```text
vis = srgb_decode(zoneTexVis[band].rgb)     ; per channel, gamma nibble 7 = R|G|B, alpha raw
glow = saturate(N.y - 0.5) * (1 - windowDepth) * E.w * vis + ring    ; unchanged, zone-shader.md
```

`srgb_decode` is the piecewise curve (`c / 12.92` below `0.04045`, else `((c + 0.055) / 1.055)^2.4`), the
one RPCS3's `_process_texel` applies per gamma bit and the one
[hd-track-tone.md](../../../rendering/hd-track-tone.md), "The texture unit's decode is the sRGB curve",
already wired for every other gamma-7 unit of HD.

## Evidence

**1. The strip's address word.** `Environment_LoadStageTextures` (`0x003d6dc8`), after the 640-texel glyph
loop, builds the strip:

```text
FUN_005a6ce0(&img, 0x100, 1, 8, 0, 0, 0, 0)        ; Image_Create: 256 x 1, format 8
FUN_005aa1d8(&tex, base + 0x34a8, 0)               ; Texture_CreateForImage -> base + 0x32b0 (zoneTexVis)
tex+0x28 = (tex+0x28 & 0xff0fffff) | 0x00700000    ; gamma nibble = 7
tex+0x34 = (tex+0x34 & 0xf0f0ffff) | 0x01010000    ; min 1, mag 1: nearest
256 x u32 zero fill of *(tex+0x10)
```

**2. `+0x28` is `SET_TEXTURE_ADDRESS`, `+0x34` is `SET_TEXTURE_FILTER`.** `Texture_CreateForImage` calls
`Texture_BuildGcmRegisters` (`0x005a9998`, [renderer.md](renderer.md)), which leaves the object's words at
`+0x24` format (`0xa029`/`0xa039`/`0xa02d` plus the mapped format), `+0x28 = 0x60030303`,
`+0x2c = 0x80078000` then `& 0x8000000f | 0x78000` (`CONTROL0`: enable, max LOD), `+0x34 = 0x02052000`
(`FILTER`, then min/mag patched at bits 16-19 and 24-27). Read against the RSX method order
(`OFFSET 0x1a00`, `FORMAT 0x1a04`, `ADDRESS 0x1a08`, `CONTROL0 0x1a0c`, `CONTROL1 0x1a10`, `FILTER 0x1a14`),
object offsets `+0x20..+0x34` are those six words in that order. Three independent checks:

- `0x60030303` is the address word read **live** on HD's raw units (`ship-sun-occlusion.md`, units 4 and 5;
  `track-reflect.md`, unit 3: "gamma nibble 0").
- `0x60030303 | 0x00700000 = 0x60730303` is the word read **live** on the Zone probe unit
  ([env-probes.md](env-probes.md), "gamma nibble 7"; [zone-hull-light.md](zone-hull-light.md), unit 4). The
  strip takes the same word by the same idiom.
- The `+0x34` mask `0xf0f0ffff` is exactly `FILTER`'s min (16-19) and mag (24-27) fields, and the stage
  textures' `0x1030000` (mag nearest, min nearest-mip-nearest) on the `...Nearest` clones is what the shader's
  name for them asks. The `+0x2c` patch `& 0xffffff8f | 0x20` in the same loop is `CONTROL0`'s max-aniso
  field (bits 4-6), 2x.

Nothing after the load clears the nibble: `Environment_UpdateStageBlend` (`0x003da540`) writes the texels
through `*(tex+0x10)`, not the words ([zone-effectsettings-loader.md](zone-effectsettings-loader.md)).
**This is the writer [renderer.md](renderer.md) left unlocated.** Its "Gamma is in the ADDRESS word" entry
reads `0x60730303` live on the sky faces and `0x60710101` on a mesh's diffuse unit, notes that
`Texture_BuildGcmRegisters` only ever writes nibble 0, and concludes that the `0x700000` is ORed on after the
builder "by code not located". `0x003d6dc8` is one such OR, on this object, at load. The older statements
that the engine has no per-texture gamma control ([renderer.md](renderer.md), "Per-texture sRGB/`GAMMA`
decode: settled negative", which searched `FILTER`; repeated in [magstrip-wake.md](magstrip-wake.md), "Still
open", and [env-probes.md](env-probes.md)) are the ones that entry already corrects; they do not contradict
this page.

Confidence **75** that the unit decodes the strip's RGB at the draw: the OR on this object is read
statically, the word layout is confirmed by live reads on other units, and a bind-time rebuild of the word is
ruled out only indirectly (live words elsewhere carry bits the builder never writes). The strip's own word
has not been read live at a Zone draw.

**3. Ours, before the wiring.** `crates/mesh/shaders/zone.wesl` `zone_glow` sampled `zone_vis_tex`
(`Rgba8Unorm`) raw into `lit_linear`, HD's linear sum, where every other HD gamma-7 texel is decoded. See
"Wired" below.

## What applying it does (measured, scratch build, reverted)

`decode(...)` (the target's own curve, `srgb_decode` on HD) around the `textureSample` in `zone_glow`, the three
Detonator pairs re-rendered with the original's recorded bands (`--zone-bands`), same binary otherwise.
Boxes: A `x 250..800, y 780..850`, B `x 1090..1230, y 780..1000`, mean RGB, 1920x1080:

| Pair | band 3 held | box A ours main / decode / original | box B ours main / decode / original | mean grey main to decode |
| --- | ---: | --- | --- | --- |
| `mines-ahead` | 0.514 | `(9.8, 1.5, 23.3)` / `(1.1, 0, 5.3)` / `(5.1, 0, 14.0)` | `(20.4, 3.3, 46.4)` / `(2.9, 0, 11.6)` / `(7.4, 0, 24.9)` | 18.58 to 18.02 |
| `cannon-volley` | 0.915 | `(17.6, 1.7, 37.3)` / `(2.3, 0, 14.1)` / `(6.2, 0, 18.3)` | `(31.2, 3.7, 64.1)` / `(4.8, 0, 25.2)` / `(8.5, 0, 28.2)` | 18.28 to 17.13 |
| `pad` | 0.584 | `(14.2, 1.8, 32.2)` / `(2.2, 0, 8.8)` / `(6.7, 0, 19.1)` | `(24.6, 3.8, 53.4)` / `(3.8, 0, 15.4)` / `(9.5, 0, 29.1)` | 12.74 to 11.79 |

Blue ratio ours over original: main 1.66 to 2.27; decode 0.38, 0.47, 0.77, 0.89, 0.46, 0.53 (geometric
mean 0.56). The decode moves ours' box blue by about 0.23 (23.3 to 5.3), not 0.5. The main column reproduces
`det-floor`'s "after" exactly. Whole-frame RMSE moves 0.198/0.192/0.149 to 0.202/0.195/0.150: slightly
worse on all three, while mean grey falls on all three.

**The level check does not settle it either way.** The original's box blue is 14.0, 18.3, 19.1 at held band 3
0.51, 0.92, 0.58 (B: 24.9, 28.2, 29.1): nearly flat, and `pad` above `mines-ahead` by more than band 3 says.
Decode-then-encode is near-linear in the band; ours with the decode spreads 5.3 to 14.1 (2.7x) over the same
bands, the raw main 23.3 to 37.3 (1.6x). Neither shape matches; the band-timing caveat of `det-pair-bands`
(the frame is 0 to 3 ticks older than the RAM read) is still live, and something band-independent adds to the
original's floor: its red is 5.1 to 9.5 where the strip's decoded red (`14 * L`) is under 1.

## Still unread: what makes the original's floor brighter than the decode predicts

1. **The window-depth mapping.** The near-plane fix assumed a GL window mapping (`1 - windowDepth = near / w`);
   with viewport z scale 1 and offset 0 the factor is `2 near / w` ([zone-shader.md](zone-shader.md), "The glow's
   depth factor", "with a D3D mapping the factor doubles"). Read `SET_VIEWPORT_SCALE`/`OFFSET` z
   (`0x0a30`/`0x0a20` lane 2) at the race's scene pass. A doubled linear glow is about 1.33 times brighter
   after the encode: it would close part, not all, of the gap.
2. **A band-independent floor term**: the original's floor red (5-9) is not the strip. The Track Fog lerp
   (`1.8 0.141 0.007`) is the named candidate; the Detonator arm's radial bloom is not (it draws only behind a craft out of
   the race, [zone-frame.md](zone-frame.md), "The radial pass").
3. **Live confirmation of this page**: one RPCS3 frame dump in a Detonator race (`scripts/rpcs3_draw_hook.py`),
   the track-surface draw's unit that binds `*(0x00c7dfb0 + 0x32b0)`'s offset: its `SET_TEXTURE_ADDRESS` should
   read `0x60730303`.

## Wired (2026-11-27, `det-vis-srgb`)

`oag_title::ZoneFrame::glow_vis_srgb` (HD alone, `true`) reaches the shader as `Zone::shape.z`.
`zone_glow` decodes the fetched texel with the piecewise sRGB curve before the `up * depth * E.w` gain, for
the authored linear sum only. The stand-in gamma sum (`plain` in `shade.wesl`) keeps the stored texel, since
`decode(plain)` decodes that whole sum. Tests: `zone_recolour::the_visualiser_strip_is_srgb_decoded_into_the_linear_sum_when_the_zone_says_so`
(a tint of 128 reads 84 decoded and 123 raw; red with the shader's `select` replaced by the raw texel) and
the `shape.z` assertion in `detonator_ground_truth::an_hd_detonator_race_loads_its_own_stage_table` (red with
HD's field `false`).

The six Detonator pairs, mean grey: `cannon-volley` 10.53 to **9.11**, `-hot` 7.70 to 4.82, `mines-ahead` 9.68 to
9.04, `pad` 4.44 to 3.24; the two end screens unchanged. Read at player size, the floor under `-hot` matches
the original's; on `cannon-volley`, `mines-ahead` and `pad` ours is now darker than the original's (box blue
0.31 to 0.53 of it), where before it was a lilac wash brighter than it. **The second factor stays open**
(the three items above); nothing was tuned toward it. A Zone race keeps its look: `zone-vineta-k-z5` with no
bands is pixel for pixel the same, and with made-up bands fed (`--zone-bands`, no original to compare) only the
near floor darkens.

## Cross-title

- **2048, its own `ZoneMode2048` tables: checked, differs.** They author no `EQ` tints, so ours' strip stays
  blank there ([detonator-look.md](detonator-look.md), "Cross-title"); there is nothing for a decode to act on.
- **2048, HD-lineage circuits: checked, applies, not wired.** Those circuits take HD's own title-wide tables
  ([zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md)), which do author the tints
  (on Vineta K, `eq_tints` reads `(1, 216, 255)` in Zone and `(171, 9, 255)` / `(14, 1, 55)` in Detonator), so
  ours lights their strip, still raw (`oag_2048` carries `zone_frame: None`). Whether 2048's engine decodes its
  strip is unread: next, its strip's GXM texture and gamma mode in `eboot.elf`.
- **Omega: checked, applies, not wired.** `FUN_015f4920` (`eboot.bin`, the load that names
  `data/Tex/zoneModeTrack0.gnf`) creates three `256 x 32` textures with GNM data format `0xfac90a` and
  zero-fills them (`0x8000` bytes each). That format word is `8_8_8_8` (surface `0x0a`), channel type `9`
  (sRGB), channels `X Y Z W`. Their descriptors are at `0x01e5f6b0` (stride `0x38`). The frame preparation
  (`Scene_PrepareFrame_q`, `0x015d080b`) stores `0x01e5f6b0 + i * 0x38` into the shader-constant slot
  `+0x8a8`, whose name `ShaderConstants_InitNameTable` fills with `zoneTexVis` (stored at `+0x8a0`,
  `0x015c4a6d`, out of `R11` loaded at `0x015c47de`). So Omega's texture unit decodes the strip too.
  Confidence 75 (static; the descriptor's sRGB is not read live). What Omega writes into the 32 rows, and its
  pixel program's use of them, are unread. Not wired: `oag_omega` carries `zone_frame: None`, so its HD-lineage
  circuits' strip is still raw in ours.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x005a6ce0` | `Image_Create` | 65 | stores width `+0x1c`, height `+0x20`, format `+0x24`, type `+0x28`; with its sixth argument set, pitch `width * bpp` rounded up to 64 into `+0x2c`; called `(0x100, 1, 8)`, `(0x200, 0x200, 8)`, `(0x20, 0x20, 8)` in `0x003d6dc8` |
| `0x005aa1d8` | `Texture_CreateForImage` | 70 | reads an image's width, height, type and format, allocates a `0x50` texture object, `Texture_BuildGcmRegisters` over it, copies `+0x14` into the `OFFSET` word `+0x20` and sets the location bits of the `FORMAT` word `+0x24` (1 or 2 by byte `+7`) |
