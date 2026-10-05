# HD's model shadow maps: one map per ship, cast along the circuit's sun

2026-09-11. Read because the `original` shadow tier on Wipeout HD put every
craft's shadow on the wrong side of the craft, and the question "which way
does HD's own `Job RenderModelShadowMaps` look" had never been answered from
the code - [`shadows.md`](../../../rendering/shadows.md) carried "built around
the `.envsettings` sun" as an assumption. It is now a reading. The bug itself
turned out to be this project's receiver lookup and not the direction (the
shader comment in `mesh.wgsl` and the test in
`crates/render/tests/shadow_map_coverage.rs` record that), but the reading
stands on its own.

Static only, so every score here caps at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md). Every
TOC-relative load was resolved with `scripts/ps3-toc.py resolve` against the
function's own OPD TOC rather than read off Ghidra's symbol names - see
[memory.md](memory.md) for why that matters on this binary.

## How the job was found

`Job RenderModelShadowMaps` is the string at `0x007b1060`. Its one referrer
(`ps3-toc.py attrib`) is `Scene_PrepareFrame` (`0x003aa888`), which
constructs the job object at `0x003ae200..0x003ae268`: base vtable
`TOC[-0x6278]` = `0x008621c8`, then after the base constructor
`TOC[-0x6260]` = `0x0086c880`, then the most-derived `TOC[-0x6258]` =
**`0x0086c898`**. Slot 3 of that vtable is the run function, the same slot
[`rcsmaterial.md`](../../../formats/rcsmaterial.md#the-frames-pass-order)
already used to reach `RenderModelShadowsOnTrack`: OPD `0x0088b838` ->
trampoline `0x006cde70` -> **`0x003ed810`**.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x003ed810` | `Shadow_RenderModelShadowMaps` | 82 |
| `0x003ec3e8` | `Shadow_BuildShipShadowMatrices` | 80 |
| `0x003a9520` | `EnvSettings_GetOrCreate` | 84 |
| `0x003e4d50` | `Shadow_RenderModelAmbientShadowsOnTrack` | 80 |
| `0x004053e0` | `Shadow_CompileShadowedTrackRedraw` | 75 |
| `0x00402a88` | `Shadow_CompileAmbientShadowTrackRedraw` | 72 |

The last three are from the second pass below: the ambient job's (empty)
run function, and the two track-chunk compilers that bind `shadowMatrix`
/ `shadowMapTex` and `ambientShadowMatrix` / `ambientShadowTex`
respectively. 75 and 72 rather than higher because what the compiled stream
*draws with* - the shader variant and its colour - is not read; the
parameter slots, the blend state and the chunk cull are.

### `Shadow_RenderModelShadowMaps` (`0x003ed810`)

The job's run function. In order:

1. `EnvSettings_GetOrCreate()`, then loads the vec4 at `+0x450` of the block
   it returns and normalises it (`vrsqrtefp` plus one Newton step). That
   field is **`Lighting.Sun direction`** - see the next function.
2. Sets up the map target (`FUN_005c2380(ctx, 1,1,1,0)` / `FUN_005c2524`,
   colour-mask and clear shapes; the identities are not read) and sets the
   material-state flag word `+4 |= 0x4000`. **Read 2026-09-21 on
   [ship-sun-occlusion.md](ship-sun-occlusion.md)**: those are
   `Rsx_SetColorMask` and `Rsx_SetDepthMask`, the per-ship map is bound as
   the *depth* target with colour writes off and front faces culled, so the
   map holds depth and the receiver's `TXP` is a compared sample - not
   coverage, as `shadows.md` had it.
3. Walks the ship table: count at `0xc86880 + 0x933c4`, index list at
   `+0x933c8`, records `0x1b0` bytes apart. A ship is drawn when its byte
   `+0x140` is non-zero and bit 0 of `+0xe4` is set - the same activity test
   the other per-ship passes use.
4. Per ship: `FUN_005a40f8(ctx, texture, ...)` (a target/viewport for that
   ship's own map slot), then **`Shadow_BuildShipShadowMatrices(sun,
   ship_world, bbox, ship+0x80, ship+0x40)`** where `bbox` is
   `*(ship+0x128) + 0x20` or null. Then it points engine parameter **1
   (`viewProj`)** at `ship+0x80` (`*(ctx+0xd8) + 0x38`, four vec4s) and runs
   the ship's compiled draw ops at `ship+0x12c` through
   `Render_RunCompiledOps`. So the *scene* camera is replaced by the light's
   for the duration of the ship's own draw - the model draws itself into the
   map with its ordinary vertex path.
5. After the loop, restores `viewProj` from the camera (`FUN_003aa2c8()`)
   and clears the `0x4000` flag.

**One map per ship**, each fitted to that ship alone: there is no grid-wide
fit and no shared box. `ship+0x40` is the matrix the receiving pass will
later publish as engine parameter 40 (`shadowMatrix`) - it is the bias form
of the same projection, below.

Confidence 82: reached through the job's own vtable from its name string,
every step is one contiguous function, and the per-ship loop's table and
activity test match the other passes'. Not higher because nothing here has
been watched under RPCS3.

### `Shadow_BuildShipShadowMatrices` (`0x003ec3e8`)

`(sun, ship_world, bbox_or_null, out_view_proj, out_shadow_matrix)`, `sun`
already normalised. Read from the disassembly at `0x003ec3e8..0x003ec4a0`
(the VMX prologue) and the decompilation past it:

**The view.** With `M` the ship's world matrix (rows at `+0x00/+0x10/+0x20`,
position at `+0x30`) and `c` the constant vec4 at `0x007b3220` =
`(70.0, 0, 0, 0)`:

```text
eye    = M.pos + sun * 70.0
target = M.pos
up     = cross(sun, M.row2)          # lvsl/lvsr rotate + vmaddfp/vnmsubfp, the VMX cross-product idiom
view   = FUN_005a2b18(eye, target, up)
```

`FUN_005a2b18` is a right-handed look-at: forward = normalise(eye - target),
right = normalise(cross(up, forward)), up' = cross(forward, right), last row
`-eye . axes`. So **the map looks from a point 70 units up the sun vector at
the ship, along the sun**, and the sun vector points *towards* the light -
the same sense `oag_mesh::mesh_render::Light::direction` carries.

**The box.** The eight corners of `bbox` - or, when null, the fixed
`(-6,-6,-6)..(6,6,6)` cube the function writes once into
`0xc86880 + 0x97690` - go through `M * view`, and their x/y extents are the
orthographic bounds:

```text
proj = FUN_005a2780(min_x, max_x, min_y, max_y, near = 1.0, far = 140.0)
```

`FUN_005a2780` is a GL-shaped orthographic (`2/(r-l)`, `2/(t-b)`, `2/(n-f)`,
`-(r+l)/(r-l)`, `-(t+b)/(t-b)`, `(n+f)/(n-f)`), the near/far constants at
`0x008b7de0` / `0x008b7e74`. With the eye 70 out, `1..140` is a slab 69
units towards the light and 70 past the ship - which is where the road under
it is.

**The outputs.** `out_view_proj = view * proj` with the view's w column
masked, and `out_shadow_matrix = out_view_proj * bias`, the bias built from
`0x008b7e78` = `0.5` and `-0.5` in the usual `(0.5, -0.5, 0.5)` texture
mapping shape, plus an offset read at runtime (`*0x008b7e70`, a pointer to
`0x00f5b934`) added to `0.5` - a texel-centre term by shape, value unread.

Confidence 80: the vector idioms (cross product, normalise, look-at,
corner-extents) are each recognisable and the constants resolve, but the
reading of which row of `M` feeds the `up` hint is from register tracing
alone. It does not matter for the picture - an orthographic map's in-plane
rotation cancels between caster and receiver - so it was not chased further.

### `EnvSettings_GetOrCreate` (`0x003a9520`)

Lazily initialises the global environment-settings block at `iRam008b6fb4`
(flag at `+0x5c0`), writing every default and registering every key
against its field: 90 calls to the four registrars (`FUN_005d3ec0` colour,
`FUN_005d3cc8` vec3, `FUN_005d4418` float, `FUN_005d46b8` bool). The names
are the `.envsettings` vocabulary
[`envsettings.md`](../../../formats/envsettings.md) already lists - checked
through this function's own TOC: `TOC[-0x63c8]` -> `0x007b02d0`
`Lighting.Sun direction` (field `+0x450`), `TOC[-0x63c0]` `Lighting.Physical
Sun direction` (`+0x470`), `TOC[-0x63ac]` `Lighting.Ambient false direction`
(`+0x4a0`), `TOC[-0x6384]` `Lighting.Debug.Draw Ship Shadows` (`+0x508`,
default 1).

So the direction the shadow maps cast along is the same key this renderer
already lights with, and nothing else: no separate shadow light exists in
HD's vocabulary (2048 added one - see `shadows.md`).

Confidence 84: unambiguous, and the one thing not proved is that the block's
*runtime* value is the file's - the registrar is what the loader writes
through, and the loader was not traced here.

## The other three shadow jobs, read the same way (2026-09-11, later the same day)

Each job object's most-derived vtable was reached exactly as above -
`Scene_PrepareFrame` stores three vtables in turn, the last one wins - and
slot 3 followed through its OPD trampoline:

| Job (name string) | vtable | slot 3 -> run function | What it is |
| --- | --- | --- | --- |
| `Job RenderModelShadowsOnTrack` (`0x007b10f8`) | `0x0086c910` | `0x006cdf00` -> **`Shadow_DrawOccluderStencilVolumes`** (`0x003e6918`) | the stencil-volume pass [shadow-stencilvolume.md](shadow-stencilvolume.md) already traced |
| `Job RenderModelAmbientShadowsOnTrack` (`0x007b1118`) | `0x0086c928` | `0x006cdf30` -> `Shadow_RenderModelAmbientShadowsOnTrack` (`0x003e4d50`), **an empty function** (`blr`) | **dead** |

Two consequences:

- **"On track" is the stencil pass.** `RenderModelShadowsOnTrack` does not
  project the map by itself; it is the per-ship two-sided depth-fail stencil
  over the shifted box, and its accumulate half (`Shadow_CompileShadowedTrackRedraw`, `FUN_004053e0`, called from
  it with the ship's `+0x40` shadow matrix, its `+0xe0` map texture and the
  `0x008b7e24` = **50.0** extrusion distance) is a track-chunk re-draw
  compiler: it publishes the ship's matrix as engine parameter 40
  (`shadowMatrix`, written at `*(ctx+0xd8)+0x518`) and its map as parameter
  41 (`shadowMapTex`, `+0x53c`), then compiles the track chunks within that
  radius of the ship into a command stream under blend `FUNC_ADD`,
  `SRC_ALPHA / ONE_MINUS_SRC_ALPHA` (RSX `0x8006`, `0x302/0x303`). That is
  the compositing pass `shadows.md` had as "unread": the road is drawn
  again through its `ShadowToAlpha` variant, whose `1 - shadow` alpha
  selects between the freshly-drawn (shadowed) colour and what is already in
  the frame. What colour that variant computes - which is what decides *how
  dark* - is still the fragment program's business and unread. The stencil
  pass also reads the light from the same env block field, `+0x450`,
  negated and taken into the ship's own frame, so it and the map agree on
  the sun.
- **HD/Fury never draws `ambient_shadow.gtf`.** The ambient job's run
  function is empty, and its compiler twin `Shadow_CompileAmbientShadowTrackRedraw` (`FUN_00402a88`) - the same
  track-chunk compiler bound to engine parameters 73/74/75
  (`ambientShadowMatrix` at `+0x938`, `ambientShadowBlendFactor` at
  `+0x958`, `ambientShadowTex` at `+0x97c`) under blend `ZERO /
  ONE_MINUS_SRC_ALPHA`, a multiplicative darkening by the texture's alpha -
  has **no reference anywhere in the image**: its OPD entry `0x0088cab0`
  appears in no TOC and no code. So the nine per-team `ambient_shadow.gtf`
  are shipped for a pass that is compiled in and never scheduled. The `blob`
  tier that plays them is therefore a substitute the original itself does
  not draw, and `shadows.md`'s "plays the disc's own image" needs that
  qualifier.

## Measured live, 2026-09-11 evening: the block, the sun and the craft

Three `rpcs3-drive.py capture` runs on Talon's Junction with `--region
0x8b6fb4@:0x600 --region 0xd19c44:0x80 --region 0xc86880:0x18000` (kept
under `data/reference/hd-capture/talons-ships/`, `-env/`, `-shadow/`), read
back with a twenty-line struct decoder:

- **The global env block holds the circuit's own file mid-race.** At
  `[0x8b6fb4]` during the race: sun `(-0.7776, 0.5815, -0.2391)`, fog
  `(0.263, 0.216, 0.380)` at `0.001`, ambient `(0.404, 0.392, 0.510)`, sky
  rotation `17` - every value `talons_junction/track.envsettings` authors,
  unchanged. **No sign or axis transform between file and runtime**, which
  closes the question the direction reading above left at "the loader was
  not traced". The front end's own block is a separate object (`[0x9948ec]
  + 0x60`, at `0x328d51d0` that run) holding `fe.envsettings`' values; one
  earlier capture found those values in the *global* block too, in a run
  that never pressed through the health warning, so the global is the FE's
  until the race scene loads and the circuit's from then on.
- **The model table is real and populated only mid-frame.** Count at
  `0xd19c44` read `16` / `14` / `12` across three pauses (and `0` in a run
  paused elsewhere in the frame), two entries per craft (`+0xe4` flags
  `0x1c8b` and `0x0480`, same matrix), records `0x1b0` apart from
  `0xc86880`. Every craft's world matrix has row lengths of exactly
  **0.75** - HD carries the same global craft scale
  `oag_render::exhaust::CRAFT_ROW_SCALE` recovered on Pulse - and `+0x30`
  is the position.
- **Hover height matches.** Raycasting the recorded positions against this
  project's own Talon's track (`oag-game --pose x,y,z --ticks 1
  --log-every 1`, height above the nearest spline sample): `3.88` and
  `3.94` for two craft at speed on the straight, `4.33` and `2.76` on the
  banked corner, against the `4.0` this simulation hovers at. The grid
  reads `2.18`, but 9.65 units off the spline, so that one is the sample's
  distance and not a height.

So after the receiver's mirror was fixed, the cast direction, the sun's
value and the craft's height above the road all agree with the original, and
a plan-view render (`--camera-pose 10,-20,-195.9,0,-1,0,1,0,0`, +X
up-screen) shows this renderer's shadow exactly where that sun puts it:
ahead of the craft along `+X`, a little to `+Z`. The "shadow sits further
behind than the captures show" note that opened this leg was a misreading of
a chase-camera diff; withdrawn.

## What this changes for `oag-render`

- The direction was right: `shadow::map::Fit::towards_light` from the
  circuit's sun is what the original does.
- The fit is not: the original fits **one map per ship to that ship's own
  box**, where this renderer fits one map to the whole grid. That is a
  resolution difference and a documented choice (`shadow::map::SIZE`), not
  a placement one.
- `shadowMapTexSize` stays unread - this job never touches it.
