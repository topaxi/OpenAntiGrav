# HD's white nose glow is not an effect of its own; the wingtip contrails are, and a speed pad runs them

Measured 2026-11-09 (lane `hd-nose-glow-control`) on RPCS3 (EU Fury image, recompiler, Vulkan, `Write Color
Buffers` on): ten restores or cold boots. The claim under test, written before the first capture:

> In the original, the compact white glow at the craft's nose appears at race speed with no rival trail ahead,
> so it is an effect separate from the trail-hit burst.

**It came out false.** The glow is not there in a Time Trial or a Zone run at race speed with nothing ahead. What
the maintainer reads as white contrails at the Zone ship's wing tips is a real, separate effect, found in the
executable: HD's `wingtip` locators drive two ribbons that run while a speed pad or a turbo acts
([wingtip-vortices.md](../ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md)). It is not the nose glow either: the
streaks start at the wing tips and run back, and they appear in frames with no glow at the nose.

## What would have falsified it, stated first

The 2026-11-08 pass showed that a bare count of near-white pixels in a nose box cannot isolate the burst (2,903 on
against 3,114 off), so the test here is a **controlled absence**, not a count against the burst:

- **Hypothesis true:** frames at race speed with nothing ahead show a compact white feature at the craft's nose.
- **Falsified if:** at least 8 frames per control, taken at 100 units/s or more, show no such feature when read at
  full size, and a nose-box count of near-white pixels (`min(R,G,B) >= 235`, box `x 1060..1240, y 570..700` on the
  2000x1200 root grab, drawn craft not projected pose, since the original draws its hull a frame behind its RAM
  pose) is zero outside thin track lines and lightning.
- Speed is read from memory (`body+0x190`, `rpcs3_hd_field.field_rows`), not assumed. The trail-hit pairs ran at
  121.6 units/s; the HUD reads 437 km/h at 121 units/s.

## Control B: Time Trial, nothing ahead

Restore `hd-timetrial-talons-venom-start` (Racebox Time Trial, Talon's Junction, `Feisar_c1`, no other craft),
hold cross, grab the root window every 0.3 to 0.4 s with the speed read at each grab.

| Restore | Frames | At >= 100 units/s (both reads) | Slower read of each pair | Nose box > 0 |
| --- | --- | --- | --- | --- |
| 1 | 16 | 12 | 115.7 to 124.4 (a single read reached 161) | 2 (21 and 67 pixels) |
| 2 | 16 | 14 | 100.4 to 122.6 (a single read reached 144) | 3 (57, 102 and 306 pixels) |

The five nonzero boxes were read at full size: the small counts are the white lines of the track passing behind
the nose, the 306 is the purple lightning arcs of a magstrip section crossing the hull. The other 21 of the 26
frames read zero. At rest (speed 0, two frames) the box reads 0. **No compact white glow at the nose in any of the
26 frames; the hull's nozzle shows its orange flame and the ribbon streams back white, as at every speed.** The
wing-tip boxes (`x 900..1020, y 740..840` left, `x 1240..1340, y 730..830` right) read nonzero in 11 of the 26
frames; the frames read at full size show track and scenery lines behind the wings, not a feature on the tips.

## Control Z: a Zone run

The saved Zone state (`zone-moa-therma-hd-early`) is a results screen (`ZONE SESSION COMPLETE`), so the Zone control
was a cold boot into Zone (`rpcs3-hd-zone-probe.py`, 45 and 60 s of throttle, no steering, so the craft grazes the
walls and the speed is ragged: 3.6 to 154 units/s). Twelve frames viewed at full size over two boots, ten of them
at 103 to 154 units/s (about 370 to 550 km/h if the HUD's ratio holds): **no white glow or streak at the Zone
ship's nose or wing tips.** The horn tips (the forward-swept wing tips where the `wingtip` locators sit) stay dark;
the white streaks that appear in some frames are the scenery's speed lines and cross the whole frame. Neither run
crossed a speed pad or used a turbo, which is why the contrails below do not show.

## The wingtip contrails, with the speed-pad state up

The executable reads the pad state: `ShipWingtipVortices_Update` runs on each of the craft's two `wingtip` nodes
every frame, refills a 1.3 s timer while `craft-entry+0x2e8` (the speed pad's remaining force time) is above zero or
a turbo flag is set, and lays a ribbon of the `+0x74` pool (the smoke-trail one) with a strength of the timer's
fraction times a speed ramp (zero below 50 units/s, `0.3` from 100). Nothing in a plain run raises it.

The A/B: in a running Time Trial, with the throttle held at about 120 units/s, `entry+0x2e8` was written `1.0` every
40 ms (the field the pad's own code decrements, so it reads `0.93` to `1.0` through the run; a physical pad crossing
was not made) after six frames without the write.

| | Frames | Streaks from the wing tips | Node `+0xe4` (timer) | Node `+0xe0` (ribbon handle) |
| --- | --- | --- | --- | --- |
| write off | 6 of 6 read (restore 2) | none | `0.0` | `0` |
| write on | 6 of 6 read (restore 2) | two thin pale streaks, one from each wing tip, running back toward the camera, in all six | `1.28` | two handles `0x70` apart |

The other restore's pair (one off and one on frame read) and an earlier forced run (four on frames read) agree. The
on-frames also show the exhaust plume longer and red where the off-frames show the orange nozzle flame and the white
ribbon. Both restores held 111 to 126 units/s through the A/B.

**What is not shown:** a natural crossing of a speed pad or a Turbo pickup; the streaks' alpha and length against a
frame of ours; the Zone ship's pair (its locators are on the forward wings, 4.6 units behind the cannon flashes,
where the maintainer places the contrails). The streaks here are thin and pale, not a compact glow, and sit
behind the wing tips, so they are not the nose glow of the trail-hit pairs either.

## The burst, for contrast

Five tucked frames per condition in `hd-race-talons-venom-pack` (the player placed 8 units behind the nearest rival
along its velocity, 30 frames, `rpcs3-hd-trail-hit-pair.py`'s tuck, the whole run in one connection). The craft and the
rival read `+0x7d2c = 1` on all ten tucks (the red variant, Fury skin). A fresh restore of the same state on 2026-11-09
read `+0x7d2c = 0` on all eight craft, as [emulator-recipes.md](../reverse-engineering/emulator-recipes.md) says
(classic), so the recipe stands; the tuck runs' `1` came after their own setup, not from the state.

- **Burst names intact:** white ray fans and a white glow around the player's nose in the four frames read (speed 133 to
  150 units/s).
- **Burst names broken** (the first byte of both bare names overwritten, `rpcs3-hd-trail-hit-names.py both`): none at
  the player's nose in the four frames read; what white remains is the rivals' ribbons and compact white engine flares
  at the rivals' rears.

The poses are not matched (the Autopilot steers the tucked craft differently each time), so this is a contrast, not
a pair, and carries little weight. It also cannot tell the burst from the rival: in every tuck the rival's nozzle
sits 8 units ahead, just beyond the player's nose, and the 2026-11-08 nose box read more near-white with the names
broken than with them intact. So the glow needs a rival ahead; **whether it is the burst's `CORE_IMPACT` or the
rival's engine flare and ribbon head is not separated.** The separating capture is a tuck with the names broken from a
fresh restore, keeping the frames where the rival's rear overlaps the player's nose.

## What this settles, and what it does not

| Question | Answer | Confidence |
| --- | --- | --- |
| Is the white nose glow present at race speed with nothing ahead? | **No**: 0 of 26 Time Trial frames, 0 of 10 Zone frames read | 85 (two restores, two boots, one circuit and one hull per control) |
| Is it a separate always-on, speed or Zone effect of the craft? | **No** | 85 |
| Is it the burst's? | **Not separated** from the rival's flare; needs a rival ahead | 40 |
| Do HD's `wingtip` locators draw anything? | **Yes**: two ribbons, run by the speed-pad force time or a turbo, scaled by forward speed | 80 (code read, live node fields, A/B frames) |
| What sets it off in play? | The speed pad's force time (`entry+0x2e8`): **60 natural refills on all eight craft, each with a pad crossing** (2026-11-09, below); Turbo flags `0x20`/`0x40` code only | 90 (pad); 80 (Turbo) |
| Are the maintainer's Zone contrails these? | **Probably**: the Zone ship's two nodes refill on Zone's pads at 75 to 137 units/s and lay the ribbon at strength up to `0.3` (below); a frame isolating the streaks from wall-scrape sparks was not taken | 75 |

## Measured live: the natural trigger and the Zone ship (2026-11-09, lane `hd-wingtip-vortices`)

RPCS3, EU Fury image, recompiler, audio Null. `scripts/rpcs3-hd-wingtip-poll.py` finds every craft's two `wingtip` nodes
(vtable `0x0086a340`) and samples them without pausing.

- **Race** (`hd-race-talons-venom-pack` restored, eight craft on the Autopilot, 60 s): 60 timer refills to `1.267`
  to `1.283` (`1.3` less a frame or two), on all eight craft, both nodes of a craft in the same sample, **every one with
  the pad force time above zero** (`0.10` to `0.25`) and no flag `0x20`/`0x40` set. The strength the ribbon holds
  (`element+0x60`) sat on `ramp(speed) * timer / 1.3` in 2,018 of 2,019 samples; 35 fresh ribbon nodes matched the
  push's drift law to `1e-4` ([wingtip-vortices.md](../ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md), "The push").
- **Zone** (cold boot, Single Player Zone, Vineta K, cross held, no steering, 150 s): the Zone ship's two nodes refilled
  on three pad crossings (pad time `0.10` to `0.24`, 75 to 137 units/s), strength `0.15` to `0.30`. The frames at those
  moments are not decisive: one shows white streaks off the right wing, but the craft was grazing the right wall, and
  they read as scrape sparks; the ribbon is born `0.05` wide at most `0.3` opacity, so a single frame at the moment of
  the crossing shows little.

## Wired in ours (2026-11-09, lane `hd-wingtip-wire`)

HD's craft lay the contrails as the evidence page reads them: `oag_fx::fat_line`'s `Vortex` element (nodes born at the
wing tip drifting at `2.5` along `-normalize(cos(theta) row1 + sin(theta) (row2 x row1))` with the craft's `0.75` in the
rows, a 120-degree fan, `+phase` left and `-phase` right, alpha `t * strength`, two clear nodes at each end),
`Style::vortex_line` (pool `+0x74`'s `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`, drawn between the Missile ribbons and the Rocket
smoke), and `oag_raceplay::wingtip` (a 1.3 s timer per locator refilled while `ShipState::pad_timer` or `turbo_timer` is
above zero and the craft is racing, strength `ramp(speed) * timer / 1.3`, a pool of 18). The config is
`oag_title::wingtip::Wingtip::hd_lineage`; the texture `/data/tex/smoke_trails_frame1_alpha.gtf` (1024x128).

- **Speed units.** Ours and the original's agree: the autopilot's craft crosses its pads at 125 to 146 units/s on HD,
  Pulse and Pure (above the ramp's 100, so the ribbons hold their `0.3` ceiling at the crossing and fade with the timer).
- **The side.** The `wingtip_left` locator lands on the craft's physical left in this engine (`-2.08` along the hull's
  right axis on slots 0 to 2), so the original's side flag `1` (`+phase`) is the world's left; the race reads the side
  off the world so a locator with no name (Pulse's, Pure's) mirrors the same way.
- **Frames** (Talon's Junction, Venom, the autopilot's first pad crossing, the pad force time `0.25 s`, the crossing
  at tick 525 on HD and 523 on Pulse; 1280x720). An A/B of the same tick with the contrails left out
  (a local, uncommitted patch that made `race.wingtip_vertices` return nothing; the Pulse and Pure pairs below toggle the setting instead) changes 11,700 pixels at tick 535, 10,400 at 550 and 4,700 at 565 on HD, all in a band from the two wing tips toward the camera (a diff of the two frames, times 8, is two thin straight
  streaks from the wing tips, widening toward the bottom of the frame). Read at full size and as a crop the streaks are
  thin and pale, at most `0.3` opaque over a blue road, which is what the original's frames read as ("two thin pale
  streaks, one from each wing tip, running back toward the camera", the A/B above). That is the same description, not
  a matched pair: no RPCS3 frame at our tick and pose was taken for the metric on a wing-tip box. The original's own
  natural-crossing frames (the stage-1 poller's timer-refill grabs on the same circuit and class, two of the four read at
  full size and as crops) show no streak readable at the wing tips either, which fits the ribbon being born `0.05` wide at
  most `0.3` opaque: at the moment of the crossing neither side's picture reads at 1:1, so the match on screen is **unknown**,
  not shown.
- **Cold-run check.** HD, 2048 and Omega each load the ribbon and 8 of 8 craft carry both locators; a pad force time on
  slot 0 takes exactly two pool elements and releasing it frees them within the timer's 1.3 s and a node's 0.7 s
  (`crates/raceplay/tests/hd_wingtip_ground_truth.rs`).

### Pulse and Pure behind `graphics.wingtip_contrails` (off by default)

Pulse's and Pure's class tables stop at `0x3eb`, so neither authors a `wingtip` locator. Both hulls do author the
lateral pair of `Ship Collision Fx` locators the HD contrails' points coincide with (HD Feisar and Zone:
`ship_collision_fx2` is the `wingtip` point): Pulse's Assegai has `(2.69, -1.11, -2.19)` where HD's `wingtip_left` is
`(2.78, -1.40, -2.02)`, Pure's Assegai the same pair, Triakis `(2.65, -1.08, -5.23)` against HD's `(2.54, -0.73, -5.44)`.
With the setting on and an HD-lineage source mounted (`oag_game::launcher::wingtip_lender`: HD, else 2048, else Omega)
the race lays HD's ribbon (the lender's config and texture, read through its own archives) from, on each side, the
locator of greatest `|x|`, the rearmost of those within `0.1` of it, as a pure translation. **That anchor is chosen, not
measured**, and so is running HD's speed ramp on a title that never ran it; the report names the line an enhancement.
With no lender the report says so and nothing is drawn. A/B at tick 540 (Pulse) and 556 (Pure) with the setting on and
off changes 8,300 and 4,300 pixels in the same band under the wings; read as crops the streaks are thin and pale, the
Pulse craft's much hidden by its own bright exhaust plume.

## Reproduce

```sh
scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-timetrial-talons-venom-start.SAVESTAT.zst --pause
OAG_RPCS3_ATTACH=1 python3 <film script>   # hold cross; per frame: speed = |*(*(*(ship+0x5fac)+0x270)+0x190)|, root grab
```

The film script is `scripts/rpcs3-hd-trail-film.py` with a speed read from `rpcs3_hd_field.field_rows` added. The
A/B adds a writer thread (`RemoteMem.write` of the big-endian float `1.0` to `*(ship+0x5fac)+0x2e8` every 40 ms from
the seventh frame) and finds the two nodes by a heap scan (`0x30000000..0x36000000`, one second): aligned words equal
to the vtable `0x0086a340` whose `+0xec` is the player's ship; `+0xe0`, `+0xe4`, `+0xe8` read live. The Zone run is
`scripts/rpcs3-hd-zone-probe.py` with its `drive` phase's screenshot interval cut to 6 samples and `trim=False`. Raw
frames are game content and stay under `data/`.

## Other titles

2048 and Omega author the same two `wingtip` locators per hull at the same coordinates as HD, and Omega's executable
carries the importer's strings. Whether either runs the update was not read: **wired by lineage** (labelled
`InheritedFrom("Wipeout HD")` in `oag_2048::TITLE` and `oag_omega::TITLE`), each with its own container's texture
(`/data/tex/smoke_trails_frame1_alpha.gxt` on 2048, `.gnf` on Omega, both 1024x128 and decoded). Both load and lay the two
elements under a pad force time in `hd_wingtip_ground_truth`. Unconfirmed: that either executable runs it. Their trail-hit burst is a separate
matter: their originals never fire it
([Omega](../ghidra/functions/ps4-omega-eu/trail-hits.md), [2048](../ghidra/functions/vita-2048-eu-v104/trail-hits.md)).
Detail: [wingtip-vortices.md](../ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md), "The other titles".
