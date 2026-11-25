# Detonator: how a stage lays its mines and its bomb

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function here declares TOC
`r2 = 0x008ad4d8` in its own OPD entry (`scripts/ps3-toc.py toc`: "exact"), so the TOC defect
[memory.md](memory.md) describes does not apply. Static reading only, 2026-11-22: nothing here
ran. Ghidra cuts `0x000654e0` short on bad instruction data, so every address below was read off
`llvm-objdump -d --mattr=+altivec,+vsx` over the real extent; `lvlx` (Cell, primary 31 / XO 519,
which llvm prints as `<unknown>`) was decoded by hand at `0x00065038`, `0x00065108`,
`0x00065e90` and `0x00066b64`. Read [detonator-mode.md](detonator-mode.md) first: it found the
stage counter `rm+0x2e10` and the call into this page's placer.

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x000654e0` | `Detonator_PlaceStageObjects` | 80 |
| `0x00064f70` | `Detonator_SpawnObject` | 78 |
| `0x00063510` | `Detonator_LoadPatternFile` | 85 |
| `0x00063dc0` | `Detonator_LoadGamePlayValues` | 85 |
| `0x00066c60` | `Detonator_PlaceOpeningStage` | 72 |
| `0x008c1478` | `g_DetonatorGamePlayValues` (data) | 80 |

`rm` below is the `SPDetonator` race manager (`SPDetonator_RaceManager.cpp`, constructors
`0x00064470` and `0x000649f0` by `scripts/ps3-toc.py map`).

## Where the data comes from: two XML files, one for every track

Both `SPDetonator` constructors run, at `0x00064868`-`0x0006489c` (and the twin at
`0x00064de8`-`0x00064e1c`):

```text
sprintf(sp+0x70, "/data/xml/DetonatorLayouts/%s.xml", <0x009371a8>)   ; TOC -0x6840, -0x683c
Detonator_LoadPatternFile(rm, "/data/xml/DetonatorLayouts/PatternFile.xml", 1)      ; TOC -0x6838
Detonator_LoadGamePlayValues(rm, "/data/xml/DetonatorLayouts/GamePlayValues.xml", 1) ; TOC -0x6834
```

**The per-track path is formatted and never read**: `sp+0x70` is not passed to either loader
and nothing else in the constructor loads it (`0x0006486c` is its only reference up to the
`blr` at `0x00064928`). Confidence **75**. No HD archive ships any other file in that directory:
`DATA00.PSARC` carries `/data/xml/detonatorlayouts/gameplayvalues.xml` (742 bytes) and
`/data/xml/detonatorlayouts/patternfile.xml` (16,108 bytes) and nothing else, and `DATA01` to
`DATA06` carry none. So **the patterns are one global file, the same on every track**; nothing
is per-track authored, nothing is in the `.vex`. Confidence **85**.

The `true` third argument picks the read-the-whole-file path (`0x00063d0c`: `fopen(path, "r")`,
size, read into a buffer, parse from memory); the attribute compare is `strcasecmp`
(`0x00676328`, [ai-stats.md](ai-stats.md)), integer and float parsers are `0x00677248` and
`0x00677258`.

### `PatternFile.xml`: groups of clusters (`Detonator_LoadPatternFile`, `0x00063510`)

The file is a sequence of `<Pattern stageToAppearFrom="n">` elements, each holding
`<Cluster1>`..`<ClusterN>` children. One `Pattern` is one *group*, stored at
`rm + 0x2e30 + g*0xd60`, the count of groups at `rm+0x6de24` (cleared at `0x000635bc`,
incremented per `Pattern` at `0x00063b8c`). Room for 127 groups and 49 clusters per group
((`0x6de24-0x2e30`)/`0xd60`, (`0xd50-8`)/`0x44`); no bound check was seen in the loader.

| Group field | Attribute | Evidence |
| --- | --- | --- |
| `+0x004` int | cluster count (incremented per child) | `0x000637a8`-`0x000637b0` |
| `+0xd50` int | `stageToAppearFrom`, default `1` | default `0x00063614`-`0x00063630`; parse `0x00063cc8`-`0x00063d00` |
| `+0x008 + k*0x44` | cluster `k`, 68 bytes, table below | the stores at `0x00063670`-`0x00063790` |

| Cluster field | Attribute | Type | Evidence (parse, store) |
| --- | --- | --- | --- |
| `+0x00` | `numOfObjects` | int | `0x00063834`-`0x0006385c`, `0x00063698` |
| `+0x04` | `distanceBetweenObjects` | int | `0x00063a9c`-`0x00063ac4`, `0x000636b8` |
| `+0x08` | `type` | int | `0x00063ad4`-`0x00063afc`, `0x0006372c` |
| `+0x0c` | `percentageAcrossTrack` | float parsed, **stored truncated to int** (`fctiwz`) | `0x00063b0c`-`0x00063b34`, `0x00063680`/`0x000636d8` |
| `+0x10` | `angleUp` | float | `0x00063b44`-`0x00063b6c`, `0x00063728` |
| `+0x14` | `angleRight` | float | `0x00063c44`-`0x00063c6c`, `0x00063720` |
| `+0x18 + 4*(i-1)` | `object<i>Type`, `i` = 1..10 | int | `sprintf("object%iType", i)` loop `0x00063864`-`0x00063a8c`, copy `0x00063730`-`0x00063790` |
| `+0x40` | `distanceFromOriginalPosition` | int | `0x00063c7c`-`0x00063ca4`, `0x00063714` |

The ten `object<i>Type` words are cleared to `0` before each cluster (`0x000637d0`-`0x000637f4`).
Field meanings in the names column are the file's; what the placer does with each is below.
Confidence **85** for the layout (every store read at instruction level against its attribute
string).

**The disc's file** (identical bytes on HD, 2048 and Omega after line endings, see the
cross-check): 18 patterns, 79 clusters. `stageToAppearFrom` is `1` on five patterns and one
each of `2`..`10`, `13`, `14`, two on `11`. Every cluster is `type="0"`, every `object<i>Type`
is `0`, 76 clusters have `numOfObjects="1"` (one `2`, two `3`), `distanceBetweenObjects` is `3`
on 71 (`0`, `1`, `2`, `5` on the rest), `percentageAcrossTrack` runs `-5`..`5`,
`distanceFromOriginalPosition` `0`..`8`, `angleRight` is always `0` and `angleUp` `0` except
one `-0.3`.

### `GamePlayValues.xml`: the stage counts (`Detonator_LoadGamePlayValues`, `0x00063dc0`)

One element, `<GamePlayValues ...>`. Its attributes land in the static block
`g_DetonatorGamePlayValues` (`0x008c1478`, TOC `-0x68a0`) and in a flag block at `*(TOC -0x690c)`
(`0x0097ce84`, `.bss`):

| Attribute | Lands at | Type | ELF default | Disc value | Evidence |
| --- | --- | --- | --- | --- | --- |
| `RemoveMinesEachStage` | flags `+4` | bool | (bss) `0` | `0` | `0x00063ecc`-`0x00063ef4` |
| `DoMusicResponsePreviousStageMines` | flags `+5` | bool | `0` | `0` | `0x00063f84`-`0x00063fac` |
| `DistanceOffsetBetweenMines` | `+0x00` | float | `3.5` | `3.5` | `0x00063fbc`-`0x00063fe0` |
| `NumStepsToIgnoreAfterJump` | `+0x04` | int | `20` | `20` | `0x000640e0`, `0x000641f4`-`0x00064204` |
| `MineCountsRepresentMaxTotalToAppear` | `+0x08` | bool (byte) | `1` | `1` | `0x000640fc`, `0x00064210`-`0x00064220` |
| `EnergyRechargePerStage` | `+0x0c` | int | `5` | `10` | `0x00064118`-`0x00064140` |
| `PointsForChainReactionMines` | `+0x10` | int | `100` | `100` | `0x0006426c`-`0x00064294` |
| `MineCountStage<i>`, `i` = 1..14 | `+0x14 + 4*(i-1)` | int | `70, 80, ... 200` | see below | `sprintf("MineCountStage%i")` loop `0x00063f00`-`0x00063f74` (`cmpwi cr6, i, 14` at `0x00063f48`) |
| `BombChainReactionDistance` | a stack local, then dropped | float | - | `5000` | `0x00064238`-`0x0006425c`; `sp+0x70` has no other reference in the function |
| `CannonLockoutTimeFromManualReload` | `0x008c1838` | float | - | `0.75` | `0x00063ff0`-`0x00064014` |
| `CannonLockoutTimeRunOutBullets` | `0x008c183c` | float | - | `0.75` | `0x00064024`-`0x00064048` |

**The disc's `MineCountStage1`..`14`: `80, 90, 95, 100, 105, 110, 120, 130, 140, 150, 170, 180,
190, 200`.** The ELF's own defaults (`70`..`200` in steps of `10`, static `.data` at
`0x008c148c`) are overwritten by the file at every `SPDetonator` construction, so they are a
fallback, not the law: play the file. Confidence **88** for the mapping.
`EnergyRechargePerStage` is spent by `Detonator_UpdateRace` at `0x00067c48`-`0x00067c6c` (read
`+0x0c`, add to a craft value through `0x000d1940`/`0x000d17e0`) on every stage step: a side
finding, unread past that, confidence 60.

## When it runs

- **Race start, once**: `Detonator_PlaceOpeningStage` (`0x00066c60`) tests the latch byte
  `rm+0x2e1c`; if clear it sets it and tail-calls `Detonator_PlaceStageObjects(rm, stage - 1)`
  (`0x00066cd0`-`0x00066cf4`) with `stage = rm+0x2e10 = 1`, so `oldStage = 0`. The same block is
  inlined in `Detonator_UpdateRace` at `0x00068410`-`0x00068430`. Confidence **78**.
- **Every stage step**: `Detonator_UpdateRace` stores `new = old + 1` to `rm+0x2e10` and calls
  the placer with the *old* stage while `new <= 14` (`0x00067c74`-`0x00067c84`, then
  `0x000680d8`-`0x000680e0`). So the placer runs for stages 1 (start) through 14, and
  `table[oldStage]` (`+0x14 + oldStage*4`) is always `MineCountStage<new stage>`.
  Confidence **85**.

## `Detonator_PlaceStageObjects(rm, oldStage)` (`0x000654e0`)

`r19 = rm`, `r18 = oldStage`. On entry `rm+0x6de48` (objects placed by this call) is cleared
(`0x000655a0`).

### 0. Housekeeping (unread in depth)

- `flags+4` (`RemoveMinesEachStage`) set: `0x0012ae38(*g_RaceWorld)` (`0x00065ac4`). Disc: `0`.
- `table+0x54` byte (ELF default `1`, no attribute writes it): `0x0012ae10(*g_RaceWorld)`
  (`0x000655a8`-`0x000655b4`, `0x00065ae4`).
- `flags+5` (`DoMusicResponsePreviousStageMines`) **clear**: `0x0011a360(mineManager)`
  (`0x00065b00`-`0x00065b0c`), which calls virtual slot `+0x3c` on every live mine
  (`0x0011a370`-`0x0011a3c8`). Disc: `0`, so it runs on every call. What slot `+0x3c` does is
  unread.

Read 2026-11-22 ([detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md)):
`0x0012ae10` is `BombManager_DetonateAll` (every live bomb triggered, its lane freed: the one-bomb
rule), `0x0012ae38` adds `MineManager_RemoveAll`, and slot `+0x3c` darkens a `DetonatorMine`
(`+0x160 = 0`): last stage's mines turn dark.

`*g_RaceWorld` is `*(TOC -0x6874)` = `*0x00994198`; its `+0x60` is the mine manager and `+0x5c`
the bomb manager (below).

### 1. Survey the lap, once (`oldStage == 0`, `0x00065b38`-`0x00066134`)

Builds a per-race array of *track entries*, one per spline section of the lap, at
`*(rm+0x6de50)`, count `rm+0x6de4c`, 48 bytes each.

- **Walk**: start from the player craft's four-word track cursor (`craft+0x7820..+0x782c`,
  read as `craft+0x6ff0+0x830..0x83c` at `0x00065b38`-`0x00065b7c`; the cursor
  [projectile-launch-normal.md](projectile-launch-normal.md) names), seek with
  `0x000a5318(.., -1)`, then step with `0x000a54b8` and count until the cursor's section pointer
  equals the start's again (`0x00065bac`-`0x00065c00`; the count starts at `1`, `0x00065ba8`).
  Allocate `count*48` (`0x00065c40`), clear it, walk again storing each cursor in the entry
  (`+0x0c..+0x18`, `0x00065cb8`-`0x00065d98`). When the step returns `1` (a route fork) the
  block `0x0006675c`-`0x00066b38` walks the fork with selectors `1`/`2`/`3` and the tests
  `0x000a96d8`, `0x000a5278`, `0x000a5290`; **fork handling is unread** (open: nothing is laid on an alternate path in ours; the EMP wave's fork already clears a mine put there, `detonator-emp.md`, "The fork, wired"). Confidence for a
  single-route lap: **72**.
- **Per entry** (`0x00065e64`-`0x00066078`), with `sec = entry+0x18` (the cursor's section
  pointer) whose vectors `+0x00` (position), `+0x10`, `+0x20`, `+0x30` and floats `+0x44`,
  `+0x48` are read:
  - `up = -sec.v20` (`v28`, `0x00065eac`). The probes below run from `pos + 8*up` to
    `pos - 64*up`, and the mine's matrix row 1 is `up`, so `sec.v20` reads as the section's
    down axis. Confidence 65.
  - Two horizontal `Collision_SweepSegment` (`0x0007be58`) probes from `start = pos + up`, to
    `start + sec.v30 * (2*sec+0x48)` and to `start - sec.v30 * (2*sec+0x44)` (`0x00065ec4`-
    `0x00065f38`; the `2.0` is TOC `-0x6820`). Half-widths `wL = sec+0x44`, `wR = sec+0x48`;
    on a return code of `0` or `5` a width is clamped to the hit distance
    (`0x00066138`-`0x00066234`). **As the instructions read, the first probe's code gates the
    clamp of the second probe's width and the reverse**; confidence 60 for which side clamps.
  - `W = 0.9 * (wL' + wR')` (`0.9` is TOC `-0x6818`, `0x00065fa0`).
  - **`lanes = trunc(W / 3.5)`, made odd by subtracting 1 when even** (`fdivs` by `table+0`,
    `fctiwz`, `clrlwi/xori/sub` at `0x00065fac`-`0x00065ff0`), stored as a byte at `entry+0`.
  - **Centre** `entry+0x20 = start - sec.v30*wL' + sec.v30*(0.5*W)` (`0x00065fa8`-`0x0006600c`;
    `0.5` is TOC `-0x6870`). So the lane fan is centred 45 % of the usable width in from the
    `-v30` edge. Confidence 70.
  - A lane array of `lanes` ints at `entry+4`, every lane `3` (= free) (`0x00066010`-
    `0x00066054`). **`lanes > 20` marks the entry unusable** (`entry+8 = 0`, `0x00066058`-
    `0x00066060`).
  - A vertical probe at the centre (`0x00066238`-`0x000662a0`, `8.0` and `64.0` at TOC
    `-0x6814`/`-0x6810`). Code `4`, `127` or `2` (`0x000662a8`-`0x000662bc`; on Pulse `127` is
    "no hit", `collision-cage.md`) marks this entry **and the next `NumStepsToIgnoreAfterJump - 1`
    = 19** unusable (`r24` count against `table+4`, `0x000662cc`-`0x000662fc`, `0x00066b3c`).
    That is the jump gap.
  - Otherwise `entry+8 = 1` (`0x00066b70`) and **each lane gets its own vertical probe** from
    `centre + v30*3.5*j + 8*up` to `... - 64*up`, `j = -lanes/2 .. lanes/2`
    (`0x00066b54`-`0x00066c5c`); a code of `0`, `2`, `4`, `5` or `127` sets that lane to `0`
    (blocked).
- **The start zone**: every entry whose `sec+0x00` lies within **150.0** (`0x0077d320`, TOC
  `-0x680c`) of entry 0's is marked unusable (`0x00066090`-`0x00066130`).

Confidence for the survey as a whole **70**: the instruction reading is solid, the meaning of the
collision return codes is not.

Entry layout (48 bytes): `+0x00` u8 lane count, `+0x04` lane array (int per lane: `3` free,
`0` blocked or a mine, `1` a bomb), `+0x08` u8 usable, `+0x0c..+0x18` the track cursor,
`+0x20` vec4 centre.

### 2. How many pattern groups (`0x000655e8`-`0x000656c4`)

```text
eligible(g) := group[g].stageToAppearFrom - 1 <= oldStage          ; 0x0006560c-0x00065618
n_elig      := number of eligible groups
objs        := sum over eligible groups, over their clusters, of numOfObjects  ; 0x00065648-0x00065658
avg         := objs / n_elig                         (signed integer divide, 0x00065674)
want        := table.MineCountStage[oldStage]        (= the new stage's count)
if table.MineCountsRepresentMaxTotalToAppear:        ; disc: true, 0x00065678-0x000656ac
    want -= mineManager.live                         ; *(*g_RaceWorld+0x60)+0x398
else:
    want -= rm+0x6de48                               ; placed so far by this call (0)
n_groups    := want / avg                            (signed, 0x000656ac / 0x00065abc)
```

So with the disc's flag the stage count is **a target total including the mines still alive**
from earlier stages, matching the manual's "fresh mines (up to a stage maximum)". Confidence **82**.

### 3. Lay the groups (`0x000656c0`-`0x00065830`)

```text
total := entry count;  chunk := total / n_groups (unsigned, 0x000656c4);  base := chunk
repeat n_groups times, while total - base >= chunk:
    do g := Libc_Rand() % group_count  until eligible(g)            ; 0x000656d8-0x00065710
    for each cluster c of g, in order:
        pos := base + c.distanceFromOriginalPosition
        if pos > total - 1 (unsigned): stop this group               ; 0x00065764, 0x00065790
        if c.type == 0:                                              ; 0x000657a0-0x000657a8
            Detonator_SpawnObject(rm, craft, pos, c.numOfObjects,
                                  (float)c.distanceBetweenObjects,
                                  c.percentageAcrossTrack, &c.object1Type)   ; 0x000657ac-0x00065804
    base := pos_of_last_cluster_tried + chunk                        ; 0x00065820
```

`craft` is `*(RaceManager_GetInstance()+0x13e8)`, the local player. A group with no clusters
leaves `pos = base`. Confidence **82**.

### 4. Top up with single mines (`0x00065834`-`0x00065934`)

```text
left := MineCountStage[oldStage] - mineManager.live      (flag true; else - rm+0x6de48)
while left > 0:
    do
        e    := Libc_Rand() % total          (unsigned)
        lane := Libc_Rand() % e.lanes - e.lanes/2
    until e.usable and 0 <= lane < e.lanes and e.laneArray[lane] == 3   ; 0x000658b4-0x000658fc
    Detonator_SpawnObject(rm, craft, e, 1, 5.0, lane, &{0})              ; 0x00065900-0x0006592c
    left -= 1
```

`5.0` is TOC `-0x6800`. **A shipped quirk, kept on all three titles**: the retry tests
`laneArray[lane]`, but `Detonator_SpawnObject` places at `laneArray[lanes/2 + lane]`, and
`lane < 0` is rejected, so **top-up mines only ever land on the centre lane or the `+v30` side**
(offsets `0..lanes/2` from the centre). When the slot `SpawnObject` checks is taken, nothing is
placed and `left` still drops. Confidence **80**.

### 5. One bomb (`0x00065938`-`0x000659e0`)

The same draw, without the `lane >= 0` test (`0x00065994`-`0x000659b0`: only `e.usable` and
`laneArray[lane] == 3`, so a negative `lane` reads before the array), then
`Detonator_SpawnObject(rm, craft, e, 1, 5.0, lane, &{1})`. Exactly one attempt to spawn a bomb
per call; if `SpawnObject`'s own lane test fails, that stage has no new bomb. Confidence **78**.

## `Detonator_SpawnObject(rm, craft, pos, count, spacing, lane, kinds[])` (`0x00064f70`)

Arguments `r3`..`r9`, `spacing` in `f1` (PPC64 ELF: it holds `r7`'s slot). For `i` in
`0..count` (`0x000650b0`-`0x00065204`), with `kind = kinds[i]` and `e` = entry `pos`:

1. **Mine cap** (`kind == 0` only, `0x000650b8`-`0x00065100`): if
   `MineCountStage[rm+0x2e10 - 1] - mineManager.live <= 0` (flag true; else `- rm+0x6de48`),
   **return**, abandoning the rest of the cluster.
2. If `e.usable` (`0x00065134`) and `j = e.lanes/2 + lane` has `0 <= j < e.lanes` and
   `e.laneArray[j] == 3` (`0x00065180`-`0x000651c4`):
   - **Transform** (`0x00065108`-`0x0006517c`; `100.0` at TOC `-0x6830`, the identity at
     `0x00769d70`): position `P = e.centre + sec.v30 * (3.5 * lane)`; a matrix with rows
     `(1,0,0,0)`, `up = -sec.v20`, `(0,0,1,0)`, `P`; and a vector `sec.v10 * 100`.
     Confidence 70 for the position, 55 for what the receiving init does with the rest.
   - `kind == 0`, **mine** (`0x000652f4`-`0x000653e4`): `obj = mineManager.slots[live]`
     (`mgr+0x78 + live*4`), `obj+0x40 = 1`, `obj+0x44 = 0`, `obj+0x48 = ++*0x00994060` (a global
     serial); `mgr+0x34 |= 2`; init `0x0010e968(obj, &matrix, &(v10*100), f1 = sec+0x40,
     0, craft, 0, live, (s8)sec+0x60; stack pos, j)`; then `live += 1` (`mgr+0x398`),
     `rm+0x6de48 += 1`, `rm+0x6de34 += 1` (mines laid this race).
   - `kind == 1`, **bomb** (`0x000653e8`-`0x000654d8`): `obj = bombManager.slots[live]`
     (`mgr+0x58 + live*4`), the same `+0x40/+0x44/+0x48` writes, `mgr+0x34 |= 2`, virtual
     slot `+0x40` `(obj, &P, &up, 0, f1 = sec+0x40, *(craft+0x5ee0), (s8)sec+0x60, .., pos;
     stack j)`; `live += 1` (`mgr+0x98`), `rm+0x6de38 += 1` (bombs laid).
   - any kind: `e.laneArray[j] = kind` (`0x0006529c`-`0x000652c4`), so a mine leaves `0` and
     a bomb `1` in its lane; another kind only blocks the lane.
3. **Advance** (`0x000651c8`-`0x000651f4`): if `spacing > 0`, `pos += max(1, ceil(spacing))`
   entries. The objects of one cluster run **down the track in one lane**, `spacing` sections
   apart. There is no bound check: a long cluster near the end of the lap reads past the
   entry array (the disc's clusters span at most 6 entries past `pos`).

Confidence **78** for the whole function.

### Pools

- **Mine manager** (`MineManager.cpp`, constructors `0x0011b538`/`0x0011b8e8`): capacity
  `+0x3a0 = 200` when the game mode `*(*(TOC -0x36ec)+0xe0)` is `14` (Detonator), else `64`
  (`0x0011b628`-`0x0011b63c`, `0x0011b8d0`); one object per slot built up front, a
  `DetonatorMine` (368 bytes, `0x0013a7d8`) in mode `14` when a further flag byte is set, a `Mine` (256 bytes, `0x00140b80`)
  otherwise (`0x0011b658`-`0x0011b70c`); live count `+0x398 = 0`. Confidence 78.
- **Bomb manager** (`BombManager.cpp`, `0x0012f088`/`0x0012f430`): capacity `+0x9c = 2` in mode
  `14`, else `16` (`0x0012f174`-`0x0012f184`, `0x0012f418`); slots at `+0x58`, live count
  `+0x98`. Confidence 75.
- `SpawnObject` checks neither capacity. Mines are bounded by the stage counts (max `200`, the
  pool size) through the cap; **bombs are not bounded**: a third live bomb would index past the
  two-slot pool, so the original presumably relies on bombs going off within a stage. It does:
  step 0's `BombManager_DetonateAll` triggers the old bomb before step 5 lays the new one
  ([detonator-bomb.md](detonator-bomb.md#placement-and-the-stage-step)).

## What Phase 4/5 can and cannot take from this

**Can implement**: both files and their fields (read from the disc, not transcribed); the
per-entry survey (with a stand-in for the collision codes: a lane is free when the downward
probe hits drivable track); groups per stage, the eligibility gate, the `avg` divide, the
chunked placement, the top-up, the one bomb, the lane quirk, the cluster spacing, the lane mark
values, the 200 mine pool and the "target total" meaning of the stage counts. `Libc_Rand`'s
sequence is the PS3's and is not reproduced: the draws go through the seeded `Rng`, in the same
order (one draw per group attempt, two per top-up or bomb attempt).

**Unread, each a named gap**: what the collision return codes `0`/`2`/`4`/`5` mean; the fork
walk; `sec+0x40` and `sec+0x60`; what the mine init and bomb vcall do with the matrix's fixed
rows, `v10*100` and those two values; `angleUp`/`angleRight` (parsed, never passed to
`SpawnObject`; another reader of the group array was not searched for, confidence 70 that they
are unused). `0x0012ae38`, `0x0012ae10`, mine slot `+0x3c`, how a mine or bomb leaves its pool
and frees its lane, and the third bomb were read later: see the note under step 0.

## Cross-check: 2048 and Omega

Both executables carry the same strings (`data\xml\DetonatorLayouts\PatternFile.xml`,
`...\GamePlayValues.xml`, `...\%s.xml`, `MineCountStage%i`, `stageToAppearFrom`,
`percentageAcrossTrack`, `NumStepsToIgnoreAfterJump`), and the files are the same:
`GamePlayValues.xml` is byte-identical on HD `DATA00`, 2048 `PSP2/data.psarc` and Omega
`data00.psarc` (MD5 `4b150957...`); `PatternFile.xml` is 16,224 bytes with CRLF on 2048 and
Omega (MD5 `82f47f1b...`) and 16,108 bytes with LF on HD, the same content once `\r` is
dropped (MD5 `eb2a5416...`).

- **2048 v1.04, `FUN_8117cf22`**: **checked, applies, not wired**
  ([vita-2048-eu-v104/detonator-placement.md](../vita-2048-eu-v104/detonator-placement.md)).
  `llvm-objdump --triple=thumbv7` over `0x8117cf22`-`0x8117e9c2`: the same group stride
  `0xd60` (`0x8117d414`, `0x8117d4b0`), cluster stride `0x44`, entry stride `0x30`, the lane
  value `3` tested twice (`0x8117e0e0`, `0x8117e186`), the `5.0` spacing (`0x8117cf2c`), the
  placed-this-call counter cleared on entry (`rm+0x9626c`, `0x8117cf44`), four
  `Collision_SweepSegment` calls, and three calls to one helper (`0x8117cb9c`, the spawn).
  Structural, not read line by line. Confidence 70.
- **Omega, `FUN_012a2300`**: **checked, applies, not wired**
  ([ps4-omega-eu/detonator-placement.md](../ps4-omega-eu/detonator-placement.md)). GNU
  `objdump -b binary -m i386:x86-64` over segment 0 of the SELF (file `0x50f0`, image base
  `0x01000000`): steps 2 to 5 read line by line (`0x012a3720`-`0x012a3a97`) and match HD
  instruction for instruction, with pointer-width layout changes (group stride `0xd68`, entry
  stride `0x40`: lanes `+0x00`, lane array `+0x08`, usable `+0x10`), the stage counts at
  `0x0193fc10`, the flag at `0x0193fc00`, the start-zone `150.0` (`0x01801000`), the `5.0`
  (`0x01801004`), spawn `0x012a3aa0` and rand `0x017a59f0`. It takes the stage as a second
  argument (`esi`), which Ghidra's signature drops. Its `RemoveMinesEachStage`-style head
  (`0x012a2338`-`0x012a23e0`: every live mine `+0x74 |= 4`, every live bomb's lane back to `3`)
  is the piece HD's `0x0012ae38` hides. Confidence 75.
