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
| `DetonatorStage` (Text) | `+0xe4` | `0x00081958` | HUD stage `rm+0x2dee` (u16) over the constant `14` | `%d/%d` | `0x000803d8` | 70 |
| `DetonatorStageParent`, `BulletBack`, `EMPBack`, `DamageBarBg` frames (Image) | none | - | static art | - | - | 85 |
| `Emp_0`..`Emp_7` (Image) | `+0x120`..`+0x13c` | `0x00082138` (`Emp_%d`) | pip `i` white when the EMP charge `rm+0x2dd8` is `> i`, else vertex colour 0 | `Image_SetVertexColours` | `0x00080830` | 75 |
| `EmpReady` (Image) | `+0x140` | `0x00082138` | shown (`+0x34` bit 4) when the charge reaches `table+0x1bc` (the craft's `EmpStats` threshold) and not during the post-fire flash `+0x174` | visibility | `0x00080830` | 65 |
| `EmpBarEffect` (ImageModel `emp_hud.vex`) | `+0x144` | `0x00082138` | the gauge fraction `min(n * 0.125, 1.0)` (`n = rm+0x2dd8`, so 8 charges fill it) scaled by `0.5` into `+0x148`, and the effect's parameters named `Colour`/`EMP`/`Speed`/`Brightness`/`EMP_Rays` (strings bound in `0x00082138`; their writes unread) | effect constants | `0x00080830` | 50 |
| `DamageBar`, `DamageBarBg`, `ShieldBarText` | generic | `Hud_BindWidgets` `0x0009bb30` | shield (energy) percent, the shared readout, white in mode 14 | `(int)pct`, no `%` | `Hud_UpdateShieldReadout` `0x000866c8` | 82 |
| `ForwardWarningBackground`, `ForwardWarningBombIcon`, `WarningFront*`/`WarningBack*` (Image) | warning slots | `0x0023bb98` | the bomb-ahead warning, HUD message `8` (see below) | visibility + 2.0 s timers | `0x00083318`, table walk `0x0008f9f0`-`0x0008fb04` | 55 |
| `ScoreFeedBack0`..`15` (Text) | `hud+0x3d0 + 0xc*k` | `0x0009bb30` | a floating `+N` for every score add: `DetonatorScoring_Add*` -> `Hud_RaiseScoreFeedback` (`0x00083ba0`) | `+%3.0f` (`+%3.2f` under 1.0) | `0x00083ba0` | 75 |
| `DetonatorGoldMedal`/`Silver`/`Bronze` (ImageModel, `hd_gold/silver/bronze.vex`, `StartEnabled=false`) | not bound by this controller | - | shown when the score passes the cell's `DetonatorGoldMedal`/`Silver`/`Bronze` thresholds, in `Detonator_UpdateHud` ([detonator-cannon.md](detonator-cannon.md)) | model | `0x00067120` | 65 |
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
| `0x00081068` | `DetonatorHud_Construct` | 75 |
| `0x00081958` | `DetonatorHud_BindWeaponWidgets` | 75 |
| `0x00082138` | `DetonatorHud_BindEmpWidgets` | 75 |
| `0x000803d8` | `DetonatorHud_UpdateWeaponTexts` | 68 |
| `0x00080830` | `DetonatorHud_UpdateEmp` | 68 |
| `0x0007f4b0` | `DetonatorHud_UpdateScoreText` | 65 |
| `0x0007e950` | `DetonatorHud_UpdateVolleyIcons` | 62 |
| `0x0007e498` | `DetonatorHud_ResetVolleyIcons` | 70 |
| `0x0007e5a8` | `DetonatorHud_ShowBankedScore` | 62 |
| `0x00083318` | `Hud_RaiseWarning` | 65 |

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
on the craft model, not on the HUD (hypothesis, 55: the constant block's owner is unread).

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
   `x = 435`) goes first. 60: the 15 words are the per-volley fired mark, inferred from this
   consumer (no writer was read).
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
The initial value is not stored by anything in `0x60000`-`0x6e000`, so it is read as the constructor's
zeroing (50): the HUD would read `0/14` until the first crossing. Confidence 65 on "counts
crossings, capped 14", 50 on the start value. **Open: how it relates to the port's
`World::detonator.stages_laid`** (which is `1` at the opening stage): a 15-second capture of the
original's first lap line (`DetonatorStage` text before and after the first crossing) settles it.

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

The id-to-icon table is at TOC `-0x5cc0` (`0x00920848`, 12-byte `{widget, id, name}` rows, walked at
`0x0008f9f0`-`0x0008fb04`): **id 8 is `ForwardWarningMineIcon`**, 9 `RearWarningCannonIcon`, 13
`ForwardWarningBombIcon`, 5 Rocket, 0 Quake, 2 Plasma, 7 Missile, 1 Repulser, 11 Shuriken, 12
LeachBeam. `Detonator_warnings.xml` authors `ForwardWarningBackground` and
**`ForwardWarningBombIcon` only**, and no `ForwardWarningMineIcon` (the generic `hud_warnings.xml`
authors both). So a bomb ahead raises id 8, whose widget does not exist in Detonator's layout: on
HD the icon cannot show, and at most the background plate and the directional `WarningFront*` /
`WarningBack*` arrows would (hypothesis 50; whether the plate follows the slot's timer is unread).
Recipe: one RPCS3 capture flying toward a bomb (a bomb 500 ahead) with `grab_with_clock`, and a
look at the strip above the DamageBar.

## 2048 and Omega (checked)

All twelve widget-name strings (`Bullet%02d`, `DetonatorScore`, `PulseBullets`, `EmpScore`,
`DetonatorStage`, `PulseBankedScore`, `Multi%02d`, `Emp_%d`, `x%02d`, `%d/%d`, `ForwardWarning*`)
are in both executables. 2048's binder and update pair are `FUN_8114ac5c` (`Bullet%02d`) and
`FUN_8114b3a0` (`x%02d`); Omega's `FUN_01269b40` and `FUN_01268d10`. Bodies not diffed against HD's.
Per the 2048-checked-against-Omega rule:

- **Layout: same files.** 2048's `data/xml/Detonator_HUD.xml` (3541 bytes), `HUD_DetonatorWeapons.xml`
  (11448) and `Detonator_warnings.xml` (2000) are the HD files byte for byte modulo CR (DATA00's are
  3472/11448/2000 with LF). Omega's `data00` `detonator_hud.xml` (3541) is the same; its
  `hud_detonatorweapons.xml` differs in **one tag: `<aImageModel name="EmpBarEffect"` ... `</aImageModel>`**
  (a typo in the shipped file; the loader's treatment of an unknown tag is unread, so Omega probably
  never creates `EmpBarEffect`, 60), and its `detonator_warnings.xml` doubles the `fury_hud.gtf`
  rectangles (`U=770 V=554 TxtrWidth=516`, a 2x atlas).
- **2048 patch v1.04 adds a second skin**, `data/xml/2048_hud/Detonator_HUD_HD.xml` (+ its own
  `Detonator_warnings.xml`): `ScoreTxt` (`IG_HUD_SCORE`) with the `DetonatorScore` child, a
  `DetonatorStageParent` (`FE_STAGE`) with `DetonatorStage`, `HUD_damage_indicator.xml`, and the same
  three medals at `ImageX 720`. It includes `DetonatorWeapons.xml` (a different path from HD's
  `HUD_DetonatorWeapons.xml`, unread whether it is the same fragment). So on 2048 the score and stage
  widgets are text-only, with no `detonator_hud2` frame.
- Omega's VR patch (`data08`) ships `Detonator_hud_VR.xml`, `Detonator_hud_VR_Ext.xml`,
  `hud_detonatorweapons_VR[_Ext].xml`: not read.
- Outcome: **checked, applies** (widget names, formats and layout), with **checked, differs** for the
  Omega `EmpBarEffect` tag, Omega's 2x warning atlas and 2048's v1.04 text skin. The 2048 and Omega
  update bodies are not diffed (hypothesis that they match HD's, 60).

## What stays open

- The `EmpBarEffect` shader constants (`Colour`, `EMP`, `Speed`, `Brightness`, `EMP_Rays`) and the
  colour table at `0x00765b38`: the writes after `0x00082600`.
- The bullet icons' per-state art (`0x0007f0a0`) and what `clip+0x80..0xb8` are.
- The constant block `0x0029d578` writes (the ship's bullet gauge) and whether it is drawn.
- Message 8's icon on HD and the directional arrows' use (above).
- The initial value of `rm+0x2dee`.
