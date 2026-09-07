---
categories: [frontend, rendering]
---

# HD's strip widget draws bare text; the real menu also draws a tab shape, an underline, and a background scene

2026-09-01. Grew out of a now-closed thread on HD's horizontal main menu, whose two open questions (the `selected` colour and whether the strip carousels) closed with a live RPCS3 capture (`scripts/rpcs3-drive.py`, boot chain to `Main Menu`, then `Additional` -> `settings`/`Game Options`; screenshots under `data/reference/hd-main-menu-screenshot*`, `hd-menu-highlight-shift`, `hd-menu-style-toggle*`, `hd-settings-screenshot*` on the machine that captured them - gitignored, not committed). Full evidence and confidence scores are on [hd-frontend.md](../../docs/formats/hd-frontend.md#the-fe-style-switch-is-live-works-and-is-not-an-archive-swap---confirmed-by-capture). Three gaps opened this thread; two shipped the same day, and the third - the background scene - is still fully open.

## Shipped

**The tab fill, the corrected text colour and the underline all landed together**, in `crates/game/src/menu/strip.rs`, `crates/game/src/menu/frame.rs`, `crates/game/src/menu/skin.rs`, `crates/title/src/menu.rs` and `crates/hd/src/frontend.rs`:

- `strip::draw` no longer reads `Skin::selected()` for a strip's text at all - every entry, selected or not, draws in `Skin::normal()` (`FEGlobals->TextColor`), matching the capture exactly. `Skin::selected()` itself is untouched; `rows::draw` (both PSP titles) still reads it.
- Every entry gets a background tab: `HD_Grey` (unselected) or `HD_Blue` (selected), read live off the served archive's own globals rather than hard-coded - `oag_title::MenuStrip::selected_fill` names the global (`Some("HD_Blue")` for HD), resolved by `crate::menu::frame::read` the same way `crate::loading`'s own palette resolves its names. The unselected fill needed no new field: `Frame::ink` (`HD_Grey`, already derived from the frame screen's own marks) was already the right value. Both colours are **confirmed exact reads** - the capture's own pixels are `HD_Grey`/`HD_Blue` to the byte on the served archive, not an approximation.
- The selected entry's underline mark draws too, in `Skin::normal()` - measured off the one capture with a visible mark, which showed it in the label's own white rather than either fill colour. It only draws alongside its own tab fill, not on selection alone: with no frame colours to fill the tab (a title with no frame, or a served archive whose globals don't carry the name), the mark would otherwise be a floating dash under nothing.
- **The tab's chamfered corner draws.** Shipped first as a one-step band, on the reading that the corner was a `17`x`7` cut and that a true diagonal needed a primitive `Draw`/`Quad`/`ui.wgsl` did not have. Both halves of that turned out to be wrong the next day - the `17` was the whole cut region rather than the diagonal, and the primitive cost one trailing field. Superseded by the measured pentagon below; kept here because the one-step band is what `git log` shows shipping first and why.
- All of it is **confidence 55**: one capture, the same entry measured twice and agreeing, converted into this build's own 480x272 units against the capture's own resolution relative to HD's `1920x1080` grid (`480 / 1278`). Low relative to the colours (88-90) on purpose - geometry and colour are different claims and this thread does not let one confidence score cover both.
- **A real ruler bug shipped and was caught the same day, by a render, not by review.** The first version of the six tab/underline numbers divided a raw capture pixel by `STRIP_GAP`'s own invented `24`, not by the capture's actual resolution - `24` stood in for the capture's own tab-to-tab gap (`9`px) on the reasoning that nothing disc-authored was in the same image to calibrate against instead, which is true but does not make an invented number a ruler. It produced a tab roughly seven times too tall (`101` where `14.3` renders correctly) and, because every other number in the group went through the same wrong division, a self-consistently-wrong shape throughout - nothing about it looked broken from the arithmetic alone. `--menu-page main --screenshot` is what caught it: the oversized tab's top edge reached up into the screen title sitting above it and, drawing later, overpainted the title's own lower ink - which is also what closes the "chrome title overlap" item below; see there for why that was never a font question. Re-derived at the correct ruler (`480 / 1278 = 0.3756`, the capture's own downscale of HD's `1920`-wide grid against this build's `480`-wide one) and re-rendered to confirm the tab now matches the capture's own proportions - short, wide, tight padding, matching `data/reference/hd-menu-highlight-shift/00-campaign.png` by eye. `UNDERLINE_OFFSET_Y` alone needed one further small correction on top of the ruler (`7.9` to `9.0`, both checked by eye against a render): the ruler assumes the label's own `y` sits at the glyph's cap top, true in the capture but not in this build's own font atlas, which carries a little ascender headroom above the cap inside the cell that `y` anchors to. See `UNDERLINE_OFFSET_Y`'s own doc in `skin.rs`.
- **`STRIP_GAP` is now measured too, and the same session's re-measurement found a second, smaller bug in how the first capture-based note read it.** Four tab-to-tab gaps, measured fill-right-edge to fill-left-edge below the chamfer (full tab width, no per-row narrowing), came back `7`, `8`, `8`, `8` raw px across the five main-menu entries - call it `8`. That `8` is the *visible* gap, not `STRIP_GAP` itself: `strip::draw` steps `x` by `width + gap` from one label's pen position to the next, and a tab's fill starts `TAB_LEFT_PAD` left of its own label, so the fill-to-fill gap actually equals `STRIP_GAP - TAB_LEFT_PAD`. The thread's own first note on `STRIP_GAP` read the visible gap straight as the constant and got `3.4` converted - missing that same step, a formula gap rather than a units one this time. Correct value is the visible gap plus `TAB_LEFT_PAD`'s own raw `6`: `8 + 6 = 14` raw, `14 * 0.3756 = 5.3` converted - `STRIP_GAP` is now `5.3`, down from the invented `24`. Re-rendered and confirmed: entries sit tight, matching the capture's near-touching tabs rather than the previous wide, readability-driven spacing. This also retired the doc's claim that `STRIP_GAP` was "deliberately larger than `OUR_LEADING`" for readability - it no longer is, and a measured value outranks that reasoning now that one exists.

**The tab's corner is drawn as the measured shape, 2026-09-02: a 45-degree cut, then a flat landing.** `Draw::ChamferedFill` pulls a quad's top-right corner left - one trailing `chamfer` field on the existing six-vertex instance, zero for every other caller, no pipeline or bind-group change. The tab is **not** one of these: it is that applied to a *band* the height of the cut and narrowed by the landing, over an ordinary `Draw::Fill`, so the landing is simply the gap between the band's right edge and the tab's and nothing draws it. `TAB_CHAMFER_WIDTH`'s `6.4` split into `TAB_CHAMFER_CUT` `2.25` and `TAB_CHAMFER_LANDING` `4.4`; the height went `2.6` -> `2.25` because the cut is 45 degrees and the old value counted both antialiased edge rows of a 720p capture.

- **Checked at HD's own resolution, not by eye.** `--menu-page main --screenshot --size 1920x1080` renders this build in HD's own grid, where the corner is directly comparable with the framebuffer capture: ours is `9` across, `9` down, landing `18`; the real menu is `9`, `9.5`, `17.5`. Within half a pixel on all three. Rendering at `--size 1920x1080` rather than the default is what makes that a comparison instead of an impression, and is worth doing for any HD-vs-capture question.
- **What the measurement was**, for a reader without the (gitignored) captures: row by row on a `3840x2160` framebuffer grab, the tab's right edge runs a 45-degree diagonal for 19 rows (18 across, `x=55`->`x=72`), then the top edge continues **flat for 35 more pixels** to the right edge at `x=107`. A pentagon, `---\___|`. Cross-checked on HD's own `1280x720` output from a separate boot: `6` columns of diagonal over `6` rows and a `12`-pixel landing, the same within a pixel. The old `17`x`7` reading measured the whole cut region and called all of it the diagonal - which is why the 2026-09-01 `Draw::ChamferedFill` attempt looked wrong beside the capture and got reverted, and why the one-step band that shipped instead was a right angle. Both were right about what they saw.
- **It is rasterised geometry, not `corner2.gtf`.** The per-row steps at `3840` are `+2,0,+2,0,+1,+1,+1,+1,+1,+2,0,+2,0` - irregular, averaging one, which is a line rasteriser at a non-integer position; a magnified 16x16 mask would show uniform runs at the magnification factor. The `<Bracket>` corner asset does not draw this widget, so the "same shape reused or two assets sharing a design language" question is moot here.
- **How to reproduce the capture**, since `data/reference/` is gitignored: set `Resolution Scale: 200` in `~/.config/rpcs3/config.yml`, then `OAG_RPCS3_GEOMETRY=2560x1440x24 python3 scripts/rpcs3-drive.py display` and `uv run --with evdev python3 scripts/rpcs3-drive.py shot --screen "Main Menu"`. Put the scale back to `100` afterwards. **A root-window grab cannot do this** - it returns the emulator's 1280x720 window whatever the display geometry or resolution scale say, the trap [rpcs3-capture.md](../../docs/reverse-engineering/rpcs3-capture.md#a-root-window-grab-is-not-the-framebuffer-and-only-one-of-them-can-measure-geometry) now records.
- `zoom` moved off `menu.rs` onto `Draw` beside `colour_mut`, for that method's own stated reason - it spells the variant list out, so a variant missed there silently does not move with the page it is on. That also clawed 39 lines back off `menu.rs`'s size ceiling, which the chamfer arm had pushed over.

**A capture-driven correction landed alongside, not asked for by either original gap**: `Strip::color`/`MenuStrip::color` (the widget's own literal `0xff705070`) was read, since this thread's parent was opened, as overriding `TextColor` - "a widget declaring its own colour would otherwise be pointless" - and the capture shows that reasoning was wrong: text is `TextColor` regardless of this field. What the literal *is* for is still open (see below); the field stays, only the conclusion drawn from it changed. Tests, doc comments and `docs/formats/hd-frontend.md` all updated in the same change.

**A visual defect surfaced by shipping the fill, not caused by it, and now resolved**: the first render of `--menu-page main --screenshot` against `hdfury-ps3-eu-dec.iso` showed this build's own chrome title ("OPENANTIGRAV") visibly overlapping the top of the now-solid `RACE` tab. The first write-up of this thread guessed a font-metrics cause - this build's chrome-title face rendering taller than HD's real "Title" face at the same nominal scale - which was never checked and turned out to be backwards: HD's `Title` role (`helvb.fnt`, line height `44`) is taller than the `Default` role this build actually draws chrome text in (`helv.fnt`, line height `33`), so the wrong-font theory predicts this build's title should render *shorter* than the real one, not taller. **The real cause was the ruler bug above**: `TAB_TOP_PAD` and `TAB_HEIGHT` were seven times too large, so the tab's own top edge sat roughly 68px higher than it should have - high enough to reach up into the title's own ink and overpaint it, since `layers.chrome` (the title) draws before `layers.body` (the tabs) in `Layers::flatten`'s paint order and whichever draws later wins the overlap. Fixed by the same re-render that fixed the tab shape; no title-specific change was needed at all. The font-role mismatch is real but unrelated - see Open.

## Still fully open

**HD's `Top FE Screen` carries a `<BackgroundAnim>` and this build draws nothing there at all - the prerequisite decoder has landed, and the blocker moved downstream, to material/shader.** `.vex`/`.rcsmodel` decoding was the thing this thread said it was blocked on ([a-chunk-header-is-0x20-bytes-and-then.md](../tooling/a-chunk-header-is-0x20-bytes-and-then.md), 2026-08-25, landed); it has, and `FrontEndScene_HD_ATG.vex` now parses and draws end to end:

```sh
cargo run -q -p oag-view --bin oag-view -- \
  "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC" \
  --mesh 'Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex' \
  --screenshot /tmp/fescene.png
```

56 of 56 mesh nodes, 9,184 triangles, 18,315 authored vertex normals, radius 333.10 - a ring of small repeating segments, unlit today because nothing shades it (see next). **2026-09-01 capture recap**: something real-time and animated (dense red/gold particle motion, no fixed image, different between two boots) draws on the served Fury style, and nothing draws on the HD style - flat white, no motion.

**2026-09-02: checked which material this scene actually authors, and it argues against the "style-specific `FEBackgroundAnim*` shader" hypothesis this thread previously favoured.** The scene's one and only material, per `DATA02.PSARC`'s own manifest, is `/data/materials/frontendscene/basic_vertexemissive.rcsmaterial` - an ordinary per-surface `.rcsmaterial`, not one of the 121 engine-owned `_vp`/`_fp` names `renderer.md`'s shader-registry census reads out of `EBOOT.elf` (`FEBackgroundAnim*` among them). Its one variant: class `RigidBody`, feature set `HalfBrightAmbientSunSpot0SVC0` - the plain sun-plus-ambient lit-race permutation for "no lightmap, no vertex colour", per `oag_formats::rcsmaterial::Features::chunk_word`'s own doctest, despite the file's name promising a vertex-driven emissive term. Full detail and confidence (60 - the key is read off the file, not guessed, but which pass draws it in-game is not traced from the executable) on [hd-frontend.md](../../docs/formats/hd-frontend.md#the-fe-style-switch-is-live-works-and-is-not-an-archive-swap---confirmed-by-capture).

**And checked why `oag-view`'s own report said "0 of 1 resolved" - mechanical, not a missing shader.** `crates/render/src/mesh/rcs/skin.rs`'s `variants()` looks up every chunk's variant as `parsed.variant(rcsmaterial::Class::Static, key)` - `Class::Static` **hardcoded**, on the reasoning (correct for every caller that exists today: track art) that "`Static` is what every world chunk uses". `FrontEndScene_HD_ATG`'s one chunk is class `RigidBody`, which this lookup never tries, on any file - so "unshipped" here means "this resolver only ever asks for `Static`", not "no `.rcsmaterial` on the disc ships this permutation". Nothing in `oag-render` calls `RcsMaterial::variant` with `Class::RigidBody` or `Class::StaticQuake` anywhere - ship materials (`Class::RigidBody`'s own documented case) are not resolved through this path either, today.

Both consequences are new open items below: the geometry most likely is not what `FEBackgroundAnimFuryWave`/`FEBackgroundAnimFuryBlend` shade, which redirects rather than answers the thread's own shader-selection question; and even a resolver that tried `RigidBody` has no `.envsettings`-style sun/ambient to feed the variant it would find, since a front-end screen carries none the way a circuit does. Both are upstream of this thread - the per-material lighting/shading path generally is [hd-needs-a-per-material-shader-path-and.md](../rendering/hd-needs-a-per-material-shader-path-and.md)'s own open ground (no per-material lighting branch exists in `mesh.wgsl` at all yet, which is why 53 of 57 of the ship's own `RigidBody` meshes render dark), not something to build inside this one.

**One thing this capture also exposed, worth remembering before `<aImage>` ever gets a parser**: `DATA00`'s own `mainmenu_definition.xml` opens with the comment `<?this is the real menu ?>` and carries an `<aImage>` block named "Temporary Colour Check" - four swatches naming `HD_Blue`/`HD_Grey`/`HD_LightGrey`/`HD_White`/`HD_BG` by their live value, clearly a developer debug overlay. `<aImage>` is not a tag anything here recognises today, so it is inert by omission rather than by a considered exclusion - but the retail capture also shows no trace of it on either style, confirming the omission happens to be correct. When `<aImage>` (or any other still-unhandled tag) does get read, this specific instance needs the same treatment `frame.rs` already gives `FE_TRIAL_MODE`/`FE_PURCHASE_NOW`: excluded on purpose, not drawn because it parses.


## 2026-09-05: the strip re-measured at a disc-calibrated ruler

**Two of this thread's own numbers were wrong, and the ruler is why they are now
right.** A 3840x2160 RPCS3 framebuffer grab was measured at a scale *solved from
the picture* - the `FE Screen` frame's own two `line.gtf` rules are disc-authored
and sit in the same frame, giving `capture = authored * 1.92 + (76.8, 43.2)`,
an offset that lands on a centred 4% underscan on both axes without being fitted
to one. Cross-checked on `Title_Arrow_HD.gtf`, a third anchor sharing no property
with the rules: predicted `x 385.9..426.2, y 183.4..244.8`, measured `385..426`
and `183..245`. Full derivation on
[hd-frontend.md](../../docs/formats/hd-frontend.md#the-strip-measured-against-a-disc-calibrated-capture).

**Shipped:**

- **The anchor's *meaning* was inverted.** `<HorizMenu>`'s `x="160" y="125"` is
  the **tab rect's top-left corner**; `strip::draw` read it as the label's pen
  and padded outward and *upward* from the text. Real tab corner `(160.5, 126.0)`
  against this build's `(150.8, 117.5)` - one left-pad too far left and one
  top-pad too high, on both axes. The pads' magnitudes were never wrong; only
  which edge they hang off was. Now `(160.0, 125.0)`. Decoupling them also
  removed a latent trap: the tab's top used to be a function of `TAB_TOP_PAD`,
  so tuning where the label sat inside its tab silently moved the tab itself.
- **`TAB_HEIGHT` `14.3` -> `15.6`** (480x272 grid), i.e. `56.8` -> `62.0`
  authored, matching the real `62.0`. Measured on a selected *and* an unselected
  tab, equal to four digits - checked rather than assumed, because the selected
  tab turns out to be 24% *wider* than its neighbours, so width does vary with
  selection and height had to be shown not to. Confidence 75 against this
  thread's group of 55, purely on ruler quality.

**Closed, and this thread was right to leave them open rather than guess:**

- **`STRIP_GAP` needed no change.** Real fill-to-fill gaps are `11.46, 10.94,
  11.46, 11.46`; ours computes `STRIP_GAP - TAB_LEFT_PAD = 12.0`. Within `0.67`.
- **`TAB_LEFT_PAD` and both underline constants needed no change either** - real
  `9.4..9.9` / `20.8` / `7.82` against ours `9.2` / `19.6` / `7.5`. After the
  anchor fix the underline lands at authored `168.0` against the real `167.1`.
- **The highlight does not pulse**, so drawing it flat is correct for HD. See
  the doc page; nothing in the front-end authors an oscillating anything, and
  the strip band is byte-identical between two captures of the same screen at
  different times. **This does not touch the Pulse/PSP thread's own pulse item**,
  which is a different title on a different engine.

**Newly measured and deliberately *not* implemented - the tab width.** The four
unselected tabs are `296.88` wide each to two decimals while their labels are
not the same length, the selected one is `366.67`, and the strip's outer edges
are the frame's own `160` and `1760` with the total conserved. **But the
`<HorizMenu>` schema accepts `gap` and `ItemWidth` - `online_definition.xml` uses
both when it wants fixed-width tabs - and the main menu sets neither, its five
`<Entry>` elements carrying `IDString` and nothing else.** So those are runtime
engine defaults, and hard-coding `296.88` would invent a constant the disc
deliberately declines to state. This build still sizes a tab to its label, and
the difference is recorded rather than papered over.

## Open, added 2026-09-05

- **The label sits `8.7` authored units too low inside its tab** (real cap top
  `8.27` below the tab's top; ours `17.0`). The residual is this build's own font
  atlas rather than the menu geometry - the pen anchors the glyph *cell*, which
  carries `9.46` units of ascender headroom above the cap. Correcting it in
  `TAB_TOP_PAD` alone needs `-1.19` authored units, a negative "pad" that is
  really "the authored inset minus this build's atlas headroom" and rots the
  moment the atlas changes. It belongs with the `Default`-only font-role item
  already below, not ahead of it.
- **The main menu's title authors `random="true"`** - a character-scramble
  reveal - and this build draws it plain. Not previously written down.
- **`transition_secs` is `0.5` where the widget authors `transition="0.4"`**
  (and a `delay="0.2"` nothing reads). Left alone on purpose: `transition_secs`
  drives a *page change* in `menu_stage.rs`, while the widget's `transition`
  reads - on the evidence of its siblings, `<Text transition="2">` on the title
  and `<Image transition="0.3">` on the frame rules - as how long the widget
  takes to animate in when its screen appears. Swapping one confidence-70 number
  for another without settling that is not progress. What would settle it: find
  what consumes `<HorizMenu>`'s `transition` in `EBOOT.elf`, or time a page
  change and a screen entry separately from a capture.
- **No easing curve is authored anywhere on the disc.** 122 `<Key>` elements
  carry `Time`, `X` and `Y` and nothing else, so `Tween::eased` cannot be fixed
  from data on this title either - it is executable-side work. Recorded here
  because it closes off a route someone would otherwise try.

## Open

- **What `0xff705070` is for - narrowed 2026-09-02, not closed.** A wider sweep (all `frontend/gui/*.xml`, `DATA00`-`DATA06`) found it is not `<HorizMenu>`-specific: the same literal is the widget-own colour on eight `<HorizMenu>`/`<VertMenu>` `<Values>` blocks (`mainmenu`, `additional` x3, `manual` x4), and separately recurs fourteen times per copy of `online_definition.xml`, on five `<Menu>` widgets and six standalone `<Text>` status labels (`friendRequests`, `FriendInfo`, `SkinInfo`, `blockedInfo`, `statusInfo`, `pendingInfo`). Never reached through `FEGlobals->`, unlike every colour already confirmed live. On the menu-family widgets it is very likely inert the same way the captured strip case already is (confidence 70, generalised from one capture rather than independently checked per widget); on the standalone `<Text>` widgets it is a materially different draw in this build's own analogous code (`Frontend::draw_screen_at` reads a `Text`'s own colour directly, unlike a menu entry) and clusters entirely on info/status labels, so it reads as a real, deliberately-chosen secondary colour there rather than dead weight (confidence 55 - plausible, not verified; online screens aren't implemented in this project and no capture of them exists). Full detail on [hd-frontend.md](../../docs/formats/hd-frontend.md#the-fe-style-switch-is-live-works-and-is-not-an-archive-swap---confirmed-by-capture). Does not change any of this build's code - HD's online/community screens are out of this project's scope - so this stays a documentation-only close, not an implementation one.
- **This build draws every menu text - the chrome title included - in the `Default` font role (`helv.fnt`, line height `33`) regardless of what a widget's own `font=` attribute names.** HD's main-menu title authors `font="Title"`, which its language plugins resolve to a distinct file (`helvb.fnt`, line height `44`) - see `docs/formats/fnt.md`'s font table. `crate::screen::Text::font` parses the role string already; nothing downstream reads it. `oag_title::MenuSkin` has no `title_font` field to carry HD's choice through, the way `menu_font` carries the row face. This is cosmetic only (the title renders in the wrong face, not the wrong place - see Shipped for why the overlap this exposed was a geometry bug, not this) and it is not new: `crate::frontend::font_line_height`'s own doc comment already names the general shape of the fix - "reading the height back off the atlas... which needs each role's `.fnt` actually loaded rather than only the `Default` one" - as real, left, future work. This is that gap's HD-title instance, not a new one.
- Whether HD's style genuinely draws nothing behind the menu or draws something too subtle/slow to show in one capture (`ScreenSetting name="Main Menu" blur="0"` at least says the screen is meant to be sharp).
- Which of the seven non-Fury `FEBackgroundAnim*` shader names (if any) the HD style resolves to - open, and now more likely a full-screen effect layered separately from `FrontEndScene_HD_ATG.vex` than a shader that draws that mesh, per the 2026-09-02 material finding above.
- **New, 2026-09-02: `FrontEndScene_HD_ATG.vex`'s own material asks to be lit like ordinary in-race geometry (sun colour, sun direction, ambient), and a front-end screen has no `.envsettings` to supply any of the three.** Even a resolver that found its `(RigidBody, HalfBrightAmbientSunSpot0SVC0)` row would have nothing authored to feed it. Whether the real engine invents a default light for this case, borrows one from elsewhere, or the geometry draws unlit despite the variant key is unread - open on the executable side.
- **New, 2026-09-02, checked rather than left open: does `oag-render` already resolve a `(RigidBody, ...)` variant anywhere? No.** `crates/render/src/mesh/rcs/skin.rs`'s `variants()` hardcodes `Class::Static` for every chunk on every file, so this is not "does the exact permutation exist" but "the resolver never asks". That is [hd-needs-a-per-material-shader-path-and.md](../rendering/hd-needs-a-per-material-shader-path-and.md)'s ground (`RigidBody` is the ship's own class, and that thread's own numbers - 53 of 57 ship meshes falling to flat `albedo * ambient` - are this same gap, not a coincidence), not a small addition to make here.

## Next Steps

- **The decoder landed; both real blockers now sit in the per-material lighting/shading chain, which this thread does not own.** [hd-needs-a-per-material-shader-path-and.md](../rendering/hd-needs-a-per-material-shader-path-and.md) is where a `Class::RigidBody` lookup and a no-lightmap/no-vertex-colour lit path would land, for ships and for this scene together - not a fork of that thread's work, its own next increment. Once that lands far enough to draw a `RigidBody` mesh lit by *something*, this thread's own remaining question is real again: which light source a front-end screen (no `.envsettings`) should feed it, and whether the result is what the Fury capture's motion actually is or a second, separate `FEBackgroundAnim*` effect on top.
- The `Default`-only font role, once multi-role `.fnt` loading is worth doing for its own reasons (it is a boot-path change, not a strip one) - not urgent on its own either, since the strip's own text is unaffected and only the title's face is wrong.
