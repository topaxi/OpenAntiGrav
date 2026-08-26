# Vita key material

This file itself is tracked (`!/data/**/README.md` in the repo root `.gitignore`,
technique only, no secrets); `vita-zrif.tsv` below, with the actual zRIF/klicensee
values, stays gitignored under `/data/*`. Exists purely so a re-decrypt of
`data/extracted/vita/` doesn't need a fresh trip to nopaystation.com or the F00D
service every time.

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
