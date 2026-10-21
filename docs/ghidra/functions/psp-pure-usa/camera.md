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
- Pure's default view: read live, see "Pure's default view is Close" below.

## Pure's default view is Close, read live (2026-10-21)

**The eye, in the craft's frame, on a fresh profile.** Confidence **90**: three samples on each of two cold boots
(profile wiped between them), the eye read directly, identical to the printed digit.

**Method.** PPSSPP v1.20.4, software renderer, a fresh `HOME` (no save of any title), walked by hand
from the first-boot screens to Single Player, Time Trial, Vector, Alpha, Vineta K, Feisar, and no
button pressed after the race loaded; the craft at rest on the grid, race clock running. A
breakpoint on `0x08849970` (below) stops once per frame with `a0` the camera controller and
`*(a0 + 0x40)` the active tripod; a breakpoint on `Ship_UpdateCraft` (`0x0892d214`) gives the body
in `a1`. The eye is minus the tripod matrix's fourth row (the matrix is stored with the negated
eye, as Pulse's camera node is), projected on the body's three rows.

| Read | Value |
| --- | --- |
| eye in the body frame (right, up, forward) | `(0.000, +3.000, -11.250)` |
| `|eye - body|` | `11.643` |
| fov (`tripod + 0x50`) | `59.99997` |
| authored Close block (`pos_height 4`, `pos_length -15`, scale `0.75`) | `(3.0, -11.25)`, `11.643`: **equal**, so Close is the default and the `0.75` scale is live |
| authored Far block | `(3.0, -14.25)`, `14.562`: **not** what flies |

The default view is **Close**, the eye `0.75` times its authored offsets, the fov `60` unscaled:
Pure agrees with Pulse PSP on all three. `Craft_Construct`'s `0.75` and `Ship_UpdateCameraRigs`'s
four scaled vectors (above) are therefore confirmed by the eye they produce, not only by reading.

### `0x08849970` is the player view update (name `Camera_CopyTripodToView`, 85)

Hit once per frame in a race. It copies the 4x4 at `*(a0 + 0x40)` (the active tripod, picked by
the view setting) into the global camera node's `+0x40..+0x7c`, sets the fov global from
`*(tripod + 0x50)`, the tan-half-fov pair, and calls the view-commit function `0x0883ff34`
(which builds the frustum planes and pushes the view; also hit every frame). Pulse's
`Camera_UpdatePlayerView` (`0x0883c0cc`) is the larger function that also reshapes the eye's
distance; Pure's holds none of that. **`0x088473e4`**, the function that carries the `"Camera"`
setting lookup (`0x08a47598`), never ran in a Time Trial race in 8 and 15 second windows: it is
not the player's chase update (a spectator or replay view is the guess, unproven, no name).

### The view matrix is a shear, and ours was a rotation (2026-10-21, fixed)

The tripod's matrix, in the body frame at the grid: the columns are `(-0.995, 0, 0)` on the
left axis, `(0, 1, 0)` on the up axis and `(0, 0.0995, -0.995)` on the third - lengths `0.995`,
`1.0`, `1.0`, and the second and third are **not perpendicular** (`0.0995` apart in dot product).
The GE view matrix of the frame confirms it (a `gpu.record.dump` of the frame, the hull's draws,
`psp-ge-dump.py`): rows `(0.002, 0.003, 0.995)`, `(-0.0, 1.0, 0.097)`, `(-0.995, -0.0, 0.002)`.
That is a look-at built without renormalising: the right axis is `forward x up` (length
`cos 5.71 deg = 0.995`), the up axis is the craft's own up, and only the third axis is pitched
(`atan(3 / 30) = 5.71 deg` down toward the look-at point). A point ahead of the eye at its own
height stays on the screen's centre line: **the horizon does not rise with the look-at pitch.**
Pulse's captured camera node is orthonormal to rounding (`data/traces/pad0-boost.csv`, rows
`|c0|, |c1|, |c2|` = 1, no cross term), so the shear is Pure's.

**What it explains.** The 2026-10-21 frame comparison saw the original's craft "about 1.2 times
larger" and lower in the frame. At a matched pose (ours posed from the emulator's own body
matrix, fov 60, Close) the world, the hull's width (120 px against 123) and its mesh (`feisarShape`
345 vertices, world box `10.48 x 1.65 x 4.12`) agree; what differed was the camera's pitch.
Ours looked 5.71 degrees down, which lifted the horizon and every hull point by about 24 px of 272
(`tan(5.71 deg) / tan(30 deg) * 136 = 23.5`) and stretched the hull's length on screen from 87 to
97 px. The `1.2x` was this, plus a pose that differed between the two frames. Fed the original's
camera matrix and the same hull pose, ours drew the same picture as before the fix (our `look_at`
re-orthonormalises it), which is what located the cause in the matrix and not in the eye.

**Ported** as `oag_title::craft_laws::CraftLaws::chase_view_sheared` (Pure `measured(true)`,
HD, Omega and 2048 `inherited(false, "Pulse")`: unread there),
`oag_race::chase_rig::ChaseParams::view_sheared` and
`oag_render::camera::chase::sheared_look_at`. Both external views take it (one function builds
the Close and Far tripods; Far was not read live). The cockpit view is untouched (not read).
Before and after at 480x272, the same pose: RMSE against the original frame `0.307 -> 0.178`
(the rest is the HUD, the clock and the original's lighting).

### The hull's drawn matrix (entity `+0x9ec`, read live)

`*(entity + 0x9ec) + 0x80` at the grid: the body's axes times `0.75` and the body's position
exactly (`38.865, 37.473, -96.697`), identical on three reads three seconds apart. Ours draws
`orientation * scale 0.75` at the body position: equal. (`entity` is `a0` of `0x0892a7d0`;
`a0` of `Ship_UpdateCraft` is the physics object, whose `+0x9ec` is not this.)

## Open

- The Far view's matrix (same code path, not read) and the cockpit view's.
- HD, Omega and 2048: whether their view matrices are sheared. Pulse's is not.

Second cold boot (2026-10-21, `HOME` wiped, the same walk): eye `(0.000, +3.000, -11.250)`, `11.643`, fov `59.99997`, the hull node's axes `0.75` and its translation the body's, the tripod's third column `(0, 0.0995, -0.995)` in the body frame again.
