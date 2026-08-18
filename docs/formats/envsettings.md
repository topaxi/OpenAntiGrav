# `.envsettings`: the light rig a Wipeout HD circuit authors

**33 files, plain UTF-8 text, `key=value` per line.** One per circuit and one
per reversed circuit, plus a front-end file that is a different thing. They
state the sun, the ambient, the fog and the tonemapper for the circuit they sit
beside.

Implemented in
[`oag_formats::envsettings`](../../crates/formats/src/envsettings.rs), read into
a race by [`oag_game::race::load`](../../crates/game/src/race/load.rs), drawn
through [`oag_render::mesh_render::Light`](../../crates/render/src/mesh_render.rs),
and checked against the disc by
[`envsettings_ground_truth.rs`](../../crates/formats/tests/envsettings_ground_truth.rs).

```sh
just psarc cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    /data/environments/talons_junction/track.envsettings
just play hd --race --screenshot /tmp/hd.png --ticks 2
```

## Why it matters more than its size suggests

**The renderer's light rig was a stand-in, and this is what it stood in for.**
`crates/render/src/mesh.wgsl` said so in its own header: a fixed two-light rig
with invented directions - `(0.4, 0.8, 0.5)` and `(-0.5, 0.2, -0.7)`, weighted
`0.15 + 0.75 * key + 0.25 * fill` - chosen so that geometry reads clearly rather
than to reproduce the game's look. [`CLAUDE.md`](../../CLAUDE.md)'s rule about
not inventing what the assets already author applies directly, and on HD the
data is not even hard to find: it is plain text sitting beside the track file.

## The format

```text
"Lighting.Constant ambient color"=0.403922 0.392157 0.509804
"Lighting.Sky colour"=128 128 128 0
"Lighting.Sun color"=2.000000 1.827451 0.886275
"Lighting.Sun direction"=-0.777600 0.581520 -0.239110
"Fog.Fog Density"=0.001000
```

No sections, no escaping, no comments, no blank-line grouping. The key is
double-quoted and dotted; the dotted prefix (`Lighting`, `Fog`, `Water`,
`HDR and Bloom`) is part of the key rather than a section header, because there
is no section syntax and no leaf name repeats under two prefixes. The value is
one to four whitespace-separated numbers.

**Confidence 90** on the syntax and the key set: every line of all 33 files
parses, and the 32 circuit files share **34 keys**. Nothing here reads HD's
executable, so *what the original does with any value* is not decoded - the
names below are the file's own, not recovered ones.

### The 34 keys every circuit carries

| Group | Keys |
| --- | --- |
| `Lighting` | `Sun direction`, `Sun color`, `Sun specular scale`, `Constant ambient color`, `Sky colour`, `Sky rotation`, `Spotlight colour scale`, `Spotlight specular scale`, `Ambient false direction`, `Prelit ambient colour scale`, `Prelit ambient colour power`, `Prelit ambient false specular power`, `Prelit ambient false specular intensity`, `Enable dynamic lights` |
| `Fog` | `Fog Color`, `Fog Density`, `Alternate Fog Color`, `Alternate Fog Density` |
| `Water` | `Water distortion`, `Water plane height`, `Refract distortion` |
| `HDR and Bloom` | `Bloom adaption rate`, `Bloom adaption boost`, `Bloom from alpha contribution`, `Bloom from frame contribution`, `Bloom from frame exponent`, `Bloom horizontal size`, `Bloom vertical size`, `Tone adaption boost`, `Radial bloom Enabled`, `Radial Bloom angle`, `Radial bloom distance scale`, `Radial bloom rgb contribution`, `Radial bloom rgb exponent` |

Four more appear on some and not all, and are listed by the ground truth rather
than assumed away: `Tone darkening clamp` and `Tone maximum brightness` on 12
circuits, `Physical Sun direction` and `Use Lens Flare` on 29, and
`Enable spu vertex lights` on 4.

## Two number encodings, and only the formatting tells them apart

**A parser that reads every value as a `0..=1` float gets one key wrong by a
factor of 255.**

```text
"Lighting.Constant ambient color"=0.403922 0.392157 0.509804   <- normalised
"Lighting.Sky colour"=128 128 128 0                            <- bytes
```

Measured over all 32 circuit files, five keys are written with no decimal point.
Four of them are `0`/`1` flags - `Radial bloom Enabled`, `Enable dynamic
lights`, `Enable spu vertex lights`, `Use Lens Flare` - and the fifth is
`Sky colour`, whose range reaches 255. So the integer formatting *is* the type
tag, and `Value::is_integer` records it; `EnvSettings::rgba8` is a separate
accessor from `EnvSettings::vec3` for that reason, and each refuses the other's
shape.

## The sun direction is not a unit vector

Across the 32 circuits the length of `Lighting.Sun direction` takes **eight
distinct values**: 0.000, 1.000, 1.208, 1.784, 2.375, 4.743, 7.348 and 34.641.
Most circuits are unit and several are not, and **four are degenerate** -
`modesto_heights` writes `-0.000030 0.000040 -0.000060`, which is an off switch
rather than a direction.

So a reader that assumes a unit vector is wrong on most of the corpus and one
that uses the raw triple as a direction scales its light by an accident.
`EnvSettings::direction` normalises and answers `None` below a length of `1e-3`,
which is what keeps a degenerate triple out of a shader as a NaN. **Whether the
length carries an intensity is not established**; that needs HD's executable.

## It is authored for a linear HDR pipeline, and this project's is not

This is the constraint that decides how much of the file can be used today.

| Key | Brightest across the corpus |
| --- | ---: |
| `Lighting.Sun color` | **4.000** |
| `Lighting.Constant ambient color` | **3.000** |
| `Fog.Fog Color` | **1.852** |
| `HDR and Bloom.Tone maximum brightness` | 4.000 |

Values above 1.0 are only meaningful to a renderer that works in linear light
and tonemaps. This one is gamma-authoritative by
[ADR-0020](../architecture/adr/0020-gamma-authoritative-colour-space.md) and has
no tonemap stage, so a sun colour of 4.0 multiplied into an `Rgba8Unorm` target
clips everything past a quarter brightness to white.

**What is drawn, therefore, is the direction and the hue and not the
magnitude.** `mesh_render::Light::authored` divides the sun colour by its own
largest component - `(2.0, 1.83, 0.89)` becomes `(1.0, 0.91, 0.44)`, the same
warmth inside the range the target holds - and clamps the ambient. That
reduction is **this project's, not the disc's**, and the load report says so per
race:

```text
/data/environments/talons_junction/track.envsettings: sun [-0.78, 0.58, -0.24]
hue [1.00, 0.91, 0.44] over ambient [0.40, 0.39, 0.51] - the authored magnitude
(2.00) is dropped, this target having no headroom for it
```

**Measured, so that the reduction is a decision and not a hope**: on Talon's
Junction at the starting grid the share of pixels at full white goes from
**0.41 % to 1.13 %** and mean luminance from 22.7 % to 31.5 %. Applying the
authored magnitude raw instead of its hue is what the measurement exists to
stop.

That the file authors magnitudes above 1.0 at all is **evidence that
gamma-authoritative is the wrong model for HD specifically** - HD is an RSX
title that renders linear and tonemaps, where Pulse's GE blends stored bytes.
Recorded here rather than acted on: ADR-0020 is immutable, and superseding it
needs a reference frame from the running original, which nothing in this project
has.

## What is read and what is not

| Key | State |
| --- | --- |
| `Lighting.Sun direction` | **Drawn.** Normalised; the stand-in rig is used where it is degenerate |
| `Lighting.Constant ambient color` | **Drawn**, clamped to `0..=1` |
| `Lighting.Sun color` | **Hue drawn, magnitude dropped** - see above |
| `Lighting.Sky colour` | Read, unused |
| `Lighting.Sky rotation` | **Drawn**: the sky cubemap is turned by it, about the vertical. Degrees is the corpus's own unit - 180 and -40 survive no radian reading - and the sign and axis are this project's choice, said per race in the load report |
| `Fog.Fog Color`, `Fog Density` | **Drawn, on the curve read out of the circuit materials' own fragment microcode**: `f = exp(-(density * view_depth)^2)`, the colour lerped in by `f`. See [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-race-fog-curve-is-read-out-of-the-circuit-materials-own-microcode). What is *not* read is how the engine fills the shader's coefficient from `Fog Density`; the authored value is passed through unscaled and judged against an rpcs3 reference frame |
| `Fog.Alternate *` | Read, unused - what selects the alternate pair over the primary is unread |
| `HDR and Bloom.*` | Read, unused. The bloom here is not HD's, and the tone parameters need a tonemap stage that does not exist |
| `Water.*` | Read, unused. No water surface is drawn |
| `Lighting.Prelit *`, `Spotlight *`, `Ambient false direction` | Read, unused, and undecoded - what "prelit ambient false specular" means is not established |

**The asset viewer is deliberately not lit by this**, the same way it is
deliberately unfogged: `just view --mesh` and `--track` bind `Scene::off` and
draw through the stand-in rig, because the viewer's job is to show what is on
the disc clearly rather than as the circuit stages it. The authored rig reaches
a race and nothing else.

## The one file that is not a circuit's

`/data/fe/fury.envsettings`, 436 lines, with `Feedback`, `Equaliser` and
`Music Pulse` keys for the Fury front end. It parses through the same reader and
shares almost nothing else; the ground truth names it rather than filtering by
line count, so that a *second* non-circuit file shows up as a failure instead of
being quietly absorbed.

## Where the executable touches this, located and not read

Recovered 2026-08-18 with Ghidra open on `EBOOT.elf` and
[`scripts/ps3-toc.py`](../../scripts/ps3-toc.py), which resolves each function's
own TOC rather than the one Ghidra picks for all of them - see
[`memory.md`](../ghidra/functions/ps3-hdfury-eu/memory.md), because 59 % of
TOC-relative loads otherwise land on a real string that is not the one the code
loads.

| Address | What names it |
| --- | --- |
| `0x003a83d8`, `0x003a9520` | `"Lighting.Sky colour"` and `"Lighting.Sky rotation"` |
| `0x003f3fb0` | `"sky.gtf"`, `"skycube"` and `"Data/Tex/ZoneSky.gtf"`, all three |

**Nothing is renamed on the strength of that**, per
[`CLAUDE.md`](../../CLAUDE.md)'s rule: naming a function needs more than knowing
which strings it mentions, and below 50 confidence the hypothesis is written down
instead of dressed up as a name. `0x003f3fb0` is a large per-race setup function
that also builds pads and cameras; which part of it loads a sky is unread.

Two things the string sweep turned up that this page's key list does not have:

- **`"Lighting.Debug.Draw sky"`** - a debug toggle, so the sky is drawn through
  something switchable and separately named.
- **`"%s.Lighting.Sky reflection colour"`, `"%s.Sky horizon colour"` and
  `"%s.Sky zenith colour"`** - `%s`-prefixed, so there is a **second, prefixed
  key namespace** these settings are read through, and it is not the flat one the
  33 files on the disc write. What supplies the prefix is unread.

### The prefixed namespace covers fog too, and one lookalike is not fog at all

Swept 2026-08-18. `"%s.Lighting.Fog colour"`, `"%s.Lighting.Fog density"`,
`"%s.Lighting.Alt Fog colour"`, `"%s.Lighting.Alt Fog density"`,
`"%s.Lighting.Track Fog colour"` and `"%s.Lighting.Track Fog density"` sit at
`0x007b24d8`-`0x007b2568` and are all named by one function, `0x003d0b98`. So the
prefixed namespace is not a sky-only thing: it covers the same fog values these
files write flat, and it carries a **`Track Fog` pair no file on the disc
writes**.

**`"%s.sections[%d].fogStart"`, `"fogLength"` and `"fogExponent"`
(`0x00788cc8`-`0x00788d08`) are not this file's fog and not the race's.** They
read exactly like the fog curve anyone would go looking for, which is why they
are named here: their neighbours in the same key block are `effectMode`, `fovy`,
`duration`, `dofStart`, `dofStrength`, `dofFactor`, `focusStart` and `focusEnd`,
and the `.cpp` name immediately after them at `0x00788e58` is
`BackgroundController_Item.cpp`. They belong to the **front end's background
camera flythrough**, a per-section rig with depth of field, and have nothing to
do with a circuit.

## Open

- **What the original does with any of it.** Every name here is the file's own.
  No handler has been read, and nothing has been checked against a running
  frame - the ceiling the [confidence rubric](../reverse-engineering/confidence-rubric.md)
  puts on a static reading.
- **Whether the sun direction's length is an intensity.** Eight lengths across
  32 circuits; normalising is this project's choice.
- **The sky is drawn now, and `Sky colour` still is not understood.** HD's
  circuits author **no `Skycube` node** - class `0x3c6` is in the class table
  and zero nodes carry it - and the sky is `sky.gtf` beside the track, a
  six-face **cubemap** [`gtf::Texture::face_to_rgba`](gtf.md) decodes.
  `oag_render::mesh::sky_cube` builds it into a camera-centred cube whose quads
  sample each face exactly where the RSX's own cubemap addressing would, drawn
  through the same `Depth::Sky` path as Pulse's authored `Skycube` mesh; the
  picture is the disc's and the cube is the geometry a cubemap defines for
  itself. `Sky rotation` turns it (see the table above). What `Sky colour`
  does - `128 128 128 0` on most circuits - is still unread and unused.
- **The fog curve is read**, out of the circuit materials' own fragment
  microcode rather than the executable's `fogFactors` blocks (which turned out
  to be the front end's): `f = exp(-(coefficient * view_depth)^2)`, the
  coefficient and the fog colour patched together into one `float4` the shader
  interface itself names `fogColour` - a `~crc32` preimage. The census and the
  instruction listing are in
  [`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-race-fog-curve-is-read-out-of-the-circuit-materials-own-microcode).
  **Still open**: whether `fogColour.w` is `Fog.Fog Density` unscaled - the
  race passes it through unscaled and says so - and what selects the
  `Alternate` pair.
- **`Alternate Fog Color` and `Alternate Fog Density`**, on every circuit, with
  nothing read about what selects them.

## See also

- [hd-status](hd-status.md) - the format layer across the whole disc
- [gtf](gtf.md) - the textures, including the sky cubemap that is refused
- [rcsmodel](rcsmodel.md) - the geometry this lights
- [skycube](skycube.md) - how Pulse authors a sky, which HD does not
- [ADR-0020](../architecture/adr/0020-gamma-authoritative-colour-space.md) - the
  colour-space decision this file argues against for HD
