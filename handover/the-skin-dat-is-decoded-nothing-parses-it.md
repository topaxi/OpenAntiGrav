# The skin `.dat` is decoded; nothing parses it and nothing selects a livery

2026-09-05. Replaces `team-model-and-skin-variants-are-declared-and.md`, whose
one blocking question - the `.dat` payload format - is answered. That thread's
own predecessor was
[per-team-hulls-are-drawn-which-team-flies.md](per-team-hulls-are-drawn-which-team-flies.md).
This is the implementation half, and it is unblocked now rather than blocked.

**What's known, all of it evidenced.** The schema side:
[`docs/formats/dlc-pack.md`](../docs/formats/dlc-pack.md#entry-0-is-a-manifest)
has the XML - every team declares `Normal`, `Concept` and `Zone` variants, and
`Normal` alone nests two `PI_ModelSkin`s, `Alternative` and `Eliminator`, each
with an `Unlock` pair (the team's own `loyalty` threshold and a `Team="any"`
one, always the higher). The `location="ship"` on `PI_TeamModel` is not the
raceable hull's path; that is still the team directory plus `Ship.vex`.

The format side is
[`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`](../docs/ghidra/functions/psp-pulse-usa/ship-skin.md),
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

- **Nothing parses the format.** `oag-formats` has no reader for it. This is
  the smallest next piece and it is now fully specified - see
  [`ship-skin.md`](../docs/ghidra/functions/psp-pulse-usa/ship-skin.md)'s byte
  table. A ground-truth test against `Data\Ships\Assegai\ship_alt.dat` is
  cheap: entry 632 of `pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad`, 26912
  bytes, header `Assegai\0` then `ms`.
- **Nothing draws a livery.** Now that it is settled as a texture swap, this is
  a variant selection on `Livery` rather than a second model path - the
  question the previous thread could not answer.
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
  same change as the consumer, not before.

## Next Steps

- Write the `.dat` reader in `oag-formats` from the byte table, with an
  `#[ignore]`d ground-truth test over `Assegai\ship_alt.dat`. Take dimensions
  as a parameter or derive them from the total; do not invent a header field.
- Then, in one change, extend `catalogue::Team` to collect
  `PI_TeamModel`/`PI_ModelSkin` and give `Livery` a variant it can select.
- Decide how a skin gets picked for a race - mode-gated the way the original
  appears to do it, or a stand-in the way
  [`livery::teams_for_slots`](../crates/game/src/livery.rs) already is for slot
  assignment. This is a design call, and it only needs making once the parser
  and the collection above exist.
- Optional and independent: trace the six non-lobby callers of
  `Skin_ApplyToModel` to confirm the mode-vs-unlock split above.
