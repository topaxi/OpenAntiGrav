# Documented functions

One page per analysed function, using
[the template](../function-template.md). Filename is the assigned name, or the
address if confidence is below 50.

| Directory | Binary |
| --- | --- |
| [`psp-pulse-usa/`](psp-pulse-usa/) | `PSP_GAME/SYSDIR/BOOT.BIN` from Pulse PSP (UCUS-98712) |
| [`ps2-pulse-eu/`](ps2-pulse-eu/) | `SCES_547.48` from Pulse PS2 (SCES-54748) |

Both directories group their pages by subsystem rather than one file per
function: a subsystem's evidence is mostly shared between its functions, and
splitting it leaves every page asserting what its neighbour proves.

Three more directories hold **corroboration-only** names, not reverse
engineering in their own right: each has a single `corroboration.md` page and
a `names.tsv` of names carried over from `psp-pulse-usa` by cross-binary
fuzzy matching (`find_similar_functions_fuzzy`), verified per function with
`diff_functions` before anything was renamed.

| Directory | Binary |
| --- | --- |
| [`psp-pulse-eu/`](psp-pulse-eu/) | `PSP_GAME/SYSDIR/BOOT.BIN` from Pulse PSP EU (UCES-00465) |
| [`psp-pure-usa/`](psp-pure-usa/) | `BOOT.BIN` from Pure PSP (UCUS-98612), format ancestor only |
| [`psp-pure-eu/`](psp-pure-eu/) | `BOOT.BIN` from Pure PSP EU (UCES-00001), format ancestor only |

One more is neither a target nor corroboration, but a **lineage probe** on
different hardware:

| Directory | Binary |
| --- | --- |
| [`ps3-hdfury-eu/`](ps3-hdfury-eu/) | `EBOOT.elf` from Wipeout HD Fury (BCES-00664) - the *decrypted* form of the disc's `EBOOT.BIN`, the only entry here that is not the file the disc ships |

## Index

Maintained as pages are added, sorted by subsystem.

The PS3 pages are not in the table below, which is a PSP-versus-PS2 comparison
and has no column that would mean anything for a third platform on different
hardware. They are listed in
[`ps3-hdfury-eu/README.md`](ps3-hdfury-eu/README.md), and cover the framework
memory layer, the collision narrowphase, the `RaceManager` and `ModeManager`
hierarchies, and the race HUD.

| Subsystem | PSP (`BOOT.BIN`) | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| WAD subsystem | [psp-pulse-usa/wad-subsystem.md](psp-pulse-usa/wad-subsystem.md) | [ps2-pulse-eu/wad-subsystem.md](ps2-pulse-eu/wad-subsystem.md) |
| LZSS decoder | covered by the WAD page | [ps2-pulse-eu/lzss.md](ps2-pulse-eu/lzss.md) |
| XML reader | [psp-pulse-usa/xml-reader.md](psp-pulse-usa/xml-reader.md) | [ps2-pulse-eu/xml-reader.md](ps2-pulse-eu/xml-reader.md) |
| Front-end globals (`FEGlobals->`) | covered by the PS2 page | [ps2-pulse-eu/fe-globals.md](ps2-pulse-eu/fe-globals.md) |
| Main loop | [psp-pulse-usa/main-loop.md](psp-pulse-usa/main-loop.md) | - |
| Input | [psp-pulse-usa/input.md](psp-pulse-usa/input.md) | [ps2-pulse-eu/input.md](ps2-pulse-eu/input.md) |
| Ship parts (`Airbrake` flaps) | [psp-pulse-usa/ship-parts.md](psp-pulse-usa/ship-parts.md) | - |
| Input bindings and control schemes | [psp-pulse-usa/input-bindings.md](psp-pulse-usa/input-bindings.md) | - |
| Collision | [psp-pulse-usa/collision.md](psp-pulse-usa/collision.md) | - |
| Camera views | [psp-pulse-usa/camera.md](psp-pulse-usa/camera.md) | [ps2-pulse-eu/camera.md](ps2-pulse-eu/camera.md) |
| Ship exhaust (`Engine Flare`, `Trail`) | [psp-pulse-usa/exhaust.md](psp-pulse-usa/exhaust.md) | - |
| Particle system (`Psys`, `.pob` interpreter) | [psp-pulse-usa/particle-system.md](psp-pulse-usa/particle-system.md) | - |
| Firing a weapon (the request word) | [psp-pulse-usa/weapon-fire.md](psp-pulse-usa/weapon-fire.md) | - |
| The Rocket's model, effects and flight | [psp-pulse-usa/rocket-visuals.md](psp-pulse-usa/rocket-visuals.md) | - |
| Bloom (four-pass framebuffer post-process) | [psp-pulse-usa/bloom.md](psp-pulse-usa/bloom.md) | - |
| Craft update and handling stats | [psp-pulse-usa/engine.md](psp-pulse-usa/engine.md) | [ps2-pulse-eu/handling-xml.md](ps2-pulse-eu/handling-xml.md) (loader), [ps2-pulse-eu/craft-update.md](ps2-pulse-eu/craft-update.md) (hover, damping, integrator) |
| Video and the Movie widget | [psp-pulse-usa/frontend-video.md](psp-pulse-usa/frontend-video.md) | - |
| Library and import symbols | [psp-pulse-usa/imports.md](psp-pulse-usa/imports.md) | [ps2-pulse-eu/libc.md](ps2-pulse-eu/libc.md) |

Start at each directory's README for the applied-rename tables and, on the PS2
side, for the list of PSP claims a second binary now confirms or contradicts.
