# What Pulse never authors, and the log lines that used to say it was missing

2026-10-06. A race on Pulse (PSP or PS2) used to log a WARN for every effect, texture
and cue its loader asked for and did not find. Each line is either a **cause fixed**
or an absence **proven by design**; this page holds the evidence for the second kind
and the fixes for the first, so no line was demoted without a reason. The census
tools are the ones this project already has: `oag-wad list`/`hash`/`extract`,
`oag-unpack list`, and `grep -a` over the executables.

## Seven `Data\Psys` names Pulse does not author, and the eighth that only the PS2 does

`oag_title::engine_effects` is the engine's shared name list. It claimed to be
"as Pulse's executable names them", and eight of its entries are not. Pulse's
`Data.wad` (PSP) and `WADS2.WAD` (PS2) hold 35 and 41 `SYSP` blobs, every one named by
its own hash ([pob.md](pob.md#recovering-all-35-names-by-their-own-hash)), and none of
them is any of these.

| Name | Where it comes from | PSP | PS2 |
| --- | --- | --- | --- |
| `WO_PLASMA_LIGHTNING_EXPAND` | HD's plasma detonation (`WeaponExplosions_Start`) | absent | absent |
| `WO_PLASMA_LIGHTNING_COLLAPSE` | HD, its other half, 1.3 s later | absent | absent |
| `WO_TRAIL_HITSHIP`, `_RED` | HD's `Trail_HitShipEffect`; `oag_fx::exhaust::hd` | absent | absent |
| `WO_LEACHBEAM_ABSORB` | HD's `LeachBall_Advance` | absent | absent |
| `WO_MAGSTRIP_SPARKS`, `_ZONE` | 2048's ship constructor (`GameMode_IsHdLineage` false) | absent | absent |
| `WO_SHIP_ENGINEFLARE` | the PS2's own authored flare | absent | **present** |

How "absent" was established, and how "never requested" was:

1. **By hash, in every archive of both discs.** The eight names hash to `498f3499`,
   `4bb1faff`, `636b4809`, `fc0c5fc0`, `734a2af7`, `7af37ef8`, `51fa2612` and
   `0afe819a`. Against the entry directory of every WAD on both discs (`Data.wad`,
   `FE.wad`, `FEData.wad`, `BEData.wad`, and `WADS2.WAD`, `WADSP.WAD`, `PRERACE.WAD`,
   `PS2MUSIC.WAD`) exactly one hit exists: `0afe819a` in `WADS2.WAD` (the PS2 engine
   flare). `PRERACE.WAD` and `PS2MUSIC.WAD` print no `entries` summary line from
   `oag-wad list`, so their zero is a directory grep, not an extracted census.
2. **By content, so "under another name or directory" is ruled out.** `Data.wad`
   (1,138 entries), `WADS2.WAD` (7,200) and `WADSP.WAD` (193) were extracted and every
   blob searched for `HITSHIP|LIGHTNING|LEACHBEAM_ABSORB|MAGSTRIP|ENGINEFLARE`. The
   only hits are three large non-effect blobs (the string `LIGHTNING` in two, the cue
   name `PLASMAHITSHIP` in the weapon sound bank), one
   `.POB` whose emitter reuses an `EngineFlare` *texture directory*
   (`WO_MISSILE_BOUNCE`'s art, not an effect), and the PS2's own
   `WO_SHIP_ENGINEFLARE.POB`.
3. **By request.** Neither `BOOT.BIN` (PSP, USA and EU) nor `SCES_547.48` (PS2)
   contains any of the eight strings, nor any of the eight hashes as a 32-bit value
   in either byte order. None of the eight appears there. This is the weaker
   leg on its own: [pob.md](pob.md) records that a continuous effect such as the
   engine flare is absent from its executable's string table as well and is
   triggered anyway. It is sufficient here because legs 1 and 2 show the file does
   not exist, so no request could be satisfied; leg 3 only says nothing in the
   executable even asks.
4. **Where each came from instead.** HD plays the lightning pair, the trail pair
   and the leach burst
   ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md),
   [engine-trail.md](../ghidra/functions/ps3-hdfury-eu/engine-trail.md)); 2048 plays the
   magstrip pair (`crates/2048/src/lib.rs`, `crates/game/tests/magstrip_wire_2048_ground_truth.rs`).
   Pulse draws its plasma ending from `WO_PLASMA_FLASH`
   ([plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md)) and has no magstrip wake
   particle effect (its `MagEffect1/2.vex` models are a different mechanism).

**The fix is Pulse's table, not the list.** `crates/pulse/src/effects.rs` chains
`Effects::without` for the seven, and scopes the PS2's engine flare with the new
`EffectSpec::on` (`Platforms::Only(&[Platform::Ps2])`); the loader asks
`Effects::names_on(platform)`. `crates/game/tests/psys_inventory_ground_truth.rs`
(`pulse_psp_asks_for_nothing_its_archive_lacks`,
`pulse_ps2_asks_for_nothing_its_archive_lacks`) fails by name if a name Pulse's table
requests is not in that platform's archive, so re-adding one cannot go unnoticed.
The PSP draws its engine flare procedurally from the `Engine Flare` locator, which
`engine_effects::ENGINE_FLARE_EFFECT` already said and nothing contradicts.

**Left alone, and open:** `Effects::engine()` still carries all of them for Pure, HD,
2048 and Omega. Separating "the engine's names" from "HD's and 2048's" there is
cross-title work in `crates/hd` and `crates/2048`'s lanes. Pure's executable names
none of the seven either (checked, same grep) and its `names()` therefore still asks
for them; that is Pure's table to correct, and was not touched.

## The cannon bolt, muzzle flash and ghost static on the PS2 were a name-lookup gap

`Data\Weapons\Textures\Cannon_bolt.mip`, `Cannon_muzzle_flash.mip` and
`Data\Tex\staticglow.mip` were reported absent on the PS2. `SCES_547.48` itself contains all three `.mip` strings, so the PS2 requests them
and finds them under `.pct`. They are on the disc as
`.pct` (`077abc83`, `fe1eb1c7`, `fd15c258`), the rewrite
[`ps2_texture_name`](../../crates/formats/src/wad.rs) already documents. The two
loaders read through `Archives::read_name` and so never applied it. They now go
through `assets::exhaust_texture`, which reads through `oag_pulse::read_image` and
decodes either format; the PS2 now reports `32x32 .pct, 8 bpp`. Checked, applies:
HD and the Vita/PS4 titles name `.gtf`/`.gxt` and go through `decode_gtf`, unchanged.

## `talonsj~SETREG_01` and `_02` are control cues, silent by design

Both run only opcodes `0x14` and `0x1e`. `0x14` is a bare no-op (`Scream_OpNop14`,
confidence 82, [sound.md](../ghidra/functions/psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08))
and `0x1e` is `Scream_OpSetRegister` (confidence 88, same page), neither of which
starts a waveform. `oag_formats::sblk`'s timeline walk models both as pass-through
([timeline.rs](../../crates/formats/src/sblk/timeline.rs)); a cue whose walk is complete,
starts no grain and passed at least one such opcode is now reported as
`control only: ... authors no sound` at debug, not `play nothing` at WARN. A cue with
an opcode the walk does not read still takes the old WARN. The same reclassification
applies to every title that reaches `load_cue_record` (HD, 2048, Omega), by the same
walk. Run once each (HD `talons_junction`, 2048 `altima`, Omega `tech_de_ra`) and
diffed against the reference: no `play nothing` line moved on any of them (none of
their cues is control-only on those circuits), so this lane changed no 2048 or Omega
log line.

## The blob shadow is only a stand-in when the tier is on

`blob` (`graphics.shadows = blob`, default `off`) is this project's own tier, and on
every title but HD it draws a generated falloff because the disc ships no silhouette:
`blob` `0x3e0` and `textureBlob` `0x3df` are authored zero times in all 415 Pulse
`.vex` files ([shadows.md](../rendering/shadows.md)), and 72 candidate
`Data\Ships\<team>\{textures\,}ambient_shadow.{gtf,mip,pct,tga}` names (six teams,
three directory spellings, four extensions) hash to nothing in any Pulse WAD. The
original draws no blob on Pulse, so there is no disc image to wire; Pulse's own shadow
is the `original` tier's `Dynamic Shadow Occluder` hull, already built.

What changed is *when the log says so*. The load used to warn eight times in every
race that a falloff would be used, although nothing draws it unless the player
selects `blob`. `oag_render::shadow::Silhouette` now records `generated`, the load
logs the census at debug, and `Scene::shadow_geometry` warns once, the first frame
the `blob` tier is on and a generated silhouette is in use. That is the moment the
stand-in becomes visible to a player, so it is the moment it is named. The other titles take the same path, each with its census:

- **HD: proven absent for the two slots.** HD ships nine `ambient_shadow.gtf`
  (`ag_systems`, `assegai`, `egx`, `feisar`, `goteki`, `piranha`, `qirex`,
  `triakis`, `zone`; `scripts/psarc.py list` over all seven archives). The two
  slots that fell back on the reference run are `Icaras` and `Auricom`, which have
  none under any directory. Checked, differs: HD is the one title whose disc ships
  the silhouette, for nine teams.
- **2048: absent for every race team.** The only `Ambient_Shadow` in the Vita
  archives is `data/art/published/hdships/Zone/Textures/Ambient_Shadow.gxt`, a
  `.gxt` and for the Zone craft alone; the loader asks `.gtf` names for
  `feisar2048`, `Piranha2048` and the rest. Checked, applies, not wired: the Zone
  craft's `.gxt` silhouette could be read through `oag_texture`'s GXT decoder.
- **Omega: absent for every race team.** 46,005 names over the nine `.psarc` files
  of the base and patch trees; the only `Ambient_Shadow` are
  `hdships/zone/Textures/Ambient_Shadow.gnf` and `HDShips/Zone_VR/...`, the Zone
  craft alone, as `.gnf`. Same status as 2048.

## Billboard slots: closed for model slots (2026-10-06, billboards lane)

Rendered with `oag-view --mesh`: `Data\Billboards\Pulse_Adverts\goteki\GOTEKI_LANDSCAPE_01.vex` is a
self-contained scene of flat panels plus **logo and lettering as geometry**, with only
generic textures, like the start gantry's lettering
([start-gantry.md](../rendering/start-gantry.md)). The earlier reading here - "a model
that needs a transform, the constructor writes the identity matrix" - is retired: **the
model is drawn through its own camera into a 128 x 128 texture and the circuit's
`billboardN.tga` quad shows that texture**, read off a live PPSSPP GE dump
([billboards.md](../ghidra/functions/psp-pulse-usa/billboards.md), 2026-10-06 section).
Pulse PSP and PS2 now draw it (`oag_raceplay::adverts`), and the load report's
`N billboard-slot placeholder draw(s) suppressed` WARN no longer fires on a circuit whose
slots all name a model (Talon's Junction). It still fires, honestly, on a circuit with a
**colour** slot and a quad for it (`05_Track`: three), because which advert a colour picks
is a pool walk this project has not read. HD is not wired.
