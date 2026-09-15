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

**Still open**: the manifest (entry 0, the one whose digest is sixteen zero
bytes) is **NUL-delimited on this archive family, not newline-delimited**.
Measured directly on `data00.psarc`'s manifest (598,799 bytes): exactly one
`\n` byte (the trailing one), zero `\r`, and **22,581 `\x00` bytes**.
`parse_manifest`'s `.lines()` split finds none of those, so
`Archive::paths()` currently returns the *entire manifest blob* as one
"path". Splitting on NUL instead recovers 22,582 well-formed paths
(`Data/art/published/hdships/ag_systems/fe/Logo.2x.gnf`, ...) - the same
`Data/...` namespace `ps3-hdfury-eu` uses, `.gnf` in place of PS3's `.gtf`
(Sony's PS4-native texture container - **no reader exists for it anywhere
in this project**, checked directly: zero hits for `gnf` across
`crates/texture`, `crates/formats`, `docs/formats/`).

**The open half of this, not yet explained**: the header's own `entry_count`
for `data00.psarc` is 10,927, but the NUL-split manifest yields 22,582
paths - roughly double, not the `entry_count - 1` a PS3-shaped manifest
would produce. Not chased further this session; the entry table's own
per-entry layout (still 30 bytes, still parses sanely for entries 0-2 by
hand) may carry two entries per file on this version, or the manifest may
enumerate files the entry table addresses differently. Whichever it is,
`Archive::open`/`Directory::parse` need it settled before file-level
extraction (`entry_range`/`read_entry`) can be trusted on this archive
family - right now they would use PS3-shaped assumptions on a table that
looks structurally similar but is not proven to line up 1:1.

## Open

- The `entry_count` (10,927) versus manifest-path-count (22,582) mismatch on
  `data00.psarc` - the actual blocker for real asset extraction, described
  above. Needs either a second archive's manifest compared the same way (do
  the smaller ones, e.g. `data03.psarc` at 662 declared entries, show the
  same ~2x ratio, or is it content-dependent?) or a byte-level diff between
  what the entry table's block-count arithmetic predicts per file and what
  the manifest lists.
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

- Resolve the manifest/entry-count mismatch (see Open, first item), then
  extend `oag_formats::psarc::parse_manifest` (or add a PS4-specific
  variant) to split on NUL instead of `\n` for archives declaring version
  1.4 - `Header` already carries the version fields, so the container can
  self-select the delimiter rather than needing a caller flag.
- Once extraction works, `psarc_list`/`psarc_cat`
  (`crates/assets/examples/`) already work unmodified against a 1.4 archive
  - re-run them for a full per-archive file/extension census (this session
  only got `data00.psarc`'s raw manifest text, not a parsed listing) and
  decide whether `data03.psarc` (662 entries, by far the smallest) is a
  distinct content package (DLC-shaped) worth checking against
  `data/dlc/` before assuming it is just "more of the same".
- Not a reverse-engineering target in its own right yet (Omega Collection
  stays "if feasible" in the roadmap) - this thread is groundwork, not a
  milestone. The natural next RE pass, if one happens, is `GameRoot`/
  `Game_Main`-equivalent boot-chain naming, the same starting point
  `vita-2048-eu-v104/game-boot.md` used, now that the binary imports cleanly
  and the lineage match gives it a same-role function to look for by name.
