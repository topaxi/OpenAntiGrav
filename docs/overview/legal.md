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

## Fixed public keys

**Decision (maintainer, 2026-10-07): the project may ship the fixed public
package keys that other open-source projects already ship.** That is a
narrow rule. It covers constants that are the same for every disc or package of
a kind and are printed in published open-source code. It does **not** cover:

- a per-game licence (a zRIF, `work.bin`, a `.rif`, a Vita klicensee);
- a per-disc key (a redump `.dkey`, a PS3 disc key typed in the chooser);
- a console-derived key (Vita F00D keys, anything a hardware secure processor
  derives).

Those stay the player's. The engine reads them from a file beside the image,
from the app's own keys folder, or from the key prompt, and never prints one
(`DiscKey`'s `Debug` is redacted; no error formats key bytes).

Constants shipped so far, each checked against a local copy of the precedent on
2026-10-07 rather than recalled:

| Constant | Where it is in this repository | Precedent |
| --- | --- | --- |
| PS3 `data1` secret (16 bytes) and its IV | `crates/disc/src/ps3_crypt.rs`, `scripts/ps3iso.py`, `docs/formats/ps3-disc.md` | RPCS3, `rpcs3/Loader/ISO.cpp`, `iso_file_decryption::set_key_from_d1` (`key_d1`, `iv_d1`), GPL-2.0 |

Precedents for the package keys this decision lets a reader ship. **The PS4
fake-package pair is shipped (2026-10-07, `ps4-pkg`)**; the Vita keys are not,
because nothing here needs one:

| Constants | Precedent |
| --- | --- |
| Vita PKG keys `pkg_vita_2`, `pkg_vita_3`, `pkg_vita_4`, and the PSP/PS3 package keys | `pkg2zip`, `pkg2zip.c`, public domain (Unlicense text in its `LICENSE`); Vita3K and `psvpfstools` carry the same family |
| PS4 fake-package keys: the entry-key-3 and fake-package RSA-2048 private exponents and moduli (4 x 256 bytes), **shipped** in `crates/disc/src/ps4_pkg/keys.rs` | `LibOrbisPkg`, `LibOrbisPkg/Util/Keys.cs` (`RSAKeyset.FakeKeyset`, `RSAKeyset.PkgDerivedKey3Keyset`), checked against a local clone on 2026-10-07. **Licence: GNU LGPL-3.0** (its README and `LICENSE.txt`), *not* MIT as this table said until now. The values are numeric constants of a scheme any fake package opens with, transcribed by script; no `LibOrbisPkg` code is copied or translated, the reader being written from the layout in [ps4-package.md](../formats/ps4-package.md) |

A Vita PFS file key is **not** in that second table: it is derived by the
console, see [Vita packages](../formats/vita-package.md).

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
