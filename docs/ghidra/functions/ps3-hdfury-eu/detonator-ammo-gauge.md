# Detonator: the cannon gauge on the craft's back

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below is under
`0x32d5e0`, on TOC `r2 = 0x008ad4d8` ([memory.md](memory.md)); constants resolved with
`scripts/ps3-toc.py resolve`, bodies read off `llvm-objdump -d --mattr=+altivec,+vsx`. The shader
microcode is `scripts/ps3-microcode.py fp-file|vp-file` over `DATA00.PSARC`'s
`/data/materials/ships/detonator_bulletgaugesegments.rcsmaterial`; parameter hashes are
`scripts/ps3-sho.py hash <name>`. Static reading and a live RPCS3 run, 2026-11-25 (`det-ammo-gauge`).

This page answers the maintainer's report from play: "The detonator craft shows the current ammo on
the back, ours does not." It replaces the 55 hypothesis on [detonator-hud.md](detonator-hud.md)
("PulseBullets").

## The answer

| Quantity | Value | Confidence |
| --- | --- | ---: |
| What draws it | the Detonator hull's own `detonator_bulletgaugesegments` material: 30 segment quads, two columns of 15 (`ship.rcsmodel`), texture `detonator_bullet_segments.gtf` | 90 (live frames) |
| Fill | `StandardBullets = count * (1/15)` (`0x3d888889`) | 92 (live, exact words) |
| Count | the clip `ship+0x164`; during a reload lockout (`ship+0x178 > 0`) `trunc((1 - ship+0x178 / ship+0x198) * 15)` | 85 (static; live red refill climbing in fifteenths) |
| Colour | `StandardColour = (0, 0.1, 1)` blue, ready; `(1, 0, 0)` red, locked | 92 (live) |
| Before the first update | full and red: the bind writes `StandardBullets = 1.0`, `StandardColour = (1, 0, 0, 1)` | 88 (live, the grid) |
| Gate | the update runs only while the craft's speed is over `10` (`Detonator_UpdateHud`'s top block, [detonator-mode.md](detonator-mode.md)); at or below it the block keeps its value | 75 (static, the caller) |
| Which segments | the lit share is the **front** `count` segments of each column; the rest draw black | 90 (live frames at 15, 12, 4, 0 and in a refill) |
| `PowerBullets`, `PowerColour` | `0` and `(1, 1, 1, 1)` for the whole race; no writer but the initialisers | 85 (static, one TOC slot; live constant) |

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0029d578` | `DetonatorGauge_SetBullets` | 88 |
| `0x0029d458` | `DetonatorGauge_BindHull` | 80 |
| `0x0029d658` | `DetonatorGauge_InitBlock` | 75 |
| `0x00ad7740` (data) | `g_DetonatorGaugeBlock` | 85 |
| `0x00107c28` | `Ship_UpdateDetonatorHullParams` | 62 |

## The block: `g_DetonatorGaugeBlock` (`0x00ad7740`)

A static global. The only TOC slot that holds its address is `0x008b2e60` (TOC `0x008ad4d8`,
displacement `+0x5988`): a byte search of the ELF for the word `0x00ad7740` finds that one slot,
and the disassembly loads `22920(r2)` in exactly three functions, the three below. Layout, as
`DetonatorGauge_InitBlock` writes the name ids (`~crc32`, through `0x00676ff8` and `not`) and
`DetonatorGauge_BindHull` binds the values:

| Offset | Holds | Name id | Initial |
| --- | --- | --- | --- |
| `+0x00` / `+0x04` | id / `float` | `PowerBullets` `0xfe6372ef` | `0` (bind) |
| `+0x08` / `+0x10` | id / `float4` | `PowerColour` `0xf782cc50` | `(1, 1, 1, 1)` (static init) |
| `+0x20` / `+0x24` | id / `float` | `StandardBullets` `0x6bdb824c` | `1.0` (bind) |
| `+0x28` / `+0x30` | id / `float4` | `StandardColour` `0xf038ed98` | `(1, 0, 0, 1)` (bind) |

### `DetonatorGauge_InitBlock` (`0x0029d658`)

The static-initialiser shape (`r3 == 1 && r4 == 0xffff`, reached from the thunk `0x0029d748`).
It stores `~crc32` of the four names (`PowerBullets` `0x0079bcc0`, `PowerColour` `0x0079bcd0`,
`StandardBullets` `0x0079bce0`, `StandardColour` `0x0079bcf0`, TOC `+0x599c..+0x59a8`) at `+0`,
`+8`, `+0x20`, `+0x28`, and `(1, 1, 1, 1)` at `+0x10` and `+0x30` (`stvx` from a stack vector of
`1.0f`). **75**: the shape is the compiler's, the stores are read; the thunk's caller is not.

### `DetonatorGauge_BindHull` (`0x0029d458`)

Called twice (`0x000648b0`, `0x00064e30`). Fetches `*(RaceManager_GetInstance()+0x13e8)+0x6adc`,
then its `+0x204c` material set (`0x00677008`), and binds the four ids to `+0x04`, `+0x10`, `+0x24`
and `+0x30` through `Material_BindInstanceParamPointer` (`0x00677018`, 85, [plasma.md](plasma.md):
**by pointer**, so the material's authored values never reach the draw). Then `+0x24 = 1.0`,
`+0x04 = 0`, and `+0x30 = (c, 0, 0, c)` with `c` the first word at `0x0079bc90` (`1.0`, TOC
`+0x598c`). **80**: read, and the grid read below holds every value.

The authored values the bind discards (`hd_material_dump`): `PowerBullets 1.0`, `PowerColour
(0.21952, 1, 1)`, `StandardBullets 1.0`, `StandardColour (0.88318, 0.0017, 0.0017)`.

### `DetonatorGauge_SetBullets` (`0x0029d578`), `(count, locked)`

```text
if locked == 0:  +0x30 = (0, b, a, a)       a = word0 of 0x0079bca0 = 1.0, b = word0 of 0x0079bcb0 = 0.1
else:            +0x30 = (a, 0, 0, a)       ; vsel lanes from vsldoi masks, 0x0029d58c-0x0029d638
+0x24 = float(extsw(count)) * k            k = 0x3d888889 (TOC +0x5998, 1/15)
```

So blue `(0, 0.1, 1, 1)` when ready and red `(1, 0, 0, 1)` when locked. **88.**

### The caller: `Detonator_UpdateHud` (`0x00067120`)

```text
s = rm+0x13e8 -> +0x5edc                            ; 0x00067160-0x00067170
locked = s+0x178 > 0.0                              ; TOC -0x68e8 = 0.0, 0x00067180-0x00067194
if !locked: DetonatorGauge_SetBullets(s+0x164, 0)  ; 0x000671b4
else:       DetonatorGauge_SetBullets(fctiwz((1.0 - s+0x178 / s+0x198) * 15.0), 1)
                                                    ; 0x00067438-0x00067484, 1.0 TOC -0x68ec, 15.0 TOC -0x67c4
```

`s+0x164` is the clip, `s+0x178` the lockout left and `s+0x198` the lockout the reload set
([detonator-cannon.md](detonator-cannon.md), "The reload"). The whole function sits in the speed
gate of [detonator-mode.md](detonator-mode.md) ("Top block"), so the gauge freezes while the craft
is at or under `10`.

## The segment program

Every variant of `detonator_bulletgaugesegments` (52, 10 fragment blocks) computes the same thing.
SVC0, vertex block `#1`/`#2` and fragment block `#2`:

```text
VP:  o[TC3].xy = v8.xy                                  ; uvA (attribute 0x1aefe524 = uvA.xy, uvB.xy)
     o[TC0].w  = v8.z ; o[TC1].w = 0.5 * c[208] + v8.w  ; uvB + (0, 0.5 * StandardBullets), c[208] = 0x6bdb824c
FP:  B = tex(unit 0, (TC0.w, TC1.w))
     A = tex(unit 0, TC3.xy + (0, 0.5 * PowerBullets))
     out = B * StandardColour * B.r + A * PowerColour * (1 - B.r)     ; MUL, MAD -H0.x, MAD
     out = (out * fog + fogColour * (1 - fog)) / 2                      ; the lit variants' fog, then /2
```

SVC1 (vertex `#8` `@0x4020`, fragment `#6` `@0x41e0`) packs `TC3 = (uvB.x, uvB.y + 0.5 *
StandardBullets, uvA.x, uvA.y)` and computes the same `B` and `A`. **No light term in any variant.**
The hull's gauge chunks author `uvA == uvB`: each segment spans `v` over one thirtieth
(`k/30 .. (k+1)/30`, `k = 0` the rearmost at `z = -5.64`, `k = 14` the front at `z = -3.97`).
`detonator_bullet_segments.gtf` (32x1024) is two flat halves, one white and one black. In the
original a full gauge (`StandardBullets = 1`) lights every segment and an empty one none, so the
shift `v + 0.5 * fill` carries the front `count` segments onto the white half, and `A` (unshifted)
reads the black half: an unlit segment is black. In ours the same shift shows the same front
segments (`oag_mesh::mesh::rcs::bullet_gauge`, frames below). **85** on the program, **90** on what it shows.

## Live (RPCS3, 2026-11-25)

`hd-detonator-talons-feisar-grid` restored, the race started with CROSS, the cannon held on
TRIANGLE, the block read through GDB at each stop and the frame grabbed (`grab_with_clock`):

| Stop | `StandardBullets` (word) | `StandardColour` | Frame |
| --- | --- | --- | --- |
| grid, before GO | `1.0` | `(1, 0, 0, 1)` | the craft not in view |
| lap clock 4.54 | `1.0` | `(0, 0.1, 1, 1)` | 15 blue per column |
| 4.86, after 0.5 s of fire | `0.80000007` (`0x3f4cccce`, 12) | blue | front 12 lit, rear 3 black |
| 5.70 | `0.26666668` (`0x3e888889`, 4) | blue | front 4 lit |
| 9.61, clip spent, trigger held | `0.0` | blue | none lit |
| 6.97 / 7.02 / 7.13 / 7.32 (second run, after the release) | `0`, `0.0667`, `0.2`, `0.4667` | `(1, 0, 0, 1)` | red, refilling from the front |
| 20.0 | `1.0` | blue | 15 lit |

`PowerBullets` read `0` and `PowerColour` `(1, 1, 1, 1)` at every stop; the four ids read back as
the `~crc32` above. The frame lags the RAM by a tick or two, as on every paused HD grab
([`docs/tools/compare-page.md`](../../../tools/compare-page.md)). Which model the bind lands on
(`*(rm+0x13e8)+0x6adc`, where `Detonator_UpdateHud` reads the ship at `+0x5edc` of the same base) is
settled by the frames: the gauge on the hull changes with the block.

## `Ship_BindDetonatorHullParams`'s block (`0x00992f40`): `Charge1`, `Charge2`

Read for the old D5 question on [detonator-hull.md](detonator-hull.md). The block's one TOC slot is
`0x008a9820` (TOC `0x008ad4d8`, `-0x3cb8`; a byte search finds no other), loaded by
`0x00107ac0`, `0x00107af8` (a static init: `+0 = (0, 0, 0, 1)`), `Ship_BindDetonatorHullParams` and
`Ship_UpdateDetonatorHullParams` (`0x00107c28`), plus two resets at `0x00107f20` and `0x00107fb0`.

`Ship_UpdateDetonatorHullParams(state, colour_src, f1 = dt, f2 = v)` keeps five smoothed channels
`{value, target, rate}` at `state+0x00/0x0c/0x18/0x24/0x30`. It takes the craft's colour
(`0x00677be8`) into the first three targets, then sets the last two targets from `v`: both `1.0`
when `v >= 1.0`; `+0x24` (Charge1) `1.0` and `+0x30` (Charge2) `0.0` when `0 < v < 1`; both
`0.0` otherwise (`0x00107ef8`-`0x00107f18`). Each channel moves toward its target by `rate * dt`
and clamps. The block then gets the colour in `+0`, Charge1 in `+0x10` and Charge2 in `+0x14`
(`0x00107dc0`-`0x00107e78`).

**`v` is the EMP charge.** Both callers sit in one craft update, behind `game+0xe0 == 14`
(`0x000eb404`). It reads `n = rm+0x2dd8` (the EMP charge, [detonator-emp.md](detonator-emp.md)) and
the table's `+0x1bc`/`+0x1c0` (`minNumPadsToCollectEMP` 4, `maxNumPadsToCollectEMP` 8,
`0x000eb414`-`0x000eb43c`). Below the minimum it branches to `0x000ed494`, which passes `v = 0.0`
(TOC `-0x49d8` = `0x00000000`). Otherwise it maps `n` through `0x002a3718` with the two bounds and
`1.0` (TOC `-0x49c4`), caps the result at `1.0` (`fsel`, `0x000eb498`-`0x000eb4a8`) and passes it
(`0x000eb4b0`). So **Charge1 lights once the EMP is ready and Charge2 once it is full**: the same
thresholds as the HUD's `EmpReady` and the `EMPFULL` cue. **62** on the channel law, **70** on `v`
(static; `0x002a3718`'s body is unread, and no capture ran). The fuselage's `rich_iridescent`
material and the strip and wing-edge glow it drives are not wired.

## Wired

`oag_title::Detonator::ammo_gauge` (`DetonatorAmmoGauge`: `rounds`, `ready`, `locked`, `bind`,
`speed_gate`) is `Some` on HD (`oag_hd::race::DETONATOR_AMMO_GAUGE`). `oag_raceplay::ammo_gauge`
steps the block per frame from the cannon (`Cannon::clip`, `lockout`, `lockout_total`) and the
player's speed, and the scene writes it into the player's hull glow table
(`PROGRAM_BULLET_GAUGE`). `oag_mesh::mesh::rcs::bullet_gauge` gives the material its entry by what
its programs declare. Tests: `oag_raceplay::ammo_gauge::tests` (the law, with the live words) and
`hd_rim_glow_ground_truth::the_detonator_hull_gauge_earns_its_segment_program` (the disc's hull).

**Colour.** A lit segment's mean over the blue-dominant pixels: the original
(`detonator-talons-cannon-volley`, `theirs.png`, 1920x1080, colour buffers written) `(1, 93, 219)`;
ours in a free race at 15 and at 6 `(2, 78, 224)` and `(2, 77, 220)`. Green reads 15 lower, cause
not read.

A seek in a replay viewer forgets the held gauge (`Race::restore_for_viewing`), so a seek to the
grid draws the bind state. A Restart Race loads the race again.

**A pose-held comparison pair shows the grid state.** `--pose-hold` puts ours' craft at rest, so
its speed never passes the gate and the gauge stays full and red where the original's moving craft
shows blue. It is the law doing what it should on a craft at rest, not a pair to read the gauge off.
The frames that show it are a free race (`--race --mode detonator`, an `--input-script` holding
SQUARE).

## 2048 and Omega

**Checked, applies, not wired.** Both ship the material: 2048's `data00.psarc` carries 17 entries
(`Data/materials/Ships/detonator_bulletgaugesegments.rcsmaterial`, `_1`, `_2`, the
`BulletGaugeSegmentsMask*` family under `Data/art/published/...`), Omega's archives 110. 2048's
executables (`PCSF00007`, `PCSA00015`, base and v1.04) carry the four strings `PowerBullets`,
`PowerColour`, `StandardBullets`, `StandardColour`; Omega's `eboot.bin` carries none as plain text.
Both draw through the PSP2/GNM programs (`oag_mesh::mesh::rcs::psp2`), whose gauge programs and
whose writer of the four constants are unread, so `Detonator::ammo_gauge` is `None` there.
