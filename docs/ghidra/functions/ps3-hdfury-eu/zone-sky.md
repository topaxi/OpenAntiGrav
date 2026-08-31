# HD/Fury's Zone sky is a **file swap**, and the gate that does it names four game modes

`EBOOT.elf` (Wipeout HD/Fury, PS3), image base `0x00000000`. Static reading
throughout, so every score here is capped at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

**The question this page answers.** From the maintainer, playing HD/Fury: *"the
skybox looks visibly different in zone races, almost like solid colours or
gradients."* Two mechanisms were candidates - a gradient built from the
`Sky horizon colour` / `Sky zenith colour` keys the Zone stage table
cross-fades, or a different cubemap - and
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s own next step
insisted the answer be read rather than guessed, because a horizon-to-zenith
lerp is exactly the kind of legible invention `CLAUDE.md` rules out.

**Both are real, and the first one is the one a port can draw today.** A Zone
race loads `Data/Tex/ZoneSky.gtf` **instead of** the circuit's own `sky.gtf`,
through the same call, into the same handle, under the same sampler-state
patch. The gradient exists too, on the same gate, but its draw is unread - see
[What is not established](#what-is-not-established).

## The gate byte: `g_ZoneEffectsActive` at `0x00d45f84`

Not a new address. `zone-effectsettings-loader.md`'s twenty-third pass already
found `Scene_PrepareFrame` testing the byte at `0x00d45f84` at `0x003ab65c` and
skipping shader parameters 52-67 when it is zero. **Its writer is found here**,
and it is `Environment_LoadRaceScene` (`0x003f3fb0`) - the same per-race
environment loader that reads `ZoneMode.effectSettings`, `sky.gtf` and
`skycube`:

```text
003f4044  lwz  r11, -0x52C4(r2)   ; TOC 0x008bd3c4 -> 0x008b8100 -> g_GameState
003f404c  lwz  r9,  0xe0(r11)     ; the mode id
003f4050  cmplwi r9, 21
003f4054  bt   29, 0x3f4150       ; > 21: clear the byte
003f4058  li   r10, 1
003f405c  lis  r0, 32
003f4060  sld  r9, r10, r9        ; 1 << mode
003f4064  ori  r0, r0, 24640      ; r0 = 0x00206040
003f4068  and  r8, r9, r0
003f406c  cmpdi r8, 0
003f4070  bt   30, 0x3f4150       ; not in the mask: clear the byte
...
003f4078  lwz  r25, -0x52A0(r2)   ; -> 0x008b8124 -> 0x00d45f84
003f4080  stb  r10, 0(r25)        ; set

003f4150  lwz  r25, -0x52A0(r2)
003f4154  li   r0, 0
003f4158  stb  r0, 0(r25)         ; clear, on every other path
```

So the byte is **`(1 << mode) & 0x00206040 != 0`**, and the mask names exactly
four mode ids: **6, 13, 14 and 21**. Confidence **82** on the byte's meaning -
the arithmetic is direct, the two writers are the only ones in the image, and
`0x00936fe8 + 0xe0` is the mode field [mode-manager.md](mode-manager.md)
identified from the executable's own `GetMode()==%d` format string.

### This settles a hypothesis `mode-manager.md` left open

That page enumerated 22 mode ids and found seven - `0`, `6`, `7`, `11`, `13`,
`14`, `15` - with no `ModeManager` subclass of their own. It wrote:

> the loading screen distinguishes 6, 13 and 14 from each other, so they are
> real modes. The likely reading is that they are the ones with a `RaceManager`
> subclass and no `ModeManager` of their own [...] Zone, Zone Battle and
> Detonator are the obvious candidates. **Not established.**

This gate mask is independent evidence for exactly that: **three of the four
modes that switch on the whole Zone/Detonator effect block are 6, 13 and 14**,
the three that page singled out. And one of them is already pinned -
`zone-effectsettings-loader.md`'s tenth pass read mode `0xe` (14) as
**Detonator** off a different branch of this same loader. So 6 and 13 are Zone
and Zone Battle, in an order this page does **not** claim to know.

**The fourth id, 21, does not fit that story and is recorded rather than
explained.** `mode-manager.md` puts 21 with `MPArcade`, alongside 16 and 18.
The reading that costs nothing is that 21 is the online form of one of the two,
sharing `MPArcade`'s manager the way 16/18/21 already share it - but that is a
hypothesis (confidence 40), and the mask is a fact.

## The sky branch itself

`Environment_LoadRaceScene` tests the same byte again a page later and takes
one of two branches:

```text
003f426c  cmpwi 7, r0, 0          ; r0 = *(0x00d45f84), loaded at 003f4268
003f4270  bf    30, 0x3f4e24      ; non-zero -> the Zone sky

; the Zone branch
003f4e2c  lwz  r16, -0x5288(r2)   ; -> 0x008b813c
003f4e30  lwz  r4,  -0x5290(r2)   ; -> 0x008b8134 -> "Data/Tex/ZoneSky.gtf"
003f4e38  lwz  r5,  -0x528C(r2)   ; -> 0x008b8138 -> "skycube"
003f4e3c  addi r3, r1, 168
003f4e44  bl   0x5dde98
003f4e68  lwz  r17, -0x5310(r2)   ; -> 0x008b80b4 -> 0x00d42c60
003f4e70  stw  r31, 0x1014(r17)   ; the handle
003f4e98  b    0x3f441c           ; join

; the circuit branch, reached by falling through at 003f4274
003f43a4  lwz  r16, -0x5288(r2)
003f43ac  ...                     ; r4 = a path built from the circuit's own
003f43b0  clrldi r4, r29, 32      ;      directory plus "\" and "sky.gtf"
003f43bc  bl   0x5dde98           ; the same loader
003f43e8  stw  r31, 0x1014(r17)   ; the same handle slot
003f441c  ...                     ; the same sampler-state patch
```

**The convergence is what makes this a swap rather than a variant.** Same
loader call, same `"skycube"` tag, same handle slot `*(0x008b80b4) + 0x1014`,
same `rlwinm`/`ori` patch of the texture-control words at `+0x20`. Nothing
downstream of `0x3f441c` can tell the two apart, so the *picture* is the only
thing that changes. Confidence **84**.

**And `Environment_LoadRaceScene` is the only sky loader in the image.**
`scripts/ps3-toc.py attrib` on all three strings - `"Data/Tex/ZoneSky.gtf"`
(`0x007b3ef8`), `"skycube"` (`0x007b3f10`) and `"sky.gtf"` (`0x007b3f28`) -
returns `0x003f3fb0` and nothing else. **And the per-hit OPD check this page
warns about below was actually run**: a byte sweep for `lwz rX,-0x5290(r2)` -
the displacement that reaches the `ZoneSky.gtf` string's own slot - finds three
instructions, but `0x000b70b8` and `0x000b7d7c` carry the *other* TOC
(`0x008ad4d8`), where the same displacement resolves to `0x008a8248`, an
unrelated slot. So `0x003f4e30` is the only instruction in the image that ever
names this file. That is what lets the branch be read as either/or rather than
as one of several routes.

## The file

`/data/tex/zonesky.gtf`, **`DATA02.PSARC`**, 12,416 bytes. Its own header:

| field | value |
| --- | --- |
| textures | 1 |
| format | `0x86` (DXT1) |
| cubemap | 1 |
| mip levels | 1 |
| size | 64x64 |
| payload | 12,288 B = 6 faces x 2,048 B |

Against `01_vineta_k`'s own `sky.gtf` - `2048x2048` DXT1, 12,582,912 B of
payload - that is **1,024 times fewer texels a face**, which is the measured
form of the
maintainer's observation. It is not itself a flat fill: decoded, five of its
six faces carry 445-628 distinct colours in the cyan-teal the Zone palette
works in, and only face 3 (the nadir) is near-flat at 14. Asserted against the
disc by `crates/game/tests/zone_sky_ground_truth.rs`, which races the *same*
circuit in both modes so the mode is the only variable, and leans on the size
rather than on a picture comparison: **no `sky.gtf` on this disc is 64x64**, so
a load that failed to swap cannot satisfy it. **Circuit skies are not all one
size**, which is why the ratio above is quoted against a named circuit rather
than as a disc-wide constant. Every `sky.gtf` on the disc, by payload:

| bytes | shape | environments |
| ---: | --- | --- |
| 262,784 | the odd one out | `zone_3` |
| 3,145,856 | 1024x1024 DXT1, one level | `amphiseum`, `modesto_heights`, `talons_junction`, `tech_de_ra` |
| 12,583,040 | 2048x2048 DXT1, one level | the eight remaining racing circuits, plus `zone_1`, `zone_2`, `zone_4` |
| 16,777,856 | 2048x2048 DXT1 **with mips** | `12_sol_2` |

Same format throughout, two sizes and one mip chain - not, as an earlier
revision of this page had it, one payload in two formats.

**There is exactly one `zonesky.gtf` on the disc.** All seven archives were
listed and counted rather than searched with a silenced error - `DATA00` 3,157
entries, `DATA01` 53, `DATA02` 5,486, `DATA03` 1,163, `DATA04` 54, `DATA05`
151, `DATA06` 1,600 - and `zonesky` matches once, in `DATA02`. The count is
recorded because this thread has already published one census whose every
absolute number had to be retracted for coming off a contaminated extraction
directory.

### Two corrections to the handover thread's premises

- **`zone_3/sky.gtf` is 262,784 B, not 12,583,040.** The claim that all four
  Zone arenas share one cubemap was measured on `zone_1` and `zone_2` (which
  are byte-identical) and `zone_4` (which matches them in size); `zone_3` does
  not. Nothing rests on it any more, but it should not be carried forward.
- **The arenas' own `sky.gtf` are dead art in Zone mode.** The branch above is
  either/or, so `zone_1..zone_4/sky.gtf` are never what a Zone race shows. That
  is the more interesting half of the correction, and it is the reason the
  "all four share one cubemap" measurement never explained the observation.

## What is **not** established

**The gradient.** `Sky horizon colour` and `Sky zenith colour` *are* consumed,
and this page locates the consumer - which the thread's own next-step note
listed as unknown. The four cross-faded vec4s at `0x00c81560`-`0x00c81590` are
loaded together at `0x003adf58`-`0x003adf78`, inside the same function as
`Scene_PrepareFrame`, under **the same gate byte**:

```text
003ad598  lbz  r0, 0(r17)         ; r17 = 0x00d45f84, unchanged since 003aaea0
003ad59c  cmpwi 7, r0, 0
003ad5a0  bf   30, 0x3adf58       ; Zone: the gradient block
003ad5a4  ...                     ; otherwise: the ordinary path
```

Read from there, with confidence **80** because each step is a direct decode:

- all four are scaled by a common constant from `0x007b0e40` (six components:
  horizon and zenith, each in its inner and outer form);
- inner and outer are then **lerped on the CPU** by one weight `w`, as
  `inner*w + outer*(1 - w)`, `w` fetched from a small indexed record. The
  `1 - w` is read, not assumed: the complement is `fsubs f30, f0, f31` where
  `f0` comes from a GPR round-tripped through the stack (`stw r29,1632(1)` /
  `lfs f0,1632(1)`) and `r29` is set by `lis r29, 16256` at `0x003ad218` -
  `0x3f800000`, exactly `1.0f`. Nothing writes `r29` between there and the
  use on this path, and nothing branches into the range in between, so the
  constant reaches it;
- each result is `fctiwz`'d and packed into a 32-bit word with `0xff` in the
  low byte - two packed colours, one for horizon and one for zenith. **Which
  byte is which channel is deliberately not stated**: the shifts come out
  `<<24`, `<<16`, `<<8` in an order that does not follow the load order, and
  nothing here pins the channel assignment;
- both are passed to `FUN_005ebd58`, which has **exactly one caller in the
  whole image** - this site.

**What that draw does with them is unread, and nothing is drawn on it.**
`FUN_005ebd58` builds a dozen four-float groups on the stack and calls one
helper twenty-odd times; whether that is a dome, a screen-space quad, a fog
term or something else has not been established, and the *shape* is the whole
question a port would need answered. A horizon-to-zenith lerp over a sky dome
is the obvious guess and remains exactly the guess this page exists to avoid
making. `FUN_005ebd58` is deliberately **not renamed** (confidence that it
draws a sky gradient: 55, below the threshold).

Also not established, and unchanged from
[zone-effectsettings-loader.md](zone-effectsettings-loader.md): what advances
the Zone stage at runtime, and therefore what moves the cross-fade this block
consumes.

## Reproduce

```sh
ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
ISO=data/images/hdfury-ps3-eu-dec.iso

python3 scripts/ps3-toc.py attrib 0x007b3ef8      # -> 0x003f3fb0, and only it
python3 scripts/ps3-toc.py u32    0x008b8124      # -> 0x00d45f84
python3 scripts/ps3-toc.py u32    0x008b80b4      # -> 0x00d42c60
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4030 --stop-address=0x3f4170 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4e24 --stop-address=0x3f4ea0 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3adf58 --stop-address=0x3ae1d0 $ELF

python3 scripts/psarc.py list $ISO:PS3_GAME/USRDIR/DATA02.PSARC | grep -i zonesky
python3 scripts/psarc.py cat  $ISO:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/tex/zonesky.gtf | head -c 64 | xxd

OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
    --run-ignored all -E 'binary(zone_sky_ground_truth)'
```

Two traps this page tripped over, both already on
`zone-effectsettings-loader.md` and both worth repeating because they cost time
again here:

- **Per-function TOC.** Every displacement above resolves through
  `Environment_LoadRaceScene`'s own TOC (`0x008bd3c4`), not the entry point's.
  `scripts/ps3-toc.py resolve <func> <hexdisp>` parses its displacement as
  **hex**.
- **Confirm a sweep re-finds a positive before believing its negative.** The
  first sweep for consumers of `0x00c81560`-`0x00c81590` returned nothing *and
  also* returned nothing for `zoneOrigin` at `0x00c81550`, a parameter already
  known to be published. The defect was in the sweep, not in the data: these
  addresses are materialised by `lwz rD,disp(r2)` off per-function TOC slots,
  which `scripts/ps3-toc.py`'s own `scan_toc_loads` handles and a hand-rolled
  register tracker did not. With the control re-found, the four sky slots have
  one loader each.

## Names applied

- `Environment_LoadRaceScene` (`0x003f3fb0`, function, 75) - the per-race
  environment loader: it reads `ZoneMode.effectSettings`, writes the mode gate
  byte, and loads either the circuit's `sky.gtf` or `Data/Tex/ZoneSky.gtf`.
  Independently identified as environment setup by two earlier investigations
  (`docs/formats/envsettings.md`, `zone-effectsettings-loader.md`); this page
  adds the mode gate and the sky branch. 75 rather than higher because the
  function is large and only these parts of it have been read.
- `g_ZoneEffectsActive` (`0x00d45f84`, data, 80) - the byte that switches on
  HD's Zone/Detonator scene effects: set for mode ids 6, 13, 14 and 21 and
  cleared otherwise, read by the sky branch here and by `Scene_PrepareFrame`
  twice.

## See also

- [zone-effectsettings-loader.md](zone-effectsettings-loader.md) - the stage
  table, the parameter publication idiom, and the cross-fade that fills
  `0x00c81560`-`0x00c81590`.
- [zone-shader.md](zone-shader.md) - the surface equation the same gate turns on.
- [mode-manager.md](mode-manager.md) - the mode enum this page's mask indexes.
- [../../../formats/effectsettings.md](../../../formats/effectsettings.md) - the
  file's own keys, `Sky horizon colour` and `Sky zenith colour` among them.
