# 2048's HUD draws its always-on four; the rest of what the frames show is state-gated and unwired

2026-09-16, branch `lane/2048-hud`. Full write-up in
[2048-hud.md](../../docs/formats/2048-hud.md#captured-on-vita3k-what-a-race-actually-shows-2026-09-16)
and the capture recipe in
[vita3k-capture.md](../../docs/reverse-engineering/vita3k-capture.md).
**The one to know**: the title runs on this machine, at speed, on the real
GPU, on a display that is not the user's - but only through a headless
weston plus a rooted Xwayland, because Xvfb has no DRI3 and Vita3K's Vulkan
renderer refuses to start without it (`Failed to select proper Vulkan
queues`). Touch needs a 300 ms hold, accelerate is R1 (`e`), and Pilot
Assist Extreme drives a lap unattended.

`oag_2048::hud::ALWAYS_ON` is `ThrustBarBG`, `EnergyBgFrame`, `EnergyBg`,
`EnergyBar`, `ZoneCounterBG` - the intersection across grid, mid-race,
pickup held, damaged, time trial and Zone frames, plus the shield fill wired
2026-09-20 (below). `oag_display::space::Space::VITA` (960x544) is measured
off the same frames; before it a Vita source drew its HUD in the PSP's grid,
twice the size, right-hand column off screen.

## Open

- **The shield fill's runtime tint - and it is `EnergyBg`, not `EnergyBar`,
  that the executable tints.** `EnergyBar` crops **vertically from the
  bottom** to the shield fraction (`oag_game::hud::draw::crop_vertically`,
  wired 2026-09-20 - see
  [2048-hud.md](../../docs/formats/2048-hud.md#energybar-is-wired-a-vertical-crop-from-the-bottom-2026-09-20)),
  checked against `21-race2-start.png` (100%, exact) and `36-w-5.png` (95%,
  the boundary row to within one pixel), and the crop is independently
  confirmed by decompiling `Hud_UpdateEnergyBar` (`0x811957a2`, see
  [pickup-icon-uv-table.md](../../docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md#hud_updateenergybar---0x811957a2)).
  That same read found two corrections: `EnergyBarDelay` is not a red flash,
  it is the same crop fed an exponentially-smoothed *lagging* fraction (a
  trailing damage edge); the widget that actually turns red
  (`0xffff0000`) at <=20% shield or during a flash timer, else a fixed grey
  (`0xffa7a5a7`), is `EnergyBg` - which this thread and `oag_2048::hud::
  ALWAYS_ON`'s own doc comment currently read as a fixed translucent
  constant with "nothing tints". **Not yet corrected in either place, and
  not yet wired** - the values are sourced (not a tint recalled from a
  frame), so wiring `EnergyBg`'s colour is now a small, well-evidenced
  follow-up rather than a research question. `EnergyBar` itself still has
  no located colour write anywhere in `Hud_UpdateEnergyBar`, so why it
  reads white at runtime (vs. its authored green-at-half-alpha) is still
  genuinely open.
- **The pickup grant announcement.** The held state is wired (below);
  the frames also show `PickupIcon` at a *second*, top-centre position
  once, with the `PickupText` caption, the instant a pickup is granted -
  unwired, because `Readout` carries no time-since-grant state to key it
  off. Needs a new readout field, not just a draw-side change.
- **The speed fills** `ThrustBar`, `SpeedBar0`-`4`: seen filling, model
  unread. The swoosh takes a purple tint on a speed pad.
- **`PilotAssist`** off the assist setting; **`ZoneLight0`-`9`** one per
  zone reached. Both are a readout field and a gate away.
- **The text.** `Laps` and `Position` are single strings (`1/3`, `8/8`), the
  arcade layout shows `TOTAL` and `POS` together where `place_owns_the_anchor`
  hides one, `EnergyText` is the `%` readout, and Zone's `SCORE` and
  `ZONE`/number/class text are positioned by something the composed layout
  does not carry (`ZoneNumber` is authored top-left and drawn bottom-left; no
  `Score` widget exists). Separately, `oag_2048::TITLE` names no language
  plugin and no HUD font, so every caption draws as its raw id in 5x7.
- **Unseen states**: the sights (no opponent in range at any sampled instant
  - Square did not fire, what does is unmeasured), the radar, the countdown,
  Elimination, Speed Lap, the pickup absorb/`ManualShield`, `GiftCannon`,
  `RaceMedal`, `GloryMoment`, `ObjectivePoint`.
- **`scripts/vita3k-drive.py`'s `race` menu walk is save-state dependent and
  not proven unattended-reliable even on a fresh save** - see its own
  `FIRST_EVENT_TAPS` doc comment and
  [vita3k-capture.md](../../docs/reverse-engineering/vita3k-capture.md#scriptsvita3k-drivepy-on-the-model-of-scriptsrpcs3-drivepy).
  `display`/`boot`/`stop` and the `tap`/`key`/`hold`/`shot` primitives are
  each checked end to end (drove a real race - Metro Park Time Trial - by
  hand); the fixed six-tap sequence into a race is not, on this machine's
  own already-progressed save. A genuinely fresh save (or a person adapting
  the coordinates the way the manual recipe always needed) is still open.

## Next Steps

1. ~~Wire `EnergyBar` as a vertical bottom-up crop keyed to
   `readout.shield_fraction()`~~ - done 2026-09-20, see above.
2. Give `oag_game::hud::text_for` the four 2048 arms (`Laps`, `Position` as
   `place/ships`, `EnergyText`, `TotalTime` beside a place) and find the
   title's font and strings so the captions resolve.
3. ~~Read the per-weapon UV table for `PickupIcon` off `eboot.elf`~~ - done
   2026-09-20: `g_pickup_icon_uv_table` (`0x81489070`), all eleven weapons,
   see [pickup-icon-uv-table.md](../../docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md).
   `pickup_sprites` wired and checked against five weapons live
   (`just play 2048 --race --give <weapon>`). Left open by that pass: the
   grant announcement and the `EnergyBg`/`EnergyBarDelay` corrections above.
4. ~~Write `scripts/vita3k-drive.py` from the recipe page~~ - written
   2026-09-20. `display`/`boot`/`stop`/`tap`/`key`/`hold`/`shot` are checked
   end to end; `race`'s own fixed menu walk is not - see the bullet above.
   Re-running `race` against a genuinely fresh save (or fixing the Game Mode
   tile's select-then-confirm timing some other way) is the open half.
