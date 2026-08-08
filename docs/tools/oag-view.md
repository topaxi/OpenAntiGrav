# `oag-view`

The first thing in the project with a window. Displays assets decoded straight
from a disc image, with nothing extracted to disk first.

It exists to prove the pipeline end to end: **CHD → ISO 9660 → WAD → LZSS →
texture decode → GPU**. If it draws the right picture, every layer beneath it is
right.

```sh
oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
oag-view <archive> --names data/extracted/psp/all.names
```

Left and right arrows change asset, Escape quits. With `--mesh`, `--track` or
`--collision` the window shows one model instead, and arrow keys orbit the
camera; see [Models](#models). The disc image is opened read-only and nothing is
written.

The 3D renderer itself is no longer part of this tool. The mesh loader, the wgpu
pipeline and its shader, the track-ribbon builder, the offscreen capture and the
camera maths live in `oag-render`, so the game draws a track and a ship through
exactly the same code; see
[workspace layout](../architecture/workspace-layout.md). What is left here is
the viewer: the CLI, the window, the key handling and the texture browser. That
also means a change in the picture below is now a change in a shared renderer,
not in a tool nothing else depends on.

## Headless screenshots

```sh
oag-view <archive> --index 0 --screenshot /tmp/logo.png
```

Renders one asset offscreen and exits. Works over SSH and without a compositor,
and gives the render path something a test can assert on. It is the basis for
visual regression checks once there is geometry to get wrong.

The screenshot path deliberately does **not** share a pipeline with the window:
it renders the texture straight, with no checkerboard and no letterboxing, so
the output is exactly the decoded image and a byte comparison means something.

**Verified:** a screenshot of the Pulse logo is byte-identical to the PNG that
`oag-wad extract --png` produces on the CPU. Upload, GPU texture, copy back and
encode preserve every pixel.

## The checkerboard is not decoration

Wipeout's UI textures are white artwork whose shape lives entirely in the alpha
channel. On any flat background they are either invisible or indistinguishable
from a solid rectangle, so the window composites them over a checkerboard.

Sampling is **nearest** for magnification. These are small textures, and
smoothing them would hide exactly the decoding errors this tool exists to find.

## Models

```sh
oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    --mesh 'Data\Ships\Feisar\Ship.vex' --screenshot /tmp/ship.png
```

Decodes a [`.vex` model](../formats/vex.md), flattens every mesh into one
vertex and index buffer, and renders it with a depth buffer and a fixed
two-light rig.

Without `--screenshot`, this opens a window with an **orbit camera**: Left and
Right rotate, Up and Down pitch, `+`/`-` (or PageUp/PageDown) zoom, Escape
quits. `--yaw` and `--pitch` set the starting angle. The window draws through
the same pipeline `--screenshot` does, so what you see interactively is what
the screenshot would have captured at that angle.

The orbit camera's arithmetic is `oag_render::camera::orbit`, which is a pure
function of the held keys and the elapsed time and is unit tested without a
window or a GPU. Only the mapping from real keys onto it is in this crate.

The lighting is not Pulse's. A single light leaves faces pointing away from it
unreadably black, and the point here is to see the geometry and catch decoding
errors, not to reproduce the game's look.

**Back-face culling is deliberately off.** Triangle-strip winding is
reconstructed rather than read from the file, so culling would turn a winding
mistake into invisible geometry instead of a visible artefact.

The camera frames the model from its own bounding sphere, so any model fills the
view whatever scale is baked into the file. That means **a wrong scale would
still look right here**: this is not a check on the scale factor. The mesh
header's own `f32` bounding box is what checks that, and note that the agreement
recorded in [vex.md](../formats/vex.md) was measured **by hand, once**. There is
no standing assertion, so a regression in the scale factor would not be caught
today.

### `--draws` and `--only`: attributing a wrong pixel to a draw call

```sh
oag-view <image>:<archive> --mesh 'Data\Environments\16_Track\track.vex' --draws
oag-view <image>:<archive> --mesh 'Data\Environments\16_Track\track.vex' \
    --only node:74 --screenshot /tmp/one-node.png
```

`--draws` prints one line per draw call - which of the three lists it is in
(and therefore which pipeline draws it), the scene-tree node it came from, its
triangle count, the texture it binds and that texture's size, how many whole
tiles of it the vertices span, the mean vertex colour, and the world centre -
followed by the material side of the same chain per batch (material index,
material flags, `pass_mask`, header byte 3, and whether the texture ordinal the
material names is in range and decoded), and a texture table.

Run it `--release` on a track. A circuit is ~2,000 draw calls over ~180,000
vertices and the debug build takes minutes to walk them; the release build
takes seconds.

The line to look for first is a draw binding **`none -> white 1x1`**: that is
the fallback `mesh_render::build` binds for an unresolved texture, and such a
draw paints white whatever its material meant to paint. A `uv` span well over
1.00 means the texture repeats, under 1.00 that one copy is stretched - which
is the difference between a small texture used as intended and one that is
being magnified because something upstream is wrong.

`--only <text>` then draws **only** the parts whose node name or texture label
contains that text, reframing the camera on what is left; `--only node:N`
selects one scene-tree node by index, which is the only handle a track's
geometry has, since every `Mesh` node in a track `.vex` carries an empty name.

The two together are what turns a guess into an attribution. Working out that
`16_Track`'s blown-out white board over the start line was `billboard8.tga` and
not the `col_banners2_ADD.tga` sign beside it took exactly this: the name said
one thing, `--draws` said the banner resolved its texture and sat in the blend
list, and hiding it left the white block untouched. See
[vex.md](../formats/vex.md#texture-rows-are-padded-to-16-bytes).

## Tracks

```sh
oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    --track 'Data\Environments\01_Track\track.vex' --screenshot /tmp/track.png
```

Draws the **driveable spline**, not the track's art meshes: the surface ribbon
built from each control point's own half-widths, coloured per visibility
`section`, with the authored racing line and the AI corridor edges drawn at hover
height above it. It also prints the path and junction graph.

Two reasons it draws the spline rather than the geometry. It needs nothing from
the scene hierarchy, which is still undecoded, so it works now. And a wrong
spline decode does not look subtly off, it looks like scribble, which makes this
a real check on [the track format](../formats/track.md) rather than a picture.

The camera looks down rather than along, because a track is flat and wide. The
same orbit window and controls described under [Models](#models) apply here.

Junctions show as small gaps in the ribbon: each path owns its own control
points, so consecutive paths are one step apart rather than sharing a vertex.

## Collision

```sh
oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    --collision 'Data\Environments\01_Track\track.vex' --screenshot /tmp/walls.png
```

Draws the [collision soup](../formats/collision.md) - the geometry the physics
world is actually made of - out of the same `.vex` `--track` reads. Each of the
five collision classes gets its own colour:

| Class | Colour |
| --- | --- |
| `Wall` | red |
| `Floor` | blue |
| `Reset` | magenta |
| `Mag Floor` | green |
| `Cage` | grey, and **not drawn unless `--cage` is given** |

**`Cage` is excluded by default on purpose, and the view is not buggy for
leaving it out.** The original parses cage nodes and branches straight past them,
so they are not collidable; `oag_gameplay::collision_world` drops them for the
same reason. The default picture is therefore what the physics world contains,
not what the file holds. `--cage` shows the rest.

### It draws outlines, not surfaces

The default is a **wireframe**: each triangle is drawn as an outline, so
geometry behind a wall stays visible. That is not a style choice. The mesh
pipeline writes depth, compares `Less` and forces alpha to 1.0, so a solid render
of a closed wall is a picture of the outside of a box and tells you nothing about
what is inside it. The whole point of this view is the opposite question - *do
the walls enclose the driveable ribbon?* - so it has to be see-through.

The outlines are built by insetting each triangle towards its own centroid and
filling the ring between the two. That needs no `POLYGON_MODE_LINE` device
feature, so it works on every backend including the headless capture path. Two
consequences worth knowing: interior edges are drawn twice, once from each
adjacent face, and the line width is one value for the whole model, taken from
the mean edge length rather than the bounding radius. Scaling it to the bounding
radius was the first attempt and produced sub-pixel dotted lines on anything long
and thin, which a track is.

`--solid` fills the triangles instead. Useful for a single surface, useless for a
closed one. Solid faces are shaded **two-sided, on the CPU**, not through the
light rig, because collision winding does not reliably face outwards: through the
rig, a back-facing triangle comes out black and half a track's collision would
silently vanish into the background.

### `--with-spline` is the actual check

```sh
oag-view <archive> --collision 'Data\Environments\01_Track\track.vex' \
    --with-spline --screenshot /tmp/enclosure.png
```

Overlays the `--track` ribbon on the collision outlines, from the same file. The
picture is the real check - the ribbon should run down the middle of the
corridor the walls make - and the printed line is the part a script can read.

That printed line is a **sanity check, not a proof of enclosure**, and it is
worded that way on purpose. It compares the ribbon's bounding box against the box
of all *collidable* geometry, so on a looping track both boxes are the whole
envelope: it catches a spline that lands somewhere else or at a different scale,
and nothing finer. Two deliberate choices behind it:

- **All collidable geometry, not walls alone.** A wall-only box is only as tall
  as the walls.
- **Loose on y.** The ribbon raises the racing line and corridor strips by
  `track::HOVER_LIFT`, so an exact vertical comparison would report a failure on
  correct data.

If the `.vex` carries collision but no `WO Track` node, the overlay is skipped
with a warning and the collision view still draws.

### What it prints

Per class: node, mesh and triangle counts, and the extent on each axis. The
extents are the interesting part, and they are what produced
[the survey of all 16 tracks](../formats/collision.md#resolved-the-broadphase-clamps-world-space-rather-than-rebasing-it):
the sweep-and-prune reading packs a 2,048-unit window, and this tool measures the
shipped geometry against it. It prints two numbers per file, and flags each
against its own bound:

- **reach** - the furthest any collidable vertex sits from the origin on any axis,
  flagged when it exceeds ±1024. Seven of the sixteen environments trip this.
- **span** - the longest axis of the collidable bounding box, flagged when it
  exceeds 2,048. **Nothing on either disc trips this**; the widest is `10_Track`
  at 2026.6.

Because the numbers are per file rather than a whole-disc maximum, that
distinction is visible at all - which is what turned a flat contradiction into a
question about where the packing's origin comes from.

That output is a finding, not a rendering bug. A reach past ±1024 says the packed
input cannot be raw world space; nothing about the picture is affected either way.

### Not collision response

This view reads geometry and surface class only. It does not read restitution and
does not simulate anything; `oag-render` cannot, by
[the dependency rules](../architecture/workspace-layout.md). It also does **not**
go through `oag-gameplay`: it decodes with `oag_formats::collision` directly, so
the viewer does not pull a gameplay crate in to draw a triangle. The one rule it
copies from `collision_world` is the cage exclusion above.

## Limitations

- **Track lighting is the viewer's, not the game's.** Track batches carry vertex
  colours and normals both, and the viewer treats the colours as prelit rather
  than lighting them twice. Which one the GE actually uses is unrecovered state.
- **Batch list B is not drawn**, so a second pass (reflections, decals) may be
  missing.
- Zlib-compressed WAD entries are skipped; no shipped archive uses them.
- One texture at a time; there is no atlas or contact-sheet view.

## Requirements

A GPU with a Vulkan, Metal or DX12 driver. On Linux, `vulkan-radeon` or
equivalent. The screenshot mode needs a device but not a display.
