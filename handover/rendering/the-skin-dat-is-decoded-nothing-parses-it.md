---
categories: [rendering, tooling]
---

# The skin `.dat` is decoded; nothing parses it and nothing selects a livery

2026-09-05. Replaces `team-model-and-skin-variants-are-declared-and.md`, whose
one blocking question - the `.dat` payload format - is answered. That thread's
own predecessor was
[per-team-hulls-are-drawn-which-team-flies.md](per-team-hulls-are-drawn-which-team-flies.md).
This is the implementation half, and it is unblocked now rather than blocked.

**What's known, all of it evidenced.** The schema side:
[`docs/formats/dlc-pack.md`](../../docs/formats/dlc-pack.md#entry-0-is-a-manifest)
has the XML - every team declares `Normal`, `Concept` and `Zone` variants, and
`Normal` alone nests two `PI_ModelSkin`s, `Alternative` and `Eliminator`, each
with an `Unlock` pair (the team's own `loyalty` threshold and a `Team="any"`
one, always the higher). The `location="ship"` on `PI_TeamModel` is not the
raceable hull's path; that is still the team directory plus `Ship.vex`.

The format side is
[`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`](../../docs/ghidra/functions/psp-pulse-usa/ship-skin.md),
new with this thread. In one line: **a skin is a texture swap on the same
geometry, not a second model.** A `.dat` is a `0x20` header (the team's display
name, NUL-terminated - the `"ms"` tag both earlier readings recorded is not a
field, it is residue in a reused export buffer, and validating it would reject
AG Systems) then four palette-plus-pixels blocks - `0x20`/`0x2060`/`0x40a0`/`0x60e0`,
three of them 128x128 4bpp and the last 64x64 4bpp, sixteen `RGBA8` entries
each - which the loader uploads over the hull model's `texture1.tga` through
`texture4.tga`, matched by name, case-insensitively. Every shipped file is
exactly 26912 bytes. Confidence 90: the offsets and the 128x128 are plain
immediates in `Skin_ApplyToModel` / `Skin_ComposeQuarterAtlas`, and the
arithmetic closes with nothing left over against all sixteen real files.

**Three things that will bite a parser**, all on the evidence page in full:

1. **There is no width, height or palette-size field anywhere in the file.**
   `Texture_UploadPaletted` takes all of it from the *target texture's*
   descriptor. The `0x40`/`0x2000` block sizes are what the hull's textures
   happen to be, not something the file declares. A parser must be told the
   dimensions, or must derive them from the 26912-byte total.
2. **Mips are generated, never stored.** The original halves the image
   repeatedly in index space with a palette-aware 2x2 filter
   (`Texture_Downsample4bpp`).
3. **The fourth slot is composed rather than copied** when the applier's flag
   argument is zero: three quarter-scale tiles packed into one 64x64 image,
   using block 1's palette for all three, with the **top-right quadrant left
   as uninitialised stack**. Reproduce that, do not fill it in.

## Open

- **`oag_formats::ship_skin` parses the header and the four blocks, and
  `oag_render::mesh::ship_skin::apply` overlays a parsed skin onto an
  already-built `Model`'s texture slots - matched by name, case-insensitive,
  the same rule `Skin_ApplyToModel` uses.** 2026-09-07, both tested (unit and
  ground-truth against `Data\Ships\Assegai\ship_alt.dat`). **Nothing calls
  `apply` from a race yet** - see Next Steps: `catalogue::Team` still does not
  collect `PI_ModelSkin`, and no caller reads an Alternative/Eliminator pick.
  **Deliberately not in scope**: `Skin_ComposeQuarterAtlas`'s fourth-slot
  composite (the quadrant-mapping question below is still open).
- **The other axis this thread's schema also describes - `PI_TeamModel`'s
  Normal/Concept **hull** choice, not the skin - is recovered and wired end
  to end, separately from the skin work above.** 2026-09-07: a session
  merging in a same-day main-branch change (`9e4a2907`, "drop the RACE page's
  VARIANT row on titles with no variant axis") found its premise wrong for
  Pulse - every one of its eight base teams declares a real, disc-shipped
  `PI_TeamModel name="Concept"` (`extra.vex`), confirmed against
  `pulse-psp-usa.chd` by `crates/game/examples/pulse_variant_probe.rs`. Landed
  as `oag_title::race::HullVariant` (a third, structurally different source
  from `TeamVariants`/`GuestRoster` - see that type's own doc comment for why
  it does not fit either), wired through `--variant`/the RACE page's VARIANT
  row/RACE REMIX, and verified end to end on the real disc
  (`livery_ground_truth.rs::a_hull_variant_swaps_the_players_own_hull_and_nobody_elses`).
- **`PI_TeamModel name="Zone"` names a third file, `zone01.vex`, and it is
  deliberately not offered as a `HullVariant`.** `Data\Ships\<Team>\zone01.vex`
  really does exist for all eight base teams - confirmed by the same probe -
  but it is not what a Zone race loads: that is `Zone.vex`, reached through a
  recovered selector (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`), and
  the two files are not byte-identical. **What `zone01.vex` is for is not
  established.** Diffing it against `Zone.vex` and `Ship.vex` - vertex/triangle
  counts first, geometry after - is the next real step here; until then,
  offering it anywhere would be authoring a stand-in for something the disc
  already specifies differently.
- **The quadrant mapping of the composed fourth texture is conditional.** It is
  read as a linear layout; the format setter's trailing `1` argument may be a
  swizzle flag, which would move each tile. Settle it before drawing the
  composite, or skip the composite (it is only reached for models that have a
  `texture4.tga` at all, which is the race hull and not the front-end preview).
- **What selects `Eliminator` is a state check, not an unlock.** The loader
  picks `ship_eliminator.dat` when a global compares equal to `0x12`, and
  consults no unlock at all on that path; `ship_alt.dat` is the `loyalty`-gated
  one. Confidence 75 on that split. **What `0x12` denotes is a separate and
  much weaker claim (55).** "Eliminator race mode" is the natural reading, but
  there is **no recovered Pulse mode-id table to check it against** -
  `oag_race::Mode` has no Eliminator variant and the twenty-two documented mode
  ids are HD's and 2048's, a different engine - and the enclosing function is
  multiplayer-lobby code, so a lobby game-type id fits the evidence equally
  well. Do not build a mode gate on this number without checking it first.
  **Neither global's address
  is resolved**: both fall inside `.text` under the usual base, the tell that
  the relocation base is wrong for them. They are **not** `$gp`-relative - that
  hypothesis is retracted, this binary never uses `$gp` as a load/store base -
  they need the segment-1 base, and
  `scripts/psp-relocate.py resolve <instruction-address>` reads it straight off
  the relocation record rather than deriving it. Try that first; tracing the six
  non-lobby callers of `Skin_ApplyToModel` - `0x08825638`, `0x088256b8`,
  `0x08828398`, `0x0882885c`, `0x08843804`, `0x088eaa84` - settles it without
  either address if the resolve comes back empty.
- **The generic `%s\%s.dat` fallback path is unexercised.** When the specific
  file fails to load, the loader walks the team node's children for the one
  named `Normal` and rebuilds the path from that child's `+0x94` field. Nothing
  observed what that field holds, so **which file the fallback names is
  unknown** - do not assume it is `ship_alt.dat`. The byte format is the same
  whichever it is, so this does not block the parser.
- **The applier's flag argument was only ever seen as zero.** A non-zero flag
  uploads the *stored* 64x64 block 4 instead of the composite, which is the
  only thing that makes block 4 meaningful at all - and the one traced caller
  passes zero. A parser must read block 4 regardless; what needs settling is
  which callers pass what, which is the same six-caller trace listed below.
- **What `loyalty` accumulates is still untraced**, unchanged from the previous
  thread: whether it is per-save or per-team, and what `Team="any"` changes
  about the check.
- **`catalogue::Team` still does not collect `PI_TeamModel`/`PI_ModelSkin`.**
  Deliberately, and the module's rule still applies: land the fields in the
  same change as the consumer. `oag_title::race::HullVariant` sidesteps this
  for the Normal/Concept hull axis (it needs no per-team catalogue data, since
  every team offers the same two stems) - it is only the *skin* half
  (Alternative/Eliminator) that still wants a real per-team `PI_ModelSkin`
  path, which nothing collects yet.

## Next Steps


- Give the skin axis (`ship_skin::parse` + `mesh::ship_skin::apply`, both
  landed) a real caller: extend `catalogue::Team` to collect `PI_ModelSkin`
  paths under `PI_TeamModel name="Normal"`, then decide how a race picks
  Alternative/Eliminator - mode-gated the way the original appears to (see the
  `0x12` trap below - do **not** build that gate without settling it first),
  or an honest stand-in on the same footing
  [`livery::teams_for_slots`](../../crates/game/src/livery.rs) already stands on.
  This is a design call, and it only needs making once the collection exists.
- Diff `Data\Ships\<Team>\zone01.vex` against `Zone.vex` and `Ship.vex` to
  find out what the third `PI_TeamModel` declares - vertex/triangle counts
  first. Independent of the skin work above.
- Optional and independent: trace the six non-lobby callers of
  `Skin_ApplyToModel` to confirm the mode-vs-unlock split for
  Alternative/Eliminator.
