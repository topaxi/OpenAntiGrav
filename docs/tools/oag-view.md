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

Left and right arrows change asset, Escape quits. With `--mesh` or `--track`
the window shows one model instead, and arrow keys orbit the camera; see
[Models](#models). The disc image is opened read-only and nothing is written.

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
