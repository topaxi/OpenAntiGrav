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
| `Lighting.Sky colour`, `Sky rotation` | Read, unused. The sky itself does not draw - see below |
| `Fog.*` | Read, unused, and **an HD race has no fog at all**. HD's circuits author no `fogCube` node, so `fog_volumes` is empty on a PS3 source and `Fog::off` is what binds - the circuit states a fog colour and a density and gets nothing. Wiring it needs a second fog *mode*, not a value copy: `Fog Density` is an **exponential** coefficient and this project's uniform is a linear `near`/`far` ramp |
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

## Open

- **What the original does with any of it.** Every name here is the file's own.
  No handler has been read, and nothing has been checked against a running
  frame - the ceiling the [confidence rubric](../reverse-engineering/confidence-rubric.md)
  puts on a static reading.
- **Whether the sun direction's length is an intensity.** Eight lengths across
  32 circuits; normalising is this project's choice.
- **The sky.** `Lighting.Sky colour` and `Sky rotation` are read and nothing
  uses them, because HD's circuits author **no `Skycube` node** - class `0x3c6`
  is in the class table and zero nodes carry it. The sky is
  `sky.gtf` beside the track, a 1024x1024 DXT1 **cubemap** that
  [`gtf::Texture::to_rgba`](gtf.md) refuses; the geometry to draw it through
  would be this project's, so it is not drawn at all rather than invented.
- **Fog**, which needs the exponential mode above.

## See also

- [hd-status](hd-status.md) - the format layer across the whole disc
- [gtf](gtf.md) - the textures, including the sky cubemap that is refused
- [rcsmodel](rcsmodel.md) - the geometry this lights
- [skycube](skycube.md) - how Pulse authors a sky, which HD does not
- [ADR-0020](../architecture/adr/0020-gamma-authoritative-colour-space.md) - the
  colour-space decision this file argues against for HD
