# Detonator's HUD: what each widget reads

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Static reading, 2026-11-22: nothing here
ran. Every `lwz rN,disp(r2)` was resolved with `scripts/ps3-toc.py resolve <function> -0x<disp>`
(all of these functions declare TOC `0x008ad4d8`, `ps3-toc.py toc` prints `exact` or `inherited`);
store/load sites by `scripts/ps3-dform-scan.py`; bodies Ghidra truncates by `llvm-objdump -d
--mattr=+altivec,+vsx`. Read first: [detonator-cannon.md](detonator-cannon.md) (the score, clip and
streak laws these widgets display), [hud-readouts.md](hud-readouts.md) (`DamageBar`,
`ShieldBarText`), [hud-message-triggers.md](hud-message-triggers.md), and
[hd-hud.md](../../../formats/hd-hud.md) (how the XML composes).

## The answer in one table

The layout is `/data/xml/detonator_hud.xml` (DATA00, path literal `Data\XML\Detonator_HUD.xml`,
TOC `-0x5fbc` in `0x00081068`: that is what ties the controller below to this file) with
`HUD_DetonatorWeapons.xml`, `Detonator_warnings.xml`, `HUD_scoring_feedback.xml` and the generic
`HUD_colours/ready_go/info_text/time_diff` fragments. The Detonator controller is one object:
`0x00082138` (EMP half) calls `0x00081958` (weapon-display half) calls `0x00081068` (base); each
finds its widgets by name with `0x0016d9a8(hud, name, 1)` and files the pointer at the slot shown.
Its two per-tick updaters are vtable entries (`0x00862e3c` -> `0x00080830`, `0x00862e7c` ->
`0x000803d8`).

| XML widget | Slot | Binder | Value it shows | Format | Updater | Conf |
| --- | --- | --- | --- | --- | --- | ---: |
| `DetonatorScore` (Text) | `+0x44` | `0x00081068` | score `rm+0x2df0` (through `RaceManager`'s vtable `+0x3c`, arg `0`, record `+0x14`) | `%07d`, opaque white | `0x0007f4b0` | 78 |
| `Bullet00`..`Bullet14` (Image) | `+0x48`..`+0x80`, entry `{width, widget, state, .., flag}` | `0x00081068` (`Bullet%02d`) | one icon per volley of the 15-volley clip; `Bullet(14-j)` goes to state 3 (spent) when the clip record's word `+0x80+4j` is set | sprite state | `0x0007e950` | 60 |
| `Multi00`..`Multi14` (Image) | `+0x88`..`+0xc0` | `0x00081958` (`Multi%02d`) | a pip per volley: `Multi(14-i)` lights (latch byte `this+0xc4+i`, entry flag `+0x10`) once volley `i` has hit (`hits[2i] > 1 or hits[2i+1] > 1`); a streak award recolours its last 5/10/15 pips `0xff00eeff` | pip fade `+0xf0..+0xfc` timers | `0x0007e950`, `0x0007e6d0`, fade in `0x000803d8` | 65 |
| `PulseBullets` (Text) | `+0xd4` | `0x00081958` | the player's clip count `ship+0x164` (0..15) | `%02d` | `0x000803d8` | 80 |
| `PulseBonus` (Text) | `+0xec` | `0x00081958` | a streak award line, set by `Hud_RaiseChainBonus` (`0x0007e6d0`): the localised `HUD_CHAIN_5/10/15` text and the amount (1000/3000/5000), shown 2.0 s then faded over 1.0 s (`+0xf8 = 2.0`, `+0xfc = 1.0`) | `%s - %d` | `0x0007e6d0`, fade `0x000803d8` | 80 |
| `DetonatorStage` (Text) | `+0xe4` | `0x00081958` | HUD stage `rm+0x2dee` (u16, starts at 1, +1 per start-line crossing, capped 14) over the constant `14` | `%d/%d` | `0x000803d8` | 72 |
| `DetonatorStageParent`, `BulletBack`, `EMPBack`, `DamageBarBg` frames (Image) | none | - | static art | - | - | 85 |
| `Emp_0`..`Emp_7` (Image) | `+0x120`..`+0x13c` | `0x00082138` (`Emp_%d`) | pip `i` white when the EMP charge `rm+0x2dd8` is `> i`, else vertex colour 0 | `Image_SetVertexColours` | `0x00080830` | 75 |
| `EmpReady` (Image) | `+0x140` | `0x00082138` | shown (`+0x34` bit 4) when the charge reaches `table+0x1bc` (the craft's `EmpStats` threshold) and not during the post-fire flash `+0x174` | visibility | `0x00080830` | 65 |
| `EmpBarEffect` (ImageModel `emp_hud.vex`) | `+0x144` | `0x00082138` | the gauge fraction `min(n * 0.125, 1.0)` (`n = rm+0x2dd8`, so 8 charges fill it) scaled by `0.5` into `+0x148`, and the effect's parameters named `Colour`/`EMP`/`Speed`/`Brightness`/`EMP_Rays` (strings bound in `0x00082138`; their writes unread) | effect constants | `0x00080830` | 50 |
| `DamageBar`, `DamageBarBg`, `ShieldBarText` | generic | `Hud_BindWidgets` `0x0009bb30` | shield (energy) percent, the shared readout, white in mode 14 | `(int)pct`, no `%` | `Hud_UpdateShieldReadout` `0x000866c8` | 82 |
| `ForwardWarningBackground`, `ForwardWarningBombIcon`, `WarningFront*`/`WarningBack*` (Image) | warning slots | `0x0023bb98` | the bomb-ahead warning, HUD message `8` (see below) | visibility + 2.0 s timers | `0x00083318`, table walk `0x0008f9f0`-`0x0008fb04` | 55 |
| `ScoreFeedBack0`..`15` (Text) | `hud+0x3d0 + 0xc*k` | `0x0009bb30` | a floating `+N` for every score add: `DetonatorScoring_Add*` -> `Hud_RaiseScoreFeedback` (`0x00083ba0`) | `+%3.0f` (`+%3.2f` under 1.0) | `0x00083ba0` | 75 |
| `DetonatorGoldMedal`/`Silver`/`Bronze` (ImageModel, `hd_gold/silver/bronze.vex`, `StartEnabled=false`) | not bound by this controller | - | shown when the score passes the cell's `DetonatorGoldMedal`/`Silver`/`Bronze` thresholds, in `Detonator_UpdateHud` ([detonator-cannon.md](detonator-cannon.md)); wired, see "The medals" | model | `0x00067120` | 65 |
| `HUD_info_text`, `HUD_time_diff`, `HUD_ready_go`, `HUD_colours` fragments | generic | generic binders | the shared message lines, lap-line gap and start gate; what Detonator raises into them is unread | - | `Hud_Update` | 60 |
| `PulseScore`, `PulseMultiplier` (Text) | `+0xd8`, `+0xdc` | **none** | nothing: see below | - | - | 75 |
| `EmpScore` (Text) | `+0xe0` | `0x00081958` | nothing: bound, never read or written again | - | - | 70 |
| `PulseBankedScore` (Text) | `+0xe8` | `0x00081958` | nothing visible: its text setter `0x0007e5a8` has no caller | `%d` | fade only | 65 |

## `PulseScore` and `PulseMultiplier` are dead on HD (75)

`0x000803d8` writes `%04d` of `g_DetonatorScoring+0x24` into the widget at `+0xd8` and `x%02d` of
`int(g_DetonatorScoring+0x28)` (`m_mine`) into the one at `+0xdc`, each only when the slot is
non-null (`0x0008045c`, `0x000804b0`). Nothing fills those two slots:

- the only stores to `+0xd8`/`+0xdc` anywhere in `0x7e000`-`0x84000` are `0x7edb8`/`0x7edbc`,
  `0x7ee80`/`0x7ee84`, `0x80e80`/`0x80e84` and `0x80f58`/`0x80f5c`, all `stw r0` (zero) in the
  constructors and the zeroing path (`ps3-dform-scan.py ... --disp 0xd8,0xdc`);
- the binders `0x00081068`, `0x00081958`, `0x00082138` store `+0x44`, `+0x48..+0x80`, `+0x88..+0xc0`,
  `+0xd4`, `+0xe0`, `+0xe4`, `+0xe8`, `+0xec`, `+0x140`, `+0x144`, `+0x120..+0x13c` and nothing else;
- the executable carries no string `PulseScore`; `PulseMultiplier` appears once, as an attribute
  name in the generic widget `Values` parser (`0x00201470`, TOC slot `+0x2124`), not as a lookup.

So the XML's `PulseScore` and `PulseMultiplier` Text widgets stay empty and **the `x%02d`
multiplier and the `%04d` counter are never drawn on HD**. This corrects
[detonator-cannon.md](detonator-cannon.md)'s line "The HUD (`0x000803d8`) shows `m_mine` as `x%02d`
and `+0x24` as `%04d`": the code formats them but has no widget to put them in. 75 and not higher
because the scan covers the controller's own range; a store through a different base register from
outside it is not excluded. The same holds for `EmpScore` (bound at `+0xe0`, no later reader), and
`PulseBankedScore` (`+0xe8`) takes only a fade: its text is set by `0x0007e5a8`, which no direct
branch or vtable entry reaches (`ps3-dform-scan.py --target 0x7e5a8` is empty, and the OPD entry
`0x00873128` is referenced from nowhere).

## The names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00081068` | `DetonatorHud_LoadAndBindScore` | 75 |
| `0x00081958` | `DetonatorHud_BindWeaponWidgets` | 75 |
| `0x00082138` | `DetonatorHud_BindEmpWidgets` | 75 |
| `0x000803d8` | `DetonatorHud_UpdateWeaponTexts` | 68 |
| `0x00080830` | `DetonatorHud_UpdateEmp` | 68 |
| `0x0007f4b0` | `DetonatorHud_UpdateScoreText` | 65 |
| `0x0007e950` | `DetonatorHud_UpdateVolleyIcons` | 62 |
| `0x0007e498` | `DetonatorHud_ResetVolleyIcons` | 70 |
| `0x0007e5a8` | `DetonatorHud_ShowBankedScore` | 62 |
| `0x00083318` | `Hud_RaiseWarning` | 65 |
| `0x0008caf0` | `Hud_UpdateScoreFeedback` | 72 |

`Hud_RaiseChainBonus`, `Hud_RaiseScoreFeedback`, `Detonator_UpdateHud` and `Detonator_IsBombAhead`
keep the names their pages gave them. The three binders sit at 75: each is a run of
name-string lookups into the one HUD definition, and two independent readings (the TOC-resolved
strings, and the Ghidra decompile's offsets) agree on every slot. The updaters are 62-68: the
formats and sources are TOC-resolved, the visual states are partly unread (below).

## The updaters, line by line

### `0x000803d8` (weapon texts), `f1 = dt`

```text
ship := RaceManager_GetInstance()->+0x13e8 -> +0x5edc            ; 0x00080420-0x00080434
text(+0xd4, "%02d", ship+0x164)                                   ; the clip count, PulseBullets
if +0xd8: text(+0xd8, "%04d", int(g_DetonatorScoring+0x24))       ; never non-null
if +0xdc: text(+0xdc, "x%02d", int(g_DetonatorScoring+0x28))      ; never non-null
for i in 0..14: Multi pip entry (this+0x88+4i): fade step           ; 0x00080520-0x000805dc
text(+0xe4, "%d/%d", rm_vtable+0x3c(0)->+0x12 (u16), 14)          ; DetonatorStage, 0x000805e0-0x00080644
if +0xf0 > 0: +0xf0 -= dt; +0xe8 colour = -1 (PulseBankedScore holds)
   elif +0xf4 > 0: +0xf4 -= dt; +0xe8 alpha = +0xf4 * 255 (fades over 1 s)
likewise +0xf8/+0xfc for +0xec (PulseBonus)                       ; 0x00080680-0x00080788
```

`PulseBullets` prints the raw `ship+0x164`. The reload refills that to 15 at the **start** of the
lockout ([detonator-cannon.md](detonator-cannon.md), "The reload"), so during the 0.75 s lockout the
widget already reads `15`: the reload gauge `(1 - s+0x178 / s+0x198) * 15` that `Detonator_UpdateHud`
computes does **not** feed a widget in this controller. It is passed with the "locked" flag to
`0x0029d578` (`count, flag`), which stores `count * k` (`DAT_008b2e70`) into a shader constant block
at `PTR_DAT_008b2e60+0x24` and a mask vector at `+0x30`: the ship's own
`detonator_bulletgaugesegments` material (DATA00 `/data/materials/ships/`), i.e. an ammo gauge drawn
on the craft model, not on the HUD. Read, confirmed live and wired on
[detonator-ammo-gauge.md](detonator-ammo-gauge.md) (`0x0029d578` is `DetonatorGauge_SetBullets`, 88).

### `0x00080830` (EMP half), `f1 = dt`

With `n = rm+0x2dd8` (the EMP charge count, a u32):

```text
this+0x160 = colour_table[clamp(n, 0..8)]                         ; TOC -0x5fe8, 0x00080848-0x00080870
if !this+0x174:                                                    ; not flashing after a fire
    frac = n * 0.125 ; frac = min(frac, 1.0)                       ; TOC -0x5fd0, -0x603c
    this+0x164 = frac ; this+0x148 = frac * 0.5                    ; EmpBarEffect input
    if n < table+0x1bc: EmpReady hidden (+0x34 &= ~4) ; +0x168 = 0.75 ; +0x16c = +0x170 = 0
    else:               EmpReady shown (+0x34 |= 4) ; +0x168 follows a sine of timer +0x16c
                        (rising or holding), +0x170 = last frac     ; shapes unread
else: +0x16c += dt*k ; EmpReady hidden ; +0x168 = sine*k + c ; at the timer limit +0x174 = 0
for i in 0..7: Emp_i vertex colour = 0xffffffff if n > i else 0
```

`rm+0x2dd8 > i` lighting `Emp_i` is the same counter `Detonator_UpdateRace` bumps from the craft's
`0x8000000` flag and reads against `table+0x1bc` ([detonator-mode.md](detonator-mode.md), "EMP
charge", 70): two sites agree on the counter and the threshold. The pulse shapes (`+0x168`'s sine, the post-fire flash `+0x174`, whose writer is unread) and the `EmpBarEffect` shader parameter writes after
`0x00082600` were not read; the colour table at `0x00765b38` was not dumped. 8 charges is the HUD's
own pip count; whether `table+0x1bc` is below 8 is unread (the EMP weapon-table block's `.reload`
reading is on [detonator-mode.md](detonator-mode.md)).

### `0x0007f4b0` (score and the bullet icons)

`text(+0x44, "%07d", (race manager vtable slot 0x3c, argument 0) +0x14)`, colour `0xffffffff`: the zero-padded 7-digit
score. Then every `Bullet` icon entry runs `0x0007f0a0`, and a three-state machine on `this+0x40`
(1 = appearing, 2 = disappearing) staggers each icon's fade by `fVar44 * k` (`k` from TOC
`-0x6028`..`-0x5fc0`). That animation is presentation, unread beyond the state numbers it compares (entry `+0x08`:
2 loaded, 3 spent, 5 and 6 on the way in, 4 and 7 on the way out, 0 and 1 idle; 50, from the comparisons alone).

### `0x0007e950` (`(hud, clip)`), the volley icons

`clip` is the record at `mgr+0x268` ([detonator-cannon.md](detonator-cannon.md), "The streak
bonus"): `hits[30]` words at `+0x08`, followed by 15 words at `+0x80`.

1. For `j` in `0..14`: if the word at `clip+0x80+4j` is non-zero and `Bullet(14-j)`'s state is `2`,
   set it to `3` (`0x0007e950`-`0x0007ea1c`). Volley `j` is spent; the rightmost icon (`Bullet14`,
   `x = 435`) goes first. 55: the 15 words are the per-volley fired mark, inferred from this
   consumer. `PulseCannonManager_FireVolley` (`0x00138978`) indexes `clip + 0x50 + 4k` and
   `clip + 8 + 4*ring`, but no store to `clip+0x80..+0xb8` was found in `0x138000`-`0x14a000`
   or `0x121000`-`0x123000`, so the writer is unread. In the normal case (no manual reload
   mid-clip) the number of spent icons is the number of volleys fired, `15 - clip`.
2. For `i` in `0..14`: if `hits[2i] > 1` or `hits[2i+1] > 1` and the latch byte `this+0xc4+i` is
   clear, set it and set the `Multi(14-i)` entry's flag `+0x10` (`0x0007ea20`-`0x0007ed80`). A hitting
   volley lights its pip; `0x0007e498` (called from `PulseCannonManager_Update` at the post-reload
   tick) clears the 15 latches, resets pips to `-1`/flag 0.

`Hud_RaiseChainBonus(hud, kind, amount, at)` (`0x0007e6d0`): `kind` 1/2/3 = 15/10/5; sets `PulseBonus`'s
text from `HUD_CHAIN_15/10/5` and `"%s - %d"`, recolours the last `kind`-many pips
(`0xff00eeff`), and counts the award in `rm+0x2dea` (tens), `+0x2de8` (fives), `+0x2dec` (fifteens).

## The stage counter

`DetonatorStage` shows `rm+0x2dee` over a literal `14` (`"%d/%d"`, TOC `-0x5fec`, the 14 loaded by
`li r6,0xe` at `0x00080618`). `Detonator_UpdateHud` is the only writer found: on a start-line
crossing it does `rm+0x2dee += 1` capped at `14` (`0x000671e4`-`0x000671fc`, `0x000674e8`-`0x000674f4`),
then the "Hold Your Fire" trophy test at stage `>= 10`. It is a different counter from the stage
step `rm+0x2e10` that `Detonator_UpdateRace` places by ([detonator-placement.md](detonator-placement.md)).
The initial value is **1**: `SPDetonator`'s constructors (`0x00064470`, `0x000649f0`) store the
halfword `1` at `rm+0x2ddc+0x12` (`sth r9,18(r8)` at `0x00064520`, `r9 = 1`, `r8 = rm+0x2ddc`
after the `stwu` at `0x00064500`), next to the zeroing of the neighbouring counters
(`+0x2de0`..`+0x2dec`). So the HUD reads `1/14` from the first tick and counts line crossings from
there, capped at 14: it matches the port's `World::detonator.stages_laid` (1 at the opening stage,
one more per lap, `place_stage` caps at 14). Confidence 72 on the mapping; what raises the
displayed count past the line crossing (the `Detonator_UpdateHud` speed gate) is read.

## The bomb warning is HUD message 8, and it is not the bomb icon (55)

`Detonator_UpdateRace` calls `0x00083318(hud, 8, r)` at `0x00068060` with `r` the **return value of
`0x000588f0` through `Detonator_IsBombAhead`** (`0x00068040`-`0x00068060`: `r5 = r3`), not the craft
([detonator-cannon.md](detonator-cannon.md) had `craft`; the call site shows the register is the
return of the call before it). `0x00083318` takes `(hud, id, direction)`:

```text
if id > 12: return                         ; so id 13 (the Bomb icon's id) can never be raised here
if id in {0,1,2,5,7,10,11,12}:             ; 0x1ca7: weapon-type warnings (rear)
    hud+0x180 = id ; hud+0x178 = 2.0 ; direction timers hud+0x624..0x62c
elif id in {8, 9}:                         ; 0x300: the forward slot
    hud+0x17c = id ; hud+0x174 = 2.0 ; direction timers hud+0x618..0x620
direction: 4 sets all three, 1/2/3 set one
```

The id-to-icon table is at TOC `-0x5cc0` (`0x00920854`), 12-byte `{widget, id, name}` rows. Three
loops (`0x0008f9f0`-`0x0008fa40`, `0x0008fa44`-`0x0008fa98`, `0x0008fab0`-`0x0008faf4`) each walk
**ten** rows from that base (`li r0,10`; `ps3-toc.py resolve 0x8fa48 -0x5cc0` is the base all three
load) and show the row's widget when its id equals `hud+0x17c` or `hud+0x180`. The rows:
**id 8 `ForwardWarningMineIcon`**, 9 `RearWarningCannonIcon`, 5 Rocket, 0 Quake, 2 Plasma,
7 Missile, 1 Repulser, `0xb` Shuriken, `0xc` LeachBeam, and a tenth, id `0xa`, whose name word
`0x00993020` points into `.bss` (not a string; unread). The two rows just **before** the base,
`ForwardWarningBombIcon` (`0x00920848`, id `0xd`) and `RearWarningBackground` (`0x0092083c`,
id `0xd`), are outside what the walk reads, and `0x00083318` refuses `id > 12`, so nothing
raises id 13 through this path. HD's ids are its own numbering, not Pulse's announce indices
([hud-incoming-warnings.md](../psp-pulse-usa/hud-incoming-warnings.md): Mine 9, Bomb 8).
`Detonator_warnings.xml` authors `ForwardWarningBackground` and **`ForwardWarningBombIcon`
only**, and no `ForwardWarningMineIcon` (the generic `hud_warnings.xml` authors both). So a bomb
ahead raises id 8, whose widget does not exist in Detonator's layout: on HD the bomb icon
cannot show through this path, and at most the background plate and the directional
`WarningFront*` / `WarningBack*` arrows would (hypothesis 50; whether the plate follows the slot's
timer is unread). `oag_hud::incoming` already carries the Pulse law (`Icon::Bomb`/`Mine`, the
`500` reach); Detonator's raise is not in it. Recipe: one RPCS3 capture flying toward a bomb (a
bomb within 500 ahead) with `grab_with_clock`, and a look at the strip above the DamageBar.

## 2048 and Omega (checked)

Per-title layout path (`search_strings` on each program): **HD** `Data\XML\Detonator_HUD.xml`;
**Omega** `Data\XML\Detonator_HUD.xml` (its only Detonator HUD string); **2048**
`Data\XML\2048_hud\Detonator_HUD_HD.xml` only, so 2048 never loads the base `Detonator_HUD.xml`
its base package ships. Each title's `Detonator_HUD` entry name is therefore its own:
`oag_2048::hud` must name the `2048_hud` file (today it lists `Data\XML\Detonator_HUD.xml`,
`crates/2048/src/hud.rs:560`), where Omega and HD take the plain path.

- **Omega: checked, applies.** Strings `Bullet%02d`, `DetonatorScore`, `PulseBullets`, `EmpScore`,
  `DetonatorStage`, `PulseBankedScore`, `Multi%02d`, `x%02d`, `Emp_%d` and `ForwardWarning*` are in
  the executable (binder `FUN_01269b40`, update `FUN_01268d10`); `data00`'s `detonator_hud.xml` and
  `hud_detonatorweapons.xml` match HD's except that the weapons file spells `EmpBarEffect`'s
  element **`<aImageModel>`** (a shipped typo), and the executable has **no** `EmpBarEffect` or
  `EmpReady` string, so Omega binds the pips (`Emp_%d`) but, as far as strings show, neither
  (60). Its `detonator_warnings.xml` doubles the `fury_hud.gtf` rectangles (`U=770 V=554
  TxtrWidth=516`, the 2x atlas). Omega's VR patch (`data08`) ships `Detonator_hud_VR*.xml`,
  `hud_detonatorweapons_VR[_Ext].xml`, `detonator_warnings_VR.xml`: not read.
- **2048: checked, differs.** Same score/clip/stage/bonus widget names and the same `%d/%d`,
  `x%02d` strings (binder `FUN_8114ac5c`, update `FUN_8114b3a0`), but a different layout:
  `Detonator_HUD_HD.xml` carries text-only `ScoreTxt`/`DetonatorScore` (`IG_HUD_SCORE`) and
  `DetonatorStageParent`/`DetonatorStage` (`FE_STAGE`) with `HUD_damage_indicator.xml` and the
  three medals at `ImageX 720`, and includes `DetonatorWeapons.xml` (`2048_hud/`, 11424 bytes in
  `data1`, a re-laid 960x540 copy of HD's: `EMPBack` at `offset 60,430`, weapon display at `634,430`; `data2`'s
  8934-byte copy is a third layout with `EmpCharge`/`EmpFull` instead of `Emp_%d`/`EmpReady`). The
  2048 executable binds `EmpBarEffect`, `EmpCharge`, `EmpFull` and has no `Emp_%d`/`EmpReady`
  string: **its EMP readout is a different widget set** (`EmpCharge` = the gauge, `EmpFull` = the
  full cue, hypothesis 60). `Detonator_warnings.xml` is HD's with 480x270 offsets and no `fury_hud` rows
  changed. `PulseMultiplier` is again only a generic `Values` attribute name (`0x81454428`).
- 2048's base-package `Detonator_HUD.xml`, `HUD_DetonatorWeapons.xml` and `Detonator_warnings.xml`
  are byte-identical to HD's (modulo CR) and unreferenced by the executable (no string names them).
- **Outcome:** Omega **checked, applies** (with the `aImageModel` and 2x-atlas differences);
  2048 **checked, differs** (own skin, own EMP widgets). The update bodies of both are not diffed
  against HD's (60 that the shared widgets read the same sources: the strings and formats are the
  same). Wiring: HD and Omega share the `detonator_hud.xml` bindings; 2048 needs its own layout
  entry and an EMP binding for `EmpCharge`/`EmpFull`, or skips EMP until phase 10.

## The floating score: `Hud_RaiseScoreFeedback` and `Hud_UpdateScoreFeedback` (2026-11-26, `det-mine-blast`)

Static read (`EBOOT.elf`; constants by `read_memory`, TOC slots by `scripts/ps3-toc.py resolve`). Each of the sixteen
`ScoreFeedBack0`..`15` slots is 12 bytes at `hud+0x3d0 + 12k`: `+0` the Text widget, `+4` a pointer to a world
position, `+8` a byte, `1` while the slot is free.

`Hud_RaiseScoreFeedback(amount f1, hud, _, at)` (`0x00083ba0`):

```text
k := the first slot whose byte is 1 (free); none free: return               ; 0x00083bb4..
for every other busy slot: if its alpha >= 180 (0x008a7624): alpha *= 0.7 (0x008a7614)
slot[k].at := at                                                             ; the scored object's position
text := amount < 1.0 (0x008a75bc) ? "+%3.2f" : "+%3.0f"                     ; TOC 0x008a7618 / 0x008a761c
colour alpha := 0xff (text and its second colour); slot[k].free := 0
widget+0xb8 = widget+0xbc := amount < 1000.0 ? 1.2 : 2.0                     ; 0x008a7628, 0x008a7620, 0x008a762c
```

`Hud_UpdateScoreFeedback(dt f1, hud)` (`0x0008caf0`), modes 8, 14 and 20 only (`hud+0x6bc`):

```text
k := (1.0 - dt) * 0.995                                                     ; 0x008a75bc, 0x008a77ec
for each slot with a position:
    if alpha < 1.0: free it, hide the widget (+0x34 &= ~4)
    else:
        alpha := alpha * k
        rise  := 5.0 - 5.0 * alpha / 255                                   ; 0x008a77f0 = -1/255, 0x008a7710 = 5.0
        p     := at + rise * (view camera's row at +0x1e0)                 ; *(craft+0x6944), the viewing craft's camera
        widget+0xb0, +0xb4 := 0x00083f48(p)                                ; world to screen
        set the text's alpha to alpha, and the second (border) colour's
```

Both colours are stored as bytes: the update reads the text colour's top byte, frees the slot when it is under
`1`, multiplies, writes `trunc(product)` into `Text_SetColour` and into `0x002005e0` (the second colour), and
takes the rise off the untruncated product (decompiled again 2026-11-27, `det-score-popup`). At one update a
tick the first values are `255, 249, 243, 237, 231`. The raise's dimming also truncates (`255 -> 178`).

So the `+N` stands at the scored object, rises 5 units along the camera's row (`+0x1e0`, read as up: the frame's popup
sits above the blast) as it fades, and fades by a factor `(1 - dt) * 0.995` per update: at one update a tick
`0.978`, about 32 ticks to half (the `cannon-volley` frame's `+100`, about 27 ticks after the hit, reads about half
its full brightness and sits just above the blast, which agrees). Its size is the widget's scale 1.2 under 1000 points, 2.0 from 1000. A
busy slot at alpha 180 or more is dimmed to 0.7 when another is raised. Confidence **72** for the law (two functions
that agree on the slot layout; the fade and the rise against one frame), **55** for `+0x1e0` being the camera's up
and for `0x00083f48` being the plain projection (neither read).

This is the popup's first sighting in a frame ([eliminator-score.md](eliminator-score.md) caught none on Eliminator).
### Wired (2026-11-27, `det-score-popup`)

`oag_hud::ScoreFeedback` holds the sixteen slots; `oag_hud::score_popup` draws each through the layout's own
`ScoreFeedBack%d` widget (`HUD_scoring_feedback.xml`, embedded in `detonator_hud.xml`: font `HUDSmall`,
centred, colour `0x00ffffff`, `BorderColor 0xffff0000`). The amount rides out of the simulation on
`oag_weapons::detonator::Event::points` (`MineShot`, `MineChained`, `BombShot`, `MineEmped`, `BombEmped`; a
streak's `AddFlat` raises none). One update per 60 Hz tick with `dt = 1/60`; the face takes HD's lit-text
`0.8` like every other HUD string, the border does not. Gate: `oag_title::Detonator::floats_score`, HD only.
**Chosen:** the position is copied at the raise (the original keeps a pointer into the object), the camera's
up is the view matrix's second row, and the projection is the reticle's without its 250-unit range test.

Against the `cannon-volley` pair's `+100` (the original's ink spans x 995-1070, y 420-443): ours spans x
998-1073, y 405-427, the same size and colour. The 15 px is the popup's age, not the law: ours' round hits
36 updates before the pair's picture, the original's blast reads about 27. Rendering ours' popup at 27 updates
puts it at the original's height within 3 px (predicted from the rise, no fit). Pair mean grey 10.53 to 10.54;
the box around the popup 17.11 to 22.42 on the published pair, because a popup 15 px off adds its own strokes
to the difference. Moved to the original's rows (the layer alone, PIL greys) the box reads 18.14 without, 10.68
with. The other five Detonator pairs are pixel-identical (none scores).

**2048 and Omega: checked, applies, not wired.** Both author the sixteen widgets (Omega as HD's, scale 1.2;
2048's skin in the `HUD` font at 0.6, same colours), so only the title flag holds them off: neither title's
caller of them is located and no frame of either shows one.

## What stays open

- The `EmpBarEffect` shader constants (`Colour`, `EMP`, `Speed`, `Brightness`, `EMP_Rays`) and the
  colour table at `0x00765b38`: the writes after `0x00082600`.
- The bullet icons' per-state art (`0x0007f0a0`) and what `clip+0x80..0xb8` are.
- ~~The constant block `0x0029d578` writes (the ship's bullet gauge) and whether it is drawn.~~
  Read and wired: [detonator-ammo-gauge.md](detonator-ammo-gauge.md).
- Message 8's icon on HD and the directional arrows' use (above), and the `.bss` tenth row.
- What the generic fragments (`HUD_info_text`, `HUD_time_diff`, `HUD_ready_go`) show in Detonator: generic binders (`Hud_BindWidgets` and its siblings, see [hud-sight.md](hud-sight.md), [hud-message-triggers.md](hud-message-triggers.md)); what `Detonator_UpdateHud` writes into the message word at `*(hud+0x40)+0xf4` is unread (recipe: scan `0x67120`-`0x67d00` for a store at disp `0xf4`).
- The writer of the 15 words at `clip+0x80`.

## Wired

The port draws this layout since the HUD wiring: `oag_title::HudLayouts::detonator` names the root
per title (HD `/data/xml/detonator_hud.xml`, Omega `Data\XML\Detonator_HUD.xml`, 2048
`Data\XML\2048_hud\Detonator_HUD_HD.xml`; both PSP titles keep the time trial's), `oag_hud::detonator`
draws it and `oag_raceplay::detonator_hud` fills it from `World::detonator.cannon` and
`stages_laid`. Test: `crates/raceplay/tests/detonator_hud_ground_truth.rs` (HD, Omega, 2048),
which stands a race on a known cannon and compares the drawn texts exactly.

| Widget | Reads | Note |
| --- | --- | --- |
| `DetonatorScore` `%07d`, `PulseBullets` `%02d`, `DetonatorStage` `%d/14` | `cannon.score`, `cannon.clip`, `stages_laid` (min 1, max 14) | as the table |
| `Bullet00`..`14` | drawn while `index < clip`; a spent icon is hidden | **chosen:** the per-state art (`0x0007f0a0`) is unread, 55 |
| `Multi00`..`14` | `Multi(14-i)` drawn once volley `i` hit; a streak award recolours the last 5/10/15 pips ending at the latest hit volley to `0xff00eeff` | recolour range 60 (read from the `Hud_RaiseChainBonus` description, not the pip loop); an unlit pip is hidden, **chosen** |
| `PulseBonus` | the award's `HUD_CHAIN_n` text and amount (`STREAK_*` of the cannon law), held 2.0 s, faded over 1.0 s | **HD's language table has no `HUD_CHAIN_n`**: the line draws nothing there and the loader report names the ids; Omega's reads `5 CHAIN` |
| `ShieldBarText`, `DamageBar`, `DamageBarBg` | the generic shield readout; white in Detonator (the original's mode-14 test, `hud-readouts.md`) | `DamageBar` is the score box's white stripes, cropped by the shield (`docs/formats/hd-hud.md`, "Detonator's score box") |
| `DetonatorStageParent`, `BulletBack`, `EMPBack` | static art, always on | `EMPBack` is the plate the EMP pips sit on |
| `Emp_0`..`Emp_7`, `EmpReady` | the EMP charge (`RaceSim::detonator_emp`) | a pip drawn while the charge is over its index (vertex colour `0` draws nothing), `EmpReady` once the charge reaches `minNumPadsToCollectEMP`; the post-fire flash and the ready pulse (`+0x168`, `+0x174`) are not modelled |

Not drawn, and named in the loader report: the four widgets the original never binds
(`PulseScore`, `PulseMultiplier`, `EmpScore`, `PulseBankedScore`), `EmpBarEffect` (an
`<ImageModel>`; its gauge is `RaceSim::detonator_emp_gauge`),
`ForwardWarning*`/`Warning*` (the bomb-ahead raise is unread) and, on Omega and 2048, `ScoreFeedBack0`..`15`
(HD draws them: "The floating score").
The reload gauge is the craft's own `detonator_bulletgaugesegments` material
constant, not a HUD widget: untouched.

### The medals (wired, 2026-11-23)

| Piece | What it does |
| --- | --- |
| Reader | `oag_hud::Layout::image_models`: every `<ImageModel>` as `oag_hud::ImageModel` (name, `Src`, `ImageX/Y/Width/Height` with the `<Item>` offsets composed, `ModelX/Y/Z`, `RotX/Y`, `Colour`, `StartEnabled`). Record only: no generic draw, so `EmpBarEffect` stays off. A `<aImageModel>` (Omega's typo) is not one. |
| Targets | `medal_watch::tick` hands a Detonator cell's `targets_for_difficulty(rung)` to the race (`Race::set_medal_targets`, view state, never in `World`); both tick loops (`RaceStage::tick_messages`, the headless capture's `--campaign-cell`) reach it. A race with no cell has none and draws no medal. |
| Law | `oag_raceplay::detonator_hud::medal`: `score >= target` (`evaluate_targets`, flipped: Detonator is a counting mode, the same law the results use, so the HUD cannot promise a medal the results withhold), best tier only. **Chosen:** all three widgets share `ImageX 1440 ImageY 80`, and when each one enables, and whether a lower one stays up under a higher, is not read (the page's row above is 65 on the compare, no more). |
| Model | `oag_game::hud_medal::Medals`: one `oag_game::preview::Preview` (the front end's model preview) per medal, drawn into the widget's rectangle where the HUD's safe area puts it, in `RaceStage::draw_window_hud` and `race_capture::capture`. The spin is the file's own `Anim Transform` (a turn about Y, `LoopEnd 360`); the camera is the preview's orbit with `RotX`/`RotY` as pitch/yaw (**chosen**: the projection `ModelZ -87` sits in is unread). |
| Gate | `oag_title::Detonator::shows_medals`: HD measured `true`, Omega inherited `true` (same layout bytes, the executable carries each medal string once where HD's carries two, so its binder is unread), 2048 `false`. |

The three `.vex` files are PS3 models (geometry in `hd_*.rcsmodel` beside them, `DATA02`); they now
decode (1,480 triangles each). The front-end trophies author the node's box 0.01x the vertices
(`docs/formats/rcsmodel.md`, "Inline chunks authored in other units"), which dropped the one
submesh as a stray before. **Open: the tiers' colour.** The three share `medal.rcsmaterial` and differ
in the model's own parameters (diffuse `0x0fe400f9`: gold `0.178 0.130 0`, silver `0.252` grey, bronze
`0.199 0.075 0`; specular colour `0x370a63cb`: `1 0.91 0.41`, `0.706 0.849 1`, `1 0.913 0.431`;
exponent `0x64341a19`: 25, 25, 73.36). The program is a lit sum over a normal map plus a
reflection of the paraboloid probe on unit 1 (`scripts/ps3-microcode.py fp-file`, block 0), which the
mesh reader classes as the glint-water family, so a preview draws it as a plain grey metal and
the three tiers look the same. The tier is the model drawn, not its tint, until that is read.
Picture: a frame of a campaign cell (`grid8_4_3`) at the cell's Medium targets with the score forced
(`--force-detonator-score 600:180000`) shows the hexagonal trophy at the widget's rectangle;
without a cell or below bronze nothing draws.

- **Omega: ported.** Same test (the medals too: `Detonator_HUD.xml` authors the same three at the same values, tested); the EMP pips draw as HD's; `EmpReady`, which Omega's layout authors but
  its executable names no string for (so probably never binds, 60, above), is dropped from the
  layout (`oag_title::Detonator::binds_emp_ready`, the loader report says so); `<aImageModel>` on `EmpBarEffect` composes with nothing skipped
  and no error (the layout reader models no `<ImageModel>` at all, spelled right or not; the
  test asserts the report has no `HUD: skipped` line), and the
  Omega atlas (`detonator_hud2.tga` 1024x1024 beside a 2048x2048 `fury_hud.gtf`) resolves for every
  sprite checked. Omega's `LAYOUTS_2048` (a 2048-lineage circuit raced as Detonator) names the bare
  Omega root too, since the executable names no other.
- **2048: checked, differs, medals left.** Its skin authors the three at `ImageX 720` on its 960x540 grid; `shows_medals` is `false` there (the brief for this change excluded it).
- **2048: checked, differs, wired to its own layout.** `Detonator_HUD_HD.xml` composes from nine
  files; the `DetonatorWeapons.xml` the archive order serves is the 960x540 copy with
  `EmpCharge`/`EmpFull` (the widgets the executable binds), `DetonatorWeaponDisplay` frame and the
  `Bullet`/`Multi` arcs. The score, clip and stage widgets read the same cannon (same test, 2048
  package). The `EmpCharge`/`EmpFull` gauge waits on phase 10 like HD's.
- **No reference frame.** `data/reference/` holds no Detonator capture, so the picture is judged
  against the layout's own geometry, not a frame of the original: a capture recipe is under "The
  bomb warning" above.
