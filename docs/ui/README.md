# User interface

> **In progress.** The **HUD layout is recovered and parsed** (M5) - see
> [hud.md](hud.md). The menus exist and navigate (M7) - see
> [menus.md](../architecture/menus.md) for the tree, which is this project's
> own, and [menus-original.md](menus-original.md) for the original's layout,
> colours and page-change animation, which are measured and which this build
> now draws with. The race box's own two screens - Track Select and Ship
> Select - are read off the disc and drawn; see
> [selection-screens.md](selection-screens.md). The Race Campaign's two
> screens - Grid Selection and Cell Selection - draw the same way; see
> [campaign-screens.md](campaign-screens.md). A finished Pulse race's own
> three `EndRace` screens (`Results`/`Rewards`/`Menu`) draw too; see
> [endrace-screens.md](endrace-screens.md).

The languages this build adds on top of the disc's own, and the `human`
flag on every string, are in [project-languages.md](project-languages.md).

## The HUD is authored data

Worth stating here because this page previously implied otherwise. Pulse's
in-race HUD is **not** drawn from code: five layouts ship in `Data.wad` as
`Data\XML\*_HUD.xml`, carrying exact pixel rectangles, atlas sub-rectangles,
colour constants, font roles and alignment. They decode with the same front-end
XML machinery described below. [hud.md](hud.md) has the schema, the widget
inventory and the confidence scores.

## Scope

- HUD: speed, shield energy, position, lap, lap times, weapon
- Zone mode's distinct HUD
- The front end: menus, ship and track selection, options
- Progression and unlock screens
- Text rendering and localisation

## The front end is data-driven

Screens are defined in **XML**, not code. A decompressed PS2 archive entry gave
the first look at one:

```xml
<Screen name="Top">
  <LoadXML>
    <Values Src="Data\Plugins\grids\grid_00.xml"></Values>
  </LoadXML>
</Screen>
```

Widgets are registered by name with a vtable, and their XML attributes are
parsed into fields. The `Movie` widget is fully mapped in
[frontend-video.md](../ghidra/functions/psp-pulse-usa/frontend-video.md), and is the
worked example of how any widget is wired.

Many front-end XML blobs are stored with element names replaced by two-letter
codes and a `<code as="Values" bs="Screen" cs="Mode3D">` dictionary element
mapping them back. That is a size optimisation, not encryption.

Input is bound by **name** (`activate`, `cancel`, `start`), not by button. See
[input](../ghidra/functions/psp-pulse-usa/input.md).

## Assets

The front end has its own archives: `FE.wad` and `FEData.wad` on PSP. The first
blob examined in `FE.wad` carries an `\x01FNT` tag, so font data is in there.
See [formats/wad.md](../formats/wad.md).

Pulse's front end is heavily stylised and animated, and reproducing its feel is
likely to be more work than reproducing its function.
