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
the version the Store serves. Its `VERSION` (03.00) is newer than the Fury disc's (01.03) while
its `APP_VER` (01.25) is older than the disc's (02.00): they are different numbering
schemes, and nothing here says which build is later.

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
archives; `--no-gui` with the same arguments sat idle and installed nothing. **Not
isolated:** the licence file was in `exdata/` before the first install, so whether
this package installs without it was not tried.

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

### The executable decrypts, and says what this build names

`USRDIR/EBOOT.BIN` is a SELF. RPCS3's own `--decrypt` turns it into a plain ELF
(8,710,312 bytes) in a private profile that holds the player's licence file; it
decrypts only with that licence in place, and **not tried without it**. `strings -a`
over the result, against the same over the Fury disc's `EBOOT.elf`, 2026-10-07:

| Literal | PSN executable | Fury disc executable |
| --- | --- | --- |
| `Data\FE\Images\file2.gtf` | absent | present |
| `Data\FE\Images\file.gtf` | **present** | absent |
| `Data/RibbonEffects/enginetrail_triangle.vex` | **present** | `enginetrail_bluered_triangle.vex` |
| `Detonator` | 0 lines | 109 lines |
| `talons_junction` | 0 | 8 |
| `_fury` | 0 | 112 |
| `points2` (Fury backdrop) | 0 | 32 |
| `Selection_Definition` (either dialect) | 0 | 0 |
| `data0N.psarc` | 0 | 9 |

So this build has **no Detonator mode and no Talon's Junction at all**, names the
plain exhaust template and the `file.gtf` menu frame. Confidence 85: the literals are
read, the pointer table that makes `file.gtf` the block frame is not (the disc's
`file2.gtf` is at `0x00920a00`; the PSN build was not walked), and "present" says the
string exists, not how it is used. `file.gtf` is 64 x 64 `DXT3` (4,224 bytes, in `data02`
on the disc too), so the nine-patch geometry the disc's `file2.gtf` (64 x 64
`A8R8G8B8`) feeds applies unchanged.

### The campaign

**Same campaign as the disc's HD branch, measured by diff.** Read from the
install and the disc (`data/scratch/hd-psn-campaign/`, cmp over every file):

| What | PSN | Disc | Result |
| --- | --- | --- | --- |
| `data02` `Definition.xml`, `grid_00`..`07`, `CellMode_Definition.xml` (42,548 bytes), `Selection_Definition.xml` | `data02` | `DATA02` | **byte-identical**, all of them |
| `data04` `Definition.xml`, `grid_00`..`07` (per-difficulty schema) | `data04` | `DATA04` | **byte-identical** |
| `data04` `grid_00`..`07` against `DATA06`'s HD grids | `data04` | `DATA06` | identical bar one attribute: `DATA06`'s `<Values>` carries `Campaign="HD"`, the PSN copy does not (there is no second campaign to tell it from) |

So the eight events, tracks, classes and medal targets are the disc's `grid0`..`grid7`,
confidence 95 (full-file comparison, no field-level reading needed). What the PSN
package does **not** carry is `DATA06`: the later build of `CellMode_Definition.xml`
that adds `Campaign Selection` and `Grid Selection Fury`. Its one copy is the older
one. `oag_title::Campaign::screen_archive` says which archive's copy to read
(`Some(DATA06)` on the disc, `None` here, meaning the copy the mounts serve), and
`load_hd` opens straight on `Grid Selection` whenever the pair `Campaign Selection` /
`Grid Selection Fury` is not both authored. No new reader: it is the disc's HD
reader over the older copy, and the older copy's `Grid Selection` and `Cell
Selection` draw through the same draw list as `DATA06`'s.

**Measured on RPCS3 (2026-10-07, confidence 90):** the PSN build booted from its own
install (`rpcs3 --no-gui <EBOOT.BIN>`, a private profile copy, `Vulkan` on lavapipe,
audio renderer `Null`, about 5.7 fps, so every key is held about a second) goes
health warning, "Are you new", autosave notice, **MAIN MENU** (`CAMPAIGN` first),
confirm, **`EVENT 01/08`** (no HD/Fury list in between), confirm, **`CHOOSE RACE`**.
Frames: `data/scratch/hd-psn-campaign/shots/rpcs3-psn-main.png`, `rpcs3-psn-grid.png`,
`rpcs3-psn-cell.png` (not committed). Not measured: a second entry point.

**Ours, walked live** (windowed on `:96`, software Vulkan, `--no-audio`, a private
profile): RACE CAMPAIGN clicked with the pointer opens `Grid Selection`
(`shots/live-1-after-click.png`), a click on the card opens `Cell Selection`
(`live-2-cell.png`), a click on the open hexagon launches Vineta K, lap 1 of 3, with
opponents (`live-4-after.png`). `--menu-page grid-select` and `cell-select` match the
disc's HD frames at the layout level (`shots/psn-grid-select.png`, `disc-grid-select-hd.png`).

**Differences from the RPCS3 frames, and whether the disc has them too.** The
`Cell Selection` layout is the right one (same panels, same positions), so the data
is right; the art differs:

- no `Event`/`Track`/`Speed Class`/`Weapons` emblems (RPCS3 draws an icon above each
  value) - the disc's `cell-select-hd` has the same gap: **shared**, not PSN's;
- values drawn grey where RPCS3 draws them teal - same on the disc's still: **shared**;
- `TARGET` has no `(NOVICE)` suffix where RPCS3 shows `TARGET (NOVICE)` (the AI difficulty); ours draws the Gold/Silver/Bronze rows in their medal colours, RPCS3 in teal with no icons. The disc's `DATA06` layout draws medal icons instead, so this one is **PSN-specific**: the older `Target0/1/2` widgets, not compared further;
- the hex lattice (`Bg_x_y`, `CM_HEX_Bg` = `0x50646464`, defined in `skin.xml` in
  every copy) is a faint dark grey on our black background; RPCS3's frame is white
  behind it on every screen of that run (including the main menu, so a property of
  this lavapipe run, not the game's data), so it reads as a different picture.
  Not a measured gap.

None of these was chased: the lane is the reader and the wiring.

**Medal atlas.** `Hexmedal_HD.gtf` has two copies on PSN as on the disc: `data02`'s
flat 1024x256 (262,272 bytes) and `data04`'s per-difficulty 1024x768 (786,560). The
disc read prefers `data04` through an archive-label suffix match, written for the
disc's `PS3_GAME/USRDIR/DATA04.PSARC` and silently missing the PSN's
`.../USRDIR/data04.psarc`: the PSN campaign fell back to the flat atlas. It matches
by file name, case-folded, now (`same_archive`). Grid files: `read_name` serves
`data02`'s flat copy on both sources (parity with the disc, unchanged); the PS3's
own last-wins overlay would use `data04`'s per-difficulty ones, which is open on
both and not changed here.

### The variant's Title data, each row against what the PSN package ships

| Field | Disc (`TITLE`) | PSN | Why |
| --- | --- | --- | --- |
| `race.track` | `talons_junction` | `01_vineta_k` | Talon's Junction is `DATA00`'s and absent from the executable too; a bare `--race` failed with "in none of this source's archives". Chosen, not measured, which of the eight |
| `race.zone` | `Separate(zone_1..4)` | `SameCircuit` | the four Zone circuits are `DATA00`'s. Chosen, not measured |
| `exhaust` | `enginetrail_bluered_triangle` (`DATA06`) | `enginetrail_triangle` (`data02`) | **measured**: the PSN executable names the plain `.vex` (table above), confidence 85 |
| `front_end.menu` frame texture | `file2.gtf` (`DATA06`) | `file.gtf` (`data02`) | **measured**: the PSN executable names `file.gtf`, not `file2.gtf`, confidence 85 |
| `campaign.screen_archive` | `Some(DATA06)` | `None` | the screen file is `data02`'s, the disc's own `DATA02` copy; **measured**, confidence 90 (below) |
| `campaign.selection_strings` | `true` | `false` | the ids it overlays are `DATA06`'s alone |
| `front_end.team_select`, `track_select` | `Team_/Track_Selection_Definition.xml` | `None` | both are `DATA06`'s; the race box keeps its plain TRACK and TEAM rows. The older `Selection_Definition.xml` the package carries is a different dialect and is not read |

### What draws differently, honestly absent

- **Menu boxes** draw, from `file.gtf`. The palette is the **teal HD one** the served
  `skin.xml` (`DATA03`'s) authors, where the disc's boot serves `DATA00`'s Fury
  black-and-red; see `docs/formats/hd-frontend.md`, "the HD_ palette".
- **Menu backdrop.** The Fury point clouds (`data/FE/Fury/*.points2`) are absent and the
  executable names none; the background is black.
- **Race box.** The TRACK and TEAM rows list the package's own 16 circuits and 12 teams;
  the two Fury pickers are absent (`Team_/Track_Selection_Definition.xml`).
- **Campaign: opens, straight on the base campaign.** The RACE CAMPAIGN tab opens
  `Grid Selection` over the package's eight grids (`Event 01/08`), then `Cell
  Selection`, and confirming a cell loads the race. There is no HD/Fury chooser
  in front of it: see "The campaign" below for what the data says and the
  RPCS3 frames that measure it. `--menu-page campaign-select` is refused (no
  `Campaign Selection` screen exists to show); `grid-select`, `cell-select` and
  `cell-select-hd` draw.
- **Modes.** `time_trial`, `speed_lap`, `zone` and `single_race` each load on Vineta K
  with opponents, and the load reports are line-for-line the disc's. Eliminator is not
  reachable through `--mode`; its `WeaponStats_Elimination.xml` and the plain
  `elimination_hud.xml` resolve, the `wo3_hud/` and `2097_hud/` Eliminator variants do not
  (a HUD-style option, not the default). Detonator mode is absent from the executable.
- **Five effects** are missing from `data02` on PSN that the disc's carries
  (`WO_PLASMA_FLASH`, `WO_SHIP_ENGINEFLARE`, `WO_TRAIL_HITSHIP_RED`,
  `WO_LEACHBEAM_ENERGY`, `WO_BLUE_WELDER`); the loader reports each as "will not be
  drawn".
- **Fury craft variants** (`_c1`/`_n1`): absent, so a craft shows its base livery.
- **Ads**: `LSAD_*` flyer art is `DATA06`'s.

### The mount order decides more than the roster

`DATA03` before `DATA02` flips the served copy of every path both ship. They share 128
paths: 50 shader and 4 texture and 2 track files identical by MD5, **72 differing** (70
xml tables, 2 ui files). The differing ones include `frontend/gui/skin.xml` (24,576
bytes served, against `DATA02`'s 23,527), `frontend/definition.xml` and
`frontend/gui/endrace_definition.xml` (30,623 against 30,110). Both copies of the
end-of-race file author `EndRace Results`, `EndRace Rewards` and `EndRace Menu`, so
that screen set is complete either way; `frontend::FRONT_END.endrace_entry`'s comment
(DATA02's copy is the one served) is the disc's and is not true here. `just psarc-diff
--other data03 --base data02` reproduces the counts. Which copy a PS3 loads is
unmeasured: chosen, not measured.

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

- The pointer table that names `file.gtf` as the block frame was not walked in the PSN
  executable (the literal is read).
- The race page's TRACK row shows ids in a `--menu-page` still because no language is
  chosen there; not checked through a language-chosen boot.
- Whether the PSN executable loads `DATA02`'s or `DATA03`'s copy of the 72 shared,
  differing files: `DATA03` first (chosen).
- The per-difficulty `data04` grids against `data02`'s flat ones as the grid source (both sources read `data02`'s today).
- The campaign `Cell Selection` art gaps above (emblems and value colours shared with the disc; the `TARGET (NOVICE)` header PSN-specific).
- Whether a PS3 base HD races Zone on every circuit (`SameCircuit` is chosen).
- A picker for the older `Selection_Definition.xml` dialect: `data02`'s copy (29,440 bytes) is the disc's `DATA02` one byte for byte, and `data03`'s (62,322 bytes, the copy served) is a different, larger one. Not the same reader as the campaign's, so it stays open.
