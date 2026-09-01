# Vita and PSP key material

This file itself is tracked (`!/data/**/README.md` in the repo root `.gitignore`,
technique only, no secrets); the actual key tables below stay gitignored under
`/data/*`. Exists purely so a re-decrypt doesn't need a fresh trip to the same
external source every time.

## `vita-zrif.tsv`

One row per PKG this project has a license for: `title_id`, `region`, `content_id`,
`zrif`, `klicensee` (hex, derived from the zRIF - see below), `source`.

**A patch reuses its base app's zRIF.** Same content ID family, same NpDrm license -
checked directly against WipEout 2048's patch v1.04 PKGs, not assumed. So this file
only needs the *base* app row per region; there is no separate patch row.

### zRIF -> klicensee, in software, no hardware needed

A zRIF is `base64(zlib(rif_bytes, preset_dictionary))` - see
[KorewaWatchful/libzrif](https://github.com/KorewaWatchful/libzrif)'s `keyflate.c`
(`inflateKey`/`deflateKey`, windowBits `10`, a fixed 1024-byte preset dictionary baked
into that file). No AES, no console-derived secret - decoding a zRIF back to its 512-byte
`SceNpDrmLicense` (`packages/include/packages/license.h` in
[Vita3K/Vita3K](https://github.com/Vita3K/Vita3K)) is pure zlib:

```python
import base64, zlib
# dict_bytes: the first 1024 bytes of keyflate.c's g_dict[], extracted from the C source
d = zlib.decompressobj(wbits=10, zdict=dict_bytes)
license = d.decompress(base64.b64decode(zrif)) + d.flush()  # 512 bytes
content_id = license[0x10:0x40].split(b"\x00")[0]  # SceNpDrmLicense.content_id
klicensee  = license[0x50:0x60]                     # SceNpDrmLicense.key
```

This is a **different, lighter** layer than `psvpfsparser`'s own `-f00d_url`/`-f00d_cache`
(that derives the *PFS filesystem* key, and does need the F00D service). The klicensee
above is what `eboot.bin`'s own NpDrm segment decryption (`scripts/vita-self-decrypt.py`)
needs - see
[`docs/reverse-engineering/toolchain.md#vita`](../../docs/reverse-engineering/toolchain.md#vita)
for what that klicensee unlocks and why an earlier header-only strip was never enough.

zRIFs recovered so far both came from nopaystation.com's own TSV exports
(`https://nopaystation.com/tsv/PSV_GAMES.tsv`), fetched directly and grepped by title ID
rather than scraped through a page - the earlier attempt via a page-rendering fetch
against a large TSV was unreliable for picking out one specific row.

## `vita-f00d-cache.tsv`

Not yet repopulated in this checkout - see the open handover thread. Only needed for the
PFS layer (`psvpfsparser -f00d_url`/`-f00d_cache`), not for the klicensee above.

## `pure-dlc-keys.txt`

One 128-bit key per Wipeout Pure DLC content ID, all three regions (EU `UCES00001D*`,
JP `UCJS10007D*`, US `UCUS98612D*`). Unlike the Vita material above, **this is not a
per-console secret** - PSP disc-bound "Type D" content like Pure's uses a fixed,
publicly-known key per pack rather than an account- or console-derived one, which is
exactly why `pi.wad` decrypts identically for anyone with the file, no license
service involved.

Source: Thomas Perl's [`wipeout-pure-dlc2dlc`](https://gitlab.com/thp/wipeout-pure-dlc2dlc)
(ISC license), `keys.txt` as of its 2021-05-30 release, fetched verbatim 2026-09-01.
That tool exists to convert a DLC pack between regions for real hardware; this project
only needs its key table and its `crypt_with_key`/`xtea8` algorithm (documented in
[`docs/formats/dlc-pack.md`](../../docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table)),
not the region-conversion feature itself.

**Verified against this project's own copy of all seven EU packs in `data/dlc/`, not
just cited**: for each, XORing the documented XTEA-8 keystream (keyed by the row
matching that pack's own content ID, which `PARAM.sfo`'s `TITLE` field names) over
every byte but the last 256 turns it into a file `oag-wad` parses unmodified - real
entry counts, real LZSS/zlib payloads, no format-specific decoder needed. The last 256
bytes are a per-region signature this project's decoder never reads; see the doc page
for what it is and why it is out of scope.
