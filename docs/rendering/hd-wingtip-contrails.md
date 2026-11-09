# HD's white nose glow: not an effect of its own, and no wingtip contrails

Measured 2026-11-09 (lane `hd-nose-glow-control`) on RPCS3 (EU Fury image, recompiler, Vulkan, `Write Color
Buffers` on), four boots and restores. The claim under test, written before the first capture:

> In the original, the compact white glow at the craft's nose appears at race speed with no rival trail ahead,
> so it is an effect separate from the trail-hit burst.

**It came out false.** The glow is not there in a Time Trial or a Zone run at race speed. The trail-hit page's
reading stands: it belongs to the burst ([engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md),
"The trail-hit burst as a matched pair"). The maintainer's white contrails at the Zone ship's wing tips were not
reproduced, and the game's own code does not read the `wingtip` locators at all
([wingtip-vortices.md](../ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md)).

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
at 103 to 154 units/s (about 370 to 550 km/h if the HUD's ratio holds): **no white glow or contrail at the Zone
ship's nose or wing tips.** The horn tips (the forward-swept wing tips where the `wingtip` locators sit) stay dark;
the white streaks that appear in some frames are the scenery's speed lines and cross the whole frame, not the tips.
This does not cover Zone speeds above 154 units/s or a boost; neither was reached.

## The burst, for contrast

Five tucked frames per condition in `hd-race-talons-venom-pack` (the player placed 8 units behind the nearest rival
along its velocity, 30 frames, `rpcs3-hd-trail-hit-pair.py`'s tuck, the whole run in one connection). The craft and the rival read
`+0x7d2c = 1` on all ten tucks (the red variant, Fury skin), which does not match the state's table entry in
[emulator-recipes.md](../reverse-engineering/emulator-recipes.md) (it says classic; the entry should be rechecked,
not rewritten from five reads).

- **Burst names intact:** white ray fans and a white glow around the player's nose in the four frames read (speed 133 to 150 units/s).
- **Burst names broken** (the first byte of both bare names overwritten, `rpcs3-hd-trail-hit-names.py both`): none at
  the player's nose in the four frames read; what white remains is the rivals' ribbons and engine flares.

The poses are not matched (the Autopilot steers the tucked craft differently each time), so this is a contrast, not
a pair, and carries little weight on its own. Together with Control B it says the nose glow needs the burst.

## What this settles, and what it does not

| Question | Answer | Confidence |
| --- | --- | --- |
| Is the white nose glow present at race speed with nothing ahead? | **No**: 0 of 26 Time Trial frames, 0 of 10 Zone frames read | 85 (two restores, two boots, one circuit and one hull per control) |
| Is it a separate always-on, speed or Zone effect? | **No** | 85 |
| Is it the burst's? | **Yes, by exclusion and the earlier A/B** (the emitter drop on ours, the original's names-broken frames of 2026-11-08); the matched pair is not reproduced here | 75 |
| Do HD's `wingtip` locators draw anything? | **No code reads them** (collected into the craft, never read) | 80 |
| What are the maintainer's white contrails? | **Unexplained.** Not seen at 150 units/s or below on the Feisar or the Zone ship | n/a |

For the contrails the unmeasured conditions are what to try next: a Zone speed class above Flash, a speed-pad boost,
and the airbrake. A frame of the same ship in the original and in ours at the
same pose and speed would also show whether what the player sees is the engine ribbon's edge catching the light.

## Reproduce

```sh
scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-timetrial-talons-venom-start.SAVESTAT.zst --pause
OAG_RPCS3_ATTACH=1 python3 <film script>   # hold cross; per frame: speed = |*(*(*(ship+0x5fac)+0x270)+0x190)|, root grab
```

The film script is `scripts/rpcs3-hd-trail-film.py` with a speed read from `rpcs3_hd_field.field_rows` added; the
Zone run is `scripts/rpcs3-hd-zone-probe.py` with its `drive` phase's screenshot interval cut to 6 samples and
`trim=False`. Raw frames are game content and stay under `data/`.

## Other titles

2048 and Omega author the same two `wingtip` locators per hull at the same coordinates as HD, and Omega's executable
carries the importer's strings. Whether either draws anything from them was not read. Their trail-hit burst is a
separate matter: their originals never fire it
([Omega](../ghidra/functions/ps4-omega-eu/trail-hits.md), [2048](../ghidra/functions/vita-2048-eu-v104/trail-hits.md)).
Detail:
[wingtip-vortices.md](../ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md), "The other titles".
