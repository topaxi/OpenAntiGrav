# User interface

> **Not yet started.** Milestone M5 for the HUD, M6 for menus.

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
[frontend-video.md](../ghidra/functions/psp-pulse/frontend-video.md), and is the
worked example of how any widget is wired.

Many front-end XML blobs are stored with element names replaced by two-letter
codes and a `<code as="Values" bs="Screen" cs="Mode3D">` dictionary element
mapping them back. That is a size optimisation, not encryption.

Input is bound by **name** (`activate`, `cancel`, `start`), not by button. See
[input](../ghidra/functions/psp-pulse/input.md).

## Assets

The front end has its own archives: `FE.wad` and `FEData.wad` on PSP. The first
blob examined in `FE.wad` carries an `\x01FNT` tag, so font data is in there.
See [formats/wad.md](../formats/wad.md).

Pulse's front end is heavily stylised and animated, and reproducing its feel is
likely to be more work than reproducing its function.
