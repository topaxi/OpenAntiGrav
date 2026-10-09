# Camera views (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

The PSP counterpart is [psp-pulse-usa/camera.md](../psp-pulse-usa/camera.md), which
established the three player views, the SELECT cycle and the sign convention of
the on-disc offsets, most of it against the running game. This page records the
PS2 equivalent and confirms the cycle in a second binary.

**The names below are applied**, from [names.tsv](names.tsv).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0014f208` | `Camera_UpdatePlayerView` | 88 |
| `0x002db350` | `g_settings` | 78 |
| `0x0013e280` | `Camera_SubmitScene` | 80 |
| `0x00158568` | `Ship_UpdateCameraRigs` | 95 |
| `0x00150d20` | `Craft_Construct` | 80 |
| `0x0027e8cc` | `g_craft_scale` (data) | 95 |

The parameter blocks themselves are parsed by the five
`HandlingXml_Parse*Camera` functions, documented in
[handling-xml.md](handling-xml.md); all 27 floats sit at the same offsets as on
PSP.

## The SELECT cycle is the same, in the same order

`Camera_UpdatePlayerView` (`0x0014f208`) opens with the same three steps as the
PSP function:

```c
setting = Camera_GetViewSetting(playerIndex);       /* 0x0014f108 */
if (Input_IsPressed(g_input, playerIndex, 0xf, 0)) {  /* 0xf == SELECT */
    Input_ConsumePress(g_input, playerIndex, 0xf);
    *(u32 *)(g_settings + 0x45c) = 1;                 /* settings dirty */
    if      (streq(setting, "OPT_INT"))   next = "OPT_CLOSE";
    else if (streq(setting, "OPT_CLOSE")) next = "OPT_FAR";
    else if (streq(setting, "OPT_FAR"))   next = "OPT_INT";
    else                                  next = setting;
    Settings_SetString(g_settings, hash("Camera"), next, strlen(next) + 1);
}
```

**`OPT_INT` -> `OPT_CLOSE` -> `OPT_FAR` -> `OPT_INT`**, wrapping, on button
index `0xf`, with the press consumed and a dirty flag set on the settings
object. The string literals are at `0x002a5c98`, `0x002a5c80` and
`0x002a5c90`; the `"Camera"` key is at `0x002a5cb0`. The dirty flag is at
`g_settings + 0x45c` here against `+0x45b` on PSP, and is a word rather than a
byte.

The three views then select a rig by name and set the hide-own-ship flag:

| Setting | Rig | Cached index at `controller+0x3c` | Craft flag |
| --- | --- | ---: | ---: |
| `OPT_INT` | (internal; no `printf`-built name) | 0 | `+0xf4 = 1` |
| `OPT_CLOSE` | `"player%d external close tripod"` | 1 | `+0xf4 = 0` |
| `OPT_FAR` | `"player%d external far tripod"` | 2 | `+0xf4 = 0` |

and `+0xf4` is copied to `+0xfc` immediately afterwards - the same
write-then-mirror pair the PSP page found at `craft+0x6d` / `craft+0x6f`, with
the same "1 only for the internal view" correlation. Two builds agreeing on
that pattern raises the PSP page's confidence-70 reading of it as "hide the
player's own ship".

The rig names are `printf`-formatted with the player index, which the PSP's
fixed `player_internal_tripod` is not. That is the split-screen difference
showing through, not a behavioural one.

Confidence **88** for the cycle: the three literals and their rotation are
unambiguous in the decompilation and identical to the PSP reading. Not higher
because nothing on the PS2 side has been observed running.

## The chase distance is computed here, and it is not a constant 3/4

The PSP page's largest open question is that both external offsets reach the
eye at exactly **0.75** of their authored value, from a source it could not
find. The PS2 function does its distance work inline and visibly, which is a
lead rather than an answer:

- it takes the ship-to-eye vector, normalises it (`vrsqrt`), and keeps the
  length `L`;
- it forms `d = min(L - 1.0, 3.0)` and probes along the ray with a trace
  (`0x00132ed0`), shortening `d` when geometry is hit;
- it low-passes the result into a per-player global at `0x0027e8b0` with
  `x += (target - x) * 0.5` each frame;
- and it applies a vertical lift of `(7.5 - d) * 0.5` when `d < 7.5`.

None of `0.75`, nor a multiply by three quarters, appears anywhere in it. So
**the PS2 does not reach its eye position the way the PSP page assumed the PSP
does**, and the 3/4 factor is not a shared constant sitting in the camera code
of both. Whoever picks that question up should treat the PS2 path as a
different algorithm rather than as a second copy of the same one. Confidence
**70** on that negative: the function was read, not run, and the trace at
`0x00132ed0` was not decoded.

## Correction 2026-10-01: the PS2 eye IS scaled by 0.75, in the rig, not in `Camera_UpdatePlayerView`

The section above is right about the function it read and wrong about the
conclusion it drew. `Camera_UpdatePlayerView` (`0x0014f208`) does not place the
eye at all: it takes the tripod's position at `tripod+0x30`, which something
else already wrote, and only pulls it in when the trace at `0x00132ed0` hits
geometry (a wall pull-in, with its low-pass and lift). The place the authored
`pos_*` become an eye is `FUN_00158568`, now `Ship_UpdateCameraRigs`, and it is
the PSP's `Ship_UpdateCameraRigs` (`0x08845ed0`) line for line. Read headless,
2026-10-01, decompiler output plus the instruction stream.

**The write.** `FUN_00150d20` (the ship-entity constructor, now `Craft_Construct`;
it also calls `Ship_InitCraft`) stores `0.75` (`0x3f400000`) to `DAT_0027e8cc`
at `0x00150f64` and its reciprocal `1.3333334` (`0x3faaaaab`) to `DAT_0027e8d0`
at `0x00150f60`, four instructions apart, the PSP's `0x08841000` / `0x08841018`
pair. It then does `Body_SetPosition(DAT_0027e8cc * 150.0, ...)`, as the PSP does.
The ELF image holds `1.0` at that address; the `0.75` is the constructor's, so a
reader of the file alone sees the wrong value. Readers of `DAT_0027e8cc`:
`Ship_HoverFourCorner` (`0x0015a9fc`), `Ship_InitCraft` (`0x0015957c`,
`0x00159590`), `Ship_UpdateCraft` (`0x00159b44`), `Craft_Construct` itself
(`0x0015127c`, `0x001512e0`: the `<Misc>` hull dimensions and the body height),
`FUN_00158568` four times (`0x001587a8`, `0x001587c4`, `0x00158a2c`,
`0x00158a44`) and `FUN_0014bab0`, `FUN_00155188`, `FUN_001f82bc`. The PSP's
consumer list is the same shape.

**The rig.** With `params = *(*(craft+0x124)+0x8c)`, `FUN_00158568` runs the
close block (`+0x50` pos_height, `+0x54` pos_length, `+0x58` lookat_height, `+0x5c`
lookat_length, `+0x64`/`+0x68` springs) then the far block (`+0x34`..`+0x4c`, the
same seven in the same order), each as

```c
anchorLook = shipPos + fwd * lookat_length + up * lookat_height;
anchorPos  = shipPos + fwd * pos_length;
eye        = prevEye + spring(anchorPos - prevEye) * dt;   /* prevEye at craft+0x870 / +0x880 */
eye        = anchorLook + normalize(eye - anchorLook) * length(anchorPos - anchorLook);
prevEye    = eye;                                          /* stored BEFORE the scale */
eye       += up * (pos_height + stats[0x94] [*2 for far]);
eye        = shipPos + (eye  - shipPos) * g_craft_scale;   /* -> craft+0x860 close, +0x850 far */
look       = shipPos + (anchorLook - shipPos) * g_craft_scale;
```

so the spring state is unscaled and the scale is applied after it, the ordering
the PSP page measured against the original. `fov` and the springs are not
scaled. Even the odd `pos_height + stats[0x74]` term (doubled for the far block)
has the same shape on the PSP (`*(craft+0x94)+0x74`), and so does the far block's
extra `up.y` factor on that term.

**Look-at is per block.** The close look-at reads `+0x58`/`+0x5c` and the far
look-at `+0x3c`/`+0x40`, each from its own block, so the PS2's authored far
look-at `(6, 28)` against the PSP's `(0, 20)` reaches the aim point as authored.

### Measured live, 2026-10-01

A running PCSX2 (`v2.7`, PINE, own data path and display) loaded the harness's
race savestate (slot 1, Assegai-class authored blocks). At boot the ELF's
`0x0027e8cc` reads `0x3f800000` (`1.0`); after the state loads it reads
`0x3f400000` (`0.75`) and `0x0027e8d0` reads `0x3faaaaab`, so the constructor's
store is real. The craft was found by signature in the state's EE RAM
(`scripts/pcsx2-camera-eye.py find`: the one address whose rig-published far and
close eyes sit `14.562` and `11.643` from the body at `*(craft+0x824)+0x30`), then
read live (`scripts/pcsx2-camera-eye.py live`) in the craft's own right/up/forward
axes, `craft+0x404` forward, `+0x408` up:

| Eye (craft+) | Forward | Up | Distance | Authored block x 0.75 |
| --- | ---: | ---: | ---: | --- |
| close (`+0x860`) | -11.250 | +3.000 | 11.643 | `(-15, +4)` -> `(-11.25, +3.0)` |
| far (`+0x850`) | -14.250 | +3.000 | 14.562 | `(-19, +4)` -> `(-14.25, +3.0)` |

Identical to Pulse PSP's live measurement (`(-11.25, +3.0)`, 11.643), to three
decimals, at rest; moving, the close eye holds 11.639-11.643 across a 5-second
run while the far eye relaxes onto 14.562 as its spring settles (it read 14.70
then 14.64, 14.58, 14.565, 14.562 as the craft pulled away from a standstill:
the far rig keeps its own sprung state, which is why the far figure lags). The
savestate flies the close view (the saved frame shows the craft well inside the
frame); whether that is a cold-boot default is not established, the state's own
profile history being unknown.

Confidence **95** that the PS2's external eye and look-at are scaled by `0.75`
about the craft, after the spring: the write, the reciprocal pair, the four reads
in the rig, a live read of the global and a live read of both eyes at exactly
the authored block times `0.75`, on two binaries (this and the PSP's) that agree.
Not 100: the look-at point was not read live, only the eye.
The earlier "does not reach its eye the way the PSP does" reading stands retired.
`Camera_UpdatePlayerView`'s probe-and-low-pass remains a separate wall pull-in
(`min(L - 1, 3)` along the ray, low-passed at `0.5` into `0x0027e8b0`), a no-op in
open air; it is not implemented here.

## The projection the race renders with, measured 2026-10-01

Read live off a running PCSX2 (own data path, PINE), on the race savestate the
eye was measured on (Assegai, `16_Track`, at rest on the grid, `OPT_CLOSE`,
game option `Aspect Ratio` = `0` = `4:3`, PCSX2 `AspectRatio = Stretch` at a
640x448 window so the frame is 1:1). The camera object `Camera_SubmitScene`
runs on was found by signature: the address `a` where `*(a + 0x110)` is the fov
and `a + 0xd0 .. +0x10c` is a perspective matrix. It sits at `0x609bb0` in
that state.

| Read | Value |
| --- | --- |
| `*(a + 0x110)` fov | `60.000004` degrees (`craft + 0x8e0`, the close tripod's `params + 0x60`, `+ craft + 0x820` = `5.9e-6`) |
| matrix `m00`, `m11` | `1.21244`, `1.73205` |
| `m11 / m00` | `1.42857` = `10/7` = `640/448` |
| vertical field `2 atan(1 / m11)` | `60.0000` degrees |
| eye, in the craft's forward/up/right axes | `(-11.250, +3.000, 0.000)`: the rig's scaled close eye, unmoved by `Camera_UpdatePlayerView`'s pull-in |
| `DAT_00284fe8` | `0` (`4:3`) |

Identical on two loads of the state and again 30 frames after each. Confidence **95** that the
PS2 race renders a vertical field of exactly the authored `fov` at aspect `10/7`
with the eye of the rig, at the `4:3` setting: a live read of the matrix itself,
twice, agreeing with the literal `0x3fb6db6e` at `0x0013e604` the decompile
shows. Not higher: one circuit, one team, one craft at rest (so the speed term
`craft + 0x820` was read at `0`, not exercised).

**A second perspective matrix in the same RAM is not the race camera.** Searching
for a matrix with `m11 / m00 = 10/7` also finds `0x2f9080`, with a vertical field
of exactly `65` degrees, near `1` and far `100`. `65` is the engine's own
default field when no camera station is set (`Camera_FramingFov` on the PSP) and
the near formula's reference value; it is another camera's projection, and
reading it as the race's would have the craft drawn at the wrong size by `1.08`.

### What that does to the framing, measured against ours

At the same eye, a hull's apparent width in the frame goes as
`1 / (tan(fov/2) * aspect)`. The original's horizontal half-tangent is
`tan 30 * 10/7 = 0.825` in a 640-wide frame; the race fitted the PSP's `480/272`
to every source, so ours was `tan 30 * 1.765 = 1.019` and drew the hull
`0.825 / 1.019 = 0.81` as wide. Measured on the craft, original (this state)
against ours, both 640x448, same eye: wing span in the crops
`crops_orig_before_after.png` (3x, read by eye,
about +-3 px of 375): original 375, ours before 305 (0.81, as predicted), ours
after 375 (1.00). Same vertical span. **The `38 % against 23 %` of the shield
lane's report is not reproduced: this matched-pose pair gives 25.5 % against
20.5 % of the frame width before the fix** (`0.81`), and `25.5 %` against
`25.5 %` after, in a 640x448 window with the viewport set to `free`.

**Reverted the same day by maintainer decision**: this project presents the PS2 widescreen only and keeps the PSP-shape fit it had before 2026-10-01, so the fix below is not in the code. The measurement stands; reinstating it is reverting the revert.

What was built and reverted, 2026-10-01, for the PS2 source only: `oag_display::space::camera_authored_aspect`
returns `640/448` for the PS2 and the PSP's `480/272` for everything else, and
`Race::vertical_fov` fits to it. It changes only a window **narrower than the
PSP's shape** (`fit_vertical_fov` keeps the authored vertical field at or above
it, whichever constant is used), so the default presentation, a `30:17`
viewport, is unchanged and still shows the hull at `0.81` of the original's
`4:3`-option frame. That remainder is not a camera term: the original's `4:3`
frame is `640/448` shown anamorphically at whatever the display is, and ours
presents the field at the viewport's own shape. See
[aspect-ratio.md](../../../ps2/aspect-ratio.md).

## The `Aspect Ratio` option's widen, at one address inside `Camera_SubmitScene`

`docs/ps2/aspect-ratio.md` ("What the option changes: one multiply, in the
camera") already reads this in full - confidence 95, verified against every
reference to the setting's global. Restated here only because that page lives
outside this directory and `names.tsv` needs a citation inside it: at
`0x0013e6ec`, `Camera_SubmitScene` reads `DAT_00284fe8` (`0` for `4:3`, `1` for
`16:9`) and multiplies the projection's aspect by `4/3` before building it -
`FUN_001e9420(fov_rad, aspect, near, far, 1.0)`, an ordinary perspective
build. The authored FOV degrees are unchanged; only the aspect widens, so at
`16:9` the horizontal field is genuinely a third wider, not a stretch of the
same picture.

**`FUN_0013e280` is now named `Camera_SubmitScene`, at confidence 80 - the
same name and the same confidence the PSP disc's own equivalent already
carries** ([`psp-pulse-usa/camera.md`](../psp-pulse-usa/camera.md), which
named its `0x08878874` at 80 for exactly the same reason: "the plane block is
read as shape, hence 80 rather than 90"). Read in full while chasing the
multiply above: the function opens with a camera-shake decay/apply block, now
understood and documented on its own page - [collision-shake.md](collision-shake.md),
which also has the arming side of it (`Camera_ArmShake`, called from the
collision-response path on impact). Past the multiply this page covers,
`Camera_SubmitScene` also pushes a view matrix to the GS display stack
(`DAT_00282aec+0x1410`..`+0x144c`) before building the projection, and
afterward computes five plane-like `{xyz, dot}` quads (`+0x140`, `+0x150`,
`+0x160`, `+0x170`, `+0x180`) - see the next section for what those actually
are, now that both blocks this page previously left unread (the shake and the
quads) have been read properly. That reading, plus the PSP function of the
same name already doing the identical sequence in the identical order
(shake apply -> GS matrix push -> projection build -> frustum-plane set), is
what earns the whole-function name here: not the aspect-multiply alone, which
was never enough on its own.

## The plane-like quads are five view-frustum culling planes, cross-checked against the PSP's own `Camera_SubmitScene`

The PS2 function's own decompilation of this block is VU0 macro-pipeline
pseudocode of the same unreliable shape this project already discounts
elsewhere (`FUN_0020cf50`/`FUN_0020cec0`, sin/cos-shaped, "unread beyond
that") - one operand even shows Ghidra rendering a raw bit-pattern move as a
genuine `(int)` truncation of a cosine (`(long)(int)-fVar18`) right next to
an operand it renders correctly as a raw move (`(long)iVar11`), which cannot
both be right and is a known class of decompiler artifact for this
instruction mix, not a fact about the disc's own arithmetic. So this reading
leans on the PSP's `Camera_SubmitScene` (`0x08878874`), whose equivalent
block decompiles as ordinary scalar float code with no VU0 ambiguity at all,
and treats the PS2 side as confirming *shape and landing offsets* rather than
exact per-term signs.

**What the PSP's clean version does, section by section** (`param_1+0x90`
through `+0xdc`, five quads spaced `0x10` apart - the PS2's are at
`+0x140`-`+0x18c`, offset because the PS2 reuses `+0x90..+0x10c` as scratch
for the GS matrix push a few lines earlier instead of separate globals the
way the PSP does):

1. `+0x90/.../+0x9c`: normal `= (row0.z, row1.z, row2.z)` - the same three
   z-lane values the degeneracy check just above summed the squares of -
   dotted with the camera's own position. A plane through the eye,
   perpendicular to whichever world axis that z-lane column represents (the
   forward axis, on the reading `positional-audio.md` and `collision-shake.md`
   both already lean on: the basis's *columns*, not its rows, are the world
   axes). Reads as a near/forward reference plane.
2. `+0xa0`: normal `= sin(h)*row0 - cos(h)*row2`, `+0xb0`: normal
   `= -sin(h)*row0 - cos(h)*row2` - the forward row rotated by `±h` around
   the up row, combined with the right row, `h` being the *horizontal*
   half-FOV. Reads as the right/left side clip planes.
3. `+0xc0`: normal `= -sin(v)*row1 - cos(v)*row2`, `+0xd0`: normal
   `= sin(v)*row1 - cos(v)*row2` - the same shape, one row over, `v` the
   *vertical* half-FOV. Reads as the top/bottom clip planes.
4. Every plane's fourth term is `-dot(normal, position)` - each plane passes
   through the camera, as a frustum plane must.

`h` is `v * 1.7647059` on the PSP (`v` in radians is `fov_deg * pi/180 * 0.5`,
and `1.7647059 = 480/272`, the PSP's own render aspect) and `v * 1.4285715` on
the PS2 (`1.4285715 = 10/7 = 640/448`, a PS2-typical render-target aspect) -
**both a hardcoded literal, not the runtime `Aspect Ratio` setting's widened
aspect.** So whatever consumes these five planes keeps culling against the
*render target's own* aspect on both platforms, unaffected by the 16:9 option
this section's multiply implements - the widen only reaches the visible
projection matrix, not this frustum set. That is read at the same confidence
as the plane block itself (below), since it rests on the same literal.

Confidence **80** for "these are five view-frustum culling planes (one
forward/near, two horizontal side, two vertical side), each through the
camera's position, built from the camera's own basis and the vertical/
horizontal half-FOV": the PSP reading is unambiguous scalar arithmetic
(confidence ~90 on its own), cross-checked structurally against the PS2's
five quads landing at the same relative offsets in the same order doing the
same class of sin/cos-weighted row combination (confidence ~70 given the
VU0 decompile noise above) - averaging to the function's own overall 80,
matching the PSP page's own precedent for scoring this exact block. **Not
determined**: what actually reads these five planes back (no caller of
`Camera_SubmitScene` was found to check against - see the "Not determined"
xref note below), and therefore whether "culling" is the right verb rather
than, say, a shadow-volume or reflection-clip use.

## Not determined

- **`0x0014f108`**, which returns the current view setting, reading either from
  `g_settings` or from a per-player array at `0x0027e8a8` depending on a global
  at `0x002dab3c` (which reads like an attract/demo switch). Not renamed: the
  branch is clear but what selects it is not.
- **The rig lookup** at `0x00149450` and the trace at `0x00132ed0`.
- **The second, spectator/photo camera enum** that the PSP page warns not to
  confuse with this one. Not searched for here.
- **No caller of `Camera_SubmitScene` was found** - `get_xrefs_to` on
  `0x0013e280` returns nothing, and a byte-pattern search for the direct
  `jal Camera_SubmitScene` encoding (`A0 F8 04 0C`) finds no match anywhere
  in the image either, so it is not a missed-xref problem Ghidra could fix
  with more analysis: nothing calls this address with a direct `jal` at all.
  Checked whether this is the PSP relocation-table defect other threads have
  hit: it is not - `SCES_547.48`'s relocation table has **zero** entries
  total (checked with `getRelocations`, both globally and at this address
  and at `Camera_UpdatePlayerView`'s for comparison), which is a different
  situation from an *unapplied* one, and consistent with this being a fully
  linked PS2 executable that never carried a relocation section to begin
  with. The likelier explanation is an indirect call - `jalr` through a
  per-camera function-pointer/vtable slot, which a direct-`jal` byte search
  cannot find and which would also fit the "second, spectator/photo camera"
  object this page already can't locate (a vtable would explain both at
  once).

  **2026-09-06: the vtable-shaped table exists, confirmed by finding
  `Camera_SubmitScene`'s own address stored as data, and it too has no
  caller.** A byte-pattern search for `0x0013e280` little-endian (`80 E2 13
  00`) across `SCES_547.48` returns exactly one hit, at `0x00293084`. Reading
  the surrounding bytes shows a null-terminated table of nine 8-byte entries
  starting at `0x00293070`, each `{u32 zero, u32 pointer}`: the pointer at
  `0x00293084` is `Camera_SubmitScene`'s own address, the third entry.
  Two of the other eight pointers land inside recognised function bodies
  (`0x0013edb8`, a function Ghidra already has a boundary for but which
  itself has zero xrefs either); the remaining six point into byte ranges
  Ghidra has never marked as function starts at all, which is consistent
  with a dispatch table whose entries were never reached by any path static
  analysis could follow - not with hand-picked data that happens to alias
  code. `analyze_data_region` on `0x00293070` reports zero xrefs and
  classifies it only as `PRIMITIVE`/`DAT_00293070`, i.e. this table's shape
  is not something Ghidra's own analysis noticed either.

  This does not, on its own, prove the table is *the* mechanism that calls
  `Camera_SubmitScene` - no `jalr` was traced loading a register from this
  table's address, and the table itself has zero xrefs of any kind, the same
  as the function pointer it holds. A second, matching negative result rules
  out the most likely alternative for how it could still be reached in a way
  Ghidra would show: searched for a `lui`/`ori` or `lui`/`addiu` pair
  constructing the table's base address (`0x00293070`, hi half `0x0029`) -
  `lui reg, 0x29` does not occur anywhere in the image (631 `lui` matches
  contain the substring `0x29`, none of them exactly that operand), so
  nothing builds this address via ordinary two-instruction absolute
  addressing either. Whatever loads a pointer out of this table does it by a
  path this project's static tools can't yet follow - a runtime-computed
  base (e.g. a struct field set up elsewhere and indexed at a variable
  offset), most likely. Not chased further past this; worth flagging for
  whoever picks up the vtable question or the general relocation-defect
  thread, since the empty-xrefs symptom looks identical to that defect's
  from the outside and isn't it.
- **`FUN_001e9420`**, the perspective-matrix builder the aspect widen calls
  into - read only for its two diagonal terms, not renamed.
- **What reads the five frustum planes back** - see the plane section above.
- **The roll/bank angle at `+0x124`** (PSP's equivalent is the global
  `DAT_002ad0a8`, not a struct field - a platform layout difference, not a
  behavioural one): computed by Gram-Schmidt-orthogonalising the "up" column
  against a horizontal reference built from two basis rows' z-lanes, then an
  `asin`-shaped call (`FUN_0023dd20` PS2 / `func_0x0017ad98` PSP, neither
  named) on the residual, sign-flipped by a forward/right dot test. Reads as
  a camera bank/roll angle, structurally identical on both platforms, but
  nothing that *consumes* `+0x124` was traced, so what it drives (HUD
  horizon indicator, motion-blur direction, or something else) is unknown.
  Confidence 60 on "it's a roll angle in radians"; 0 on what uses it.
- **The PSP's own tail past this** (a screen-space, texture-LOD-bias-shaped
  calculation using `log2`-style divides by `0.6931472` = `ln 2`) **has no
  PS2 counterpart in this function** - the PS2 side ends three calls after
  the plane block (`FUN_001d39b8`, `FUN_001e88b8` x2, none read here) with
  nothing resembling the PSP's LOD math. Genuine platform difference or
  handled elsewhere on PS2 - not chased.
- **Nothing here was verified at runtime.**

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Camera_UpdatePlayerView` | `0x0014f208` | `0x0883c0cc` |
| `g_settings` | `0x002db350` | `0x08b31774` |
| `Camera_SetMode` | not located | `0x08880724` |
| `Camera_SubmitScene` | `0x0013e280` | `0x08878874` |

## History

- 2026-07-27: first pass. Cycle 88 from an exact match with the PSP reading;
  the 3/4 chase factor recorded as absent from the PS2 path.
- 2026-09-03: added the `Aspect Ratio` widen's address inside `FUN_0013e280`,
  restated from `docs/ps2/aspect-ratio.md` so `names.tsv` has a page in this
  directory to cite against this project's own race camera reusing the PSP's
  authored FOV on every title. Reading `FUN_0013e280` in full for that also
  turned up its shake-decay block, since documented separately
  ([collision-shake.md](collision-shake.md)) once its arming side was found.
  Still deliberately did not rename `FUN_0013e280` itself: a frustum-plane-
  shaped tail past the projection build remains unanalysed.
- 2026-09-05: the frustum-plane tail is read - five plane equations, cross-
  checked against the PSP's own `Camera_SubmitScene` (`0x08878874`), whose
  equivalent block decompiles cleanly with none of the PS2's VU0 pseudocode
  ambiguity. Both the shake block (settled the same day on
  [collision-shake.md](collision-shake.md)) and this block now being read
  properly - the bar an open thread set for naming the whole function -
  `FUN_0013e280` is now `Camera_SubmitScene`, confidence 80, the same name
  and the same confidence the PSP disc's own equivalent already carries, for
  the same reason ("the plane block is read as shape, hence 80 rather than
  90"). Also checked,
  prompted by the empty xrefs to this address: `SCES_547.48`'s relocation
  table is entirely empty, so this is not the PSP relocation-table defect
  other threads have hit, just a program that never carried one - the
  missing callers are more likely an indirect `jalr` dispatch than a Ghidra
  analysis gap.
- 2026-09-06: found `Camera_SubmitScene`'s own address stored as data, inside
  a null-terminated, nine-entry `{u32 zero, u32 pointer}` table at
  `0x00293070` with no xrefs of its own and no `lui`/`ori` pair anywhere in
  the image constructing that table's base address either - so the
  vtable-shaped table this page already suspected does exist, but nothing
  Ghidra's static analysis can see reaches it. See the "Not determined"
  section for the full trace.
- 2026-10-01: the chase eye is scaled on the PS2 too: see the correction section.
  `FUN_00158568` named `Ship_UpdateCameraRigs`, `FUN_00150d20` `Craft_Construct`,
  `DAT_0027e8cc` `g_craft_scale`. Read live on PCSX2 the same day: both eyes sit at
  the authored blocks times 0.75, the PSP's measured numbers exactly
  (`scripts/pcsx2-camera-eye.py`).
