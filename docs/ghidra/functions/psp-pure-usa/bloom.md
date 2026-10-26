# Pure's bloom: Pulse's four-pass chain over Pulse's stencil mask

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Lane `pure-sign-glow`, 2026-10-26: the maintainer's report that
Modesto Heights' neon signs glow in the original and read crisp in ours. Evidence: a recorded GE
list of the compare pair's own state on PPSSPP's software renderer (`scripts/psp-ge-dump.py`),
replays of it with PRIMs removed, EDRAM readouts of the replayed frame, and the static read below.
Pulse's page is [`../psp-pulse-usa/bloom.md`](../psp-pulse-usa/bloom.md); every number here is
checked against it.

## The halo is the bloom

Replaying the Modesto list (430 PRIMs) with **one PRIM changed**: the composite (PRIM 403,
`FIXA 0xafafaf` / `FIXB 0xffffff`, a 2 x 2 sprite grid over the frame) made to add nothing. The
signs go crisp and the frame equals ours before this lane. The sign box (x 180-280, y 60-105,
480 x 272 frame) averages luma:

| Frame | Mean luma (Rec. 601) |
| --- | ---: |
| original (`theirs.png`) | 111.1 |
| ours before | 75.5 |
| ours after | 111.3 |
| original replayed with PRIM 403 made additive-zero | crisp signs, no halo (looks as "ours before") |

Removing every additive scenery PRIM (382 to 396) as well takes the signs' faces off, so the faces
are additive batches and the halo is their bloom; removing the composite alone keeps the faces.

## The chain in the list

All four Pure circuits' lists (Vineta K, Chenghou, Blue Ridge, Modesto) carry the same chain, once,
after the scene and before the first HUD sprite (Modesto: PRIMs 398 to 403, HUD from 404):

| PRIM | What | State |
| ---: | --- | --- |
| 398 | bright pass: 8 tiles (16 vertices), texture = the frame as a 512-stride 32-bit buffer, into `0x110000` stride 256 | `BLENDMODE 0xa2`: `src * SRC_ALPHA + FIX 0` |
| 399, 401 | clear of the target: one untextured sprite, colour 0 | blend off |
| 400, 402 | blur X then blur Y: 1,320 vertices each (4 rows x 15 columns x 11 taps x 2), the taps' vertex colours `0x14 0x1e 0x28 0x32 0x40 0x40 0x40 0x32 0x28 0x1e 0x14` | `FIX 0xffffff` / `FIX 0xffffff` |
| 403 | composite: 4 sprites, 256 x 136 source stretched 2:1 over the frame | `FIXA 0xafafaf`, `FIXB 0xffffff`, `MASKALPHA 0xff` |

The 11 weights, 175 (`0xaf`), the 64 x 272 tile and the 240 x 136 buffer are Pulse's constants
(`0x08ab2344`, `0x08ab2348` there). The bright pass in Pure's list reads 512 columns of the
frame buffer's stride into 256 and the composite writes 512 back (the last 64-wide tile is not
clipped to the 480 visible); the 2:1 ratio and the registration are Pulse's, so nothing differs
over the visible 480. The blur covers 240 x 136 in both (`FUN_088630c4`, below).

## The mask is Pulse's stencil stamp

The list's stencil state, by batch (`STENCILTEST` func `ALWAYS`, ref in bits 8 to 15,
`STENCILOP` `KEEP, KEEP, REPLACE` on 290 of the 430 PRIMs):

- ordinary opaque and cutout batches: ref `4`, as Pulse's rule 1 (`docs/rendering/glow-mask.md`);
- the `_GLOW` batches: ref = their texture's glow byte, e.g. `0x9e` (the wall decal), `0xb7`
  (`Neon_fantasy_ADD_GLOW`), `0x87` (`col_arrows1_GLOW_ADD`), `0xf8`, `0xaa`, `0xff`: Pulse's rule 2;
- transparent batches without the glow bits: the stencil test is off, so alpha is kept: rule 3;
- PRIMs 265 and 266 are the craft's shadow volume (colour masked, `STENCILOP` `INCR` and `DECR`
  on the depth-fail word `0x400`/`0x500`) and 267 and 268 a quad with ref `4` and `REPLACE` on all
  three outcomes (`STENCILOP 0x20202`): the same pair Pulse's list shows after the shadow volume
  (`docs/rendering/glow-mask.md`, "The hull overlay's mask is wiped"); not read further here.

The mask read out of EDRAM after replaying the list (`0x04000000`, 480 x 272 at stride 512) holds
`4` over 125,374 pixels, `0x9e` over 3,452, `0xff` over 707, `0` over 665, `0xb7` over 346 and
`0xaa` over 16. Ours (`OAG_DUMP_GLOW_MASK`, same pose) over the scene rows (8 to 229): `0x9e` over
3,445, `0xb7` over 318, and every other difference inside the HUD's boxes, which the original keeps
at `4` by masking alpha and ours does not (the bloom reads the scene before the HUD, so it does
not reach the glow). Chenghou's replayed mask matches the same way (the extra `0xff` pixels are the
HUD's).

The sun's lens flare (`docs/rendering/lens-flare.md`, Vineta K and Chenghou) draws with the stencil
test off and `MASKALPHA 0xff`: it stamps nothing, and ours' flare pipeline writes
`ColorWrites::COLOR` only.

## Order against the HUD

On Pure the composite (PRIM 403) is drawn **before** the first HUD sprite (404 to 429; their
`MASKALPHA` is `0xff`). Pulse's queue key puts the composite after every HUD widget
(`../psp-pulse-usa/bloom.md`, "The bloom draws over the HUD"). Ours composites over the HUD for
Pulse and under it for Pure: `Looks::bloom_under_hud`.

## Functions

| Address | Name | Confidence | What it does |
| --- | --- | ---: | --- |
| `0x0886277c` | `Bloom_Construct` | 85 | the singleton constructor: stores itself at `0x00054430`, `+0x4c = 0.01f`, the two blur vertex buffers at EDRAM `+0x16f080` and `+0x174340`, four display lists (sizes `0x9c`, `0x78`, `0x68`, `0x94`) and the `0x294` and `0xb4` sprite buffers, then runs the four list builders. Pulse's `Bloom_Construct` (`0x089072d4`) field for field |
| `0x08862d50` | `Bloom_BuildBrightPassList` | 85 | disables stencil, depth, alpha test, cull and lighting, enables texture and blend, `Gu_BlendFunc(ADD, SRC_ALPHA, FIX 0)`, draws tiled sprites with source `480 x 272` (`0x43f00000`, `0x43880000`) and destination `240 x 136` (`0x43700000`, `0x43080000`) |
| `0x08862e60` | `Bloom_BuildBlurHorizontalList` | 85 | binds the first scratch buffer, `Gu_BlendFunc(ADD, FIX, FIX)` both `0xffffffff`, calls `Bloom_EmitBlurTaps(self, 0)` |
| `0x08862efc` | `Bloom_BuildBlurVerticalList` | 85 | same on the second scratch buffer, `Bloom_EmitBlurTaps(self, 1)` |
| `0x08862f7c` | `Bloom_BuildCompositeList` | 85 | depth mask off, `Gu_BlendFunc(ADD, FIX strength*0x010101, FIX 0xffffff)`, source `240 x 136` (`0x43700000`, `0x43080000`) onto `480 x 272` |
| `0x088630c4` | `Bloom_EmitBlurTaps` | 85 | clears the target with one untextured sprite `(0,0)-(0xf0,0x88)`, then loops `v` by `0x22` to `0x88` and `u` by `0x10` to `0xf0` over 11 taps with offset `i - 5` on x (`param_2 == 0`) or y (`param_2 == 1`) and the 11 weight bytes as vertex colours: Pulse's `Bloom_EmitBlurTaps` (`0x08907c40`) |

| Address | Kind | Name | Confidence |
| --- | --- | --- | ---: |
| `0x08a8e898` | data | `g_bloom_composite_strength` | 90 |
| `0x08a8e89c` | data | `g_bloom_blur_weights` | 92 |

`g_bloom_blur_weights` is the 11 bytes `14 1e 28 32 40 40 40 32 28 1e 14` (`search_byte_patterns`;
one reference, `Bloom_EmitBlurTaps` at `0x088631b8`), `g_bloom_composite_strength` the byte `0xaf` in
the word in front of it. The bloom's draw entry (the render queue key) was not read: the order
against the HUD is read off the list, not the queue.

## Applies to

- **Pulse:** the same chain; its own page.
- **Other Pure circuits:** the chain is in all four lists (PRIMs 585 to 589 on Vineta K, 669 to 673
  on Chenghou, 650 to 654 on Blue Ridge).
- **2048 / Omega / HD:** not applicable: HD and Omega run their own chains (`oag_post::hd_bloom`),
  2048 has none read. Checked: the weights and strength are Pulse's, not theirs; nothing carries over.
