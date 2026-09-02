# Team model and skin variants are declared and documented; the `.dat` payload and the drawing are both still open

2026-09-02. Split out of
[per-team-hulls-are-drawn-which-team-flies.md](per-team-hulls-are-drawn-which-team-flies.md)'s
`PI_TeamModel`/`PI_ModelSkin` bullet, which was pure RE and is now closed on
its own terms - see [`docs/formats/dlc-pack.md`](../docs/formats/dlc-pack.md#entry-0-is-a-manifest)
for the schema and [`docs/formats/handling-stats.md`](../docs/formats/handling-stats.md#related-files)
for the `.dat` shape reading. This thread is the implementation half that
schema unblocks, which is deliberately a separate piece of work from the RE
that found it.

**What's known.** Every team declares four model variants - `Normal` (the
raceable hull, unrelated to this field: the hull path is built from the
team's own `location` plus `Ship.vex`, never from `PI_TeamModel`'s own
`location="ship"` file stem), `Concept`, `Zone`, and `Normal` alone nests two
`PI_ModelSkin`s (`Alternative`, `Eliminator`). Each leaf variant/skin owns an
`Unlock` pair: the team's own `loyalty` threshold and a `Team="any"` one,
always the higher of the two. `ship_alt.dat`/`ship_eliminator.dat` are shape-
read as paletted image data - a name string, an `"ms"` tag, a 16-entry RGBA8
palette, then a payload of nibble values in `0..15` - not meshes, and not
decoded past that shape.

## Open

- **The `.dat` payload format is undecoded past its shape.** No loader has
  been traced, so the dimensions, any header fields between the palette and
  the dense payload, and whether the palette is per-file or shared are all
  unknown. `docs/formats/vex.md:1388` records the loader's own format string,
  `%s\ship_eliminator.dat`, at `0x08a7ff7c` - xref that string in Ghidra to
  find the function that reads it, rather than guessing dimensions from the
  payload length again (52864 nibble-values factors into nothing clean, which
  is itself a signal that there's a real header being read as padding).
- **What `loyalty` actually gates has not been traced.** The XML shape (own
  team below `any`) reads as a loyalty-points unlock system, but nothing has
  confirmed what accumulates loyalty, whether it is per-save or per-team, or
  what "any" vs. the team's own id changes about the check.
- **Nothing draws a second hull or a second livery texture yet.** Whether an
  `Alternative`/`Eliminator` skin is a texture swap on the same geometry or a
  genuinely different model is exactly what the `.dat` decode would settle -
  if it is a texture, `Livery` gains a variant selection; if the payload turns
  out to reference geometry after all, it is closer to the `Normal` hull's own
  loading path.
- **`catalogue::Team` still does not collect `PI_TeamModel`/`PI_ModelSkin`.**
  Deliberately - the module doc's own rule ("a public field no caller reads is
  worse than one that appears when it is needed") still applies until there is
  a decoded format and a drawing path to feed. Collecting the XML fields is
  the easy half once the `.dat` format is decoded; do both together rather
  than landing an unused field first.

## Next Steps

- Xref `%s\ship_eliminator.dat` (`0x08a7ff7c`) to find the loader, and read it
  far enough to answer: what precedes the payload (a real header, not
  padding), what the payload's dimensions actually are, and whether it's a
  texture upload or something else entirely.
- Once the format is decoded and confidence-scored, extend `catalogue::Team`
  to collect `PI_TeamModel`/`PI_ModelSkin` in the same change that adds a
  consumer for them - not before, per the module's own rule.
- If it does turn out to be a livery texture, decide how a skin gets selected
  for a race (loyalty-gated the way the original did, or a stand-in the way
  [`livery::teams_for_slots`](../crates/game/src/livery.rs) already is for
  slot assignment) - that's a design call once the format answers what there
  is to select between.
