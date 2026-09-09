# The per-circuit `.EnvSettings` key registrar

Function in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The name here is applied**, from [names.tsv](names.tsv).
Found while chasing why `oag_tables::envsettings`'s HD-keyed accessors
(`SUN_COLOUR`, `AMBIENT_COLOUR`) never resolve a light rig against a Wipeout
2048 `track.EnvSettings` - see
[`docs/formats/envsettings.md`](../../../formats/envsettings.md#wipeout-2048-authors-the-same-shape-under-different-key-names)
for the file-side half of this finding.

## `Environment_RegisterLightingSchema` - `0x810175fc`

**Confidence: 85**

A one-time (`DAT_815cb520`-guarded) registrar for one global `"EnvInfo"`
object (`FUN_812f8af6(&DAT_815cb528, "EnvInfo", 0)`): every key a
`track.EnvSettings` file can carry is bound, in the file's own key order, to a
fixed global address with a baked-in default matching the value a freshly
generated file would carry before an artist edits it. The helper it calls
per key (`FUN_812f9d30`/`FUN_812f9dac`/`FUN_812f9e66`/`FUN_812f9c74`/
`FUN_812f9e28`/`FUN_812f9d6e`/`FUN_812f9bfa`/`FUN_812f9ea4`, unnamed - shape
differs by value arity, not traced further) reads as a generic
"register a keyed field of this shape" primitive, the same role
`FwKeyedText_ParseEntry` plays for HD's own settings reader
(`docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`).

Evidence:

- **Exhaustive, in-order correspondence with the shipped files.** All 61 keys
  the function registers appear, in the same order, in every one of the 14
  `track.EnvSettings` files sampled (10 base circuits, 4 DLC1 circuits) -
  `data/art/published/environments/*/track.EnvSettings` and
  `data/art/published/DLC1/environments/*/track.EnvSettings`, read via
  `cargo run -p oag-assets --example psarc_cat`. No key in the function is
  absent from a file and no key in a file is absent from the function.
- **The tag string is unambiguous.** `"EnvInfo"` names exactly the concern
  this file's own key groups describe (`Lighting`, `Water`, `Debug.Track`,
  `Debug.Ships`).
- **`FrontendRoot_Construct` (`0x8106322a`) references the same key strings**
  and is already named and evidenced elsewhere
  (`docs/ghidra/functions/vita-2048-eu-v104/game-boot.md`) as a debug/tuning
  menu builder, not a second parser - recorded here so nobody re-derives that
  when the same strings turn up again from a different caller.

What holds this at 85 rather than higher: the per-key helper functions
(`FUN_812f9d30` etc.) are not individually named or fully typed, so the
*exact* storage width of a few fields (e.g. whether `"Lighting.Show sun
position"` is a real bool or a tri-state, given its trailing `2` argument
differs from every other bool's `0`) is read from the call shape rather than
from a decoded struct definition.

### `"Lighting.Sun color"` is registered and its consumer is not settled

The function binds `"Lighting.Sun color"` to its own address
(`&DAT_815cb950`), separate from `"Lighting.Sun diffuse colour"`
(`&DAT_815d13f0`) and `"Lighting.Sun specular colour"` (`&DAT_815d1390`) - not
a parser alias, a genuinely distinct field. Two different addressing
conventions are in play for this registrar's fields, and they matter for
anyone chasing a consumer next:

- **`"Lighting.Constant ambient colour"`, both sun-direction keys and the two
  split sun-colour terms** (`&DAT_815d13xx` addresses) are read and written
  by absolute global address - confirmed directly: `FUN_8101bc7a` (the
  per-frame scene setup, unnamed, called every tick) both reads and writes
  `DAT_815d13e0`/`DAT_815d13f0` literally, and `get_xrefs_to` on those
  addresses finds real callers (`FUN_8101ef64`, `FUN_81019988`,
  `FUN_81019fc6`, `FUN_8101d068`, `FUN_8101cf42`, `FUN_81018792`) beyond the
  registrar itself.
- **`"Lighting.Sun color"` and most of the rest of the schema**
  (`&DAT_815cbXXX` addresses) are accessed at a fixed **displacement off
  `Environment_RegisterLightingSchema()`'s own return pointer**, not by
  absolute address - confirmed by `FUN_8101bc7a`'s own
  `*(float *)(iVar36 + 0x43c) * 0.017453292` reading `"Lighting.Sky
  rotation"` (`0x815cb964 - 0x815cb528 = 0x43c`, and degrees-to-radians is
  exactly `Sky rotation`'s documented unit). `get_xrefs_to` on
  `0x815cb950` (`"Sun color"`'s own address) finds only the registrar - which
  proves nothing either way for this addressing mode, the same trap
  `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`
  records for a TOC-relative load on HD's binary.
- A displacement search (`search_instructions` for operand `0x428`, `Sun
  color`'s own offset) returned 275 raw matches project-wide, all noise on
  the two checked - one an unrelated struct at a different base address, one
  `FUN_81017e1a` re-writing the same default this registrar bakes in, not
  reading it. **Not exhaustive**: a proper answer needs the consumer traced
  from an actual `bl Environment_RegisterLightingSchema` call site's return
  register forward, not a literal-operand grep.
- **Corroborating, not proving, that it is unread**: across the 14 files
  sampled, five (`Anulpha_Pass`, `Chenghou_Project`, `bridge`, `park`, `sol`)
  carry the exact triple the registrar's own default initialises the field to
  (`~1.5, 1.294118, 1.0`) - the kind of value an artist leaves untouched on a
  field that does not visibly change the picture. The other nine vary the
  field per circuit, so it is at minimum *sometimes* edited, which argues
  against a build-time-dead field and for "read by something, just not found
  this pass."

Left open rather than guessed at, per `CLAUDE.md`'s rule against wiring a
plausible-looking key on a name match alone -
[`docs/formats/envsettings.md`](../../../formats/envsettings.md) and
[`crates/tables/src/envsettings.rs`](../../../../crates/tables/src/envsettings.rs)
both say so at the point a caller would otherwise reach for `"Sun color"`.

## Open

- The per-key registration helpers (`FUN_812f9d30`, `FUN_812f9dac`,
  `FUN_812f9e66`, `FUN_812f9c74`, `FUN_812f9e28`, `FUN_812f9d6e`,
  `FUN_812f9bfa`, `FUN_812f9ea4`) are unnamed. Naming them would pin down the
  exact storage shape (vec3 vs vec4, bool vs tri-state) the call-site
  argument counts above are inferred from.
- `"Lighting.Sun color"`'s consumer, or confirmation it has none - see above.
  Needs a callers-of-`Environment_RegisterLightingSchema` sweep that reads
  each caller's own use of its return value at displacement `0x428`, not a
  whole-binary literal scan.
- `"Lighting.Sun specular colour"`'s consumer is unlocated too, though its
  liveness (not its reader) is already settled by the same absolute-address
  xref evidence that settled `"Sun diffuse colour"`.
