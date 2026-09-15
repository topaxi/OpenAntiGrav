# Ship visual-effect object constructors

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found comparing this binary against `ps4-omega-eu/eboot.bin` (WipEout: Omega
Collection, PS4) to test whether a name recovered on one binary can transfer
to the other despite the architecture change - see
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) for the
full side-by-side comparison this page's confidence rests on.

## `MagstripWake_Construct` - `0x811aeee2`

**Confidence: 85**

A constructor: calls `FUN_81221830()` first, sets a vtable pointer at offset
0, stores the literal `"Backend/Ships/MagstripWake.cpp"` at `param_1[0xb]` -
the same tagged-object idiom [`GameRoot_Construct`](game-boot.md) and
[`RcsModel_Load`](track-and-collision-loaders.md) rest their own confidence
on - looks up a resource named `"arc_anchor_point"`, allocates two small
buffers (`0x150` bytes each) plus one larger one (`0x588` bytes), ORs `6`
into a flags field at `param_1[0xc]`, and increments a live-instance counter
(`DAT_818a64dc`) before returning.

This is the shorter of the two builds compared: `ps4-omega-eu`'s equivalent
constructor additionally builds a procedural texture and a CRC-keyed lookup
table inline that this version does not - either genuinely new for the PS4
remaster, or (unread here) handled by a callee this function reaches through
one of its own unresolved calls (`FUN_812e5110`, `FUN_8128564c`). Not
determined which.

`MagstripWake` is the visible "electric arc" trail a ship leaves on a
magnetic strip pad - see
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) for the
full field-by-field match table and why a WipEout-specific term like this one
carries more weight than a generic engine word would.

**Not yet checked**: address-identical cross-check against
`/vita-2048-usa-v104/eboot.elf`, the corroboration the other pages in this
directory use - this is the first name in this directory recovered by
cross-title comparison rather than single-binary reading, so that particular
check has not been run yet.
