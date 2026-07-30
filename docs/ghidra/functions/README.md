# Documented functions

One page per analysed function, using
[the template](../function-template.md). Filename is the assigned name, or the
address if confidence is below 50.

| Directory | Binary |
| --- | --- |
| [`psp-pulse/`](psp-pulse/) | `PSP_GAME/SYSDIR/BOOT.BIN` from Pulse PSP (UCUS-98712) |
| [`ps2-pulse/`](ps2-pulse/) | `SCES_547.48` from Pulse PS2 (SCES-54748) |

Both directories group their pages by subsystem rather than one file per
function: a subsystem's evidence is mostly shared between its functions, and
splitting it leaves every page asserting what its neighbour proves.

## Index

Maintained as pages are added, sorted by subsystem.

| Subsystem | PSP (`BOOT.BIN`) | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| WAD subsystem | [psp-pulse/wad-subsystem.md](psp-pulse/wad-subsystem.md) | [ps2-pulse/wad-subsystem.md](ps2-pulse/wad-subsystem.md) |
| LZSS decoder | covered by the WAD page | [ps2-pulse/lzss.md](ps2-pulse/lzss.md) |
| XML reader | [psp-pulse/xml-reader.md](psp-pulse/xml-reader.md) | [ps2-pulse/xml-reader.md](ps2-pulse/xml-reader.md) |
| Front-end globals (`FEGlobals->`) | covered by the PS2 page | [ps2-pulse/fe-globals.md](ps2-pulse/fe-globals.md) |
| Main loop | [psp-pulse/main-loop.md](psp-pulse/main-loop.md) | - |
| Input | [psp-pulse/input.md](psp-pulse/input.md) | [ps2-pulse/input.md](ps2-pulse/input.md) |
| Collision | [psp-pulse/collision.md](psp-pulse/collision.md) | - |
| Camera views | [psp-pulse/camera.md](psp-pulse/camera.md) | [ps2-pulse/camera.md](ps2-pulse/camera.md) |
| Ship exhaust (`Engine Flare`, `Trail`) | [psp-pulse/exhaust.md](psp-pulse/exhaust.md) | - |
| Craft update and handling stats | [psp-pulse/engine.md](psp-pulse/engine.md) | [ps2-pulse/handling-xml.md](ps2-pulse/handling-xml.md) (loader), [ps2-pulse/craft-update.md](ps2-pulse/craft-update.md) (hover, damping, integrator) |
| Video and the Movie widget | [psp-pulse/frontend-video.md](psp-pulse/frontend-video.md) | - |
| Library and import symbols | [psp-pulse/imports.md](psp-pulse/imports.md) | [ps2-pulse/libc.md](ps2-pulse/libc.md) |

Start at each directory's README for the applied-rename tables and, on the PS2
side, for the list of PSP claims a second binary now confirms or contradicts.
