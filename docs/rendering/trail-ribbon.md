# The trail ribbon: what the assets author, per title

Asked 2026-08-23: can the exhaust trail be *derived from the shipped data*
rather than from constants, so it renders accurately on all four sources?

**Short answer: on Wipeout HD/Fury, yes and almost for free. On Pulse, no - the
data holds the ribbon's mount point and its noise texture and nothing else, and
every number that decides how it looks is a table in the executable.** Pure sits
between the two and its executable has not been read.

## Method

A sweep over every entry of each source's bulk archive, testing each blob for
`VEXX` magic and counting nodes of the `Trail` class, then decoding those nodes'
payloads. Reproduce it with a throwaway example against
`oag_assets::Archive`/`psarc::Archive`; nothing here needs a running game.

The class id is version-keyed and getting that wrong is the trap this sweep was
built to avoid. Version 6 is `vex::CLASS_TRAIL` = `0x3c8`. **Version 4 is
`0x37a`**, which is not a guess: it is the class-name run's own arithmetic,
`base + index("Trail") + gap`, with the base and gap structure that
`crates/pure/tests/class_table_ground_truth.rs` already pins at confidence 94
for eight other ids. It predicts nodes named `trail_con_left`/`trail_con_right`
in Pure's ship files, and that is exactly what is there - a prediction and an
asset check, so **confidence 92**. Version 3 has no recovered numbering at all
and 15 of Pure's files are skipped for it; that is a hole in the sweep, not a
negative result.

## What is on each disc

| Source | entries | `.vex` | files with a `Trail` node | nodes | payload sizes |
| --- | ---: | ---: | ---: | ---: | --- |
| Pulse PSP `Data.wad` | 1,142 | 340 (v4 33, v6 307) | 3 | 6 | all 64 |
| Pulse PS2 `WADS2.WAD` | 7,200 | 994 (all v6) | 10 | 20 | all 64 |
| Pure PSP `Data.wad` | 832 | 171 (v3 15, v4 156) | 27 | 53 | 52 x 64, 1 x 0 |
| HD/Fury, 7 PSARCs | 11,664 | 183 (all v6) | **0** | 0 | - |

Node names across all of them: `trail_con_left`, `trail_con_right`,
`trail_con_left_wing`, `trail_con_right_wing`, and on Pure one `motrail`.

## The payload is a mount, not a parameter block - confidence 92

**Every one of the 78 non-empty `Trail` payloads is exactly 64 bytes**, and 64
bytes is a 4x4 matrix. Decoded, each is an identity rotation with a translation
in row 3, the row-vector convention `vex::transform` documents - for example
Pulse's `trail_con_left` at `(2.6497, -0.8177, 0.5184)`.

There is no capacity, no layer count, no width, no colour, no scroll rate and no
texture reference anywhere in it. A `Trail` node says **where** a ribbon hangs
and nothing about what it looks like.

That is the whole negative result, and it is why `oag_render::exhaust`'s
`TRAIL_*` constants cannot be replaced by a parser. On Pulse those numbers come
from `Trail_InitPreset`'s three-preset table at `&DAT_08ad226c` in `BOOT.BIN`,
with `ExhaustFlare_Init` passing preset 2 - all of it recovered and tabulated on
[exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md), "The ship's trail
parameters, from preset 2".

The one genuinely asset-side piece on Pulse is the **texture**:
`Data\Tex\engineFlare\Engine_noise.mip`, a literal string in both executables,
so an exact `wad::hash_name` hit rather than a mined candidate.

## Who authors a mount, and the divergence nobody had looked for

Named-file checks, rather than the index sweep, are what make this readable:

- **Pulse PSP**: `Assegai\shipwreck.vex` and `Triakis\shipwreck.vex`. No racing
  craft authors one - which fits the recovered runtime exactly, where a racing
  ship's ribbon is built **in code** by the `Engine Flare` constructor rather
  than bound from a node. One further file carries a pair and is not identified.
- **Pulse PS2**: `Triakis\Ship.vex` **does** author a `trail_con_left`/`right`
  pair, where the PSP file for the same team does not. Nine further files carry
  pairs and are not identified. **This is a real PSP/PS2 authoring divergence in
  the ship set and it is new here** - whether the PS2 runtime binds those mounts
  or ignores them the way a code-built ribbon would is unread.
- **Pure**: **8 of 12 teams' `Ship.vex`** author a pair - `AG_Systems`,
  `Assegai`, `Auricom`, `Feisar`, `Harimau`, `Piranha`, `Qirex`, `Triakis`. So
  Pure hangs the craft's ribbon off the asset where Pulse hangs it off the
  constructor. Pure's executable has not been read, so what binds them is
  unknown, but a mount authored on eight of twelve racing craft is not
  accidental.

## HD/Fury author the whole thing, and it already decodes

There is no `Trail` node anywhere in HD's 183 `.vex` files. It uses a different
mechanism entirely: a **`/data/ribboneffects/` family**, five ribbons, each one a
model plus a material plus textures.

| Ribbon | Files |
| --- | --- |
| engine trail | `enginetrail_triangle.{vex,rcsmodel}`, `materials/hd_enginetrail.rcsmaterial`, `textures/hd_enginetrail.gtf`, `textures/hd_enginetrail_noise.gtf` |
| wake trail | `waketrail_triangle.*`, `hd_waketrail.rcsmaterial`, `hd_waketrail.gtf`, `hd_waketrail_clouds.gtf` |
| rocket trail | `rockettrail_triangle.*`, plus a separate `rockettrail_shadow_triangle.*` |
| leech beam | `leachbeam_triangle.*`, `hd_leechbeam_glow.gtf` |

**Each `*_triangle.vex` decodes to literally one triangle** through
`oag_render::mesh::rcs::build` - the existing PS3 path, unchanged, no new
decoder - with one texture slot filled and a **second texture loaded but not
drawn**. That is the shape of a *template*: the engine extrudes the ribbon from
the craft's position history and skins it with the material, and the second
texture is the noise map the name of the file next to it advertises. It also
explains one line of `rcs::build`'s own report that has been unexplained until
now.

So on HD the answer to "derive it from the assets" is **yes**, at confidence 90,
and the loader for it is already written. What is *not* read is the
`.rcsmaterial`'s parameters - `oag_formats::rcsmaterial` currently decodes the
shader-variant table "and nothing else yet" - so scroll rates, widths and taper
would still have to come from somewhere.

### The per-livery reading was refuted, and its question is reopened

An earlier version of this page read the ribbon's per-craft variation off
`/data/ships/<team>/livery<N>/engine_flame1.gtf` and `engine_flame_noise.gtf`,
at a stated confidence of 75 with the gap named: nothing had established that
the *ribbon* sampled them rather than the engine **flare**.

**Reading `enginetrail_triangle.rcsmodel`'s material settled it against that
reading.** Its one material names
`data/ribboneffects/textures/hd_enginetrail.gtf` (256x256) and, as its second
texture, `data/ribboneffects/textures/hd_enginetrail_noise.gtf` (128x128). The
per-livery pair is 64x64 and is named by neither. So the engine trail's textures
are the shared `ribboneffects` ones, and the livery pair almost certainly
belongs to the flare - which is a hypothesis, not a finding: the flare's own
material has not been read.

**Which reopens the question.** There is no `fury`-named ribbon asset (all 120
`fury` paths on the EU disc are front-end - track-select emblems, `envsettings`,
campaign flyers), and the ribbon asset that does exist is shared and carries no
per-team skin. What makes one craft's trail differ from another's is therefore
**unestablished**: vertex colour supplied by the team, a second material this
sweep did not find, or nothing at all. Do not read this section as settling it.

### The blend, and three independent sources agreeing

The material's factor pair is `src 0x0302`, `dst 0x0001` -
`GL_SRC_ALPHA`/`GL_ONE`, through `oag_formats::rcsmodel`'s already-recovered
`Factor` enum rather than a second reading of the same bytes. That is
`mesh_render::ADDITIVE_BLEND` exactly.

**So the same additive equation now has three independent sources**: Pulse's
PSP preset (`Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX 0xffffff)`), the PS2
build's GS `ALPHA_1` register write (`0x48`, decoded on
[batch-draw-state.md](../ghidra/functions/ps2-pulse-eu/batch-draw-state.md)),
and HD's own material data. Three platforms, three encodings, one equation.

The sibling ribbons discriminate, which is what says the field is really read:
the rocket trail is `0x0302`/`0x0303`, alpha-over rather than additive.

## What follows for the code

`oag_render::exhaust` holds Pulse-PSP's preset as `TRAIL_*` constants and every
title races on them, which is exactly why "accurately across the games" is
structurally impossible today. The shape the evidence argues for:

1. **Per-title trail parameters in the title packages**, the way
   `oag_pulse::race` already carries what Pulse ships - not a parser, because
   there is nothing to parse on Pulse. `oag-render` reading a title package is
   already how the plume and the presentation tables work and is allowed by the
   dependency rules ([ADR-0022](../architecture/adr/0022-title-packages.md)).
2. **Mount points from the asset where one exists**, since a `Trail` node's
   64 bytes *are* usable and are the one thing the data does supply. That
   immediately covers Pure's eight ships and Pulse's wrecks.
3. **HD through `ribboneffects`**, not through the same type - its ribbon is a
   textured template with a material, not a preset, and forcing it into a
   parameter struct built for Pulse would be the fourth column that does not
   fit.

**Point 3 is implemented as of 2026-08-23** and points 1 and 2 are not.
`oag_title::exhaust::Exhaust` is the axis - an enum, because a title either
names a texture or authors a template and no source does both - and HD now
loads its ribbon's noise texture and its blend off the disc through
`race::assets::trail_texture` instead of falling back to a procedural glow.

**What is asset-derived on HD now**: the noise texture and the blend equation,
both named by the material.

**What is still PSP's on HD, and is a documented divergence rather than a
finding**: every geometry number - `TRAIL_SAMPLES`, the taper, the three layers,
their half-widths, their scroll rates - and the three baked layer colours. HD's
own ribbon code is unread, so fitting those to a picture would be an invention
where leaving them is at least a labelled borrowing.

**What is located and deliberately unwired**: `hd_enginetrail.gtf`, the 256x256
colour map. This renderer's ribbon shader samples one texture and the one it
samples is a noise map, so the colour map has no correct slot to go in;
putting it in the noise slot would be a plausible-looking substitution of the
kind `CLAUDE.md` names. The load report says so on every HD race.

## Open

- Pure's executable is unread, so what binds its eight authored mounts, and what
  parameters it uses, is unknown. It is an unencrypted ELF like Pulse's.
- Whether the PS2 runtime binds `Triakis\Ship.vex`'s authored mounts, or builds
  its ribbon in code as the PSP does and ignores them.
- The 15 version-3 `.vex` files on Pure's disc are excluded from the sweep, the
  version-3 class numbering being unrecovered.
- 1 of Pulse PSP's 3 hits and 9 of the PS2's 10 are unidentified by name.
- The single 0-byte `Trail` payload on Pure - every other one of the 78 is 64.
- `hd_enginetrail.rcsmaterial`'s own contents, which would settle the
  per-livery-texture question and supply HD's scroll and width parameters.
