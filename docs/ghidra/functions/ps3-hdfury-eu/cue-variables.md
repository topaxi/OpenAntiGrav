# HD's cue variables, the rest of the grain opcodes, and the race-cue audit

2026-11-15, lane `hd-sound-audit`. Wipeout HD/Fury EU (`BCES-00664`, `hdfury-ps3-eu-dec.iso`),
Ghidra program `/hdfury/EBOOT-ps3-hdfury-eu.elf`, RPCS3 for the live half. The question was the
maintainer's: HD's sound "is wrong in places" where Pure and Pulse sound right, `SPEEDUPPAD`
first. The dispatch table, the pitch chain and the tick are on [sound.md](sound.md); this page
adds what a cue's command list **reads** while it runs.

## Bottom line

1. **Most of HD's race cues are not one sound or a set of takes. They are a list that keys several
   voices and chooses between authored versions with four *cue variables* the caller supplies.**
   Variable 2 is the **listener's perspective** (0 inside the craft or an unplaced cue, 1 the
   player's craft from outside, 2 another craft), variable 0 is a **severity or state**, variable 1
   the **surface** of a collision. Confidence 88 (handlers read, the writer read, live values).
2. **This engine walks an HD race cue with Pulse's operand layout and no variables, so the walk
   stops at the first guard and the cue falls back to a flat pick of one waveform.** 31 of the 37
   `weapons.bnk` cues it plays, plus `.COLLISIONS` and `~magstrip01`, are played as one waveform
   out of everything the list can reach. Confidence 95 (the loader's own report, below).
3. **`SPEEDUPPAD` is the plainest case**: the original keys three voices (a body, then a whoosh
   behind-left and behind-right 42 ms apart, detuned together by up to two semitones); this
   engine plays one of the three at random, centred and undetuned. That is the "sometimes
   doesn't sound correct": three different wrong sounds, one of them the body alone.

**Wired, 2026-11-15, lane `hd-cue-walk`**: see [Wired](#wired). Rows 1, 3 and 4 of the audit below play
their lists now, row 7's bend and row 8's rear half for the race cues; the "this engine" paragraphs describe the engine
before that change.

## Names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x00625d50` | function | `Scream_DoGrainRandomBend` | 88 |
| `0x006242b0` | function | `Scream_DoGrainSetBend` | 82 |
| `0x00623530` | function | `Scream_DoGrainSetVar` | 82 |
| `0x00623580` | function | `Scream_DoGrainIncVar` | 82 |
| `0x00625ad8` | function | `Scream_DoGrainGotoRandom` | 82 |
| `0x00623c88` | function | `Scream_DoGrainWaitVoices` | 78 |
| `0x00626318` | function | `Scream_DoGrainKeyOff` | 60 |
| `0x00625820` | function | `Scream_DoGrainLoopBack` | 85 |
| `0x006234d0` | function | `Scream_DoGrainStop` | 80 |
| `0x00623830` | function | `Scream_DoGrainEnd` | 85 |
| `0x006265d0` | function | `Scream_DoGrainModulator` | 70 |
| `0x00624418` | function | `Scream_SetCueParameter` | 90 |
| `0x00087fa8` | function | `HudSight_UpdateTone` | 80 |
| `0x0029fd00` | function | `SpeedupPad_PlayCue` | 82 |
| `0x000cff78` | function | `Craft_PlayPlayerCue` | 78 |
| `0x002fc940` | function | `SoundEmitter_Init` | 80 |
| `0x002fcc20` | function | `SoundEmitter_PlayCue` | 80 |
| `0x002fcd70` | function | `SoundEmitter_Update` | 78 |
| `0x002fee20` | function | `SoundEmitterCue_Init` | 80 |
| `0x002fdc00` | function | `SoundEmitterCue_Update` | 80 |

The opcode handlers are reached from the dispatch table at `0x00927614` (OPD word, then code
word), all with TOC `0x008bd3c4`. None of the readings below rests on a TOC-relative string. The
game-side functions (`0x0029fd00` to `0x002fee20`) carry TOC `0x008ad4d8`.
`Scream_DoGrainKeyOff` is below 70: its callee `0x0062fd98` was not read, and the name leans on
2048's `0x29` being a key-off.

## The operand bytes sit one place over from Pulse's

A command is 8 bytes. On HD (big-endian) the opcode is memory byte 0 and the three operand bytes
follow it: `[op, a, b, c]`. Every HD handler reads `a` as `lbz 1(r5)`. On Pulse (little-endian) the
opcode is memory byte 3 and the operands come first, `[a, b, c, op]`. Read as one 32-bit word in
the bank's own byte order, **`a` is `word & 0xff` on Pulse and `(word >> 16) & 0xff` on HD**, and
`c` is the reverse. A handler that takes the whole 24-bit operand (a key-on's descriptor offset, a
child record, `0x1a`'s range) reads the same on both; a handler that takes a byte does not.

`oag_formats::sblk::timeline` reads bytes Pulse's way except where a caller opted in:

| Opcode | HD handler reads | `timeline.rs` reads on HD | Effect |
| --- | --- | --- | --- |
| `0x19` alternate | count `a`, stride `b`, last pick cached in `c` (`Scream_DoGrainAlternate`) | count `word & 0xff` = `c`, unless `WalkModel::hd_alternates`, which only the front end sets (`crates/sound/src/sfx/layers.rs:179`) | a race cue's group reads as zero alternates: unread, flat pick |
| `0x1b` random bend | percent `a` (`Scream_DoGrainRandomBend`) | `word & 0xff` = `c` = 0 (`crates/formats/src/sblk/timeline.rs:401`) | **every HD bend is read as 0 %**, also on cues that play as a timeline |
| `0x22` guard | variable `a`, mode `b`, threshold `c` (`Scream_DoGrainGuard`, sound.md) | variable `c`, threshold `a` (`guard_skips`, `timeline.rs:222`), and only under `WalkModel::globals` | latent: no HD cue walked with `globals` meets a guard today |
| `0x23`/`0x24` marker, goto | id `a` | `(word >> 16) & 0xff` = `a` | right |

## The opcodes read this pass

Every opcode HD's 50 bank files use is now read: `01 04 05 08 14 15 16 18 19 1a 1b 1c 1e 20 22 23 24
25 26 29 2b` (census of all 1,963 cues). The ones not on [sound.md](sound.md):

| Opcode | Function | What it does | Evidence |
| --- | --- | --- | --- |
| `0x1b` | `Scream_DoGrainRandomBend` | `Scream_SetVoiceBend(handler, (r - 0x8000) * (s8)a / 100)`, `r` a 16-bit draw rescaled to `0..0xffff`. The bend is the **handler's**: it reaches every voice keyed after it and the ones already sounding, each through its own descriptor's bend range (`+0x08` down, `+0x09` up, semitones) | decompile; same algorithm as Pulse's `Scream_OpRandomBend` |
| `0x1c` | `Scream_DoGrainSetBend` | a **fixed** bend: `a < 0` gives `a << 8`, else `a * 0x7fff / 0x7f`, handed to `Scream_SetVoiceBend` | decompile |
| `0x1e` | `Scream_DoGrainSetVar` | `var[a] = b` (`handler + 0xa4 + a`); `a < 0` writes the global table `*0x008c0038` at `~a` | decompile |
| `0x20` | `Scream_DoGrainIncVar` | `var[a] += 1`, clamped at `0x7f`; `a < 0` the global | decompile |
| `0x25` | `Scream_DoGrainGotoRandom` | goto the marker whose id is `a + rand16 % (b - a + 1)`; the same depth guard of 8 as `0x24` | decompile |
| `0x26` | `Scream_DoGrainWaitVoices` | while the handler still has voices (`0x0062ea50`) or flag `0x10` of `+0x22` is set: return 1 and step the program counter back, so the command runs again a tick later. **Waits for the cue's voices to end** | decompile |
| `0x29` | `Scream_DoGrainKeyOff` | `0x0062fd98(handler + 0x24, 0)`; callee unread | decompile, name from 2048 |
| `0x16` | `Scream_DoGrainLoopBack` | scan back for the nearest `0x15`, resume there; the second run inside one tick returns 1 (flag `0x40` of `+0x22`, cleared by `Scream_TickCommandList`); no `0x15` is an error | decompile; Pulse's `Scream_OpLoopBack` reads the same |
| `0x18` | `Scream_DoGrainStop` | returns `-1`: `Scream_StepCommandList` returns 1 and the caller stops the handler | two instructions |
| `0x2b` | `Scream_DoGrainEnd` | program counter = command count - 1, so the next step runs off the end | decompile; as Pulse |
| `0x04` | `Scream_DoGrainModulator` | fills one of the handler's four 0x34-byte modulator slots (`handler + 0xb4 + slot * 0x34`) from a 16-byte record: slot `[0]`, target `[1]` (0 clears the slot), `[2]`, shape `[3]`, rate and depth words, a random phase when bit 1 of `[8..10]` is set; then `0x006285f0`. **An LFO on the voices of the cue**; which target is pitch and which volume is not read | decompile |
| `0x1a` | `0x00625e38` (not named here) | `rand16 % (operand24 + 1)` extra ticks before the next command | decompile |

`Scream_StepCommandList` (`0x00625218`) carries the alternate bookkeeping as Pulse's does: after
the chosen block's `stride` commands it adds the skip stored at `+0xac`. A second `0x19` while one
is open is an error that stops the handler.

## Where the four variables come from

The SBlk start routine `0x00626e78(bank, index, volume, pan, request)` copies request bytes
`+0x0c..+0x0f` into `handler + 0xa4..+0xa7` when request flag `0x20` is set, and zeroes them when it
is not. `Scream_DoGrainPlayChild` (`0x006274b0`) copies the parent's four bytes into the child's
request (`lbz 0xa4..0xa7(r31)`, `stb 0x80..0x83(r1)`), so **a child cue sees its parent's
variables**. Two game-side paths fill the request:

- **Unplaced cues**: `Sound_PlayBankCue` (`0x002ffa58`) sets flag `0x20` only when its eighth
  argument (`r10`, a pointer to four bytes) is non-null. A ninth, stack argument is a block of
  **eight floats** copied to request `+0x28` with flags `0x3000` (and from there to
  `handler + 0x48`).
- **Placed cues**: `SoundEmitter_PlayCue` (`0x002fcc20`) allocates a 0x70-byte record,
  `SoundEmitterCue_Init` (`0x002fee20`) fills it (four variable ints at `+0x38..+0x44`, all `-1`;
  eight floats at `+0x18`, float 2 set to `1.0`), and `SoundEmitter_Update` (`0x002fcd70`) runs
  `SoundEmitterCue_Update` (`0x002fdc00`) on every record of the emitter each frame with the
  **emitter's** own four ints (`emitter + 0x90..+0x9c`). A value of zero or more is copied into
  the record and, when the sound is started, into the request with flag `0x20`.
  `SoundEmitter_Init` (`0x002fc940`) writes `-1` to all four and `200.0` to the radius at `+0x38`.

**Variable 2 is written by `Ship_UpdateEngineCrossfade` (`0x000d5968`, [xfade.md](xfade.md))**: at
`0x000d5a44` it stores `craft + 0x5f14` into `emitter + 0x98`, the emitter being `craft + 0x5ee0`.
`craft + 0x5f14` is set in the same function: `2` (`0x000d5a28`, `0x000d5eb4`, `0x000d5fe4`) for a
craft whose role word (`craft + 0x628c`) is not zero or that the camera object does not target;
for the player's craft `0` (`0x000d60b0`) when `craft + 0x5f42` is set (the cockpit view,
[absorb-feedback.md](absorb-feedback.md)) and `1` (`0x000d5ed0`) when it is not. The branches
around the camera object (`0x000d5ff8`, a mode mask of `0x8c1`) were not followed to the end.

### Measured live

`scripts/rpcs3-hd-cue-handlers.py` polls the live handler list (`*(*(0x008c00fc) + 0x24)`, next at
`+0x70`) about 150 times a second through the emulator's memory, with no breakpoint, and logs each
new handler's cue, variables and float block. Three race captures on two save states (Talon's
Junction, Venom, single race, chase camera, RPCS3 recompiler): 200 s of racing (1,659 handlers)
and two sweeps that armed pickup states 0 to 10 on the player and fired each (1,632 and 2,150
handlers; weapons fired in the second only, the first state had them off). A fourth, 40 s
(1,146 handlers), ran after a race had ended with the player's craft on autopilot.
A breakpoint on the start routine was tried first and abandoned: the engine layers restart so
often that the game advanced 46 sound ticks in 240 s.

| Cue (bank) | Variables seen `[v0, v1, v2, v3]` | Reading |
| --- | --- | --- |
| `SPEEDUPPAD` (weapons), player's own | `[0,0,0,0]`, volume word 1024, float 2 = 1.0: 27 of 27 in the three race captures | unplaced, perspective 0 |
| `SPEEDUPPAD`, other craft | `[0,0,2,0]` and `[1,0,2,0]`, attenuated: 194 of 194 | perspective 2 |
| `SPEEDUPPAD`, after the race ended | `[_,0,2,0]` 41, and `[0,0,1,0]`, `[1,0,1,0]` 4 (no unplaced start) | the player's craft is an emitter at perspective 1 |
| `TURBO` | player `[0,0,0,0]` full volume; others `[0,0,2,0]`, `[2,0,2,0]`, `[1,1,2,0]` | as the pad |
| `c_Burst`, `c_CrackLoopL/R`, `c_ElecArcA/B` (children of `.COLLSHIELD`, shiphd) | `[0,0,1,0]` every time | the player's craft, from outside |
| `c_BurstC`, `c_CrackLoopC`, `c_ElecArcC/D` | `[0,0,2,0]` every time | another craft: the mono versions |
| `c_GShipShipS` / `M` / `L` (children of `.GRIND`) | `[0,..]` / `[1,..]` / `[2,..]`, 56 / 8 / 12 | variable 0 is the severity |
| `c_CShipShipM` / `c_CShipShipL` (children of `.COLLISIONS`) | `[0,..]` / `[1,..]` | the same, on the table below |
| `MINELAUNCH`, `QUAKELAUNCH`, `~SHIELD`, `LEACH` fired by the player | `[0,0,1,0]` | the firing craft's emitter |
| `CANNON` fired by others | `[0,0,2,0]`, `[1,0,2,0]` | perspective 2 |
| `ROCKET`, `~ROCKETTVL`, `MISSILE`, `~MISSILETVL`, `PLASMA`, `~PLASMATVL`, every explosion cue | `[0,0,0,0]`, attenuated | a projectile's own emitter sets no variable |
| speech, results and `MESSAGE` cues | `[0,0,0,0]`, float block all zero | no block supplied |

Variable 0 also reads `1` on some craft cues with no severity (`[1,0,2,0]` on a pad): what sets it
there is not read. Confidence 88 for variable 2, 92 for variable 0 and 90 for variable 1 on the collision family
(below), 60 for "variable 0 is otherwise unused".

### The collision family, which closes sound.md's open item

sound.md left open "which `var_ref` means severity and which ship-versus-wall" and expected the
global table. It is neither global. `.COLLISIONS` (shiphd.bnk cue 50) is
`22 01 01 00 / 08 c_CShipShip / 22 01 01 01 / 08 c_CShipWall`: **variable 1 == 0 branches to the
ship-ship cue, variable 1 == 1 to the ship-wall cue**. `c_CShipShip`, `c_CShipWall` and
`c_GShipShip` each open with `22 00 01 00 / 24 00 / 22 00 01 01 / 24 01 / 24 02`: **variable 0
== 0 runs the block at marker 0, == 1 the block at marker 1, anything else the block at marker
2**, and each block plays one child:

| Parent | variable 0 = 0 | = 1 | otherwise |
| --- | --- | --- | --- |
| `c_CShipWall` (cue 55) | `c_CShipWallS` | `c_CShipWallM` | `c_CShipWallL` |
| `c_GShipShip` (cue 41) | `c_GShipShipS` | `c_GShipShipM` | `c_GShipShipL` |
| `c_CShipShip` (cue 51) | `c_CShipShipM` | `c_CShipShipL` | `c_CShipShipL` |
| `.COLLSHIELD` (cue 60) | `c_GS_Small` + `c_CS_Small` | `c_GS_Medium` + `c_CS_Medium` | the `Large` pair |

`c_CShipShip` does name `c_CShipShipS`, in a command that sits between block 0's closing goto and
marker 1 (command 749), where nothing reaches it: **a ship-to-ship hit never plays the small
set**. `.GRIND` (cue 40) has `.COLLISIONS`' shape over `c_GShipShip`. Live, the child that
started matched this table on **149 of 149** starts over the four captures: `c_GShipShipS`/`M`/`L`
56/8/12 with variable 0 at 0/1/2; `c_CShipShipM` 21 at 0, `c_CShipShipL` 17 at 1 and 4 at 2, and
no `c_CShipShipS` at all; `c_CShipWallS`/`M`/`L` 1/3/2; the shield's `Small`/`Medium`/`Large`
children 20/4/1. Confidence 92.

`.COLLISIONS` never shows by that name in a capture: opcode `0x08` replaces the handler's cue, so
the poll sees `c_CShipShip`. Two more top-level cues start beside it and this engine has neither:
`.COLLSHIELD` (cue 60, 12 starts) and `.GRINDSHIELD` (cue 61), the shield's crackle, each a
three-way switch on variable 0 over children that themselves switch on variable 2.

## `SPEEDUPPAD`

`weapons.bnk` cue 48, 23 commands, cue volume 120. Bytes as stored (`DATA00` and `DATA01` carry the
same 5,073,952-byte file; the race's live bank has 73 cues, 867 commands and 170 waveforms, which
is `weapons.bnk` and not `weapons_zone.bnk`'s 167):

```text
01 00 b3 20  d0     key-on A   -74/66 (22,020 Hz, 1.094 s), angle 0, bend range 0/0
1b 64 00 00  d0     random bend, 100 %
22 02 01 00  d0     unless var2 == 0, skip the next command
24 00 00 00  d0     goto marker 0
22 02 01 01  d0     unless var2 == 1 ...
24 01 00 00  d0     goto marker 1
22 02 01 02  d0     unless var2 == 2 ...
24 02 00 00  d0     goto marker 2
24 63 00 00  d0     goto marker 0x63 (the end)
23 00               marker 0
01 00 b3 c4  d0     key-on B   -62/66 (44,051 Hz, 2.142 s), angle 220, bend range 2/2
01 00 b4 68  d10    key-on C   same length, another waveform, angle 140, bend 2/2, 10 ticks on
24 63
23 01               marker 1:  B at angle 0, C at angle 0 10 ticks on
24 63
23 02               marker 2
19 02 01 00         one of the next two (HD layout: count 2, stride 1)
01 ...       d0     B at angle 0
01 ...       d5     C at angle 0, 5 ticks on
24 63
23 63
```

| Variable 2 | What plays |
| --- | --- |
| 0 | A centred, B behind-left (220 degrees) at tick 0, C behind-right (140 degrees) at tick 10 (41.7 ms); B and C share one bend draw of up to +/-2 semitones |
| 1 | A, B and C all centred on the emitter, C 10 ticks late, same bend |
| 2 | A and **one** of B (tick 0) or C (tick 5), never the previous pick |
| 3 or more | A alone |

**The call site.** `SpeedupPad_PlayCue` (`0x0029fd00`) is the only code that references the string
(`0x0079bd50`, TOC slot `0x008b2f3c`, loads at `0x0029fde4` and `0x0029fe40`). For the player
(role word `craft + 0x628c == 0`, and a two-condition test on two globals this page did not
identify does not divert it) it builds eight zeroed
floats, sets float 2 (`0x003045d8` returns 2) to `1.0` and float 0 (`0x003045c8` returns 0) to
`1.0` or `0.0` by a bit test (`0x000a32e0` on the byte `*(craft + 0x782c) + 0x60`), and calls
`Craft_PlayPlayerCue(craft, bank, "SPEEDUPPAD", 0x400, 0, -1, 0, floats)` (`0x000cff78`), which
calls `Sound_PlayBankCue` with **`r10 = 0`** (`li r9,0` at `0x0029fdf8` becomes `r10` at
`0x000d0080`) and the floats on the stack. No variable block: all four read 0, the rear pair. For
any other craft it calls `SoundEmitter_PlayCue(craft + 0x5ee0, bank, 0, "SPEEDUPPAD", ...)`, and
the emitter's variable 2 is 2. Both are what the capture shows (table above). Confidence 92: bank
bytes, the call site's two branches and 221 live starts agree.

**This engine** (`crates/sound/src/sfx/cue.rs:120`, `:195`; `banks.rs:567` `voices`, `:631`
`pick`): the walk meets `0x1b` after a key-on and `0x22` with no variables, reports both unread,
and `layers::timelines` returns `None` (`layers.rs:187`). The loader's line is
`sfx: SPEEDUPPAD -> 7 waveform(s) from Data\Sound\weapons.bnk` with no timeline. `pick` then draws
one of the seven key-ons uniformly, never the previous: A with weight 1, B with 3, C with 3. One
voice, centred, undetuned.

## The audit

**Method.** Every cue `crates/sound/src/sfx/cue.rs` names for HD was run two ways. *Original*: the
list interpreted with the handlers above (HD operand bytes, alternates, gotos, random gotos,
loops, wait-voices, children inheriting variables and bend), 400 seeded draws per cue, with the
variables at zero and again with each variable at 1, 2 and 3. *Ours*: the same decision
`banks.rs` `load_cue` and `layers.rs` `timelines` make (`cue_timelines_modelled` under the walk
model the loader builds, then the flat, incomplete and rear-angle tests), checked against the
loader's own report from `oag-game <iso> --race --dry-run` with `RUST_LOG=debug`. "Voices" below is
key-ons per play at variables zero; "of N" is how many waveforms our flat pick draws from.

**Matches** (our timeline keys the original's voices at the original's angles): `MESSAGE`, `PLASMA`, `LEACH`, `LEACHFAIL`,
`~ROCKETTVL`, `shieldactive`, `autopilot_eng`, `disengaging`, `PERFECT_LAP`, `NEW_LAP_REC`,
`ready`, `go` and `HBEAT` in `speech.bnk`, `speech_elim.bnk` and `speech_zone.bnk`, the nine
`speech_results.bnk` lines, `navLeft`, `navRight`, `accept_fury`, `reject_fury`.

**Mismatches, ranked by what a player hears.** Impact is an estimate (how often the cue plays
times how wrong it is), not a measurement.

| # | Cue(s), bank | The original | Ours | Conf | Impact |
| ---: | --- | --- | --- | ---: | --- |
| 1 | `SPEEDUPPAD`, weapons | 3 voices, section above | 1 of 7 (3 distinct), `banks.rs:631` | 92 | high: every pad, three different wrong sounds |
| 2 | `.COLLISIONS`, shiphd | variable 1 picks ship or wall, variable 0 picks the severity child (wall S/M/L, ship M/L/L), then 3 to 6 voices over 4 ticks, 100 % bend of +/-1 | 1 of 112 (34 distinct), a heavy hit for a graze (`banks.rs:797` says so) | 90 | high: every contact |
| 3 | `ROCKEXPLWALL`, `ROCKEXPLSHIP`, `MISSILEEXPWALL`, `MISSILEEXPSHIP`, `SHURIKENEXPL`, `QUAKEHIT`, `PLASMAHITSHIP`, `REPULSORHIT`, `SHURIKENHIT`, `CANNONEXPLWALL`, `CANNONEXPLSHIP` | layered explosions: 2 to 17 voices (`ROCKEXPLSHIP` 10, `SHURIKENEXPL` 17, `REPULSORHIT` 12, `PLASMAHITSHIP` 8 over 215 ticks), children picked by `0x19` and `0x25`, hard left/right pairs at 90/270, 100 % bends | 1 of 8 to 41: one layer of the explosion | 85 | high: every hit sounds thin or like the wrong part |
| 4 | `ROCKET`, `MISSILE`, `CANNON`, `MINELAUNCH`, `TURBO`, `SHURIKEN`, `LEACHENERGY`, `ABSORB`, `REPULSOR` | `ROCKET`: one of 10 takes as a **left/right pair** (90/270, or both on the emitter); `CANNON` 3 voices; `MISSILE` 4; `MINELAUNCH` 8 over 95 ticks; `TURBO` 6 over 90 ticks with a 10 % bend; `ABSORB` a +/-30 degree pair 10 ticks apart plus one of four tails; `REPULSOR` 8 over 210 ticks | 1 of 4 to 88: half a stereo pair, or one stage of a sequence | 85 | high for the weapons fired often |
| 5 | `~SHIELD`, `~AUTOPILOT`, `~BLOWUP`, `~ROCKLOCK`, `~PLASMATVL`, `~MISSILETVL`, `~SHURIKENTRAVEL`, `~QUAKETRAVEL`, `~LEACHATTACH`, `~magstrip01` | held lists: `~SHIELD` three loops (90, 270, 330 degrees) under two `0x04` modulators; `~AUTOPILOT` three one-shots, a looping pair and a random blip every 22 ticks; `~BLOWUP` a pair and a hit re-keyed every 45 ticks; `~ROCKLOCK` a left/right beep pair every 53 ticks with a fixed bend chosen by variable 0; the travel cues a loop plus a tick every 33 ticks | one waveform, looped or not: `~SHIELD` 1 of 8 including versions the list does not play at that perspective; the repeating program is PSP-only (`layers.rs:382`), `~ROCKLOCK` is `pick_at` over two key-ons of one waveform (`sfx.rs:462`) | 82 | high while held |
| 6 | cues the original starts and this engine has no cue for | `.COLLSHIELD`, `.GRINDSHIELD`, `.GRIND` (shiphd); `WEAPONPICKUP`, `MINERADAR`, `~BOMBRADAR`, `BOMBLAUNCH`, `BOMBEXPL`, `MINEEXPL` (both wired for a Detonator race alone, 2026-11-22: `Cue::BombExplosion`, `Cue::MineExplosion`, [detonator-cannon.md](detonator-cannon.md); an ordinary race's mines and bombs stay silent), `~SHIP_ONFIRE` (weapons); `CANNON_VO`, `mines` (speech, a weapon's name spoken); `FINAL_LAP`, `RACE_COMPLETE` (speech_results) | silence | 80 that they play, triggers unread | medium to high: the shield crackle and the pickup are constant |
| 7 | `PLASMAHITWALL`, `QUAKELAUNCH`, `accept`, `reject` | a 100 % (10 % on the menu cues) random bend through ranges of 1 to 5 semitones | timeline plays, bend read as 0 % (`timeline.rs:401`) | 90 | low to medium: every repeat is identical |
| 8 | angles the pan law rejects: 9 cues carry a rear angle (120 to 240 degrees), 14 more a voice at exactly 90 | rear and side speakers; in stereo the pan law folds them | `pan_of_angle` has no rear half (`crates/audio/src/spatial.rs:561`), which is one more reason a cue goes flat (`layers.rs:209`) | 70 | low to medium once rows 1 to 5 land |
| 9 | `0x04` modulators on `~SHIELD`, `~MISSILETVL`, `~LEACHATTACH`, `~magstrip01` | an LFO per cue, `Scream_DoGrainModulator` | ignored | 70, targets unread | low to medium |
| 10 | `cont_elim`, speech | a 200-tick wait, the line at +/-30 degrees 5 ticks apart, then `0x26` | flat: one centred voice at once (`0x26` unread) | 85 | low |
| 11 | ADSR | of the 330 key-on descriptors 28 audited `weapons.bnk` cues reach, 28 author a slow attack or a release (`a5ff/9fca`, `b4ff/9fce`, ...); 302 are `80ff/9fc0`, instant on and off | not applied; `0x006323a0` unread | 60 | low |
| 12 | `navUp`, `navDown`, frontend | one voice at 180 degrees | played centred, a recorded choice (`layers.rs:203`) | 85 | low |

Rows 1 to 5 are one fix: an HD walk model that reads the operand bytes HD's way and carries the
four variables, so these cues build timelines and repeating programs the way Pulse's already do.
For a cue this engine plays from the player's own seat the variables are `[0,0,0,0]` when the
original plays it unplaced (`SPEEDUPPAD`, `TURBO`) and `[_,_,1,_]` when it rides the player's
craft emitter in the chase view; another craft's are `[_,_,2,_]`.

## Not determined

- **Per cue, which of the two paths the original takes** (unplaced, or on the craft's emitter):
  measured for the cues in the live table only. `SoundEmitter_PlayCue`'s callers give the rest.
- **What the game computes a collision's severity and surface from.** The caller is
  `FUN_000d8bd0` (`0x000d8bd0`, not named, not read): it loads the `.COLLISIONS` string (TOC
  slot `0x008a8e34`) at `0x000d9038` and `0x000d924c` and stores a value to `emitter + 0x90`
  (variable 0) at `0x000d9014`. `.COLLSHIELD`, `.GRINDSHIELD` and `.GRIND` are the neighbouring
  strings (`0x00782080`, `0x007820a0`, `0x007820b0`). Its thresholds are the severity rule.
- **What sets variable 0 on a craft emitter outside a collision**, and variable 1 on `TURBO`
  (`[1,1,2,0]` once). Search for stores to `emitter + 0x90` and `+0x94`.
- **The eight floats** (request `+0x28`, `handler + 0x48`): float 2 is `1.0` on every race cue and
  zero on speech; float 0 is a ramp on emitter sounds (`SoundEmitterCue_Update` moves record
  `+0x14` toward a target each frame) and a bit test on the player's pad. The reader of
  `handler + 0x48` is not found; [audio-mix.md](audio-mix.md)'s group volumes are the likely
  consumer. Hypothesis, no score.
- **`0x04`'s targets and shapes** (`0x006285f0` and the per-tick modulator update), **the ADSR
  mapping** (`0x006323a0`, descriptor `+0x0a`/`+0x0c`), and the two per-voice stages
  `0x006329e0` and `0x006326c8`: not read.
- **The triggers of row 6's cues.** `BOMBEXPL` is in [weapons.md](weapons.md)
  (`NormalBomb_Detonate`); the rest need their string references chased.
- **Zone, Detonator and Eliminator weapon banks**: `weapons_zone.bnk`, `weapons_det.bnk` and
  `weapons_nitro.bnk` each carry their own `SPEEDUPPAD` (same list). Which a mode loads is not
  read; this engine always reads `weapons.bnk`.
- **2048**: see [Lineage](#lineage).

## Wired

2026-11-15, lane `hd-cue-walk`. `WalkModel::cue_variables` (`crates/formats/src/sblk/timeline.rs`) walks a cue with the
table's operand bytes and four variables; `SequenceTick::walks_cue_variables` (the PS3 tick) selects it for the one-shot
hud, ship and weapons cues (`CueTables::walks_variables`; held cues and `~ENGINE` keep their waveform, except the two that run as lists, below). Pulse, Pure and
2048 keep the walk they had.

- **Perspective** (`CueTables::variables(player)`): another craft's emitter 2; the player's craft 1 (chase view) where
  the cue rides it; 0 for the cues the original plays unplaced from the player's seat (`SPEEDUPPAD`, `TURBO`) and for a
  projectile's own emitter and everything it hits (`ROCKET`, `MISSILE`, `PLASMA`, the explosions), which the live table
  measured as `[0,0,0,0]`. `Banks` builds both seats' timelines and `voices_for` picks; both or neither. The cockpit
  view (0 in the original) is not read; `ABSORB`, `SHURIKEN` and the player's `CANNON` have no capture of their own. All
  **chosen**, the rest measured.
- **Severity and surface stay 0**, and `0x08` stays unwalked in the race model, so `.COLLISIONS` keeps its flat pick
  (severity's source, `FUN_000d8bd0`, is unread). The walk itself reaches `c_CShipWall`'s first block with `[0,1,0,0]`
  (unit test `collisions_ship_or_wall_follows_variable_one_*`).
- **Rear half of the pan law**: a grain at 90 to 270 degrees pans at `sin(angle)` (`pan_of_angle_folded`). Pulse's
  `Scream_PanVolumePair` rear branch reads the table at `a' - 180` with the pair swapped and one term negated; the two
  gains are then `pan_gains` at `sin(angle)` exactly (unit test), and the negation is a phase inversion dropped here
  (**chosen**). That law is read on Pulse's executable; HD's own pan table is not.
- **Combinations**: a cue's alternate groups are independent draws (`ROCKEXPLSHIP` seven groups, 46,080 combinations,
  `SHURIKENEXPL` seventeen). Past 64 the loader samples 64 timelines from a seed (`Bank::cue_timelines_sampled`) and a
  play picks one at random.
- **Ground truth** (Fury disc, `crates/raceplay/tests/sfx_timeline_ground_truth.rs`, one test
  `hd_s_race_cues_play_the_lists_the_bank_authors` loading the banks once and running named checks):
  `hd_s_speedup_pad_is_a_body_and_two_rear_whooshes_for_the_player_and_one_for_others` (player: 3 voices at ticks 0, 0,
  10, angles 0/220/140, one shared bend inside two semitones, left/right levels through the mixer; another craft: 2
  voices, the whoosh at tick 0 or 5) and `hd_s_explosion_and_launch_cues_play_their_layered_voices` (voices per play and
  last tick per cue, the rocket's hard left/right pair, `ABSORB` silent for another craft, and the census: only
  `.COLLISIONS` is flat among the one-shot cues) and `hd_s_plasma_hit_wall_detunes_from_play_to_play` (the 100 % bend).
  The first two were run with `walks_cue_variables` off and both fail (1 voice, not 3; 1 voice, not 10).
- **Census** of `oag-game <hd iso> --race --dry-run` with `RUST_LOG=debug` (51 cue lines): timelines 16 before, 38 after;
  flat picks 35 before, 13 after (`.COLLISIONS`, 11 held cues, speech `cont_elim`).
- **Mix**: layered voices were not dropped. With the mixer's limiter off, the share of audible output samples that hard
  clip, worst of 40 draws: `QUAKELAUNCH` 15.6 % (a timeline before this change too; a single voice 4.1 %), `REPULSOR`
  9.9 %, `CANNONEXPLSHIP` 9.2 %, `SHURIKENEXPL` (17 voices) 4.9 %, `ROCKEXPLSHIP` 4.9 %, `REPULSORHIT` 4.1 %; every other
  cue under 3 %. The game's limiter is on.
- **Not modelled by the timeline walk, so a list that has one keeps its flat pick** (the runner of
  [Held cues run as lists](#held-cues-run-as-lists) does run `0x1c`): `0x1c` (fixed bend), `0x26` (wait for voices), `0x04`
  (modulators), `0x29` (key off), a nonzero `0x1a` wait, and a variable outside the handler's four. None of the one-shot
  cues walked above has one.

**2026-11-16, lane `hd-impact`: the collision family is wired.** What computes the severity and
the surface is read ([collision-cues.md](collision-cues.md)), `.COLLISIONS` no longer keeps a flat
pick for a contact, and `.GRIND`, `.COLLSHIELD` and `.GRINDSHIELD` (row 6 of the audit) play. The
"Severity and surface stay 0" bullet above describes the engine before that.

## Held cues run as lists

2026-11-27, lane `hd-held-race-cues`. `~BLOWUP` and `~ROCKLOCK` (`weapons.bnk` cues 50 and 55; HD has no `hud.bnk`) play
through `oag_formats::sblk::runner` on HD's layout and 240 Hz tick, in place of one flat waveform. The command lists:

```text
~BLOWUP   cue volume 110             ~ROCKLOCK   cue volume 60
01 .. 38   d0    0 deg, vol 110      15           d0
01 .. dc   d0    0 deg, vol 110      22 00 01 00  d0    unless var0 == 0, skip the next
01 .. 80   d0    0 deg, vol 80       1c 00        d50   bend 0
15         d0                        22 00 01 01  d0    unless var0 == 1, skip the next
01 .. 24   d0   90 deg, vol 50       1c 06        d15   bend 6
01 .. c8   d5  270 deg, vol 50       01 .. 84     d0    150 deg, bend range 12/12
01 .. 6c   d0    0 deg, vol 110      01 .. 28     d3    210 deg, bend range 12/12
1a 000000  d40                       16           d0
16         d0
```

| Cue | State | Cadence (240 Hz ticks) | Bend | Confidence |
| --- | --- | --- | --- | --- |
| `~ROCKLOCK` | variable 0 = 0 (seeking) | a pair every 53 (221 ms); the first beep at tick 50 | 0 | 90 |
| `~ROCKLOCK` | variable 0 = 1 (locked) | a pair every 18 (75 ms): the skipped `1c 00` takes its 50-tick wait with it | `6 * 0x7fff / 0x7f` = 1548, +0.567 semitone through 12/12 | 90 |
| `~BLOWUP` | none | three voices at the start, then the 90/270 pair (270 at +5) and a centred hit every 45 | none | 88 |

The tick and the stepper are the ones [sound.md](sound.md) reads (`Scream_TickCommandList` `0x006253d0`,
`Scream_StepCommandList` `0x00625218`), with the same countdown, the same extra tick from a second `0x16` in a tick and the
same end of list as Pulse's. Read this pass:

- `Scream_DoGrainSetBend` (`0x006242b0`): `a < 0` gives `a << 8`, else `(a * 0x8000 - a) / 0x7f`, to
  `Scream_SetVoiceBend` on the handler. The bend reaches the voices already keyed.
- `Scream_DoGrainAlternate` (`0x00625e88`): `rand16 % a`, advanced by one when it equals the last pick (kept in the
  bank's own command byte `c`); `stride` commands run, then the rest of the group is skipped. A group opened inside one
  stops the handler. The runner keeps `c` per play, seeded from the bank (**chosen**; no held cue wired here has a group).
- `Scream_DoGrainGoto` (`0x006255f8`): the first `0x23` with `a` equal to the id; a ninth goto in one tick stays put
  and waits one more tick; no match stops the handler. The depth counter is a global (`*0x008c0038 + 0x20`)
  whose reset is unread; the runner counts per handler per tick (**chosen**; neither wired cue has a goto).
- `Scream_SetCueParameter` (`0x00624418`, behind the thunk `0x006774c8` -> `0x0062b2d0`) writes `handler + 0xa4 + index`
  for index 0 to 3, the bytes the guard reads. Confidence 90.
- `HudSight_UpdateTone` (`0x00087fa8`) starts `~ROCKLOCK` with `Sound_PlayBankCue(..., 0x400, 0, 0, 0, 0)` (no variable
  block, so all four read 0), writes variable 0 each pass (state `1` writes 0, state `2` writes 1) and stops the handle at
  state 0. State `2` as "locked" is read off the list it selects, not off the state's writer. Confidence 80.

**`~BLOWUP` has no trigger on HD.** The executable holds no `BLOWUP` string, and HD's destroyed craft plays
`EXPLBIG`, `~SHIP_DESTROYED` and `~SHIP_ONFIRE` ([ship-destroy.md](ship-destroy.md), "Sound on HD"). The program is built
and tested, and nothing starts it.

**Not run against RPCS3.** The cadences are read from the bank and the handlers, not heard on the original.

Wired: `Bank::cue_runner_modelled` (`WalkModel::cue_variables`), `oag_sound::sfx::layers::program` on the PS3 tick
(rear angles pan as `sin(angle)`), `repeating::Playing` (the handler's bend to every live voice). It refuses a list with
`0x04`, `0x05`, `0x1b`, `0x26`, `0x29` or a global variable. Tests: `crates/formats/src/sblk/runner/tests.rs`,
`hd_sound_ground_truth::hd_s_blowup_and_rocklock_run_their_lists`,
`sfx_repeating_ground_truth::hd_s_*`.

## Lineage

- **Held cues, 2048: checked, applies, not wired.** The Vita `weapons.bnk` ships `~BLOWUP` and `~ROCKLOCK` with the same
  command shapes, waits (50, 15, 40) and bend 6, with Pulse's operand byte order (`22 00 01 01`'s threshold in the
  high byte). Its `0x1c` and guard handlers are unread, so the PS3 layout is not borrowed for it.
- **Held cues, Omega: checked, differs.** `_NGP_ShipDeath` and `_NGP_MissileLock` are Wwise events.
- **2048: checked, applies in shape, not wired.** `weapons.bnk` (79 cues) and `shipHD.bnk` (100 cues) carry `0x22` guards
  too (28 and 58 commands, run through the Vita walk), among `0x04`, `0x1a`, `0x1b`, `0x1c` and `0x29`. Their handlers are
  the Vita ones (`WalkModel::vita`: marker ids in the low byte, `0x2e` for the random goto): the guard's byte positions and
  which variable is the perspective are unread, and 2048's tick is the Vita race build's (`SequenceTick::Vita` is wired for
  the countdown only). It keeps the flat pick; the walk here is HD's, not borrowed for it.
- **Omega: checked, differs.** Its banks are Wwise: a cue's perspective is a separate authored event (`<cue>_PC__<bank>`,
  the player's version, already played through `Banks::player_timelines`) and switch containers, not variables in a list.
  The `variables` table does not apply.
