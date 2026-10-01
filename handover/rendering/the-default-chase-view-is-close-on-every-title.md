# The default chase view is close on every title, and only Pulse PSP's default is measured

2026-10-01. `CameraView::default()` and the settings default are `Close` for every
title, a maintainer decision taken the same day: close is assumed to be the default
in every title until a title's own original says otherwise. The measurement behind it
is Pulse PSP's alone: two cold boots of the original with an empty profile fly
`OPT_CLOSE`, eye `(-11.25, +3.0)` in the craft's frame, 11.643 units back (far would
be 14.562). See `camera.md`, "The default view is `OPT_CLOSE`, measured 2026-10-01"
([camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md)). That boot flew
Venom/Assegai, so Assegai's authored blocks are the reference below.

## What the discs author, per title

Read straight off each disc's `handlingstats.xml` on 2026-10-01: `oag-wad cat --expand`
for Pulse and Pure, `scripts/psarc.py` on the decrypted PS3 image for HD and on the
extracted Vita `data.psarc` for 2048. Every block carries the same seven attributes
(`fov`, `lookat_height`, `lookat_length`, `pos_height`, `pos_length`, `spring_horiz`,
`spring_vert`). `fov` is 60 on every racing team of every title, and the
`ExternalCloseCamPitchMod`, `ExternalFarCamPitchMod`, `ReplayCamPitchMod` and
`CameraSideOffset` blocks of the global `HandlingStats.xml` are identical on all five.

Reference, Pulse PSP's Assegai: **close** `pos (height 4, length -15)`, look-at
`(height 0, length 25)`; **far** `pos (4, -19)`, look-at `(0, 20)`. These are authored
values. The runtime scale is a separate per-title fact (last column).

| Title (images read) | Close block vs Pulse PSP | Far block vs Pulse PSP | Default view | Runtime scale |
| --- | --- | --- | --- | --- |
| Pulse PSP (USA and EU, identical) | the reference. All eight teams and `Zone` agree on `pos`, look-at and `fov`; only the springs vary per team. `Zone_01` has `pos_length -15.5` | six teams agree with the reference. `AG_Systems` and `Triakis` author `pos_height 5`, so their far differs; `Zone_01` has `pos_length -19` | **measured** `OPT_CLOSE`, two cold boots | **measured 0.75**: close 11.643, far 14.562 (Assegai) |
| Pulse PS2 (EU) | **identical** to the PSP on all 14 shipped directories (12 teams, `Zone`, `Zone_01`). The four DLC teams are on this disc and match too | `pos` and `fov` identical. **Look-at differs on all 14: `(6, 28)` against the PSP's `(0, 20)`** | assumed close, unmeasured | unmeasured. `camera.md` records no x0.75 anywhere in the PS2's `Camera_UpdatePlayerView` |
| Pure PSP (USA and EU, identical) | **identical** on the six base teams Pure shares with Pulse PSP, on `Zone`, and (against the PS2) on `Auricom` and `Harimau`. `Zone_01` differs (`fov 70`, `pos (3.0, -9.5)`, springs 18/14), and so does `Medievil` (`pos (3.0, -12)`) | identical on the same teams. `Zone_01` differs (`fov 70`, `pos (3.8, -15)`, springs 16/12) and so does `Medievil` (`pos (3.5, -18)`) | assumed close, unmeasured | unmeasured; no counterpart of `g_craft_scale` is recorded for this binary |
| HD / Fury (EU) | **differs on all 12 teams.** `pos_height` 3.4-3.7, `pos_length` -10.5 to -12.4, look-at `(-4, 25)`. Authored distance 11.07 (Triakis) to 12.86 (Piranha) | **differs.** `pos_height` 4 on all, `pos_length` -14 (Icaras) to -16.5 (Harimau), look-at `(0, 25)`. Authored distance 14.56 to 16.98 | assumed close, unmeasured | unmeasured |
| 2048 (Vita, USA and EU, identical) | its 12 HD-derived teams (`hdships`) are **camera-identical to HD's own files** (39 compared). Its five native teams, 20 craft files: `pos_height` 3.1-3.7, `pos_length` -10 to -11.4, look-at `(-4, 25)`, authored distance 10.5-11.9 | native: `pos_height` 3-4, `pos_length` -13 to -15.3, look-at `(0, 25)` (`35` on one Feisar craft), `spring_horiz` 5 or 7 against 10-11. Authored distance 13.3-15.7 | close **declared** by its own options definition (the picker starts on `OPT_CLOSE`), not measured | unmeasured |
| Omega | out of scope (no racing) | - | - | - |

What it says:

- **Pulse PSP, Pulse PS2 and Pure author one close rig**, and the PS2's far look-at is
  the one authored difference among the three. A per-title offset table is not needed
  for them.
- **HD's and 2048's authored offsets are about a quarter smaller than Pulse's.** Pulse
  PSP's authored close is 15.5 away and the player sees 11.64 (the 0.75 scale). HD's
  authored close is 11.1-12.9 and its authored far 14.6-17.0, which is the distance a
  Pulse PSP player sees at close and far. **A hypothesis, not a measurement:** HD and
  2048 may run at scale 1.0 with the on-screen distance authored directly. If so,
  Pulse's 0.75 applied to them puts HD's close eye about 8.3-9.6 units back where the
  original's would be about 11-13. The engine applies
  `oag_physics::hover::TARGET_GLOBAL_SCALE` (0.75) to the eye of every title today
  (`ChaseParams::craft_scale`, `crates/game/src/race/camera.rs`).
- HD's close look-at is 4 units below the craft where Pulse and Pure aim level. Whether
  our rig honours that for HD is not checked here.

Per-team rows are not reproduced (ADR-0006).

### Not read, or not conclusive

- Pulse PSP's four DLC teams (`Auricom`, `Harimau`, `Icaras`, `Mantis`) and Pure's
  `Vanuber` live in `data/dlc/`, not on the base discs. Not read.
- 2048's `dlc1`, `dlc2` and patch archives carry no `handlingstats` entry (EU and USA),
  so nothing there overrides the base.
- HD's `Test` ship and its `feisar/handlingstats_traditional.txt` both author **Pulse's**
  blocks, not HD's own. Nothing found in this project reads either, and nothing found
  says what does. They are left out of the HD ranges above, as are the `zone`,
  `zone battle` and `detonator` craft.
- HD's `_c1` and `_n1` Fury files match each other but differ from the plain file on
  9 of 12 teams; the ranges above are the plain files'.

## Open

- Each unmeasured title's default view, read from the running original on an empty
  profile (PCSX2 for Pulse PS2, PPSSPP for Pure, RPCS3 for HD/Fury and 2048).
- **The runtime scale of the eye per title.** The authored offsets answer "do the
  blocks differ" (HD and 2048 do, and so does the PS2's far look-at). They do not say
  what distance the player sees. Pulse PSP's 0.75 is `g_craft_scale`, a code literal;
  the PS2's camera function has no such multiply; Pure, HD and 2048 are not looked at.
  Settled by finding the equivalent of `Craft_Construct_q`'s literal in each binary
  (Pure's `BOOT.BIN`, HD's `EBOOT.elf`, 2048's `eboot.elf`), or by reading the eye in
  the craft frame off a running original the way `scripts/psp-camera-pair.py` did.
- Whether HD's `lookat_height` and 2048's `head_tilt` (an attribute of its native
  `ExternalCamera*` blocks only, which `ExternalCamera::from_node` does not read) have a
  consumer. Nothing in the tree reads either today.
- Two ground-truth fixtures calibrated to the far eye now pin `Far` explicitly
  (`hd_engine_flare_ground_truth`, `sfx_ground_truth`); revisit them if HD's default
  turns out to be measured differently.

## Next Steps

1. The cheaper check first, static, per binary: does a 0.75-shaped craft scale reach
   the camera in Pure, HD and 2048. If HD and 2048 turn out to be 1.0,
   `ChaseParams::craft_scale` becomes a per-title value (a code lane, not a docs one).
2. Measure the default view per title on its emulator with an empty profile, the way
   `scripts/psp-camera-pair.py` did for Pulse PSP, and read the eye in the craft frame
   in the same boot: that gives the default and the scale together.
