---
status: measure
titles: [2048]
updated: 2026-11-19
summary: "2048 Zone stage colours draw (altima-z5 112.94 to 48.50), DLC/Omega Vineta K road on the Track group; open: fog binding, Omega stage, start grid"
---

# 2048's scenery moves and the glow-layer scrolls play; the Zone circuit draws with the stage colours and the start-grid clip is unwired

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
`oag_mesh::mesh::rcs::psp2::placement`, and played through the same table
Pulse's and HD's `Anim Transform`s already ride - `oag_mesh::mesh::Motion`
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
(`crates/mesh/tests/psp2_scenery_animation_ground_truth.rs`, one test per
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
  shader is read (`race::load::geometry::sibling_model`; `crates/raceplay/tests/vita_2048_zone_scenery_ground_truth.rs`).
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

- **`fc01_dummy` is not flat fog (2026-10-27, [gxp.md](../../docs/formats/gxp.md#the-instruction-stream)).** The instruction listing reads its fragment program as `fogColour * (1 - f)`, `f` the vertex program's exponential-squared fog visibility (1 near, 0 far): black near, `fogColour` far, not `fogColour` everywhere. The `psp2::FOG_FLAT` draw (`oag_mesh`, `oag_raceplay::load`) was written from the 2026-10-19 reading and has not been re-read against it. Confidence 85 on the arithmetic, 60 on `f` meaning visibility; the constants (`sa32.w`, `sa36.x`, `sa36.z`) come from the material's uniform image, unread. Omega: not checkable until its GCN program is read.
- **2026-11-19, `zone-road-2048-omega`: the Zone surface is read and the stage colours draw**
  ([2048-material-params.md](../../docs/formats/2048-material-params.md), "The Zone surface").
  `Zone_ColourN` surfaces are `Colour * (ambient + sun * sat(N.L) + Emissive)` with the stage's
  `Colour k` and the title-wide Zone light; `2048-vita/zone-altima-z5` 112.94 -> 48.50, hills and
  tower box 62.3 -> 46.4. The pair's camera is cold (original camera not read), so the road boxes
  stay camera-bound: left box orig 87 / ours 14, right 40 / 22. Still open from it:
  - **`fc01_dummy`'s `fogColour`**: which stage fog (`Fog.Environment Fog Colour` or
    `Fog.Track Fog Colour`, density in the fourth lane) the engine binds is unread; ours keeps the
    authored circuit fog, fixed at load. The original's far road strip `(214, 245, 18)` equals the
    stage's `Track Paint.Primary Colour` (0.83 1.0 0.16), consumer unread (probably the road's paint
    lines, in a material variant not yet listed).
  - **The stage change is a step**, not 2048's own transition (unread).
  - **`2048-vita/zone-dlc-vineta-k-z5`** (86.04 -> 62.92) and **`omega/zone-vineta-k-z5`** (74.25 ->
    72.92): the road takes HD's Track group by shape name (chosen, not measured; the container's
    per-chunk bit is unread, `+0x0a` flags do not carry it). Omega's hue is HD's Flash purple where
    the original is olive: Omega's zone-to-stage ladder is unread (stage 9 is the closest of 1 to 12).
  - **The 2048 `zoneGrowing*` fragment variants** (`Weapon_Pads` #59/#61, `zonefc06_..._07` #37/#64:
    palette lookups of `zoneGrowingTexture`) belong to the stage table's `Growing Texture` accent;
    their palette textures' source is unread.
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

- A muted Vita3K capture of an `altima` Zone race **with the camera read** (the adopted pair's is cold)
  to judge the road and the walls by region; then the `fogColour` binding and `Track Paint` consumer.
- Read Omega's Zone stage writer (ladder) so its Vineta K Zone pair picks the right table stage.
- `fc01_dummy` is read (flat `fogColour`). Still open: one
  `fc01_Effects_VScroll_Emissive` vertex program for the scroll sign, and a
  Vita3K capture of 2048's own Altima Zone (campaign progress needed) to confirm the fog colour is what the engine binds. `BloomFactor`/`BloomGate`/`Exposure*` authored in every `track.EnvSettings` stay unread: their consumer is the composite (`wo_composite_zone_fp`, USSE), whose arithmetic is unread.
- Find the start-grid animation's trigger in the race-intro code in Ghidra
  (`/vita-2048-eu-v104`), starting from the `trackpart_startgridanims` string.
- Find the skeleton and clip loaders off the `"PSP2/Psp2.Rcs*Loader.cpp"`
  allocator-tag strings `RcsModel_Load` was named from.

## From the HANDOVER.md index (moved 2026-09-25)

2048's `track.vex` authors no `Anim Transform`; what moves its scenery is a node table in `track.rcsmodel` plus the `.rcsskeleton`/`.rcsanimclip` beside it, played through the same node-matrix table the other titles use (`mesh::Motion`) and checked node by node against Wipeout HD on the twelve shared circuits. 2026-10-05: a Zone race draws `trackZone.rcsmodel`, the material uniform table is solved (names, colours, scroll rates) and the glow-layer and `speed_multipliaer` scrolls play, moving lit meshes under a non-uniform scale shade off the inverse transpose. Open: the Zone picture is unjudged (no capture; the road shader `fc01_dummy` is not drawn), scroll rates and the plain scroll's sign are chosen, the start-grid clip is a pit-bot rig with no trigger. See [2048-animation.md](../../docs/formats/2048-animation.md)
