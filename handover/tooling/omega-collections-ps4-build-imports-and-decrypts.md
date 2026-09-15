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
value, 13, across all five). Two real differences found, one fixed this
session, one still open:

**Fixed**: the block-width probe (`read_block_table`) computed its "highest
referenced block" across every entry unconditionally, including entries
with `size == 0`, whose `first_block` field is unspecified. `data00.psarc`
entry 9042 (`size` and `offset` both `0`) carries `first_block =
0x960c4925` (2,517,387,557) - large enough that no block-table width could
ever "cover" it, so the whole archive read as an unrecognised layout. Fixed
by applying the same `size > 0` filter the out-of-range check one function
below it already uses. Landed this session (`crates/formats/src/psarc.rs`),
all 12 existing unit tests still pass.

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
"Version 1.4" section. Landed in `crates/formats/src/psarc.rs`
(`Header::nul_delimited_manifest`, NUL-aware `parse_manifest`,
`match_paths_to_entries`) and `crates/assets/src/psarc.rs`; `.gnf` (Sony's
PS4-native texture container) still has **no reader anywhere in this
project** and is unaffected by this fix.

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

**Still open, and more serious than the manifest question was**: reading an
entry's declared bytes does not currently produce its real content. Checked
on `data03.psarc` across 53 sampled entries: zero had a zlib `0x78` marker at
the position `entry.offset` names, and zero at the position an alternative
(block-table-cumulative-sum) reading predicts instead. A targeted check
around one entry found 512 bytes of literal, non-sparse zero surrounding its
declared offset, and no zlib stream in a 10 MB window around it decompresses
to that entry's declared size with its format's expected magic. One
unverified lead: `docs/reverse-engineering/source-images.md`'s own PS4
extraction command reads only the base `.pkg`, not the patch, and records
~25 GiB across the five archives where this session's own `data/extracted/`
measures ~40.8 GiB - whether the current extraction folded in
`omega-ps4-eu-patch.pkg`, and whether that merge was clean, is not
established. See `docs/formats/psarc.md`'s "Block data location" section for
the full measurements and exact repro commands.

## Open

- **Block data location** (see above) - entries resolve to real paths now,
  but reading their declared bytes does not yet produce real content on any
  archive checked. This blocks real asset extraction more fundamentally than
  the (now-resolved) manifest mismatch did. Needs either the PKG
  extraction re-verified/redone with both `omega-ps4-eu.pkg` and
  `omega-ps4-eu-patch.pkg` correctly merged, or a from-scratch reading of
  what `entry.offset`/`first_block` actually encode on this family - not
  attempted this session per the standing rule against guessing a fix
  without a verified cause.
- `GameModes/*.cpp` (`GameMode_ModeManager`, `GameMode_RaceManager`,
  `GameMode_TournamentModeManager`) has no obvious Vita/PS3 counterpart in
  the `.cpp`-path census - worth checking whether Vita's own
  `GameModes/GameMode_RaceManager.cpp` (present in its own string table per
  `vita-2048-eu-v104/README.md`'s own comparison table) is the same file
  under a path this census missed, before concluding PS4 added a layer.
- `.gnf` (PS4's native texture container) has no reader in this project at
  all. Not urgent - nothing can extract a real `.gnf` file out of a PSARC
  yet regardless, per the manifest issue above - but worth noting as the
  next format gap once extraction works, the same role `.gtf`/`.gxt` fill
  for PS3/Vita.
- Ghidra's own `analyzed` flag reads `false` on `/ps4-omega-eu/eboot.bin`
  even though `analyzing` is `false` and the function count (20,941) has
  been stable across repeated checks - not chased further, likely a
  metadata quirk of triggering analysis through the bridge rather than
  Ghidra's own GUI dialog, same as noted for the Vita programs in
  [2048s-vita-eboots-are-imported-re-not-started.md](2048s-vita-eboots-are-imported-re-not-started.md).

## Next Steps

- Resolve the block-data-location problem (see Open, first item) - this is
  the actual blocker for real asset extraction now, not the manifest
  question. Start with the extraction-provenance lead: re-run
  `PkgTool.Core pkg_extract` against `omega-ps4-eu-patch.pkg` (not just the
  base `.pkg`) per `docs/reverse-engineering/source-images.md`'s existing
  command, and compare the resulting `dataNN.psarc` sizes and a spot-checked
  entry's bytes against this session's measurements before assuming the
  container format itself needs more reverse-engineering.
- Once real extraction works, `psarc_list`/`psarc_cat`
  (`crates/assets/examples/`) already work against a 1.4 archive's directory
  and manifest - re-run them for a full per-archive file/extension census
  and decide whether `data03.psarc` (328 real entries, by far the smallest)
  is a distinct content package (DLC-shaped) worth checking against
  `data/dlc/` before assuming it is just "more of the same".
- Not a reverse-engineering target in its own right yet (Omega Collection
  stays "if feasible" in the roadmap) - this thread is groundwork, not a
  milestone. The natural next RE pass, if one happens, is `GameRoot`/
  `Game_Main`-equivalent boot-chain naming, the same starting point
  `vita-2048-eu-v104/game-boot.md` used, now that the binary imports cleanly
  and the lineage match gives it a same-role function to look for by name.
