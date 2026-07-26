# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-26, at commit `8abef86`.

## In flight

**A front-end agent is building `crates/game/`** (`oag-game`): the boot sequence,
the intro movie, and the Language Selection screen, driven by the original's
string-keyed state machine. It was told to own `crates/game/` and the workspace
`members` list and to leave everything else alone, so if that crate is present
but unfinished, that is where it stopped. PSP `.PMF` is H.264 plus ATRAC3+, so
expect the demuxer to land before anything decodes.

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
2. **The confidence scores systematically claim 95+ for work nobody has run.** I
   amended the [rubric](docs/reverse-engineering/confidence-rubric.md) to say
   what data validation actually buys (ceiling 94) instead of leaving the top band
   decorative, and scored the track work by it. I did **not** rescore other
   people's pages: `formats/wad.md` (95 and 97),
   `formats/psp-texture.md` (95), `formats/vex.md` (95) and `physics/README.md`
   (97 on a negative claim from static reading). `formats/lzss.md` is done: it is
   94 now, for the reason the rubric gives.
3. **ADR-0006's "no game content" rule is applied inconsistently.** Several
   format pages quote real bytes, palettes and string tables, which the ADR as
   written forbids since it explicitly rejected the de-minimis argument. Most of
   it is defensible as format documentation, so the ADR probably needs a stated
   carve-out for structural excerpts rather than the pages needing trimming. That
   is a call for the developer.
4. **`vex::textures` skipping an unsupported depth shifts every later texture
   index**, because `mesh.rs` resolves materials positionally into that vector.
   One skipped texture mis-assigns the rest, and the `filter` in `mesh.rs` turns
   the overflow into a silent untextured draw.

## What is deliberately not done

These are choices, not oversights.

| Not done | Why |
| --- | --- |
| Models in the window | `oag-view` shows models and tracks via `--screenshot` only. Putting them in the window needs an orbit camera; the refactor was started and abandoned, so `mesh_render.rs` is offscreen-only. |
| Track art meshes | `--track` draws the spline ribbon, not the 594 `Mesh` nodes. Assembling those needs `Transform` payloads and the scene hierarchy, which are undecoded. That is the next piece of M1. |
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

Every format decoded here had a self-check available, and using it has caught
real errors five times:

- **WAD**: the offset chain. Testing both orderings of the size fields gave
  193/193 against 2/193, which resolved a transposition.
- **`.vex` geometry**: the mesh's own declared bounding box, to 0.15% of extent.
  This caught `child_count` being a `u32` and the node stride being variable.
- **Embedded textures**: the sizes sum exactly to the declared block length.
- **Track**: exactly 64 `section` nodes, matching the 64-bit PVS mask.
- **`WO Track`**: the payload has to close. `0x20 + 0x20 + 0x20*paths +
  0x10*junctions + 0x70*points` equals the payload length on **40 of 40** track
  files, which is what found the reserved block the previous pass had missed.

**Look for the arithmetic that must hold before writing the parser.** It is
usually there, and it converts a plausible reading into a determination. When you
find one, put it in a test rather than in prose: the `.vex` bounding-box check
above was a one-off measurement written up as if it were a standing guarantee,
and the review caught that. `crates/formats/tests/track_ground_truth.rs` is the
shape to copy, including `OAG_REQUIRE_GAME_DATA=1` so an absent disc image fails
instead of skipping green.

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

1. **Decode `Transform` payloads and the scene hierarchy**, then render a
   track's art meshes. That finishes M1's exit criterion, and `--track` already
   proves the spline half of it.
2. **Confirm the image base** against PPSSPP before more addresses are written
   down.
3. **Runtime verification** of anything in the physics documentation, which is
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
