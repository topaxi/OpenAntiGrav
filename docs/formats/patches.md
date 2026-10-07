# Patches and DLC packs

What each patch we hold changes, what it contains, and which published patches we do
not hold. Written 2026-10-05 from a census of every entry of every held package.
Tool: `just psarc-diff` (`oag-psarc-diff`, `crates/tools`): it classifies each entry of
one set of PSARC archives against another as **new** (no entry of that path on the base
side), **identical** (same path, same size, same MD5) or **replaced** (same path,
different bytes - the cause is never read). Names, sizes and hashes only; no bytes are
committed. Families are a heuristic over extension and directory.

Census sources: counted from the archives' own manifests, not from a truncated listing.
2048 EU `data.psarc` 18,430 paths and USA 18,441; Omega's nine archives 46,005 paths
(26,408 base in `data00`-`data04`, 19,597 patch in `data05`/`07`/`08`/`09`), which
matches the figure `psarc.md` already records.

## What we hold

| Title | Held | Version | Source of the version |
| --- | --- | --- | --- |
| 2048 EU `PCSF00007` | base, patch, DLC1, DLC2 | base `APP_VER` 01.00 (build 2011-12-06), patch 01.04 (build 2012-06-06), DLC1 01.01, DLC2 01.00 | `param.sfo` of each package |
| 2048 USA `PCSA00015` | base, patch, DLC1 | base 01.00, patch 01.04, DLC1 01.00 | same |
| Omega EU `CUSA05670` | base, patch | base 01.00 (`c_date` 2017-12-24), **patch 01.07** | `param.sfo` and `changeinfo.xml` read out of `omega-ps4-eu-patch.pkg` (confidence 95) |
| HD EU `NPEA00057` (PSN, no Fury) | one full package | `VERSION` 03.00, `APP_VER` 01.25 | installed `PARAM.SFO`; **not an update** - see [hd-psn.md](hd-psn.md), nothing to mount ahead of a base |
| HD/Fury EU `BCES00664` | disc only | disc `APP_VER` 02.00, `VERSION` 01.03 | `PS3_GAME/PARAM.SFO` |
| Pulse, Pure | UMDs, DLC zips | no patches exist | see [dlc-pack.md](dlc-pack.md) |

## 2048 v1.04

**The package is cumulative, and Sony's own changelog is inside it.**
`sce_sys/changeinfo/changeinfo*.xml` (13 languages, EU; 3 in USA) reads, identically in
every language:

| `app_ver` | Sony's note |
| --- | --- |
| 01.01 | Improved Load Times; Various Bug Fixes; New Audio Track, Orbital "Beelzedub" |
| 01.02 | Improved Load Times; Various Bug Fixes |
| 01.03 | **DLC Support**; Various Bug Fixes |
| 01.04 | Various Bug Fixes |

The patch package carries `APP_VER` 01.04 over a 01.00 base, so it is the sum of 01.01 to
01.04: whatever the census finds cannot be attributed to 1.04 alone. Confidence 95.

### Package contents

Both regions: `eboot.bin` (SELF) plus a decrypted `eboot.elf`, three `.suprx`, `PSP2/data1.psarc`
and `PSP2/data2.psarc`, `sce_sys` (changeinfo, trophy, `about`, keystone). USA adds the
`sce_sys/manual` pages (80 per language set). **The EU and USA `data1`/`data2` are the same
archives entry for entry**: 854 + 1,526 paths, all identical by MD5. They differ from their
own base only because the two bases differ (EU base has 11 audio files USA lacks, USA has a
different manual, intro video and 21 XML files).

### Census against the base `data.psarc` (EU)

| Archive | Paths | New | Replaced | Identical |
| --- | --- | --- | --- | --- |
| `data1.psarc` | 854 | 101 | 734 | 19 |
| `data2.psarc` | 1,526 | 394 | 1,122 | 10 |
| both | 570 paths in both, 397 of them the same size (bytes not compared) | | | |

USA against its own base: `data1` 70 new / 686 replaced / 3 identical, `data2` 347 / 1,063 / 3
(counts from the same tool; the labels move because the USA base differs from the EU base on some
paths). The patch archives are byte-identical between regions, 854 + 1,526 of 854 + 1,526 by MD5.

By family (EU, entry counts, `data1` / `data2`):

| Family | `data1` | `data2` |
| --- | --- | --- |
| shaders (`.rcsmaterial`) | 17 new, 664 replaced | 19 new, 492 replaced |
| ships | 13 new | 48 new, 435 replaced, 2 identical |
| tracks | 14 new, 1 replaced | 46 new, 128 replaced |
| ui | 33 new, 17 replaced | 56 new, 2 replaced, 1 identical |
| xml tables | 24 new, 29 replaced, 19 identical | 21 new, 44 replaced, 1 identical |
| audio | 0 | 182 new (`.at9`), 2 replaced (`.bnk`) |
| textures, models, scenery data | 23 manual pages replaced | 12 models and 6 `.pob` replaced, 17 `.pob` new, 7 textures |
| executable | n/a, in `eboot.elf` below | |

Where the new and replaced entries are (by directory, EU):

- `data/art/published/hdships/...`: 48 new and 433 replaced, 2 identical (`data2`). This is the
  HD/Fury craft set: the base already ships 38 `EngineFlare` pairs for it, the patch re-ships 24.
- `data/art/published/DLC1/...`: 172 replaced materials (`data1`) and 16 new files (`data2`).
  **The base `data.psarc` already carries `DLC1` paths and `data/audio/DLC1/*.bnk`**, and 17 of
  the DLC1 pack's audio banks are byte-identical to the base's copies: the base ships the
  pre-release skeleton of the add-on circuits, and the patch and the packs overwrite it.
- `data/art/published/environments/...`: 307 replaced (`data1`), 396 replaced and 11 new (`data2`).
- `data/xml/SP.xml` replaced in both archives (1,316,362 against 1,316,363 bytes: one byte);
  `data/plugins/grids/grid_00`-`15.xml` replaced (each 4-130 bytes larger);
  `plugins/teams/Definition.xml` replaced (same size); `xml/Hardcore.xml`, `Premium.xml`,
  `MjolnirSchema.xml` and `MjolnirVersion.xml` new. Cause unread.
- `data/ZoneEnvironmentHDFury/{Zone,Detonator}ModeHDFury{,DLC3}.effectSettings`: four new files
  (see [effectsettings.md](effectsettings.md)).
- `data/audio/sound/streams/atrac/<13 languages>/FE_TEAM_*.at9`: 182 new, 14 teams by 13
  languages. The base has none. Team-name announcer clips; the 14 codes include the HD teams.
- `data/particles2048/`: 17 new `.pob` and 5 sprites.
- `data/Tex/zoneModeTrack*.gxt`, `detonatorModeTrack*.gxt` (30 files): replaced, 262,208 bytes each.
- `data/audio/sound/speech.bnk` and `speech_fe_NGP.bnk`: replaced (+4,704 and +4,080 bytes).

### The HD-named and `_NGP` banks

The base ships both sets (`weapons.bnk` 4,992,512 B, `speech.bnk`, `shipHD.bnk`, and the
`*_NGP.bnk` set). **The patch touches the HD-named set once, `speech.bnk`, and one `_NGP` bank,
`speech_fe_NGP.bnk`; it does not touch `weapons.bnk` or `shipHD.bnk`.** The DLC packs ship
`data/audio/DLC1/weapons_det.bnk` and `speech_det.bnk` (the Detonator banks) as *replaced* against
the base's copies, and 15 to 17 other `DLC1` banks identical to the base. So the HD-named
banks are the base's own, the HD add-on does not replace them, and only the Detonator pair is
re-authored. Cause unread.

### The executable

`eboot.elf` is a plain ELF in both regions (magic `7F454C46`, three program headers), so
strings are readable; `eboot.bin` is a SELF (`SCE\0`), not read.

| | base (EU) | patch (EU) |
| --- | --- | --- |
| `eboot.elf` size | 5,476,656 | 5,523,699 |
| code `LOAD` (`0x81000000`) | `0x508cc0` | `0x5181e8` (+65,320) |
| data `LOAD` file / mem size | `0x2b330` / `0x4d1578` | `0x26ae0` / `0x4ce7c0` |
| build stamp string | 11:13:30 Nov 15 2011 | 10:20:48 Mar 21 2012 |
| `eboot.bin` size | 2,627,328 | 2,616,928 |

USA base/patch `eboot.elf`: 5,460,256 / 5,523,699. **The two patch executables are the same
length with different bytes**, so one build serves both regions with the title-id literals
changed (hash differs, size equal; the cause was not diffed).

Printable strings only in the patch (334 after filtering code noise) include: the
mounting of `addcont0:DLC1W2048PACKAGE/PSP2/dlc1.psarc`, `DLC2W2048PACKAGE/.../dlc2.psarc` and the
literals `data1.psarc`, `data2.psarc`; `sceAppUtilDrmOpen failed .. no DLC1/DLC2/**DLC3** pass`;
`HD campaign event`, `Fury campaign event`, `Added %d HD canvas buttons` / `Fury` / `2048`,
`TouchCampaignHD`, `TouchCampaignFury`, `ProfileHDCampaign`, `offlinehd`, `offlinefury`;
`GameMode_CheckPointRace`, `m_numberOfCheckPoints`, `m_timePerCheckPoint`;
`data/art/published/hdships`, `PlayerLiveryHD`, `PlayerSkinFury`, `fe_fury_livery_normal`;
`wo_composite_zone_hdfury_{vp,fp}`; `data/ZoneEnvironmentHDFury/*`; `Data/art/published/DLC3/Plugins/tracks`;
`manualDLC1`, `data/Books/ManualDLC2`; the ranking and leaderboard code (`DoPostRaceRankingsUpdate`,
`Backend/Ships/GhostShipTransfer.cpp`); `HasDemoVersion` and the 14 `DEMO_*_upsell` images.

Printable strings only in the base (249): the debug and stress-test text (`MPSTRESS`, `Invalid Net
Msg Size: ...`), the `FaceRecognition` data paths and shaders (face-recognition code is gone), and
the `data/Tex/zoneMode*.gxt` / `DetonatorMode*.gxt` literals (renamed `...Track*` in the data).
The base executable already knows `DetonatorModeDLC3.effectSettings`, so "DLC3" is older than the patch.

### Verdict on the hypothesis "v1.04 is mostly the HD/Fury DLC's compatibility and setup (confidence 60)"

Split into two claims, because the package is cumulative:

1. **"The patch package is mostly add-on support": raised to 75.** Evidence for: the executable
   gains DLC mounting (1.03's documented "DLC Support"), HD and Fury campaign screens and
   profile items, HD livery and skin names, the HD Fury Zone shader and `effectSettings`,
   `hdships` (481 entries new or replaced) and the add-on circuits' skeleton (`DLC1`). The 2012-06-19
   add-on packs (HD and Fury, per the sources below) are exactly what these name. Evidence against
   "mostly": 1,156 `.rcsmaterial` replaced across the two archives (664 + 492) and over a thousand replaced
   ship, track and texture files have no add-on explanation in the census, and 1.01/1.02's own
   notes say load times, which a re-export of materials and models would explain. Cause unread.
2. **"1.04 itself is DLC compatibility": killed (confidence 85).** Sony's note for 1.04 is only bug
   fixes, and "DLC Support" is 1.03. The package cannot separate 1.04 from 1.03.

Not found: the Orbital "Beelzedub" track of 1.01. No new music stream exists in either archive
(the only new `.at9` are `FE_TEAM_*`), and nothing in the paths or strings carries the name; it may
sit inside an existing `music/NN/music_stereo.at9` or in the base already. Open.

### DLC packs against base and patch

Counts against the base `data.psarc`; the same run against base plus patch changes only `data1`/`data2`
overlaps. EU `dlc1.psarc` 1,903 paths and `dlc2.psarc` 1,721.

| Pack | New | Replaced | Identical |
| --- | --- | --- | --- |
| EU DLC1 (HD add-on; circuit directories Anulpha_Pass, Chenghou_Project, Metropia, Moa_Therma, Sebenco_Climb, Sol_2, Ubermall, Vineta_K) | 1,591 | 290 | 22 |
| EU DLC2 (Fury add-on; amphiseum, modesto_heights, talons_junction, tech_de_ra, zone_1 to zone_4) | 1,194 against base+patch (1,210 against the base alone) | 3 | 16 (against base+patch) |
| USA DLC1 | 1,602 | 290 | 11 |

EU and USA DLC1 are identical archive for archive (1,903 of 1,903 by MD5), so the `VERSION` 01.01 (EU) against
01.00 (USA) in `param.sfo` is metadata only. Which pack is which: DLC1 is the HD add-on and DLC2 the HD Fury
add-on per the sources below (confidence 80, by track directories). The packs replace 107 `DLC1` track files
of the base and re-ship the base's own banks; neither pack replaces anything the patch ships beyond the
Detonator banks above.

## Omega patch

The patch is **version 01.07**, not 1.04 (confidence 95). Its `changeinfo.xml`, read from the package's
own entry table (`0x1260`), lists:

| `app_ver` | Note (verbatim) |
| --- | --- |
| 01.01 | Added Pure Racing mode to online multiplayer. Added Turkish language support. Minor bug fixes. |
| 01.02 | Fixed leaderboard glitches. Changed photo mode UI hiding behaviour. Fixed HD zone ship health indicator. Miscellaneous bug fixes. |
| 01.03 | **Added support for PlayStation VR. Added 3D audio support for PlayStation VR. Added additional remix music track.** |
| 01.04 to 01.07 | Miscellaneous bug fixes (each) |

The PlayStation Blog's "Update 1.04" of 2018-03-28 therefore matches the file's 01.03 content
(the public name and the file's `app_ver` disagree by one; the file is the primary source). The patch
is `SYSTEM_VER` 0x04700000 (4.70), build tool 2018-era `sdk_ver` 04508000; base `sdk_ver` 02508000.
The base package has no `changeinfo`. The package extracted under `data/extracted/ps4/omega-eu-patch` has
no `sce_sys/param.sfo`; read it from the `.pkg` entry table (the helper used is in the scratch notes).

### Census against base (`data00`-`data04`)

| Archive | Paths | New | Replaced | Identical |
| --- | --- | --- | --- | --- |
| `data05` | 8,205 | 17 | 7,976 | 212 |
| `data07` | 6 | 0 | 6 | 0 |
| `data08` | 11,266 | 676 | 9,360 | 1,230 |
| `data09` | 120 | 6 | 44 | 70 |
| patch total | 19,597 | 699 | 17,386 | 1,512 |

Replaced by type: **15,640 `.rcsmaterial`** (a wholesale recompile, 2.6 GB; cause unread), 1,392 `.gnf`,
119 `.pob`, 115 `.xml`, 40 `.EnvSettings`, 38 `.txt`, 12 `.rcsmodel`, 11 `.bnk`. New by type: 283 `.gnf`,
139 `.xml`, 100 `.vex`, 94 `.rcsmodel`, 47 `.rcsmaterial`, 20 `.wem`, 10 `.fnt`.

**VR: yes, carried.** New ship families under `data/art/published/`: `HDShips/VR_HD`, `VR_FURY`,
`VR_FURY_n1`, `Detonator_VR`, `Zone_VR` and `ships/VR_2048/{1,2,3,4}` (409 new ship entries in all), plus
`data/fe/FE.BGAnimationsVR.Settings`. That is nine directories, not the eight models the blog
named; the count is of directories, so the mapping to "models" is open. The executable gains
`sceHmd*` (`sceHmdOpen`, `sceHmdReprojection*`), `libSceHmd`, `libSceHmdSetupDialog` and VR settings
(`VR Comfort`, `VR Warning Locked To Pilot`, `Ship Effects VR.Internal Camera Scale`). VR is not planned.

**3D audio: where it lives.** The base executable already imports `libSceAudio3d` and carries the Wwise
`AK::SceAudio3dSink`. The patch executable adds `SCE_Audio3d_Ambisonic_Output`. In data, the Wwise banks
that changed are `Init.bnk` (28,788 to 35,121 bytes), `Ship_NGP.bnk` (44.6 MB to 51.2 MB), `Music.bnk`
(+39,802), `Weapons_NGP.bnk` (38.1 MB to 37.6 MB), `English(US)/Speech_NGP.bnk`, `speech_fe.bnk`,
`speech_zone_NGP.bnk`, `env0_det.bnk`, `frontend_fliers.bnk`, and `frontend.bnk` (898 KB to 22.7 MB in
`data08`). Twenty new `.wem` files (11 of 6,614,272 bytes and 9 of 7,434,752 bytes)
are the only new Wwise media. Which of these implement 3D audio, and which is the "remix" track, is not
read: the connection is by timing and by name only (confidence 50). The "Shake It" remix by name is not
found in any path or string (confidence 30 that it is among the twenty `.wem`).

Both Omega executables are SELF files with readable strings (`eboot.bin` 10,493,719 to 11,004,033 bytes);
`libSceNpToolkit2.prx` grows 593,934 to 693,254 and the two other `.prx` shrink slightly.

Omega check of the 2048 finding: Omega's patch does the same thing as 2048's - it replaces most of the
base's material and model entries and adds HD-lineage ships - but the replaced set is 25 times bigger
(15,640 against 1,156 materials). **Checked, differs**: scale, and the cause for 2048 is documented
(add-on support), for Omega only inferred (VR, confidence 50).

## HD/Fury: no patch on the disc

`PS3_UPDATE/PS3UPDAT.PUP` is the console firmware (`SCEUF`, entry `0x100` reads `2.76`), not game data;
`PARAM.SFO` says `APP_VER` 02.00, `VERSION` 01.03, `PS3_SYSTEM_VER` 02.7600. The seven `DATA00`-`DATA06.PSARC`
are the whole game. `EBOOT.elf` carries `ONL_MSG_CHECK_PATCH` / `PATCH_DOWNLOAD` and `/app_home/DLC3/...`
literals: the patch and DLC hooks exist, with no payload on the disc (confidence 90). No later HD update was
ever on the disc, and we hold no HD patch.

## Published and not held

Searched 2026-10-05; nothing was downloaded. Confidence is in the source, not in a measurement.

| Title | Item | What it is | Held? | Source |
| --- | --- | --- | --- | --- |
| HD Fury PS3 `BCES00664` | **patch 2.51**, said to be the last | crossplay with 2048, a fix for a rare freeze on ship select, online stats restored | **no** | PlayStationTrophies thread "Patch 2.51" via search (page not fetchable), RPCS3 wiki and forum list 2.51 for BCES00664; confidence 70 |
| HD PS3 | 2.00 | new statistics section, new ship and track selection screens, community and online features | no | search summary of the PlayStation Blog; confidence 55 |
| HD PS3 | 2.10 (2009-10-28) | small front-end and audio fixes, Eliminator and Detonator score fixes, adverts no longer affect load times | no | [PlayStation Blog, "WipEout HD Patch 2.10 Incoming"](https://blog.playstation.com/archive/2009/10/28/wipeout-hd-patch-2-10-incoming); confidence 80 |
| HD PS3 | 1.xx and 2.2x-2.5x between | not found | no | open |
| 2048 Vita | 1.01, 1.02, 1.03 | in the held package's changelog above | the 1.04 package contains them | the package |
| 2048 Vita | later than 1.04 | none found in any search; Sony's changelog stops at 01.04 | n/a | confidence 65 |
| 2048 Vita | **DLC3** | the executable names `W2048DLC3PACKAGE` and `Data/art/published/DLC3/Plugins/tracks`; no published pack of that name found | no | open; the two published add-on packs (HD, HD Fury, 2012-06-19, free to PS3 owners) are the held DLC1 and DLC2 ([PlayStation Blog](https://blog.playstation.com/2012/06/19/two-wipeout-2048-dlc-packs-on-psn-today/), [Engadget](https://www.engadget.com/2012-06-19-wipeout-hd-and-fury-come-to-vita-as-2048-dlc-free-if-you-alread.html)) |
| 2048 Vita USA | DLC2 (HD Fury add-on) | the USA store sold both packs | **no, USA DLC2 absent** | same sources; confidence 80 |
| Omega PS4 | 1.01 to 1.07 | list above, from the package | 1.07 is the held patch; each earlier update is subsumed | the package; the search results name 01.02, 01.04 to 01.07 as seen in the wild, 01.07 as current with minimum firmware 5.55 |
| Pure PSP (EU) | Classic Pack 1 and Classic Pack 2 | classic music, ships and skins; Altima VII and Odessa Keys, Porto Kora and Vohl Square | **no** | [wipeout.fandom Downloadable Content](https://wipeout.fandom.com/wiki/Downloadable_Content) via search, Busy Gamer Nation (US 2005-08-01); EU shipped two packs; confidence 60 |
| Pure PSP | Gamma Pack 2 | US only (2005-06-22); the EU Gamma Pack was one combined pack | not applicable to EU | confidence 55 |
| Pure PSP | Gamma, Delta, Omega (EU only), Oblivion, A7, Voice of Cod, GamesRadar | all held (7 zips) | yes | [dlc-pack.md](dlc-pack.md) |
| Pulse PSP | Auricom, Harimau, Icaras, Mirage | all four held; the search names no others | yes | wipeout.fandom, PSDeals; confidence 75 |

## Open

- Whether the 2048 patch's `data1` and `data2` are layers (570 shared paths): which wins at runtime.
- Cause of the 1,156 replaced 2048 `.rcsmaterial` and 15,640 Omega `.rcsmaterial`: needs a material diff.
- Where Orbital "Beelzedub" and Omega's "Shake It" remix live (inferred wems and music banks are unread).
- Which of the 20 new Omega `.wem` and the changed Wwise banks implement the 3D audio.
- 2048 DLC3 pack: existence; HD 1.xx/2.xx between 2.10 and 2.51.
- The game opens only `PSP2/data.psarc`: none of the 2048 patch is read by the engine here (see
  [2048-status.md](2048-status.md)); the patch's `hdships`, `SP.xml` and grid revisions are unused.

## What each title mounts (2026-10-07, `title-patches` lane)

| Title | Update content in `data/` | Mounted | Order and evidence |
| --- | --- | --- | --- |
| 2048 EU / USA | `patch-v104/PSP2/data1.psarc`, `data2.psarc` (identical in both regions) | **yes**, as `ArchiveCandidates::patch` | `data2`, `data1`, `data`, then `dlc1`, `dlc2`. The v1.04 executable mounts `data`, `data1`, `data2`, `dlc1`, `dlc2` at one FIOS2 mount point ([archive-mount.md](../ghidra/functions/vita-2048-eu-v104/archive-mount.md), confidence 80); the base is shadowed by the patch (Vita3K, confidence 85); `data2` over `data1` and DLC against patch are **chosen, not measured** |
| Omega EU | patch v1.07: `data05`, `07`, `08`, `09` | yes, all four | `data09` (bulk), then `data08`, `data07`, `data05`, then base `data00` to `data04`; patch ahead of base is **chosen, not measured** (no PS4 observed). None of the 19,597 patch paths is hidden behind a base copy by this order; order among the four patch archives is unmeasured |
| HD / Fury EU | none: `PS3_UPDATE/PS3UPDAT.PUP` on the disc is system firmware | n/a | not checkable (no game update held, disc is 1.03) |
| Pulse (PSP, PS2), Pure | none exist | n/a | not checkable; their DLC zips are mounted as packs, see [dlc-pack.md](dlc-pack.md) |
