# User interface

> **Not yet started.** Milestone M5 for the HUD, M6 for menus.

## Scope

- HUD: speed, shield energy, position, lap, lap times, weapon
- Zone mode's distinct HUD
- The front end: menus, ship and track selection, options
- Progression and unlock screens
- Text rendering and localisation

## Assets

The front end has its own archives: `FE.wad` and `FEData.wad` on PSP. The first
blob examined in `FE.wad` carries an `\x01FNT` tag, so font data is in there.
See [formats/wad.md](../formats/wad.md).

Pulse's front end is heavily stylised and animated, and reproducing its feel is
likely to be more work than reproducing its function.
