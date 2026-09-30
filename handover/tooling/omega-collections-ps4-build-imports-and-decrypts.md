# Omega Collection's PS4 build imports, decrypts and is mapped; asset-level access is not there yet

2026-09-15. `omega-ps4-eu.pkg`/`omega-ps4-eu-patch.pkg` (a scene fPKG, not a
retail PSN download - see
[data/README.md](../../data/README.md#the-omega-collection-pkg-decrypts-on-a-pc-the-archives-are-plain-the-executable-isnt-yet))
decrypt fully on a PC with [LibOrbisPkg](https://github.com/maxton/LibOrbisPkg),
no console needed, into `data/extracted/ps4/omega-eu/uroot/`: `eboot.bin`
(10.5 MiB), `sce_module/{libc,libSceFios2,libSceNpToolkit2}.prx`, and five
`dataNN.psarc` asset archives (~40.8 GiB total: `data00` 12.6 GiB, `data01`
10.0 GiB, `data02` 9.1 GiB, `data03` 2.4 GiB, `data04` 6.0 GiB). Full evidence
and the exact commands: [source-images.md](../../docs/reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4).

`eboot.bin` is a PS4 SELF whose segments are, for this specific fPKG build,
already unencrypted (checked directly: all 10 segments' `ENCRYPTED` bit
reads 0) - [GhidraOrbis](https://github.com/astrelsky/GhidraOrbis)
(`just build-ghidra-orbis`, pinned to tag `1.0144` - `master`'s HEAD has a
confirmed compile break, see
[toolchain.md#ps4](../../docs/reverse-engineering/toolchain.md#ps4)) imports
it with no further decrypt step: `.sce_special`/`.dynamic` overlay blocks and
2,172 loader-seeded functions before auto-analysis even runs are the tell
that the real Orbis loader claimed it, not a stock-loader fallback. After
`run_analysis`: **20,941 functions**, image base `0x01000000`, `x86:LE:64:default`.
Program path `/ps4-omega-eu/eboot.bin`.

## The codebase-lineage question, extended a third time

`vita-2048-eu-v104/README.md` already confirmed 2048 (Vita) shares
`ps3-hdfury-eu`'s (PS3) own `Backend/...` source tree. This binary does too:
74 of 76 `.cpp` debug-tag paths under `Backend/...` match a Vita path by
suffix exactly, and the first function named here,
[`MagstripWake_Construct`](../../docs/ghidra/functions/ps4-omega-eu/ships-effects.md),
matches Vita's own copy field-for-field (tagged-object offset, resource
lookup, flag bits, instance counter) despite the ARM-to-x86-64 architecture
change. Full writeup, including one *negative* result
(`Collision/SimpleMesh.cpp`'s tagged pair does not transfer cleanly - the PS4
candidate is one ~145 KB function against Vita's two small, separate ones,
read as heavier inlining rather than a 1:1 match, left unnamed rather than
forced): [`ps4-omega-eu/README.md`](../../docs/ghidra/functions/ps4-omega-eu/README.md).

**Subsystem census** (`search_strings("\.cpp")`, 357 total debug-tag paths,
bucketed by top-level source directory):

| Directory | Count | What |
| --- | ---: | --- |
| `Frontend/` | 189 | Menu items (90), screens (40), general/plugin scaffolding |
| `Backend/` | 76 | Gameplay: `General` (race/mode managers, collision), `Ships`, `Weapons`, `World` |
| `System/` | 59 | `Render` (importers, 50+), `Sound`, `System`, `Particles` |
| `Network/` | 8 | `Community`, `raknet`, `Unity` |
| `GameModes/` | 3 | `GameMode_{ModeManager,RaceManager,TournamentModeManager}` - a wrapper layer `Backend/General`'s own per-mode managers don't have on Vita/PS3 |
| `Game/` | 4 | `GameRoot`, `Profile`, `GloryMoment`, `FunkLayer` |
| (Sony `Reach` SDK, PAL render-resource loaders, `WOShips`) | ~18 | Third-party/platform scaffolding, not this project's own code |

Only one function is named so far (`MagstripWake_Construct`) - this is a
map of where the code *is*, not RE work done on most of it. `GameModes/` is
worth a second look: it is not present in the Vita `Backend/` census at all,
so either PS4 adds a wrapper layer over the per-mode managers both other
titles already have, or it is a rename of something Vita calls by a
different path - not determined.

## Assets: same `Data/...` namespace as PS3, but the PSARC container changed underneath

`oag_formats::psarc` (written against PS3's archives) declares itself
version 1.3; every one of these five archives declares **1.4** (same `flags`
value, 13, across all five). A first real difference was found and fixed in
an earlier session; the paragraphs below are later sessions layering more
findings on top, ending with a block-data-location problem still open.

**Fixed in an earlier session**: the block-width probe (`read_block_table`)
computed its "highest referenced block" across every entry unconditionally,
including entries with `size == 0`, whose `first_block` field is
unspecified. `data00.psarc` entry 9042 (`size` and `offset` both `0`)
carries `first_block = 0x960c4925` (2,517,387,557) - large enough that no
block-table width could ever "cover" it, so the whole archive read as an
unrecognised layout. Fixed by applying the same `size > 0` filter the
out-of-range check one function below it already uses
(`crates/formats/src/psarc.rs`, `0b8d16ad`).

**Resolved this session**: the manifest (entry 0, the one whose digest is
sixteen zero bytes) is **NUL-delimited on this archive family, not
newline-delimited**. Measured directly on `data00.psarc`'s manifest
(598,798 bytes): zero `\n` bytes, and NUL-splitting it recovers 10,714
well-formed paths - **not the 22,582 the previous session's count reported**,
which was `split(NUL).len()` including 11,868 empty segments produced by
three long zero-byte runs inside the manifest text itself (verified against
`crates/formats/src/psarc.rs`'s Rust reader directly, not just the Python
reimplementation this session started with). The `entry_count` (10,927)
versus manifest-path-count mismatch this thread flagged as unexplained
**is resolved, and it is not the `entry_count - 1` a PS3-shaped manifest
would produce, but not a 2x ratio either**: entry order carries no
relationship to manifest order at all on this family - the entry table is a
concatenation of several separately digest-sorted runs, interleaved with
thousands of fully-zeroed placeholder rows. The correspondence that does
hold is by MD5 digest, the same field PS3 archives already carry for
verification: 1,492 of `data00.psarc`'s 1,533 non-zero-digest entries
(97%) resolve to a manifest path this way, and the same holds on all four
other archives (97-99%). Full measurements, the corrupt-row fix below, and
the still-open block-data-location problem: `docs/formats/psarc.md`'s new
"The PS4 Omega Collection family" section. Landed in
`crates/formats/src/psarc.rs` and `crates/assets/src/psarc.rs`; `.gnf`
(Sony's PS4-native texture container) still has **no reader anywhere in
this project** and is unaffected by this fix.

**Regressed in a later session, fixed in the same one it was caught:** the
fix above landed as `Header::nul_delimited_manifest` (`version_major == 1 &&
version_minor >= 4`) dispatching `parse_manifest` and
`match_paths_to_entries` between a NUL-delimited/digest-matched reading and
a newline-delimited/positional one. That merged to `main`
(`70064846`) and broke a real, previously-green ground-truth suite: a peer
session bisected `oag-game::zone_grade_ground_truth`'s two 2048-branch tests
failing at `70064846` but not at its parent, `1b3c8c57`, with `data.psarc`
(Vita `2048`, `PCSF00007`) reporting `NoSuchPath` for
`Data\art\published\environments\altima\ZoneMode2048.effectSettings`.
Checked directly: Vita's `data.psarc` **also declares version 1.4** and is
the well-behaved PS3 shape throughout - newline-delimited manifest (18,429
`\n` bytes, zero `\x00`), 18,430 manifest lines matching its entry count
exactly, and 100% of its real entries' digests matching their positional
manifest line's, entry `n + 1` to line `n`. The header's declared version
does not predict which shape an archive is; it was never a safe dispatch
key, and this project had no second `1.4`-declaring archive to catch it
until now. Fixed by making both functions read the data instead of the
header: `parse_manifest` picks NUL-delimited only when the manifest has no
`\n` byte at all (and has at least one `\x00`); `match_paths_to_entries`
always matches by digest, unconditionally, which reproduces a well-behaved
archive's own positional order as a corollary rather than needing a
separate code path for it - proven identical on the synthetic case and
confirmed against `data00`-`data04` and Vita's `data.psarc` alike.
`Header::nul_delimited_manifest` is removed; neither function takes a
`Header` any more. `docs/formats/psarc.md`'s "The PS4 Omega Collection
family" heading (was "Version 1.4") reflects the correction throughout.

**Also resolved this session, a separate bug found while verifying the
above**: three of the five archives (`data00.psarc` entry 9042 - the
already-known sentinel - plus newly-found `data02.psarc` entry 4676 and
`data04.psarc` entry 318) each carry one row with a real digest but a
`first_block` in the billions, which used to fail the whole directory parse.
Two of the five archives (`data02`, `data04`) could not be opened *at all*
before this fix. `read_block_table` now excludes an implausible
`first_block` from its own probe the same way it already excludes a zero
`size`, and `Directory::parse` no longer validates every entry's block range
eagerly - `Directory::entry_range` already does that lazily, per entry.

**Still open, and a real/zero split rather than uniformly broken - corrected
after an independent review caught the first framing overclaiming.** An
early check sampled only entries whose first block's table row is non-zero
and found zero real reads there, which was reported as "reading an entry's
declared bytes does not currently produce its real content" - overstated,
because that sample happened to pick a subset with worse odds, not a
representative one. A full sweep over every real entry through the crate's
own unmodified reader (`Archive::read_path`) finds `entry.offset` already
locates real content on 819/1,528 (54%) of `data00.psarc`, 348/892 (39%) of
`data01.psarc`, and 99/327 (30%) of `data03.psarc` - and zero bytes for the
rest, with no predictor found yet (not the stored-block shape, not the
offset's position in the file; the block table's own arithmetic is
otherwise self-consistent - `max(entry.offset + entry.size)` over every real
entry on `data03.psarc` lands exactly on the file's true size). **The
extraction-provenance lead this paragraph used to float here is closed,
negative, 2026-09-15** - see Open, below, and
[the new thread it split into](omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md).
See `docs/formats/psarc.md`'s "Block data location" section for the full
measurements and exact repro commands.

**Asked directly and checked before closing the session: is this still
zlib?** No block belonging to a real entry, on `data00.psarc`, `data01.psarc`
or `data03.psarc`, shows the signature a genuinely deflated block would -
every block's table value is either exactly `0` (a full padded `block_size`
of stored bytes) or exactly the entry's remaining byte count (a short,
unpadded stored block). Zero blocks fall in between. The header's own
`compression` field still reads `"zlib"`, unchanged from PS3, but nothing
checked on this family is actually compressed - every real file sampled is
stored raw. This rules out a codec mismatch as part of the block-data-location
mystery: the open problem is purely about finding the right offset, not
about a second, decompression-shaped problem on top of it.

## Open

- **Racing starts, and the node table now reads (2026-09-29, `omega-race`,
  `omega-nodes`).** On a corrected extraction `oag-game <dir> --race` loads
  Omega's own spline (39 of 39 `track.vex` files), collision (a 19-byte k-d node,
  38 of 38), hull and circuit geometry (2048's `.rcsmodel` with 64-bit pointers,
  all 1,272 entries) and textures, and the craft drives the circuit. What it
  does **not** read: the skeleton and clip
  (`property tag ends at 3496925615 but the file is 57856 bytes`), lightmaps
  (291 materials name one on `tech_de_ra`), `track.final.pvs`, Omega's `.bnk`
  banks (version word `1145588546`, the reader expects `3`), and the
  `.EnvSettings` schema (read through 2048's reader; the patch copy has no
  HDR/bloom block). Evidence and numbers:
  [`omega-status.md`](../../docs/formats/omega-status.md#racing-a-race-starts-on-this-titles-own-data).
- **The block-data-location questions below were an extraction bug, not a
  format.** A short read in the extraction tool zeroed most of every entry
  (`omega-status.md`, "An extraction-tool bug"); the corrected copy has 60,151 of
  `tech_de_ra\track.vex`'s 133,888 bytes different from the short-read one's.
  **Promoted 2026-09-29**: `data/extracted/ps4` is the corrected extraction (the
  short-read one is `data/extracted/ps4.bak`), `omega_psarc_ground_truth` was
  re-measured on it (`psarc.md`, "Block data location and the short-read
  extraction"), and `crates/game/tests/omega_race_ground_truth.rs` skips only
  on a short-read copy, `OAG_OMEGA_SOURCE=<dir>` overriding the directory.

- **Block data location: closed - it was the extraction tool (2026-09-29).**
  Every "garbage", "all-zero" and "roughly a third to a half real" figure
  this thread carried came from the short-read extraction. On the whole one,
  all nine archives are a bijection between manifest paths and entries
  (46,005 of 46,005), every entry reads, and every entry with a magic carries
  it (26 `.vex` in the big-endian spelling, `XXEV`). The patch's four archives
  are names the base `.pkg` does not have, and nothing was merged or absorbed.
  Numbers and the list of retired claims:
  [`docs/formats/psarc.md`](../../docs/formats/psarc.md#block-data-location-and-the-short-read-extraction);
  the patch finding:
  [omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md](omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md).
- **Closed, negative, 2026-09-16 (`lane/omega-rcs`): PS4 `.rcsmodel`/
  `.rcsmaterial` are not this format stored little-endian.** The byte-order
  hypothesis this bullet used to carry is withdrawn: every real sample opens
  a genuine new tag (`0xCA5CADED`/`0xCA5CADE5` little-endian, "cascaded" in
  hex-speak) rather than the PS3 version word with its bytes reversed, and no
  field past the tag matches this page's PS3 layout at either byte order.
  `.vex` **is** the simple byte-order case, confirmed directly this session
  (`Data/art/published/hdships/auricom/Ship_LOD.vex` decodes to four real,
  named nodes through `oag_vex::vex` unmodified) - so `.rcsmodel`/
  `.rcsmaterial` are the exception, not `.vex` the rule. `psarc_oracle` now
  scores the tag the same way it scores `.vex`'s `VEXX` and `.gnf`'s `GNF `:
  77 of 250 `.rcsmodel` and 439 of 1,270 `.rcsmaterial` disc entries carry
  it, and 91%/88% of those still resolve one of this project's own
  `~crc32` name-hash preimages somewhere in the file - genuine material data
  in PS3's own hash namespace, wrapped in an unread container. No parser
  exists for that container: the bytes past the tag include an offset table,
  an embedded node name (`OutlineShape`) and a 4x4 identity matrix that this
  format's PS3 shape does not carry at all, which is real reverse-engineering
  work this session's evidence does not support rushing. Full measurements:
  [rcsmodel.md](../../docs/formats/rcsmodel.md#the-ps4-omega-collection-a-different-container-not-a-byte-swap-of-this-one),
  [rcsmaterial.md](../../docs/formats/rcsmaterial.md#the-ps4-omega-collection-wraps-this-in-a-different-unread-container).
  `oag_rcs::rcsmodel`/`oag_rcs::rcsmaterial` are left untouched: the PS3
  ground-truth suites stay exactly as they were, and no drawing was
  attempted on the PS4 side since no parser exists to feed one.
- `GameModes/*.cpp` (`GameMode_ModeManager`, `GameMode_RaceManager`,
  `GameMode_TournamentModeManager`) has no obvious Vita/PS3 counterpart in
  the `.cpp`-path census - worth checking whether Vita's own
  `GameModes/GameMode_RaceManager.cpp` (present in its own string table per
  `vita-2048-eu-v104/README.md`'s own comparison table) is the same file
  under a path this census missed, before concluding PS4 added a layer.
- **`.gnf` has a reader now** - `oag_texture::gnf`, header and the 8-dword
  GCN "T#" descriptor (surface format, dimensions, tile mode), triangulated
  from two open-source readers and AMD's public GCN ISA reference rather
  than a first-party Sony spec. See [`docs/formats/gnf.md`](../../docs/formats/gnf.md).
  **No pixel decoded**: every real `.gnf` sampled declares a genuinely
  tiled mode, and untiling GCN correctly needs the full macro/micro
  tile/pipe/bank-swizzle table with no real PS4 to check the picture
  against - left as the next gap, the same shape `.gtf`/`.gxt` closed for
  PS3/Vita once a linear (untiled) sample turns up, if one does.
- Ghidra's own `analyzed` flag reads `false` on `/ps4-omega-eu/eboot.bin`
  even though `analyzing` is `false` and the function count (20,941) has
  been stable across repeated checks - not chased further, likely a
  metadata quirk of triggering analysis through the bridge rather than
  Ghidra's own GUI dialog, same as noted for the Vita programs in
  [2048s-vita-eboots-are-imported-re-not-started.md](2048s-vita-eboots-are-imported-re-not-started.md).

## Next Steps

- **Read the PS4 skeleton and clip** (`oag_rcs::rcsskeleton`/`rcsanimclip` fail
  with `property tag ends at 3496925615 but the file is 57856 bytes`; the node
  table reads since 2026-09-29, and the model's ids appear verbatim in the
  skeleton in 61 files, so the container is there). That moves the droid, the
  crowd and the rotors, and places the 18,255 submeshes on 2048's Zone models
  that have no matrix of their own (now not drawn, `Report::unplaced`). 3-4
  hours if it is the Vita's layout with wider pointers, as the node table was.
- **Bind the `lightmap` sampler** (`oag_rcs::rcsmodel::psp2::material::read_ps4`
  already ranks it last; the mesh's second UV set carries its coordinate).
- **Decide the mount order with a capture, not a guess**: `oag_omega::EXTRA_CANDIDATES`
  puts the patch ahead of the base because the patch's `.EnvSettings` copies are
  newer (chosen, not measured).

- Resolve why the "garbage" bucket exists (see Open, first item) - real,
  substantial bytes present at an offset with no arithmetic problem, that
  are neither all-zero nor the format their own extension claims. Nothing
  tried so far (stored-block shape, file position, a base/patch merge, a
  constant per-entry offset shift) predicts it; the executable itself has no
  PSARC reader to compare against, so the only avenue left is inside
  `libSceFios2.prx` (Sony's signed system module, held but not opened in
  Ghidra - see `psarc-mount.md`'s own "Not read") or a from-scratch
  statistical pass over the "garbage" population itself.
- A full per-archive file/extension census is done -
  `crates/assets/examples/psarc_oracle.rs`'s own output, and
  `docs/formats/README.md` now carries a row per extension found. Still
  open from an earlier session: whether `data03.psarc` (328 real entries, by
  far the smallest of the base `.pkg`'s five) is a distinct content package
  worth checking against `data/dlc/` before assuming it is just "more of the
  same". **The same census on the patch's own four archives is done and
  says no** -
  [omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md](omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md)
  names every HD circuit and every 2048 zone-mode environment already known
  to this project across `data05`/`data08`, no name outside either roster,
  so `data03.psarc` is now the only one of the nine archives left unchecked
  for the same question.
- **The executable became an RE target 2026-09-15** - see
  [`docs/ghidra/functions/ps4-omega-eu/ship-collision-fx.md`](../../docs/ghidra/functions/ps4-omega-eu/ship-collision-fx.md):
  independently reading HD/Fury's own `Ship_DispatchCollisionFx`/
  `ShipCollisionFx_Trigger` chain on this binary reached the same kind
  mapping, raised both functions' confidence past the `_q` threshold on
  **both** binaries, and resolved one of HD's own open questions (the
  plain-damage spark variants are owned by sibling functions, not folded
  into the same dispatcher). Gameplay/simulation work is still unplanned -
  this is corroboration groundwork, not a milestone.
- **The boot chain and five weapon-manager constructors are named, same
  day.** [`docs/ghidra/functions/ps4-omega-eu/game-boot.md`](../../docs/ghidra/functions/ps4-omega-eu/game-boot.md)
  names `Game_Main`, `SoundManager_Construct` and `FrontendRoot_Construct`,
  and finds that this binary inlines `GameRoot_Construct`/
  `SystemRoot_Construct`/`SpeechManager_Construct`/`MusicManager_Construct`
  directly into `Game_Main` rather than keeping them as separate functions -
  new relative to both `vita-2048-eu-v104` and `ps3-hdfury-eu`, where all
  four are their own functions.
  [`docs/ghidra/functions/ps4-omega-eu/weapons.md`](../../docs/ghidra/functions/ps4-omega-eu/weapons.md)
  transfers nine names from `ps3-hdfury-eu/weapons.md` by `.cpp` tag
  (`QuakeManager_Construct`, `Rocket_Construct`, `RocketManager_Construct`,
  `MissileManager_Construct`, `EMPManager_Construct`, `BombManager_Construct`,
  `CannonManager_Construct`, `LightBarrierManager_Construct`,
  `PlasmaManager_Construct`) and records a negative result on
  `find_similar_functions_fuzzy`/`bulk_fuzzy_match` as a cross-architecture
  naming shortcut: real on the PS3↔PS4 pair only one time out of two probes,
  pure noise on Vita↔PS4 even seeded from a confirmed match, and
  `bulk_fuzzy_match`'s own `filter` argument does not appear to restrict its
  candidate set. The `.cpp` debug-tag technique stays the reliable one for
  this binary's architecture spread. Two more classes turned out to have no
  function of their own to name at all: `WeaponExplosions_Construct` (its
  own function on `ps3-hdfury-eu`) and `MineManager_Construct`/
  `LeachBeamManager_Construct` are all inlined into larger owning functions
  here instead, the same shape `game-boot.md` already found for
  `Game_Main`'s own manager constructors.
  `weapons.md` also names `RaceManager_Construct` (87, the base race-rule
  root), `ModeManager_ConstructByMode` (80, a single dispatcher building any
  of twelve per-mode `ModeManager` subclasses from one shared body - first-
  in-this-project evidence that neither HD nor 2048 name their own per-mode
  subclasses at all), and - unlike `ModeManager` - all sixteen `_RaceManager`
  subclasses as **separate** functions (`AIBatchRaceManager_Construct`,
  `DemoRaceManager_Construct`, the `MP*`/`SP*` family, `SPZoneRaceManager_Construct`,
  `GameModeRaceManager_Construct`) plus the intermediate
  `MPRaceManager_Construct` the four `MP*RaceManager` leaves call instead of
  the base directly - each proven by an explicit call to its own base
  constructor, not tag matching alone.
  `docs/ghidra/functions/ps4-omega-eu/weapons.md#gamemodes-is-a-real-distinct-directory---not-a-rename`
  resolves this thread's own open `GameModes/*.cpp` question along the way
  (it is a distinct `GameModes/` directory, not a rename of `Backend/General/`'s
  own tags - both exist side by side, and `GameModeRaceManager_Construct` is
  a real, separate, decompiled function).
- **`.rcsmodel`/`.rcsmaterial`'s PS4 container itself, past the tag.** This
  session (`lane/omega-rcs`, 2026-09-16) found and named the tag
  (`0xCA5CADED`/`0xCA5CADE5`) and confirmed the payload past it still hashes
  material names in PS3's own `~crc32` namespace, but left the container's
  own fields unread beyond an offset table shape, a constant per-block tag
  (`0xe35e00df`, `0xe9f17935` - checked against ~40 plausible type-name
  candidates, no hit) and one file's embedded node name (`OutlineShape`) and
  4x4 identity matrix. Reading it for real needs more samples than one
  session found tag-valid *and* present at an identical PS3 path (only
  `detonator/ship_lod.rcsmodel` qualified), and ideally a PS4 executable
  reader to compare against - `eboot.bin` has 20,941 functions and none
  named yet for asset loading of this specific type. See
  [rcsmodel.md](../../docs/formats/rcsmodel.md#the-ps4-omega-collection-a-different-container-not-a-byte-swap-of-this-one)
  for exactly what is and is not measured.

## From the HANDOVER.md index (moved 2026-09-25)

`eboot.bin` imports clean via GhidraOrbis (20,941 functions; 38 named as of 2026-09-15 - `MagstripWake_Construct`, the ship-collision-fx pair, the boot chain, nine weapon-manager constructors, the full `RaceManager`/`ModeManager` family including all sixteen `_RaceManager` subclasses, two race-family helpers (`RaceManager_ConstructArcadeHud`, `NitroRaceManager_ConstructTuning`), and `Billboard_ConstructResource`/`TrackStartup_Load`, mostly transferred from `vita-2048-eu-v104`/`ps3-hdfury-eu` by `.cpp` debug tag; the HUD-selection function also resolved a `vita-2048-eu-v104` open question about two unreached HUD skins, the billboard match's exact magic-number agreement raised `ps3-hdfury-eu`'s own confidence on the same function past its naming floor, and `TrackStartup_Load` resolved `ps3-hdfury-eu`'s own long-open `mode_descriptor` question) and confirms the same `Backend/...` codebase lineage a third time. The five `dataNN.psarc` asset archives have a NUL-delimited manifest and digest-based (not positional) entry correspondence, both now resolved and landed, along with a corrupt-row bug that left two of the five archives unparseable at all - **not a property of the declared container version** (both fixes first dispatched on `version_minor >= 4`, which broke Vita `2048`'s own `data.psarc` on `main` for one merge, since it also declares 1.4 but is the well-behaved PS3 shape throughout; both functions now read the archive's own bytes instead, caught and fixed same-session). What's still open, and bigger: a resolved entry's declared bytes read as real content roughly a third to two-thirds of the time (30-69%, varying by archive) on the PS4 archives specifically, and zero the rest, with no predictor found yet for which. **The extraction-provenance lead is closed, negative, 2026-09-15**: `PkgTool.Core pkg_extract` has no base/patch merge logic (checked against source), and extracting the patch `.pkg` separately turns up four archives (`data05`/`07`/`08`/`09`, no `data06`) the base `.pkg` doesn't have at all rather than corrected copies of `data00`-`04` - and the same real/zero split reproduces at similar magnitude on those four, freshly extracted. Split into [omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md](../tooling/omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md), which found none of the four are DLC
