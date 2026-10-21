# Zone's HUD: the speed-class line, the unwritten `SubWeapon`, and the lap counter

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. **Read 2026-10-21** (`pulse-zone-render`):
one disassembly, the string table, and live PPSSPP reads of the HUD object on the two Zone states the
compare page holds (Moa Therma White zone 50, Talon's Junction White zone 8).

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x0881ad10` | `Hud_UpdateZoneSpeedClass` | function | 85 |

## The speed-class line: `Hud_UpdateZoneSpeedClass` (`0x0881ad10`)

`Hud_Update` (`0x0881bf50`) calls it when `hud+0x40 & 0x20`, every frame. `hud+0x3c` is the registered
`"PLAYER_HUD"` block ([race-progress.md](race-progress.md)); in Zone `+0x1c` is the **zone** and `+0x20` the
**score** (live: `51` and `141813` on the zone-50 state a moment after it loaded, the mode object's own
`+0x1a10`/`+0x1a1c` reading `50`/`139901` at the load stop). The widgets, bound in `Hud_BindWidgets`
(`0x0881fbec`, `0x08820488` and its neighbours) only when `+0x40 & 0x20`: `hud+0x214` `Zone`, `+0x218` `Score`,
`+0x230` `SpeedClass`, `+0x234` `SpeedClassTxt`.

```text
sprintf(buf, "%d", zone)   -> Zone          sprintf(buf, "%d", score) -> Score
SpeedClassTxt  <- "RC_SC"                    (the caption, written every frame)
SpeedClass     <- by zone, descending:
    zone >= 35  "MSC_ZEN"       and SpeedClassTxt <- ""  (the caption goes)
    zone >= 22  "MSC_SPPHANTOM"
    zone >= 20  "Phantom"       (a literal, not an id)
    zone >= 14  "MSC_SPHANTOM"
    zone >= 12  "Rapier"        (literal)
    zone >=  7  "MSC_SRAPIER"
    zone >=  5  "Flash"         (literal)
    zone >=  3  "MSC_SFLASH"
    zone >=  2  "Venom"         (literal)
    else        "MSC_SVENOM"
```

The ids are `0x08a79c58`-`0x08a79cb8` in `.rodata`; the four literals print as written in every language.
These are the same ten names HD's `ZONE_SPEED_CLASSES` carries, on other zones. **Confidence 85:** thresholds
from the disassembly (`slti` against `0x23`, `0x16`, `0x14`, `0xe`, `0xc`, `0x7`, `0x5`, `0x3`, `0x2`), the
widgets from the bind code and the live reads; the picture agrees on both pairs (zone 8 draws `sub-rapier` at
the authored `SpeedClass` slot, zone 50 draws no `speed class` caption).

Wired as `oag_pulse::hud::ZONE_SPEED_CLASSES` (`rung_zones`, `caption_hidden_from_zone`), read by
`oag_hud::draw::text_for`. Pulse has no `ZONE_STAGES`, so no grade follows from it.

## `SubWeapon` is bound, never written, and drawn

`hud+0x248` is `SubWeapon` (`Zone_HUD.xml` authors it at `(240, 45)`, `HUD` font, scale 1). The only code that
touches it is the bind at the end of `Hud_BindWidgets`; no update function writes its text, and its visibility
flag (`+0x2c` bit `0x4`) stays set (`0xf086`, like `Zone` and `Score`; the lap-gap widgets read `0xb082`, hidden).
Its text buffer (`0x08b561b0` on both states) holds `1000000000.00`, a copy of the float formatter's scratch
(`FUN_08974b24`'s buffer `0x08b208f8` holds the same bytes). So **the original draws `1000000000.00` in the
HUD font under the class name for the whole Zone run**, on both pairs. Wired as
`HudArt::zone_stale_text` (`oag_pulse::hud::ART`), drawn from zone 1 like the counters. **Not measured:**
`TimeTrial_HUD.xml` authors the same widget (and `Arcade_HUD.xml` at scale 0.6 with a border); no mid-race Time
Trial or Arcade original was read for it, so nothing is drawn there. Pulse PS2 shares the data and is not
measured.

## The top-left counter is the lap, not a zone count

`Zone_HUD.xml` authors `Lap` at the top left (`LapTxt`/`Lap`, `IG_HUD_LAP`), and `Hud_UpdateLapCounter`
([race-progress.md](race-progress.md)) writes `PLAYER_HUD+0x08` into it: live **24** on the zone-50 state, so the
craft had flown 24 laps. Zone runs on until the shield is gone, so the lap keeps counting. The pair scripts pass
`--hud-state` a lap of `1` (their own caveat says "Zone_HUD authors none of them", which is wrong for `Lap`);
re-rendered with the original's lap the counter matches. This is a capture input, not a law.

## The speed readout reads the same in Zone

The Zone pairs' note of "1244 km/h against 1020" was read off the 480-pixel side-by-side by eye. At 1x the
original's `SpeedBarText` reads `1249 kmh` and ours `1250 kmh`: the same scale (`Hud_UpdateSpeedBar` divides by
700 and writes `(int)v`), and the one km/h is the HUD reading the speed one tick behind the trace row.
Nothing differs in Zone.

## The craft's shadow is drawn in Zone

A GE dump of the original on the zone-50 state (`psp-ge-dump.py`) holds the shadow pass: two stencil-volume
draws (`0x400`/`0x500`, 456 vertices) and the wipe quad blending `dst * 0x9f/255`; a breakpoint on
`Shadow_RenderOccluderVolume` hits every frame with the craft's own attached occluder (`parent+0x94->+0x2f8` =
1.6 to 2.1). So Zone does **not** turn the shadow off. The dark polygon the original does not show under the
craft is washed out by the plume and the bloom around it. Ours' darkness is now the measured `1 - 159/255`
(`oag_render::shadow::HULL_DARKNESS`, was `0.35`). Residual: ours' plume bloom is narrower than the original's
at tick 0 (the original's white-out reaches about twice as wide), so a strip of the polygon still shows on either
side.

## The plume at tick 0

Ours renders at tick 0 with no exhaust history, so the plume is cold. `--field-warmup` replays the exhaust at the
captured velocity, but it wanted a field file with opponents, and it left each craft at the replay's velocity and
thrust, which moved the frame (the camera reads them). Both fixed: a player-only `--field-from` is accepted (a Zone
run has no opponents) and `warm_up_field` hands the velocity and thrust back
(`field_capture_ground_truth.rs`). Zone pairs re-rendered with a one-row `field.csv` (slot 0 and its `vel_*` from
the pair's `trace.csv`) and the original's lap: mean grey difference Moa z50 10.58 to 7.98, Talon's z8 17.68 to
15.18 (their own metric reads about 0.5 lower). `psp-pulse-pair.py --zone` owns the pair flow; it should write that
`field.csv` and the lap into `--hud-state` (handover thread).

## Cross-title (static)

- **Pure PSP:** no `MSC_SRAPIER` string in `BOOT.BIN`, so no class ladder of this shape (checked, differs); it has
  a `SubWeapon` widget too (`0x08a7a244`), its Zone text unmeasured.
- **HD:** its ladder is stage-keyed (`oag_hd::hud::ZONE_SPEED_CLASSES`, checked, differs in the key, the same names).
