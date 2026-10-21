# Pure's chase-camera rig, and the 0.75 craft scale it applies

Reads whether Pure scales its external chase eye the way Pulse PSP does (measured
there: [`psp-pulse-usa/camera.md`](../psp-pulse-usa/camera.md), "The 3/4 factor is
`g_craft_scale`"). Read headless on 2026-10-01, `psp-pure-usa` `BOOT.BIN` (image
base `0x08804000`), decompiler plus the instruction stream. **Static only**:
nothing here was read from a running Pure. Names went into
[`names.tsv`](names.tsv) in the same change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089261ac` | `Craft_Construct` | 80 |
| `0x0892a7d0` | `Ship_UpdateCameraRigs` | 85 |

## The answer: yes, 0.75, applied to the eye and the look-at, after the spring

**The write.** `FUN_089261ac`, the ship constructor, loads `0x3f400000` (`0.75`) at
`0x08926270` and stores it, at `0x08926290`, to the global the database shows as
`_DAT_002afd08`. It then stores `1.0 / 0.75` to `_DAT_002afd0c` at `0x089262c8`
(Pulse stores the literal `1.3333334`; Pure divides, and guards the first store
with a compare against zero that stores the same value on both arms). The
constructor is also where `"%s external close tripod"` (`0x08a7a574`, used at
`0x0892688c`) and `"%s external far tripod"` (`0x08a7a590`, `0x089268d0`) are
built, which is where Pulse's `Craft_Construct` builds them too.

`002afd08` is the **unrelocated** form of the address: `lui s6, 0x2b` with
`-0x2f8`, the PSP relocation defect `psp-pure-eu/string-anchors.md` describes. The
loaded address is not recovered here, so no data row is added to `names.tsv`.

**The consumer.** `FUN_0892a7d0` is Pulse's `Ship_UpdateCameraRigs` (`0x08845ed0`):
it reads the close block at `+0x50`..`+0x68` and the far block at `+0x34`..`+0x4c`
of `*(*(craft+200)+0x74)`, builds `anchorLook`/`anchorPos`, springs the eye with
`spring * dt`, and then scales four vectors by `_DAT_002afd08` (reads at
`0x0892aaf4`, `0x0892ab6c`, `0x0892aeb8`, `0x0892af30`): the close eye, the close
look-at, the far eye and the far look-at, each as `ship + (p - ship) * scale`.
The spring state (`craft+0x8f0`, `+0x900`, `+0x910`, `+0x920`) is written before the
scale, so the scale is applied to a copy and the state stays unscaled, the ordering
Pulse PSP's capture measured. `fov` and the springs are not scaled.

One difference from Pulse: Pulse's close and far eye add `stats[0x74]` (doubled for
far) to `pos_height`; Pure's decompile has no such term (`vscl_q(up, [params+0x50])`
alone). Whether that is a real divergence or the decompiler dropping a load is not
checked, and it does not touch the scale.

Confidence **85** that Pure's external eye and look-at carry `0.75`: one unambiguous
write of the literal in the constructor and four reads of the same global in the
rig, in the same shape as Pulse PSP's measured rig, on a second PSP binary. Not
higher: decompilation only, no run of Pure.

## Not read

- The internal and backward rigs (`FUN_0892a7d0`'s other branches) and whether
  Pure's internal rig leaves the scale alone as Pulse's does.
- Pure's default view as a number: see "Pure's default view" below (a frame comparison, not an eye read).

## Pure's default view is not Far (qualitative, 2026-10-21)

A fresh Pure profile does not start on the Far chase view. Confidence **70**: one boot,
a frame comparison and no eye reading.

**Method.** PPSSPP v1.20.4, software renderer, a fresh `HOME` (no save of any title),
walked by hand from the first-boot screens (language, memory-stick notice, new profile with
the default name and tag, save profile Yes) to Single Player, Time Trial, Vector, Vineta K,
Feisar, and no button pressed after the race loaded. The craft sat at rest on the grid
(race clock 15 s, 0 km/h). Our frame: `oag-game --race --track 01_Vineta_K --team feisar
--camera-view close|far --ticks 1100 --size 480x272` on the same pressing.

**Falsifier written first.** Far puts the eye 1.4 times further back than Close (Pulse's
measured 14.56 against 11.64), so the craft would draw about 0.8 of Close's width. The
original's craft is **wider than ours at Close** (about 95 px against 75 of 480, nose lower
on screen), 2.3 times Far's (about 40 px). So the default is not Far and is at least as near
as Close; Internal hides the hull, which is drawn, so it is not Internal. By elimination it
is Close.

**Residual.** The original's craft is about 1.2 times larger than ours at Close. Not
explained: the grid pose of the two frames differs (original read mid-session, ours at a
fixed tick), and no eye was read. Pure's authored Close block equals Pulse's. An eye read
needs Pure's camera node, which this lane did not locate (`compare-page` thread, Next Step 4).
The profile (`PROFILE.BIN`) is binary with no readable option string, unlike HD's.
