# HD/Fury: the Zone hull wears the picked team's livery

Function in `EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. Recovered 2026-11-16 (`zone-brand` lane), prompted by the
maintainer's report that our HD Zone ship read "blank" on its side while the original's
carries the branding of the team the player picked. The 2048 and Omega halves of the same
loader are [`vita-2048-eu-v104/zone-craft.md`](../vita-2048-eu-v104/zone-craft.md) and
[`ps4-omega-eu/zone-craft.md`](../ps4-omega-eu/zone-craft.md).

## The law

In a Zone race (game mode 6) every craft flies `Data\Ships\Zone\Ship.vex`, and the loader
queues one texture-path substitution, `zoneship_zone` -> the craft's livery name. The livery
name is the `texturelocation` of the team's `PI_TeamModel name="zone"` in
`Data\Plugins\frontend\definition.xml`, for example `zoneship_quirex` for Qirex. So a texture
request `data/ships/zone/zoneship_zone/team.gtf` reads `data/ships/zone/zoneship_quirex/team.gtf`.

**Confidence: 80.** This is a static read of one function, with every string resolved through
the function's own TOC. The disc's own data corroborates it independently (see "Census"), and
2048 and Omega read the same structure. There was no live RPCS3 run. This sits in the rubric's 70-84 "Probable" band ("Decompilation only, consistent call sites", ceiling 84; `docs/reverse-engineering/confidence-rubric.md`). It is 5 above the 2048 and Omega reads (75) because the disc's own folder layout independently requires the swap.

## `Ship_ReloadModelForSkin` - `0x000dbdc8`

The name and the generic reload behaviour (confidence 78) are on
[`renderer.md`](renderer.md). This section adds the Zone and livery half. The decompile hits bad
instruction data in this function, so the read below is from the disassembly. Every TOC slot was
resolved with `scripts/ps3-toc.py resolve 0x000dbdc8 <disp>`; Ghidra's own TOC is wrong for this
binary ([`memory.md`](memory.md)).

| TOC disp | slot | target |
| --- | --- | --- |
| `-0x4a00` | `0x008a8ad8` | `g_GameState` (`+0xe0` is the game mode) |
| `-0x45f0` | `0x008a8ee8` | `"Livery1"` |
| `-0x45ec` | `0x008a8eec` | `"Data\Ships\Zone\"` |
| `-0x45e8` | `0x008a8ef0` | `"Data\Ships\Zone\Ship.vex"` |
| `-0x45e4` | `0x008a8ef4` | `0x00f868e0`, the substitution table (count at `+0x70`) |
| `-0x45e0` | `0x008a8ef8` | `"zoneship_zone"` (`0x007821e8`) |
| `-0x45d4` | `0x008a8f04` | `"livery1"` |
| `-0x45dc` | `0x008a8efc` | `"Data\Ships\Detonator\"` |
| `-0x4574` | `0x008a8f64` | `"Zoneship_Faisar"` (`0x007823b0`) |

1. **Livery name.** The code reads it into a stack buffer at `r1+0x120`.
   - When the craft entry `ship+0x6298` is set: `lwz r4,0x1b0(r9)` then `bl 0x00676348`, a strcpy (`0x000dbf3c`). `+0x1b4` beside it is the entry's model location, which the race branch formats as `'%s\%s.vex'` with `'ship'` (`0x000dbf80`).
   - With no entry, and with the mode in mask `0x206040` (modes 6, `0xd`, `0xe`, `0x15`; `0x000dc500`-`0x000dc518`): the literal `Zoneship_Faisar` (`0x000dc51c`).
   - Otherwise: `Livery1` (`0x000dc5e0`).
2. **Mode 6** (`cmpwi cr4,r23,0x6`, `0x000dbf48`, branch to `0x000dc638`) copies the two Zone path literals. It does not read the craft. It then increments the count at `table+0x70` (`0x000dc79c`-`0x000dc7bc`). Into record `count`, `0x38` bytes each (`count*0x40 - count*8`, `0x000dc8f4`/`0x000dc930`), it writes:
   - at `+0`: `zoneship_zone`, truncated to 13 bytes (`li r6,0xd`, `0x000dc820`; the 13-byte copy at `0x000dcf50`-`0x000dcfc0`);
   - at `+0x1c`: the livery name (`0x000dc958`, `0x000dc974`).

   The pair is (from, to).
3. **Any other race** with a non-empty livery name (`0x000dbf58`, branch to `0x000dd198`) pushes the pair `livery1` -> livery name into the same table (`0x000dd1a0`, `0x000dd1b0`-`0x000dd1b8`, `0x000dd460`). This is HD's ordinary per-team livery swap, already wired as `oag_livery::LiveryFolder`.
4. **Detonator** (mode `0xe`, `0x000dcbb0`) pushes its own pair under `Data\Ships\Detonator\`.

**What makes the entry the `zone` model.** The livery is whatever `entry+0x1b0` holds. If the
entry were the team's race model (`texturelocation="livery1"`), the Zone hull would read
`data/ships/zone/livery1/...`, and no such folder ships. Every `name="zone"` model names a
`zoneship_<x>` folder that does ship, and nothing else in the definition does. Which code sets
`ship+0x6298` to the `zone` model in mode 6 was not read. That is the main reason this stays at
80 rather than higher.

**The table's reader was not found.** It would be a second, independent site.
`Texture_LoadWithFallback` (`0x005de2d0`) does not read `0x00f868e0`: its TOC loads are
`0x1eec`-`0x1f0c`, and it reads `+0x88`/`+0x78` off another global.
`scripts/ps3-toc.py attrib 0x00f868e0` lists 26 functions whose TOC holds the address. The
next ones to try are those in the model/texture range: `0x003df6d8`, `0x003eeb08`, `0x003f0348`
(`ModelRecord_Create`) and `0x003f3fb0`. Look for a loop over `+0x70` records that compares the
`+0` string against a path.

## Census

Disc: `hdfury-ps3-eu-dec.iso`, `PS3_GAME/USRDIR/DATA02.PSARC`, read with
`scripts/psarc.py list` and `cat`.

- **The hull's own requests.** `data/ships/zone/ship.rcsmodel` names nine textures, all under `data/ships/zone/zoneship_zone/`: `team`, `zone_tp_1024`, `zone_tp_1024_n`, `zone_glass`, `cf_damage_lights`, `colours_flashing_glow`, `damage_lights_n`, `dg_flash3` and `dg_flasha`. Our hull build reports 10 swapped reads; which texture is read twice was not checked.
- **The default folder is incomplete.** `zoneship_zone/` ships only `team.gtf`, `zone_tp_1024.gtf`, `cf_flash.gtf` and `cf_flash3.gtf`. A hull that is never swapped asks for five textures that do not exist. The game cannot have shipped that unswapped, which is the data-side corroboration of the substitution.
- **Twelve team folders.** `zoneship_ag_sys`, `assegai`, `auricom`, `egx`, `faisar`, `goteki`, `harimau`, `icaras`, `mirage`, `piranha`, `quirex` and `triakis` each ship all nine. `zoneship_dead/` is the death shell.
- **The definition.** `Data\Plugins\frontend\definition.xml` ships in five archives. The copies in `DATA00`, `DATA03`, `DATA05` and `DATA06` author twelve `PI_TeamModel name="zone"`, each with `modellocation="Data\Ships\Zone"` and a `texturelocation` naming one of the folders above, with `AG_Systems` -> `zoneship_ag_sys` and `Feisar` -> `zoneship_faisar` (sic). `DATA02`'s copy authors eight. Ours resolves the twelve-team copy (`hd_zone_hull_wears_the_picked_teams_livery` asserts 12).

## As wired

`oag_hd::race::DEFAULTS.zone_craft` is
`ZoneCraft::OwnShipAt { root: Data\Ships, ship: "zone", livery_key: "zoneship_zone" }`. It was
`OwnShip("zone")`, which named the same hull but no livery key, so the swap that 2048 and Omega
already had never ran on HD. The definition read (`oag_raceplay::load::zone_livery`) and the
texture swap (`oag_livery`'s `zone_livery`) are the existing ones.

- **Test**, `crates/raceplay/tests/zone_craft_ground_truth.rs`, `hd_zone_hull_wears_the_picked_teams_livery`. It checks that the twelve liveries are read off the disc's definition and that each folder's `team.gtf` is on the disc. For Qirex, AG_Systems, Feisar and Icaras it checks that the hull build reports `Zone livery <x> replaces zoneship_zone` with nothing missing. It fails on the old `OwnShip` table.
- **Frames** (headless, `oag-game --no-audio --race data/images/hdfury-ps3-eu-dec.iso --mode zone --team <T> --ticks 60 --screenshot <png>`, chase camera at the start):
  - Before the change, Qirex: the nose and both pods are blank white, with only a small default signature on the right pod.
  - After the change: Qirex shows the Qirex logotype on both pods and the nose, and Auricom shows `auricom` in the same places.
  - No RPCS3 frame of the original was taken.

## Ship Select preview (`zone-preview` lane, 2026-11-16)

Ship Select in a Zone race draws the one Zone hull in the picked team's livery, as the race
does. The decision "which folder does this team's Zone hull wear" is one function,
`oag_livery::zone_livery::swap`, over the rows `oag_raceplay::catalogue::Team::craft_liveries`
builds (the team and each variant of it). The race's hull build (`oag_livery::load`, given
`load::zone_livery::resolve`'s rows) and the picker (`oag_game::preview::variant::own_craft_folders`,
over the front end's own catalogue, so no extra disc read and nothing on the render thread) both
call it. The texture reader is shared too (`oag_livery::zone_livery::Reads`), so a texture the
livery folder lacks falls back to the default skin in both. The hull entry is
`ShipPaths::hull_entry_name` in both (`own_craft_entry` in the picker). All of it asks the mode, so Detonator previews its own hull the same way.

- **Test**, `crates/game/tests/zone_preview_ground_truth.rs`: for all twelve HD teams the preview's
  entry equals the race's hull entry, the folder is `zoneship_zone` -> the definition's name, and
  the preview's texels equal those of the race's built hull and differ from the default skin's.
  Fails on a preview that stops applying the livery.
- **Frames** (headless, `oag-game --no-audio --menu-page ship-select --size 1280x720`, a profile
  with `race.mode = "zone"`): Qirex shows the Qirex logotype on the pod, Auricom shows `auricom`;
  before the change Qirex showed the blank hull with a small default mark (Auricom was not shot before). Time Trial shows the
  team's own race hull unchanged. The race frame of the same team
  (`--race --mode zone --team Qirex`) carries the same logotype on both pods and the nose. No
  original capture was taken.

## Lineage (2048 and Omega)

- **2048 v1.04: ported** (the race; the preview has no screen to wire, see below). It uses the same key, `zoneship_zone`, and the same twelve HD-era liveries in `Data\Plugins\teams\Definition.xml`, in both `data.psarc` and the patch's `data2.psarc`. The swap was wired on 2026-10-06. It now has its own test, `vita_2048_zone_hull_wears_the_livery_its_definition_names`: Qirex and AG_Systems swap with nothing missing, and native `feisar2048\1` keeps the default.
- **Omega: ported, and the key differs.** Omega uses the same mechanism with key `zoneship_team`. It was wired and tested on 2026-10-06 (`omega_zone_hull_wears_the_livery_its_definition_names`).
- **Common to all three:** the original's `Zoneship_Faisar` literal applies only when a craft has no entry. What a 2048-era craft's entry holds in Zone is unread on every title, so a native 2048 craft keeps the default skin in ours.

## Open

- **The wreck: built through the swap, not drawn in Zone.** `oag_livery::wreck::load` builds the Zone craft's own `shipwreck.vex` (`Data\Ships\zone\` on HD, `hdships\Zone\` on 2048 and Omega) through the same `zone_livery::Reads` as the hull. Ground truth, Qirex: HD swaps 3 requests and 2048 swaps its requests, both under `zoneship_zone` (`hd_zone_wreck_wears_the_picked_teams_livery`, `vita_2048_zone_wreck_wears_the_picked_teams_livery`). **Omega: checked, differs.** Its wreck asks for `zoneship_zone\zone_tp_wrecked.gnf` and `zone_glass.gnf`, not `zoneship_team`, so the hull's key rewrites none (`omega_zone_wreck_asks_for_a_folder_the_hulls_key_does_not_name`); whether Omega's original reads the wreck's folder through some other pair is unread. A Zone race does not draw the wreck: `shipwreck.vex` is never selected in mode 6, whose end is the death shell and `zoneship_dead.vex` debris ([zone-destroy.md](zone-destroy.md)). `--race --mode zone --team Qirex --force-wreck 60:0` on HD shows the Qirex-painted hull through the blast, then the race ends. **The LODs:** `ship_lod*.vex` is read only by the front end's team-hull widget (`oag_game::team_hull`), no race loads it, so no wiring is open; the widget does not go through the Zone swap.
- **Fury `_c1` variants** inherit their base team's livery through `zone_livery::liveries`. No HD variant was exercised in the test.
- **The table's reader:** see "The table's reader was not found" above.
- **Plain PSN HD** (`hd-psn-eu`) was not exercised. Its definition and folders were not censused; the swap falls back to the default skin, with a report line, if a folder is missing.

## Lineage of the preview wiring

- **Omega: ported**, `omega_zone_preview_wears_the_livery_the_race_does` (Qirex, Auricom, AG_Systems
  and Assegai; the picker reads the PS4 scene through `psp2_scene`). Auricom's `zoneship_auricom\Team.gnf`
  is byte-identical to the default `zoneship_team\Team.gnf` (census over all twelve), so the Auricom preview
  equals the default skin; that is data, not a gap.
- **2048: checked, differs.** Two screens could carry the preview. The race-box Ship Select
  (`FrontEnd::race_box` is `None`; `--menu-page ship-select` refuses) does not exist here. The touch
  `Select Team` screen does draw a hull (`oag_game::team_hull`, `team_model_origin`), but it lists only
  the five native 2048 teams' `Ship_LOD.vex` and the front end carries no mode there
  (`oag_ui::frontend::team`); native teams author no Zone livery, so there is nothing to paint. The same
  decision is pinned against the race for all twelve HD-era teams
  (`vita_2048_zone_preview_wears_the_livery_the_race_does`), ready for a screen. The original's menu
  draws the craft's own hull remapped by the livery key (`vita-2048-eu-v104/zone-craft.md`, 60); ours
  would draw the shared hull as the race does, as asked.
