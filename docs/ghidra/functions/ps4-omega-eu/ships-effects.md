# Ship visual-effect object constructors

Functions in `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
`x86:LE:64:default`, imported with
[GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) per
[toolchain.md#ps4](../../../reverse-engineering/toolchain.md#ps4). **The names
here are applied**, from [names.tsv](names.tsv). Found while checking the
lineage hypothesis [`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md)
already confirmed for 2048 against `ps3-hdfury-eu` - does Omega Collection's
PS4 build share the same source tree too, and if so, does that let a name
recovered on one binary transfer to the other despite the architecture
change (x86-64 versus ARM Thumb-2 versus PowerPC64)?

## The lineage question, extended a third time

`search_strings("Backend")` against this binary returns 76 `.cpp` debug-tag
paths under `C:\WOPS4\Wipeout\Code\Backend\...`; the same search against
`vita-2048-eu-v104/eboot.elf` returns 82 relative paths under `Backend/...`
(no drive-letter prefix - a build-relative path rather than an absolute one,
the only difference). 74 of the 76 PS4 paths match a Vita path by suffix
exactly, filename for filename, subdirectory for subdirectory
(`General/Collision/Shape/KdTreeMeshShape.cpp`, `Ships/ShipCTRL.cpp`,
`Weapons/LeachBeamManager.cpp`, `World/WorldManager.cpp`, ...). The two
misses are `Weapons/BaseMine.cpp` and `Weapons/Bomb.cpp`, present on Vita and
not found in this binary's string table - not proof they were removed, since
[`vita-2048-eu-v104/race-hud-selection.md`](../vita-2048-eu-v104/race-hud-selection.md)'s
own caveat applies here too: a `search_strings` miss only means "not
independently corroborated", not "absent". **Omega Collection's PS4 build is
the same `Backend/` source tree 2048 and HD/Fury already share, recompiled a
third time**, not a fresh PS4-specific rewrite.

## `MagstripWake_Construct` - `0x012e38d0`

**Confidence: 85**

A constructor: sets a vtable pointer at offset 0, stores the literal
`"C:\WOPS4\Wipeout\Code\Backend\Ships\MagstripWake.cpp"` at `param_1[0xb]` -
the same tagged-allocation/tagged-object idiom
[`GameRoot_Construct`](../vita-2048-eu-v104/game-boot.md) and
[`RcsModel_Load`](../vita-2048-eu-v104/track-and-collision-loaders.md) rest
their own confidence on - looks up a resource named `"arc_anchor_point"`,
allocates and zero-initialises the object's working buffers, ORs `6` into a
flags field, and increments a live-instance counter before returning. Two
allocations further down build a small procedural texture (`HD_electric_arc_
8x8.gnf`, `HD_ElectricArc_Contact.gnf`) and a CRC-driven table lookup keyed
off the literal string `"AI track data"` - visual/effect setup specific to
this build, not present in the Vita version's much shorter equivalent.

**Evidence is the structural match against the same constructor on
`vita-2048-eu-v104`, decompiled side by side, not the filename tag alone:**

| | PS4 `FUN_012e38d0` | Vita `FUN_811aeee2` |
| --- | --- | --- |
| vtable store | `*param_1 = &PTR_FUN_01915060` | `*param_1 = &PTR_LAB_81221a9e_1_81511b70` |
| `__FILE__`-style tag, same field offset | `param_1[0xb]` = `"...MagstripWake.cpp"` | `param_1[0xb]` = `"Backend/Ships/MagstripWake.cpp"` |
| resource lookup by name | `FUN_012e42c0(param_2, "arc_anchor_point")` | `FUN_811aee9c(param_2, "arc_anchor_point")` |
| flags field | `*(undefined4*)(param_1+0xc) = 0x3006` then later `\| 6` | `param_1[0xc] = param_1[0xc] \| 6` |
| live-instance counter | `_DAT_01a1109c += 1` (rewritten `DAT_020e2f4c` region) | `DAT_818a64dc += 1` |

The field-offset match (`+0xb` on both, despite one binary being 32-bit ARM
and the other 64-bit x86) and the identical `"arc_anchor_point"` /
`\| 6` / instance-counter sequence are not something two independent
constructors converge on by chance - this is the same source function,
recompiled. The PS4 side additionally inlines texture and lookup-table setup
the Vita build calls out to separately (unread here), which is why its
decompile is longer without changing the shared skeleton above.

`MagstripWake` is the visible "electric arc" trail a ship leaves on a
magnetic strip pad - a WipEout-specific term, not a generic engine word,
which is part of why this match is trusted: a coincidental filename hit on a
word like `Init` or `Update` would not carry the same weight.

**Not yet checked**: address-identical cross-check against a second PS4
region/patch build (only one PS4 binary is imported so far, unlike the
Vita/PS3 pairs elsewhere in this project), and instruction-level
disassembly rather than only the decompiler's output.

## Same finding, other binary

The Vita-side half of this match
(`FUN_811aeee2` @ `0x811aeee2`) is recorded in
[`vita-2048-eu-v104/ships-effects.md`](../vita-2048-eu-v104/ships-effects.md)
under its own `names.tsv`, since a name applies to one binary's own database
and needs its own evidence page per [ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md) -
this page and that one cite each other rather than one citing a name the
other's `names.tsv` has not actually recorded.
