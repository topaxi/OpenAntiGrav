# The front end's models are lit by the front end's own `.envsettings`

2026-11-12. Why HD/Fury's Ship Select hull draws darker than the original (the pair
`hd/menu-ship-select`, hull box `716,150 1012x650`, sigma 7: mean absolute 14.33, mean bias
R -7.6, G -11.7, B -11.1). **The original lights a front-end model with the same engine
parameters a race binds, filled from a block the front end loads from `data/fe/fe.fury.EnvSettings`
(Fury style) or `data/fe/fe.envsettings` (HD style).** This project lights the preview with
`mesh.wesl`'s invented two-light stand-in. Read statically on the GUI bridge (program
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
0.016, plausible for a mostly black page. **This value is unread** (next step below).

**The fog is refused at either level.** At the fitted scale the black exp2 fog moves the box from
4.62 to 6.48 and steepens the left-to-right gradient. Whether the hull's front-end programs read
parameter 7 at all is unread.

Residuals at the fitted scale, not followed: the left of the box reads brighter in the original
(green 1.19 at `x 716..966` against 0.95 at `x 1466..1716`), with red more so (1.76 against 0.97),
and the darkest pixels too (green 1.45 below 0.05 linear). That is a spatial or additive term, not
exposure. Candidates are the reflection the draw binds as parameter 8, which this preview does not
draw, `Ambient false direction` (parameter 11), and the front end's bloom.

## What to wire, and what is open

**Ready to wire (one value open):** the preview's scene block takes the front end's rig, read
through the same `envsettings` path the race uses (`oag_raceplay::load::environment`'s
`envsettings_light`), from `data/fe/fe.fury.EnvSettings` in the Fury style and
`data/fe/fe.envsettings` in the HD style, with HD's `/2` output scale. Then the lit hull takes the
front end's tone scale. `oag_post::hd_bloom` already runs HD's adaptation and resolve, so the
faithful form is the menu frame through that resolve with the front end's tone constants (boost 35,
clamp 3, maximum 4), not a fixed multiplier; 3.45 is the check value. Acceptance: the hull box
near 4.6 mean abs (from 14.33), its bias within about 2 of zero in each channel, and the linear
ratio channel-neutral.

**Unread, with the read recipe:** the front end's adapted luminance. Read it as
`docs/rendering/hd-track-tone.md` did for a race: the PPU's adapted luma at `FunkLayer + 0x25c`
(the object `0xc50ee0`, through `*0x8b73bc`), several reads a few seconds apart on Ship Select
at rest on the compare profile, then exposure is `4 - min(adapted * 35, 3)`. **This page predicts
`adapted` of about 0.016.** A second falsifier is that `0x00c489f0 + 0x540` reads `35.0` there.
Until it is read, the 3.45 is "measured off one frame".

**Not wired, not read:** the fog's effect on a front-end draw, the parameter 8 reflection
(`*(0x00d43c80)`'s texture at `+0x20`), `iblScalePower`'s value, and `Ambient false direction`,
which `oag_mesh::mesh_render::Light` has no field for.

## Omega

**Checked, differs.** Omega's patch (`data08.psarc`) and base (`data00.psarc`) ship the same
four `data/fe/fe*.EnvSettings` paths, and the patch's `eboot.bin` carries the four path strings.
**Omega always runs the HD style**: its boot screen compares `"HD"` with itself to pick
`WOHD_Colours` (`docs/formats/omega-frontend.md`, "The colour-skin switch"), and the HD style's
`bgAnim` is the only backdrop it has ([menu-backdrop-scene.md](menu-backdrop-scene.md)). So if
its loader keeps HD's branch, its Ship Select lights from `fe.EnvSettings`. That file differs from
HD's `fe.envsettings`: ambient `0.2 0.3 0.32` (spelled `colour`) against `0.4` grey, with the same
sun `3.19749 2.094043 1.768024` along `0.658008 0.677334 0.329004`. Its `fe.fury.EnvSettings`
authors HD's Fury `Lighting.` and `Fog.` values, plus a `Tonemap`/`TonemapHDR` block with exposure
minimum and maximum both `1`. The loader and draw were not traced in Omega's executable, and no
Omega memory readout exists: the file choice is inferred from the style, not read.
