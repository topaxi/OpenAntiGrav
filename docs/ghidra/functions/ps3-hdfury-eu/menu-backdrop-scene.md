# The HD-style menu backdrop: a 3D scene flown through, drawn as a grey line drawing

2026-09-30. The other half of [menu-backdrop.md](menu-backdrop.md). The HD style's
`Top FE Screen` carries `<BackgroundAnim name="bgAnim">` (`BackgroundAnim_Item.cpp`),
and the Fury style disables it: `BackgroundAnim_Load` ends with
`if (!FrontEnd_IsFuryStyle()) flags |= 4`, the enable bit, where the Fury widget is
`startenabled="false"`. So the two backdrops are never up together.

What it draws is not a point cloud and not a ring of lattice. It renders the authored
scene `FrontEndScene_HD_ATG.vex` - the start line of a circuit, two ships, a track
surface and a run of hoops - through the scene's **own animated camera**, into a target
cleared to white, and filters that target into a grey drawing: a faint fill where the
scene is, a darker outline along its edges. The menu's white `HD_BG` shows through
everywhere else. This page is where each number was read; `oag_ui::scene_backdrop` and
`oag_game::render::scene` are the code.

Read [memory.md](memory.md) first for the per-function TOC defect: every address below was
resolved with `scripts/ps3-toc.py`, and `0x0017d3f0`, `0x0017ca38`, `0x0017af80`,
`0x0017bac0`, `0x0017be68`, `0x0017d690` and `0x001d35e8` all read TOC `0x008ad4d8`
(`exact`).

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0017d350` | `BackgroundAnim_Construct` | 80 |
| `0x0017d3f0` | `BackgroundAnim_StaticInit` | 82 |
| `0x0017ca38` | `BackgroundAnim_Load` | 80 |
| `0x0017af80` | `BackgroundAnim_Update` | 82 |
| `0x0017bac0` | `BackgroundAnim_AdvanceClock` | 72 |
| `0x0017be68` | `BackgroundAnim_Render` | 80 |
| `0x0017d690` | `BackgroundAnim_ParseScreenSetting` | 85 |
| `0x001d35e8` | `ModelItem_RenderScene` | 65 |
| `0x003e3e50` | `Post_ScreenBlur` | 62 |

The class is `BackgroundAnim_Item.cpp` (`0x007882a0`; constructors `0x0017d2b0`,
`0x0017d350`); its vtable is `0x00865248` and its virtuals are `0x0017af80` (slot 3),
`0x0017bac0` (5), `0x0017be68` (7), `0x0017b930` (12, destructor), `0x0017ca38` (14, Load)
and `0x0017d690` (18, parse). The base class is the model widget both this and
`Flyer_Item` derive from (`0x001d3150`).

## The `ScreenSetting` row, and what `Parse` does to it

`0x0017d690` reads each `<ScreenSetting>` into a `0x84`-byte record: a `0x40`-byte name,
the `use_bands` byte, seven floats - `min_band_height`, `max_band_height`,
`min_trigger_time`, `max_trigger_time`, `min_travel_time`, `max_travel_time`, `blur` -
and three triples, `Main`, `Band1`, `Band2`, each `(edge_level, fill_level, edge_width)`.
`edge_level` and `fill_level` are **clamped to `0..1`** (the literals at `0x008ac0d0` and
`0x008ac0f8`) and `edge_width` to **`0..24`** (`0x008ac1b4`). An absent attribute is zero:
the record is zero-filled first. Confidence 85.

`Update` (`0x0017af80`) keeps a working copy of the record at `+0x25c` and, once per call,
**eases it toward the row of the screen showing**: `copy = copy * 0.97 + target * 0.03`
(the literals `0x3f7851ec` and `0x3cf5c280` at `0x008ac0ec`/`0x008ac0f0`), for the levels,
the width, the blur and the band numbers alike. The call takes a `dt` and does not use it
for the easing, so the ease is per frame, not per second: about 0.55 seconds to 63% at
sixty frames. `Load` zero-fills the copy, so **the picture fades in from nothing** on the
first page. Confidence 90 on the constants, 70 on "once per frame" (nothing read calls it
otherwise).

`blur` leaves as `blur * (1/1080) * height` (the literal at `0x008ac0e8`, `height` the
target's) into the engine's global blur radius (`g_RenderGlobals + 4`, `0x003de210`), which
`Render` runs at its end through `0x003e3e50`. Confidence 80 on the scaling, 62 on what the
pass is: it runs a chain of offset passes of doubling radius, and their weights are unread.

## The clock and the camera

`0x0017bac0` takes the model's animation time and wraps it: `t = fmodf(t, 60.0)` (`0x008ac110`
is `0x42700000`), and on a wrap restarts the animation. Omega's skin authors the same value
as `AnimLength="60.0"`; HD's does not name it and its binary holds the literal, which is why
an HD string search for `AnimLength` finds nothing. The scene's own clocks agree: every
animated node in the `.vex` carries `LoopEnd` 3,600 frames at 1/60, and they are the
`Anim Transform` nodes `oag_vex::vex::anim_transform` already reads (Confidence 90).

`UseModelCamera="true"` makes the widget use the scene's camera: `cameraShape1`, parented by
`camera1` and `camera1_group`. `camera1` carries **2,274 translation and 699 rotation keys**
over the 60 seconds, so the picture is a fly-through, not a still. The camera payload
(`oag_vex::camera`) holds no field of view - `+0x1c` is `1.5`, an aspect, as on the flyers.
The field of view is the model widget's default `fov` attribute, **`57.3` degrees**
(`0x001d326c`, the literal `0x42653333` at `0x008ae2e8`), vertical, which is `1.000` radians
to three places - what `Flyer_Item` measured with `tanf(0.5)`. Near and far are the widget's
own `nearZ`/`farZ` (`0.5`, `50000.0` in Omega's skin; the defaults are `20` and `100`,
`0x001d3264`/`0x001d3284`), and the aspect is the display's. Confidence 80.

## The filter: `FEBackgroundAnim_fp`

`Render` (`0x0017be68`) clears a target to white (`FUN_00678138(0xffffff)`), draws the scene
into it through the base class's `0x001d35e8`, and draws a full-screen quad through
`FEBackgroundAnim_fp` with two constant blocks:

- `params` (`0x7031f10c`): `(w, h, -w/2, -h/2)` with `w = edge_width / target_width` and
  `h = edge_width / target_height` (`0x0017c0a4`-`0x0017c124`).
- `params2` (`0xc1112c5a`): `(edge_level, 1 - fill_level, fill_level, 0)`.

The program (`SHO` block `0x00936200`, registered at `0x0017d3f0`; vertex program `0x00936800`
is a full-screen quad, and `FEBackgroundAnimCopy_fp` `0x00936100` a plain texture read)
decodes with `scripts/ps3-microcode.py fp 0x936200` and reads as a **Roberts cross**:

```
base = uv + (-w/2, -h/2)
t0 = tex(base)            t1 = tex(base + (w, 0))
t2 = tex(base + (w, h))   t3 = tex(base + (0, h))
e  = min(6 * sum_rgb(|t0 - t2| + |t1 - t3|), 1)
a  = floor(0.340088 * sum_rgb(t0))        ; 1 where the scene left the white alone
g  = 1 - e * edge_level - (1 - e) * (1 - a) * fill_level
out = (g, g, g, 1)
```

The two absolute values are the source-absolute bits (bit 29 of the second word for the first
source, bit 18 of the third for the second, as `nvfx_shader.h` places them), which
`scripts/ps3-microcode.py` does not print; they were read off the raw words
(`@0x19`, `0x3c9dc908`/`0x0005c900`). The taps at `uv + (w, 0)` and `uv + (0, h)` read an
unwritten register half as zero, which the program relies on (it is the only way the four taps
form a cross). So: **a white page, filled areas darkened by `fill_level`, edges by
`edge_level`, `edge_width` pixels wide**. Main Menu asks `0.6 / 0.2 / 0.5`, most other
screens `0.3 / 0.1 / 1.5` with `blur` 3 to 6 at 1080 lines. Confidence 85 on the arithmetic,
75 on the two absolute bits.

## What Omega does with it

Omega's `skin.xml` (`data09.psarc`) carries the same widget with three more `<Values>` rows
for PSVR (`IsVR="true"`, `FrontEndScene1_VR`..`3_VR`) and a child `<Model name="GroundPlane">`
naming `frontendscene_ground_VR.vex`; its `bgAnimFury` is unused because Omega ships **no
`.points2`** in any of its nine archives. Its scene, `FrontEndScene_HD_ATG.vex`/`.rcsmodel`,
is **byte-identical in `data00` and `data08`**, and is HD's file re-exported: the geometry is a
PS4 `.rcsmodel` (9,184 triangles, 56 mesh objects, one material, `Basic_Emissive`) and the
motion is in the `.vex` beside it, not in a `.rcsskeleton`/`.rcsanimclip` pair. The shape names
of the two files agree, which is how
`oag_render::mesh::rcs::psp2::placement::plan_from_vex` binds them: 54 of the 56 meshes hang
under one of the seven `Anim Transform` nodes.

**The boot screens have no backdrop, by the skin's own structure.** `Language Selection`,
`EpilepsyWarning`, `FirstPlay`, `Save Warning`, `EULA`, `Studio Logo` are siblings of `Top FE
Screen`, not children of its `FE Screen`, so the widget is not under them. The screens that
have it are `Main Menu` and everything the navigation controller opens.

## Chosen, not measured

- **The blur kernel**: a separable gaussian of half the radius's sigma. The engine's weights
  are unread (`0x003e3e50`).
- **Linear sampling** of the scene target. The sampler state is unread; the taps sit half a
  pixel off a texel centre at `edge_width` 1, where linear and nearest differ.
- **The page-to-row mapping**: this build's root page takes `Main Menu`, every other page
  `default`. The disc's `SoundTest`, `Manual`, `Controls` and `Race Records` rows (`blur` 6) have
  no page here to claim them.
- **A still's look is fully eased in**; a live stage fades in as the original does.
- **The scene's shading**: drawn by the mesh pass the menus' other models use. Only "not white" and
  colour differences matter to the filter.

## Not drawn

- **Bands** (`use_bands`): `false` on every row of every skin read. Parsed, reported, not run.
- **`GroundPlane`**: named `_VR`, no `IsVR` flag of its own; not drawn.
- **HD's own HD-style menu.** The same widget is in HD's `skin.xml`, but this build draws it
  only where `oag_title::BootProfile::menu_scene` is set, which is Omega alone. The one RPCS3
  capture of HD's HD style (`hd-frontend.md`) shows a flat white page behind the menu. Whether
  that was the fade-in from a zero working copy, a capture that was too early, or a widget
  that draws nothing there is not settled, so no reference exists for this picture: **it is not
  validated against the original.**

## Open

- The blur pass `0x003e3e50`: read its kernel.
- The scene target's size and sampler state.
- Whether Omega's PS4 build reads `AnimLength` or keeps HD's literal: no PS4 program is
  open in the bridge.
- The live campaign stage and the end-of-race screens sit under the same widget and draw
  it in the original; this build's do not yet (the campaign *stills* do).
