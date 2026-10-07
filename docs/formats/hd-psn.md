# Wipeout HD, the plain PSN download

**What it is:** `NPEA00057`, Europe, package version 03.00 - the base Wipeout HD
from the PlayStation Store, **without Fury**. It is a source this project opens as
`Wipeout HD` (`oag_hd::psn::PSN`), beside the Fury disc image and under the same
name. Written 2026-10-07 from a census of every file of the package; counts come
from each archive's own manifest (`scripts/psarc.py list`), never from a truncated
listing.

## What the package is

| Fact | Value | Source |
| --- | --- | --- |
| Content ID | `EP9000-NPEA00057_00-WIPEOUTHDBASE000` | PKG header at `0x30`, 36 bytes |
| `TITLE_ID` | `NPEA00057` | installed `PARAM.SFO` |
| `VERSION` / `APP_VER` / `CATEGORY` | `03.00` / `01.25` / `HG` (a hard-disk game, not a disc) | same |
| PKG size and items | 1,090,506,976 bytes, 16 items | PKG header |
| Disc comparison | the Fury disc is `BCES00664`, `VERSION` 01.03, `APP_VER` 02.00, `CATEGORY` `DG` | `PS3_GAME/PARAM.SFO` |

**Not an update.** It is a full package: executable, trophies and four archives. No
update archive exists to mount ahead of a base, so `ArchiveCandidates::patch` stays
empty for HD (the 2048 mechanism of `patches.md` has nothing to do here). v3.00 is
the version the Store serves; it is cumulative inside the archives.

The second download, "Unlock Key", is a 102,400-byte package whose only payload is a
336-byte licence file (`.EDAT`) and two art files, plus a 16-byte `.rap`. None of
that is game data and none of it is read by this project.

## Installed layout (census)

Installed by RPCS3 into `dev_hdd0/game/NPEA00057/`:

| Path | Bytes | Notes |
| --- | ---: | --- |
| `USRDIR/data01.psarc` | 141,534,156 | sound and music, 53 entries; **same size as the disc's `DATA01`, all 53 identical by MD5** |
| `USRDIR/data02.psarc` | 630,597,630 | 5,292 entries; the disc's `DATA02` has 5,486 |
| `USRDIR/data03.psarc` | 303,208,282 | 1,275 entries; the disc's `DATA03` has 1,163 |
| `USRDIR/data04.psarc` | 2,158,895 | 54 entries; **same size as the disc's `DATA04`, identical** |
| `USRDIR/EBOOT.BIN` | 8,713,056 | a SELF (`SCE\0`), encrypted: this project cannot read it |
| `PARAM.SFO`, `ICON0.PNG`, `ICON1.PAM`, `PIC0-2.PNG`, `PS3LOGO.DAT`, `TROPDIR/` | | the HDD-install shape: **`PARAM.SFO` at the root beside `USRDIR`**, no `PS3_GAME` level |

**All four archives are plain `PSAR`.** No EDAT/SDAT (NPD) wrapper: the PKG's own
AES layer is the only encryption and RPCS3 removes it on install. 6,674 entries in
all (53 + 5,292 + 1,275 + 54). Confidence 95 for the counts (asserted by
`crates/hd/tests/hd_psn_ground_truth.rs`).

### Against the Fury disc (name, size, MD5; `just psarc-diff`)

6,503 distinct paths on PSN. 54 are PSN-only; 4,689 disc paths are absent from PSN.
The absent ones are Fury's, by directory: `talons_junction` (396), `modesto_heights`
(657), `amphiseum` (553), `tech_de_ra` (458) - the four Fury environments that have
no entry in the PSN circuit list - plus `detonator` (168), the `_c1` team variants
(72 per team, `DATA06`), `data/fe/images` and `data/fe/flyers`. **The PSN package has
no `DATA00`, `DATA05` or `DATA06`.** Of what it does ship, `data02` is identical to
the disc's `DATA02` on every family except 8 replaced files (a shader, a texture,
scenery data, five ui files) and `data03` has 24 new ships, 6 new tracks and 5
replaced ones. That comparison is against a **Fury pressing**, so "replaced" says
the bytes differ, not that one is newer.

### What the circuit and craft set is (counts from the manifests)

- **8 circuits, each forward and reversed**: `01_vineta_k`, `02_track`, `03_track`,
  `04_chenghou_project`, `05_ubermall`, `10_sebenco_climb`, `12_sol_2`,
  `15_anulpha_pass`. **No `zone_1`..`zone_4`**, no Talon's Junction.
- **12 teams** with `handlingstats.xml` and `ship.vex` (plus `zone`, `test`). No
  `detonator`.
- The front-end definition (`/data/plugins/frontend/definition.xml`) is in `data02`
  (11,133 bytes, 8 teams, 8 circuits) **and** `data03` (17,667 bytes, 12 teams, 16
  circuits). The 16 circuits are the 8 above, forward and reversed - every one
  resolves. Grids `grid_00`..`grid_07` exist; `grid_08`..`grid_15` (Fury's) do not.

## How a player gets the folder

The route is **RPCS3's own package installer**, which is an established tool and
removes the PKG layer. Exact steps are in
[`installing.md`](../overview/installing.md#wipeout-hd-from-the-psn-download). Chosen
over reading `.pkg` in our code: the format is documented and RPCS3 already does it,
and the other package titles here (2048, Omega) are also read from folders a tool
produced. The licence file is the player's own; this project supplies none and
records none.

**Measured:** installing with `rpcs3 --headless --installpkg` produced plain
archives. **Not isolated:** the licence file was in `exdata/` before the first
install, so whether this package installs without it was not tried.

## What this project does with it

`oag_source` offers any folder under `data/extracted/ps3/` that has `PARAM.SFO`
beside `USRDIR/` (a decrypted disc extract keeps these under `PS3_GAME/` and is not
offered; the disc image is its source). European `TITLE_ID`s (`NPEA`, `NPEB`,
`BCES`) sort first. The serial is read from the root `PARAM.SFO`
(`NPEA-00057`).

`oag_hd::TITLE`'s candidates match `USRDIR/DATAnn.PSARC` by tail, case-insensitively,
so one list reads the disc and the install; the disc resolves exactly as before. With
no `DATA00` the bulk role falls to `DATA03`. **Chosen, not measured:** `DATA03` first,
so its 12-team definition is served; had `DATA02` come first the roster would have
silently dropped Auricom, Harimau, Icaras and Mirage. `oag_hd::title_of` returns
`oag_hd::psn::PSN` for a source with no `DATA00`.

### The variant's Title data, each row against what the PSN package ships

| Field | Disc (`TITLE`) | PSN | Why |
| --- | --- | --- | --- |
| `race.track` | `talons_junction` | `01_vineta_k` | Talon's Junction is `DATA00`'s; a bare `--race` failed with "in none of this source's archives" |
| `race.zone` | `Separate(zone_1..4)` | `SameCircuit` | the four Zone circuits are `DATA00`'s. Chosen, not measured |
| `exhaust` | `enginetrail_bluered_triangle` (`DATA06`) | `enginetrail_triangle` (`data02`) | the disc's own comment calls the plain one the pre-Fury build's. Chosen: the PSN executable cannot be read |
| `front_end.team_select`, `track_select` | `Team_/Track_Selection_Definition.xml` | `None` | both are `DATA06`'s; the race box keeps its plain TRACK and TEAM rows. The older `Selection_Definition.xml` the package carries is a different dialect and is not read |

### What draws differently, honestly absent

- **Menu boxes.** `Data\FE\Images\file2.gtf` (the nine-patch, `DATA06`) is absent, so
  `oag-game` warns `menu blocks: ... did not decode, so entries draw with no box` and
  the rows draw as bare text (screenshot below). `data/fe/images/file.gtf` (4,224
  bytes, both sources) is a candidate of a different size (the nine-patch code
  assumes 64 x 64); **unmeasured, not substituted**.
- **Menu backdrop.** The Fury point clouds (`data/FE/Fury/*.points2`) are absent;
  the background is black.
- **Five effects** are missing from `data02` on PSN that the disc's carries
  (`WO_PLASMA_FLASH`, `WO_SHIP_ENGINEFLARE`, `WO_TRAIL_HITSHIP_RED`,
  `WO_LEACHBEAM_ENERGY`, `WO_BLUE_WELDER`); the loader reports each as "will not be
  drawn".
- **Fury craft variants** (`_c1`/`_n1`): absent, so a craft shows its base livery.
- **Ads**: `LSAD_*` flyer art is `DATA06`'s.

### Same circuit, same craft, both sources

`--race --track /data/environments/01_vineta_k/track.vex --team assegai --ticks 400
--hold cross`: **the simulation is bit-for-bit the same on both** (tick 400, speed
104.26, position `[-74.7, 37.9, -96.6]`, 836 draws, 301,772 triangles). The picture
differs where the data does: the PSN hull's livery and a darker track grade, and the
engine trail is the plain ribbon rather than Fury's. Screenshots under
`data/scratch/hd-psn/shots/` (not committed).

## Lineage check (2048 / Omega)

**Checked, differs.** 2048's `WipEout 2048 - WipEout HD` pack is a Vita DLC package
in 2048's container (`rcsmodel`/`gnf` family), and what 2048 re-ships of HD are the
four Zone circuits (`docs/formats/track.md`) - which are `DATA00`'s on PS3 and absent
from this PSN package. Omega's HD-derived front end is HD's own `PI001` carried
forward from the Fury build and reads through the same plugin names; it needs
`Team_/Track_Selection_Definition.xml`, which PSN lacks. Nothing here is ported; the
PSN source does not touch the 2048 or Omega readers. Not byte-compared across
formats: the evidence is the path census above and the existing pages.

## Open

- What draws HD's menu boxes when `file2.gtf` is absent (the executable is an
  encrypted SELF; RPCS3 would show it).
- Whether the PSN executable picks `DATA02`'s or `DATA03`'s `skin.xml`: both are
  mounted, `DATA03` first (chosen).
- Whether a PS3 base HD races Zone on every circuit (`SameCircuit` is chosen).
- A picker for the older `Selection_Definition.xml` dialect.
