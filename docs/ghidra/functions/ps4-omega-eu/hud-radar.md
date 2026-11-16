# The white ring round the craft is 2048's HUD radar

Read 2026-11-16, lane `grid-ring-glow`, on the **1.07 patch** eboot (Ghidra program
`/omega/eboot-ps4-omega-eu-v107.bin`; every Omega address on this page is a patch address, see
[patch-v107-addresses.md](patch-v107-addresses.md)) and on 2048's v1.04 executable
(`/2048/eboot-vita-2048-eu-v104.elf`). No names are applied: the Omega functions are read on the
patch build, which this directory's `names.tsv` does not key on, and 2048's `Hud_BindWidgets` is
already named ([pickup-icon-uv-table.md](../vita-2048-eu-v104/pickup-icon-uv-table.md)).

## What it is

**Confidence 90** (decompiled in two executables that agree, the layout's placement matches both
originals' frames to a few pixels, and the wired widget lands on the ring in ours).

The white arc behind the craft on the 2048-mode grid, with a translucent split bowl inside it and a
ringed white dot at the top centre, is the HUD widget `Radar` with its child `PlayerDot`, authored
by `Data\XML\2048_hud\HUD_half_radar.xml`. Every played single race includes that file
(`2048_hud\Arcade_HUD.xml` and `Arcade_HUD_HD.xml`, a `<LoadXML>` beside `HUD_objectives.xml`); the
Eliminator and Zombie roots include `HUD_full_radar.xml` instead (a full ellipse, `hud_2048` U1106
V732, not measured here). It is a 2D sprite pair in the 544-line HUD space, drawn over the craft,
not a model under it.

| | 2048 (Vita) | Omega (PS4, 2048 mode) |
| --- | --- | --- |
| `Radar` destination (x, y, w, h) | (349, 439, 263, 72) | (381.75, 439, 197.25, 54) |
| `Radar` source | `hud_2048.gxt` (2, 419, 188, 52) | `hud_2048.2x.gxt` (8, 1676, 752, 208) |
| `PlayerDot` destination | centred (480, 439), 42 x 16 | centred (480, 439), 31.5 x 12 |

Both are centred on x = 480 and hang from y = 439. At 1080p Omega's spans x 765 to 1156 and y 871
to 978; the ring in `omega/queens-mall-2048`'s original measures x 765 to 1155 and y 875 to 975.

## Why it is always up

- **Omega, `FUN_012d1590`** (the 2048-dialect HUD bind, the analogue of 2048's `Hud_BindWidgets`):
  looks `Radar` up into a local (`0x012d384x`), stores half its width at `hud+0x8f0`, half its
  height at `hud+0x8f4` and a range of `200.0` (`0x43480000`) at `hud+0x8f8`, then binds
  `RadarDot0`-`7` into `hud+0x8b0..` and zeroes each dot's colour and low flag nibble. It never
  clears bit 2 of `Radar+0x60`, the visible bit the same function clears to hide `RaceMedal`,
  `ManualShield` and `SpeedPadThrustBarBG`, and keeps no pointer to `Radar` that a later pass could
  use to hide it.
- **2048, `Hud_BindWidgets` (`0x81192cd0`)**: the same sequence (`Radar` into a local at
  `0x81193c88`, half sizes at `hud+0x6d4`/`+0x6d8`, `200.0` at `hud+0x6dc`, dots at `hud+0x6b4..`
  moved to `(0, 0)`), and it never clears `Radar+0x30` bit 2, the bit it clears for `RaceMedal`,
  `ManualShield`, `TargetShip*` and the rest.
- **`PlayerDot` is named by no code**: `search_strings("PlayerDot")` finds nothing in either
  executable, so the child stays as authored.

[2048-hud.md](../../../formats/2048-hud.md) once listed the radar as "never seen in any state
reached", and the same page later recorded "the white ring round the hull" as unmatched: the ring
is the radar. Why the earlier capture list missed it is not checked; the time-trial and Zone
roots include no radar file.

## The dots: read, not wired

**Confidence 75** (one executable's decompile, consistent with its bind; no capture with an
opponent behind the player).

Omega's 2048-dialect update `FUN_012d3c50` (`0x012d5737`, `0x012d5820`) places the dots, gated on
the HUD's flag word `hud+0x30 & 0x30000` (`0x10000` set: the full radar). For each other craft:

1. `d` = its position less the player's; `f` = `d` normalised, dotted with the player's forward
   axis; `t = min(|d| / 200, 1)`.
2. On the half radar a craft is placed only when `f < 0` (behind); the full radar places all.
3. `x = -(t * (normalised d . right) * half_width)`; `y = f * t * half_height`, then times `-0.5`
   when `f >= 0`, else times a per-radar constant (`DAT_0186e838`, indexed by the full flag).
4. Alpha: half radar `255 - 127.5 * t` within range and 0 beyond; full radar
   `max(1 - t, 0.2) * 255`. The colour is white with that alpha.

On a grid where the player starts last, no craft is behind, so no dot draws: the grid frames need
only `Radar` and `PlayerDot`.

## Wired

`oag_2048::hud::ALWAYS_ON` and `oag_omega::hud::ART_2048.always_on` carry `Radar` and
`PlayerDot`. Tests: `vita_2048_hud_ground_truth::the_played_single_race_draws_the_half_radar_round_the_craft`,
`omega_hud_ground_truth::omega_2048_mode_draws_the_half_radar_round_the_craft`.

Hull box (1080p `760x420+580+640`; the Vita pair at 960x544, `380x210+290+330`), mean absolute
error per channel against the original, ours before and after, both rendered from the same tree:

| Pair | Before | After |
| --- | --- | --- |
| `2048-vita/moa-therma` | 44.44 | 39.14 |
| `omega/capital-reach-2048` | 34.46 | 31.70 |
| `omega/queens-mall-2048` | 48.14 | 47.05 |
| `omega/the-amphiseum`, `omega/modesto-heights` (HD mode) | 58.21, 57.83 | unchanged |

**2048 against Omega: ported**, each with its own test; the two files differ in size and atlas
(above), and each title reads its own.

## The HD-mode arc is a different widget, not wired

**Confidence 60.** Omega's HD-mode grid frames (`omega/the-amphiseum`, `omega/modesto-heights`) draw
a translucent notched arc across the craft. It is `ProximityBar` from `Data\XML\hud_proximity.xml`
(`HUD_Components` U358 V524, 666 x 114, `Vertices="10"`, at (960, 906)), included by HD's
`arcade_hud.xml`. Omega's HD-dialect bind `FUN_0127c860` binds it at `hud+0xc30` and hides it; the
update `FUN_0126b030` shows and hides it with a group of widgets (`hud+0xc20` to `+0xc88`, near
`0x01278fb6` and `0x012792cb`) under a game-state test this lane did not decode. HD (PS3) frames of
the same grids (`hd/the-amphiseum`, `hd/modesto-heights`) show no arc, so it is not inherited from
HD as-is.
