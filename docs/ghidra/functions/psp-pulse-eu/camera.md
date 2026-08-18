# Camera: the field-of-view chain, cross-verified on EU

The EU half of
[`psp-pulse-usa/camera.md`](../psp-pulse-usa/camera.md#the-field-of-view-is-dynamic-and-this-is-the-whole-chain)'s
2026-08-08 recovery. That page carries the evidence and the live measurements;
this one carries the EU addresses and says how far the correspondence was
checked.

**Direction of the check.** The chain was recovered on **USA** and verified
here, which is the opposite of this directory's
[stated preference](corroboration.md) for EU as the first target. The reason is
mechanical rather than considered: only `/psp-pulse-usa/BOOT.BIN` was loaded in
Ghidra when the work started, and every address `exhaust.md` and
`psp-pulse-usa/camera.md` already carried was a USA one. Recorded so the
direction is visible rather than implied.

## Confirmed

| Function | USA | EU | How |
| --- | --- | --- | --- |
| `Camera_SubmitScene` | `0x08878874` | **`0x088786d0`** | decompiled and read side by side; identical |
| `Camera_PublishTripod` | `0x08885d78` | **`0x08885bd4`** | decompiled and read side by side; identical |

Both bodies match statement for statement, including every constant the
finding rests on:

- `Camera_SubmitScene` builds `m11 = 1/tan(fov * pi/180 * 0.5)` and
  `m00 = m11 / 1.7647059`, takes its near plane as
  `cam+0x80 + (65.0 / fov - 1.0) * 0.5`, its far from `display+0x1698`,
  installs the result at `display+0x1190`, and ends with the same
  `* 480.0` LOD term and `Gfx_Enqueue(display, cam, 0x30000000)`.
- `Camera_PublishTripod` copies the tripod's 4x4 into the global camera and
  publishes `fov = *(tripod+0x50)`, `tan(fov/2)`, and **`tan(fov/2) * 480.0 /
  272.0`** - the two literals (`lui 0x43f0`, `lui 0x4388`) that pin the fov
  unit as vertical degrees and the aspect as hardcoded 480x272.

Confidence **90** for the correspondence: two full decompilations compared
directly, not a fuzzy match, and the constants that carry the finding are
present in both.

Both are `-0x1a4` from their USA addresses, which is the local shift of that
region rather than a program-wide delta - do not use it to derive the others.

## Not checked

`Ship_UpdateCameraRigs` (USA `0x08845ed0`), `Hud_Update` (USA `0x0881bf50`),
`VexCamera_BuildProjection` (USA `0x08901dc4`) and
`VexCamera_EvaluateFovCurve` (USA `0x089021c8`) were **not** located on EU.
One candidate is noted rather than claimed: EU `FUN_08901b48` is a small
function carrying the `lui 0x477f` (`65535.0`) that
`VexCamera_EvaluateFovCurve` divides its curve values by, and is the only such
site outside `Trail_BakeVertexColours` and one other. That is suggestive and
nothing more - **confidence 40**, below the rubric's rename threshold, so it
is written down instead of named.

## A trap: this program's data addresses are not USA's plus a delta

`Camera_PublishTripod` writes its three globals through `lui a0, 0x6` /
`swc1 f12, -0x5140(a0)`, i.e. **`0x0005aec0`** - a low address, while the same
function's code sits at `0x0888xxxx`. The neighbouring pair land at
`0x0005aec4` (horizontal tangent) and `0x0005aec8` (vertical tangent), and
`Camera_SubmitScene`'s cached matrices are at `0x00059870`..`0x000598ec` in the
same low space.

So in this Ghidra import the EU program's **data references decode into a
different base from its code**, and the USA data addresses
(`g_camera_fov_degrees` at `0x08b34310` and its two neighbours) have no
arithmetic relationship to them. **No EU data rows are proposed here** - naming
`0x0005aec0` would pin a name to an address that a correctly based import would
move. Fixing the EU import's data base is a separate job; until then, EU data
findings should be recorded as offsets from a named function's access, the way
this section does.
