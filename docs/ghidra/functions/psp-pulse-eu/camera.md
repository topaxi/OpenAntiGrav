# Camera: the field-of-view chain, cross-verified on EU

The EU half of
[`psp-pulse-usa/camera.md`](../psp-pulse-usa/camera.md#the-field-of-view-is-dynamic-and-this-is-the-whole-chain)'s
2026-08-08 recovery. That page carries the evidence and the live measurements;
this one carries the EU addresses and says how far the correspondence was
checked.

**Direction of the check.** The chain was recovered on **USA** and verified
here, which is the opposite of this directory's
[stated preference](corroboration.md) for EU as the first target. The reason is
mechanical rather than considered: only `/pulse/BOOT-psp-pulse-usa.BIN` was loaded in
Ghidra when the work started, and every address `exhaust.md` and
`psp-pulse-usa/camera.md` already carried was a USA one. Recorded so the
direction is visible rather than implied.

## Confirmed

| Function | USA | EU | How |
| --- | --- | --- | --- |
| `Camera_SubmitScene` | `0x08878874` | **`0x088786d0`** | decompiled and read side by side; identical |
| `Camera_PublishTripod` | `0x08885d78` | **`0x08885bd4`** | decompiled and read side by side; identical |
| `Ship_UpdateCameraRigs` | `0x08845ed0` | **`0x08845d80`** | `diff_functions`: 737/737 instructions equal, zero added/removed |
| `VexCamera_BuildProjection` | `0x08901dc4` | **`0x08901744`** | `diff_functions`: 257/257 instructions equal, zero added/removed |
| `Hud_Update` | `0x0881bf50` | **`0x0881be60`** | `diff_functions`: 435/442 equal; every one of the 7 differences is a relocated immediate (a `lui` operand or a `DATA_EXT` offset) |
| `VexCamera_EvaluateFovCurve` | `0x089021c8` | **`0x08901b48`** | `diff_functions`: 87/87 instructions equal, zero added/removed |

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

Confidence **90** for all six correspondences: full instruction-level diffs,
not a fuzzy score alone (`find_similar_functions_fuzzy` was used only to
propose a candidate; `diff_functions` is what confirmed each one, and four of
the six - `Ship_UpdateCameraRigs`, `VexCamera_BuildProjection`,
`VexCamera_EvaluateFovCurve` and, closely, `Hud_Update` - are not just close
but **byte-for-byte identical**, stronger than the constant-matching evidence
the first two rows rest on). `Hud_Update`'s 7 differences are exactly the
shape this page's own trap section below predicts - a `lui` immediate or a
rodata/bss offset that moved with EU's smaller `BOOT.BIN` - never a differing
opcode or operand register.

Both `Camera_SubmitScene`/`Camera_PublishTripod` are `-0x1a4` from their USA
addresses, which is the local shift of that region rather than a
program-wide delta - do not use it to derive the others.
`Ship_UpdateCameraRigs` is `-0x150`, `VexCamera_BuildProjection` is `-0x680`,
`VexCamera_EvaluateFovCurve` is `-0x680` too (the pair moved together, intact
adjacency and all), `Hud_Update` is `-0xf0`: three more distinct shifts,
confirming again that there is no single delta to reuse across this binary.

## Not checked

None - all six of `psp-pulse-usa/camera.md`'s field-of-view chain functions
are now cross-verified on EU. `VexCamera_EvaluateFovCurve` was the last:
`find_similar_functions_fuzzy` alone gave no candidate above 0.75 (the tool
compares against every EU function generically and does not know to look
right next to `VexCamera_BuildProjection`), but the candidate this page had
already flagged at confidence 40 - EU `FUN_08901b48`, carrying the same
`65535.0` divisor - sat with no Ghidra function boundary defined over it at
all. `create_function` at that address (348 bytes, 87 instructions) plus
`diff_functions` against the USA function settled it outright: 87/87 equal,
zero added or removed. The address is also structurally exact, not just a
lucky guess - `VexCamera_BuildProjection`'s EU body ends at `0x08901b47`, one
byte before `0x08901b48` starts, mirroring the two functions sitting
back-to-back on USA (`VexCamera_BuildProjection` ends exactly where
`VexCamera_EvaluateFovCurve` begins, `0x08901dc4` + `0x404` = `0x089021c8`).

**Reproduction note**: `scripts/apply-ghidra-names.py` renames an existing
function, it does not create one, so on a fresh import `just apply-names`
will report "No function found" for this row until a function boundary
exists at `0x08901b48` on `psp-pulse-eu`. Run `create_function` (or let a
full `analyzeHeadless` auto-analysis pass discover it, unconfirmed whether it
does) before applying this row specifically.

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
