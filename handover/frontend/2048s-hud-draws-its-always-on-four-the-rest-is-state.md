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
`ZoneCounterBG` - the intersection across grid, mid-race, pickup held,
damaged, time trial and Zone frames. `oag_display::space::Space::VITA`
(960x544) is measured off the same frames; before it a Vita source drew its
HUD in the PSP's grid, twice the size, right-hand column off screen.

## Open

- **The shield fill.** The frames show `EnergyBar` cropped **vertically from
  the bottom** to the shield fraction, white at runtime where the layout
  authors green at half alpha, with `EnergyBarDelay` flashing red for a
  moment after a hit (`36-w-15.png`: red cap, red stripe outlines). Nothing
  here crops vertically (`oag_game::hud::draw::crop_horizontally` is Pulse's
  model) and nothing tints. Our shield reads full whatever the race does.
- **The pickup slot.** `PickupBgFrame` is up exactly while a pickup is held;
  `PickupIcon` shows at its authored top-centre rect with the `PickupText`
  caption once (the grant), then inside the bottom-left frame, UV rewritten
  per weapon. `pickup_sprites` selects by widget *name* for Pulse and HD;
  2048 needs a per-weapon UV table (Missile and Rocket discs are identified
  on `hud_2048.gxt`, the other eleven are not) and a second position.
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
- The capture scripts are twenty-line shell files under
  `~/.cache/oag/2048-hud/` and are not in the tree; a
  `scripts/vita3k-drive.py` on the model of `scripts/rpcs3-drive.py` would
  make the next capture one command.

## Next Steps

1. Wire `EnergyBar` as a vertical bottom-up crop keyed to
   `readout.shield_fraction()`, in its authored colour, and record the
   white/red runtime tints as the remaining gap - `36-w-5.png` (95%) and
   `68-zone-5.png` (27%) are the two frames to compare against.
2. Give `oag_game::hud::text_for` the four 2048 arms (`Laps`, `Position` as
   `place/ships`, `EnergyText`, `TotalTime` beside a place) and find the
   title's font and strings so the captions resolve.
3. Read the per-weapon UV table for `PickupIcon` off `eboot.elf` - the two
   known discs' UVs (Missile, Rocket) are the anchors to search for.
4. Write `scripts/vita3k-drive.py` from the recipe page so steps 1-3 can be
   checked against a fresh frame in one command.
