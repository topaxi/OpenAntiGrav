# The speed and shield bars: a floored tip, a right-anchored window, a marker and a label that ride it

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. **Read 2026-10-21** (`pulse-hud-align`):
two decompiles, one disassembly and live PPSSPP reads of the widgets' own fields. Names below are applied
from [names.tsv](names.tsv).

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x0881a3e0` | `Hud_UpdateSpeedBar` | function | 88 |
| `0x0897e158` | `Math_Floorf` | function | 85 |

`Hud_UpdateEnergyBar` (`0x0881c638`, [shield.md](shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient))
runs the same tip law for the shield bar; this page is the half that page did not read.

## The law

`Hud_Update` (`0x0881bf50`) calls `Hud_UpdateSpeedBar` when `hud+0x40 & 1`. With `v` = the craft's speed in km/h
(`PLAYER_HUD+0x00`, `PlayerStatus+0x48`):

```text
v  = 0 if v < 5.0                                    (a crawl reads zero)
f  = min(v / 700.0, 1.0)                             (0x44 0x2f 0x00 0x00 literal 700.0)
f' = f * 0.96 + 0.04                                 (a bar is never empty)
w  = floorf(f' * W)       W = hud+0x178 = SpeedBarBg's width (168)
SpeedBar.width        (+0x9c) = w
SpeedBar uv           (+0xc4) = uv_width * f'        (+0xbc) = uv_origin + uv_width * (1 - f')
SpeedBarMark.x                = authored x + w - 6.0 (virtual setter +0xd4, base from getter +0xdc)
SpeedBarText.x        (+0x9c) = authored x + w - 6.0
SpeedBarText string           = (int)v, then " kmh"  (truncated, not rounded)
```

The **uv window is the right `f'` of the art**, placed in a quad `w` wide at the bar's left edge: the art's
pointed end is the tip at every speed, where a left crop would show a notch. The shield bar is the same with `W =
hud+0x17c` (`ShieldBarBg`, 140), `f = max(shield * 100 / max, 0) / 100` clamped, the label `(int)percent` then
`%`, and no idle floor. `Math_Floorf` is the PSP libm `floorf` (it masks the fraction bits of the float, `0x0897e158`
disassembly), the `FUN_0897e158` that [countdown-voice.md](countdown-voice.md) already read as "whole seconds
left".

## Evidence

- **Live, 60 consecutive ticks** of `Hud_UpdateSpeedBar` on the `race-talons-venom-alongside` state (PPSSPP
  breakpoint at the next function, `0x0881b230`, reading the widgets after the update): the predicted `floor(f' *
  168)` equalled `SpeedBar+0x9c` and `SpeedBarText+0x9c` equalled it minus 6 on every one, at speeds 498 to 554 km/h
  (`134, 129, 126, 122, 121`); the shield bar matched on 59 of 60 (the other was the shield changing in the same
  tick). With `600.0` in place of `700.0` the prediction at 554 km/h is 155 px against the live 134. At standstill the width is 6
  (`floor(0.04 * 168)`) and the label's offset 0.
- **The window**: the bar's `uv_origin + uv_width` read `0.6797` at every speed sampled (0.1544 + 0.5253, 0.1734 +
  0.5063, ...): the right edge is fixed, the origin moves.
- **Frames**: the compare pair `talons-alongside` (a pointed fill ending at the same column in both) and the 24 grid
  pairs (a 6 px stub with the art's point, label offset 0).

## Ours

`oag_title::hud::BarTip` on `oag_pulse::hud::ART` (`floor_permille 40`, `span_permille 960`, `tip_dx -6`,
`idle_kmh 5`, the two bars) and `oag_hud::bar_tip`: the fill's floored width and right window, the marker and the
label moved by `w - 6`, the labels truncated. Pulse's `speed_full_kmh` is **700** (it was an inferred 600). The
mean grey difference over the 24 grid pairs' bar region (480x272, x 300-480, y 215-272) is 5.65 before, 4.65 after;
the race pair's 29.6 to 24.6 (the rest of that gap is the original flying at 572 km/h against ours at 587).

## Other titles

- **Pulse PS2: checked, applies, wired through the shared `oag_pulse::hud::ART`.** Its HUD update functions are
  unread; the layout and art are one definition across pressings.
- **Pure: checked, not read.** `bar_tip` stays `None` and `speed_full_kmh` 600; its HUD update is unread.
- **HD and Omega: checked, differs.** HD's speed fill is a `<CurveBar>` and its `Hud_UpdateSpeedReadout`
  (`0x00086e88`) crops to `speed / 700`; no tip-following marker or label (HD authors no `SpeedBarMark`).
- **2048: not checkable here** (no frame at speed compared; its thrust bar is a chase, not this law).

## Not wired

The speed bar's alpha pulse (`+0xf4` from `PlayerStatus+0x274+0xa8`, a 0 to 255 flash scaled by `/10`) and the
per-frame HUD shake (`Hud_Update`'s `+0x3c/+0x40` offsets on two widget groups) are read and left: the shake is
`hd-cockpit-lean`'s territory on HD and unmeasured on Pulse.
