# HD/Fury's Zone circuits take `ZoneModeDLC3.effectSettings`, and that is where the Zone hull's sun comes from

`EBOOT.elf` (Wipeout HD/Fury, PS3, `ps3-hdfury-eu`), image base `0x00000000`. Read
2026-11-01 (`hd-zone-hull-light`) off Ghidra and `llvm-objdump -d --mcpu=pwr6`, with
every TOC-relative load resolved against the function's own OPD entry
([memory.md](memory.md), "Every function has its own TOC").

**The question.** The maintainer, playing: "on HD in zone races, the craft is too dark".
The Zone hull's live `directionalLight0Colour` is `(1.144, 1.091, 1.906)`, read off the
hull's fragment program in two RPCS3 draw dumps of a `zone_1` Zone race at `Sub Flash`
([hd-ship-materials.md](../../../rendering/hd-ship-materials.md), "The Zone-mode hull").
This port fed `zonemode.effectsettings`' stage 3 `Sun colour`, `1 1 1`, and no stage of
that file authors the live number.

**The answer: the original does not read `zonemode.effectsettings` on a Zone circuit.**
It reads `zonemodedlc3.effectsettings`, whose stage 3 `Sun colour` is
`1.143733 1.091405 1.906221`: the live value to every printed digit, in all three lanes.
There is no blend; there is a second file.

## 1. `directionalLight0Colour` in Zone is the stage table's `Sun colour`, cross-faded

`Scene_PrepareFrame` (`0x003aa888`, TOC `0x008bd3c4`) publishes shader parameter 15,
`directionalLight0Colour`, from `block+0x7bb0` (`block = 0x00c49000`; the publication run
is [zone-effectsettings-loader.md](zone-effectsettings-loader.md), "The rest of the
publication block"). It fills `block+0x7bb0` two ways, on `g_ZoneEffectsActive`
(`0x00d45f84`, [zone-sky.md](zone-sky.md)):

| `g_ZoneEffectsActive` | `block+0x7bb0` (and `+0x7bc0`) from | that is |
| --- | --- | --- |
| `0` | the environment object's `+0x430` | the circuit's `.envsettings` sun |
| non-zero | `*(TOC 0x008b71e0)` = `0x00c815d0` | the Zone stage blend's output |

`0x00c815d0` is `0x00c7dfb0 + 0x3620`. Its only writer in the image is
`Environment_UpdateStageBlend` (`0x003da540`); the other `li r0, 0x3620` (`0x003d10b8`)
is the registrar's start-up clear:

```text
003dc338  addi  r9, r31, 400        ; 400 = 0x190, the stage record's `Lighting.Sun colour`
003dc354  add   r9, r9, r30         ; r30 = 0x00c7dfb0, r31 = stage A * 0x250
003dc35c  lfs   f0, 8(r5)           ; t, the blend weight
003dc360  li    r0, 0x3620
003dc368  addi  r11, r10, 400       ; r10 = stage B * 0x250
003dc3d8  lvx   v1, r9, r28         ; r28 = 0x1000: stages[A] + 0x190
003dc3e8  vmaddfp v1, v1, v0, v29   ; v0 = splat(t)
003dc41c  stvx  v1, r30, r0         ; 0x00c815d0 = t * stages[A].Sun
003dc42c  fsubs f0, f31, f0         ; 1 - t
003dc430  lvx   v13, r11, r28       ; stages[B] + 0x190
003dc490  vmaddfp v13, v13, v0, v1
003dc4a8  stvx  v13, r30, r0        ; 0x00c815d0 = t * Sun[A] + (1 - t) * Sun[B]
```

`stages = 0x00c7dfb0 + 0x1000 = 0x00c7efb0`, `g_effect_settings_stages`, and `+0x190` is
`Lighting.Sun colour` in the key-to-offset table `Environment_RegisterStageSchema`
registers ([zone-effectsettings-loader.md](zone-effectsettings-loader.md), fourteenth
pass). The same run writes `+0x1a0`..`+0x1c0` to `0x3630`..`0x3650` and three more vec4s
to `0x35f0`..`0x3610`, which `Scene_PrepareFrame` publishes as `constantAmbientColour`,
`prelitScaleSpecular` and `prelitBias` on the same Zone branch. Confidence **85**: one
`lvx` base traced to the stage record, one `li` destination, no branch between them;
`Scene_PrepareFrame` applies nothing to `+0x7bb0` after the store.

**So the hull's sun is an authored stage colour.** Which file fills the stage table is
section 2.

## 2. `Environment_LoadRaceScene` picks the file by the circuit's track id

`Environment_LoadRaceScene` (`0x003f3fb0`, OPD TOC `0x008bd3c4`, read off the OPD) sets
`g_ZoneEffectsActive` when `(1 << mode) & 0x00206040` (modes 6, 13, 14, 21,
[zone-sky.md](zone-sky.md)), then compares the race's track id (`*(track + 0x78)`, the
track record at `g_GameState + 0x1bc`) against four strings and calls
`Environment_LoadStageTextures` (`0x003d6dc8`) with one of four paths:

```text
003f4074  lwz   r30, 444(r11)          ; g_GameState + 0x1bc, the track record
003f408c  lwz   r4, -0x52c0(r2)        ; 0x008b8104 -> "25_Track"
003f4090  lwz   r31, 120(r9)           ; the track id
003f4098  bl    0x0043e838             ; case-blind compare, 0 on equal
003f40a0  ...   r29 = (result == 0)
003f40a4  lwz   r4, -0x52bc(r2)        ; 0x008b8108 -> "26_Track", r29 |= equal
003f40e0  lwz   r4, -0x52b8(r2)        ; 0x008b810c -> "27_Track", r29 |= equal
003f4108  lwz   r4, -0x52b4(r2)        ; 0x008b8110 -> "28_Track"
003f4118  bt    30, 0x3f5158           ; equal -> the DLC3 pair
003f4124  bf    30, 0x3f5158           ; r29 set -> the DLC3 pair
; base pair
003f4128  lbz   r0, 0(r27)             ; byte 0x009384e1
003f4130  bf    30, 0x3f4140           ; non-zero -> Zone
003f4134  lwz   r0, 224(r26)           ; the mode id, g_GameState + 0xe0
003f4138  cmpwi 7, r0, 14              ; mode 14 (Detonator) -> 0x3f51b4
003f4140  lwz   r3, -0x52a4(r2)        ; 0x008b8120 -> "Data/Environments/ZoneMode.effectSettings"
003f51b4  lwz   r3, -0x52a8(r2)        ; 0x008b811c -> "Data/Environments/DetonatorMode.effectSettings"
; DLC3 pair, the same two tests
003f5170  lwz   r3, -0x52ac(r2)        ; 0x008b8118 -> "Data/Environments/ZoneModeDLC3.effectSettings"
003f5198  lwz   r3, -0x52b0(r2)        ; 0x008b8114 -> "Data/Environments/DetonatorModeDLC3.effectSettings"
```

`0x0043e838` (TOC `0x008bd3c4`) walks both strings a byte at a time, folds each through
the ctype table at `*(TOC - 0x2b7c)` (`+32` where the upper-case bit is set) and returns
the difference: a case-blind `strcmp`, `0` on equal. `25_Track`..`28_Track` are HD's four
Zone circuits, `zone_1`..`zone_4` in that order (`oag_hd::race::ZONE_TRACK_2`'s docs, the
`PI_Track` `location` pairing read off the disc). The byte at `0x009384e1` is unread
elsewhere too ([endrace-loyalty.md](endrace-loyalty.md)); it only chooses Zone against
Detonator, never base against DLC3.

**The law: a Zone race on `zone_1`..`zone_4` takes `zonemodedlc3.effectsettings`; on any
other circuit, `zonemode.effectsettings`.** Detonator splits the same way between
`detonatormode` and `detonatormodedlc3`.

## Evidence and confidence

| Claim | Evidence | Confidence |
| --- | --- | ---: |
| In Zone, `directionalLight0Colour` = `t * Sun[A] + (1 - t) * Sun[B]` of the stage table | section 1 | 85 |
| A Zone circuit (`25_Track`..`28_Track`) loads `ZoneModeDLC3.effectSettings`, any other circuit `ZoneMode.effectSettings` | section 2, read off the branch; the live `directionalLight0Colour` `(1.144, 1.091, 1.906)` in a `zone_1` race is DLC3's stage 3 `Sun colour` `(1.143733, 1.091405, 1.906221)` exactly, where the base file authors `1 1 1` and no base stage authors anything above 1.0 | 88 |
| Second binary: 2048's `Environment_Load` reaches for `ZoneModeHDFury[DLC3].effectSettings` by whether the circuit names `Zone_1`..`Zone_4` | [zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md), the same rule shape | corroboration |
| Detonator splits the same way | section 2 only, nothing live | 80 |

The 88 is short of 95 by its live leg: one stage (`Sub Flash`) read live, and the
selector itself was never watched (no breakpoint on `0x003f5170`). A second live stage,
or a breakpoint on the DLC3 branch, would close it.

## Names

| Address | Name | Kind | Confidence |
| --- | --- | --- | ---: |
| `0x0043e838` | `Str_CompareNoCase` | function | 80 |
| `0x008b8104` | `g_ZoneCircuitTrackIds` | data | 88 |
| `0x008b8114` | `g_ZonePalettePaths` | data | 88 |

## What this port does with it

`oag_title::ZonePalette::TitleWide` carries the split as data (`entry`, `zone_circuits`,
`zone_circuit_entry`); HD's `DEFAULTS` names `ZONE_TRACKS` and
`oag_hd::race::ZONE_CIRCUIT_PALETTE`. Pinned by
`zone_grade_ground_truth::a_zone_circuit_lights_the_hull_with_the_dlc3_sun` (stage 3
gives `(1.143733, 1.091405, 1.906221)`; red when the split is dropped: checked) and
`an_ordinary_circuit_in_zone_keeps_the_base_table`.

**The whole table changes on the Zone circuits, not the sun alone.** DLC3 authors 48
keys a stage where the base file authors 23 (the `Scene.*`/`Track.*` gradients and middle
colours, aurora, airbrake and Detonator colours are new), and of the keys both carry,
`Sun colour` differs at 14 stages, `Fog colour` at 12, `Constant Ambient Colour` at 10,
`Scene.Base Colour`/`Highlight` at 13 and 14, `Track.Texture Colour` at all 15,
`Sky horizon`/`zenith colour` at 10. So the Zone circuits' recolour moves with this
change too, to what the original loads there.

## Cross-title

- **Omega: checked, applies, not wired.** Omega's `eboot.bin` names 2048's four paths
  (`data/ZoneEnvironmentHDFury/ZoneModeHDFury[DLC3].effectSettings` and the Detonator
  pair), and `data00.psarc` ships all four under `Data/zoneenvironmenthdfury/`;
  `ZoneModeHDFuryDLC3.effectSettings` is byte-identical to HD's `zonemodedlc3`
  (43,417 B, `cmp`). Omega's `zone_palette` is `None`: its Zone draws no stage grade at
  all, so there is nothing to switch; Omega's own selector is not read in its Ghidra
  program.
- **2048: same rule shape, files absent.** Its `Environment_Load` reaches for the same
  pair on the four ported `zone_N` circuits, and none of them ships on its packages
  ([effectsettings.md](../../../formats/effectsettings.md)); its own circuits take
  `ZoneMode2048.effectSettings`, unchanged here.
