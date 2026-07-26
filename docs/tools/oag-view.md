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

Left and right arrows change asset, Escape quits. The disc image is opened
read-only and nothing is written.

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

The lighting is not Pulse's. A single light leaves faces pointing away from it
unreadably black, and the point here is to see the geometry and catch decoding
errors, not to reproduce the game's look.

**Back-face culling is deliberately off.** Triangle-strip winding is
reconstructed rather than read from the file, so culling would turn a winding
mistake into invisible geometry instead of a visible artefact.

The camera frames the model from its own bounding sphere, so any model fills the
view whatever scale is baked into the file. That means **a wrong scale would
still look right here** — this is not a check on the scale factor. The
bounding-box assertion in `oag-formats` is.

## Limitations

- **Models are screenshot-only** for now; the window shows textures. A model
  needs an orbit camera to be worth putting in a window.
- **Untextured.** Materials carry a texture index into the model's embedded
  texture array; wiring that up is the next step.
- Zlib-compressed WAD entries are skipped; no shipped archive uses them.
- One texture at a time; there is no atlas or contact-sheet view.

## Requirements

A GPU with a Vulkan, Metal or DX12 driver. On Linux, `vulkan-radeon` or
equivalent. The screenshot mode needs a device but not a display.
