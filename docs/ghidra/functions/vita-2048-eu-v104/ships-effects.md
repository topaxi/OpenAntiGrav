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
`/2048/eboot-vita-2048-usa-v104.elf`, the corroboration the other pages in this
directory use - this is the first name in this directory recovered by
cross-title comparison rather than single-binary reading, so that particular
check has not been run yet.

## 2026-10-05: the Vita `MagstripWake` against the PS4 object (magfloor-omega-re lane)

Full object description, evidence and caveats: [`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md),
section "2026-10-05". This page records only what the Vita binary adds or settles.

- **Vtable `0x81511b70`** (Thumb addresses, low bit set): slot 3 `0x811af185`
  (update, function `FUN_811af184`), slot 5 `0x811af271`, slot 7 `0x811af2bf`,
  slot 9 `0x811aefbb`, slot 10 `0x811af00f`. The slot numbers of the own methods
  match the PS4 vtable (3, 5, 7, 9, 10, the rest base class). Slots 5 and 7 lie in
  bytes Ghidra has not made functions of; they were not decompiled.
- **`MagstripWake_Destruct` - `0x811aefba`**, conf. 78. Sets the vtable,
  `DAT_818a64dc -= 1` (the constructor's `+= 1`), frees the three buffers the
  constructor allocated at `+0x4c`, `+0x50`, `+0x54` (`0x150`, `0x150`, `0x588`
  bytes) and zeroes them. Same shape as the PS4 destructor, which is the second
  independent site for the pair.
- **The Vita constructor is called from one place that matters**, `FUN_811c82dc`
  (the ship constructor, `Backend/Ships/Ship.cpp`), at its tail: it builds the wake
  `if (FUN_81000930(&DAT_8153fc40, 0xffffffff) != 0)` and stores it at
  `ship[0x17e0]`, else stores 0. The other caller is `FUN_811b8806`.
- **The particle effect is built by the derived constructor `FUN_812c847a`**, which
  calls `FUN_811c82dc` and then, when `FUN_81000930(&DAT_8153fc40, 0xffffffff) == 0`,
  plays `WO_MAGSTRIP_SPARKS` (when `FUN_810018d4(&DAT_8153fc40) == 0`) or
  `WO_MAGSTRIP_ZONE` (otherwise) at `ship+0x7598`, handle at `ship+0x7590`.
  **The two tests use the opposite sense of the same predicate, so the wake and the
  `.POB` are mutually exclusive here too** - the same split the PS4 build makes with
  `DAT_01f999e4 < 0x17`. What `FUN_81000930` and `FUN_810018d4` test (the object at
  `DAT_8153fc40` is probably the mode record) is **not identified**, so this page does
  not say which side a given 2048 mode lands on (conf. 65 for the mutual
  exclusion, none for the mapping). `FUN_810018d4` also picks
  `WO_SHIP_COLL_SPARK_DAMAGE` vs `..._ZONE`, which is the reason for reading it as a
  Zone-mode test (50).
- **Sound:** `Ship_StartMagstripSound` at `0x811a8064` (`FUN_811a8064`) is the Vita twin of the PS4 `Ship_StartMagstripSound`
  (`0x012f8c70`): same `"~magstrip01"` cue, group created with `FUN_812641fa`,
  `[300.0, 20.0]` written at `+0x40`/`+0x44` (the PS4 build writes `[300.0, 50.0]`).
  Named `Ship_StartMagstripSound`, conf. 55. The Vita build never constructs a
  `MagStrip_Player`/`MagStrip_NPC` group name; it uses a single group.
- **Assets the Vita strings name:** `data/Tex/HD_electric_arc_8x8.gxt`,
  `data/Tex/HD_ElectricArc_Contact.gxt`, `MagStripArc_vp`/`_fp`,
  `WO_MAGSTRIP_ZONE.POB`, `WO_MAGSTRIP_SPARKS.POB`. The Vita archive census was not
  run in this lane (the PS4 and PS3 ones were).
