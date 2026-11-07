# HD's own lock-on sight: two per-tick updates, one shared bind

## Measured live, 2026-11-07: the sight is the PSP law in HD's pixels, with its own colours

Read off the original, running (RPCS3, Wipeout HD Fury EU, Racebox Single Race, Vineta K, Venom, Feisar,
HUD Style HD (the blue palette) and the default Menu Style, on both sides; Fury unknown; 1920x1080 HUD grid), by polling the HUD object through
`/proc` memory and photographing the paused frame at each state change:
`scripts/rpcs3-hd-lock-sight.py` (finds the HUD object by its ten `0x866210`-vtable widget pointers at
`+0x578`, gives the player a pickup state - 1 Missile, 10 LeachBeam, `docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md` -
holds cross, puts the craft behind a rival at its heading and speed with `--behind`, logs a row per poll).
Game time is read off the frame deltas the HUD accumulates (`dt` was `1/30` s in the Missile runs and `1/60` s
in the LeachBeam run: RPCS3 paces frames, never the wall clock). Three Missile runs and one LeachBeam run;
the figures below repeat across them.

### The Missile, `Hud_UpdateMissileSight` (`0x0008a5b8`)

| Field | Reading | Confidence |
| --- | --- | ---: |
| `hud+0xec` | adds `dt` every frame a target is on screen; **not reset when the lock drops**, zeroed only when the target leaves (it read `0.0` on the first frame with none) | 90 |
| `hud+0xdc` (byte) | `1` on the first frame `0xec` passes `0.8` (`0.8000` in three of three runs, `24` frames at `1/30`); drops to `0` on a frame the reticle has not caught its target and is retaken at once while `0xec` is still above `0.8` | 90 |
| `hud+0xe0`, `+0xe4` | **the target's raw projection** in the 1920x1080 grid (the previous frame's, read at `0x0008ab40`/`0x0008ab48` and overwritten with this frame's at `0x0008ab64`/`0x0008ab68`; the aim is `2 * new - old`). The first pass of this page read it as the sight's centre; the chase centre is `hud+0xd4`, `+0xd8` ("Moving pursuit" below) | 92 |
| `hud+0xd4`, `+0xd8` | **the sight's chase centre** in the same grid: read at `0x0008abcc`/`0x0008abd4`, stepped at `0x0008b948`..`0x0008b95c`, snapped to the aim on arrival at `0x0008ad54`/`0x0008ad58` | 92 |
| `hud+0x6b8` (byte) | `1` while not locked, `0` from the lock frame on, `1` again when the lock drops: the one-shot latch of the cue call `FUN_00677518(..., 0x38)` | 85 |
| `hud+0x634` | `+= 4 dt`, wraps at `2 pi`: confirmed on every row | 95 |
| widget `+0x34` bit `0x4` | shown; `+0xac`/`+0xb0` width and height, `+0x12c` rotation (the ring angles turn, so it is a rotation: raised from 65 to 90), `+0xbc..+0xc8` the four vertex colours | 90 |

**The cue `0x38` is `Lockon_Rumble` (56)**, not a sound (`docs/gameplay/rumble.md`, row 56). It is wired:
`oag_game::rumble` raises `Level::LockedOn` while the reticle is locked. Whether HD also plays a sound at the lock was not
looked for in this pass.

**Sizes.** Every ring is `authored * extent / 11.294` with `extent` the PSP law's own half-extent
(`oag_race::sight`: `30` open, `9.6` seeking, `6` locked, eased at `50 * 1.4 = 70` a second): `340` px on the
`128` px `Outer` when open (`30 / 2.65625`), `108.8` seeking. The ring closed `26.5` px a frame at `1/30`
(`795` px/s, `70 * 128 / 11.294`). Before the first sighting the rings sit open and the backdrop hides once they are
fully open. Locked, the original does **not** use the locked extent: `Outer` and `Inner` are both `91.4` px (`Inner`'s
authored `108` at the seeking scale) and `LockedOnLines` stays `108.8`. The seeking size fell from `108.8` to `76`
as the craft closed on the target in two runs, so something in the distance moves it **below** the PSP's `zoom >= 1`:
not fitted, `oag_race::sight`'s zoom is what draws it.

**Colours** (ARGB, written every frame): seeking `Outer` and `Inner` yellow `ffff00`, backdrop and `Middle` and
`LockedOnLines` white; locked `Outer`, `Inner` and `LockedOnMiddle` red `ff0000`, the rest white. The authored red
outer and green inner of the layout never show.

**Motion.** `Middle` is a sawtooth: `0` to half of `Outer` once per `pi/2` of the ring angle (`0.39` s; it
matched `r634 mod (pi/2)` on five rows to the pixel), its alpha climbing with it. Locked, `LockedOnMiddle` shrinks from half of
`Outer` to nothing every `0.25` s and `Outer`'s alpha climbs from nothing to full about every `0.79` s. `Outer` and
`Inner` are written `-30` and `+30` degrees on the lock frame itself.

**What differs from the brief's reading.** The lock frame and the visuals are not simultaneous: `Middle`
is hidden and `LockedOnLines`/`LockedOnMiddle` shown `1` to `11` frames after `0xdc` flips (`0.05` to `0.38` s over three
locks); in the first run `Middle` grew to `Inner`'s size on the lock frame before it hid. So "shrinks then
hides" is wrong, it grows to `Inner`'s size and hides, after a variable delay. Not ported: ours hides it at the lock.

### The LeachBeam, `Hud_UpdateLeachBeamSight` (`0x00090d30`): it has a lock, and it is the PSP's

The brief's open question ("whether the LeachBeam has a lock state of its own") is answered. On a target the four rings
**close from `600` px (`3.409x` the authored `176`) to `120` px (`0.682x`) at `1,400` px a second**, their vertex colour
alpha climbing as they close, and at `120` px **every ring turns from yellow `ffff00` to red `ff0000` at full alpha**. Over
`0.34` s from the first sighting (`600 - 120 = 480` px at `1,400` px/s). That is `FUN_0881e8c8`'s law in
`30 -> 6` extent units at the same `70` a second (`docs/ghidra/functions/psp-pulse-usa/lock-sight.md`, "The LeachBeam's
reticle is its own function"), with `locked = extent == 6`, alpha `6 / extent` and no blink. The three-ring reveal on the `0.5` s
`hud+0x604` loop runs on top of it (the same four bits cycled while the rings closed and while they were locked), which is
what this page already had at 88. The Missile's `0.8` s hold has nothing to do with it. `hud+0x604` runs whatever is held;
the Missile's fields freeze while a LeachBeam is.

### Ours before and after

| | Before | After |
| --- | --- | --- |
| LeachBeam lock on HD's grid | **48 ticks** (the Missile's hold; `race_on` probe) | **20 ticks**, `0.33` s |
| Missile lock on HD's grid | 48 ticks | 48 ticks (unchanged, matches `24` frames at `1/30`) |
| Seeking rings | authored red outer and green inner, authored size, a grey disc for the backdrop | yellow, sized off the extent, `Middle` pulsing |
| Locked rings | the same red/green, the `Middle` swapped for the pair | red, `Inner`'s size, parked at `+-30` degrees |
| LeachBeam rings | authored size and colour, a fixed three-ring loop | closing yellow, red at the lock |

Wired as `oag_title::hud::Sights::Concentric::live` (`LiveSight`: the two extent units), `Sights::runs_leach_law`
(true for HD), `oag_hud::sight_draw::{missile_live_draws, leach_reveal_draws}`. Tests:
`crates/hud/src/reticle_tests.rs::hd_*` (draw), `crates/raceplay/tests/lock_sight_ground_truth.rs::hd_leachbeam_locks_in_its_closing_time_not_the_missiles_hold`
and `::hd_missile_locks_after_the_point_eight_second_hold` (timing; the first reads `48` ticks without the change).

The LeachBeam's fire gate (`oag_raceplay::weapons`) now waits for that lock on HD as it does on Pulse: **inherited from the PSP**, HD's
fire path was not read for it.

### Moving pursuit, 2026-11-07: the chase is the PSP's times 4.0

**Law, confidence 92.** On the target-on-screen path `Hud_UpdateMissileSight` (`0x0008a5b8`) builds the chase rate as
the PSP does and then multiplies it by one more constant:

| Step | Address | Value |
| --- | --- | --- |
| `dt * 30.0` (`f31`) | `0x0008a898`, `30.0` at `0x008a77bc` | the PSP's `step` |
| `rate = (0.12 d + 1.5) * 1.1` | `0x0008abf0`, `0.12` at `0x008a77a8`, `1.5` at `0x008a776c`, `1.1` at `0x008a77ac` | the PSP's, `d` the `0.7`-weighted distance (`0x008a7614`) |
| `rate = 15.0` once `hud+0xec > 0.8` and the lock byte is set | `0x0008ac08`, `15.0` at `0x008a77b8` | the PSP's |
| `zoom = clamp(30 / w, 1, 5)` | `0x0008ac18`..`0x0008ac38`, `5.0` at `0x008a7710` | the PSP's |
| **`f11 = rate * zoom * 4.0`** | `0x0008ac3c` (`fmuls f0,f10,f0` with `4.0` at `0x008a7770`), `0x0008ac40` | **new: the PSP has no factor** |
| arrival: `f11 * dt * 30.0 > d` | `0x0008ad3c`, `0x0008ad48` | the PSP's compare, so the radius is four times the PSP's |
| chase: `centre += (aim - centre) * (f11 * dt * 30 / d)` | `0x0008b934`..`0x0008b960` | the PSP's, four times the step |

`4.0` is the HUD grid's width over the PSP's (`1920 / 480`): HD rescaled the chase into its own pixels. Every other
constant is the PSP's, unscaled. The `1.0` seed the first page took for `f11` (`0x008a75bc`) is the no-target path
(`0x0008acf8`..`0x0008ad0c`), where the rate is `1` and nothing is multiplied. `r22` is only "a target is on screen this
frame" (`li r22,1` at `0x0008ac20`, `0` at `0x0008ad20`), not a second gate. The lock byte is set at `0x0008b7ec` from
`0x0008ad70` (`hud+0xec > 0.8`) on the arrival path, exactly as the PSP's.

**Why the first pass saw no gate.** It logged `hud+0xe0/0xe4` as the sight's centre. That is the *target's* projection, so
"the centre moved 7 to 8 px a frame at the lock" was the target moving; the real centre (`0xd4/0xd8`) was never logged.
With `4.0` the steady-state lag behind a `7.4` px/frame target is about `1.5` px and the arrival radius about `14` px, so
the reticle is on its target every frame.

**Live, confidence 92.** RPCS3, Wipeout HD Fury EU, Racebox Single Race, Talons Junction, the player teleported 60 units behind a
rival at its heading and speed (`scripts/rpcs3-hd-lock-sight.py --behind 60`, which now also logs `hud+0xd4/0xd8` and picks the HUD
object by its widgets' authored sizes: the scan alone also matches runs of image widgets elsewhere in the heap), cross held,
`dt = 1/30`. Over the 8 recorded chase frames with a clear zoom of `1` (the target past 30 units),
the centre moved **`4.00` times** (`3.99` to `4.00`) the step `dt * 30 * 1.1 * (0.12 d + 1.5)` the PSP's law gives, at `d` from
`16.7` to `1,168` px; and on the arrival frames the centre equalled the aim exactly from `12.8` and `9.0` px away,
where `3.3` and `2.8` px is all a gain of one allows. The lock byte (`hud+0xdc`) set on the first frame after `hud+0xec` passed
`0.8` (`0.8002`) with the centre on the aim and held for the 42 frames to the end of the sighting, with the target moving `3` to `150` px a
frame. Pinned in `crates/race/src/sight/tests.rs::hd_chases_at_four_times_the_psp_step_as_the_original_does` (the eight frames
as a table; the PSP's step misses each by over a pixel) and `::a_target_crossing_the_screen_locks_with_the_hd_gain_and_never_without_it`.

**Ours before and after.** Same pursuit on the disc (`crates/raceplay/tests/lock_sight_ground_truth.rs::hd_missile_holds_its_lock_on_a_craft_it_is_chasing_across_the_screen`:
single race, cross held 400 ticks, a Missile always in hand, 1,500 ticks). Ticks 48 to 371 are the grid's lock on a parked
target and say nothing about the chase, so the figure is the moving phase (slot 0 past 20 m/s): HD locked on **111 of 360**
visible ticks (`31 %`) before and **325 of 360** (`90 %`) after (all phases: 359 of 656 before, 573 of 656 after); Pulse
585 of 634 over all phases, before and after, bit for bit (its gain is `1.0`). The first lane's "0 of 318" did not reproduce
on this tree (its scenario was not kept); the shortfall did.
Wired as `oag_title::hud::LiveSight::chase_gain` (`4.0`; `Sights::chase_gain()` is `1.0` wherever `live` is `None`),
`oag_raceplay::Sighting::chase_gain`, `oag_race::sight::Sight::set_chase_gain`, applied as `rate *= zoom * gain` on the
target-on-screen path only. The simulation's per-slot locks and the drawn reticle take the same value from the same
`Sighting`, so the red, the tone and the shot agree. Only person-flown slots run a `Sight` (`Race::advance_locks`: opponents
have no lock, their shot takes the window's candidate at once), so HD's AI is untouched; a player's Missile shot on HD now
finds its lock taken far more often while chasing, as the original's does.

**Not covered.** `Hud_UpdateLeachBeamSight` (`0x00090d30`) was not read for a gain: the LeachBeam's lock is its own closing
law, and its centre chase on HD keeps the Missile's unscaled step in ours. Open.

### Open, and checked against 2048 and Omega

- ~~**A moving pursuit never locks in ours on HD, and does on Pulse: the reproduction of the maintainer's "not as in Pulse".**~~
  **Resolved 2026-11-07, "Moving pursuit" above**: the reading below took `hud+0xe0/0xe4` for the sight's centre and missed the
  `4.0` at `0x0008ac3c`. Kept for its evidence. The original text:
  The first lane's reading, kept for its evidence (its centre was the target's projection, and the `4.0` was missed):
  same scenario on both discs (single race, cross held 400 ticks, a Missile always in hand, 1,500 ticks): Pulse
  `272` ticks visible, `141` locked; HD `318` visible, `0` locked. In three runs `hud+0xdc` flipped on the first frame
  `hud+0xec` passed `0.8` while `hud+0xe0` was moving `7` to `8` px a frame, and was held while it sped to `40` px a frame;
  `r22` was not traced.
- The distance dependence of the seeking size; the `Middle` growth before it hides; the sound, if any, at the lock; the
  alpha ramp of `Outer` and `Middle` before they are full (ours multiplies the law's own alpha).
- **2048 and Omega**: same widget names (`Sights::Concentric` rows), checked, applies, not wired: `live: None` there, so
  their chase gain stays `1.0`. Their updates are not located: the bind functions are `FUN_81158d88` (2048 v1.04) and
  `FUN_0126c020` (Omega), the sight widgets' offsets are read there, and a search for the PSP's `0.12` literal
  (`8f c2 f5 3d`) turns up no function that reads the sight's widget offsets (Omega's only xref'd hit is `FUN_016372a0`, a
  10 KB render function). Omega draws HD's `1920 x 1080` HUD space, so the same `4.0` is the expectation there; 2048's grid is
  not HD's, so its gain is not guessed. Next: search both executables' functions for the widget offsets the bind stores at.
- The LeachBeam's own centre chase on HD (`Hud_UpdateLeachBeamSight`, `0x00090d30`) was not checked for the gain.
- A Fury HUD Style was not compared: both sides ran HUD Style HD (the blue palette).

The sections below are the earlier readings, kept for their evidence; where they disagree with this one, this one wins.


## Correction, 2026-10-19: there is no 0.5 s hold, and the Missile has a real locked state

Disassembling both functions end to end (`disassemble_function`, the decompile
of `0x0008a5b8` stops at `halt_baddata`) overturns two readings below:

- **`0x008a764c` = 0.5 is a pooled literal, not a hold time.** `Hud_UpdateMissileSight`
  multiplies the split-screen offsets by it, and `Hud_UpdateLeachBeamSight`
  adds the raw frame time to `hud+0x604` and **zeroes it at 0.5** (`0x00090fac`
  to `0x000910b8`): a looping animation clock, not a lock accumulator. The
  Missile sets its lock byte (`hud+0xdc`, `0x0008b7ec`) when `hud+0xec`
  exceeds the literal at `0x008a777c` = `0x3f4ccccd` = **0.8**, the PSP's hold.
  (Where `hud+0xec` is accumulated is not in this function; it is zeroed on
  the no-target path.) So HD holds for the same 0.8 s and no per-title hold
  axis is warranted. Confidence 85.
- **The LeachBeam's three-ring reveal is that 0.5 s loop**, so it cycles
  whether or not a lock is building. Shown: Outer to 0.125, Middle to 0.25,
  Inner to 0.375, backdrop alone to 0.5, then again. Confidence 88.
- **The Missile's locked state is not additive.** Seeking (`hud+0xdc` = 0):
  `LockedOnLines`/`LockedOnMiddle` hidden (bit `0x4` of `+0x34` cleared),
  `Middle` shown; `Outer` and `Inner` get `+0x12c` = `hud+0x634` and its
  negative, `hud+0x634` growing by `dt * 4.0` (`0x008a7770`) and wrapping at
  `2 pi` (`0x008a778c`). Locked: a one-shot cue call latched by `hud+0x6b8`
  (`FUN_00677518(..., 0x38)`, not wired here), `Middle` shrinks then hides,
  `LockedOnLines`/`LockedOnMiddle` shown, `Outer`/`Inner` written the
  literals `0xbf060aa6`/`0x3f060aa6` (minus and plus 30 degrees,
  `0x008a7798`/`0x008a779c`). `+0x12c` being a rotation is inferred (`+0xac`
  and `+0xb0` are width and height, `+0x5c`/`+0x60` their getters), not
  proven: confidence 65. Wired in `oag_hud::sight_draw`; the Middle ring's
  shrink-before-hide is not ported (it hides at once).
- Checked on 2048 and Omega: both author the identical four-seeking, two-locked
  widget names, so the same drawing applies (**ported**, no per-title code);
  neither executable's own update was read (**not checkable** for Omega, 2048
  has no HD-style sight function located).

The sections below are the 2026-09-15 reading, kept for its evidence; where
they say "hold" for `0x008a764c` or "progress", read the correction above.

# (original page)

2026-09-15. Located from the widget-name strings themselves (`search_strings`
on `"Sight"`), TOC-resolved with `scripts/ps3-toc.py` and decompiled through
the bridge. All three addresses below are TOC-`exact`.

Read [`race-hud.md`](race-hud.md) first for `Hud_LoadDefinition` and the
per-mode HUD file set; this page is the runtime side, once a HUD object
exists and is ticking.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0009bb30` | `Hud_BindWidgets` | 80 |
| `0x0008a5b8` | `Hud_UpdateMissileSight` | 78 |
| `0x00090d30` | `Hud_UpdateLeachBeamSight` | 78 |

## Finding the bind site: ten strings, one function

`search_strings("Sight")` on this binary returns exactly ten hits, all
contiguous in `.rodata`:

```
0077f890 MissileSightBG
0077f8a0 MissileSightOuter
0077f8b8 MissileSightInner
0077f8d0 MissileSightMiddle
0077f8e8 MissileSightLockedOnLines
0077f908 MissileSightLockedOnMiddle
0077f928 LeachBeamSightBG
0077f940 LeachBeamSightOuter
0077f958 LeachBeamSightMiddle
0077f970 LeachBeamSightInner
```

These addresses sit at `0x0077f890`-`0x0077f990`, which is inside this
binary's TOC overlap-and-beyond region (above `0x758110` per
[`memory.md`](memory.md#there-are-exactly-two-tocs-and-they-overlap-in-address-space)),
so `get_xrefs_to` on any of them returns nothing - not because they are
unused, but because Ghidra's own analysis resolved every TOC-relative load in
this range against the wrong TOC and never built the reference in the first
place. `scripts/ps3-toc.py attrib`, which resolves through each function's
*own* OPD-declared TOC rather than Ghidra's uniform one, finds the real
caller for all ten in one function: **`0x0009bb30`**.

That function is not sight-specific. It is a ~2,700-instruction, straight-line
sequence run once per HUD construction (guarded by a "did I already run" byte
at `param_1+0x161`) that resolves several hundred widget names to pointers -
`ForwardHUD`, `SpeedClassParent`, `PickupBackground`, the eight-wide
`LapBar%d`/`PosBar%d` progress-arc loop, the Zone ladder, and so on - of which
the ten sight widgets are one small unconditional run in the middle, no
different in shape from the widget resolution around it. Named `Hud_BindWidgets`
rather than anything sight-specific, since renaming it for the ten this page
cares about would misdescribe the other few hundred.

### The ten widgets' offsets in the HUD object

`search_byte_patterns` on each string's own address finds its TOC slot
directly - the ten are themselves contiguous, `0x008a7ac8`..`0x008a7aec` in
strict string order - and each slot's displacement from this function's own
TOC (`0x008ad4d8`, confirmed `exact`) lines up byte-for-byte with the sequence
of `_opd_FUN_0067e6d0(param_1, <name>, 0)` calls (the resolve-by-name helper)
and the offsets their results are stored at:

| Widget | Offset (`param_1 +`) |
| --- | --- |
| `MissileSightBG` | `0x578` |
| `MissileSightOuter` | `0x57c` |
| `MissileSightInner` | `0x580` |
| `MissileSightMiddle` | `0x584` |
| `MissileSightLockedOnLines` | `0x588` |
| `MissileSightLockedOnMiddle` | `0x58c` |
| `LeachBeamSightBG` | `0x590` |
| `LeachBeamSightOuter` | `0x594` |
| `LeachBeamSightMiddle` | `0x598` |
| `LeachBeamSightInner` | `0x59c` |

All ten resolve in one unconditional run with no branch between the Missile's
six and the LeachBeam's four - binding tells nothing about which is shown
when; that is the two functions below.

## Two separate per-tick updates, not one shared switch

An exhaustive `search_instructions` for `lwz ...,0x578(...)` through
`0x59c(...)` across all 1,829,838 instructions in the program turns up one
function that touches the Missile's six offsets in the sight-specific way
(lock search, easing, colour writes) - `0x0008a5b8` - and a **second, separate**
function doing the structurally identical thing to the LeachBeam's four -
`0x00090d30`. (Both offset ranges also turn up in a handful of unrelated
functions touching unrelated structs at the same small numeric offset - noise
inherent to a byte-offset search, not additional sight code; the two named
functions are the ones whose reads land inside the sight-shaped lock-search-
and-colour-write block, not a bare isolated load.)

So HD does not read a "which weapon is held" flag inside one shared sight
update the way this project's own `oag_race::sight::Held` does - it runs a
**whole second copy of the update**, keyed by which pool of eleven-plus-widget
weapon systems is active (the same `WeaponManager` gating
[`weapons.md`](weapons.md) already found branches on mode and a shared byte).
Both functions:

- resolve "my own" `RaceManager`/player-slot pointer through an identical
  eight-slot-then-fallback lookup (`+0xe8` through `+0x104`, then `+0x13e8`) -
  boilerplate, not sight law.
- run a **lock-target search** over live craft: pairwise squared-distance and
  an angle test built from `vectorReciprocalSquareRootEstimateFloatingPoint`
  and a `vectorCompareGreaterThanFloatingPoint` against a threshold constant -
  the RSX-shader-style vectorised equivalent of the PSP's scalar `w > 0` /
  screen-bounds gate in [`lock-sight.md`](../psp-pulse-usa/lock-sight.md).
- accumulate a **hold timer** (`param_1` is the per-tick delta time in
  seconds) and compare it against the *same shared constant address*,
  `DAT_008a764c` - see below.
- ease a per-widget "extent" value toward the resolved target and write it
  through each widget's own vtable setters (`+0x68`/`+0x70`, x/y) and
  `Image_SetVertexColours` for the tint - the same "chase and tint" shape
  `HudSight_Update` uses on the PSP, restated as PS3 render-object method
  calls instead of direct field writes.

Neither function ever touches the other's six/four offsets. This directly
answers the brief's (a): **the two dialects are not one function switching on
held weapon - they are two independent, structurally-mirrored update
functions**, each hardwired to its own six or four widget pointers.

## The hold time is 0.5 s, not the PSP's 0.8 s

Both functions gate their lock on the *same* constant address,
`0x008a764c`. Read directly (`read_memory`, 4 bytes, big-endian IEEE-754):

```
008a764c: 3f 00 00 00  =  0.5f
```

`bVar24 = DAT_008a764c <= fVar2;` where `fVar2` is the running hold
accumulator (`param_1 + previous accumulator`, reset on loss of target) - the
exact role [`oag_race::sight::HOLD_SECONDS`] plays for `0.8`. **This is a
concrete, directly-read value, not an inference**: one memory read, one
unambiguous bit pattern, referenced identically by both the Missile's and the
LeachBeam's update, so it is HD's one shared hold time for both weapons, and
it disagrees with the PSP's `0.8` measured in
[`lock-sight.md`](../psp-pulse-usa/lock-sight.md#the-hold-timer-and-where-it-lives).

**Not adopted here.** `oag_race::sight::HOLD_SECONDS` is shared, title-blind
engine code judged against the PSP's own recovered law; changing it for HD
alone is a simulation-behaviour change, and this lane's brief is
presentation-only and explicitly defers a change like this to a report rather
than a commit. The fix wants either a per-title hold constant or a measured
argument that `0.8` is close enough, and either is a `oag-race`/gameplay
change outside this lane's file ownership - recorded in this project's
handover tracker under the existing HD lock-on thread.

## The LeachBeam's four reveal one at a time, gated by hold-time not distance

**2026-09-15, corrects this page's own earlier reading.** This section
previously read the four widgets' bit-`0x4` toggles as gated by "distance
breakpoints" off a resolved per-weapon stats pointer. Disassembling the
exact instructions that produce `iVar16 - 0x5d14` etc. shows they are
`lfs fN, -0x5d14(r2)` - **TOC-relative loads, not pointer-plus-offset reads
off a runtime struct**. `iVar16`/`iVar17` in the decompile is Ghidra's
label for whatever currently occupies the stack slot this function's own
TOC-preservation dance (`puVar10[5] = uVar14; ... = puVar10[5];`, needed
because callee TOCs differ from the caller's in this binary) has parked
there at that point - not a weapon-stats pointer at all. `0x00090d30` is
below `0x32d5e0`, so per [memory.md](memory.md) Ghidra's own TOC
(`0x008ad4d8`) is the right one here, confirmed directly:
`scripts/ps3-toc.py toc 0x00090d30` prints `exact`.

Resolving the three loads through that TOC and reading the slots
(`scripts/ps3-toc.py resolve 0x00090d30 -0x5c98` etc., cross-checked against
a direct `read_memory` at each computed address) gives three **literal,
static** IEEE-754 floats:

| Load site | Slot | Value |
| --- | --- | --- |
| `lfs f0,-0x5c98(r2)` @ `0x00091460` | `0x008a7840` | `0x3e000000` = **0.125** |
| `lfs f0,-0x5d14(r2)` @ `0x0009147c` | `0x008a77c4` | `0x3e800000` = **0.25** |
| `lfs f0,-0x5c94(r2)` @ `0x00091a9c` | `0x008a7844` | `0x3ec00000` = **0.375** |

**All three are exact quarters of `DAT_008a764c` (`0.5`), the same hold
constant this page's previous section already read.** The value the three
are compared against, `fVar2 = *(float *)(param_2 + 0x604)`, is the same
field the top-of-function lock check reads and increments by `param_1`
(the tick's delta seconds) before comparing it to `DAT_008a764c` - so this
is **the hold-time accumulator itself, in seconds**, not a screen or world
distance. The reveal is gated by *how far into the 0.5 s hold the lock
currently is*, at each quarter.

The bit-`0x4` direction reads unambiguously from the function's own
no-target early return: `*(uint*)(BG+0x34) &= ~4` is the last thing it does
before returning "nothing to show" - so **clearing bit `0x4` hides a widget
and setting it shows one**, the opposite of neither being obviously right
from the toggle sites alone. Restated as a table, `t` being the hold-time
accumulator in seconds and BG (`+0x590`) shown throughout once a target is
being tracked:

| `t` | Outer (`+0x594`) | Middle (`+0x598`) | Inner (`+0x59c`) |
| --- | --- | --- | --- |
| `t <= 0.125` (0-25% of hold) | shown | hidden | hidden |
| `0.125 < t <= 0.25` (25-50%) | hidden | shown | hidden |
| `0.25 < t <= 0.375` (50-75%) | hidden | hidden | shown |
| `t > 0.375` (75-100%) | hidden | hidden | hidden |

So the four rings do not snap on together and do not stay filled once
revealed: each of the three outer rings gets **one exclusive quarter** of
the hold window, and the last quarter (`t > 0.375`) shows only the BG
before the lock completes at `t > 0.5`. This is a LOD/reveal sequence in
the same spirit as the PSP's brackets closing in, but built from time
fractions rather than a closing bracket's own extent.

**Confidence 88** on the three values and the table above: the constants
are direct memory reads at TOC-exact-resolved addresses, corroborated by
`scripts/ps3-toc.py` independently of the MCP bridge, and the clean
quarter-of-`0.5` values are exactly the kind of authored-not-coincidental
number a hand-tuned reveal schedule would carry. What is not independently
confirmed: the outer state machine that decides *when* this whole block
runs versus the function's several other branches (still the per-frame
"is a target currently being tracked" gating this page's earlier section
already flagged as not fully decoded) - so the table above is right for the
frames it runs on, and exactly which frames those are is not proven past
"while the hold accumulator is live and below the completion threshold."

**Not adopted here either, for the same reason `HOLD_SECONDS` is not**:
`oag_race::sight::Sight` has no accessor for the hold-time accumulator or a
0-1 progress fraction derived from it - only `locked()` and `visible()`,
both hard booleans - so `oag_hud::sight_draw` cannot drive this table
without a new `crates/race` API. `oag_title::hud::Sights::Concentric::leach`
still draws all four together in code and its own doc comment still says
chosen, not measured, for the *runtime* behaviour; what moved is that the
reveal law itself is no longer unread. See this lane's report for the
concrete ask.

### The Missile shares one of the three constants, but was not fully re-derived

`Hud_UpdateMissileSight` (`0x0008a5b8`) reads `-0x5d14(r2)` too, at
`0x0008b234` - the **same TOC slot**, `0x008a77c4` = `0.25`. It is compared
against a value accumulated in that function's own `param_2+0xec` field at
its own per-frame rate, not against `param_2+0x604`, so the same `0.25`
constant is corroborated as a shared "halfway" reveal threshold without this
pass having proven the Missile's own accumulator is unit-for-unit the same
"seconds into the hold" quantity the LeachBeam's is. Structurally the
Missile's transition is a **single** step, not three: crossing this
threshold hides `MissileSightMiddle` (`+0x584`) once and sets
`LockedOnLines`/`LockedOnMiddle` (`+0x588`/`+0x58c`) visible, guarded by a
"have I already done this" check on `MissileSightMiddle`'s own hide bit
rather than being re-evaluated every frame the way the LeachBeam's three-way
table is. That is a real, structural answer to this page's own open question
about whether [`oag_title::hud::Sights::Concentric::locked`]'s "locked adds
two more" reading (confidence 70) is an all-or-nothing pair-add or a staged
reveal like the LeachBeam's: **it is closer to all-or-nothing than to a
three-stage reveal**, gated by one threshold rather than three. Confidence
**60** - the toggle statements and the shared constant are read directly,
but the full surrounding state machine in this larger, more heavily
vectorised function was not traced to the same depth as the LeachBeam's.

## Not read this pass

- The full outer state machine gating when `Hud_UpdateLeachBeamSight`'s
  widget-toggle block runs at all, across the function's several branches -
  see above.
- `Hud_UpdateMissileSight`'s own accumulator (`param_2+0xec`) and whether it
  is genuinely hold-seconds like the LeachBeam's, or a distance-derived
  ramp that happens to cross the same `0.25` constant.
- The far-target alpha (96/255 on the PSP) - the colour writes here go through
  `Image_SetVertexColours` with a computed alpha byte, and whether any of that
  arithmetic is the same 96/255 ratio was not checked against the PSP's
  constant.
- Which of the two functions - or a third, unfound one - runs when *no*
  lockable weapon is held at all (both refuse early on `*(int*)(iVar41 or
  iVar22 + 0x5edc) == 0`, a per-weapon-system pointer being null, but nothing
  here traces where that pointer comes from beyond "resolved earlier in the
  same eight-slot player lookup").
