# HUD legibility and HUD skins across titles

Investigation, 2026-10-06 (`hud-skins` lane); parts A and C are still the
investigation, and [part D](#d-what-was-built-2026-10-06-hud-crisp-lane) records
the two fixes the `hud-crisp` lane built from it the same day. Three parts:
why Pulse's (and Pure's) HUD text reads poorly, which HUD variants the later
titles ship and whether a player can reach them, and how forcing another
title's skin would work here. Confidence scores follow the
[rubric](../reverse-engineering/confidence-rubric.md); a number with no score is
a measurement of a file, not a claim about the original.

Raw captures and extracts are under `data/scratch/hud-skins/` (gitignored):
`shots/` (PPSSPP and ours, same frame), `x/` (extracted fonts and atlases),
`img/` (atlas montages). Reproducers: `oag-tools` examples `psarc_extract`
(list or extract a PSARC entry by substring) and `hud_font_probe` (describe a
`.fnt`, write its atlas as a PNG).

## A. Why Pulse's HUD text reads poorly

### The font is not a coverage atlas, and its glow is baked in

`PulseHud.fnt` is 512x256 at 4 bpp, 25 px line height; its '0' is a 13x21 box.
The palette alpha is a baked outline *and glow*: for the '0' the texels outside
the 2-3 px body carry alpha 10, 21, 34, 55, 74, 92, 112, 135, 157, 170 - a halo
that reaches **about 7 px** beyond the stroke, with the counter of the '0'
filled at alpha 34-135 (13-53 %) black. At the PSP's 480x272 this halo is the
whole contrast mechanism: the stroke is thin and the glow is what separates it
from a busy track. `small.fnt` (10 px line, 5x9 digits) is the same design at
a size where the body is 1 px wide.
**Confidence 95** (read off the palette and the unswizzled atlas, 2026-10-06;
the swizzle rule is [fnt.md](../formats/fnt.md)).

### Measured against PPSSPP, same frame

PPSSPP 1.20.4, software renderer, Time Trial, VENOM, Talon's Junction, craft on
the start line (0.08 units from the recorded line); ours: `oag-game --race
--ticks 300`, same circuit, class and start pose (6.1, -50.1, -195.9 against
6.08, -50.07, -196.02). PPSSPP frames are the 960x544 viewport halved back to
480x272. **One PPSSPP capture of this state, seen once** (a second capture
later is a different moment, timer 7.17 and the craft moved, so it is not a
repeat). The camera and track differ by a few levels between the two sides, so
a region MAE against the original (30-37 for ours) is background-dominated and
is **not used as evidence**; glyph pixel profiles are, and the halo row below is
qualitative for that reason.

| Factor | Measurement | Result |
| --- | --- | --- |
| Scale 1.0 text (`CurrentTime`, captions) | stroke peak along one row through a '0': original 246, ours 234; fall-off 228/181/103 against 197/130/76; counter darkening 52 against 64 outside (original), 71 against 87 (ours) | **Ours matches the original within about 5 %.** The font is the same ink at the same size. Confidence 80 |
| **Halo clipped to the glyph box** (`borderExtendPixels`) | Pulse's own language definition declares `HUD ... borderExtendPixels="5"` and `HUDSmall ... "3"` (Data.wad entries 1132-1140, every language; HD's copy says the same). **Nothing in `crates/` reads it** (`rg` finds it only in docs), and `render/text.rs` draws each glyph as a quad of exactly the metric box (`cell.width x cell.height`). The atlas dump of the '0' shows 2 px of dark outline (alpha 170-216) *left* of the box edge `u0`, more on the right, and glow below `v1`. Experiment (env-gated, reverted): extend rect and UV by 2 and by 5 texels | Original: a dark halo outside the strokes (3 px left at -5 %, 8 px below at -25 to -45 %). Ours as shipped: **none** (87 87 87 87 then the stroke). With the quad extended, ours grows the same shadow (87 87 77 73 70 then the stroke), and the zoomed glyphs (`shots/cmp-ext.png`) look like the original's soft-edged ones. Backgrounds differ slightly, so the depth is not matched, only the presence. **The most likely cause of "less legible than the original at the same size". Confidence 75** (presence of the halo and of the clip are certain; that this is the *whole* gap is not) |
| Outline alpha (`BorderColor` 0x40, 25 %) | experiment: outline forced opaque (alpha = atlas coverage, the straight modulate a GE would do) | **Worse, not better**: digits fill with black halos the original does not show. The 25 % model is closer than the straight one; the original's exact blend of the glow is still not derived. Confidence 60 that 25 % is the right order of magnitude |
| Text drawn at scale 0.6 (`BestTime`) | at **480x272**: body peak of the `0.00.00` digits original 195 over a 105 background, ours 140 over 110, separating dots lost; nearest sampling is complete but 255. At **960x544**: original 230, ours 255, dots present | A **minification** artefact: it appears only below about 800x450, where a 0.6-scale glyph is smaller than its atlas. At any window a player runs (1440x816 and up) it is magnified and intact (`c-ours-1440.png`). Real at 1x, irrelevant at the default. Confidence 70 |
| Bloom composite over the HUD | three moving frames on this circuit, readout region, bloom on against off: mean change 0-1.5 levels, maximum 29 of 255 | The composite lands over the HUD in the original too (`Bloom_Draw` key 0x70 over HUD keys 0x52-0x6d, [bloom](../rendering/glow-mask.md)), so this is faithful, and small here. It cannot explain a washed-out readout on a dark-backed frame; on a bright boost-pad backdrop it adds to scene light that is already there. The lead's bright frame (`data/scratch/ptbr-a/shots/ptbr-race-hud-400.png`) was **not reproduced** - open. Confidence 65 |
| Scale up and filter | default `--presented` is 1440x816, a 3x linear stretch of a 480x272 design; PPSSPP's own window is a 2x bilinear stretch of the same raster. 4K is about 8x | Every raster HUD edge is a 3 px ramp at 3x. Same family of softness as PPSSPP's own window, but three times larger on screen than the PSP's 4.3 inch panel, where the baked glow was tuned. Confidence 75 as a contributor, no measurement against a sharper alternative yet |

Reading the rows together: **the glyph body is faithful, the glyph's outer halo
is not.** Our quad stops at the metric box, so the baked glow and the outer
outline texels that the font spends most of its contrast on are cut off on the
left, right and bottom, and the authored `borderExtendPixels` (5 for `HUD`, 3
for `HUDSmall`) that says how far to extend is read by nothing. On top of that
the same 480x272 raster is magnified 3-8x with linear filtering. There is no
alpha-channel bug: the palette-alpha and body/outline split is honoured, and
making the outline opaque makes the frame worse. This is also a candidate for
`hud.md`'s long-open "the original's outline reads crisper and darker than 25 %
alpha produces" - the missing darkness is outside the box, not in the alpha.

### Pure, briefly

Pure's two HUD-sized fonts (27 px and 12 px line heights, 124 and 137 glyphs,
against Pulse's 25 and 10) share the header word `unknown` = 4 with Pulse's two
outlined fonts and no other of its 10 `.fnt` entries does; that word is
undecoded ([fnt.md](../formats/fnt.md)), so "same pre-outlined technology" is
an inference from the header and the sizes. No Pure frame was attempted, so the
factor table above is **Pulse only**; the halo clip and the scale factors
should apply (confidence 55). Pure's FE.wad entry names for them are not
recovered (they sit in `Data.wad` and `FE.wad` unnamed).

## B. HUD variants per title

### The lucky win is half a win

The three descendant titles ship **Pulse's HUD sprite atlas at exactly 4x**, and
**do not** ship Pulse's typeface at a higher resolution.

**The atlas.** HD (`pulsehud.gtf`, DXT, 1024x1024), Omega (`Data/hud/textures/
pulsehud.gnf`, BC7, 1024x1024) and 2048 (`PulseHUD.gxt`, PVRTC, 1024x1024)
carry the same artwork as Pulse's `PulseHUD.mip` (256x256): same sheet layout,
same icons, 4x the pixels. Montage: `data/scratch/hud-skins/img/
cmp-atlases-omega-2048-hd.png` (HD's is vertically flipped because a `.gtf`'s
rows run bottom-up, [hd-hud.md](../formats/hd-hud.md); Omega's and 2048's are
upright). The scale is exact, not eyeballed: HD's `hud_timers.xml` names the
clock hexagon as `U=208 V=344 width=112 height=92`, which is Pulse's own
`U=52 V=86 width=28 height=23` times 4 to the texel. **Confidence 90** that the
three 1024 sheets are Pulse's atlas at 4x (visual on all three, rect arithmetic
on one widget; not every sprite rect was checked).

**The layout that uses it.** All three titles carry `arcade_hud_old.xml`
(33 references to `PulseHUD.mip`) and `hud_timers.xml` (2). `arcade_hud_old.xml`
is Pulse's `TimeTrial_HUD.xml` almost verbatim (same `x=6 y=10 width=168
height=26 U=6 V=0` bar). Its rects are in 256-space against a 1024 texture, so
either the engine rescales them or the file is dead. **Nothing references it**:
no XML in any of the three titles names `arcade_hud_old`, and HD's live
`arcade_hud.xml` composes `HUD_*.xml` fragments off `HUD_Components.gtf`
instead. Not checked against the executables. Confidence 70 that it is
unreferenced data, 40 that the engine could draw it.

**The font does not carry over.** HD's `Data/fe/fonts/pulsehud.fnt` is
2048x1024, 92 px line height, 43x80 digits, plain white-on-alpha coverage (one
colour, 16 alphas, no baked outline), 165 glyphs - and its glyphs are **a
different typeface**: heavy, squared-off capitals and a single-alphabet
lowercase, against Pulse's thin rounded technical lowercase-looking face.
Compared glyph by glyph on both atlases (`hud_font_probe`; montage
`x/hd/DATA02/data/fe/fonts/pulsehud.fnt.atlas.png` against
`x/pulse-psp-eu/PulseHud.fnt.atlas.png`). HD's own `definition.xml` declares
`<Values name="HUD" ... Src="Data\FE\Fonts\PulseHud.fnt">` for every language, so
the **file name is inherited and the face is HD's own**. Omega's
`pulsehud.fnt` and `small.fnt` are the same faces at 2x (line 184, 68),
2048x2048 and 1024x1024, and `PulseHud_tk.fnt` (Omega, data05/data08) is a
second 2048 copy at line 175 (Turkish). 2048 ships **no** `PulseHud.fnt` or
`small.fnt` at all: its HUD font is `2048_hud/font/2048_hud.fnt`. The Pulse
typeface exists only in the PSP and PS2 builds, at 25 and 10 px.
**Confidence 85.**

So a "Pulse HUD at high resolution" is available for the **sprites** (4x) but
the **text** would still be HD's face, or Pulse's at 25 px.
`pulse_ready_go.vex/.rcsmodel` (Omega, 2048, HD) carry Pulse's countdown model
name; Pulse's own `Pulse_Ready_Go.vex` is already polygons
([hud.md](hud.md)). Whether the later copies are the same geometry was not
compared. Confidence 40.

### Skins that ship

| Skin | HD / Fury | Omega | 2048 | Covers |
| --- | --- | --- | --- | --- |
| default (root `xml/*_hud.xml`) | yes, the played HUD | yes, as **fragments**, no per-mode roots | yes, the attract demo only | all modes |
| `2097_hud` | 56 files, reachable | 72 files, reachable | 58 files, **not reachable** | arcade, elimination, timetrial, speedlap, zone |
| `wo3_hud` | 56 files, reachable | 68 files, reachable | 58 files, **not reachable** | the same five |
| `2048_hud` | - | 148 files (incl. `_hd`, `_vr`, `_vr_ext`, split-screen variants) | 82 files, **the played HUD** | all, plus detonator, zombie, zone battle, checkpoint |
| `duel_hud` | 10 | 20 | 10 | Duel (no retro variant) |
| `splitscreen_hud`, `splitscreenzone_hud` | 31, 10 | 31, 16 | 31, 16 | two-viewport play, which this build has no mode for |

Counts are archive entries from the lead's listings
(`data/scratch/{omega,2048}-list.txt`) and HD's PSARC listings; Omega counts are
the union of base and patch archives. HD has `detonator_hud`, `duel_hud` and
`mptag_hud` with no retro twin ([hd-hud.md](../formats/hd-hud.md)).

### Unlock and reachability

- **HD / Fury: reachable.** The list and the unlock rows ship only in the
  Fury layer (`DATA03`, `DATA05`, `DATA06`), not in the original `DATA02`. A
  front-end list `HUD Style` (`global="HUD Style"`
  `save="true"` `default="HD"`, entries `HD`, `2097`, `WIP3OUT`) sits in
  `additional_definition.xml`; the executable branches four race managers
  (`SPEliminationRaceManager_Construct`, `SPZoneRaceManager_Construct`,
  `SPTimeTrialRaceManager_Construct`, `MPEliminationRaceManager_Construct`) on
  the strings `WIP3OUT` and `2097`, and a `HUD Style P2` gives split-screen
  player 2 its own. The branch was read in Omega's executable
  ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)); HD's own race
  managers are in [race-hud.md](../ghidra/functions/ps3-hdfury-eu/race-hud.md)
  (path literals per call, not re-read for the `HUD Style` strings here). The
  unlock rows are in
  `plugins/frontend/definition.xml`:

  ```xml
  <PI_HUD name="2097">  <Unlock locked="true" team="any" loyalty="6000"/>
  <PI_HUD name="WIP3OUT"><Unlock locked="true" team="any" loyalty="10000"/>
  ```

  So **2097 unlocks at 6,000 loyalty and Wip3out at 10,000, on any team**,
  the same `<Unlock team="any" loyalty=...>` rule Pulse's skins use
  ([selection-screens.md](selection-screens.md), `oag_game::unlock::
  loyalty_unlocked`). Read off the data, not off a running game.
  **Confidence 85**; that the picker honours `locked` is inferred from Pulse's
  use of the same element (75).
- **Omega: the same two rows** are in `definition.xml` (base data00, patch
  data08, data09), so the same two skins and thresholds. The `HUD Style` list
  itself is in the HD-derived front-end plugin (not located separately).
  Whether Omega's default is HD's HUD or the `2048_hud` set is **unmeasured**;
  the `2048_hud` subtree has `_vr` variants, which suggests the PS4 build reads
  it. Confidence 40.
- **2048: no way in found.** v1.04 has no `PI_HUD` and no `HUD Style` list. Its
  strings carry `STATS_WON_2097_HUD`/`STATS_WON_WO3_HUD` ("2097 HUD WINS"), which
  say a choice existed at some point, but `crates/2048/src/hud.rs` records that
  the executable's race managers hard-code `2048_hud\` and that the two retro
  names occur nowhere in code. Treat the 58-file skins as unreachable data until
  the executable says otherwise.
- **A Pulse skin is not selectable anywhere.** Neither HD, 2048 nor Omega wires
  `arcade_hud_old.xml` into any list or manager that was found.

### What a skin has to carry (and what ours already does)

`oag_title::HudLayouts` names five files per title (arcade, time trial, speed
lap, zone, elimination); HD exposes the three-way split as
`oag_hd::hud::skins::{DEFAULT, WO3, RETRO_2097}` but only the default feeds
`LAYOUTS`. Everything else a skin needs is `oag_title::HudArt`: texture
extension, sights, pickup colours, zone classes, runtime counters, authored
size. That second table is keyed to the skin's own assets, which matters below.

## C. Design note: forcing another title's HUD skin

**Chosen, not measured**: a design, no score. Doc only; nothing here is built.

**Shape.** A skin is the pair (layout set, art rules), the same two things
`Title::hud` and `Title::hud_art` already carry, plus a font slot. Today a
title has exactly one. The change under [ADR-0058](../architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md)
is data: a title's package declares `skins: &[HudSkin]` where each `HudSkin`
has a name (`default`, `2097`, `wo3`, `2048`), a `HudLayouts`, a `HudArt`, the
font role names, the authoring size, and its unlock row. The title's own entry
is index 0, so every existing call site reads index 0 unchanged.

**Config first.** One key, `[hud] skin = "default"`, with `"2097"`, `"wo3"`,
`"2048"` accepted; no settings row until the maintainer wants one. The value is
a *request*, resolved at load against what the mounted sources can supply:

1. a skin counts as reachable only if **every** file its `HudLayouts` names
   exists in the source the title's HUD loads from (the roots and their
   `LoadXML` fragments, which `oag_hud::compose` already walks and reports as
   missing) and its atlas decodes;
2. unreachable or unlisted value: the title's own default, with one line in the
   load report ("HUD skin `2097` unavailable on Pulse PSP: no `2097_hud`
   archive; drew the default") - never silence, never a stand-in;
3. **per-widget fallback** when a skin lacks a widget: the layout composer
   already drops a missing `LoadXML` fragment and logs it; a skin missing one
   *widget* keeps the title's default for that name. Concretely, compose the
   skin's layout, then for each widget the default draws and the skin does not,
   draw the default's, in the skin's coordinate space. This must be opt-in per
   widget class (Zone bar, medal line, message slots) because several widgets
   are timed against the title's rules, not the art; a 2097 skin has no Zone
   panel on 2048 (`CurrentZonePanel` is absent from `wo3_hud` and `2097_hud`,
   [hd-hud.md](../formats/hd-hud.md)) and falling back to the default's panel
   is correct, but falling back to a *different title's* medal rule is not.
4. the unlock stays the skin's own row: a foreign skin chosen by config is
   **not** gated (config is a developer or modder lever, like `--skin`), while
   a future settings row should offer only reachable and unlocked values.

**Which skins are layout-compatible with `oag_hud`.**

| Skin on which title | Compatible now | Needs |
| --- | --- | --- |
| HD `2097` / `wo3` on HD | Yes, same shape as HD's default: five roots, the same fragment grammar, 1920x1080 | skin table entry, the retro fonts (`2097HUD`, `wo3HUD` slots), the retro atlases (`wo2097_hud.gtf`, `wo3_hud.gtf`, [hd-hud.md](../formats/hd-hud.md)); a title with no `HudArt::runtime` for the retro counters |
| `2097`/`wo3` on Omega | Likely: same five roots in the same names | the same, plus 8-byte pointer readers already used for Omega's other assets; unmeasured |
| `2048_hud` on Omega | Partly: Omega's own `2048_hud` has 148 files with `_hd`/`_vr` variants | choose the `_hd` roots; 2048's HUD runtime (`oag_2048::hud::ART`) is not Omega's; unmeasured |
| HD skin on **Pulse/Pure** (PSP, PS2) | **No, not as a drop-in** | Pulse's race state does not feed HD's widgets (loyalty-tier medal line, 14 pickup slots, Fury Zone panel); the authoring grid is 1920x1080 against 480x272; the layouts are fragments, not self-contained files. It is a new `HudArt` plus a new `Readout` mapping, not a swap. The per-widget fallback is the mechanism that makes it viable *later*, one widget class at a time |
| **Pulse** skin on HD/Omega/2048 | **Only the sprite art** | `arcade_hud_old.xml` plus the 4x atlas gives a Pulse-shaped layout in 480x272-space on a 1024 texture, with HD's face for text (the Pulse face is not shipped there). Needs the UV rescale (256 to 1024, flipped for HD's `.gtf` as `hd-hud.md` records), the `HUD`/`HUDSmall` role mapped to HD's face with the font's line height (92) rescaled against the layout's 25, and a decision that this is a *Pulse-flavoured HD* HUD |
| HD/Omega/2048 skin on **2048/Omega** | **Same lineage, easiest** | 2048 and Omega share asset lineage ([omega-status.md](../formats/omega-status.md)): the 2048 `2048_hud` set in Omega, and the retro two on both, are the cheapest first pairing |

The 4x Pulse atlas has a second use that needs no skin machinery at all: **Pulse
itself could draw its sprites from HD's, Omega's or 2048's `pulsehud` sheet
when that source is mounted**, keeping every rect (scaled 4x) and every layout
from Pulse's own disc. That is an asset substitution, not a skin: only the
bars, hexagons and icons sharpen, the text does not, and it needs a second
disc's data present. Whether the project wants a Pulse draw to depend on
another title's disc is a product decision. Recorded, not recommended.

**What to build, in order.** See the handover thread; the first two are small
and independent of the skin design.

## D. What was built (2026-10-06, `hud-crisp` lane)

### The halo: `borderExtendPixels` is read and drawn

`oag_ui::language::Language::font_borders` holds each role's authored
`borderExtendPixels` (`border_extend(role)`, `load::role_border` resolves it
the way `role_font` resolves the file), `Atlas::with_border_extend` carries it
and `Atlas::glyph_quad` grows each glyph's rect and UVs by it on all four
sides; the pen advance and `measure` are untouched, so no layout moved. HUD and
menu text share `Renderer::push_text`, so both pick it up wherever a face
authors it - it is data, not a title branch. The UVs are clamped to the glyph
rows (a bottom-row glyph never samples the solid patch), and where the clamp
bites the rect shrinks with it.

**A glyph's reach is cut to half its distance to the nearest other glyph's
box. Chosen, not measured.** Extended in full, Pulse's PSP menu text came out
garbled (every glyph drew a sliver of its neighbour; `data/scratch/hud-crisp/
shots/menu-after.png` of the first attempt). The census,
`crates/game/tests/font_border_ground_truth.rs` (English plugin of every
title present, `just test-data`):

| Title, role (font) | authored | drawn reach |
| --- | --- | --- |
| Pulse PSP EU and USA: `HUD` (PulseHud.fnt) | 5 | 5 |
| Pulse PSP: `HUDSmall` (small.fnt) | 3 | 3 |
| Pulse PSP: `Menu` (Pulse_20), `Small`/`Title` (Pulse_14) | 5, 3, 3 | **0**, boxes 1 px apart: drawn as before |
| Pulse PS2: every role | 5, 5, 3, 3 | 5, 5, 3, 3 (boxes 7-12 px apart) |
| Pure PSP: `HUD` (HUDFont.fnt), `HUDSmall` | 5, 3 | 5, 3 |
| HD, Omega: `HUD`, `HUDSmall` | 5, 3 | 5, 3 |
| 2048: `NEOSANS`, `NEOSANS_BOLD` | 15 | **3**, boxes 6 px apart |

Screens that changed, **as seen**: the race HUD on Pulse PSP and Pure (viewed),
and on HD (212 pixels over the whole 1080p frame, imperceptible when viewed);
Pulse PS2's race and menu text (pixel diffs, not viewed beyond the menu).
**No change observed**: Pulse PSP menus (reach 0), Pure's menu, 2048's
`--menu-page main` (0 pixels, so no screen has been seen to show NEOSANS
change), and an Omega race frame (0 pixels at tick 300, whose HUD text the
border did not move; Omega's census row says 5 and 3 are authored and drawn).
Whether the original extends a tightly packed face (the PSP menu faces) at all,
and by how much, is not captured.

**Halo depth against PPSSPP's software renderer** (`--graphics=software`, SDL
build under its own Xvfb, USA pressing; ours the EU pressing, same assets),
same ship and circuit (VENOM, Talon's Junction, craft on the line, 1x; the
PPSSPP window halved back to 480x272 with a box filter). Metric: mean
luminance in rings 1-8 px outside the readout's ink (`>= 190`) divided by the
mean 9-11 px out, on the leading `0.` of `CurrentTime` (the one part both
frames show; the other digits differ), three frames from one boot (the timer
differed, the rings did not); script `data/scratch/hud-crisp/ring.py`
(scratch, not committed). A first capture on PPSSPP's GL backend (the ini
asked for software and PPSSPP ignored it) showed a *deeper* halo (rings 2-4
0.60 / 0.61 / 0.64); it is discarded as not the reference.

| Ring (px out) | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- |
| PPSSPP software, frames 1 / 2 / 3 | 0.68 / 0.68 / 0.69 | 0.69 / 0.70 / 0.71 | 0.71 / 0.72 / 0.73 | 0.86 / 0.86 / 0.88 |
| Ours before | 0.84 | 0.87 | 0.87 | 0.93 |
| Ours now | 0.75 | 0.80 | 0.82 | 0.91 |

The mean gap to the original over rings 2-5 went from 0.14 to 0.08: **about
43 % of the missing halo is back**. The rest is a ring 2-4 still 0.07-0.10
lighter than the original. The original's `best` readout peaks at 200-218 where
ours is 254 (a dimmer authored colour or a blend we do not reproduce; the
`current` readout matches). Open. Confidence 70 that the halo is drawn as the
original draws it to within the blend.

### The stretch: `[graphics] hud_scale`

`integer`, `sharp-bilinear` (default), `nearest`, `linear` (the old draw), read
by `oag_display::display::HudScale`, offered only where `HudArt::raster` is
true: **Pulse (PSP and PS2) and Pure**; HD, 2048 and Omega draw linear
whatever the file says (`oag_game::hud_overlay::stretch_for`, pinned by
`hud_scale_gate.rs`). A settings-file value, no menu row. Both Pulse's and
Pure's HUD sheets are 256x256 at a 480x272 (PSP) or 640x448 (PS2) grid; HD,
2048 and Omega ship the sheet at 4x and a 1080p grid (census above).

| Mode | What it does |
| --- | --- |
| `linear` | the stretch as before: a 4 px ramp on every edge at 1080p |
| `nearest` | nearest sampler at the fractional factor: full size, but texels alternate between two widths (visible stair-steps) |
| `integer` | nearest sampler and each glyph and plain sprite resized to a whole number of pixels per texel, top left on a whole pixel, positions still in layout space |
| `sharp-bilinear` | nearest across a texel and a linear blend over its last output pixel (`ui.wgsl`'s `sharp_uv`) |

`integer` takes the floor of the scale factor with a 2 % oversize tolerance
(chosen, not measured): the PSP grid is 272 rows, so 1080p is 3.97 pixels per
texel and 2160p is 7.94, and a strict floor would draw 3 and 7. Sprites that
are rotated, tiled, chamfered or text are not resized as sprites (text is laid
out at the whole size instead).

**Default `sharp-bilinear`, from screenshots** (`data/scratch/hud-crisp/shots/
crop-pulse-*.png`, `crop-pure-*.png`; Pulse PSP EU Time Trial and Pure PSP EU,
four modes at each size):

| Output | Pixels per texel | `integer` draws | Result |
| --- | --- | --- | --- |
| 1920x1080 | 3.97 | 4 (100.7 %, small 0.6-scale readouts 2 of 2.4: 84 %) | integer and sharp-bilinear look alike; linear is the soft one |
| 3840x2160 | 7.94 | 8 (100.7 %, small readouts 4 of 4.8: 84 %) | same |
| 1280x800 (Deck) | 2.67 | 2 (**75 %**, small readouts 1 of 1.6: **63 %**) | integer is visibly smaller and the small readouts shrink to 1 px strokes; sharp-bilinear stays full size and crisp |

**This departs from the maintainer's hint**, which guessed integer (nearest)
as the default: integer is the same picture at 1080p and 2160p and a smaller
one on the Deck, so the default is the mode that is never worse. One line
flips it: `hud_scale = "integer"` under `[graphics]` (or change
`#[default]` on `HudScale`).

The captures are the race load's own frame at `--size`; a `--presented`
capture at 1920x1080 (render scale 100 %, `reconstruction` off, both the
defaults) shows the same crisp edges (`shots/pres-cmp.png`, linear left,
sharp-bilinear right), so the stretch reaches a player's window. **Below 100 %
render scale, or with `fsr1`/`fsr3` on, the HUD is composited into the smaller
target that is upscaled afterwards, and `hud_scale` then acts at the internal
resolution** - not measured here. Whole frames at 1920x1080 and the 4K speed
and shield bars were read unscaled: the bars are flat colour, and their
stepped arrow ends are the raster art's own, as on the PSP.

So `sharp-bilinear` is identical to `integer` wherever the factor is near a
whole number, and the only mode that is both crisp and full size where it is
not (the Deck, any 1440p or odd window). `nearest` is crisp but its uneven
texel widths read as noise on a thin 0.6-scale digit. Integer is one config
value away for anyone who wants blocks of identical pixels.

### 2048 / Omega cross-check

The halo reader: **ported** (HD, Omega and 2048 draw their authored
`borderExtendPixels`; the census above is the ground truth, Omega 5 and 3 on
its HD-derived faces, 2048's 15 cut to 3). The stretch: **checked, differs**
(their HUD sheet is 4x and a 1080p grid; not offered). The 4x sheet as a
substitute source for Pulse's sprites stays open (thread).

## Open

- The bright-backdrop frame behind the washed-out readouts was not reproduced;
  a boost-pad frame on both sides is needed to size the bloom term.
- ~~The size of the halo the original draws~~: measured in part D (about 43 % of
  the gap closed; ring 2-4 still 0.07-0.10 shallower). The exact blend over
  the extended area, and what the original does with a tightly packed face,
  stay open.
- The original's sampling rule for text at fractional scale (0.6) is not read;
  it only matters below about 800x450.
- Pure: drawn and screenshotted (part D) but no PPSSPP Pure frame measured.
- Whether `arcade_hud_old.xml`/`hud_timers.xml` are drawn anywhere in HD's,
  Omega's or 2048's executable is unchecked; they are unreferenced from data.
- 2048's route to its `2097_hud`/`wo3_hud` sets, and Omega's default HUD
  (HD's or `2048_hud`), are open.
- 2048 / Omega cross-check (this page's own finding): **checked, differs** for
  the font (2048 has no `PulseHud.fnt`; Omega's is HD's face at 2x);
  **checked, applies, not wired** for the 4x atlas (both ship it, neither draws
  it); **checked, applies** for the unlock rows (Omega carries the same two;
  2048 carries none). HD front-end side: HD is the source of the unlock rows.
