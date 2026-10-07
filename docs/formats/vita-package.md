# Vita packages: `.vpk` read in place, `.pkg` refused by name

**Status: `.vpk` read in place and proven; `.pkg` + zRIF blocked on a
console-derived key, with the evidence below.** Written 2026-10-07 by the
`direct-read` lane. Applies to Wipeout 2048 (PCSF00007 EU, PCSA00015 USA).

| Input | What it is | Read in place? |
| --- | --- | --- |
| Extracted folder (`data/extracted/vita/<ID>/base`, `patch-v104`, `dlc1`, `dlc2`) | Plain files | Yes, since before this lane |
| NoNpDrm folder | The same: an installed game's folder, `sce_sys/package/work.bin` beside it | Yes - it **is** the extracted-folder shape; `work.bin` is for the console's `eboot.bin`, which this engine never runs |
| `.vpk` | A ZIP of that folder | **Yes, new**: `oag_disc::vpk` |
| `.pkg` (+ zRIF / `.rif` / `work.bin`) | PSN package | **No**: the PFS layer's file key is derived by the console's F00D, see below |

## `.vpk`

A ZIP of an installed game's folder. A NoNpDrm dump is made by copying the
folder off a console, where the file system driver has already removed the PFS
layer, so the files in one are plain; only `eboot.bin` is still an NpDrm SELF.
**Measured here on a stand-in, not a real NoNpDrm dump**: none was on disk, so
`scripts/make-test-vpk.sh` zips the extracted 2048 folders with the system `zip`
(an independent writer): the base and DLC 1 stored, the patch and DLC 2
deflated.

The reader (`crates/disc/src/vpk.rs`):

- **Central directory**, ZIP64 included (a 64-bit entry count, size or offset
  is read from the `0x0001` extra field in its documented order).
- **Stored** entries are read at random offsets (a 1.6 GB `data.psarc` needs
  that). **Deflated** entries are inflated whole and kept in a 512 MiB LRU; one
  past 1 GiB is refused by name and the message says to re-pack it stored.
  Encrypted entries and any other ZIP method are refused by name.
- **A set**: a patch and DLC ship as `.vpk` files of their own with the same
  `TITLE_ID` in `sce_sys/param.sfo`. `VpkSet::open` unions every `.vpk` beside
  the one named with that `TITLE_ID`, each file's paths prefixed by its stem
  (`base/PSP2/data.psarc`, `patch/PSP2/data1.psarc`), so the title's own
  suffix-matched archive names resolve as in an extracted folder and
  `data2` before `data1` before `data` still holds.
- The chooser lists one row per application: `PARAM.SFO`'s `CATEGORY` is `gd`
  for the base, `gp` for the patch and `ac` for DLC (measured on the four test
  files), and only `gd` is a row.
- `DiscImage` presents a package through the same `entries`/`read_entry_range`
  calls a disc answers (an `Entry`'s `lba` is the file index), so `oag-assets`
  mounts `<vpk>:<path>` specs with no other change.

### Proof

| Check | Result |
| --- | --- |
| Hand-built ZIPs (stored, deflate, siblings by `TITLE_ID`, bzip2 refused, junk refused) | 3 unit tests in `crates/disc/src/vpk/tests.rs` |
| `a_vpk_opens_as_2048_...` | the set mounts `data2, data1, data, dlc1, dlc2`, in the folder's order, with the same entry count as the folder |
| `every_archive_of_the_set_reads_byte_identical_...` | 525,861 bytes read from every mounted archive, identical to the folder source |
| Chooser row | `Wipeout 2048`, `Vita`, `PCSF-00007` |
| `oag-game <vpk> --race --ticks 1 --screenshot` vs the folder | PNGs byte-identical |

Ground truth: `crates/2048/tests/vpk_ground_truth.rs`,
`crates/disc/tests/ground_truth.rs` (categories, named refusals),
`crates/game/tests/launcher_ground_truth.rs`.

## `.pkg`: the outer layer reads, the PFS layer does not

**The outer layer** (public, [pkg2zip](https://github.com/mmozeiko/pkg2zip),
public domain) was read with a throwaway script on `2048-vita-eu.pkg`: header
magic `0x7f504b47`, `content_type 0x15` (Vita app), key type 2, 493 items, the
item table and names AES-128-CTR under a key made by ECB-encrypting the header
IV with one of three fixed package keys, the counter being the IV plus the
16-byte block index. That decrypts `sce_sys/param.sfo` (identical to the
extracted copy) and exposes `sce_pfs/files.db` (`SCENGPFS`, version 3, image spec
1, 0x400-byte pages, B-tree order 10, `files_salt` 0) and `sce_pfs/unicv.db`
(`SCEIRODB`, 2,008,064 bytes). **No package key is shipped in this
repository**: nothing here needs one.

**The PFS layer is what hides the game files** (`PSP2/data.psarc`,
`sce_module/*`, `eboot.bin`, and `sce_sys/livearea/contents/template.xml`, the
smallest, 264 bytes, which differs from its extracted copy after the outer layer
is removed). The known offline derivations were tried against that file's first
block, where plaintext is known, so the unknown cipher mask falls out directly
(`P0 = D(C0) xor mask`):

| Derivation (from `psvpfstools`, which ships no licence file, so read only as a cross-check and not copied) | Result |
| --- | --- |
| key = klicensee, mask = HMAC-SHA1(`hmac_key0`, `icv_salt`) for `icv_salt` 0..199,999, one and two salts | no match |
| key and mask from `SHA1(klicensee)` and `SHA1(salt, n)` (`generate_enckeys`), salt 0..299,999 | no match |
| key = klicensee, mask = HMAC-SHA1(`hmac_key0`, any 4/8/16/20 byte window of the first 8 KiB of `files.db` or `unicv.db`) (the `dbseed` form) | no match |

And that tool's own gamedata path sets `CRYPTO_ENGINE_CRYPTO_USE_KEYGEN`, where
the file key is `AESCBCDecryptWithKeygen(klicensee, key_id)`: a call into the
console's F00D secure processor, which `psvpfsparser` serves from an HTTP
service (`F00DUrlKeyEncryptor` sends the key and takes the derived key back) or
a cache file. The project's own `data/README.md` already says the extraction used
that tool. **Conclusion, confidence 75**: reading a Vita `.pkg` offline would need
a hardware-derived key, which is not a public package constant and is outside
the maintainer's 2026-10-07 decision (`docs/overview/legal.md`). It is not higher
because the negative result is one file under three families of derivation,
not a proof over every branch of the tool.

So a `.pkg` is **refused by name**: `DiscImage::open` on a `\x7fPKG` file says it
is a Vita `.pkg`, why, and to use a NoNpDrm `.vpk` or folder instead (checked on
both real 2048 packages). A zRIF still has a use - the NpDrm SELF of `eboot.bin`
- but the engine never reads that.

## Omega and 2048

- **Omega's `.pkg` is a different container** (`\x7fCNT`, a PS4 fake package, see
  `docs/reverse-engineering/source-images.md`); it is refused by name too and its
  in-place reader is open: *checked, differs* (nothing above applies to it), and
  *checked, applies, not wired* for the idea that its keys are public (fake
  package, no console secret, which is why `LibOrbisPkg` extracts it on a PC).
- Omega's folders and 2048's are read by the same `.rcsmodel` readers; this page
  adds nothing to either beyond the 2048 `.vpk` entry point.
