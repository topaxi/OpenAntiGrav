---
status: measure
titles: [omega]
updated: 2026-11-28
summary: "Omega from base alone: identification and the front end are the gaps; the race needs the patch's plugin tables and EnvSettings; census open"
size: L (30)
difficulty: medium (35)
---

# Omega from the base package alone, without the patch

2026-11-28, read-only scout. Question: what would it take for Wipeout Omega
Collection to boot and race from `omega-ps4-eu.pkg` alone, with no
`omega-ps4-eu-patch.pkg`? Today `oag-game` stops with "holds none of:
PSP_GAME/USRDIR/Data.wad, WADS2.WAD, PSP_GAME/USRDIR/FE.wad, WADSP.WAD",
because title identification and `oag_omega::DATA_CANDIDATES` key on
`uroot/data09.psarc`, which only the patch carries
(`crates/omega/src/lib.rs`, `DATA_CANDIDATES`; `crates/source/src/title.rs`,
`ps4_folder_content_id`; `crates/source/src/source.rs`,
`ps4_package_directories_in`).

## Open

### The skin.xml answer

**`data00.psarc` ships `Data/plugins/frontend/gui/skin.xml`, 30,235 bytes.**
`docs/formats/omega-status.md` ("What changed") is right. The table in
`crates/omega/src/lib.rs` (about line 663, the `data00.psarc` row: "10
`frontend/gui` XML, no `skin.xml`") is wrong. The `DATA00` const doc in the
same file (about line 680: "42 `frontend/gui` files including `skin.xml`")
is right. Checked by listing: base `data00` holds 42 `frontend/gui` entries,
`skin.xml` among them. The row needs fixing in the same change as any step
below. Three doc comments repeat the "no `skin.xml`" claim and need the same
fix: `crates/omega/src/lib.rs`, the `DATA_CANDIDATES` doc (about line 744,
"a base package whose own front end has no `skin.xml` to boot");
`crates/source/src/source.rs`, the `ps4_package_directories_in` doc (about
line 533, "the base package alone has no `skin.xml` to boot").

The main menu is `cmp`-identical in both copies (9,797 bytes each).

### Front-end schema: content, not schema, mostly

Extracted with `target/debug/examples/psarc_extract` from base `data00` and
patch `data09`, then tag and attribute censuses (regex, because the authored
XML is not strictly well-formed in either copy):

| File | Base `data00` | Patch `data09` | Reading |
| --- | --- | --- | --- |
| `skin.xml` | 30,235 B, 28 tag kinds, 92 attr names | 41,150 B, 30 tag kinds, 106 attr names | Patch adds `Bracket` (2 uses, upsell panels, read by `oag_ui` `brackets`) and `Model` (1 use, read by `screen/mode3d.rs`), plus 14 attribute names (`IsVR`, `Scale`, `Height`, `AnimLength`, `Previous`, ...). Four VR `.vex` includes are new. Base includes resolve in base: every `SrcRel` except `MemoryStickBootScreens.xml` and `MemoryStickScreens.xml`, which resolve nowhere (the file is `memorystickscreens_psp.xml`; same in the patch). |
| `mainmenu_definition.xml` | 9,797 B | 9,797 B | **Byte-identical** (`cmp`), same tags and attrs. |
| `racebox_definition.xml` | 15,784 B | 15,805 B | Same tags; one attr (`IsCallbackChk`). Content only. |
| `cellmode_definition.xml` | 68,013 B | 69,457 B | Same tags; `orthoScale` becomes `orthoScaleX/Y/Z`, `IsHidden`, `IsCallbackChk` added; 5 more `Text`, 5 more `Values`. |
| `additional_definition.xml` | 34,802 B | 35,944 B | Same tags; `GreyedColor`/`InactiveColor` gone, `IsCallbackChk` added. |

So the reader already in tree parses both (`oag_ui::screen::Screens`). What is
missing from base is content: the upsell brackets, the VR includes, and the
newer cell-mode and racebox layouts. Base `data00`'s `skin.xml` is "an older
build", as `crates/omega/src/lib.rs` says.

### Census of what the engine reads that resolves only through the patch

Method (reproducible; no bytes written, names, sizes and classes only):

```sh
target/debug/oag-psarc-diff \
  --other data/extracted/ps4/omega-eu-patch/uroot/{data05,data07,data08,data09}.psarc \
  --base  data/extracted/ps4/omega-eu/uroot/data0{0,1,2,3,4}.psarc \
  --no-hash --tsv <out.tsv>
```

`--no-hash` compares by size: "identical" means same size, not same bytes.
Classes: (a) identical size to a base copy, (b) different size from the base
copy, (c) absent from all five base archives.

**Whole patch, 19,597 entries (all four patch archives):** (a) 1,691, (b)
17,207, (c) 699. Of the 699 absent from base, 567 are VR-named and 132 are
not. "VR-named" is a boundary match, `(^|[/_])vr([/_.]|$)`, case-insensitive;
584 VR-named entries across all classes.

**Literal census, Omega readers** (168 path literals from
`crates/omega/src` and `crates/source/src` only, matched case-insensitively
with slashes normalised; fonts were checked separately in the other crates): 43 resolve to a patch copy: (a) 7, (b) 36, (c) 0. 89
resolve to base only, and 36 are nowhere (our own TOML/XML such as
`Data/XML/Ours.xml`, `own/*.track.toml`, `Data/Psys/OAGRocket*.toml`, and the
`.bik` boot movie, which is in none of the nine archives; open, not a patch
question). The literal census misses dynamically named reads (grids
`grid_{index:02}.xml`, track `.EnvSettings` by directory, ships by team), so
the family table below stands in for those.

| Subsystem | Patch entries (a / b / c) | What is in (c) "absent from base" | Read by a non-VR race or boot? |
| --- | --- | --- | --- |
| Front-end XML and skin (`frontend/gui`) | data08 and data09 each: 24 (a), 18 (b); (c) data08 1, data09 3; data05 and data07 add 3 and 1 (b) | `credits_text_ch`, `ingame_definition_demo`, `endrace_definition_demo`, `Definition_demo` (data09) | Yes for `skin.xml` (b). The `_demo` files are not read by the non-demo boot (not measured). |
| Plugin tables (`plugins/*`) | teams 1 (b), tracks 1 (b), music 1 (b), grids 18 (a), `team_selection` (b) | none | **Yes.** `teams/Definition.xml` 46,819 B base vs 70,000 B patch (b); `tracks/Definition.xml` 15,523 vs 16,265 (b); `music/Definition.xml` 5,931 vs 6,144 (b). |
| Track environments (`EnvSettings`, `.vex`) | 42 `.EnvSettings` (b, 4.8 KB base vs 5.6 KB patch), 9 identical (a), 1 new | `03_Moa_Therma` test materials, `Vineta_K/SkyCube` (5 entries, under 1 MB) | **Yes.** Race bloom chain differs (`crates/omega/src/lib.rs`, `EXTRA_CANDIDATES`). |
| HUD and fonts (`ui`, `xml`, `hud`) | ui 304 (a), 707 (b), 71 (c); xml 194 (a), 194 (b), 140 (c) | `pulsehud_tk`, `small_tk`, `AvantGardeDemiBT` fonts (c), `dfyuanw` CJK, `2097_hud/` font, most `*_VR*` HUD XML | The engine's `Data\FE\Fonts\PulseHud.fnt` resolves to base's `pulsehud.fnt` (a case-insensitive match, present in base). The three patch-only font families are named by no reader in `crates/omega`, `crates/source`, `crates/ui`, `crates/game`, `crates/hud` or the five front-end XML (grep, 2026-11-28); their reader is open. |
| Ships (`art/published`, `hdships`, `ships`) | 67 (a), 34 (b), 409 (c, 1.79 GB) | 403 of the 409 are VR-named (`vr_2048`, `vr_hd`, `vr_fury`, `vr_fury_n1`, `zone_vr`, `detonator_vr`, and VR-named `common_textures`); 6 are not (5 `common_textures` HUD and pilot images, `fe/images/vrshipicon.gnf`) | **No** for the flat race, for the 403. The 6 non-VR are not checked against a reader. The 34 replaced ship files have no VR name, so the non-VR race reads them. |
| Sound (`audio/sound`) | 757 (a), 11 (b), 20 (c, `.wem`, 140 MB) | 20 `.wem` banks entries, none named in `crates/omega` | Banks replaced: `frontend.bnk`, `Music.bnk`, `Ship_NGP.bnk`, `Speech_NGP.bnk`, `Weapons_NGP.bnk` (b). Which cues read the 20 `.wem` is not checked. |
| Movies | none in the nine archives | n/a | Boot movie `Data/FE/Images/StudioLiverpool.bik` is in no archive. Not a base/patch question. |
| Shaders | 121 (a), 15,618 (b), 51 (c) | 15,618 shader files differ in size from base's copies | Risk: whether base's copies pass the shader reader and draw the same. Unmeasured. |

### Language

`data/plugins/languages/simplifiedchinese` (`entries.xml`, `definition.xml`)
is new in patch `data05`/`data08`/`data09` (about 210 KB). Base has no
Chinese table. Only matters for a Chinese-language player.

## Next Steps

Ranked by player-visible win per unit of work. Confidence is the scout's, so
20-40 for most rows, and 60 at most.

1. **Recognise a base-only package as Omega** (size S (40), difficulty easy (45)).
   The base `.pkg` header already carries the Omega content id
   (`EP9000-CUSA05670_00-WIPEOUTOMEGA00EU`; checked with `strings` on the first
   8 KB). Identify on that, or on `uroot/eboot.bin`, in `ps4_folder_content_id`
   and `ps4_package_directories_in` (`crates/source/`) and in `oag_omega`'s
   open path. Drop the hard `data09` requirement from `resolve_on` for the
   base-only case. Done when the base-only run gets past the "holds none of"
   error and the loader report names `data00`. Test: a source-layer test with a
   fake base-only tree, and `oag-game --dry-run` against the base-only source.
2. **Front end from `data00` when `data09` is absent** (size S (50),
   difficulty easy (45)). Fonts do not block it: the menu's HUD font is in
   base. Three patch-only font families have no named reader yet (see the
   table); find their reader before any screen that needs them is judged. Make `DATA_CANDIDATES` fall back to `data00`
   for the front-end XML and skin, and accept that `Bracket`/`Model`
   and the newer cell-mode layout are missing (stated in the loader report).
   Done when the main menu draws with `--menu-page` (headless), compared with
   the patch build frame by frame. Risk: no simulation involved; `oag-omega`
   only.
3. **Plugin tables: teams, tracks, music from base** (size M (35), difficulty
   medium (35)). Base `teams/Definition.xml` is 46,819 B against the patch's
   70,000 B. Count the teams in each copy before choosing (base may be an older
   roster). Measure: `oag_omega`'s roster count and the livery reader. Risk:
   team and track lists feed the simulation's roster (`oag-gameplay` reads
   `oag-tables`, not `oag-omega`); a roster change alters race setup, not
   arithmetic, but say so in the lane brief.
4. **Race from base: EnvSettings and bloom** (size M (30), difficulty
   medium (35)). Patch `EnvSettings` is 5.6 KB against base 4.8 KB. Run one
   race on each copy (`just bench-render omega` or `oag-game --screenshot`) and
   compare the bloom block. This is a read of which copy is correct, not a
   fix. Do not gate on it; it needs a capture (oag-capture) to settle.
5. **Sound cue census for the 20 `.wem` and 5 replaced banks** (size M (30),
   difficulty hard (25)). Read the sound banks out of the `.psarc` archives
   (`oag-wad sounds` reads WAD archives, so a PSARC-capable lister or a new
   example is needed) and name the cues that reference the 20 new `.wem`. Stop when the table exists; the
   fix is a separate lane.

Not worth a lane now: the VR ships and HUD (584 VR-named entries across all
classes; the flat race does not name them); the Chinese language table (a
player-choice, later).

**Total estimate: about 1-2 days for steps 1-3 (boot and menus from base,
confidence 30). A race from base with a checked EnvSettings choice and
sound: 3-5 days (confidence 20).**

Open question for the maintainer, not decided here: whether a base-only
install should be offered as "Omega (base only)" with the loader report
naming what it lacks, or should keep refusing. The refusal is deliberate
(`crates/omega/src/lib.rs`, the `DATA_CANDIDATES` doc).
