# Downloadable content packs

**Status: understood.** Read by
[`oag-assets::dlc`](../../crates/assets/src/dlc.rs), unpacked by
[`oag-game::dlc`](../../crates/game/src/dlc.rs), mounted by
[`oag_assets::Archives`](../../crates/assets/src/source.rs).

Wipeout Pulse sold four downloadable packs for the PSP. Each adds one team and
two circuits. **There is no new container here**: the payload files are
[WAD archives](wad.md) under a misleading extension, so nothing in this page
required a new parser.

This page also settles a question the repository had open in four places - which
platform ships `Auricom`, `Harimau`, `Icaras` and `Mirage`. See
[the roster](#the-roster-and-the-name-that-hid-it).

## Layout

A download is a `.zip` holding one folder named for the title id, and five
files:

```
UCES00465/
  PACKn.edat        the team's ship, and the two circuits' geometry
  PACKn_UI1.edat    the team's handlingstats.xml, and front-end art
  PACKn_UI2.edat
  PACKn_UI3.edat
  PARAM.pbp         PSN packaging metadata; nothing in the engine reads it
```

Every `.edat` is a [WAD](wad.md), version 1, entries stored or LZSS-compressed,
with the offset chain closing exactly as the disc's own archives do.

### Entry 0 is a manifest

The first entry of each archive is XML shaped like a fragment of the disc's own
`Data\Plugins\PI001\Definition.xml` - the same `PI_Team` and `PI_Track` schema,
under the same `<Screen name="Top">` root:

```xml
<Screen name="Top">
  <PI_Team name="...">
    <Values type="Race" soundregister="..." multiplayer="true"
            location="Data\Ships\..." helpText="MSC_TEAMDES_..."/>
    <FE speed="" thrust="" handling="" shield=""/>
    <PI_TeamModel name="Normal">
      <Values location="ship"/>
      <PI_ModelSkin name="Alternative">
        <Values location="Data\Ships\...\ship_alt.dat"/>
        <Unlock Team="..." loyalty="" Exclusive="true"/>
        <Unlock Team="any" loyalty="" Exclusive="true"/>
      </PI_ModelSkin>
      <PI_ModelSkin name="Eliminator">
        <Values location="Data\Ships\...\ship_eliminator.dat"/>
        <Unlock Team="..." loyalty="" Exclusive="true"/>
        <Unlock Team="any" loyalty="" Exclusive="true"/>
      </PI_ModelSkin>
    </PI_TeamModel>
    <PI_TeamModel name="Concept">
      <Values location="extra"/>
      <Unlock Team="..." loyalty="" Exclusive="true"/>
      <Unlock Team="any" loyalty="" Exclusive="true"/>
    </PI_TeamModel>
    <PI_TeamModel name="Zone">
      <Values location="zone01"/>
      <Unlock Team="..." loyalty="" Exclusive="true"/>
      <Unlock Team="any" loyalty="" Exclusive="true"/>
    </PI_TeamModel>
  </PI_Team>
  <PI_Track name="..."><Values type="Race" location="Data\Environments\..."/></PI_Track>
  <LoadXML><Values Src="downloadNN.xml"/></LoadXML>
</Screen>
```

**`Unlock` is not only a `PI_ModelSkin` child.** Confirmed against all eight of
the disc's own teams and, separately, a mounted pack's own manifest
(`PACK3.edat` entry 0, Harimau): `Concept` and `Zone` carry `Unlock` directly
under `PI_TeamModel`, with no `PI_ModelSkin` nested inside them at all -
`Normal` is the one variant with skins, and `Concept`/`Zone` are each a single
unlockable model in their own right. **Read this schema, not the fragment
above alone**: every `PI_TeamModel` a team declares owns one or more `Unlock`s,
either directly or through its `PI_ModelSkin` children, never both at once.

**Every one of the eight disc teams and the one pack manifest checked declares
exactly this shape**, so it is read as the whole schema rather than one
sample:

- `PI_TeamModel name="Normal"`, `Values location="ship"` (a **file stem**, not
  a path - the disc builds the raceable hull from the team's own `location` and
  `Ship.vex`, never from this field; see `crates/livery/src/lib.rs`'s module
  docs for the trap this avoided). Two nested `PI_ModelSkin`s:
  - `name="Alternative"`, `Values location="Data\Ships\<Team>\ship_alt.dat"`
  - `name="Eliminator"`, `Values location="Data\Ships\<Team>\ship_eliminator.dat"`

  each with two `Unlock`s: one keyed to the team's own id, one keyed to the
  literal `Team="any"` - a two-tier threshold, the team's own `loyalty` always
  the lower of the pair. Both carry `Exclusive="true"` on every instance seen.
- `PI_TeamModel name="Concept"`, `Values location="extra"`, with the same
  own-team/`any` `Unlock` pair directly beneath it - no skin.
- `PI_TeamModel name="Zone"`, `Values location="zone01"`, same shape again.

**The `loyalty` numbers themselves are not reproduced here** - shipped tuning
data, the same rule `docs/formats/handling-stats.md` already applies to the
handling globals. The *relationship* above (two tiers, own team below `any`)
is a format fact and is what's recorded.

**`ship_alt.dat`/`ship_eliminator.dat` are not meshes - they are liveries, and
the format is decoded.** Each is a `0x20` header (the team's display name,
NUL-terminated - **there is no `"ms"` tag**, that reading came from one team
whose name is exactly eight bytes long, and the bytes after the terminator are
exporter residue) followed by four palette-plus-pixels blocks: three 128x128 and one
64x64, every one of them 4 bits per pixel behind its own sixteen-entry `RGBA8`
palette. All sixteen shipped files are exactly 26912 bytes. The loader matches
each block against the hull model's own texture names and uploads it in place
of `texture1.tga`..`texture4.tga`, so a skin is a **texture swap on the same
geometry** and never a second model. Confidence 90; the byte table, the loader
and the two details that must be reproduced rather than corrected are in
[`ship-skin.md`](../ghidra/functions/psp-pulse-usa/ship-skin.md).

The main archive's manifest is populated and plain (`<?xml`, unshortened); the
three UI archives carry an empty `<Screen name="Top">` stub as
[shortened XML](fexml.md). Entry 1 of the main archive is the `downloadNN.xml`
the `LoadXML` names: a `PI_Grid` championship ladder, which this engine does not
read yet.

Because the schema is the disc's,
[`oag_game::catalogue`](../../crates/game/src/catalogue.rs) reads a pack with
the very functions that read the disc, and a mounted pack's teams and circuits
are indistinguishable downstream from the source's own.

## `PACKn.edat` is not encrypted

**Confidence: 94.** The extension says EDAT, which on the PSP normally means
PGD-encrypted content needing a key. These are not:

- All four packs parse as WAD v1 with an unmodified `oag-formats::wad`: 32-33
  entries each, LZSS where the flags say LZSS, and `entry[i+1].offset ==
  align64(entry[i].offset + entry[i].size)` holding across every entry. An
  encrypted file does not accidentally satisfy an arithmetic invariant over its
  whole directory.
- Every blob decompresses to its declared uncompressed size, and the results are
  `.vex` scenes and XML that the existing decoders read.
- The executable's only NpDrm import, `sceNpDrmEdataSetupKey`, is called with no
  key argument, and `sceNpDrmSetLicenseeKey` - which would supply one - is never
  imported. See `data/README.md`.

**Capped at 94, and the cap is the rubric's rather than a hedge.** Nothing here
watched the original mount a pack, so the
[rubric](../reverse-engineering/confidence-rubric.md)'s 95-100 band - a runtime
trace *and* a second binary - is out of reach on its first leg. What this does
have is the top of the 85-94 band twice over: an exact arithmetic invariant
across every entry of four real files, and the same reading holding in a second
binary's archives. [`wad.md`](wad.md#evidence) caps the container itself at 94
for exactly this reason, and a page about that container's contents cannot score
higher than the container.

Two consequences worth stating plainly, because both cost time otherwise:

1. **There is no decryption step to write.** A contributor who sees `.edat` will
   look for one.
2. `Wad_ApplyStreamCrypt` (`0x089508e8`), the WAD subsystem's dead cipher hook,
   is *not* what protects these. It is NULL at every retail mount and these
   files are plaintext, so the two facts agree rather than one explaining the
   other. See [the WAD subsystem](../ghidra/functions/psp-pulse-usa/wad-subsystem.md).

## The ships are fully simulatable

**Confidence: 90.** A pack carries `Data\Ships\<Team>\handlingstats.xml` - in
`PACKn_UI1.edat`, not in the main archive - as shortened XML in the documented
[handling-stats](handling-stats.md) schema, with all four speed classes and
every attribute the parser requires - which is a real constraint rather than a
lenient read, because `oag_tables::handling` has no defaults and rejects a file
missing any of them. `crates/game/tests/dlc_ground_truth.rs` loads all four
teams in all four speed classes.

Below the container's own 94 rather than level with it: this is four files
parsing against a schema, not an arithmetic invariant closing over their bytes,
and it inherits [handling-stats](handling-stats.md)' own open question about
which PSP function writes the tail of the struct.

This is the difference between loading a DLC ship and *racing* one, and the
split across archives is the trap: mount only `PACKn.edat` and the model appears
while the stats do not.

## The roster, and the name that hid it

**Confidence: 94.** The four packs add these teams, by **id** - the folder under
`Data\Ships\`:

| Pack, as sold | Archive | Team id | Circuits it carries |
| --- | --- | --- | --- |
| Mirage | `PACK1` | **`Mantis`** | `11_Track`, `08_Track`, forward |
| Icaras | `PACK2` | `Icaras` | `11_Track`, `12_Track`, reversed |
| Harimau | `PACK3` | `Harimau` | `12_Track`, `15_Track`, forward |
| Auricom | `PACK4` | `Auricom` | `15_Track`, `08_Track`, reversed |

**The Mirage pack's team is `Mantis` everywhere in the data.** Three independent
sources agree, which is what puts this at the top of its band - and it stays
inside that band, not above it, because none of the three is a runtime
observation:

1. `PACK1.edat`'s manifest declares `<PI_Team name="Mantis">` with
   `location="Data\Ships\Mantis"` and `helpText="MSC_TEAMDES_MIR"` - the id and
   the Mirage string key in one element.
2. On the PS2 disc, `Data\Ships\Mantis\Ship.vex` reads out of `WADS2.WAD` and
   `Data\Ships\Mirage\Ship.vex` does not exist.
3. **The American PSP disc's own string table** maps the id `Mantis` to a
   different display name, in `Data\Plugins\PI008\entries.xml` and its four
   siblings. The name is shipped text and is not reproduced here; that it
   differs from the id is asserted by `dlc_ground_truth.rs`.

So `Mirage` is a display name and was never a folder. That single fact retires a
gap the repository recorded in four places - `HANDOVER.md`, the
[roadmap](../overview/roadmap.md), `exhaust.md`, and
`boost_plume_ground_truth.rs` - all of which said five teams' `Ship.vex` "does
not resolve by name", and three of which concluded `Auricom`/`Harimau`/`Icaras`
were **PS2-only**. They are not: they are PSP downloadable content, and the PS2
release simply bundles what the PSP sold separately. Of the original five,
`Van_Uber` alone was unaccounted for on this disc, and it is a *Pure* team -
not on Pure's base disc (its `Data\Plugins\PI001\Definition.xml` names eleven
`<PI_Team>` entries and `Van_Uber` isn't one; see
[pure-status.md](pure-status.md#van_uber-is-pure-content-not-a-pure-team-on-this-disc)),
but PSN downloadable content for Pure exactly as `Auricom`/`Harimau`/`Icaras`/
`Mantis` were for Pulse. **Confirmed, not just likely**: it is
`data/dlc/WipEout Pure - Gamma Pack 1 (Europe) (DLC).zip`'s `pi.wad`, and its
own folder id has the same underscore-dropped shape as `Mirage`/`Mantis` - see
[below](#pures-packs-decrypt-with-an-external-key-table).

The ids are also the leaf of each `location`, for every team on both discs and
in every pack - which is why `oag_livery::entry::ship_entry_name` can go on composing a path
out of a team id alone.

## The circuits are new, and the four packs interlock

**Confidence: 94.** Each pack ships its own
`Data\Environments\NN_Track\{track[_reversed].vex, zone_track[_reversed].vex,
start_grid.vex, stats[_reversed].xml, TrackStartup.xml}`. None of
`08/11/12/15_Track` is on the base disc under any spelling.

Four circuits, each in two directions, is the eight `PI_Track` entries the four
manifests declare between them - and those eight track numbers are **precisely
the eight missing** from the base definition's own numbering, which runs to 32
and ships 24. The numbering was reserved for them.

The packs split each circuit by direction rather than by circuit: `PACK1` has
`11_Track` forward and `PACK2` has it reversed. A player who owns one pack
therefore holds a `PI_Track` declaration whose geometry is in a pack they do not
have, which is why `oag_game::boot` drops a declared circuit whose `location`
resolves to nothing mounted rather than offering it and failing at the archive.

## Overlapping entries are byte-identical

**Confidence: 92.** Two packs carrying the same environment share exactly two
entries - `start_grid.vex` and `TrackStartup.xml`, which do not vary by
direction - and both are byte-identical between the packs, same stored size and
same SHA-256. Everything else differs by construction, being the forward or the
reversed file.

That is what makes **first mounted wins** a safe rule rather than a silent
choice: within the shipped set it can never pick between two different things,
because there are none to pick between.

## Mounting

`Archives` searches `data`, then `fe`, then each mounted pack in order, so **the
disc always wins**. No shipped pack collides with a disc entry - a pack's whole
content is absent from the base archives, which is what makes it downloadable -
so the order is not load-bearing for real content. It is chosen for content that
is not real: a modified pack that shadowed a disc entry would change the base
game silently.

### Region independence is deliberate

In the original a pack was locked to its own territory's disc. This engine does
not reproduce that, and the rules that keep it that way are:

- Discovery matches on **file shape** - anything that parses as a WAD - never on
  the `PACK*.edat` naming, never on the `UCES00465` folder, and never on the
  booted image's serial.
- The title-id folder is dropped when a zip is unpacked, so nothing downstream
  can come to depend on it.
- `Layout`'s other-title deny-list is not applied to packs. It exists to stop a
  *different game's disc* being opened as Pulse; a pack is not a disc.
- A pack is keyed by the team its manifest declares, never by its pack number.

The evidence that this costs nothing: the American disc already carries string
table entries for all four pack teams, in all five shipped languages. Whatever
the region lock was for, it was not that the other region's build could not name
the content.

See [ADR-0021](../architecture/adr/0021-region-independent-dlc.md).

## Not read yet

- **`downloadNN.xml`**, entry 1 of each main archive: the pack's `PI_Grid`
  championship ladder. Parsed by nothing; progression is a later milestone.
- **`PI_TeamModel`'s `Unlock` pair.** Every other part of the team-model
  schema is read now. `PI_TeamModel`'s own Normal/Concept **hull** axis is
  `oag_title::race::HullVariant`, wired through `--variant`; the
  `PI_ModelSkin`s under `Normal` are `catalogue::Team::skins` and reach a race
  through `--skin` (2026-09-09). What is **deliberately** not collected is the
  `Unlock` pair each carries: **what `loyalty` accumulates is untraced** -
  per-save or per-team, and what the `Team="any"` tier changes about the check
  - so a field holding it would be a number no caller could interpret. Every
  declared skin is offered instead, which is part of what
  `docs/ghidra/functions/psp-pulse-usa/ship-skin.md` labels **chosen rather
  than measured** on this axis.
- **`PARAM.pbp`**, and the `.edat` files' relationship to the PSN download that
  produced them.

## Pure's packs decrypt with an external key table

**Confidence: 94.** Wipeout Pure's seven PSN packs (`A7`, `Delta Pack`, `Gamma
Pack 1`, `GamesRadar Pack`, `Oblivion`, `Omega Pack`, `Voice of Cod`, in
`data/dlc/`) are packaged nothing like Pulse's: one `PARAM.sfo` + `pi.wad` pair
per pack rather than four WAD-shaped `.edat` files, and `pi.wad` measures
8.0000 bits/byte across its whole length - genuinely encrypted, not a
plaintext archive under a misleading extension the way Pulse's is.

**`pi.wad` is a specially-crafted PSP savedata file, not an NpDrm `.edat`.**
Neither `BOOT.BIN` nor any of Pure's 26 bundled PRXs imports a single NpDrm
function in either region (checked directly: `scripts/resolve-psp-imports.py
--modules` reports 33 modules / 278 stubs for `BOOT.BIN`, identical in both
regions, none of them `sceNp` or `scePspNpDrm_user` - where Pulse's carries
both). Nor does either import any hashing or KIRK-wrapper primitive
(`sceKernelUtils*`, `sceChnnlsv`, `sceUtilsBufferCopyWithRange`) that a
standard PGD decrypt would need. That absence is not a dead end here, it is
corroborating evidence for the actual mechanism: the game implements its own
encryption in plain application code, which needs no OS crypto import at all.

The scheme, algorithm and per-pack keys are public, from Thomas Perl's
[`wipeout-pure-dlc2dlc`](https://gitlab.com/thp/wipeout-pure-dlc2dlc) (a region
conversion tool for real hardware, ISC license), fetched 2026-09-01 and kept at
[`data/keys/pure-dlc-keys.txt`](../../data/keys/README.md#pure-dlc-keystxt):

- **Layout**: `pi.wad` is a plain [WAD archive](wad.md) payload followed by a
  256-byte per-**region** signature trailer (checksums plus, per that tool's
  own comment, an encrypted key - not chased further, see below). The payload
  is everything but the last 256 bytes.
- **Cipher**: 8-round XTEA, keyed per-**pack** (one 128-bit key per content ID,
  in `pure-dlc-keys.txt`), used as a stream cipher - `crypt_with_key` derives
  an 8-byte keystream block per 8-byte-aligned file offset by XTEA-encrypting
  `(0x12345678, offset / 8)` with the pack's key, and XORs it over the payload.
  This is symmetric: the same operation encrypts and decrypts.
- **Key selection is by trial, not by name, deliberately**: a pack's content
  ID (e.g. `UCES00001DGAMMAPAK`, read straight out of its own `PARAM.sfo`
  `TITLE` field) is the right key to try, but the **zip's own folder name is
  not reliable** - the Gamma pack's zip unpacks to a folder called
  `UCES00001DGAMMAPACK`, one letter off from the `GAMMAPAK` content ID both
  `PARAM.sfo` and the key table use. The upstream tool sidesteps this by
  trying every key against the first 8 bytes and keeping whichever produces
  `version == 1`; this project's checks below did the same.

**Verified against all seven shipped EU packs, not just cited**: for each,
trial-decrypting the first 8 bytes against every row in the key table finds
exactly one match, and decrypting the whole payload with that key under
8-round XTEA turns it into a file `oag-wad` parses completely unmodified - a
real `version == 1` header, a self-consistent entry count and offset chain,
real LZSS/zlib payloads:

| Pack | Content ID | Entries | Unpacked |
| --- | --- | --- | --- |
| A7 | `UCES00001DA7MUSIC` | 18 | 11 MiB |
| Delta Pack | `UCES00001DDELTAPAK` | 216 | 41 MiB |
| Gamma Pack 1 | `UCES00001DGAMMAPAK` | 192 | 46 MiB |
| GamesRadar Pack | `UCES00001DGAMESRADAR` | 13 | 156 KiB |
| Oblivion | `UCES00001DOBLIVION` | 14 | 7.5 MiB |
| Omega Pack | `UCES00001DOMEGAPAK` + `UCES00001DOMEGAPAKS` (two packs, one zip) | 147 + 38 | 43 MiB + 791 KiB |
| Voice of Cod | `UCES00001DVOCMUSIC` | 10 | 5.0 MiB |

That is an exact arithmetic invariant (the version-field magic, plus a
self-consistent entry table walked by the disc's own unmodified parser) across
seven distinct real files, each keyed differently - the top of the **85-94**
band. It does not reach 95: nothing here is a runtime trace of the actual PSP
running this code, only agreement between a cited external algorithm and this
project's own shipped data.

**This resolves `Van_Uber`.** The decrypted Gamma pack's entry 0 (the manifest,
same pattern as Pulse's `PACKn.edat`) declares `<PI_Team name="Vanuber">`
with `location="Data\Ships\Vanuber"` - **no underscore**, the same
folder-versus-display-name shape as `Mirage`/`Mantis` above, which is exactly
why the underscored `Van_Uber` spelling used everywhere external never
resolved by name-hash. `Data\Ships\Vanuber\Ship.vex` and
`\handlingstats.xml` both hash-match real entries in the pack (`9e62d495` at
entry 139, `7e135ec1` at entry 89), so the team is fully simulatable the same
way Pulse's four DLC teams are - see
[pure-status.md](pure-status.md#van_uber-is-pure-content-not-a-pure-team-on-this-disc).

**Two things this page does not answer, on purpose:**

- **The 256-byte trailer.** `keys.txt`'s own header comment says "each DLC
  has a unique encryption key... stored in the 256-byte signature... encrypted
  with a region-specific key embedded in BOOT.BIN" - and `psp-pure-eu` now
  has code matching that almost exactly (see
  [dlc-download-check.md](../ghidra/functions/psp-pure-eu/dlc-download-check.md)
  for every address and the full trace). `DlcPack_Load` splits a raw pack
  into `[payload][trailer]`; `DlcTrailer_Validate` checks the *decrypted*
  trailer for a literal `"WipeoutPure_____"` magic string, an `"SDRM"` tail,
  `0xFF` padding and a 20-byte digest over its own last 140 bytes; on success
  `DlcTrailer_ExtractKey` copies the per-pack XTEA key straight out of
  **trailer offset `0xb4`** into the field `Xtea_CryptBuffer` uses to decrypt
  the payload. So the per-pack keys `pure-dlc-keys.txt` already lists are, in
  principle, derivable from each pack's own trailer rather than needing to be
  brute-forced - that derivation is the "region-specific key" doing the
  recovering, not a second XTEA key.
  **Recovering the trailer's plaintext is RSA-shaped bignum modular
  exponentiation** (`Bignum_ModExp`/`Bignum_Compare`), with a self-certifying
  public exponent `65537` at `DAT_08aa63fc` and a modulus candidate at
  `DAT_08aa64fc` - genuinely encrypted on disk, confirmed by checking that
  none of `DlcTrailer_Validate`'s plaintext markers appear in a real pack's
  raw trailer bytes. **Not yet runnable end-to-end**: a 128-byte byte-reversal
  and a 256-byte modexp operand count disagree on the real operand width, and
  two exhaustive offline arithmetic sweeps against Gamma Pack 1's real
  trailer, across the plausible width/endianness combinations, both came back
  negative - inconclusive rather than disconfirming, since neither sweep had
  an independently-verified positive control the way a decode-and-check-the-
  version-field test would. The concrete unblock is a live memory dump (which
  buffer a byte-reversal step actually touches), not another offline guess.
  Decryption does not need this trailer at all: `oag-wad` never reads it.
- **`TEST.bin`.** 16 bytes, unread beyond its size and high-entropy-looking
  content. The upstream tool never touches it and every pack above decrypts
  correctly without it, so it is not load-bearing for reading a pack's
  content. **One concrete use is now found**: `BOOT.BIN`'s
  `WowDownload_VerifyPackFiles` (`psp-pure-eu` `0x08955494`, `psp-pure-usa`
  `0x08955b44` - see
  [dlc-download-check.md](../ghidra/functions/psp-pure-eu/dlc-download-check.md))
  checks `TEST.BIN` exists alongside `PI.WAD`/`ICON0.PNG`/`PARAM.SFO`/`PIC1.PNG`
  before treating a download as complete - a presence check only, never a
  content read, so this weakens rather than confirms the savedata-signature
  guess. Whether anything reads its 16 bytes at all, in-game or system-side,
  is still open.

**Wired in.** `oag_game::dlc::ensure_extracted` decrypts a pack the moment it
finds a key that fits, `oag_formats::pure_dlc` and the zlib support this
uncovered in `oag_assets::dlc::Archive::decode` (Pure's `pi.wad` is the first
shipped use of `Compression::Zlib`) are both implemented, and
`oag_pure::open_with_packs` mounts the result behind a Pure source the same
way `oag_pulse::open_with_packs` mounts Pulse's own - see
[ADR-0033](../architecture/adr/0033-external-key-material-for-decryption.md)
for why decryption happens in-process rather than as an offline tool, and
`crates/game/tests/pure_dlc_ground_truth.rs` for the ground-truth coverage.

## Reading one

```sh
# Unpack once (the game does this itself into data/cache/dlc).
unzip -j "data/dlc/WipEout Pulse - Harimau Pack (Europe) (DLC).zip" -d /tmp/pack

just wad list /tmp/pack/PACK3.edat
just wad cat  /tmp/pack/PACK3.edat 0                       # the manifest
just wad cat  /tmp/pack/PACK3_UI1.edat 'Data\Ships\Harimau\handlingstats.xml' --expand

# Or straight into the viewer and the game, which need no unpacking:
just view /tmp/pack/PACK3.edat --mesh 'Data\Ships\Harimau\Ship.vex' --screenshot out.png
just play --race --team Harimau --screenshot out.png
```
