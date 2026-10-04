# 2048's scenery moves and the glow-layer scrolls play; the Zone circuit loads behind an off-by-default option and the start-grid clip is unwired

2026-09-16. Started from "nothing on 2048 is animated at all" and found the
reason first: `track.vex` authors **no `Anim Transform`** (1,834 `Transform`s,
7 `Mesh`es, zero of class `0x3c0` on `altima`), so the two mechanisms the
other titles animate scenery with had nothing to reach. What moves a 2048
circuit is three files that were all sitting beside `track.rcsmodel` and
one table inside it - a node table in the model's own header, a
`.rcsskeleton` with the hierarchy and bind pose, and a `.rcsanimclip` with
5 Hz absolute keys. All three are read (`oag_rcs::rcsmodel::psp2::nodes`,
`oag_rcs::rcsskeleton`, `oag_rcs::rcsanimclip`, evaluated by
`oag_rcs::rig`), planned onto the shader's node table by
`oag_render::mesh::rcs::psp2::placement`, and played through the same table
Pulse's and HD's `Anim Transform`s already ride - `oag_render::mesh::Motion`
is the two-variant enum that lets one table carry both. Full format, every
confidence and every number in
[2048-animation.md](../../docs/formats/2048-animation.md).

**The oracle is Wipeout HD, and it is what made this a reading rather than a
plausible decode.** 2048 re-ships twelve HD circuits with the same node
names; HD's `Anim Transform` evaluator is validated; so every claim about
the rig - vertices in node space, keys as HD's animation resampled, Maya's
pivot composition, `local * parent_world` with a root's own static parent
matrix, linear/nlerp interpolation wrapping to key 0 - was checked node by
node against HD's world matrices at up to fourteen times
(`crates/render/tests/psp2_scenery_animation_ground_truth.rs`, one test per
circuit, floors set by running two deliberate breaks through the suite and
recorded in the test). The pivot was the finding that would not have come
from `altima` alone: the bare `S * R * T` puts a wind-turbine rotor 2,500
units from its tower and 63 of Anulpha Pass's 86 shared nodes on HD's
matrix, the pivot form puts the rotor on its hub and 76 of 86 on HD's, and
the pivot *translate* brings the seven trains from 520 units off to 4. And
one node on Metropia said the scale is *not* about that pivot, only the
rotation - a correction the first reading would have shipped without the
mutation pass that surfaced it.

**Reading the node table also fixed a placement defect that shipped**, the
same shape as Pulse's `Anim Transform` one: 130 of `altima`'s 1,153 meshes
are bound to a node and authored in its space, and drew as world coordinates
a median 1,086 units (max 5,765) from their place. Every craft binds all of
its meshes too; the two airbrakes are the ones with a non-identity node, and
both drew under the cockpit until now.

**Two traps worth carrying.** (1) The model's bind matrices are composed
*without* the pivots (11,775 of 11,775 written ones, exact), so they are not
where a node is drawn when a skeleton is present - they match HD's time-zero
world on only 31 of 86 nodes - and only place a model that has no skeleton
(a craft). (2) The header's `+0x12` is how many of those matrices the
exporter wrote; a `trackZone` writes 103 of 966 and the rest are zeros,
which read as "collapse to the origin" until the count was found.

**What the corpus test measures** (`crates/rcs/tests/psp2_animation_ground_truth.rs`,
`just test-data`): 77 skeleton-bearing models across the three EU packages,
19,509 nodes, 6,285 tracks, 10,160 channels, 2,776,975 keys, every channel's
key count its duration over its spacing, every clip id a skeleton node,
every submesh record reachable from exactly one mesh object.

## 2026-10-05: Zone, the scroll uniforms and the start-grid clip

- **A Zone race can draw `trackZone.rcsmodel`** with its own skeleton, 30 Hz clip
  and PVS, **behind `race::Options::zone_model`, off by default since the merge
  (2026-10-05, lead)**: with `fc01_dummy` undrawn a Zone race showed no road,
  only sky and saturated colour, so Zone races the ordinary model until that
  shader is read (`race::load::geometry::sibling_model`; `crates/game/tests/vita_2048_zone_scenery_ground_truth.rs`).
- **The material instance table is solved** - `0x18`-byte entries, `~crc32`
  names, a float pool and a half pool - and read with coverage (15,561 of
  15,565 uniform hashes are names the material's own shader declares, 52,637
  sampler entries match the `.gxt` strings). See
  [2048-material-params.md](../../docs/formats/2048-material-params.md).
  It reads a Zone circuit's eight colours (`Zone_Colour1..8`) and the scroll
  uniforms: HD's glow layer plays for `Emissive_UV_Offset`/`Scale` materials
  and a plain V scroll for `speed_multipliaer` ones.
- **The start-grid clip is a pit-bot rig, not a camera**, and is **not wired**.
- **The 51 moving lit meshes under a non-uniform scale** are now shaded off the
  inverse transpose (`mesh.wgsl`; a pixel test against the baked path, 8 in a
  channel before, 0 after).

## Open

- **The Zone circuit's picture has not been judged against the original.** No
  Vita3K capture of a Zone race exists in this tree and none was taken. What is
  drawn: the eight `Zone_ColourN` colours unlit (**chosen, not measured**: lit
  blows every surface out to white), `fc01_dummy` - the road and walls, 1,716 of
  6,349 submeshes - **not drawn** because its output is unread bytecode, and
  2,302 `zonefc07_cube_animation_1` nodes hidden. The race has no visible road
  in Zone mode here. A capture of 2048's Zone mode from the original settles
  every one of those choices at once; so would decoding `fc01_dummy`'s
  3-instruction fragment program.
- **2048's Zone look has a runtime term nothing authors**: every material
  declares `zoneGrowingPaletteScene`/`Track`, `zoneGrowingTexture` samplers and
  `zoneGrowingTextureFactors`/`zoneShipPos` uniforms. Not read.
- **Scroll rates are chosen, not measured.** `TimeScaler` (else the authored
  `time`, else 1.0) is the glow layer's rate, and the plain scroll runs `+v`;
  no bytecode was read for either, and the sign is a guess. Materials that name
  `time` and nothing else (about 60, `mageffect08`, `fc08_effects_crowd`,
  `scanlinebillboard`...), `TimeScaler` alone (`fc02_effects_uscroll_*`) and the
  `frameRate` flipbook are drawn still.
- **`trackpart_startgridanims(_sp)`**: 116 and 45 submeshes of
  `startanim_fc07_lambert_emmisive` (`pitbot_col.gxt`) under a root scaled 100
  times, 110 units ahead of altima's Start Position, 38.3 s and 16.6 s clips,
  slot 7 a scalar on the root. What triggers it is not read and drawing it
  unconditionally would double the grid furniture. Its picture has not been seen.
- **Visibility is at 75**, read off which nodes a clip later shows and which a
  Zone skeleton hides, not off the executable.
- **The node id hash function** is unidentified (ten functions tried); the
  mesh object's `+0x04` word and `+0x0a` flags, property slots 6 and 8 and the
  tag word's high half, the clip header's `+0x0c`, and the skeleton's `+0x18`
  matrix for a non-root are unread.
- **No function in `vita-2048-eu-v104` was named this pass**; the skeleton's
  and clip's loaders have not been looked for.
- **HD's `02_track` is Metropia and `03_track` is Moa Therma** (35 shared
  node names with Metropia and none with Moa Therma for `02_track`, 67 and
  none the other way); `oag_hd::ENVIRONMENTS` lists them by directory only.

## Next Steps

- A muted Vita3K capture of an `altima` Zone race (see
  `docs/reverse-engineering/vita3k-capture.md`) to judge the Zone circuit;
  then decide the road and the lit-or-unlit colours from it.
- Decode the 3-instruction `fc01_dummy` fragment program and one
  `fc01_Effects_VScroll_Emissive` vertex program: the road and the scroll sign.
- Find the start-grid animation's trigger in the race-intro code in Ghidra
  (`/vita-2048-eu-v104`), starting from the `trackpart_startgridanims` string.
- Find the skeleton and clip loaders off the `"PSP2/Psp2.Rcs*Loader.cpp"`
  allocator-tag strings `RcsModel_Load` was named from.

## From the HANDOVER.md index (moved 2026-09-25)

2048's `track.vex` authors no `Anim Transform`; what moves its scenery is a node table in `track.rcsmodel` plus the `.rcsskeleton`/`.rcsanimclip` beside it, played through the same node-matrix table the other titles use (`mesh::Motion`) and checked node by node against Wipeout HD on the twelve shared circuits. 2026-10-05: a Zone race draws `trackZone.rcsmodel`, the material uniform table is solved (names, colours, scroll rates) and the glow-layer and `speed_multipliaer` scrolls play, moving lit meshes under a non-uniform scale shade off the inverse transpose. Open: the Zone picture is unjudged (no capture; the road shader `fc01_dummy` is not drawn), scroll rates and the plain scroll's sign are chosen, the start-grid clip is a pit-bot rig with no trigger. See [2048-animation.md](../../docs/formats/2048-animation.md)
