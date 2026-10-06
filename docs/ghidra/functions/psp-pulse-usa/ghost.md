# The ghost ship: `MeshNode_Ghost` and the lap recorder behind it

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, `pulse-psp-usa.chd`,
UCUS-98712), image base `0x08804000`. Read statically on 2026-09-23 as the
**look reference** for this project's own ghost (the replay format this
project ships is its own design, not a port of the one below). **Nothing on
this page is runtime-verified**: every score is a static read capped
accordingly, and the breakpoints that would raise them are listed under
"what is not determined".

Two halves:

1. **The draw** - vex class `0x3d4` `MeshNode_Ghost`, a `Mesh` with its
   submit and draw overridden. Three passes per mesh node: a depth lay, a
   fixed-weight cross-fade of the hull over the scene, and a stamp into the
   glow mask through a screen-space static texture.
2. **The recorder** - a race-mode child object that samples the player's
   pose at 20 Hz into a keyframe track in the engine's own `AnimTransform`
   payload format, keeps the **best lap**, and plays it back through a
   stock `AnimTransform` node.

## Addresses

| Address | Name | Conf | What |
| --- | --- | --- | --- |
| `0x08911480` | `MeshNode_Ghost_Register` | 90 | registers class `0x3d4`, vtable `0x08ad171c` |
| `0x08910e38` | `MeshNode_Ghost_ConvertTree` | 88 | turns every `Mesh` node of a freshly loaded model into a `MeshNode_Ghost` |
| `0x08910320` | `MeshNode_Ghost_Submit` | 88 | vtable `+0x34`: enqueues the node up to three times, one per pass |
| `0x08910fe0` | `MeshNode_Ghost_Draw` | 88 | vtable `+0x44`: the three passes |
| `0x0891055c` | `MeshNode_Ghost_RandomiseStatic` | 90 | two `rand() % 100 * 0.01` offsets for the static texture |
| `0x089105d8` | `MeshNode_Ghost_SetStaticTexMatrix` | 60 | builds and loads the pass-2 texture matrix |
| `0x08910e14` | `MeshNode_Ghost_RestoreTexMapMode` | 75 | `Gu_TexMapMode(0, 0, 0)` after the pass |
| `0x08910214` | `MeshNode_Ghost_Destroy` | 80 | vtable `+0x6c`, decrements the live-ghost count `0x08abf4dc` |
| `0x08910ef8` | `MeshNode_Ghost_LoadStaticGlow` | 88 | loads `Data\Tex\staticglow.mip` into `0x08abf4e0` |
| `0x08910280` | `Gfx_ComputeViewDepth` | 72 | a point's view-space depth for the sort key, `10000.0` for a NaN-tagged point |
| `0x08907828` | `Bloom_SetGlowMaskWritable` | 80 | `Gu_PixelMask(0)` when asked, `Gu_PixelMask(0xff000000)` otherwise |
| `0x08828740` | `Ghost_Create` | 80 | allocates the recorder (300 bytes), constructs it, applies `FE_ModelSkin` |
| `0x08837a88` | `Ghost_Construct` | 85 | the recorder's constructor |
| `0x08837e64` | `Ghost_Update` | 85 | vtable `+0x24` of `0x08ac9f90`: per-frame record, lap swap, visibility |
| `0x08837358` | `Ghost_RecordSample` | 88 | appends one rotation key and one position key |
| `0x08838544` | `Ghost_QuantisePositions` | 88 | packs a finished lap's positions to `s16` |
| `0x088375a8` | `Ghost_SetVisible` | 85 | binds or unbinds the playback track and the model's draw flags |
| `0x08837624` | `Ghost_LoadModel` | 82 | loads `<team><variant>.vex` as a ghost model |
| `0x088382b8` | `Ghost_LoadFromSave` | 82 | parses a saved ghost (format version 1) into the playback buffer |
| `0x08947cc8` | `Savedata_StartGhostLoad` | 65 | starts an asynchronous `SAVE.GHO` load |
| `0x0894983c` | `Savedata_HashGhostKey` | 75 | hashes `"%s%s"` of the track and the class name |

## How a model becomes a ghost

`Ghost_LoadModel` calls `Vex_LoadModel(node, path, 0x45000000, 0xfdb2,
0x3e9, 1)`. `Vex_LoadModel` tests `flags & 1` right where an ordinary load
calls `Mesh_BuildModelDrawData`, and calls `MeshNode_Ghost_ConvertTree`
instead. That walks the tree (child at `+0x10`, sibling at `+0x0c`) and, for
every node whose class tag is `Mesh_ClassTag()`, overwrites the vtable at
`+0x38` with the one `MeshNode_Ghost_Register` stored (`0x08ad171c`) and
bumps the live count at `0x08abf4dc`. So **the ghost is the ordinary team
ship model, loaded a second time with every mesh node reclassed**; there is
no ghost-specific mesh on the disc. A second class (tag `FUN_08a6e878`) has
`+0x80` set to `1` and a separate counter `0x08ab22f8` bumped; which class
that is was not read.

`Ghost_LoadModel` builds the path with `"%s%s.vex"` from a team record
field `+0x94` and the registry value `FE_TeamModel`, so it is the same
team-and-variant model the front end draws. `Ghost_Create` then applies
`FE_ModelSkin` to it through `Skin_ApplyToModel`, the same skin call the
player's own craft gets. A loaded ghost carries its own team in the save
(below) and falls back to the player's team when that model fails to load.

## The three passes

`MeshNode_Ghost_Submit` enqueues the node with sort key
`0x4d000000 | depth_key | pass`, where `depth_key` is
`0xfffff - (-view_z * 349.525)` masked to 20 bits and cleared in its low two
bits (`Gfx_ComputeViewDepth` returns `view_z`). The three passes of one node
share the depth key, so they draw in order 0, 1, 2. Pass 1 is gated on the
byte `0x08abf4b8` and pass 2 on `0x08abf4c0`; **both are `1` in the image and
have no writer** (the only xrefs are these two reads), so all three passes
always run. Confidence 88.

Pass state, read off `MeshNode_Ghost_Draw` and the two helpers it calls.
GU enums are the SDK's (`DepthFunc`: `2` EQUAL, `6` GREATER; stencil op `0`
KEEP, `2` REPLACE; `AlphaFunc` `6` GREATER; blend factor `10` FIX). PSP depth
runs reversed, so `GREATER` is the ordinary "nearer passes" test.
`Gu_DepthMask(1)` **disables** depth writes, and `Gu_PixelMask(m)` masks the
bits set in `m` (the top byte is the alpha channel, which on the GE is the
stencil and therefore the glow mask - see
[glow-mask.md](../../../rendering/glow-mask.md)).

| | Pass 0: depth lay | Pass 1: hull | Pass 2: static into the glow mask |
| --- | --- | --- | --- |
| state list | `Gfx_BuildBatchStateList(ctx, 0x1, 0x1, 0xfdb2, 0x3e9, 0)` | `Gfx_BuildBatchStateList(ctx, 0x102, 0x1, ...)` then overrides | none of its own; overrides on top of pass 1's |
| depth test | GREATER | **EQUAL** | **EQUAL** |
| depth write | on | off | not set (inherits off) |
| colour write | **RGB masked** (`PixelMask(0xffffff)`), alpha written | RGB written, **alpha masked** (`Bloom_SetGlowMaskWritable(g_bloom, 0)` = `PixelMask(0xff000000)`) | **RGB masked** (`PixelMask(0xffffff)`), alpha written |
| texture | off (`Gu_Disable(9)`) | the ship model's own texture per batch (`model+0x1a4[material[batch byte 2]+4]`) | `staticglow.mip` through a texture matrix |
| lighting | off (`Gu_Disable(10)` at entry, all passes) | off | off |
| blend | off | **ADD, FIX `k`, FIX `1 - k`** (grey, `k` below) | off (`Gu_Disable(4)`) |
| material colour | - | `0xffffffff` | `0xffffffff` |
| alpha test | on, ALWAYS | on, GREATER `0` | on, **GREATER `0x80`** |
| stencil | ALWAYS, ref `g_display+0x1178` (the `4` the glow mask reads as "no glow"), REPLACE on pass | test off, ops KEEP | **ALWAYS, ref `glow_ref`, mask `0xff`, REPLACE on pass** |
| cull | on (`pass_mask & 0x20` clear) | on | on |
| fog | off (`flags & 1`) | off | inherits |

After the loop every pass restores `TexMapMode(0,0,0)`, re-enables
texturing, sets `DepthFunc(GREATER)`, disables the colour test and sets
`PixelMask(0)`.

So on the original the ghost is:

- **Pass 0** lays the ghost's own depth and stamps the ordinary `4` into the
  glow mask, drawing no colour. That is what makes the next two passes see
  only the ghost's nearest surface (depth EQUAL), so a ghost never shows its
  own back faces through itself.
- **Pass 1** draws the textured hull, unlit, as a **constant-weight
  cross-fade** over whatever is already in the framebuffer:
  `out = tex * k + dst * (1 - k)`, the same `k` for R, G and B, alpha left
  alone. Nothing about it is additive; it is a translucent hull.
- **Pass 2** writes **no colour at all**. It stamps `glow_ref` into the
  glow mask wherever the static texture's alpha is over `0x80`, on the
  ghost's visible surface only. The shimmer a player sees is therefore
  **the bloom** (`Bloom_Draw`, [bloom.md](bloom.md)) picking up a noisy mask,
  not a texture drawn onto the hull. A port without the recovered bloom
  would show nothing for this pass.

Confidence 85 for the table as a whole (every value is a literal at the call
site; the pass-2 depth-write state is inherited and so not literal).

## The proximity law

At the top of every pass, before the switch:

```text
k        = 0.55          (0x3f0ccccd, also the initial value of 0x08abf4bc)
glow_ref = 255           (0x08abf4d8)
if a player craft exists (*(g_race_manager + 0x2c0)):
    d = |player_hull_world.row3 - ghost_model_world.row3|   (xyz only)
    x = max(0, d - 5.0)
    if x < 10.0:
        k        = x * 0.55 / 20.0
        glow_ref = (int)(x * 255.0 / 20.0)
```

`player_hull_world` is the world matrix of the node at `craft+0x8b0` (the
hull, per [shield.md](shield.md)); `ghost_model_world` is the world matrix of
the ghost model root (`FUN_08945200(node+0x70)`). Constants `5.0`
(`0x40a00000`), `10.0` (`0x41200000`), `20.0` (`0x41a00000`), `255.0`
(`0x437f0000`) and `0.55` are all immediates in the disassembly at
`0x0891116c`-`0x089111e0`. Confidence 90.

**The ramp is not continuous, and that is the code, not a misread.** Inside
5 units the ghost is fully transparent and stamps no glow. From 5 to 15 it
rises linearly to `k = 0.275` and `glow_ref = 127`. At 15 the `x < 10` test
fails and it jumps to the far values `k = 0.55`, `glow_ref = 255`. The
divisor is 20 while the clamp is 10, so the ramp only ever reaches half of
the far value. (The handover note this page replaces read it as "5..25"; the
branch at `0x089111a8` says 15.)

## The static texture and its matrix

`MeshNode_Ghost_LoadStaticGlow` loads `Data\Tex\staticglow.mip` (string at
`0x08a88420`), called once from the race-texture loader `FUN_08821d90`
alongside `Texture_LoadEngineNoise`/`Texture_LoadEngineFlare`. The entry
exists in the USA `Data.wad`: **entry 945, name hash `1469dee8`, 5,136
bytes**, a 32 x 128, 8 bpp, one-level, swizzled `.mip` whose palette alphas
include `0x00`, `0x85` and `0xff` - so the GREATER `0x80` test keeps the
`0x85` and `0xff` texels and drops the rest. Confidence 92 for the entry.

`MeshNode_Ghost_Submit` re-randomises the offsets whenever the float at
`g_ingame+0x40` differs from the copy it keeps at `0x08abf4ec` - once per
distinct value of that clock, so every frame while the race runs and never
while it is frozen. `MeshNode_Ghost_RandomiseStatic` draws each offset as
`(rand() % 100) * 0.01`, i.e. `0.00..0.99` in steps of `0.01`, off libc
`rand`, not the race generator. Scales are the globals `0x08abf4c4 = 4.0`
and `0x08abf4c8 = 1.8`, never written. Confidence 90.

`MeshNode_Ghost_SetStaticTexMatrix` (called with rotation `0`, so its
rotated branch is dead here) multiplies:

- the node's world matrix,
- the matrices at `g_display + 0x1190 + 0x40 * [g_display+0x1690]` and
  `g_display + 0x1410 + 0x40 * [g_display+0x1694]` - read as the current
  view and projection stack tops, **not confirmed**,
- an identity carrying the two offsets in its third row, and
- a matrix `diag(4.0, 1.8)` whose last two rows are the swapped unit rows
  `(0,0,0,1)` / `(0,0,1,0)`,

forces the fourth column to `(0,0,0,1)`, loads it with `Gu_SetMatrix(3, ...)`
(the texture matrix) and sets `Gu_TexMapMode(1, 0, 0)` - texture-matrix
mapping from **vertex position** - with `Gu_TexWrap(REPEAT, REPEAT)`.

**Reading (confidence 60):** the z/w swap puts clip-space `w` in the
texture's `q`, so after the GE's perspective divide the texture coordinates
are `u = 4.0 * x_ndc + off_u`, `v = 1.8 * y_ndc + off_v` - the static is
**screen-space**, tiled 8 x 3.6 times across the screen, and jumps to a new
random offset every frame. What keeps this at 60: the multiplication order
of the VFPU `vmmul` chain was read, not stepped, and a reversed order would
put the offsets on clip `z` instead. A GE-state capture of the texture
matrix during a Time Trial would settle it.

**Cross-title (2026-10-06):** HD **checked, differs** - a string search of
`/hdfury/EBOOT-ps3-hdfury-eu.elf` finds no `staticglow`, so HD never binds a ghost
static (confidence 85: absence of a string, not of an indirect name). It is
`WeaponModels::ghost_static = None` on HD, so HD no longer requests the entry or
logs it absent. Pure, 2048 and Omega ship the file and keep Pulse's request
(**ported**, chosen, not measured: neither executable names it).

## The recorder

`Ghost_Create` is called from exactly two places: **`TimeTrial_Construct`**
(`0x0882dc1c`) and **`FreePlay_Construct`** (`0x0882d3fc`). Time Trial and
Speed Lap share one constructor (see `oag_gameplay::damage_rules`'s doc), so
the three modes that have a ghost are Time Trial, Speed Lap and Free Play.
Confidence 85.

`Ghost_Construct` allocates **two** track buffers of the same shape at
`obj+0x40` and `obj+0x90` (`0x50` apart), one recording and one playing
(`obj+0xe4` is the recording index, `obj+0xe8 = 1 - it` the playing one).
Each buffer is laid out **exactly as the `AnimTransform` payload**
[anim-transform.md](anim-transform.md) decodes - translation count at `+0x02`,
rotation count `+0x04`, the two key-time arrays at `+0x08`/`+0x0c`, base at
`+0x10`, rotation values `+0x1c`, quantum `+0x20`, translation values `+0x2c`,
flags `+0x34 = 2`, seconds per key `+0x3c` - and a stock `AnimTransform`
node (`FUN_088fde6c`, child at `obj+0xe0`) has its payload pointer
(`node+0x50`) aimed at the playing buffer. So **playback is the engine's own
keyframe animator**: lerped translation, slerped rotation, clamped at the
ends. The flag `2` selects the 8-bit rotation evaluator (`0x088ff6ec`), which
anim-transform.md lists as never exercised by scenery - the ghost is its
user. Confidence 85.

- **Sample rate: 20 Hz.** `0x08a7afac = 0.05` seconds per key, stored into
  `+0x3c`. `Ghost_Update` accumulates frame time in `obj+0xf8` and writes
  every key index from the current count up to `(int)(t / 0.05)`, so a slow
  frame is caught up rather than skipped. Confidence 88.
- **Cap: 90 seconds.** `0x08a7afa8 = 90.0`; recording stops once
  `t >= 89.0`. The key capacity `0x08ab0d60` has no writer in the image and
  reads `0` statically - presumably filled by a static initialiser not
  traced; `90 / 0.05 = 1,800` is the consistent value. Confidence 70 for
  the cap, 40 for the capacity figure.
- **What is sampled: the craft's pose.** Each key takes the matrix at
  `craft+0x794`, scales its three basis rows by `g_craft_scale` and keeps
  row 3 as the position. `Ghost_RecordSample` renormalises the basis,
  converts it to a quaternion (`FUN_08970a50`), flips it so `w >= 0`, and
  stores `x, y, z` as **signed bytes, `* 127`** (three bytes a key, `w`
  reconstructed). Positions are stored as raw `f32[3]` while recording.
- **Positions are packed at the swap.** `Ghost_QuantisePositions` takes the
  finished lap's bounding box, stores its centre as the base and
  `half_extent / 32767` as the per-axis quantum, and rewrites the positions
  in place as `s16[3]` - the payload's own translation encoding. A lap is
  therefore 9 bytes a key (plus two `u16` key times), about 16 KB for a
  90-second lap. Confidence 88.
- **Inputs are not recorded.** Nothing in the recorder reads the pad or the
  controls; it is a pose track only.

### Best lap, restarting at the line

`Ghost_Update`, every frame, keyed on the race mode's state
`g_race_manager+0x7c8` ([zone-mode.md](zone-mode.md) has the five states):

1. In states 3 and 4 (results and after) the ghost is hidden and nothing is
   recorded.
2. On the craft's **line-crossing tick** (`craft+0x911`, per
   [race-progress.md](race-progress.md)) once a lap has actually completed
   (`craft+0xacc > 2`), or on the first frame of state 2:
   - the lap just finished (`craft+0x92c`, **centiseconds**) is compared
     against the best held at `obj+0x3c` (initialised to `FLT_MAX`); if it
     is lower and non-zero it becomes the best, the two buffers **swap** (the
     lap just recorded becomes the one played) and its positions are packed;
   - the new recording buffer is emptied, the playback node's clock is reset
     to 0, and the first key is written from the craft model's world matrix.
   So both the recording and the playback **restart at the line every lap**,
   and the ghost always shows the best lap of this session (or the loaded
   one).
3. Otherwise it records keys as above and sets visibility from the options
   byte `options+0x45c` (`Ghost Ship Visible`, read by `FUN_08809bc8` from the
   profile: anything but `FE_OFF` is on).

`Ghost_SetVisible(0)` detaches the track (`node+0x50 = 0`) and clears flag
bits `6` on both the animator and the ghost model; `(1)` re-aims it at the
playing buffer and sets them. Confidence 85 for the whole sequence.

### Saving and loading

- **Keying: track and class, not mode.** `Savedata_StartGhostLoad` builds
  the savedata directory from the game id string `UCES00465`, `G%x` of
  `Savedata_HashGhostKey(track, class_name)` - `Libc_HashString` of
  `"%s%s"` (format at `0x08a89d14`), the class name coming from the
  `Venom`/`Flash`/... table at `0x08ab076c` - and the file name
  `SAVE.GHO`, into a `0x40000`-byte buffer. No mode enters the key, so **Time
  Trial, Speed Lap and Free Play share one ghost per track and class**.
  Confidence 72.
- **Auto load.** `Ghost_Construct` starts that load when the profile's
  `Auto Load Ghost` reads `FE_ON`.
- **Format** (`Ghost_LoadFromSave`, version word `1` required):

  ```text
  +0x00 u32      version, 1
  +0x04 char[32] team name the ghost was flown with   -> obj+0x104
  +0x24 u32      lap time, centiseconds               -> obj+0x124, becomes the best
  +0x28 u16      key count (both channels)
  +0x2a f32[3]   position quantum
  +0x36 f32[3]   position base
  +0x42 s8[3n]   rotation keys
  ...   s16[3n]  position keys
  ...   u8       model variant, passed to Ghost_LoadModel
  ```

  Key times are not stored; the loader regenerates both as `0, 1, 2, ...`.
  The model is loaded from the saved team and falls back to the player's
  team if that fails. Confidence 82.
- The EndRace side (`ER_SAVE_GHOST`, `EndRaceMenu_PopulateExistingGhost`) is
  [endrace-screens.md](endrace-screens.md).

## What is not determined

- **The texture-matrix order** (confidence 60 above) - a GE capture of the
  pass-2 texture matrix during a Time Trial would settle whether the static
  is screen-space as read.
- **Pass 1's texture function** is not set on this path; it inherits
  whatever the mesh state left (modulate is the expectation, not read).
- **Pass 2's depth-write state** is inherited from pass 1 (off) unless
  another node's draw interleaves between the two passes of one node; the
  sort key makes that unlikely but does not exclude it.
- **The capacity `0x08ab0d60`** has no located writer.
- **The class `FUN_08a6e878` names** in `MeshNode_Ghost_ConvertTree` (the
  one whose nodes get `+0x80 = 1`).
- **No runtime check at all.** A breakpoint on `MeshNode_Ghost_Draw` during a
  second Time Trial lap would confirm the three passes and the `k` values,
  and a `memory.read` of the displayed framebuffer's alpha around the ghost
  (the [glow-mask.md](../../../rendering/glow-mask.md) method) would show
  the static stamp directly.
- **HD's ghost** (`ps3-hdfury-eu` also registers `MeshNode_Ghost` `0x3d4`,
  per [vex-classes.md](../ps3-hdfury-eu/vex-classes.md)) was not read.
