# Wipeout 2048 status

The [ADR-0009](../architecture/adr/0009-multi-game-fanout.md) probe run a third
title on: what the existing tooling reads off *Wipeout 2048* unchanged, what it
refuses, and what `oag-2048` was filled in from. Measured against
`data/extracted/vita/PCSF00007` - the decrypted EU package, base plus both DLC -
on 2026-08-26.

**The short version.** 2048 is Wipeout HD's asset tree on a little-endian
console. The container, the plugin XML, the handling XML and the `.vex` class
table all read with no code change at all. Three *binary* formats changed
underneath it, and **all three are now read** - the spline, the collision and
the render geometry, including its normal, its diffuse texture coordinate and
its material table and its per-submesh material binding (all added
2026-08-27, after this page's own last full pass). **A race now draws
textured**: the circuit paints 2,782 of its 2,800 draws and the craft all 16
of its own. Getting there needed two independent gaps closed on the same day,
neither a follow-on to the other - the submesh-to-material index
([2048-rcsmodel.md](2048-rcsmodel.md), confidence 90, measured across all
244,889 shipped submeshes rather than read out of the executable) and the
`PVRTII4BPP` pixel format almost every 2048 texture is stored in
([gxt.md](gxt.md), confidence 92, validated against Wipeout HD's own `.gtf`
copies of 2,284 shared textures).

## What reads unchanged

| Layer | Reads? | Notes |
| --- | --- | --- |
| [PSARC](psarc.md) container | **yes** | Version 1.4, flags 1, 18,430 entries in `PSP2/data.psarc`. HD's PS3 archives are version 1.3, flags 3; nothing asserts either. |
| [`.vex`](vex.md) class table | **yes** | Version 6, little-endian, the same class IDs HD's version 6 carries. |
| [Handling stats](handling-stats.md) | **yes** | Both the HD-derived roster's per-team files and the shared `<Global>` block at `Data\XML\handlingstats.xml`. The native roster's five teams parse too, `SUPERPHANTOM` class included. |
| [`WO Track`](track.md) spline | **yes, after a version gate** | See below. |
| [Collision](2048-collision.md) | **yes, in a container of its own** | Not the `.vex` path: a `track_col.col` beside every `track.vex`. See below. |
| [Render geometry](2048-rcsmodel.md) | **positions, triangles, normals, diffuse UV, the material table, the per-submesh material binding, and the node table** | A `.rcsmodel` sharing HD's extension and no other part of its format. Draws textured, and since 2026-09-16 its node-bound meshes draw where their node puts them. `tangent`'s type nibble and the rest of section B's object graph are still unread. |
| [Scenery animation](2048-animation.md) | **yes, from two files of its own** | `track.vex` authors no `Anim Transform`; a `.rcsskeleton` and a `.rcsanimclip` beside the model carry the hierarchy and the keys. Read, wired, and checked against Wipeout HD's evaluator on the twelve shared circuits. |
| Plugin definitions | **yes, but split three ways** | `Data\Plugins\teams\`, `tracks\` and `music\` each ship their own `Definition.xml` where every other title ships one file carrying all three node kinds. `oag_title::Title::plugin_definition` names the teams one; `oag_title::Title::track_plugin_definition` is the axis that reaches the circuits one - `None` on every other title, `Some` here. The soundtrack list still has no equivalent axis and this build sees none. |

## What changed, and what state each is in

**`WO Track` is version `0x107` and its control point shrank from 112 bytes to
96 - recovered, and the parser is version-gated for it.** Everything up to and
including `racing_line` stayed exactly where it was; only `section_id` and
`flags` moved, from `+0x60`/`+0x61` to `+0x5c`/`+0x5d`. The evidence is a
**pairing against Wipeout HD**: 2048's DLC re-ships twelve HD circuits with
identical path and per-path control-point counts, so HD's fully decoded
`0x106` record is ground truth for 2048's, 10,165 control points of it. Full
numbers and confidences are in
[track.md](track.md#wipeout-2048-authors-version-0x107-and-the-control-point-shrank-to-96-bytes).
All fourteen circuits the base package ships now parse with
`AiTrack::encoded_len()` landing on the payload length to the byte.

**`.rcsmodel` is not HD's container - unread.** `Data\art\published\environments\altima\track.rcsmodel`
opens `ed ad 5c ca` where [HD's](rcsmodel.md) opens `0x000a0000`, and is 17.4 MiB
for one circuit. `RcsModel_Load` (`0x812f15b2`) is named and the outer
three-section shape is confirmed by disassembly *and* by the arithmetic closing
on the real file - `189824 + 682966 + 16573144 = 17445934`, exactly the file
length. All three sections' interiors are unread. See
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

**Collision moved out of the `.vex` into `track_col.col`, a k-d tree - and is
now read, whole.** `KdTree_Load` (`0x8118d134`) and `SimpleMesh_Load`
(`0x8118fac8`) between them account for every byte of all 26 shipped files, and
`TrackCollision_MeshFromNode` (`0x8126f800`) names six of the nine surface bytes
by switching on this project's own already-recovered `.vex` collision class IDs.
`oag_vex::kdcol` reads it. Full evidence, including the independent
170,744-triangle match against Wipeout HD's named collision classes and the
winding check, is in [2048-collision.md](2048-collision.md).

## What a race does today

`just play 2048 --race` opens the package, reads the spline, places the craft on
the start line and runs the simulation. What it does **not** do, and why:

- **The craft flies.** It hovers on the surface, is `grounded` every tick, and
  collides with the barriers - `track_col.col` is decoded and its 15,986
  triangles on `altima` become four colliders. With no steering input it drives
  until it meets a wall and stops there, which is what any title does.
- **The circuit and the craft draw, textured.** Altima comes up as 413,358
  triangles over 2,800 submeshes, 2,782 of them painted from 521 of its 527
  materials, and the craft as its own hull with all 16 draws painted from 5 of
  its 6 - livery, tech panels, plastics, light strips and canopy glass, each on
  the submeshes the file's own index names. Both lit off the file's own
  authored normals. 18 of the circuit's draws name a `.gxt` that does not
  resolve or will not decode and get no texture rather than a neighbour's. The
  screenshot is `data/shots/2048_altima_textured.png`. What is still absent
  from it is unrelated: `.envsettings` does not parse for this title, so the
  race lights off a stand-in rig, unfogged and without bloom, and `track.pvs`
  is not this project's HD PVS layout, so every chunk draws.
  See [2048-rcsmodel.md](2048-rcsmodel.md) and [gxt.md](gxt.md).
- **The scenery moves, and the craft's airbrakes are on its tail** (since
  2026-09-16). `track.vex` authors no `Anim Transform`; what animates a 2048
  circuit is the `.rcsskeleton`/`.rcsanimclip` pair beside its model and a
  node table inside the model itself, all three now read
  ([2048-animation.md](2048-animation.md)) and validated node by node
  against Wipeout HD's own evaluator on the twelve circuits both titles
  ship. `altima` runs 113 tracks over 165 nodes - river boats, balloons,
  wind-turbine rotors, a crane, pigeons - and the 130 meshes bound to those
  nodes, which drew a median 1,086 units from their place before the table
  was read, are placed and moving. The load report says so:
  `165 skeleton node(s), 113 animated over 113 track(s), 166.7 s loop`.
  Not wired: the `trackZone` clip (Zone mode's own skeleton, 30 Hz keys)
  and the start-grid animation.
- **Resolved 2026-09-20: `oag_2048::TITLE.front_end` is `Some` -
  [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
  widened `oag_title::FrontEnd::menu` to `Option` and added a second,
  touch-icon axis (`FrontEnd::touch`) rather than waiting on a `MenuSkin`
  this title was never going to author.** The boot chain (`Boot Connect` ->
  `Boot Studio Logo` -> `Boot Intro Movie` -> `Load Save Bootup` ->
  `TitleScreen`) is wired as `Provenance::Declared`, the seventeen language
  plugins are wired, and the touch-icon layout (`GameModeChoice`'s four
  buttons, `Home`'s five, the team screen's ship-model origin, the
  `FE3DCanvas` screen and atlas names) is wired as `FrontEnd::touch`.
  `FrontEnd::menu` itself stays `None`, on the same evidence as before: no
  `<FEGlobals>` block, no `<Menu>`/`<HorizMenu>` widget anywhere. The
  practical effect: `oag-game`'s menu-driven boot (`load_shell`, what
  `--screenshot` with no `--race` goes through) now refuses by a narrower,
  more accurate name - "draws no MenuSkin-shaped menu ... race on it with
  `--race` instead" - where it used to refuse for "no front end at all."
  `loading` and `music` stay `None` on the narrower gaps this page already
  named: a real percentage-bar loading screen with no located plugin XML,
  and a located `frontend.bnk` whose cues are unread. See
  [2048-frontend.md](2048-frontend.md).
- **Zone's announcer is wired, off a bank path that is read rather than
  measured.** The executable's own track-construction function decompiles to
  a real dispatch between two live Zone speech banks, gated on the selected
  circuit's own pack id; a base-package circuit like `DEFAULT_TRACK` reads as
  taking `data/audio/sound/speech_zone_NGP.bnk`, and `zone_announcer` uses it
  with the same fifteen-entry ladder Wipeout HD's own `speech_zone.bnk`
  carries. **Not yet verified against real audio** - this title's
  `data.psarc` is not extracted in this tree, so unlike the other three
  titles nothing here has run `oag-wad sounds` against the actual bank - and
  the milestone table's own reading function was swept for and not found,
  despite every direct reader of the Zone speech handle being checked. See
  [zone-audio.md](../ghidra/functions/vita-2048-eu-v104/zone-audio.md).
- **The HUD draws its four always-on sprites where the original has them,
  and nothing state-gated yet.** All 25 non-split-screen HUD roots compose
  with nothing missing and nothing skipped, the played skin is known
  (`2048_hud\`), the reticle is `Sights::Concentric`, `oag_texture::gxt`
  reads the textures - and, since 2026-09-16, **the title has been run**:
  Vita3K on a headless weston/Xwayland stack
  ([vita3k-capture.md](../reverse-engineering/vita3k-capture.md)) drove the
  EU v1.04 build through three campaign races, a time trial and a Zone run,
  and `ALWAYS_ON` is the intersection of what those frames show. The same
  frames measured the 960x544 coordinate space (`Space::VITA`; before it a
  Vita source fell through to the PSP's grid and drew the HUD at twice its
  size with the right-hand column off screen). `just play 2048 --race --mode
  single_race` now shows the shield silhouette and the speed readout at the
  original's own positions; the shield fill, the pickup slot, the speed
  fills, `PilotAssist` and Zone's lit dashes are read as state-gated and not
  wired, and the text still draws as raw `IG_HUD_*` ids in the 5x7 fallback -
  **but the cause moved, 2026-09-20, and is smaller than it was.** Wiring
  `front_end` (above) makes the seventeen language plugins reachable, and a
  `--race` capture now logs `17 language(s): American ..., ...` where it used
  to log none at all. The font still does not resolve, for a second, distinct
  reason this pass found: `crates/game/src/race/hud.rs`'s `hud_font` looks up
  a font by the literal role name `"HUD"`/`"HUDSmall"`
  (`oag_ui::language::roles`), which is Pulse's and Pure's own role spelling,
  not 2048's - every one of 2048's 17 plugins names its real HUD face
  `2048HUD` instead. Two of the seventeen (`korean`, `traditionalchinese`)
  *do* carry a leftover `HUD`/`HUDSmall` role, inherited from the HD/Pulse
  lineage and pointing at files 2048 does not ship
  (`Data\FE\Fonts\PulseHud.fnt`, `Data\FE\Fonts\koreanHudSmall.fnt`) - and
  because `hud_font` searches every loaded language for the first match
  rather than only the chosen one, `korean`'s dangling entry wins regardless
  of which language a player picked, so the "unavailable, drawing with 5x7"
  report line now names a Korean-only filename on an English race. Closing
  this needs a new axis (a per-title HUD font role name, and 2048 also names
  no distinct "small" face for `HUDSmall` to fall back to, which is a gap of
  its own rather than one this pass could fill without inventing one) - out
  of scope for [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md),
  which only wires the front end far enough to expose this. See
  [2048-frontend.md](2048-frontend.md#the-language-plugins-carry-a-hud-font-role-too)
  and [2048-hud.md](2048-hud.md).
- **2026-09-21: `--race --event "<name>"` launches a real campaign event.**
  2048's own campaign is not a racebox grid at all - it is `Data\xml\SP.xml`,
  a "mjolnir" typed-instance database, 288 instances across eight typedefs,
  five named in-file by their own `type=`/`typedefid=` pairs and four
  classified by measured field shape. `oag_tables::mjolnir` reads it,
  `oag_2048::campaign` names 2048's own facts (its five speed classes
  including `SUPERPHANTOM`, the `laps == 0` Speed Lap sentinel, the mode each
  event kind maps onto), and `race::load_event` resolves an event onto
  `race::Options` before the existing loader runs. `--event "2048 - Event 3"`
  loads Park/Flash/2 laps/`single_race` with a full AI grid; an
  `Elimination`-kind event loads too, since `oag_race::Mode::Eliminator` turns
  out to be a real, implemented mode with this title's own
  `weaponstats_Elimination_2048.xml` already wired - not the unsupported mode
  this feature was built expecting to have to refuse. See
  [2048-campaign.md](2048-campaign.md).

## The one axis 2048 forced into existence

`oag_title::RaceDefaults::ship_dir`. Pulse, Pure and Wipeout HD all keep their
rosters at `Data\Ships\<Team>\`, and `oag_title::race`'s own docs argued that
made it shared vocabulary rather than a title fact. 2048 disagrees: its twelve
HD-derived teams are at `Data\art\published\hdships\<Team>\`, carrying the same
file set under the same names. That is the third-title disagreement
[ADR-0022](../architecture/adr/0022-title-packages.md) asks for.

**Its own five teams needed a *second* axis besides `ship_dir`.**
`ag_systems2048`, `auricom2048`, `feisar2048`, `piranha2048` and `qirex2048`
keep their models under `Data\art\published\Ships\<team>\<1..4>\` and their
tuning under `Data\HandlingStats\<team>\<1..4>\` - two trees, where the
HD-derived roster keeps both together. So `RaceDefaults` carries `ship_dir`
*and* `handling_dir`, and the team id is two levels (`feisar2048\3`).

**The numbered level is the four craft each team flies**, and the disc names
them: `qirex2048\1\Textures_1\Qirex_Fighter_Livery.gxt`, then `_Agility_`,
`_Speed_` and `_Prototype_` in order, corroborated by the front end's own
`locked_Feisar2048_speed.gxt` and its three siblings. Feisar spells the first
`Combat` where Qirex spells it `Fighter`. Confidence 90.

**The HD-derived roster is reachable too, through a third axis.** One
`ship_dir` string cannot address both trees, so `oag_title::GuestRoster`
(`RaceDefaults::guest_roster`) is a *second* roster a title can carry beside
its own, naming a directory of its own and the twelve team ids and `_c1`/`_n1`
suffixes that resolve under it - `RaceDefaults::ships_for`/`handling_dir_for`
route an already-combined id (`Assegai_c1`) to whichever tree it belongs to.
See [ADR-0035](../architecture/adr/0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md).
