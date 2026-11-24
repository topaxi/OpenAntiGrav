---
status: measure
titles: [pure]
updated: 2026-11-24
summary: "Pure Zone pairs read 34.5/35.2 after camera+fov; open: Zone_01 physics unread, hull glow and shadow, walls' bands, HUD, a second boot"
categories: [gameplay]
size: M (60)
difficulty: medium (55)
---

# Pure's Zone pairs after the camera and the field of view

`pure-psp-zone-pro-tozo-z9` and `-z8` went from 64.14 and 52.87 to 34.54 and 35.22 mean grey once a Zone race read its cameras from
`Data\Ships\Zone_01\handlingstats.xml` and `--camera-fov` reached a `--no-camera` render (`docs/gameplay/race-modes.md`, "Pure's Zone
cameras"; `docs/tools/compare-page.md`, "Why the Pure Zone pairs read 53 and 64"). What the region split still shows:

## Open

- **The Zone file's six physics blocks are unread** (`<Engine>`, `<Brakes>`, `<Turning>`, `<Airbrake>`, `<Antigrav>`, `<Physical>` on `<Stats>`, no `<Class>`):
  a Zone race flies the player's team's numbers while the camera and the hull are the Zone craft's. HD already reads its classless file
  (`oag_hd::race::ZONE_HANDLING`, `oag_title::zone_handling`); Pure needs the same, with Pure's own `landing_rebound` and `rebound_jump_time` defaults read
  from a running Pure (the file authors neither) and Pure's compiled-in pitch. Unmeasured: do not wire before a Zone run on the original gives a speed and turn curve
  under player input (not the Autopilot).
- **Hull box** (z9 -7.8, z8 -10.1 after paste): the original's bright blue engine glow, which ours does not draw, and ours' ground shadow under the hull, which the original lacks in Zone.
- **Walls' black bands and the pink-violet bloom at the lights** (background above the road is still 14 to 18 of the 34 to 35): not located; the frames' mean grey agrees within 9. **Not the scenery's animation phase**: the pairs were re-made with the world clock read
  from the states (`original.world_clock_offset_s` 93.06 and 84.15, ours put on it with `--world-clock-offset`) and the mean grey is unchanged to four digits, so this circuit's frame
  carries no animated material in view.
- **HUD** (-6.4): ours prints the zone clock as 0 and fades the ZONE label differently; Pure's zone, score and zone-seconds RAM addresses are still unlocated.
- **The Far view** (`<ExternalCameraFar>` `3.8`, `-15`, fov 70) and the cockpit were not read live in Zone.
- **A second cold boot** of the Zone race: every number here is one boot and two states.

## Next Steps

1. Locate Pure's zone, score and zone-seconds words (static, `psp-pure-usa` `BOOT.BIN`, around the Zone HUD's `%d` prints): 1-2 h, `oag-re`, confidence 50.
2. Wire the Zone file's physics once a player-input run on the original gives a speed curve: 2-3 h after that run, `oag-wire`.
