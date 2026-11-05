# Wipeout HD's update chains

Sony's update packages for both HD sources, read directly, and how the engine mounts them. Fury disc
`BCES00664`: 02.10, 02.30, 02.50, 02.51. PSN `NPEA00057`: 02.01, 02.10, 02.20, 02.30, 02.50, 02.51.
Addresses and executables: [update-0251.md](../ghidra/functions/ps3-hdfury-eu/update-0251.md).
Asserted by `crates/hd/tests/hd_update_chain_ground_truth.rs`.

**Every title loads as base game plus patches** (maintainer, 2026-11-05). With no update present
HD opens exactly as it did.

## What a package holds

A package is a PS3 retail `.pkg`; `rpcs3 --headless --installpkg` installs one (it aborts on exit
after installing, on RPCS3 0.0.42-19980; the files land), `rpcs3 --decrypt` opens its `EBOOT.BIN`.
**A chain is not cumulative:** each package overwrites its own files in one install folder,
`dev_hdd0/game/<serial>`, so a complete install is every package in order.

| Package | Fury | PSN |
| --- | --- | --- |
| 02.01 | none held | `data04`, `data05`, `data06`, `GameAd.bik`, `DFEngine.sprx`, trophies, `EBOOT` |
| 02.10 | `data07`, `EBOOT` | `data07`, `EBOOT` |
| 02.20 | | `data08`, `DFEngine.sprx`, `EBOOT` |
| 02.30 | `data08`, `DFEngine.sprx`, `EBOOT` | `data08`, `DFEngine.sprx`, `EBOOT` |
| 02.50 | `data08`, `EBOOT` | `data08`, `EBOOT` |
| 02.51 | `EBOOT` **only** | `EBOOT` **only** |

02.51 alone has neither archive, and its executable names both, so an install of 02.51 alone
leaves the game with no update content. A third party's note that 02.51 carries `data08` is the
installed state, not the package. There are **16** languages in `data08`, as in `data07`, not 17.
Confidence 95: read from the packages. `CATEGORY` tells the installs apart: Fury's `PARAM.SFO` says
`GD` (a disc game's update), PSN's says `HG` throughout, base and updates alike.

## The win table

Compared by entry content, never by file bytes. A PSARC's own digest is of the path, and two copies
of one entry differ in the headers around them.

Fury's chain over the disc, 33 paths, `the_chain_wins_over_the_disc_path_by_path`:

| Served by | Disc alone served | Paths | What |
| --- | --- | ---: | --- |
| `data07` | `DATA00` | 1 | `sound/globalaudioconfig.xml` |
| `data07` | `DATA02` | 4 | `chinese` and `chinesemx`, `.fnt` and `.gtf` |
| `data07` | nothing | 8 | `environments/<fury four and zone_1..4>/audioconfig.xml` |
| `data08` | `DATA00` | 1 | `gui/ingame_definition.xml` |
| `data08` | `DATA02` | 19 | 16 `entries.xml`, `gui/additional_definition.xml`, two `manual_jap_notify*.gtf` |

`data07` ships 30 paths and `data08` 20, of which 17 overlap and `data08` wins all of them. No disc
copy matches an update's: every shadowed path changed. **No model, texture of a race, table or physics
file is in the chain**, so a race reads the same simulation data, and the simulation came out
identical (below).

What each package moved, `each_package_moved_the_paths_named_here`:

| Comparison | Paths that differ |
| --- | --- |
| Fury `data08` 02.30 to 02.50 | `korean/entries.xml` **alone**, confidence 95 (the earlier claim from `diff -rq`, now by content) |
| PSN `data08` 02.20 to 02.30 | 12 |
| PSN `data08` 02.30 to 02.50 | `english`, `french`, `korean` `entries.xml` |
| Fury against PSN `data08`, 02.30 | `english`, `french` |
| Fury against PSN `data08`, 02.50 | **none**, 20 of 20 |
| Fury against PSN `data07`, 02.10 | **none**, 30 of 30 |

PSN 02.01's archives against the disc, 2026-11-05, by content: `data04` differs in one entry of 54
(`fe/trophies/materials/medal.rcsmaterial`, also against the PSN base); `data05` keeps `DATA05`'s 151
paths and differs in 19 (`detonator.gtf`, `zonebattle.gtf`, `fe/rank/materials/medal.rcsmaterial`,
16 languages' `entries.xml`); `data06` is `DATA06` byte for byte.

## Order

Read off 02.51's loader, confidence 85:
[update-0251.md](../ghidra/functions/ps3-hdfury-eu/update-0251.md#archive-load-order). Later loads win,
so `data08` beats `data07` beats `data00` beats the rest. `oag_hd::PATCH_CANDIDATES` lists `data08`,
`data07`, and `ArchiveCandidates::patch` is searched ahead of every base archive, so the first hit
agrees with the last load for every path of the chain. Between base archives our first-hit order is
the one that stood before and is not what the console does (`DATA02` beats `DATA04` and `DATA06` here
where the console loads them later); that is the open question in
[hd-psn.md](hd-psn.md#open), and 02.01's `medal.rcsmaterial` in `data04` loses to `DATA02`'s copy for it.

## How the engine finds a chain

**Folders, no `.pkg` reader.** A player needs RPCS3's installer or a console, as for the PSN base
([hd-psn.md](hd-psn.md#how-a-player-gets-the-folder)), and no key enters this project. Chosen over
decrypting `.pkg` in place, which would carry PS3's package key for a step an established tool does.

- **PSN:** installing updates patches the install in place, so the folder the game is read from
  already holds them and the tail-matched candidates find them. `data05`, `data06` join `extra`,
  `data07`, `data08` the `patch` role, and `data04` is simply the update's.
- **Disc:** a disc cannot be patched, so `dev_hdd0/game/BCES00664` is a folder of its own.
  `oag_source::source::ps3_updates` finds `CATEGORY` `GD` folders beside the image or under
  `extracted/ps3`, and the disc's serial picks the one with its `TITLE_ID`, the rule `.vpk` and PS4
  `.pkg` patches go by. `oag_assets::UpdateFolder` carries it into `Layout::resolve`, which fills the
  `patch` role from it. A `GD` folder is never offered as a game.
- **A partial chain:** `Layout::update_gap` warns at boot when only one archive is there, or an
  update folder has none, the mistake 02.51 alone makes.
- **A server extraction** reads the source alone: the chain is on no image, and it changes nothing a
  race reads.

Disc reads stay where a source's were: the update opens with the source, on the load path, and
`oag_assets::Archives` reads it as any other archive.

Steps for a player: [installing.md](../overview/installing.md#updates-for-wipeout-hd).
`scripts/install-hd-updates.sh` lays the chains out for the test.

## What a player gets

Checked 2026-11-05 on both sources, patched against plain, `--race --track
/data/environments/01_vineta_k/track.vex --team assegai --ticks 400 --hold cross`: tick 400, speed
109.76, position `[-92.4, 37.8, -97.1]`, 821 draws on all four, and on each source the patched frame
is the plain one byte for byte. The load reports differ by the strings line and, on PSN, by one effect found (`WO_TRAIL_HITSHIP_RED`,
`data05`/`06`). Fury's `grid-select`, `cell-select` and `race` pages are identical; the patched PSN `cell-select` gains
small corner marks on the card. All 16 PSN circuit names are unchanged.

**The mix is the audible change.** `hd_mix` reads `data07`'s `GlobalAudioConfig.xml`. Against `DATA00`,
Stereo and Surround alike; every other group, map and filter value holds:

| Map | `user1` (speech) | Other |
| --- | --- | --- |
| `RaceNormal` | 1.1 to 1.2 | |
| `RaceCriticalEnergy` | 1.6 to 1.7 | |
| `PostRace` | 1.1 to 1.2 | `music` 0.7 to 0.6 |
| `DisplayResults` | 1.0 to 1.1 | |
| `SplitScreen` | 1.1 to 1.2 | `user7` (ship) 0.65 to 0.56 |

Speech rises 0.5 to 0.8 dB as a linear gain on every racing map. The eight per-circuit `audioconfig.xml`, which no circuit
had on the disc, author `low_pass="1"`, what a circuit with no file already did, so the distance
filter does not move.

**Strings.** The loader reports the English table served going from 1,898 strings (`DATA02`) to 2,335
(`data08`), and the wording moves with it (`DEATHS`, `DISTANCE TRAVELLED (KM)`). Unique ids: 2,038 on
`DATA06`, 2,039 on `data07`, 2,041 on `data08`; new ones `ONL_AGE_RESTRICTION`, `3D`, `3D_STRENGTH`,
one changed value, `BOOT_HEALTH_WARN_2`. The circuit names already came from the Fury-era copy
(`DATA06`) and now come from `data08`. **`ingame_definition.xml`:** against `DATA00`, the removed `InGame 3D
Settings` screen and a repaired first line, confidence 90. **`additional_definition.xml`:** 3D options
rewritten, one speaker-test row on `FEGlobals` colours. The chain changes no race HUD. Stereo is not
planned, so the 3D rows are not read.

## The two sources after patching

The patched PSN game runs the 02.51 executable, not the 3.00 one `oag_hd::psn::PSN` was measured
on. Its `EBOOT.BIN` is 128 bytes off Fury's and is licensed, so it is unread. The disc build's
loader ([update-0251.md](../ghidra/functions/ps3-hdfury-eu/update-0251.md#archive-load-order)) loads
`data05` and `data06` unconditionally and tests one flag before `data00` and `data07`, always set on
the disc. **So what gates Fury on PSN is not the mount of `data05`/`06`** on that reading; if the
PSN build has a body behind that flag it gates `data00` and `data07`, and no package here ships
`data00`. Confidence 55, an inference across builds. The engine mounts all it finds and keeps
the `PSN` title data, which offers no Fury screen and no circuit `data00` would hold; whether the
original offers more is open.

## Lineage (2048 / Omega)

**Checked, differs.** 2048 mounts its update through the same `patch` role and needs no folder of its
own, `patch-v104/` sits in the source ([patches.md](patches.md)); `update_gap` is `None` on it. Omega lists
its mandatory patch in `data` and `extra` (`oag_omega::archives`), read with the base; nothing moved. The
packages here are PS3's and neither title ships them.

## Open

- Live order on 02.51 (RPCS3 needs firmware 4.x).
- What the patched PSN build offers, which needs its `EBOOT.BIN` decrypted with the player's own licence.
