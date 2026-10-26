# Pure's sun lens flare: the importer, the node, the particle and the occlusion probe

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Lane `pure-lens-flare-re`, 2026-10-26: static read of
every function below plus live reads on PPSSPP (Vineta K, Time Trial grid state,
software renderer): object, vertex buffer, depth buffer and GE-list dumps, write
watchpoints. Runtime addresses equal Ghidra's. The decompiler's constants in
unrelocated code (`0x2be754`, `0x276c74`) are Ghidra addresses minus `0x08804000`.

The effect is **two objects**. A `Lensflare` scene node (class `0x390`, built by the
importer) owns a per-frame sun position and an occlusion probe; it creates one
`FxLensflare` particle object whose draw puts ten quads on screen. The law, with the
numbers a renderer needs, is [`docs/rendering/lens-flare.md`](../../../rendering/lens-flare.md).

## Functions

| Address | Name | Confidence | What it does |
| --- | --- | ---: | --- |
| `0x0888c380` | `Lensflare_ConstructNode` | 85 | node constructor: base ctor `0x08881824`, vtable `0x08abe0d8` (read live at `node+0x3c`), source name `Lensflare_Importer.cpp`, `flags \|= 4` |
| `0x0888c444` | `Lensflare_ImportNode` | 90 | the class importer (`Lensflare_ImportNode` is the string next to its registration): copies the `0x44`-byte payload block, reads `ColourR`/`ColourG`/`ColourB` (`strcasecmp`, missing reads 0), normalises payload row 3 into `node+0x140` (w 0), allocates the `0x110`-byte `FxLensflare` and calls its constructor, stores it at `0x00061064`, registers `Lensflare_ReadOcclusion` with `0x088ecc48` |
| `0x0888c91c` | `Lensflare_UpdateSunPosition` | 90 | node vtable entry 6, every frame: `object+0x50 = (node+0x140) - camera+0x70` with w 1, then enqueues the node with key `0x4b000001`. Watched live: it writes `+0x50` once per frame |
| `0x0888caa8` | `Lensflare_QueueOcclusionProbe` | 85 | the node's queued callback: projects `object+0x50` through the view and projection stacks, behind the camera (view z > 0) or more than 4 px off screen sets the probe pixel to `(1000, 1000)`, else to the sun's pixel (live `(131, 77)` for NDC `(-0.4515, 0.4343)`); stores the depth-buffer base (`VRAM + 0x710000`) at `0x002a7d04`, the pixel at `0x002a7d08`/`0x002a7d0c`, and ends a small GE list with a signal `0x200` |
| `0x0888c248` | `Lensflare_ReadOcclusion` | 90 | the GE-signal handler (event `0x200`): samples the depth buffer at 16 points around the probe pixel and writes the covered fraction to `object+0x74` |
| `0x08936a2c` | `FxLensflare_Construct` | 90 | the particle constructor: ten rings, vertex buffer `0x5a0` bytes, colour buffer `10 * 16`, atlas cells, per-ring colour chain, final qsort of the block pointers |
| `0x089365c4` | `FxLensflare_UpdateIntensity` | 92 | vtable entry 4: clears flag 4, angle between camera axis and sun direction, `+0x6c = +0x70 = 1 - angle` for angle <= 1 rad, sets flag 4 when `+0x74 > 0` |
| `0x08936758` | `FxLensflare_QueueDraw` | 80 | vtable entry 6: enqueues the object with key `0x4e000000` unless `renderer+0x5ded` is set |
| `0x08937200` | `FxLensflare_Draw` | 92 | vtable entry 8: the draw: orthographic setup, sun to screen, ten `FxLensflare_WriteQuad` calls, one 60-vertex strip draw |
| `0x08936794` | `FxLensflare_WriteQuad` | 90 | writes one ring's six vertices: positions `centre +- half`, packed `ABGR`, copies the first and last vertex for the strip's degenerate joins |

| Address | Kind | Name | Confidence |
| --- | --- | --- | ---: |
| `0x08abe0d8` | data | `Lensflare_NodeVtable` | 90 |
| `0x08ac2754` | data | `FxLensflare_Vtable` | 90 |

Both vtables were read live: `node+0x3c = 0x08abe0d8`, `object+0x3c = 0x08ac2754`. A
table is 8-byte `{adjust, pointer}` entries (the pointer in the odd word). The node's
entries 6, 8 and 14 hold `0x0888c91c`, `0x0888caa8` and `0x0888c444`; `FxLensflare`'s
entries 4, 6 and 8 hold `0x089365c4`, `0x08936758` and `0x08937200`. The names "update",
"queue draw" and "draw" follow what each function does; the base classes' other entries
were not read.

## What each read gave

### `Lensflare_ImportNode` (`0x0888c444`)

`FUN_08885b48` is the base importer's pre-step; `FUN_08885b90` its post-step. The
importer advances the arena pointer `param_2+4` by `0x44` (the 64-byte matrix plus one
word; live at `node+0x150`: identity rows 0-2, row 3 `(-51.826, 12.774, 23.323, 1)`,
word `187`) and never reads the word. The payload is 80 bytes
(`crates/pure/tests/class_table_ground_truth.rs`), so `+0x44..0x4f` is unread too.

The colour is not in the payload. `node+0xc8` is the header's attribute list
(`oag_vex::vex::node_attributes` walks it); the importer looks up `ColourR`,
`ColourG`, `ColourB` by `strcasecmp` and reads an `f32` at the entry's own value
offset, `0` when absent. Alpha is the constant `1.0`.
Vineta K reads `(0.71074, 0.71074, 0.71074)` from the file and `(0.71074, ...)` at
`object+0x60`, live. The colour and the direction are passed to the constructor as
`param_4` and (through `node+0x140`) the update; the constructor's `param_3`, `(0, 0,
0, 1)` from `0x002abbf0`, is overwritten every frame.

The direction is the normalised payload row 3 and **only that**: `node+0x140` read live
is `(-0.88972, 0.21929, 0.40039)`, exactly the file's row 3 normalised, with no parent
chain applied. Confidence 90.

### `Lensflare_UpdateSunPosition` (`0x0888c91c`) and the camera

`camera = [0x0028a1cc]` (`0x08e73f00` live, a 4x4 per frame at `+0x40`, translation row
at `+0x70`). The function writes `object+0x50 = direction - camera.translation` (so
`+0x50` is a *camera-relative* point at about the camera's own distance, not a unit
vector: live `(48.98, 40.66, -96.30)` with the camera at `(-49.97, -40.44, 96.69)`),
and `FxLensflare_UpdateIntensity` undoes the subtraction. The draw transforms `+0x50`
by the view stack.

### `FxLensflare_UpdateIntensity` (`0x089365c4`), checked to four digits

`FUN_08a32e08` returns `v = (cam[0x40].z, cam[0x50].z, cam[0x60].z)`; live
`v = (0.9953, 0.0967, -0.0)`. The code takes `d = normalise(object+0x50 + camera.translation)`
(the sun direction), `c = dot(d, -v)`, requires `c > 0`, clamps `c` to `[-0.999, 0.999]`,
and `a = acos(c)` (`FUN_088daed0` is libm `acos`). `a <= 1.0` gives
`+0x6c = +0x70 = 1.0 - a`.

Vineta K live: `d = (-0.88972, 0.21929, 0.40039)`, `c = 0.88972*0.9953 - 0.21929*0.0967 =
0.8643`, `a = 0.5264`, so `1 - a = 0.4736`; **`+0x6c` reads `0.47363`** (confidence 92).

### `Lensflare_QueueOcclusionProbe` and `Lensflare_ReadOcclusion`

The signal callback's loop (disassembly `0x0888c2c0..0x0888c33c`): rows `y-3, y-1, y+1,
y+3` and columns `x-3, x-1, x+1, x+3` (step 2, so sixteen samples), `halfword = buffer[row *
512 + column]`, each in `[0, 480) x [0, 272)`, counted when `halfword < 0x3b3`; `object+0x74
= count * 0.0625`. A probe pixel with `y == 1000` writes `0`.

Live, paused at the loop's exit (`0x0888c344`): probe `(131, 77)`, count 12, `+0x74 =
0.75`. The 9 x 9 depth window around it read `23` on columns 127-132 and `4300..4311` on
columns 133-135, a pole in front of the sun. Sampled columns are 128, 130, 132, 134:
three sky, one pole, four rows each: 12 of 16 (confidence 90). Sky is `23`, the pole `4300`:
the game's 16-bit depth runs 65535 near to 0 far, so `< 0x3b3 = 947` means "almost
nothing in front". The buffer reads all-zero once the frame has moved on (PPSSPP, when
read between callbacks), so the callback is the only place the values are meaningful.

Unread: which frame's depth the callback sees (the signal is at the end of a list built
earlier in the frame), and the exact rounding of the probe pixel (`131` from `131.66`,
`77` from `76.9`: the probe uses the 16-px quad's centre through `FUN_088da814`).

### `FxLensflare_Construct` (`0x08936a2c`)

Ring count is the constant `10` (`+0xb0`). The vertex buffer is `0x5a0 = 10 * 6 * 24`
bytes (a block is six `(u, v, ABGR, x, y, z)` vertices: four corners plus the first and
last repeated), the colour buffer `10 * 16`. The atlas cell of block `j` is
`0, 1, 2, 3, 3, 1, 2, 3, 3, 1` (the stack array `{0,1,2,3,3}` and the index resetting
from 5 to 1; read live from the blocks' UVs). `u0 = (cell & 1) * 0.5`, `v0 = (cell >> 1) * 0.5`.

Then a **qsort over the ten block pointers at `+0xbc`** (comparator `0x08936598`, no
Ghidra function; arguments `(base, 10, 4, cmp)`). Live pointers: ring `i` draws block
`[0, 1, 9, 5, 6, 2, 4, 3, 7, 8]`, so the cells by ring are `[0, 1, 1, 1, 2, 2, 3, 3, 3,
3]`: **sorted ascending by cell**. The order inside a cell (`1,9,5`; `6,2`; `4,3,7,8`) is
the sort's, not a rule, and does not change the picture. The comparator is not
decompiled; the sort key is inferred from the result (confidence 75).

Colour chain: `colour[0]` is the authored colour; each next ring takes the previous
ring's colour through RGB to HSV (sextant hue `0..6`), adds `-0.01 * k` to the hue
(`k` the 1-based ring step, so the shift compounds: ring `k` is `-0.005 k (k+1)` sextants
from ring 0), wraps into `0..6` and converts back (`0x08936ff4` is the seven-case
switch, standard). A grey colour has no hue, so all ten are equal: Vineta K's ten colour
entries read `(0.7107, 0.7107, 0.7107, 1.0)` live. **The hue path is a static read** (confidence 70);
the six coloured circuits (Modesto Heights `(1, 0.6, 0)`, ...) were not run live.

### `FxLensflare_Draw` (`0x08937200`)

Everything in the law page's tables is read from this function and confirmed against
the ten live quads. The state it sets, confirmed against the recorded GE list (the
60-vertex `PRIM`, strip, vertex type `0x19f`):

| GE state at the PRIM | Value | Meaning |
| --- | --- | --- |
| `BLENDMODE 0xDF` | `0xA2` | source `SRC_ALPHA`, destination `FIX`, equation add |
| `FIXA` / `FIXB` (`0xE0`/`0xE1`) | `0` / `0xFFFFFF` | destination weight 1: **additive**, `src * alpha + dst` |
| blend enable | 1 | |
| depth test, depth mask | off, writes disabled | |
| alpha test, cull, lighting | off | |
| `TEXFORMAT 0xC3` | `5` | 8-bit indexed, CLUT `RGBA8888` (`CLUTMODE 0xFF03`) |
| `TEXSIZE0` | `0x606` | 64 x 64 |
| `TFILTER 0xC6` | `0x107` | minify mipmap-linear, magnify linear |
| `TFUNC 0xC9` | never written in the list | engine default |
| `PROJ` | `diag(0.5625, 1, -1)` | the ortho matrix `FUN_08a30a98(-16/9, 16/9, -1, 1, -1, 1)` |

Which wrapper is which (`0x088ede58` disable, `0x088edb84` enable, `0x088f08dc`
`BlendFunc`) is established by those values, not assumed. The draw pushes a copy of the
view and projection stacks (`renderer + 0x1690` and `+0x1694`), replaces them with an
identity view and the ortho projection, and pops both after the draw.

## Not read

- The comparator `0x08936598` (no function in Ghidra; the sort result is measured).
- `FUN_0882f70c`: a uniform random in `[a, b]` is consistent with every live sample (a shared size factor
  `0.982..0.986`, ring alphas `0.2..0.31` of `0.355`), but the generator is unread.
  `DAT_002883fc != 0` replaces the random factors with `1.0` and `0.7`; who sets it is unread.
- The writer of `_DAT_00034114` (`5/3`, set once where `_DAT_00034110 == 0`) beyond that
  constant initialisation.
- The queue's ordering: what the keys `0x4b000001` and `0x4e000000` sort against, so where in
  the frame the flare lands relative to the HUD and the ship.
- The texture binding: `object+0x100` holds the texture object `0x09737f50`, written before
  the first frame. The texels it names equal the track's `LensflareComponents.tga`
  byte for byte (below), so the *selection* is by name through the track's own texture, not
  a search of the writer.

## Texture

The recorded draw binds a 64 x 64, 8-bit, 256-entry palette texture at VRAM `0x04182380`
(CLUT `0x04181f80`, 1 KiB, directly before it). Decoded from live VRAM (unswizzled in
16 x 8-byte blocks) it equals `oag_vex::vex::textures`' decode of Vineta K's
`Data\defaults\LensflareComponents.tga` (`Texture` node #187, 64 x 64, 8 bpp, four mips) **byte for byte** (`cmp` of
the two RGBA dumps; confidence 95). The same decode, identical, is on all seven circuits that author a flare
(`crates/pure/tests/class_table_ground_truth.rs`).
