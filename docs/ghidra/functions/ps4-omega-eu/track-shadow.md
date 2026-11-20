# The track shadow `S`: three exponential cascades cast along the sun, and the sun term they remove

2026-11-20, `omega-shadow-map`. **Every address on this page is the 1.07 patch eboot's** (Ghidra program
`/omega/eboot-ps4-omega-eu-v107.bin`), not the base eboot the rest of this directory and its `names.tsv` use
([patch-v107-addresses](patch-v107-addresses.md)). None of these functions has been placed in the base program, so this page
renames nothing and adds **no `names.tsv` row**: a row with a 1.07 address would replay onto unrelated bytes in the base import.
The live reads and the knockouts are on the `wipeout_pc` native port (the 1.07 eboot run natively) through
`scripts/wipeout-pc-shadow-live.py`.

**What `S` is, in one paragraph.** Each frame the craft (and whatever else is flagged a caster) are drawn into three
orthographic depth maps looking along the circuit's **`Lighting.Sun direction`**, the cascades fitted to the casters' bounding
spheres within three camera-depth bands. The depth is turned into an exponential shadow map with exponent 80. A receiver
evaluates `S = min(1, pcf(shadowMap) * exp2(clamp(c0 + c1 z, -80, 0)))` and multiplies **only its sun terms** (the diffuse
`sun * LM.a * N.L` and the sun lobe) by it. Live, inside a craft's shadow the sun term is gone entirely: `S` reaches 0, with no
residual strength.

## The parts, with confidence

| Part | Law | Evidence | Confidence |
| --- | --- | --- | ---: |
| Cast axis | `D = normalize(Lighting.Sun direction)`, the same vector as `liveLighting0direction` | static: `EnvSettings_RegisterKeys` (1.07 `FUN_015eda20`) binds `0x01eb47d0` to `Lighting.Sun direction` (`015eda81`, string `0x0189140a`); `Scene_PrepareFrame` (1.07 `FUN_015f2d30`) normalises it at `015f7380`-`015f7456` and keeps it at `[RBP-0x290]` (`015f7487`), the light basis is built from it at `015f7d65`-`015f82dd`. `ShadowLight direction` (`0x01eb47e0`, string `0x0188d035`) is normalised beside it (`015f7388`) and is not the one stored. Live (Capital Reach, three boots): the light view's forward column `0x0221af68/78/88` reads `(0.1867, 0.5000, 0.8457)`, equal to `liveLighting0direction` `(0.18675, 0.5, 0.84565)` and to the pushed direction `0x0221ac20`; `shadowLightDirection` is `(-0.310, 0.706, -0.637)` | 90 |
| Light eye | the camera position plus `300 * D` (`0x01856140`, `015f80b6`), looking back along `-D` | static | 70 |
| Caster push | each caster's sphere centre moves by `-k * D`, `k = clamp(2.5 / max(abs(D.y), 1e-4), 4, 8)` (`0x01863284` 2.5, `0x01863280` 1e-4, `0x01863288` 8, `0x0186328c` 4; stored negated at `0x019a81d4`, `015f7dab`) | static; live `-5.0` on Capital Reach (`D.y = 0.5`) | 85 |
| Cascades | three, by the camera-view depth of a caster's sphere: `[0.5, 38.9]`, `[34.1, 110.9]`, `[101.3, 254.9]` (`0x01856110` and the immediates `0x42ca999a`, `0x437ee667` at `015f8076`/`015f8080`); each cascade's light-space rectangle is the min/max of the spheres that overlap its band (`FUN_01670960`, called at `015f863b` and `015f89d3`) | static; live, the six splits at `0x020e4c60` and three filled rectangles at `0x020e4c20`/`0x020e4c40` on Capital Reach | 80 |
| Casters | (1) the frame's model instance list (`0x01edc810` count, `0x01edb240` indices into 0x360-byte records at `0x01edf3e0`): a record with bit 1 of `+0x110` set, its sphere at `+0xf0` (radius plus 1.0), culled against six planes; (2) a second list under `*0x020e4268`, gated by `0x020e4340 == 1` and a flag on `*0x01a8ac98` (the gate read 0 on Capital Reach) | static; live, only craft-shaped shadows were seen. Which models carry the bit, and what the second list holds, is not read. `CastShipShadows` / `CastNoShipShadows` are read only inside `FUN_0162a390` (`0162e1f1`), another lane's function, and were not followed | 55 |
| Map encoding | exponential shadow map: the depth pass is converted with the exponent **80.0** (`0x42a00000`, written into the conversion pass's constants in `FUN_01670ba0`) against the near and far `0x020e4bb8`/`0x020e4bb4` (set by the shadow object's method `FUN_01670930`) | static | 70 |
| `shadowCoeffs` | built per frame at `015f9260`-`015f94a4` into a scratch block, its pointer at `0x019a8070`, bound to slot 104 (bank `+0x1050`, `015fd9ed`; name-table hash `0x0ae03ae7` at `+0x1040`). Rows 0 and 1: cascade `k` to `k + 1` remaps (`(c_k - c_k1) / w_k1`, `w_k / w_k1` per axis). Row 2: the two band fades, `(far_k - 0.25 * overlap, 2 / overlap)` (`37.7, 0.4167, 108.5, 0.2083`). Row 3: `(c0, c1, -80, 0)`, `c0` and `c1` scaled by the literals `-80.0` (`0x018632c0`) and `-160.0` (`0x018632c4`) | static; live on Capital Reach row 3 `(168.0, -799.2, -80, 0)`, on Sol `(-40.8, -799.2, -80, 0)` (`panel-displacement.md`); rows 2 equal on both | 85 |
| `shadowMatrix` | slot 103 (hash `0x6c1a1be7` at `+0x1018`, bound at `015fe14a`) | live value read, not decomposed | 60 |
| Receiver law | `S = min(1, pcf(shadowMap) * exp2(clamp(c0 + c1 z, -80, 0)))` on the sun diffuse and the sun lobe only | the tarmac program `@0xa764` (`omega-status.md`, "The tarmac roads' lit programs"); live knockouts below | 85 |
| How dark | inside a craft's shadow `S` reaches 0: the shadowed pixels equal the frame with every `S` forced to 0, to 0.85 to 1.04 of the sun term over five shadows on Metro Park | live, one boot | 80 |

## The knockouts, live

`Scene_PrepareFrame` is the only reader of the literals `-80.0` and `-160.0` at `0x018632c0`/`0x018632c4` (a whole-program operand
search finds one instruction each). Two writes to this run's own port process, each undone after its frame:

- **`S = 1`**: both literals written `0`: row 3 becomes `(0, 0, 0, 0)` (read back from the bound bank while the write stood), so
  `S = min(1, pcf(map)) = 1` (the map holds `exp(80 d) >= 1`). Every map shadow is removed.
- **`S = 0`**: `-160.0` written `-1.6e6`, so `c1` reads `-7.99e6` and the exponent sits at its floor: every receiver loses its
  sun term. Recovery after the write is undone is within 0.24 s (the frame grabbed after it).

The port maps the eboot privately: after all four boots `eboot.bin` still hashes `3a52e910...f644c`, the patch's own.

| Circuit, region (1920x1080, mean grey) | shadow on | `S = 1` | `S = 0` |
| --- | ---: | ---: | ---: |
| Capital Reach, sunlit streak `400x100+400+420` | 154.2 | 152.5 | 95.6 |
| Capital Reach, whole frame | 95.6 | 95.4 | 95.0 |
| Metro Park, five craft shadows (masked where `S = 1` brightens by 15 or more) | 62.7 to 159.9 | 109.6 to 239.8 | 60.9 to 146.0 |
| Metro Park, grass left `400x300+0+700` | 113.4 | 114.2 | 75.1 |
| Metro Park, hull top `200x140+860+560` | 106.7 | 107.5 | 90.4 |
| Metro Park, whole frame | 116.4 | 116.9 | 101.2 |

What this settles:

1. **The tarmac binds the shadow-map permutation** (`@0xa764`): forcing `S = 0` takes Capital Reach's sunlit streaks from 154 to
   96 grey. `omega-status.md`'s confidence-70 assumption is now measured (confidence 90).
2. **Capital Reach's grid shows no map shadow**: `S = 1` changes no region beyond frame-to-frame noise in two boots (three rounds
   in the first). The sun there reaches the road only in the baked `LM.a` streaks, and no craft stands in one. Capital Reach's
   whole-frame gap is therefore not `S` (its whole sun term is 0.6 grey of the frame).
3. **Metro Park's grid shows the mechanism**: every craft on the sunlit brick and grass casts a shadow that `S = 1` removes, and
   inside it the frame equals the `S = 0` frame (the sun term is removed whole).
4. **The hull receives `S` too** (hull top 106.7 to 90.4 under `S = 0`), which is the `shadow` weight of
   [hull-tone.md](hull-tone.md)'s `(Ks + (K - Ks) * shadow)`. The grass (`grassmudrock_blend_...`) receives it as well.

## 2048: checked, differs

2048 casts its craft map along `Lighting.Ship shadow direction` into one map fitted to the frustum out to 60 units
([ship-shadows.md](../vita-2048-eu-v104/ship-shadows.md)). Omega casts along `Lighting.Sun direction` into three exponential
cascades out to 254.9 and has neither of 2048's debug gates. The receiver's form (a map term on the sun) is the shared lineage;
the axis, the fit and the encoding differ.

## Open

- Which models set bit 1 of `+0x110`, and what the second caster list (`*0x020e4268`) holds.
- The `pcf` taps and the cascade selection inside the receiver (`@0xa764`'s `@0x7b8`-`@0x964` per
  [panel-displacement.md](panel-displacement.md)).
- Placing `FUN_015f2d30`, `FUN_01670960`, `FUN_01670ba0` and `FUN_01670930` in the base eboot, after which they can take
  `names.tsv` rows (`Scene_PrepareFrame` is already mapped: base `0x015c63a0`).
