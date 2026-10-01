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
- Pure's default view (an unmeasured `OPT_*` default), unchanged by this page.
