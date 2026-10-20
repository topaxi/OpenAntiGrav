# The PS2 hull's lights: the same point light, a rig of its own, and our hull ignoring all of it

**Binary:** `SCES_547.48` (Pulse PS2, EU). **Status:** read live on PCSX2 from a saved
state and a GS dump; **no function named** (the writer of the block below was not found,
see "Open"). Every address below is instance data of the saved state, not a structural
fact.

## Summary

- **The PS2 lights its hulls**, and the light list holds the circuit's nearest
  `wopoint` exactly as the PSP's does: the same unit vector, the same colour bytes.
  Confidence **85** that the PS2 takes `wopoint` lights by the PSP's law (one pose,
  vector to five digits, colour to the byte).
- **Our PS2 hull takes none of it, and never did.** `mesh.rs` sets `lit = 0.0` for a
  vertex that carries a colour, every PS2 hull vertex carries one (`253`, the PS2's
  `128` rescaled), and the hull rig's branch in `mesh.wesl` needs `lit > 0.5`. The
  rig is inert on the PS2 - ambient, directionals and points alike - although the
  `hull_lights` title rule lists the PS2. Rendering the same pose with and without
  `HullPointLights` is pixel-identical (0 pixels differ). So "the PS2 shares the rule"
  in `psp-pulse-usa/scene-light.md` was true of the title rule and false of the picture.
- **The PS2's own composition is not the PSP's** and was not recovered: its hull vertex
  colours run `63..127` on the GS's `0..128` scale (a floor of `0.5`), and a **second**
  light slot holds a light no `wopoint` or `wospot` of the circuit explains. So wiring
  the PSP's rig onto the PS2 hull would be an invention, and was not done.

## Where it was read

Pulse PS2 EU on PCSX2 2.7, the Single Race state on **Moa Therma White** (`03_Track`,
the player in grid slot 8 of 8, Feisar). `03_Track` authors 65 `wopoint`/`wospot`-class
nodes, **identically on both discs** (PSP USA and PS2 EU, the same positions and
payloads); one `wopoint` is in range of the grid craft: position
`(-394.023, 20.515, 134.583)`, colour `(0.5944, 0.7499, 0.764)`, radii `25`/`50`,
19.98 units from the craft - inside the inner radius, so strength 1.

## The block

EE RAM holds one light block per drawn craft. For the player's craft (saved state,
`0x002db8e0..0x002db960`):

| Address | Words | Reading |
| --- | --- | --- |
| `0x002db900` | `(-0.41421, -0.82621, 0.38186, 0)` | light 0 vector |
| `0x002db910` | `(-0.49060, 0.77774, -0.39297, 0)` | light 1 vector |
| `0x002db920` | `(0.99608, 0.99608, 0.99608, 1)` | colour: the sun (255 clamped, `* 254/255`) |
| `0x002db930` | `(0.0549, 0.0431, 0.0431, 1)` | colour: the second directional (`14, 11, 11`) |
| `0x002db940` | `(0.58824, 0.74510, 0.75686, 1)` | colour: the `wopoint`, bytes `(150, 190, 193)` |
| `0x002db950` | `(0.0745, 0.3804, 0.3216, 1)` | colour: unexplained, bytes `(19, 97, 82)` |
| `0x002db960` | `(1, 1, 1, 0.5)` | a half; the floor of the hull colours below |

**Light 0 is the `wopoint`.** The vector from the craft to the node is
`(-8.4062, 16.5112, 7.4881) / 19.984 = (-0.42065, 0.82625, 0.37471)` in the world.
Expressed in the craft's own axes (right `(-0.99985, 0, -0.01706)`, up `(0, 1, 0)`,
forward `(0.01706, 0, -0.99985)`, read from the same state) it is `(0.41420, 0.82625,
-0.38186)`; the block holds the **negation**, `(-0.41421, -0.82621, 0.38186)`, to four
digits. So the PS2 builds the same normalised vector as the PSP's builder (which the PSP
writes unnormalised and the GE normalises) and stores it in **craft space**.

**The colour is the PSP's byte law.** `trunc(c * 255)` is `(151, 191, 194)` and
`* 255 >> 8` is `(150, 190, 193)` - the block's bytes. `255` is the default scale of a
craft that has not been placed on a spline yet.

**The second vector and colour are not a `wopoint`.** A search of all 65 nodes for a unit
vector (either sign) equal to `(-0.4906, 0.7777, -0.3930)` in the craft frame finds none
closer than 0.52, and `03_Track` authors no `wospot` at all. What this light is is open.

## The hull's vertex colours: a GS dump

A single-frame GS dump of the same state (`scripts/pcsx2-drive.py gsdump`, read with
`scripts/pcsx2-gsdump.py`) shows the player's hull as state groups of 323 to 897
triangles. Their vertex `RGBAQ` is **not constant**: 116 distinct colours in the largest
group, `63..127` per channel, and not grey - the red channel falls short of green and
blue by up to 14 where the others saturate at `127`, a **cyan** cast, which is the
`wopoint`'s hue (the circuit's sun is warm and would push red up). The PS2 has no
per-pixel light; these are the lit colours the VU1 program hands the GS.

A model's vertices carry the colour `253` (all channels) in the file, so the lit colour
is computed, not baked. Fitting the dump's colours against the model's normals was tried
and failed: the GS kick order is not the model's vertex order (the VU1 program re-chunks
strips), so the correspondence needs the draw's transform and was not built.

## Two tests that did not work, and why

- **Zeroing the light nodes in RAM** (the sun, both ambients and every copy of the
  `wopoint`'s colour, written over PINE, then 20 frames): the frame is identical outside
  the OSD toast. Not a positive control: the sun and the ambients, whose effect on the
  hull the dump proves, changed nothing either. **The lists are built at load and the
  node payloads are not read per frame** - the PSP's behaviour too
  ([scene-light.md](../psp-pulse-usa/scene-light.md): "none fire per frame").
- **Searching RAM for the light vector** found the block above only because the block's
  own vector was searched in craft space; a world-space search finds nothing.

## What ours does

`oag_raceplay::load::pulse_psp::light_hulls` builds `HullLights` and `HullPointLights`
for the PS2 too (the `hull_lights` rule lists it), and `mesh.wesl` skips both for a
coloured vertex. Nothing was changed: the PS2 composition (floor, slot count, the second
light) is unread, and drawing the PSP's would replace a flat hull with an invented one.
The probe `crates/vex/examples/ship_colour_census.rs` prints the `253` census.

## Cross-title: Pure

Pure ships the nodes and its executable ships the importers. Every circuit-sized entry of
`Data.wad` was scanned for the `wopoint` payload shape (`0x20` bytes: colour, a `1.0`,
inner and outer radius): classes `0x381` (`wopoint`) and `0x380` (`wospot`) - Pulse's
`0x3cf`/`0x3ce` less `0x4e`, the same offset as the rest of the class table
([pure-status.md](../../../formats/pure-status.md)) - in seven circuit files (148, 45,
72, 100, 153, 130 and 14 nodes), `crates/pure/examples/pure_light_nodes.rs`. The
executable's strings are `WOPoint_ImportNode`, `WOSpot_ImportNode` and
`DirectionalLight_ImportNode`. **Checked, applies (data), not wired**: Pure's hull-light
builder was not located, and no Pure hull takes any circuit light today (the `hull_lights`
rule is unset).

## Open

- The writer of the block, and the VU1 program that turns it into the hull's vertex
  colours: the colours' law (floor `0.5`, slot count, the second light) follows from it.
  `vu1MicroMem.bin` in the saved state holds the program the VU last ran.
- What the second light is (colour `(19, 97, 82)`, from below and to the side).
- Whether the block is rebuilt as the craft moves, as the PSP's `+0xac` countdown does.
