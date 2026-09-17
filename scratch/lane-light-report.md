# `lane/hd-ambient-light`: Amphiseum's ceiling hue - the `Lighting.Sky`/ambient-false-lighting/dynamic-light leads are checked and refuted

2026-09-17. Full evidence is in
[`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)'s
new section, "`Lighting.Sky colour`'s consumer is found and is a backdrop
clear, not a material term", and the matching dated entry in the rendering
handover thread named after HD's frame brightness (`handover/` and
`HANDOVER.md` may cite a thread file, per `CLAUDE.md` - nothing else may
link into `handover/`, so it is named here rather than linked). This file
is the scratch account of how the session went, for anyone who wants the
working, not just the conclusion.

## The brief

Settle whether the original consumes `Lighting.Sky colour` where this
project only consumes `Lighting.Constant ambient color`, as a candidate
explanation for Amphiseum's ceiling reading warm-gold in the reference and
blue-violet in this project's render (documented across three prior
sessions in the handover thread). Fix it if the evidence lands; otherwise
say so and change nothing.

## What was checked, in order

### 1. Struct-offset correction

`Environment_RegisterLightingSchema` (`0x003a83d8`, `EBOOT.elf`,
`ps3-hdfury-eu`) is the registrar the prior session (2026-09-17 earlier)
read. Re-decompiling it and reading the registrar calls in order:

```
puVar9 = PTR_s_Lighting_Constant_ambient_color_008b6fe8;
...
_opd_FUN_005d3ec0(iVar16, iVar16 + 0x420, puVar9, 0);                                    // Constant ambient color -> +0x420
_opd_FUN_005d40b8(iVar16, iVar16 + 0x440, PTR_s_Lighting_Sky_colour_008b6fec, 0);         // Sky colour -> +0x440
_opd_FUN_005d4418(iVar16, iVar16 + 0x444, PTR_s_Lighting_Sky_rotation_008b6ff0, 0);       // Sky rotation -> +0x444
_opd_FUN_005d3ec0(iVar16, iVar16 + 0x430, PTR_s_Lighting_Sun_color_008b6ff4, 0);          // Sun color -> +0x430
```

**`Constant ambient color` is at `+0x420`, not `+0x430`** as the prior
session's entry states - `+0x430` is `Sun color`. `Sky colour` (`+0x440`)
and `Sky rotation` (`+0x444`) were already right. Cross-checked against
seven other key/offset pairs this page already had independently (Fog
Color/Density, Alternate Fog Color/Density, the three HDR/Bloom Tone keys) -
all seven agree, so this decompile's own TOC is sound and the `+0x430` line
was the prior session's error, not a second disagreeing reading. Fixed in
renderer.md's new section.

### 2. `Lighting.Sky colour`'s consumer

Found in `Scene_PrepareFrame` (`0x003aa888`), a ~3,200-line decompile.
Pinned by tracing the local variable the read goes through, not by offset
coincidence - this page's own "Read correctly" section already warns that a
cross-TOC mismatch produces a *different, coherent, wrong* struct, not
garbage, so trusting an offset match alone would repeat that exact trap:

```c
iVar44 = EnvSettings_GetOrCreate();          // pinned: the singleton accessor, by name
...
*(float *)(puVar32 + 0xb8) = (float)((double)*(float *)(iVar44 + 0x444) * dVar64);  // Sky rotation, corroborates
...
uVar45 = *(uint *)(iVar44 + 0x440);          // Sky colour
```

`iVar44` is not reassigned between the `EnvSettings_GetOrCreate()` call and
the `+0x440` read, and the adjacent `+0x444` (`Sky rotation`) read in the
same block lands exactly where the registrar wrote it too - two offsets
agreeing, not one address matching by luck.

**What it feeds: a screen backdrop/clear-fill, gated by
`g_ZoneEffectsActive` (already-named global) and `Lighting.Debug_Draw_sky`
(`+0x591`, defaults to `1` - like the schema's other `Debug_*` keys, a
production toggle despite the name).** Zone effects inactive (the normal
race case, Amphiseum's grid pose included): the Sky colour byte is packed
into an RSX clear-colour word and issued through `Rsx_SetMethod`. Zone
effects active: a *different* code path calls `Sky_DrawGradientDome` using
entirely different fields (a zone-effects colour table), not `+0x440` at
all - Zone mode substitutes its own animated palette.

**This is a background pass, not a light term any material shader reads.**
The ceiling is drawn opaque geometry (established by the prior session's
material probe: real `animhexlights`/`cf_diff_spec`/`lambert`/
`base_diffusespecular` chunks), which would occlude any backdrop fill
underneath regardless of colour. **Refutes the brief's lead**: the original
does consume `Lighting.Sky colour`, but wiring it into `mesh.wgsl`'s ambient
term would not be recovering a real use - the disc's own code never gives
it one there.

### 3. `Constant ambient color`'s own path, checked for symmetry

Since I was already inside `Scene_PrepareFrame` tracing offsets, checked
whether `+0x420` (the corrected `Constant ambient color` offset) reaches the
shader at all, rather than assuming the prior session's "it's wired" was
right just because it's the disc's dominant term for the ceiling by
elimination. It does: under the same `g_ZoneEffectsActive == 0` branch,
`Scene_PrepareFrame` copies `+0x420` (`Constant ambient color`), `+0x450`
(`Sun direction`, normalised), `+0x4a0` (`Ambient false direction`,
normalised), `+0x4c0`/`+0x4d0` (`Prelit ambient colour scale`/`power`) and
`+0x4e0` (`Fog Color`) into a scratch buffer, then binds each into a
per-draw shader-parameter table (`iVar56 + 0x158/0x1d8/0x178/0x198.../0xf8`)
through the same `(pointer, count)` binding idiom this page's own
`fogColour` investigation already established. So the original's frame
setup really does feed the material pipeline a `Constant ambient color`
term - the same value `mesh.wgsl`'s `scene.light.ambient` already applies.
No missing-key, no wrong-key defect on the ambient front.

### 4. `Lighting.Enable ambient false lighting`

Registrar default is `0` (off). Checked on-disc for both circuits *and*
the front-end carry-forward files `staged_envsettings` (`crates/game/src/
race/load/environment.rs`) would pull from for any key a circuit's own file
omits - the same persistent-store mechanic that saved Sol 2's `Tone` triple
in an earlier session:

```
$ python3 scripts/psarc.py cat .../DATA00.PSARC /data/environments/amphiseum/track.envsettings | grep -i enable
"Lighting.Enable dynamic lights"=1

$ python3 scripts/psarc.py cat .../DATA00.PSARC /data/fe/fe.track.envsettings | grep -i enable
"Lighting.Enable dynamic lights"=1

$ python3 scripts/psarc.py cat .../DATA02.PSARC /data/fe/fe.track.envsettings | grep -i enable
"Lighting.Enable dynamic lights"=1
```

`Enable ambient false lighting` and `Enable Prelighting` appear in none of
these five files (Amphiseum's own, Talon's Junction's own, and both
front-end copies) - off by every route. The false-direction vector and
prelit scale/power values still get computed and bound to the shader table
unconditionally (no check of `+0x5a0` anywhere in `Scene_PrepareFrame`), so
this is not "the machinery is dark", only "whatever downstream code gates
its own behaviour on this bit is confirmed inactive here" - that downstream
consumer was not traced. The key names themselves ("Prelit ambient **false
specular** power/intensity") read as a fixed/fake specular-highlight system
for lightmapped surfaces, not a diffuse ambient blend - a reading of the
names, not the microcode, no confidence score.

### 5. Vex-authored dynamic lights

`Lighting.Enable dynamic lights=1` on both circuits, and the schema
registers a real SPU-driven per-vertex light subsystem
(`Debug_Draw_light_volume`, `Debug_Stall_for_spu_light_volume`,
`Enable_spu_vertex_light` all present as registered keys) that
`crates/render`/`crates/game` implement none of (`rg` over both crates for
"dynamic light"/"point light"/"spot light": zero hits). A plausible source
for a per-area colour difference a flat ambient constant cannot produce on
its own.

Checked whether either circuit's own `.vex` authors any light-rig node at
all, using a new one-off diagnostic,
[`crates/render/examples/light_census.rs`](../crates/render/examples/light_census.rs)
(`oag_vex::vex::nodes`/`class_id`, the same API
`crates/vex/tests/vex_class_ground_truth.rs` already validates against the
whole HD disc):

```
$ cargo run -p oag-render --example light_census -- data/images/hdfury-ps3-eu-dec.iso amphiseum
.../amphiseum/track.vex: 814 nodes total, 0 AmbientLight, 0 DirectionalLight, 0 PointLight
[... 8 more files, all 0/0/0 ...]

$ cargo run -p oag-render --example light_census -- data/images/hdfury-ps3-eu-dec.iso talons_junction
.../talons_junction/track.vex: 826 nodes total, 0 AmbientLight, 0 DirectionalLight, 0 PointLight
[... 8 more files, all 0/0/0 ...]
```

**Zero** `AmbientLight`/`DirectionalLight`/`PointLight` nodes (class
`0x12c`/`0x131`/`0x132`) across all 9 `.vex` files on each circuit,
including the main `track.vex`. Stated narrowly: this rules out these three
vex node classes as the dynamic-light data source on these two circuits
specifically - it does not rule out the SPU light path being fed from some
other, unenumerated source (a class this sweep did not check, or data
carried on HD's own `.rcsmodel` geometry rather than the `.vex` scene tree),
and it says nothing about any other circuit.

## What this settles and what it doesn't

**Root cause: not established.** Three candidate mechanisms tied to "what
light term does the original apply that this project doesn't" are now
checked and refuted:

1. `Lighting.Sky colour` - consumed, but by an unrelated backdrop pass.
2. `Enable ambient false lighting` - authored off, every route checked.
3. Vex-authored dynamic lights - none present on either circuit's `.vex`.

Per this project's own rule against tuning to a reference, **no shading
code was changed** - `mesh/`, `mesh.wgsl`, `emissive.rs`, `sky_cube.rs` are
unchanged; `crates/render/examples/light_census.rs` is a new diagnostic
example, not a wiring change. A negative result, reported honestly, per this
lane's own instructions on what counts as a good outcome here.

**Advisor caught two real gaps in this pass before it was reported as
done**: an unescaped-`\b` grep regex that made "`Scene_PrepareFrame` never
reads `+0x420`" look true when it was untested (the offsets in the decompile
carry a `U` suffix, `0x420U`, which a `\b`-anchored pattern cannot match),
and an unpinned `iVar44` at the `+0x440` read site that could have been a
different struct entirely, the same "different, coherent, wrong" trap this
page already documents. Both are fixed above with the correct, verified
readings; neither survived as an error in the committed docs.

## Still open

- The per-pixel `ndl` on the ceiling's own drawn chunks - still assumed,
  never measured, that `sun_diffuse` is genuinely zero there.
- A per-material microcode sweep (`scripts/ps3-microcode.py`) of the four
  ceiling materials (`animhexlights`, `cf_diff_spec`, `lambert`,
  `base_diffusespecular`) for which named engine parameters their own
  fragment programs actually declare - `fogColour`, `Ambient false
  direction` and `Constant ambient color` are all now known-bound engine
  parameters a material could reference by hash; which of the four ceiling
  materials do was not checked this session.
- `Enable ambient false lighting`'s own downstream consumer (whatever code
  checks `+0x5a0`) was not traced - only that it is off on both circuits
  checked here.
- Whether HD's SPU dynamic-light subsystem is fed from a source other than
  the three vex light classes checked here.

## Gate

`crates/render/examples/light_census.rs` is a new `.rs` file, so the full
gate was run per this lane's brief:

```
flock "$HOME/.cache/oag/gate.lock" just
flock "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data
```

Results and whether the run was watched to completion: see the report
summary this file's caller received (kept out of this file since the gate
finished after this report was drafted; update here if a discrepancy turns
up).
