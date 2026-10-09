# PS3 disc encryption

**Confidence: 94.** The format itself is publicly documented rather than
recovered here; what this page adds is verification against a real disc. The
check is named below and it is the strongest kind short of a runtime trace: three
files that the disc happens to ship **twice**, once inside a plain region and
once inside an encrypted one, come out byte-identical after decryption, and 1.98
GiB of `.psarc` payload then inflates as valid zlib into readable paths. It does
not reach 95 because no PS3 executable has been read: nothing here is
corroborated against the code that consumes it.

Applies to [`hdfury-ps3-eu.iso`](../reverse-engineering/source-images.md#hdfury-ps3-euiso---wipeout-hd--fury-ps3),
the only PS3 image this project holds.

## There are two layers, and they are unrelated

Confusing them wastes a lot of time, so state them separately.

| Layer | What it covers | What opens it |
| --- | --- | --- |
| **1. Disc** | Whole 2048-byte sectors, in the spans the disc declares encrypted | The 16-byte **disc key**, per disc |
| **2. File** | `EBOOT.BIN`, `.sprx`, `.self`, and `.edat` for digital titles | Console **SELF keys**, the same for every disc |

This page is layer 1 only. Decrypting the disc gets you a `.psarc` you can read
and an `EBOOT.BIN` you cannot - the executable is still a SELF container full of
ciphertext, which is a separate problem needing a different set of keys.

**There is no "BIOS" in either layer.** An emulator decrypts layer 2 in its own
code with keys it carries; the firmware image such an emulator asks for is
unpacked for its system modules, not executed as a privileged decryptor. The
misreading matters because it suggests obtaining firmware would solve layer 1,
and it would not - layer 1 needs a key that is specific to the individual disc.

## The region table lives in sector 0

Big-endian throughout.

| Offset | Size | Field |
| --- | --- | --- |
| `0x00` | 4 | Count of **plain** regions, `n` |
| `0x04` | 4 | Zero |
| `0x08` | `8n` | `n` pairs of `{first_lba, last_lba}`, inclusive, ascending |

The pairs cover only the plain regions; every gap between one pair's `last_lba`
and the next pair's `first_lba` is an encrypted region. The first pair starts at
sector 0 and the last one ends at the final sector of the volume, so the table
tiles the whole image with no slack.

**Do not read this table as a statement about the file in front of you.** It is
copied through decryption verbatim - a decrypted image still says its `.psarc`
span was encrypted, because on the disc it was. Sector 0 tells you the disc's
layout, never the file's current state. `scripts/ps3iso.py` distinguishes the two
with the oracle below, and prints a note when it finds a decrypted image.

Reproduce with:

```sh
python3 scripts/ps3iso.py map data/images/hdfury-ps3-eu.iso
```

### Wipeout HD / Fury's layout

Three plain regions, so `n = 3` and the table holds six bounds.

| Sectors | Bytes | State | Contents |
| --- | --- | --- | --- |
| `0x0`-`0x75f` | 3,866,624 | plain | `PS3_DISC.SFB`, `PARAM.SFO`, icons, `PIC0`-`PIC2`, `PS3LOGO.DAT` |
| `0x760`-`0xecabf` | 1,981,480,960 | **encrypted** | `DATA00`-`DATA06.PSARC`, `DFEngine.sprx`, `EBOOT.BIN` |
| `0xecac0`-`0xed03f` | 2,883,584 | plain | `TROPHY.TRP` |
| `0xed040`-`0xed4df` | 2,424,832 | **encrypted** | `USRDIR` copies of `ICON0.PNG`, `ICON1.PAM`, `PIC1.PNG` |
| `0xed4e0`-`0x10d4ff` | 268,500,992 | plain | `PS3UPDAT.PUP` |

**Every region starts at a file boundary**, and every file lies wholly inside one
region - checked by parsing the ISO 9660 directory records and comparing extents
against the table, which is an independent source agreeing with it rather than a
re-reading of it. Region *ends* are not file ends: each carries a few sectors of
padding before the next region begins, 8 past `TROPHY.TRP` and 32 past
`PS3UPDAT.PUP`. That is also why
the firmware update payload and the trophy pack are in the clear: they are not
the publisher's content to protect.

## The sector cipher

Each encrypted sector is AES-128-CBC with **no padding**, decrypted
independently of every other sector:

- **Key**: the 16-byte disc key, the same for the whole disc.
- **IV**: twelve zero bytes followed by the sector's **absolute LBA**,
  big-endian. Absolute, not an index within its region - using the
  region-relative index is the classic way to get this wrong and it fails
  silently, producing garbage only for the second region onward.
- **Chaining resets at every sector**, so sectors can be decrypted in any order.

Plain regions are copied through untouched.

## Where the key comes from, and where it does not

The disc key is **not in the image**, and no amount of analysis recovers it.

On real hardware the drive and the console authenticate, and the drive returns
`data1`, from which the disc key is derived by encrypting it (AES-128-CBC, one
block, under a fixed IV that [RPCS3](https://github.com/RPCS3/rpcs3) carries beside the secret) with a fixed secret
(`0x380bcf0b53455b3c7817ab4fa3ba90ed`). A PC drive dumping the disc never
performs that exchange, so `data1` never lands in the file. This is why redump
publishes a separate `.dkey` per disc: a redump `.dkey` holds the **derived**
key, ready to use as the sector key with no further step.

Checked rather than assumed, on this image:

- Every 16-byte window of sectors 0 and 1 - 538 distinct, tried both as the
  sector key and as `data1` through the secret - fails the oracle. The
  high-entropy blob at `0x840` is 448 bytes of disc signature, not a key store.
- The file is exactly the declared volume size (`0x10d500` sectors,
  `0x86a80000` bytes), so there is no out-of-volume tail to hide anything in.

The structural argument is the load-bearing one; those two are corroboration.

`scripts/ps3iso.py` accepts a key in either form and lets the oracle decide which
it was given, so a caller does not have to know. **No key is stored in this
repository** - `.dkey` is in `.gitignore` and in `scripts/check-leakage.py`'s
extension list, because it is the one thing that opens a disc image.

## Read in place: `oag_disc::ps3_crypt`

Since 2026-10-07 the engine reads the encrypted image directly, with no decrypted
copy. `ps3_crypt::DecryptSource` wraps the raw source and decrypts a sector when
its absolute LBA lies in a gap between sector 0's plain regions (AES-128-CBC, IV
twelve zero bytes plus the LBA big-endian, chaining reset per sector, exactly as
above). `DiscImage::open` finds a key itself: `<stem>.dkey`/`<stem>.key` beside
the image, then every `.dkey`/`.key` in `<config>/oag/keys/`; the chooser's key
prompt writes there. On the web the page mounts a key it was given as
`<stem>.dkey` beside the image, the same first place
([web.md](../tools/web.md#disc-keys)); `oag_disc::ps3_probe` answers "does this
need a key, and does this one open it" for the page and the launcher's file drop.
See [installing](../overview/installing.md#hd--fury-the-disc-is-read-encrypted-in-place).

- **The oracle is the decision, never the region table.** A decrypted image keeps
  sector 0's table, so "table says encrypted" and "a key exists" cannot choose
  whether to decrypt. `unlock` first tries the raw view; if every oracle target
  already reads as its magic or its plain twin the image is `AlreadyPlain` and is
  never double-decrypted. Only then are keys tried, each as the sector key and as
  `data1` through the documented secret, and a key is accepted only if **every**
  target passes. A walk of the directory tree or an empty target list is "locked",
  never "accept".
- **Targets** are built generically from the ISO listing, not from HD file names:
  a file starting in an encrypted gap whose same-name, same-size twin sits in a
  plain region predicts a whole sector; otherwise `.PSARC`, `.PNG`, `.SPRX`,
  `.BIN`, `.PUP` predict their magic. HD yields the same kinds of target
  as `ps3iso.py` (not counted separately here).
- **Key hygiene.** `DiscKey` has a redacted `Debug`, so a `{:?}` of a `DiscImage`
  cannot leak it, and no error formats key bytes.
- **States** (`DiscImage::ps3_state`): `NotEncrypted`, `AlreadyPlain`,
  `Decrypting`, `Locked`. `Locked` leaves the plain regions readable, which is why
  the chooser still identifies the disc from `PS3_DISC.SFB`.

**Measured on the maintainer's HD image.** This is engineering verification of the
reader (byte-identity against the independently decrypted copy), not new
evidence about the format, so the page's confidence of 94 stands:

| Check | Result |
| --- | --- |
| All 22 files, read through `DiscImage` from the encrypted image vs the decrypted copy (PUP: first MiB) | byte-identical, 1,990,646,154 bytes (1.85 GiB) compared |
| A strided 4 KiB sample through every file over 16 KiB, so the second encrypted region's absolute-LBA IV is exercised | identical |
| Race frame (`--race --ticks 1 --screenshot`, Talon's Junction) from each image | PNGs byte-identical |
| Race load, release build, three runs at load average ~13 | encrypted 1.96 / 1.87 / 1.88 s, decrypted 1.85 / 1.86 / 1.88 s |
| Wrong key, or no key | `Locked`; `key_opens` false |

Ground truth: `crates/disc/tests/ps3_crypt_ground_truth.rs` (ignored, disc-backed)
and the unit tests in `crates/disc/src/ps3_crypt/tests.rs` (FIPS-197 vector, an
independent encrypt-then-decrypt round trip proving the IV rule, key parsing,
region-table validation).

## The oracle: how a key is known to be right

A wrong key produces 2 GiB of plausible-looking noise, so the tool refuses to
write anything until a candidate key satisfies twelve independent predictions:

- **Nine format magics**, on files whose first sector begins an encrypted region
  or lies inside one: `PSAR` for the seven `.psarc`, `SCE\0` for `DFEngine.sprx`
  and `EBOOT.BIN`.
- **Three full 2048-byte cribs.** `ICON0.PNG`, `ICON1.PAM` and `PIC1.PNG` each
  appear twice on this disc, once under `PS3_GAME` in a plain region and once
  under `PS3_GAME/USRDIR` inside an encrypted one. The plain copy predicts the
  whole first sector of the encrypted one.

The cribs assume the two copies are duplicates, which cannot be checked before
decrypting. The failure direction is safe: if they are not duplicates a correct
key is rejected, never a wrong key accepted.

```sh
python3 scripts/ps3iso.py oracle data/images/hdfury-ps3-eu.iso <32-hex-key>
```

Exit 0 accepts, 1 rejects, and `decrypt` runs the same test before writing.

## Verification of the result

After decryption, on Wipeout HD / Fury:

1. **The twin files match byte for byte.** `PS3_GAME/ICON0.PNG` against
   `PS3_GAME/USRDIR/ICON0.PNG`, and likewise `ICON1.PAM` and `PIC1.PNG` -
   2.3 MiB of exact agreement between data that was never encrypted and data
   that came out of the cipher. This is the check the confidence score rests on.
2. **The archives inflate.** `DATA04.PSARC` reports `PSAR` 1.3, zlib, 64 KiB
   blocks and 55 entries, and its table of contents decompresses to real paths
   (`/data/fe/fonts/chinese.fnt`). Deflate streams do not survive corruption.
3. **The PNGs decode** at their expected dimensions.

Decryption of the whole 2.1 GiB image takes about five seconds.

## What is still closed

**`.psarc` no longer is**, and the sentence that used to stand here - "no asset
has come off this disc" - was true until 2026-08-17 and is not now. The
container is read, all seven archives list, and any entry comes out by path: see
[psarc](psarc.md) for the format and [hd-status](hd-status.md) for what turned
out to be inside, which is largely the PSP's own asset pipeline byte-swapped.

What is still closed on the asset side is the PS3-specific half: `.rcsmodel`
(all the render geometry), `.rcsmaterial`, and `.gtf` textures. Everything below
is about the executable, which is a separate half of the layer-2 problem.

**Layer 2 on `EBOOT.BIN` turned out to be free.** `rpcs3 --decrypt` writes a real
PPC64 ELF using keys RPCS3 carries, with no firmware install - it prints
`Missing Firmware` and works. That ELF is imported, analysed and partly named:
see [`docs/ghidra/functions/ps3-hdfury-eu/`](../ghidra/functions/ps3-hdfury-eu/)
and, for the import procedure, [toolchain.md](../reverse-engineering/toolchain.md#ps3).

`DFEngine.sprx` is still closed, for a different reason than encryption: its ELF
type is `0xffa4` (`ET_SCE_PPURELEXEC`, a relocatable PRX) and Ps3GhidraScripts
does not support relocations.

Identification needs nothing from this page at all:
[`oag-disc`](../../crates/disc/src/platform.rs) reads a PS3 disc's serial out of
`PS3_DISC.SFB` as of 2026-08-17, which is in a plain region, so `oag-unpack info`
works on the encrypted image exactly as it does on the decrypted one.

## See also

- [PSARC](psarc.md) - the container this layer gets you to
- [HD status](hd-status.md) - what is in it, and how much already parses
- [Format index](README.md) - the `.psarc` and SELF rows
- [Source images](../reverse-engineering/source-images.md) - this disc's entry
- [Legal](../overview/legal.md) - why no key and no content is committed
