# WipEout: Omega Collection PS4 functions

Functions from `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
`x86:LE:64:default`, image base `0x01000000`. Getting the file at all and
importing it with [GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) are
covered in [toolchain.md#ps4](../../../reverse-engineering/toolchain.md#ps4);
where the PKG itself came from and how it decrypts is in
[data/README.md](../../../../data/README.md)'s Omega Collection section and
[source-images.md](../../../reverse-engineering/source-images.md).

**A reverse-engineering target as of 2026-09-15**, promoted from "if
feasible" once the lineage match below proved out: `ShipCollisionFx_Trigger`/
`Ship_DispatchCollisionFx` ([ship-collision-fx.md](ship-collision-fx.md))
independently reached the same reading HD/Fury's own PPC64 decompile did,
raising that pair's confidence past the `_q` threshold on **both** binaries
and resolving one of HD's own open questions in the process. That is the
case for this binary specifically: x86-64 needs no custom Ghidra processor
module (unlike PPC64, Allegrex/VFPU, Emotion Engine or ARM), so its
decompiler output is the most mature of any binary in this project, and the
shared `Backend/...` tree below means a function read here can corroborate -
or correct - a reading made on a harder architecture. No gameplay/simulation
milestone opens on it (`the roadmap`'s M8 still gates that on a real second
title, and Omega stays out of that scope); this is function-level RE work
in service of the wider lineage, the same role `ps3-hdfury-eu/` and
`vita-2048-eu-v104/` already play for each other.

## The lineage question, extended a third time: shared tree, confirmed at the code level

`search_strings("Backend")` against this binary returns 76 `.cpp` debug-tag
paths under `C:\WOPS4\Wipeout\Code\Backend\...`; 74 match a path already
named in `vita-2048-eu-v104/eboot.elf`'s own string table by suffix exactly -
same subsystem, same filename, same subdirectory nesting. That is a string-
table-level check only, the same kind `vita-2048-eu-v104/README.md` used for
its own first pass against `ps3-hdfury-eu`. This directory's first named
function goes one step further: [`MagstripWake_Construct`](ships-effects.md)
is structurally the same constructor on both binaries - matching
tagged-object field offset, matching resource-name lookup, matching flag
bits, matching instance-counter pattern - despite the architecture change
from ARM Thumb-2 to x86-64. See [`ships-effects.md`](ships-effects.md) for
the full comparison and
[`vita-2048-eu-v104/ships-effects.md`](../vita-2048-eu-v104/ships-effects.md)
for that binary's own half of the same finding.

**Consequence for future work on this binary**: when naming an unidentified
`ps4-omega-eu` function, check `vita-2048-eu-v104/names.tsv` (and, through
it, `ps3-hdfury-eu/names.tsv`) for a same-role match first - a shared
`__FILE__`-style tag string is often enough to find the candidate function on
the other side, and a matching field-offset/control-flow skeleton is what
turns that into an actual name rather than a guess. A filename match alone is
not enough on its own - see the confidence rubric - the code has to line up
too, the way it did for `MagstripWake_Construct` and did not (cleanly enough
to name) for the `Collision/SimpleMesh.cpp`-tagged pair checked in the same
session: the PS4 side's candidate decompiles to a ~145 KB function versus
Vita's small, separate `SimpleMesh_Load`/`SimpleMesh_Construct` pair, which
reads as heavier inlining rather than a clean 1:1 match, and so was left
unnamed rather than forced.

## Pages

- [zone-craft.md](zone-craft.md) - Zone loads one shared `hdships\Zone` hull for
  every craft; the pick selects a livery.
- [particle-paths.md](particle-paths.md) - the `Data/Particles` versus
  `Data/Particles2048` choice and its runtime flag; no names applied.
- [ships-effects.md](ships-effects.md) - `MagstripWake_Construct`, the first
  name recovered here, transferred from and to `vita-2048-eu-v104` in the
  same pass.
- [ship-collision-fx.md](ship-collision-fx.md) - `ShipCollisionFx_Trigger`/
  `Ship_DispatchCollisionFx`, found independently of `ps3-hdfury-eu`'s own
  reading and agreeing with it - the finding that promoted this binary to an
  RE target in its own right, and the worked example of the technique above.
- [game-boot.md](game-boot.md) - `Game_Main`, `SoundManager_Construct`,
  `FrontendRoot_Construct`, and the finding that this binary inlines
  `GameRoot_Construct`/`SystemRoot_Construct`/`SpeechManager_Construct`/
  `MusicManager_Construct` directly into `Game_Main` rather than keeping them
  as separate functions the way `vita-2048-eu-v104` and `ps3-hdfury-eu` both
  do.
- [weapons.md](weapons.md) - 30 functions: nine weapon-manager constructors
  transferred from `ps3-hdfury-eu/weapons.md` by `.cpp` tag (plus the finding
  that `WeaponExplosions_Construct`, a separate function on `ps3-hdfury-eu`,
  is inlined into `PlasmaManager_Construct` here instead); `RaceManager_Construct`,
  `MPRaceManager_Construct` and all sixteen `_RaceManager` subclasses, each
  proven by an explicit call to its own base constructor rather than tag
  matching alone; and `ModeManager_ConstructByMode` - a single
  ~2,400-instruction dispatcher that builds any of twelve per-mode
  `ModeManager` subclasses from one shared body, first-in-this-project
  evidence that neither HD nor 2048 name their own mode-manager subclasses
  at all. Also records a negative result on
  `find_similar_functions_fuzzy`/`bulk_fuzzy_match` as a substitute for the
  tag technique: real on the PS3↔PS4 pair only one time out of two probes,
  and pure noise on the Vita↔PS4 pair even seeded from a confirmed match.

- [plasma.md](plasma.md) - `PlasmaManager_Update` and the Plasma's whole
  per-tick state machine: a hardcoded 1.0 s wind-up (`charge_time` authored
  and unread, calibrated), a 6.0 probe with world-vertical fall, the
  +-6.0 corridor hit, `damage * weapon_damage_multiplier` with every
  offset traced to `WeaponStats_ParseXml`, and the three-model explosion's
  ramps, 1.3 s collapse and 3.5 s lifetime - cross-checked against
  `ps3-hdfury-eu/plasma.md`.
- [billboards.md](billboards.md) - `Billboard_ConstructResource` and
  `TrackStartup_Load`, transferred from and to `ps3-hdfury-eu` by `.cpp` tag,
  an element-name census, and an exact match on two arbitrary resource-loader
  magic constants (`0xfdb2`, `0x3e9`) - strong enough to also raise that
  binary's own confidence past the naming floor. Along the way, resolves
  `ps3-hdfury-eu/billboards.md`'s own long-open `mode_descriptor` question:
  this binary's slot-7 gantry substitution is a direct branch on its own mode
  global, picking from the same small vocabulary of `321Go_*.vex` names that
  binary's own string census found but could not trace a mechanism for.

- [vex-classes.md](vex-classes.md) - a two-record spot-check confirming
  `ps3-hdfury-eu`'s own `g_VexClassTable` (866 records, confidence 95) also
  exists here in the same shape, scaled to 64-bit pointers. No `names.tsv`
  row - the spot-check doesn't clear the bar a full read would.

- [psarc-mount.md](psarc-mount.md) - `PsarcArchive_Mount`/
  `PsarcArchive_WaitAndMountAll`: this binary has no PSARC reader of its own
  at all. It mounts every `data%02d.psarc` through Sony's own FIOS2
  (`sceFiosArchiveMountSync`) behind a PlayGo chunk-locus poll, so there is
  no game-owned block-read path to compare `oag_formats::psarc` against -
  the actual read implementation is inside the signed `libSceFios2.prx`
  system module, not this executable.

- [lightmap-prelit.md](lightmap-prelit.md) - what `Lighting.Nova prelit scale
  bias power` does: the circuit pixel shaders compute `pow(lightmap, power) *
  scale + bias` on the raw UNORM atlas, with no constant ambient on a lightmapped
  surface (4,664 of 4,664 nova shaders), the atlas alpha is the sun mask, and
  the output is fp16 into a tonemap this project does not have. The CPU chain
  (`EnvSettings_RegisterKeys`, the `(1.4, 0.2, 1.5)` default, the per-frame
  upload and the slot-32 binding) and a negative on the `Tonemap.*` consumer.

- [collision.md](collision.md) - a tag-located candidate for
  `ps3-hdfury-eu/collision.md`'s own `Collision_Construct`, not named: the
  structural evidence that first looked like a match turned out to be
  shared tagged-object boilerplate this binary reuses across unrelated
  classes (documented as a negative result for future tag-transfer
  attempts), and HD's own discriminating invariant - a strided loop whose
  count times stride lands exactly on a separately-computed allocation size
  - isn't present in the disassembled window here. No `names.tsv` row.

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same
change: `scripts/apply-ghidra-names.py` refuses a row whose address and name
do not both still appear on the page named in its last column.
