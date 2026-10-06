# Legal policy

## The rule

**No game content is in this repository, and none ever will be.**

That means no assets, no textures, no models, no audio, no executables, no
extracted data, no disc images, and no data derived from any of them in a form
that reconstitutes the original.

What this project distributes is *code* that reads formats, and *documentation*
that describes them. Users supply their own legally obtained copies.

## How this is enforced

Three layers, because one is not enough:

1. **`data/` is gitignored.** Everything except its README.
2. **Extension patterns are gitignored** globally (`*.chd`, `*.iso`, `*.wad`,
   `*.elf`, `*.bin`, and others, including reproductions this project itself
   writes, such as `--screenshot` output and the movie cache), so a stray copy
   outside `data/` is still caught.
3. **CI fails** if any tracked file matches those patterns. Run the same check
   locally with `just audit-leakage`.

The extension list backing layers 2 and 3 lives in one place,
[`scripts/check-leakage.py`](../../scripts/check-leakage.py), which both checks
tracked files and asserts `.gitignore` covers the same list - so the two
layers cannot drift out of sync with each other the way three independently
hand-maintained copies once did.

## What is fine to commit

- Format documentation, including field offsets, structure layouts and
  algorithm descriptions.
- Parsers and writers for those formats.
- SHA-256 hashes of source images, for reproducibility.
- Disc serials, volume identifiers, timestamps and file listings.
- Reverse-engineering notes: addresses, function signatures, inferred behaviour.
- **Our own translations of UI text, keyed by disc string ids. Verbatim
  copies of disc text are not allowed.** The ids and the terminology (a word the disc
  uses for a weapon or a mode) may be matched, the sentences are ours.
  `assets/ui/strings/disc/` is the one place this applies; see
  [project languages](../ui/project-languages.md).
- Small hand-authored test fixtures that we wrote ourselves, such as a synthetic
  ISO 9660 directory record.

The line is between *describing* the work and *reproducing* it. A field offset
table is a description. A texture is a reproduction.

## Test fixtures

Tests must not depend on game content. Where a test needs a structure to parse,
it builds one by hand, as the ISO 9660 tests do.

Tests that genuinely need a real disc image are marked `#[ignore]` and run only
via `just test-data`, with images the developer supplies. CI never runs them.

## Clean room

This is a black-box reimplementation. Behaviour is determined by observing the
original: reading disassembly, tracing execution, comparing outputs. The
resulting Rust is written fresh, not transliterated.

Where a documented algorithm turns out to be a well-known one, we say so and
implement the well-known one.

## Reporting a problem

If you believe something in this repository infringes your rights, open an issue
and it will be removed while the claim is assessed. The project has no interest
in hosting anything it should not.
