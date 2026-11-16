# The front end's models are lit by the front end's own `.envsettings`

2026-11-12, wired the same day (`hd-ship-select-light-wire`; "The live read" and "Wired" below). Why HD/Fury's Ship Select hull drew darker than the original (the pair
`hd/menu-ship-select`, hull box `716,150 1012x650`, sigma 7: mean absolute 14.33, mean bias
R -7.6, G -11.7, B -11.1). **The original lights a front-end model with the same engine
parameters a race binds, filled from a block the front end loads from `data/fe/fe.fury.EnvSettings`
(Fury style) or `data/fe/fe.envsettings` (HD style).** This project lit the preview with
`mesh.wesl`'s invented two-light stand-in (until "Wired" below). Read statically on the GUI bridge (program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `0x008bd3c4` for every address below; [memory.md](memory.md)
for the per-function TOC defect), then checked against the pair's frame.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x003aa0e8` | `FrontEnd_LoadEnvSettings` | 85 |

`0x003be688` (`FeModel_DrawAtOrigin`, [team-selection-model.md](team-selection-model.md)) is the
draw this page reads the light binding off; its name and score are that page's.

## The loader: `FrontEnd_LoadEnvSettings(track)` (`0x003aa0e8`)

```text
003aa0fc  bl   0x00679e48            ; a stub to FrontEnd_IsFuryStyle (0x0015b620)
003aa10c  beq  cr7, 0x003aa1d0       ; HD style
  Fury:   r3 = track ? [r2-0x62a8] : [r2-0x62a4]   ; "data/fe/fe.fury.track.EnvSettings" / "data/fe/fe.fury.EnvSettings"
  HD:     r3 = track ? [r2-0x6290] : [r2-0x628c]   ; "data/fe/fe.track.envsettings"      / "data/fe/fe.envsettings"
003aa11c  lwz  r9, -0x62a0(r2)       ; 0x009948ec
003aa124  lwz  r4, 0(r9)
003aa128  addi r4, r4, 0x60          ; the front end's block: *(0x009948ec) + 0x60
003aa130  bl   0x003a83d8            ; Environment_RegisterLightingSchema(path, block)
003aa134  bl   0x003a9520            ; EnvSettings_GetOrCreate(): the global block
          ...                        ; then writes HDR and Bloom constants into the global block
```

The TOC slots read `0x008b711c` `0x007b0ce8`, `0x008b7120` `0x007b0d10`, `0x008b7124`
`0x009948ec`, `0x008b7134` `0x007b0d30`, `0x008b7138` `0x007b0d50` (`read_memory`), and the four
strings are the four paths above. `get_xrefs_to` on `fe.fury.EnvSettings` and `fe.envsettings`
finds only these TOC slots; an `off(r2)` scan for `-0x62a4` and `-0x628c` finds this function and
two others under another module's TOC (`0x00076940`, `0x00076e00`).

The constants the tail writes into the **global** block are its `HDR and Bloom` keys, Fury branch
first: `+0x52c` (`Bloom adaption rate`) `0.015`, `+0x540` (`Tone adaption boost`) `35.0`,
`+0x53c` (`Bloom adaption boost`) `38.75`, `+0x538` `0.75`, `+0x530` `0.5`, `+0x534` `0`,
`+0x550` `0.5`, `+0x554` `0.5`, `+0x55c` `0.01`, `+0x560` `0.03`, `+0x570` `(50, 71, 0, 0)`,
`+0x558` byte `0`. The HD branch writes the same constants but `+0x52c` (`0.01`) and `+0x55c`, so it does not
take `fe.envsettings`' own `Tone adaption boost` (`20`). The Fury constants are
`fe.fury.envsettings`' own values for those keys, and the function leaves `Tone darkening clamp`
(`+0x544`) and `Tone maximum brightness` (`+0x548`) at the block's defaults, `3.0` and `4.0`,
which the file also authors. So the front end draws through the HDR tone stage with its own
constants. Confidence 85 on the branch and the paths (strings, TOC slots and caller all agree), 75
on the tail's key names (offset to key from [renderer.md](renderer.md)'s registrar table).

**Ship Select asks for the non-track file.** `TeamSelectionScreen_Init` (`0x00253458`):

```text
0025379c  li   r3, 0x0
002537a4  bl   0x00678c98            ; stub -> 0x003aa0e8
```

The other seven callers of the stub (`0x00213dc0`, `0x0021984c`, `0x0021c8e0`, `0x002301d4`,
`0x0024cad8`, `0x00258bb0`, and `TrackSelectionScreen_OnEnter_q` at `0x0025d104`) are other
screens; their arguments are unread.

## The draw binds the race's light parameters from that block

`FeModel_DrawAtOrigin` (`0x003be688`) fills a state record at `*(0x008b769c)` (`0x00c514b0` in the
image) from `*(0x009948ec) + 0x60` (TOC slot `0x008b76bc`) and points engine parameters at it.
The parameter index is `(offset - 0x18) / 0x20` into `*(ctx + 0xd8)`, the idiom
[shadow-model-maps.md](shadow-model-maps.md) uses for `viewProj` (`+0x38`, parameter 1):

| Parameter | Table offset | Source in the block | Key |
| --- | --- | --- | --- |
| 7 `fogColour` | `+0xf8` | `+0x4e0`, `w` from `+0x500` | `Fog.Fog Color`, `Fog Density` |
| 8 `paraboloidReflectionTex` | `+0x118` | `*(0x00d43c80) + 0x20`, or `0` when that is null (not the block) | |
| 9 `paraboloidIblTex` | `+0x138` | `0` | |
| 10 | `+0x158` | `+0x420` | `Lighting.Constant ambient color` |
| 11 | `+0x178` | `+0x4a0`, normalised | `Lighting.Ambient false direction` |
| 12 | `+0x198` | `+0x4c0` | `Prelit ambient colour scale` |
| 13 | `+0x1b8` | `+0x4d0` | `Prelit ambient colour power` |
| 14 | `+0x1d8` | `+0x450`, normalised | `Lighting.Sun direction` |
| 15 `directionalLight0Colour` | `+0x1f8` | `+0x430` | `Lighting.Sun color` |
| 72 `iblScalePower` | `+0x918` | the state record's `+0x190` | |
| 76 `globalAlphaScaler` | `+0x998` | the state record's `+0x10` | |

The offsets in the block are those `Scene_PrepareFrame` binds in a race into the same
parameters ([renderer.md](renderer.md), "`Constant ambient color` (`+0x420`) is confirmed wired"),
and the parameter names are [renderer.md](renderer.md)'s table and
[zone-palette-select.md](zone-palette-select.md). The values are re-copied on every call;
the pointers are bound once (the `*(0x008c2c6c)` first-call byte). Confidence 85.

**So the preview hull is the race hull with front-end constants.** HD ships no `ship_FE.vex`
(`docs/ui/selection-screens.md`), the preview draws the team's `ship.vex` and its race
`.rcsmaterial` programs, and those programs read these parameters. A live RPCS3 capture
(2026-09-11, [shadow-model-maps.md](shadow-model-maps.md), "Measured live") already found
`fe.envsettings` values at `[0x9948ec] + 0x60`. The global block (`0x00c489f0`) is the circuit's
once a race loads, so a front end does not inherit the last circuit's sun.

## The rig on the disc

`data/fe/fe.fury.EnvSettings` (`DATA06.PSARC`, 1,859 bytes), the Fury-style file Ship Select
loads; `fe.envsettings` (`DATA02.PSARC`, 1,671 bytes) is the HD style's:

| Key | `fe.fury.envsettings` | `fe.envsettings` |
| --- | --- | --- |
| `Lighting.Constant ambient color` | `0.439216` grey | `0.4` grey |
| `Lighting.Sun color` | `0.992157 0.909804 0.576471` | `3.197490 2.094043 1.768024` |
| `Lighting.Sun direction` | `0.6 0.677334 0.8` | `0.658008 0.677334 0.329004` |
| `Lighting.Sun specular scale` | `3.75` | |
| `Lighting.Ambient false direction` | `0.1 1.0 0.1` | |
| `Lighting.Prelit ambient colour scale` | `0.925490 0.894118 0.898039` | `0.619608 0.384314 0.411765` |
| `Lighting.Prelit ambient colour power` | `2 2 2` | |
| `Fog.Fog Color`, `Fog Density` | black, `0.03` | `0.854902 0.556863 0.627451`, `0.00025` |
| `HDR and Bloom.Tone adaption boost` | `35` | `20` |
| `Tone darkening clamp`, `Tone maximum brightness` | `3`, `4` | not authored |

(Blank: the same as the left column.) Read with
`python3 scripts/psarc.py cat <image>:PS3_GAME/USRDIR/DATA06.PSARC /data/fe/fe.fury.envsettings`.
`data/fe/fury.envsettings`, the 436-line file [envsettings.md](../../../formats/envsettings.md)
calls the front-end file, is the Fury backdrop's and authors no `Lighting.` key.

## The frame check

The pair's own flags (`--menu-page ship-select`, 1920 by 1080, Feisar), the hull box measured as
the compare page does: both frames blurred at sigma 7, the box `716,150 1012x650`, the mean
absolute RGB difference and the mean signed difference per channel. The rig was tried by binding
`Light::authored` with `fe.fury.envsettings`' values in the preview's scene block (an experiment,
reverted):

| Preview light | Box mean abs | Bias R, G, B |
| --- | --- | --- |
| stand-in (today) | 14.33 | -7.6, -11.7, -11.1 |
| `fe.fury` rig, HD's `/2` output | 14.32 | -6.9, -11.2, -12.3 |
| the same plus the file's black fog at `0.03` (exp2) | 15.61 | -8.1, -12.7, -13.5 |
| `fe.fury` rig, no `/2` (a tone scale of 2) | 9.31 | -2.4, -5.3, -7.1 |
| `fe.fury` rig, `/2` times the fitted 3.45 (output scale 1.725) | **4.62** | +2.1, +0.7, -1.7 |
| the same plus the black fog | 6.48 | +0.1, -1.8, -3.8 |

The box number alone does not separate the rig from an exposure, so the ratio was read too:
both frames decoded to linear light, the per-pixel `theirs / ours` over the hull (both
silhouettes above 24, eroded 9 pixels, channels between 5 and 250):

| Preview light | Median ratio R, G, B | Interquartile range, G |
| --- | --- | --- |
| stand-in | 4.10, 3.87, 3.13 | 2.45 to 4.58 |
| `fe.fury` rig, `/2` | **3.53, 3.46, 3.47** | 3.12 to 3.83 |
| `fe.fury` rig, output scale 1.725 | 1.22, 1.05, 1.06 | 0.95 to 1.20 |

**With the rig, what is left is one channel-neutral scale**, flat over the brightness bins
(3.67, 3.38, 3.42 in green from dark to bright; the darkest bin stays high when ours is decoded
with the shader's own `pow(2.2)` instead of sRGB, so it is lighting, not the decode). The stand-in leaves a blue deficit a scale cannot
remove. So the rig is the cause of the colour, and a factor of about 3.45 in linear light, after
the program's `/2`, is the cause of the level. Confidence 85 on the rig (two static sites, one
earlier live read of the block, and the frame's channel-neutral ratio); 55 on the factor.

**The factor is the front end's tone stage, read off one frame, not off the code.** HD's resolve
scales by `Tone maximum brightness - min(adapted * Tone adaption boost, Tone darkening clamp)`
(`oag_tables::envsettings::TONE_ADAPTION_BOOST`), `4 - min(adapted * 35, 3)` with the front end's
constants, a value between 1 and 4. A scale of 3.45 would mean an adapted luminance of about
0.016, plausible for a mostly black page. **Read live, and it is 0.0163** ("The live read" below).

**The fog is refused at either level.** At the fitted scale the black exp2 fog moves the box from
4.62 to 6.48 and steepens the left-to-right gradient. Whether the hull's front-end programs read
parameter 7 at all is unread.

Residuals at the fitted scale, not followed: the left of the box reads brighter in the original
(green 1.19 at `x 716..966` against 0.95 at `x 1466..1716`), with red more so (1.76 against 0.97),
and the darkest pixels too (green 1.45 below 0.05 linear). That is a spatial or additive term, not
exposure. Candidates are the reflection the draw binds as parameter 8, which this preview does not
draw, `Ambient false direction` (parameter 11), and the front end's bloom.

## The live read (2026-11-12, `hd-ship-select-light-wire`)

RPCS3 cold-booted by `rpcs3-drive.py serve` with `OAG_COMPARE_PLATFORM=hd-fury` (1920x1080, `Resolution Scale
100`, `Write Color Buffers: true` in the global `config.yml`; no per-game custom config exists in this tree),
walked `right cross right cross cross` to `Team Selection` (the frame read: Feisar, at rest), then
`scripts/rpcs3-hd-fe-adapted.py` through `/proc/<pid>/mem` (guest base `0x300000000`), ten reads four seconds apart:

| Read | Value |
| --- | --- |
| `FunkLayer + 0x25c` (`0xc50ee0`), the adapted luma | 0.0157 in seven of ten, 0.0168, 0.0180 twice; **mean 0.0163** |
| the same, `+0x240` (the ladder's last mean) | 0.0118 to 0.0157, steps of 1/255: the readback is 8-bit |
| `0x00c489f0 + 0x540`, `+0x544`, `+0x548`, `+0x52c` | `35.0`, `3.0`, `4.0`, `0.015` (the page's second falsifier holds) |
| `*(0x009948ec) + 0x60`: ambient `+0x420` | `0.4392 0.4392 0.4392` |
| sun `+0x430` | `0.9922 0.9098 0.5765`, `w` `3.75` (the specular scale) |
| sun direction `+0x450` | `0.6 0.6773 0.8` |
| fog `+0x4e0`, density `+0x500` | black, `0.03` |

So the front end's exposure is `4 - min(0.0163 * 35, 3)` = **3.43**, against the 3.45 the frame fit predicted
(0.016): confirmed, confidence 75 (one boot; the quantisation step is 0.0039, so the mean is good to about
0.002, which is 0.07 of exposure). The block is the file's, in every key read: the rig, specular and fog
included. **The read moved with the profile.** A first run on RPCS3's default 1280x720 at `Resolution Scale 150`
read 0.0212 to 0.0270 (mean 0.0234, exposure 3.18), 44 % high; `docs/rendering/hd-track-tone.md` found the race
readback at that profile 9 to 35 % *low*, the opposite way, so this is a profile effect of another size and sign, on
a different boot each time (the Fury backdrop's cloud pick varies by boot), and profile and boot are not separated.
A read of this register is made on the compare profile only.

## Wired (2026-11-12)

The craft preview binds the front end's rig and tone, on both paths a player's frame takes (the capture,
`oag_game::capture::preview_pass`, and the live picker, `oag_game::picker_stage`), through one function,
`oag_game::preview::model_light`:

- the file is `oag_title::FrontEnd::model_lighting` (`oag_hd::frontend::MODEL_LIGHTING`): `Data/fe/fe.fury.EnvSettings`
  in the Fury style, `Data/fe/fe.envsettings` in the HD style, picked by the served front end's style
  (`FrontEnd_IsFuryStyle`'s stand-in, `HD_BG`'s darkness);
- read through `oag_raceplay::authored_light`, the reader a circuit's `.envsettings` takes (not hand-copied);
- the output scale is HD's `0.5` (`lit_output_halved`) times `oag_post::hd_bloom::exposure(0.0163, 35, 3, 4)`
  (3.43): **1.715** in the Fury style. The tone constants are the loader's writes; a ground-truth test holds them to
  what `fe.fury.EnvSettings` authors for those keys. The adapted luma is the Fury style's, read. **The HD style's is
  unread, and its page is white** (`HD_BG` light), where the exposure bottoms out at the clamp for any luma above
  `3 / 35` = 0.086: so the HD style is taken at `maximum - clamp` = 1 (output scale 0.5), an inference from the law
  and a white page, confidence 55, not a read. The inherited 3.43 would have been about 3.4 times too bright there;
- **the HD style is not seen on a real frame**: its frame needs the plain HD disc (the Fury disc's menus are the Fury
  style; `display.front_end_style` moves only the loading screen). With the HD file's rig forced onto the Fury frame
  the hull draws plausibly (not clipped, a little dimmer than under the Fury rig; box 9.99 against the Fury
  original, which is not a comparison);
- the file's fog is not applied: with it the box is 6.48 against 4.62, and nothing here shows the hull's programs
  read parameter 7 at all.

**Frame result**, pair `hd/menu-ship-select` re-rendered with its own recorded flags (hull box `716,150 1012x650`,
sigma 7):

| Preview light | Box mean abs | Bias R, G, B |
| --- | --- | --- |
| stand-in (before) | 14.34 | -7.6, -11.7, -11.1 |
| the front end's rig and tone (now) | **4.67** | +2.1, +0.7, -1.7 |

The page's predicted 4.62 with `+2.1, +0.7, -1.7` at a scale of 1.725, reproduced at 1.715. The lit pixels of the
hull box read 136 in the original, 92 before and 133 now (mean luma over pixels above 60).

Tests (`crates/game/tests/hd_ship_preview_ground_truth.rs`):
`the_craft_is_lit_by_the_front_ends_envsettings_and_its_tone_stage` (the light against the parsed file, the scale,
the tone keys) and `ship_selects_hull_is_as_bright_as_the_original_on_both_paths` (capture 121 to 180 and live
108 to 146 lit-pixel luma, each held to a floor between), both red with `model_lighting: None` and green with it;
`the_exposure_function_is_the_scale_the_resolve_applies` (`oag_post`) holds the function to the resolve pass.

**Still open**, as in "The frame check": the left of the box and the darkest pixels read brighter in the original
(the parameter 8 reflection, `Ambient false direction`, which `oag_mesh::mesh_render::Light` has no field for, or
the front end's bloom); the `+ bloom` summand of the resolve is not exposure-scaled and the preview draws after the
chain, so the hull gets none; `iblScalePower`'s value. The HD style's frame is unmeasured (adapted luma taken at the clamp, no HD-style disc frame seen).

## Omega

**Checked, differs; wired on Omega's own file.** Omega's patch (`data08.psarc`) and base (`data00.psarc`) ship the same
four `data/fe/fe*.EnvSettings` paths, and the patch's `eboot.bin` carries the four path strings. **Omega always runs the
HD style**: its boot screen compares `"HD"` with itself to pick `WOHD_Colours` (`docs/formats/omega-frontend.md`,
"The colour-skin switch"), and the HD style's `bgAnim` is the only backdrop it has ([menu-backdrop-scene.md](menu-backdrop-scene.md)).
The loader and draw were not traced in Omega's executable, and no Omega memory readout exists: the file choice is
inferred from the style, not read (confidence 50).

Its `fe.EnvSettings` (base `data00` 3,949 bytes, patch `data08` 4,447 bytes; the two read the same in every `Lighting.`
key below) differs from HD's **in the sun as well as the ambient**: ambient `0.2 0.3 0.32` (`Constant ambient colour`),
sun `Sun diffuse colour` `0.748 0.906 1` times `Sun diffuse intensity` 2 (that is `1.496 1.812 2.0`, a blue-white
sun), direction `0.658008 0.677334 0.329004` (HD's `fe.envsettings` direction), `Nova prelit scale bias power`
`1.4 0.2 1.5`, `Fog colour` black. (An earlier version of this section gave Omega HD's sun colour
`3.19749 2.094043 1.768024`: that was HD's `fe.envsettings`, not Omega's file.) So the same engine parameters take
different values.

`FrontEnd::model_lighting` for Omega (`oag_omega::frontend::OMEGA_MODEL_LIGHT`) names that file for both styles and
reads it in Omega's own key spelling through `authored_light`, with its sun scale and nova prelit combination; **no
tone stage and no halved lit output**: Omega's `lit_output_halved` and its front-end adaptation are unread, so the
scale is 1.0. **Inherited from HD: the wiring, not the values.** Result, `--menu-page ship-select`, Piranha, the
hull's coloured pixels: mean luma 86 before, 129 after, the hull draws and the yellow does not clip beyond what its
texture authors (`omega_lights_its_craft_from_its_own_front_end_file` holds the rig to the file). There is no
original frame to compare against (the shadPS4 race shots are untrusted and no Ship Select pair exists for Omega),
so how close this is to the original is **unknown**.

**2048** (Vita): not checked. Its front end lights nothing here (`model_lighting: None`); its loader is unread.
