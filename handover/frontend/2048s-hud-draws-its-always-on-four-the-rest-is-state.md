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
`EnergyBar`, `EnergyBarDelay`, `ZoneCounterBG` - the intersection across
grid, mid-race, pickup held, damaged, time trial and Zone frames, plus the
shield fill wired 2026-09-20 and the `EnergyBg`/`EnergyBarDelay` corrections
below, wired 2026-09-25. `oag_display::space::Space::VITA` (960x544) is
measured off the same frames; before it a Vita source drew its HUD in the
PSP's grid, twice the size, right-hand column off screen.

## Open

- **`EnergyBar`'s own runtime colour is still unresolved.** `EnergyBg` (not
  `EnergyBar`) tints red at or under 20% shield or during the shared
  post-hit flash, wired 2026-09-25 in `oag_hud::dialect_2048::
  energy_bg_tint` off the full decompile of `Hud_UpdateEnergyBar`
  (`0x811957a2`, see
  [pickup-icon-uv-table.md](../../docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md#hud_updateenergybar---0x811957a2)),
  checked live: `just play 2048 --race --mode single_race --no-audio --hold
  cross,right` at 100/97/82/34/15% shield reads grey above 20%, a red sliver
  at the tip during the post-hit flash window even above 20% (97%, 82%), and
  solid red at 15%. `EnergyBarDelay` is wired the same pass as the lagging
  trail (`crate::race::Race::advance_energy_bar_delay`,
  `oag_hud::dialect_2048::vertical_bar_fraction`), pinned by
  `crates/raceplay/src/tests/energy_bar_delay.rs` rather than a screenshot -
  no capture in this pass landed on the exact tick after a scripted hit that
  would show it trailing visibly above `EnergyBar`. Neither widget has a
  located colour write in the decompiled function, so `EnergyBar` keeps its
  authored green-at-half-alpha and `EnergyBarDelay` its authored opaque
  white; why the original reads both white (with a red flash on the delay
  layer) is still genuinely open.
- **The pickup grant announcement.** The held state is wired (below);
  the frames also show `PickupIcon` at a *second*, top-centre position
  once, with the `PickupText` caption, the instant a pickup is granted -
  unwired, because `Readout` carries no time-since-grant state to key it
  off. Needs a new readout field, not just a draw-side change.
- **Wired 2026-10-05**: the speed fills, `PilotAssist`'s draw and
  `ZoneLight0`-`9` - see `2048-hud.md` and `pickup-icon-uv-table.md`. Still
  open from them: `ThrustBar`'s true target (ShipCTRL `+0x578`, per-class
  ramp rates unrecovered; the port feeds the raw thrust state), the purple
  `SpeedPad*` tint (a bind-time ship-definition choice, not pad contact),
  `PilotAssist`'s source (no assist setting exists to feed
  `Readout::pilot_assist`) and its scale pulse, the zone lights' counter
  `hud+0x73c` past zones 2 and 5 and their class-up blink.
- **`Laps`/`Position`/`EnergyText`/`TotalTime` are wired, 2026-09-25** - see
  Next Steps 2. Zone's `SCORE` and `ZONE`/number/class text are still
  positioned by something the composed layout does not carry (`ZoneNumber`
  is authored top-left and drawn bottom-left; no `Score` widget exists), and
  neither is wired. The font/language-plugin gap this bullet used to record
  was already closed 2026-09-21 (`oag_title::HudArt::hud_font_role`,
  `2048-hud.md`'s "Resolved 2026-09-21" section) - captions draw in
  `2048_hud.fnt`, not the 5x7 fallback; this line was stale.
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
2. ~~Give `oag_hud::text_for` the four 2048 arms (`Laps`, `Position` as
   `place/ships`, `EnergyText`, `TotalTime` beside a place)~~ - done
   2026-09-25: `Laps`/`Position` are new `dialect_2048::laps_text`/
   `position_text` arms (a `position_combined` flag, `speed_unit`'s own
   layout-absence pattern, tells 2048's one-widget `8/8` apart from
   Pulse/HD's split three); `EnergyText` joins `ShieldBarText`'s existing
   arm; `TotalTime`/`TotalTimeTxt`/`PositionTxt` needed
   `place_owns_the_anchor` itself fixed to check the anchors actually
   coincide (2048's `TotalTime` and `Position` sit in different corners and
   both draw at once, where the presence-only version read this as Pulse's
   own coincident-anchor rule and hid `TotalTime` on every 2048 race with a
   field). Checked: `just play 2048 --race --mode single_race --no-audio
   --ticks 60 --screenshot` reads `LAP 1/3`, `TOTAL`/`CURRENT`, `POS 8/8`
   and the shield `%` together, matching `11-load-8.png`. The title's own
   font/strings were already found 2026-09-21 (see the "Open" bullet this
   pass corrected).
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

## From the HANDOVER.md index (moved 2026-09-25)

**the first time this project drove the Vita title** (2026-09-16, `../../docs/reverse-engineering/vita3k-capture.md`), extended 2026-09-20: `EnergyBar` crops vertically from the bottom keyed to shield fraction, and the held pickup icon's UV rewrite table is read straight off `eboot.elf` (`g_pickup_icon_uv_table`, all eleven weapons, `pickup-icon-uv-table.md`) and wired, checked live against five weapons. `scripts/vita3k-drive.py` replaces the manual shell scripts for the next capture; its `display`/`boot`/`stop`/`tap`/`key`/`hold`/`shot` are checked end to end, its fixed `race` menu walk is not (save-state dependent, measured). Extended again 2026-09-25, off the full decompile of `Hud_UpdateEnergyBar`: `EnergyBg` (not `EnergyBar`) tints red at or under 20% shield or during the shared post-hit flash, `EnergyBarDelay` is wired as the lagging trail it actually is, and `text_for` gained 2048's `Laps`/`Position`/`EnergyText` arms plus a `place_owns_the_anchor` fix so `TotalTime` no longer hides on a title whose anchors never coincide. Extended 2026-10-05: the speed segments, `ThrustBar`, `PilotAssist`'s draw and `ZoneLight0`-`9` are wired off `Hud_UpdateWidgets` (`0x81195cdc`). Open: `EnergyBar`/`EnergyBarDelay`'s own runtime colour (still no located write), the pickup grant announcement (needs new `Readout` state), `ThrustBar`'s true target, the `SpeedPad*` tint, `PilotAssist`'s source, and Zone's `SCORE`/ladder text (positioned by something the composed layout does not carry)
