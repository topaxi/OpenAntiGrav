# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-26, at commit `eb9a30d` plus uncommitted doc changes from this
session (not yet committed; see below for what they are).

## In flight

Nothing code-side. `just check` passes 297 tests; `just test-data` adds the 14
ground-truth ones that need `data/images/` and passes 311. This session touched
only `docs/`, so those numbers are unchanged.

**This session closed two of the three open items from the previous handover**
(confidence rescoring, and drafting the ADR-0006 question) and made another
negative-result pass at the `.fnt` atlas. Details below, in place of the
previous handover's "still open" list.

1. **Confidence rescoring is done** for the four pages the previous handover
   named. `formats/wad.md`'s two claims (95, 97) are now 94 each;
   `formats/psp-texture.md`'s header/palette/layout claim (95) is 94;
   `formats/vex.md`'s node-tree claim (95) is 94 — all four were data-agreement
   evidence (an exact arithmetic invariant across many real files), which the
   rubric caps at 94. `physics/README.md`'s float-vs-fixed-point claim (97) is
   now **92**: unlike the other three it has no cross-file data agreement, so
   it does not get the data-agreement ceiling, but it is an exhaustive negative
   over the whole craft path with every call site consistent — exactly what the
   rubric's 85-94 band describes — and scoring it below its neighbours on the
   same page (90, 91) for weaker evidence would have been worse than leaving it
   at 97. Each page now states which evidence class it is and why, the way
   `lzss.md` already did. No other pages were touched — the remaining
   90/91/85/86/82 scores on `physics/README.md` and the 92/90 on `vex.md` were
   not in scope and were left alone.
2. **The `.fnt` atlas layout is still not resolved**, but four more transform
   families are now ruled out and recorded in `fnt.md`: column-major reading, a
   four-bitplane decomposition, a Morton/Z-order curve inside blocks, and an
   alpha-weighted re-scoring of the whole prior block-swizzle sweep (in case the
   boolean "index nonzero" oracle was drowning in antialiasing dust rather than
   the transform being wrong — it wasn't; the ranking barely moved). None of
   this narrows the search. The exploration script lives at
   `/home/topaxi/.claude/jobs/f60165c3/tmp/fnt/` in this session's job
   directory, not in the repo — it is throwaway, not a fixture, so it was not
   committed. Worth knowing if picking this up again: the palette entries this
   session decoded for the "ink" indices (12-15) are not monotonic in alpha for
   at least two of the five fonts, which is either meaningless or a clue about
   how the index-to-palette mapping works; not chased further because it is a
   different question from the pixel layout.
3. **ADR-0006's carve-out is drafted, not applied.** A survey of `docs/`
   (below) found no raw asset-content dumps — no texture, audio or model bytes
   — only header/magic-number hex, path and name strings, and one small set of
   decoded palette-alpha values, all used as evidence for a specific claim
   rather than as content. A candidate carve-out sentence:

   > A worked example limited to what a specific claim needs — a header's raw
   > bytes, a handful of resolved path strings, a small decoded palette — is
   > format documentation, not game content, provided it could not stand in for
   > (or be reassembled into) the original asset.

   This is a proposal for the developer to accept, reject or rewrite, not a
   change to the ADR. The survey, by file:
   - `formats/psp-texture.md`: a 16-byte header hex comparison, two decoded
     RGBA-quad sequences from real UI textures. Minimal.
   - `formats/wad.md`: one 16-byte header hex dump, a 3-row hash worked-example
     table, and disc paths quoted inline throughout. Mostly minimal; the
     accumulation of real paths across the page is the closest thing to a
     borderline case here.
   - `formats/pmf.md`: standard MPEG PES marker bytes (not game-specific) plus
     3 real movie filenames. Minimal.
   - `formats/vex.md`: a 16-row table of `sprintf` path templates recovered
     from the binary, plus 7 literal team-name strings. Borderline — small but
     a real string-table excerpt.
   - `formats/fnt.md`: a 5-row table of real font filenames and dimensions.
     Minimal.
   - `formats/fexml.md`, `formats/track.md`: short real XML snippets
     illustrating the shortening scheme. Minimal.
   - `formats/handling-stats.md`: explicitly reproduces no values. Compliant
     already.
   - `psp/pulse-disc-layout.md`, `ps2/pulse-disc-layout.md`: full real
     file/directory listings (names, sizes, entropy) and a few header hex
     sequences. The most extensive real-path content on the site, but it is
     directory metadata, not payload — arguably the strongest test case for
     wording the carve-out precisely.
   - `ghidra/functions/psp-pulse/wad-subsystem.md`,
     `frontend-video.md`: real paths quoted as hash-verification examples.
     Minimal.

## The code review has landed, and most of it is done

The review the previous handover was waiting for arrived, and it was good: it
built and ran proofs of concept rather than reading. Its correctness findings are
**fixed**, each with the test that would have caught it. What remains is listed
below, and it is the part that needs judgement rather than typing.

Fixed, for the record: the RNG (see below), the 4bpp texture panic, the CHD
divide-by-zero, two unbounded allocations, `safe_join`'s missing tests and its
Windows colon leak, `vex::nodes`' depth calculation, the circular `TickClock`
drift test, `from_16_16`'s false exactness claim, three stale comments, and the
tracked Ghidra lock files.

**The RNG one is worth knowing about even though it is fixed.** `Rng::next_u32`
was not xoshiro128\*\*: it wrote the state update as one parallel assignment
where the reference reads two already-updated words, which dropped two of four
terms and took the transition matrix from rank 128 to 119. A non-bijective step
loses state every call, so the period was not what the type claimed. Every test
passed, because every test compared the generator against itself. The determinism
gate could not see it either: it catches a generator that *drifts* from a
recorded baseline, not one that was wrong before the baseline was taken. The
lesson generalises past the RNG, and it is why the reference vector in `rng.rs`
says not to regenerate it from the implementation.

The WAD compression rule and the LZSS verification claim are also resolved, and
both turned out to be worth the trip. The rule is **two** tests, not one: equal
size fields mean stored, whatever bit 31 says, and only a compressed entry
consults the flag. And `verify` was tautological, but a real oracle was available
next to it: every one of the 6,053 LZSS streams is consumed to within one or two
bytes of its end, never zero and never three, which the decoder cannot arrange
for itself. Details in [lzss.md](docs/formats/lzss.md).

Still open, and each needs a decision rather than a patch:

1. **LZSS is still tested only by its own inverse.** The encoder in its test
   module was written to match the decoder, so a wrong field order would pass
   both. The consumption check above is now the suite's only independent oracle,
   and it constrains the layout without pinning it.
2. ~~The confidence scores systematically claim 95+ for work nobody has run.~~
   Resolved this session for the four named pages; see "In flight" above for
   the new scores and why.
3. ~~ADR-0006's "no game content" rule is applied inconsistently.~~ A carve-out
   is drafted this session; see "In flight" above. Landing it is still a call
   for the developer.
4. ~~`vex::textures` skipping an unsupported depth shifts every later texture
   index.~~ Fixed: both it and `mesh_materials` return one slot per node now.

A lesson from this session worth keeping, because it turned up three times in
three different forms. **Writing something down does not make it right, and the
more confidently it is written the longer it survives.**

- `child_count` was documented as a `u32`, on the strength of running the decoder
  against a real file. It is a `u16`, and the 32-bit read is correct on 97% of
  nodes, which is exactly why it lasted.
- The `.vex` frame axis was documented as `up`. It points down.
- The font's `cell()` had a test asserting that `Français` measured one glyph
  *shorter* than `Francais`. That is the bug written down as an invariant, and a
  test in that shape actively defends the behaviour it should be catching.

In the first two the *original* reading was closer than the correction. In all
three, what settled it was an invariant that has to hold arithmetically, not a
more careful read of the same code. Prefer those, and when you find one, put it in
a test.

## What is deliberately not done

These are choices, not oversights.

| Not done | Why |
| --- | --- |
| Models in the window | `oag-view` shows models and tracks via `--screenshot` only. Putting them in the window needs an orbit camera; `--yaw` and `--pitch` stand in for one. |
| The game's own font | `oag-game`'s language picker uses a 5x7 font of ours. The `.fnt` metrics decode and validate; the atlas's pixel layout does not. The five language names render correctly, accents included, but the type is not Pulse's. See [fnt.md](docs/formats/fnt.md). |
| Front-end widget offsets | Screen XML coordinates are absolute within the real widget tree, and we do not apply parent offsets, so anything authored near an edge lands slightly outside it. Text is nudged back into the viewport as a stopgap, and it is commented as one. |
| Audio | ATRAC3+ frames are demuxed and handed over intact. Nothing decodes them. |
| Batch list B | `oag-view` renders only list A of each mesh, to avoid drawing surfaces twice. That may be dropping a legitimate second pass (reflections, decals). |
| Transparency sorting | `Glass_ADD.tga` implies additive surfaces that currently draw opaque. The `pass_mask` bits needed are documented in [vex.md](docs/formats/vex.md). |
| Mip levels | Decoded but unused. |
| `section` payloads | Documented, not implemented. They are visibility only, so nothing renders differently without them. |
| The 29 unresolved import stubs | 16 are `sceNp*`, whose NIDs are not SHA-1 of the name, so they need a published table rather than a hash. |

## Method that is not obvious

### Symbol names are reproducible now

The Ghidra database is not committed, so a fresh import starts at `FUN_08940d0c`
again. It does not have to:

```sh
just apply-names        # 400 symbols into whichever program the bridge has open
```

94 come from [`names.tsv`](docs/ghidra/functions/psp-pulse/names.tsv), which is
hand-maintained next to the evidence pages, and 306 are import stubs re-derived
from the binary itself. `scripts/apply-ghidra-names.py` refuses any row whose
address and name are not both still on its evidence page, so ADR-0005's "no
rename without documentation" is enforced rather than trusted.

**Two traps in that bridge.** Its `dry_run` parameter is *not* honoured by the
rename endpoints: it reports what it would do and does it anyway, which is how
the first name got applied by accident. And it rejects global names that lack a
Hungarian type prefix, which is not this project's convention, so data addresses
go through `create_label` instead of `rename_or_label`.

### Naming imports is a check, not a guess

A PSP import NID is the first four bytes of `SHA-1(name)`, little-endian. That is
one-way, so hash the candidate names instead and keep the matches: a wrong name
cannot produce the right NID. 306 of 335 stubs resolve this way, and the rule was
confirmed first against two NIDs the project had already recovered independently.
See [imports.md](docs/ghidra/functions/psp-pulse/imports.md). Adding a name to
`scripts/psp-import-names.txt` and re-running is the whole workflow.

### Finding entries by name

WADs store only a CRC-32 of each name, and most names are built at runtime from
templates, so they are not in the executable. `scripts/mine-names.py` rebuilds
the candidate list and gets **292 of 1,142** entries in `Data.wad`:

```sh
just mine-names data/images/pulse-psp-usa.chd
```

The step that matters is the third one. **Track directories are numbered**
(`01_Track`), not named after the track, so no amount of guessing finds them.
`Data\Plugins\PI001\Definition.xml` lists every track with a `location`
attribute; that plus the binary's `%s\%strack%s.vex` template is what makes
track files reachable at all.

The output is gitignored, since it is derived from your disc. Regenerate it
rather than looking for it.

### Validating a decoder

Every format decoded here had a self-check available, and using it has caught real
errors seven times, and confirmed a format outright twice more. That is the
single most useful habit in this repository:

- **WAD**: the offset chain. Testing both orderings of the size fields gave
  193/193 against 2/193, which resolved a transposition.
- **`.vex` geometry**: each batch's own declared bounding box. Vertices must fall
  inside it, so a wrong stride or scale shows immediately, and with a misread
  colour format the position bytes come out of the middle of a colour. This is
  what validated the 16-bit colour formats the tracks use, over 3,181 batches.
- **`.vex` node tree**: immediate child counts must sum to one less than the node
  count. Exact on every file as a `u16`; 111 million for 2,071 nodes as a `u32`,
  which is how the field's width was settled.
- **Embedded textures**: the sizes sum exactly to the declared block length.
- **Track**: exactly 64 `section` nodes, matching the 64-bit PVS mask.
- **`WO Track`**: the payload has to close. `0x20 + 0x20 + 0x20*paths +
  0x10*junctions + 0x70*points` equals the payload length on **40 of 40** track
  files, which is what found the reserved block the previous pass had missed.
- **LZSS**: where the *reader* stops. The decoder halts on output length, so a
  size match proves nothing, but every one of the 6,053 streams is consumed to
  within one or two bytes of its end. It cannot arrange that for itself.
- **PMF**: three at once, from the front-end agent. Header arithmetic exact, zero
  stray bytes when demuxing, and access-unit counts matching the presentation
  duration at 30000/1001 Hz.

**Look for the arithmetic that must hold before writing the parser.** It is
usually there, and it converts a plausible reading into a determination.

**Then put it in a test, not in prose.** The `.vex` bounding-box check spent
months as a measurement someone took by hand and wrote up as though it were a
standing guarantee; the review caught that, and it is now an assertion over
342,115 vertices. `crates/formats/tests/track_ground_truth.rs` is the shape to
copy, including `OAG_REQUIRE_GAME_DATA=1` so an absent disc image fails instead of
skipping green.

### Ghidra

`BOOT.BIN` must be loaded with the **Allegrex** processor module, not stock
`MIPS:LE:32`. Stock Ghidra silently decodes VFPU instructions as nonexistent
64-bit MIPS III ones and the decompiler builds confident, fictional C from
them. `just build-allegrex` produces the extension; see
[allegrex-vfpu.md](docs/psp/allegrex-vfpu.md).

The image base is `0x08804000`. That is the conventional PSP module load
address and **has not been confirmed against PPSSPP** - the one outstanding
item that needs the developer.

The Ghidra project currently lives in the repository root (`OpenAntiGrav.gpr`),
not in `data/ghidra/` where [workflow.md](docs/ghidra/workflow.md) says it
should. That is why its lock file leaked into git. Moving it is safe once the
project is closed.

## Provenance and how much to trust it

Most reverse-engineering findings came from background agents reading Ghidra.
They were instructed to report evidence and score confidence, and the scores in
`docs/` are theirs unless a page says otherwise.

**Independently verified against real data:** the WAD name hash, the size field
ordering, LZSS, texture decoding, `.vex` geometry and textures, the `section`
count, the whole `WO Track` layout, the junction slot ordering, the import stub
names. These are the claims to lean on.

**Single-source static reading, never executed:** the physics force law, the
collision query path, the frontend state machine, the video path. All plausible,
none runtime-verified. Nothing in this project has been run under an emulator,
which is why nothing should be scored above 94.

## Where I would go next

1. **The `.fnt` atlas layout.** No longer urgent for correctness, since the
   picker reads properly with our own glyphs, but it is the last thing between the
   front end and Pulse's own type. `fnt.md` called this "the same unresolved
   problem as `.mip` swizzling"; narrowed this session, not resolved.
   `psp-texture.md` only answers whether `.mip` pixel data is swizzled *in the
   file* (no), and says outright that whether the PSP swizzles at upload time
   is still open. So `.mip`'s file-storage answer is not a lead for `.fnt`, but
   the two could still be the same upload-time transform — that question is
   open on both sides, not closed. This session also ruled out four more
   transform families for `.fnt` itself (column-major, bitplanes, Morton order,
   and an alpha-weighted oracle) without finding it; see `fnt.md` for the full
   list of what has been tried.
2. **Confirm the image base** against PPSSPP before more addresses are written
   down.
3. **Migrate the three older copies** of the disc-to-blob pipeline onto
   `oag-assets`, which now exists but is asset access only, not yet the
   platform-normalising registry [workspace-layout.md](docs/architecture/workspace-layout.md)
   describes.
4. **Runtime verification** of anything in the physics documentation, which is
   the M3 harness in miniature and would let the top confidence band mean
   something.

## Traps that cost time here

- **`python` string-replace patching of code already edited** fails silently and
  ships a build where a feature does nothing. Caught only because output said
  "0 textures" when 13 were known. Read the file and use a real edit.
- **A `.gitignore` pattern of `/*.lock`** silently swallows `Cargo.lock`, and
  `git check-ignore` will *not* warn you because the file is already tracked.
  Use `--no-index` to test ignore rules.
- **wgpu 30** moved presentation to `Queue::present`, push constants to
  `immediate_size`, `multiview` to `multiview_mask`, and made
  `get_current_texture` return an enum rather than a `Result`.
- **The Gradle wrapper for the Allegrex build needs JDK 21**, not the system
  default. The failure does not mention the JDK.
- **`ls -la` fails in this shell** (aliased); use `eza -l`. `eza` also prints
  nothing useful for a bare directory listing in a pipe, so `fd -d 1` is more
  reliable in scripts.
