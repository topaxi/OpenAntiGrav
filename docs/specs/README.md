# Specifications

Short, standalone descriptions of the Studio Liverpool anti-gravity racing
engine's file formats and behaviour, written for someone who has not read the
rest of this repository: a modder, or an emulator or server developer. Each page
states what a thing is, which titles and platforms ship it, the byte layout or
rule, a confidence score (0-100, see the
[rubric](../reverse-engineering/confidence-rubric.md)) per claim, and the known
unknowns. The long evidence trails (decompiler addresses, test corpora, history)
live elsewhere: every page ends with links to its evidence page and to the code
that reads it. Where a spec and the code disagree, the code is the ground
truth.

## Formats

| Page | Covers | Titles |
| --- | --- | --- |
| [WAD and LZSS](wad.md) | Archive container, name hash, compression rule, LZSS bit stream | Pure, Pulse (PSP, PS2) |
| [`.vex` scenes](vex.md) | Node tree, transforms, mesh batches and vertex formats, track spline, sections, collision, pads | Pure, Pulse; partly HD, 2048, Omega |
| [`.pob` particle systems](pob.md) | Container, pointer fixups, emitter record, channels, flags, sprites | Pulse; HD, 2048, Omega |
| [Textures](textures.md) | PSP `.mip`, embedded textures, the GE swizzle, PS2 GS upload packets | Pure, Pulse; links for PS3, Vita, PS4 |

## Behaviour

| Page | Covers | Titles |
| --- | --- | --- |
| [Physics](physics.md) | Per-tick ship dynamics: thrust, drag, steering, airbrakes, hover, wall response | Pulse; differences for Pure, HD, 2048, Omega |
| [Weapons](weapons.md) | Pickups and each weapon's measured behaviour | Pulse; lineage for the other titles |
| [Race rules](race-rules.md) | Race modes, lap counting, start and finish | Pulse; differences where measured |
