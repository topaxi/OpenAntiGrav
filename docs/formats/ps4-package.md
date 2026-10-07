# PS4 fake packages: read in place

**Status: read in place and proven byte for byte against the `LibOrbisPkg`
extract.** Written 2026-10-07 by the `ps4-pkg` lane. Applies to Wipeout: Omega
Collection (`CUSA05670`, `EP9000-CUSA05670_00-WIPEOUTOMEGA00EU`): the base
`omega-ps4-eu.pkg` and the day-one patch `omega-ps4-eu-patch.pkg`.

`oag_disc::ps4_pkg` opens a package and its patch beside it as one file tree
(`DiscImage::open`, so every `<image>:<path>` spec works), with no extraction
step. A retail PSN package is keyed to a console and is refused by name; a
**fake package** (the scene's HEN/jailbreak rip, which is what both files are) is
keyed with public constants.

## Layout (all measured on both packages unless marked)

| Layer | Where | What | Byte order |
| --- | --- | --- | --- |
| Header | `0x0` | `\x7fCNT`; entry count `0x10` (u32), entry table offset `0x18` (u32), `content_id` `0x40` (36 bytes), `pfs_flags` `0x408` (u64, `0x80000000000003cc` on both), `pfs_image_offset` `0x410`, `pfs_image_size` `0x418` | big |
| Entry table | at `0x18` | 32-byte rows: `id`, name offset, `flags1`, `flags2`, data offset `+0x10`, data size `+0x14`, 8 pad. Base has 25, patch 41 | big |
| `PARAM_SFO` | entry `0x1000` | plaintext; `CATEGORY` is `gd` (base) or `gp` (patch), `TITLE_ID` `CUSA05670` on both | |
| Entry keys | entry `0x10` | 32-byte seed digest, seven 32-byte digests, seven 256-byte RSA blocks; **key 3** is used | |
| Image key | entry `0x20` | 256 bytes, AES-CBC under a key from key 3, then RSA | |
| Outer PFS | `pfs_image_offset` | AES-XTS-128, 4 KiB sectors, sector number from the image start, the first `block_size / 0x1000` = 16 sectors (the superblock) stored in the clear. Its mode has the encrypted bit (4), which the reader requires; the exact value was not recorded | little |
| `pfs_image.dat` | in the outer PFS `uroot` | a **PFSC** container whose content is the inner PFS | |
| Inner PFS | inside it | mode `0x8` (unsigned, unencrypted), 64 KiB blocks, 22 inodes (base) and 19 (patch) | little |
| Files | inner `uroot` | stored; none of the 25 carries the inode's compressed flag (`0x1`) | |

### The key chain (confidence 95: it decrypts, and the result matches 45 GB of known plaintext)

1. `dk3 = RSA-2048-decrypt(entry_keys.key[3])` with the **entry key 3** private
   key, PKCS#1 v1.5 type-2 unpadded (a 32-byte payload).
2. `iv_key = SHA-256(image_key_entry_record || dk3)`, where the record is the
   32-byte table row **as stored**. AES key = `iv_key[16..32]`, IV = `iv_key[0..16]`.
   Decrypt the 256-byte image key with AES-128-CBC.
3. `ekpfs = RSA-2048-decrypt(that)` with the **fake package** private key, same
   unpadding.
4. `seed` = 16 bytes at the outer superblock `+0x370`. If `pfs_flags & 0x2000000000000000`
   the key is first `HMAC-SHA256(ekpfs, seed)`; neither package sets it, so it is
   `ekpfs`. `enc = HMAC-SHA256(key, LE32(1) || seed)`; the **first 16 bytes are the
   XTS tweak key and the last 16 the data key**.
5. XTS: tweak = `AES-ECB(tweak_key, LE64(sector) || 0^8)`, per 16-byte block XOR,
   decrypt, XOR, multiply the tweak by x in GF(2^128) with `0x87`.

### PFS superblock, inode, directory (little-endian)

Superblock (block 0): version `0x00` = 1, magic `0x08` = 20130315, mode `0x1C`
(u16: 1 signed, 2 64-bit, 4 encrypted), **block size `0x20`** (u32), inode count
`0x30`, inode-block count `0x40`, seed `0x370`. Inodes start at block 1.
Inode (`0xA8` bytes unsigned, `0x2C8` signed): mode u16, nlink u16, flags u32,
size `+0x08`, second size `+0x10`, times, blocks `+0x60`, then 12 direct and 5
indirect block numbers (a signed inode puts a 32-byte signature before each).
Directory entries: inode u32, type u32 (2 file, 3 directory), name length u32,
entry size u32, name. The files are the inner root's `uroot` subtree.

Block lists: a file whose second direct block is not `-1` is read as a block
list (the signed form walks the indirect blocks, 36 bytes per entry). **No file
on either package takes that path** (census below), so it is written from the
reference and unproven on data.

### PFSC

`PFSC` `+0`, `0` `+4`, `6` `+8`, block size `+0x0C` (`0x10000`) repeated as a u64
at `+0x10`, sector-map offset `+0x18`, data start `+0x20`, data length `+0x28`.
The map is `data_length / block + 1` u64 offsets. A sector whose span equals the
block is stored; **greater, all zero**; smaller, a zlib stream (skip the two-byte
header, inflate **to completion**). The data length is always a whole number of
sectors on both packages' `pfs_image.dat`; the reader refuses a length that is not
(the reference would silently drop the tail), and a map out of order.

**The extractor's short read.** `LibOrbisPkg`'s `PFSCReader.ReadSector` makes one
`DeflateStream.Read` call and discards the count, which is the bug
[gnf.md](gnf.md#root-cause-confidence-90-a-short-streamread-in-the-extraction-tool-not-this-projects-reader)
and [psarc.md](psarc.md) record. This reader inflates each sector fully, and its
output equals the *patched* extract everywhere.

## Census (2026-10-07)

Source: `crates/disc/tests/ps4_pkg_ground_truth.rs` and a one-off listing; the file
lists below were also compared with `find data/extracted/ps4 -type f` (14 + 11 = 25
files, the same 25).

| | base `omega-ps4-eu.pkg` | patch `omega-ps4-eu-patch.pkg` |
| --- | --- | --- |
| Size | 23,869,194,240 | 3,404,988,416 |
| Entry-table rows | 25 | 41 |
| `pfs_image` offset, size | `0x800000`, `0x58e370000` | `0x400000`, `0xcab40000` |
| Inner files | 14 | 11 |
| Archives | `data00`..`data04` (13.5, 10.7, 9.7, 2.6, 6.4 GB) | `data05`, `data07`, `data08`, `data09` |
| Files with inode flag `0x1` (PFSC) | 0 | 0 |
| Files with a scattered block list | 0 | 0 |
| Signed inner PFS | no | no |

So the per-file PFSC branch and the block-list branch have no disc sample; they
are in the reader because the format has them, and have unit tests only
(`layers/tests.rs`). **The inode's second size field is what a PFSC file's
logical length would be per the reference; here it equals the first on all 25
files, so that naming is unverified.** The reader checks it against the PFSC
header and refuses a mismatch by name.

## Proof

| Check | Result |
| --- | --- |
| File lists, paths and sizes vs the extracted folders | exact, both packages (`file_lists_and_sizes_match_the_extracted_folders`) |
| **Every byte** of all 25 files vs `data/extracted/ps4/omega-eu{,-patch}` | **equal**: `OAG_PS4_PKG_FULL=1 cargo nextest run --release -p oag-disc --run-ignored all -E 'binary(ps4_pkg_ground_truth)'`, 26 tests passed, 92.7 s wall, the longest single file (`data00`) 92 s, about 45 GB compared, run once outside the suite |
| In the `test-data` suite | files up to 64 MiB whole; the five multi-gigabyte archives (and `data05`, `data08`) by head, tail and 48 spread 256 KiB windows. A full compare is not in the suite: it would sit at the 300 s per-test ceiling under load |
| Omega as a source | `omega_pkg_source_ground_truth`: six names (front end, a grid, plugin definitions, `tech_de_ra`'s `track.vex` and `.EnvSettings`) are served by the same archive and read the same bytes opening Omega from the `.pkg` and from the folders |
| A race frame | `oag-game <pkg> --race --no-audio --ticks 120 --screenshot` and the same from `data/extracted/ps4`: PNGs **byte-identical** (SHA-256 `1e3ba0ff...8b141`), logs identical but for a Vulkan handle |
| Crypto primitives | SHA-256, HMAC (RFC 4231 case 2) and AES-XTS (IEEE 1619 vector 1) known-answer unit tests, and both transcribed RSA keys round-trip `(m^e)^d == m` |

**Load-time cost** (debug build, 120 ticks, one machine, not a quiet one): the
race from the `.pkg` took 20 s, from the folder 8 s. Opening the pair costs
about 0.1 s (headers, the two PFS trees); the rest is reading and inflating the
archive blocks a load touches, each decrypted and inflated on request.

## How it reads (not an extractor)

Each layer owns the one below: file, XTS sectors (32 cached), the outer file's
blocks, PFSC (64 inflated sectors cached), the inner file's blocks, and for a
compressed inner file one more PFSC. A multi-gigabyte `data00.psarc` costs what
a request for it touches. The opened package is **shared process-wide by path**
(`OPENED`), so the archives, sound banks and movies a race opens through
separate readers decrypt each sector once between them (the lesson of `ed892e100`,
the `.vpk` inflate cache).

## As a source

- `DiscImage::open(<base or patch .pkg>)` opens `Ps4Set`: the file itself and
  every `.pkg` beside it that is a `\x7fCNT` package with the same `TITLE_ID`,
  base (`gd`) first, then patches, each file's path prefixed
  `<package stem>/uroot/` so it reads like the extracted folder
  (`omega-ps4-eu-patch/uroot/data09.psarc`) and `oag_omega`'s tail-matched
  candidates resolve unchanged. **The mount order is the candidate order, not the
  package order**, exactly as for the folders; it is `DATA_CANDIDATES` then
  `EXTRA_CANDIDATES` in `crates/omega/src/lib.rs` either way.
- Platform `Ps4` and the title id come from the plaintext `PARAM.SFO` entry
  (`PackageSource::platform` and `title_id`), since the inner tree holds no
  `param.sfo`.
- **Chooser**: a base `.pkg` with its patch beside it is a row (`gd` and a sibling).
  **When both an unpacked `data/extracted/ps4` and a `.pkg` pair exist, only the
  folder is listed and booted by default**: it is what the maintainer prepared,
  and the default must not change under them. Naming the `.pkg` opens it.
- A Vita `.pkg` (`\x7fPKG`) is still refused by name; see
  [vita-package.md](vita-package.md).

## Keys and licence

The two keys are public constants of the fake-package scheme, in
`crates/disc/src/ps4_pkg/keys.rs` (private exponent and modulus of each, 256 bytes
big-endian), transcribed by script from `LibOrbisPkg/Util/Keys.cs`
(`RSAKeyset.FakeKeyset`, `RSAKeyset.PkgDerivedKey3Keyset`). **`LibOrbisPkg` is
licensed LGPL-3.0, not MIT** (its README: "All code in this repository is
licensed under the GNU LGPL version 3"); see
[legal.md](../overview/legal.md#fixed-public-keys). No code was copied: the
reader is written from the layout above.

## Cross-check with 2048

2048's Vita `.pkg` is **checked, differs**: a different container (`\x7fPKG`),
and its PFS file key comes from the console's F00D service, so it stays refused
(see the Vita page). The two titles share asset lineage, not package format.
What the 2048 `.vpk` reader and this one share is `PackageSource`, `DiscImage`
and the stem-prefix convention.

## Open

- A per-file PFSC and a scattered-block file are unproven on data (none ships).
- An encrypted or 64-bit inner PFS is refused by name, no sample.
- A retail (console-keyed) PS4 package fails at step 1 with a message saying so;
  untested against a real one.
- `pfs_flags` bit `0x2000000000000000` (second key derivation) is written from
  the reference; neither package sets it.
