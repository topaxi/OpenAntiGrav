# 2048's PSARC and title plumbing are wiring-ready; two binary formats changed underneath a track

2026-08-26. Scoped to "load a Wipeout 2048 craft and track to spawn in via
`just play 2048 --race`, no menu/intro" - see [2048 vs HD/Fury lineage](vita-2048-vs-hd-fury-lineage.md)
for the codebase-lineage question this corroborates further. Worked in
`../oag-2048` (`git worktree add`), against `data/extracted/vita/PCSF00007/base/PSP2/data.psarc`
(1.6 GiB, EU base package, decrypted per
[the eboot thread](2048s-vita-eboots-are-imported-re-not-started.md)'s pipeline -
the patch archives were deliberately not touched, see below). Two scratch probes
are the reproducer for every claim here: `crates/game/examples/psarc_list.rs`
(lists/filters a loose `.psarc`'s manifest) and `crates/game/examples/vita_probe.rs`
(everything else, `cargo run -p oag-game --example vita_probe -- <psarc path>`).

## What already works, unmodified

**The PSARC container reads with zero code changes.** `oag_assets::psarc::Archive::open`
parses `data.psarc` (version 1.4, flags 1, 18,430 entries) exactly as it parses
HD/Fury's PS3 archives (version 1.3, flags 3) - the header fields are read
generically and nothing asserts either value. The manifest gives every path as a
real string (`data/art/published/environments/altima/track.vex`), same as HD.

**The asset *tree* is HD/Fury's, not Pulse's**, corroborating
[the lineage thread](vita-2048-vs-hd-fury-lineage.md) a second, independent way:
`.rcsmodel`/`.rcsmaterial`/`.rcsskeleton`/`.rcsanimclip`, `.pob`, `.pvs`/`.probes`,
`.envsettings`, `.bnk` - HD's whole extension family, present here too. 2048 also
ships all fourteen HD teams verbatim under `data/art/published/hdships/<Team>/`
(`Ship.vex`, `handlingstats.xml`, `engineflare.vex`/`.rcsmodel`, the `_c1`/`_n1`
Fury variants, everything), alongside its own five-team native roster (see
"What's new" below) and eleven native circuits
(`altima`/`arena`/`bridge`/`cathedral`/`mall`/`park`/`sol`/`square`/`subway`/`tower`,
plus a `shared` directory) under `data/art/published/environments/<name>/`, laid
out exactly like HD's own circuits (`track.vex`, `track.rcsmodel`,
`track_col.col`, `stats.xml`, `start_grid.vex`, `TrackStartup.xml`, `track.pvs`).

**Every HD-derived handling file decodes with the existing parser, unchanged.**
`oag_formats::handling::from_blob` on `data/art/published/hdships/AG_Systems/handlingstats.xml`
returns the expected four classes (`VENOM`/`FLASH`/`RAPIER`/`PHANTOM`).
`oag_formats::handling::global_from_blob` on `data/xml/handlingstats.xml` (the
`GLOBAL_ENTRY` analogue) returns a fully sane `Global` block - zone
start/increment/recharge, four classes' worth of speedup-pad and gravity
tunables, weapon-pad refresh times - so the shared `<Global>` schema is
unchanged too.

**The native roster's own handling files also decode**, which was not expected
going in: `data/HandlingStats/ag_systems2048/1/handlingstats.xml` parses with
five classes (`VENOM`/`FLASH`/`RAPIER`/`PHANTOM`/`SUPERPHANTOM` - a class the
existing schema has no trouble with, it is just another `<Class>` block). The
five teams are `ag_systems2048`, `auricom2048`, `feisar2048`, `piranha2048`,
`qirex2048` - 2048's real five-team roster - each with four numbered
subdirectories (`1`/`2`/`3`/`4`) whose relationship to speed class or ship
variant is unread.

**The ship-directory root is a title fact that has to change, and it is a clean
one.** `crates/title/src/race.rs`'s own module doc argues, in so many words,
that `Data\Ships\<Team>\...` is shared vocabulary and not a title field - true
for Pulse, Pure and HD alike, all three of which store it at `Data\Ships\`.
2048's HD-derived roster does not: `Data\art\published\hdships\<Team>\handlingstats.xml`
is confirmed resolving against the manifest (`Archive::contains`, exact literal
match after the normal backslash/case fold). This is the third-title
disagreement [ADR-0022] asks for before an axis becomes a field, and it is not
just `handling::entry_name` - `oag_pulse::race::ships::entry_name(team, model)`
builds the same `Data\Ships\{team}\{model}` shape and several call sites in
`oag-game` (`race/assets.rs`'s `ship_entry_name`/`boost_entry_name`/`shield_entry_names`,
`livery/flare.rs`, `boot/roster.rs`) go through it for the hull, boost plume,
shield and flare. None of that is wired yet - see Next Steps.

**Skipping the patch PSARCs is a deliberate, documented simplification, not an
oversight.** `data.psarc` alone carries all 18,430 entries the census above
depends on; the patch's `data1.psarc`/`data2.psarc` (136 MiB combined, inside
`patch-v104/PSP2/`) hold some subset of changed entries and were not inspected.
Patch-over-base is this project's policy for *which binary to decompile*
(EU 1.04, per `CLAUDE.md`); it says nothing about which PSARC an asset-loading
title package should prefer, and reading only the base keeps this pass's
footprint down. Worth mounting the patch archives ahead of base, HD's
`extra`-archive-order style, once something needs a file the patch actually
changed.

## What's new, and blocks a track from loading today

**`track.vex`'s class table is version 6, byte-order little-endian - identical
class IDs to HD's version 6** (measured via `vex::classes_of` on
`environments/altima/track.vex`: `mesh`, `wo_track`, `section`, `skycube`,
`floor_collision`, `engine_flare`, all the same numbers HD's version-6 tracks
carry). `mesh::geometry_is_external` reads `false` here because its whole test
is "is the vex file's own byte order big-endian" - a PS3 (PowerPC, BE) proxy
that a Vita (ARM, LE) file defeats by construction, even though the geometry
*is* external exactly as it is on HD. This one is a cheap, well-understood fix
once the two formats below are readable: the heuristic needs a third answer
alongside `Big`/`Little`, or a Vita-specific override.

**The `.rcsmodel` beside it is not HD's container.** `mesh::rcs::build_scene`
fails immediately: `oag_formats::rcsmodel`'s header read is `ByteOrder::Big.u32(data, 0)`,
unconditionally, and expects `0x000a0000`; `environments/altima/track.rcsmodel`'s
first four bytes are `ed ad 5c ca` read either way - not a byte-swap of the
expected constant, a different value entirely, at 17.4 MiB for one circuit
(HD's biggest circuit .rcsmodel is nowhere near that). The container itself
is unrecovered: no chunk table, no version scheme, nothing beyond the
observation that it opens with what looks like a hash or GUID rather than a
version word. Needs a first RE pass of its own, most likely by decompiling the
model-loading path in `vita-2048-eu-v104/eboot.elf` (Ghidra, per `CLAUDE.md`'s
"RE shall be done on EU 1.04" default) rather than guessing further from bytes
alone - a chunked container with a genuinely different header shape is not
something a stride search alone recovers.

**The `WO Track` (AI spline) payload is version `0x107`, one above every
previously-seen version (Pulse ships `0x105`), and its per-point record shrank
from 112 to 96 bytes.** This is the one format this pass got most of the way
into, and it is worth recording precisely:

- Header, path array, junction array are all **unchanged** - same offsets,
  same 32/16-byte strides. Confirmed by reading every path's and junction's
  header fields directly and getting plausible counts across all fourteen
  `track.vex` entries the archive carries (ten native circuits, four DLC).
- `AiTrack::encoded_len()`'s formula matches the payload **exactly, on all
  fourteen tracks** - `HEADER_LEN(0x20) + RESERVED_LEN(0x20) + paths*PATH_LEN(0x20)
  + junctions*JUNCTION_LEN(0x10) + total_points*96` lands on the file's own
  payload length to the byte for every one of them (`altima`: 6 paths/4
  junctions/2029 points/195104 bytes; `mall`, the largest, 18 paths/12
  junctions/3288 points/316480 bytes; and twelve more in between), and stride
  112 (the pre-2048 value) matches **none** of them. **Confidence 95** - a
  corpus-wide exact arithmetic closure with the alternative hypothesis
  (stride unchanged) failing on every member is about as strong as evidence
  gets without reading the loader itself.
- **`pos`/`tangent`/`down`/`lateral` stayed at their original offsets** (`0x00`,
  `0x10`, `0x20`, `0x30`, each a 16-byte slot for a 12-byte vector). Found by
  scoring every candidate *stride* 16..160 against how smoothly consecutive
  points' position vectors move (right stride: points sit metres apart; wrong
  stride: noise two to six orders of magnitude worse - 96 wins over the next
  candidate, 8497 vs 10.5 million), then, at the confirmed 96-byte stride,
  scoring every 4-byte offset by how close a `[f32;3]` read there sits to unit
  length across all 2029 points of `altima`. `0x10`/`0x20`/`0x30` land at
  ~2.5e-8 mean error (float rounding on a genuinely unit vector); every other
  offset is at least six orders of magnitude worse. (An earlier pass of this
  probe under-ranged that scan and left two unscanned slots at their
  initialised zero, which briefly looked like a suspicious perfect match -
  fixed; there is no unexplained zero-scoring offset once every slot is
  actually measured.) **Confidence 90.**
- **The tail - `half_width_left/right`, `ai_bound_left/right`, `racing_line`,
  `section_id`, `flags` - has moved and is not yet placed. Confidence ~10,
  hypotheses only.** The old layout put them at `0x44..0x62` with an 8-byte
  dead gap after `lateral` and 14 bytes of trailing pad to `0x70`; the new
  96-byte record has only 32 bytes left after the four vectors for the same
  job, so at least 16 of the old 36 tail bytes (gap + fields + trailing pad)
  are gone somewhere in there. Eyeballing six consecutive points of one path
  on `altima`: `0x48` (~20→14, decreasing) and `0x50` (~12.8→6.6, decreasing)
  are both positive and track-width-shaped, so are the weakest kind of
  candidate for `half_width_left`/`half_width_right`; `0x44` (~59→65,
  increasing) and `0x4c` (~-51→-57, decreasing) are a monotonic pair not in
  the old schema at all - **tested and ruled out as a literal alias of
  `pos.x`/`pos.z`** (the per-point difference itself drifts by ~18 units
  across six points rather than holding constant or scaling cleanly, so
  it is correlated with position but is not simply it); and the last 8 bytes
  (`0x58..0x60`) read bit-identical across all six sampled points
  (`2a 2a ff 00 01 00 46 44`), consistent with `section_id`/`flags` staying
  constant through one section of one path - equally consistent with a
  sample too narrow to see anything vary. **None of this should be treated as
  more than a hypothesis list.** Six points of one path of one track is not
  enough to separate "real field" from "coincidentally smooth stretch", and
  CLAUDE.md's confidence rubric would not clear a rename or a parser change on
  it. A wider sweep (every path of several tracks, checking each candidate
  offset for plausible *ranges* rather than eyeballing six rows) is the next,
  still-cheap step; see Next Steps.

**Collision moved out of the vex file entirely and into its own container.**
`collision::from_vex` correctly returns zero nodes (not a bug - the collision
class IDs are in the class table with no node instances, checked in the code
before it settles for empty rather than erroring). The 1,068,082-byte
`track_col.col` beside `track.vex` is a *new*, wholly separate top-level file:
its first twelve bytes are the literal ASCII tag `kdtr0001----`, i.e. a k-d
tree, not the flat triangle-soup chunk format `oag_formats::collision::parse_chunks`
reads out of a vex node. Nothing here has looked past that tag.

## Why nothing is wired up yet

Three things gate a race spawning at all, in `crates/game/src/race/load.rs`:
`track_render::load` (hard error via `?`, needs `WO Track`), `mesh::build_with_textures`
or `mesh::rcs::build_scene` for the drivable geometry (both currently error;
everything downstream of a missing model degrades to "drawn nothing" rather
than failing, per every other title), and `collision::from_vex` (does not error,
just returns nothing - a real track needs `track_col.col` instead, once its
own format is read). **The `WO Track` payload is the one that is fatal today**:
`oag_render::track::load` propagates its `OutOfBounds` error with `?` and
nothing downstream runs. Building the `oag-vita` crate, the `Platform::Vita`
disc variant, the `ship_dir` axis and the `just play 2048` justfile case now
would produce a command that compiles and then fails at exactly that line -
which is an honest result, but not the "spawn in" the task asked for, and
`CLAUDE.md`'s rule against inventing what the assets already author says not
to guess the tail-field offsets just to get past it: a wrong half-width or
`ai_bound` would silently steer the AI off the actual track rather than fail
loudly.

## Open

- The `.rcsmodel` container's header and chunk layout: nothing beyond "not
  HD's `0x000a0000`-versioned one, and much larger for the same circuit".
- The `WO Track` tail's exact field offsets past `pos`/`tangent`/`down`/`lateral`
  (see above) - two plausible half-width candidates, two unexplained
  monotonic fields, and 8 tail bytes seen constant across too small a sample
  to trust.
- The k-d tree `.col` format: nothing past its magic tag.
- Whether the `.rcsmodel` container is version-`0x107`-specific or shared by
  every 2048 track - checked on one track (`altima`) only. (The `WO Track`
  stride itself is now corpus-confirmed across all fourteen `track.vex`
  entries, see above - this open item is the `.rcsmodel` header alone.)
- The native roster's numbered subdirectories (`ag_systems2048/1..4`) and their
  relationship to speed class or ship variant.
- Whether the patch PSARCs (`data1.psarc`/`data2.psarc`) touch any of `altima`,
  `AG_Systems` or the global handling file at all - not checked, since base
  alone was sufficient for everything above.

## Next Steps

1. Nail the `WO Track` tail layout with a wider statistical sweep (every path
   of every one of the fourteen tracks the corpus check above already
   enumerates, range/monotonicity checks per candidate offset rather than
   eyeballing six rows) before touching `crates/formats/src/track.rs` - a
   version-gated `point_len(version)` the same shape as `reserved_len(version)`
   is the fix once the offsets are confirmed, not before. The stride itself
   (96 bytes) and the four vector offsets are already corpus-confirmed at
   confidence 90+ and do not need re-checking.
2. Open `vita-2048-eu-v104/eboot.elf` in Ghidra for the `.rcsmodel` loader -
   this is the one format here that a byte-level stride search is unlikely to
   crack alone, per the module docs' own account of how subtle a "looks
   plausible but is one field off" reading can be.
3. Once both parse, add the `ship_dir` axis (`RaceDefaults` field, or a
   dedicated one) and thread it through `handling::entry_name`,
   `oag_pulse::race::ships::entry_name` and their `oag-game` call sites -
   scoped care should keep this to the hull/handling path for an MVP craft,
   leaving boost/shield/flare cosmetics to report their own absence the way
   every other title's do for an unrecovered asset.
4. Then the wiring this pass deliberately did not do: a `Platform::Vita`
   variant in `oag-disc`, a new `oag-vita` title crate (archives = `["data.psarc"]`
   only, front end/loading/music left `None` per the "no menu, no intro" scope,
   `hud_art` left conservative - empty `always_on`, `Sights::Unread` - rather
   than copied from HD on a guess), a try in `oag_game::title::open_source`,
   and a `2048` case in the `justfile`'s `play` recipe with the same
   present-or-explain-how-to-regenerate check the `hd` case already has for
   `data/extracted/`.
