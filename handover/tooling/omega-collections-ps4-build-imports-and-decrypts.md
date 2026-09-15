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

- **Block data location** (see above) - entries resolve to real paths now,
  and roughly a third to a half of them already read real content through
  the unmodified reader, but which third/half is not yet predictable, on
  any archive checked. This blocks trustworthy real asset extraction more
  fundamentally than the (now-resolved) manifest mismatch did - a caller
  cannot yet tell a real read from a zero one without comparing against a
  known-good reference. **The extraction-provenance lead (whether the base
  `.pkg` extraction had silently absorbed, or needed, patch content) is
  closed, negative, 2026-09-15**: `PkgTool.Core pkg_extract` has no
  base/patch merge logic at all (checked directly against its source), the
  patch's own archives are four names (`data05`/`07`/`08`/`09`) the base
  `.pkg` doesn't have rather than replacements for `data00`-`04`, and the
  same real/zero split reproduces at similar magnitude on those four,
  freshly extracted this session - see
  [omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md](omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md),
  split out to hold that finding. What remains open is a from-scratch
  reading of what distinguishes a real entry from a zeroed one - not
  attempted this session either, per the standing rule against guessing a
  fix without a verified cause, but now ruled out as an extraction
  artifact rather than merely unexplained.
- `GameModes/*.cpp` (`GameMode_ModeManager`, `GameMode_RaceManager`,
  `GameMode_TournamentModeManager`) has no obvious Vita/PS3 counterpart in
  the `.cpp`-path census - worth checking whether Vita's own
  `GameModes/GameMode_RaceManager.cpp` (present in its own string table per
  `vita-2048-eu-v104/README.md`'s own comparison table) is the same file
  under a path this census missed, before concluding PS4 added a layer.
- `.gnf` (PS4's native texture container) has no reader in this project at
  all. Some real `.gnf` bytes can already come out of a PSARC (the
  real/zero split below means a specific one might or might not), so this
  is a live gap rather than a blocked one - the next format gap to close
  once the real/zero split is understood, the same role `.gtf`/`.gxt` fill
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
  question, and not an extraction-provenance question either (that lead is
  closed - see Open). A from-scratch reading of what distinguishes a real
  entry's bytes from a zeroed one is the only avenue left; nothing tried so
  far (stored-block shape, file position, a base/patch merge) predicts it.
- Once the real/zero split is understood (or at least detectable per-entry),
  `psarc_list`/`psarc_cat`/`psarc_sweep` (`crates/assets/examples/`) already
  work against a 1.4 archive's directory and manifest - re-run them for a
  full per-archive file/extension census and decide whether `data03.psarc`
  (328 real entries, by far the smallest of the base `.pkg`'s five) is a
  distinct content package worth checking against `data/dlc/` before
  assuming it is just "more of the same". **The same census on the patch's
  own four archives is done and says no** -
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
  this is corroboration groundwork, not a milestone. The natural next RE
  pass is `GameRoot`/`Game_Main`-equivalent boot-chain naming, the same
  starting point `vita-2048-eu-v104/game-boot.md` used, now that the binary
  imports cleanly and the lineage match gives it a same-role function to
  look for by name.
