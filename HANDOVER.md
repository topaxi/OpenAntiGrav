# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-26, at commit `e96c208`.

## In flight

**A code review is running as a background agent** and its results are not yet
in. It was asked to be adversarial about decoder correctness, panics on hostile
input, honesty of confidence scores, test quality, and the legal boundary. When
it lands, treat its findings as claims to verify, not as facts: it was told to
find problems, so it has an incentive to manufacture them.

If that review never arrives, it is worth re-running. The specific worry it was
pointed at is **unchecked slicing in the format parsers**, which all read
untrusted files.

## What is deliberately not done

These are choices, not oversights.

| Not done | Why |
| --- | --- |
| Models in the window | `oag-view` shows models via `--screenshot` only. Putting them in the window needs an orbit camera; the refactor was started and abandoned mid-session, so `mesh_render.rs` is offscreen-only. |
| Track spline decoder | The file layout is **not** resolved; see below. Writing a decoder on the current guess would be building on sand. |
| Batch list B | `oag-view` renders only list A of each mesh, to avoid drawing surfaces twice. That may be dropping a legitimate second pass (reflections, decals). |
| Transparency sorting | `Glass_ADD.tga` implies additive surfaces that currently draw opaque. The `pass_mask` bits needed are documented in [vex.md](docs/formats/vex.md). |
| Mip levels | Decoded but unused. |
| Ghidra renames | ~200 names proposed across `docs/ghidra/functions/psp-pulse/`, **none applied**. [ADR-0005](docs/architecture/adr/0005-ghidra-conventions.md) wants the evidence committed first, which it now is, so applying them is unblocked. |

## The one thing most likely to mislead

**The track spline structures in [`docs/formats/track.md`](docs/formats/track.md)
are the *runtime* layout, not the file layout.**

The `WO Track` header validates against a real track (magic `WOtd`, version
`0x105`, 2 paths, 2 junctions). But the path and junction offsets are **zero in
the file** — runtime pointers patched at load, the same pattern as embedded
textures. The obvious rule of packing them sequentially after the header does
**not** hold: it puts world coordinates at the `up` field, produces a
`section_id` of 185 on a 64-section track, and leaves 30 KB unaccounted for.

Do not implement from those structs without resolving that first. The likely
approaches are reading `AiTrack_ParsePayload` (`0x0887c57c`) properly, or
scanning the payload for where plausible float positions begin.

## Method that is not obvious

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

Every format decoded this session had a self-check available, and using it
caught real errors three times:

- **WAD**: the offset chain. Each blob starts at the previous blob's end,
  aligned. Testing both orderings of the size fields gave 193/193 against
  2/193, which is what resolved a transposition.
- **`.vex` geometry**: the mesh's own declared bounding box. Decoded vertices
  agreed to 0.15% of extent, which is `s16` quantisation. This caught
  `child_count` being a `u32` and the node stride being variable.
- **Embedded textures**: the sizes sum exactly to the header's declared block
  length, 47,296 against 47,296.
- **Track**: exactly 64 `section` nodes, matching the 64-bit PVS mask.

**Look for the arithmetic that must hold before writing the parser.** It is
usually there, and it converts a plausible reading into a determination.

### Ghidra

`BOOT.BIN` must be loaded with the **Allegrex** processor module, not stock
`MIPS:LE:32`. Stock Ghidra silently decodes VFPU instructions as nonexistent
64-bit MIPS III ones and the decompiler builds confident, fictional C from
them. `just build-allegrex` produces the extension; see
[allegrex-vfpu.md](docs/psp/allegrex-vfpu.md).

The image base is `0x08804000`. That is the conventional PSP module load
address and **has not been confirmed against PPSSPP** — the one outstanding
item that needs the developer.

## Provenance and how much to trust it

Most reverse-engineering findings came from background agents reading Ghidra.
They were instructed to report evidence and score confidence, and the scores in
`docs/` are theirs unless a page says otherwise.

**Independently verified by me, against real data:** the WAD name hash, the size
field ordering, LZSS, texture decoding, `.vex` geometry and textures, the
`section` count. These are the claims to lean on.

**Single-source static reading, never executed:** the physics force law, the
collision query path, the frontend state machine, the video path, the track
spline. All plausible, none runtime-verified. The physics agent said so
explicitly and it is worth repeating: nothing has been run under an emulator.

## Where I would go next

1. **Apply the Ghidra renames.** ~200 are documented with evidence and none are
   applied, so every future session re-reads `FUN_08940d0c` instead of
   `Wad_HashName`. Cheap, and compounding.
2. **Resolve the track file layout**, then render a track. That completes M1.
3. **Confirm the image base** against PPSSPP before more addresses are written
   down.
4. **Runtime verification** of anything in the physics documentation, which is
   the M3 harness in miniature and would upgrade a lot of 85s to 95s.

## Traps that cost time here

- **`python` string-replace patching of code already edited** fails silently and
  ships a build where a feature does nothing. Caught only because output said
  "0 textures" when 13 were known. Read the file and use a real edit.
- **wgpu 30** moved presentation to `Queue::present`, push constants to
  `immediate_size`, `multiview` to `multiview_mask`, and made
  `get_current_texture` return an enum rather than a `Result`.
- **The Gradle wrapper for the Allegrex build needs JDK 21**, not the system
  default. The failure does not mention the JDK.
- **`ls -la` fails in this shell** (aliased); use `eza -l`.
