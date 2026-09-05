# PS2 texture resolution should be by name, not by ordinal

Follows on from `ps2-track-texture-dedup-collapse.md` (closed 2026-09-05): that
thread asked which of two mechanisms the original PS2 build uses to resolve a
model's texture references, and answered it - see
[`ps2-texture.md`](../docs/formats/ps2-texture.md#the-original-never-suffers-this-collapse-it-resolves-every-texture-by-name-not-by-ordinal)
and [`skycube.md`](../docs/formats/skycube.md#what-this-engine-does-with-it)
for the full evidence. This thread is the implementation the RE thread
deliberately did not attempt in the same sitting.

## What's established (confidence 88, see the two doc pages above)

`Texture_FindOrLoad` (`0x0010c1e0` in `SCES_547.48`) is the only
texture-resolution primitive in the binary, and it is strictly name-based:
`strcpy` the argument, rewrite `.TGA`/`.MIP` to `.PCT`, hash with
`Wad_HashNameString`, look up in a hash-keyed cache. No ordinal-addressed
texture API exists anywhere in the executable. One model-loading pipeline
(`FUN_001c73d0` -> `FUN_001c7550`) was traced end to end confirming a node's
own per-material texture reference is resolved this way; the track loader is
architecturally different (`FUN_00145900` -> `FUN_001debe8` -> `Resource_Load`
+ per-class vtable dispatch) and was not traced to the same depth - that is
the one link short of higher confidence.

Independent of any code reading: for the exact-match control (`01_Track`) and
all five near-miss circuits, every `Texture` node's own declared name (build
prefix stripped, `.tga`->`.pct` rewritten) hashes via `wad::hash_name` to an
entry **already present** in that model's own nested per-model texture-set
WAD, keyed by that entry's own `name_hash` field - 856 matches, zero
exceptions, checked directly with a disc image and no Ghidra project.

**`oag-render`'s current mechanism (`mesh::build_with_textures`'s flat
ordinal indexing, and `mesh::ps2_texture_set`, which already parses each
entry's `name_hash` and discards it on a debug label) is not a faithful
reproduction of anything the original does. It is simply the wrong
mechanism**, and correcting it does not risk trading one wrong picture for
another - the original never had an ordinal to get right or wrong here.

## Open

- The exact call site inside a track's own Mesh-class constructor that
  resolves its materials was not located (obscured behind the per-class
  vtable dispatch `FUN_001debe8` uses) - see `ps2-texture.md`'s "one link"
  paragraph. Not required to proceed (the disc-data corroboration above
  doesn't depend on it), but would raise the combined confidence past 88 if
  anyone locates it.
- Whether the outer archive ever needs consulting for a track's own textures,
  or the nested set alone is always sufficient - the corroboration above only
  checked the nested set, because it was always sufficient on all six
  circuits tested. Untested: whether some node on some *other* circuit's
  track (of the 27 not swept) declares a name that misses its own nested set
  and needs the outer-archive fallback `ps2-texture.md`'s ship/extension-
  rewrite finding already established.

## Next Steps

1. **Change `oag_render::mesh::ps2_texture_set`'s resolution mechanism** from
   positional (`Vec<Option<Arc<ModelTexture>>>` indexed by directory order) to
   name-hash-keyed: parse the nested WAD's entries as today, but index them by
   their own `entry.name_hash` (already parsed, currently discarded - see
   `ps2_textures.rs`'s `format!("#{index} {:08x}", entry.name_hash)`) rather
   than by directory position.
2. **Change the call site in `mesh::build_class`** (`mesh.rs`, where `external`
   currently gets `resize_with`'d positionally) to build the final
   per-node-ordinal `TextureSlots` by, for each `Texture`-class node in
   order, taking its own `vex::texture_asset_path`, stripping the
   `Wipeout PSP\PS2\` build prefix (case-insensitively) if present, applying
   `oag_pulse::ps2_texture_name`'s `.tga`/`.mip`->`.pct` rewrite when
   applicable, hashing with `oag_formats::wad::hash_name`, and looking that
   hash up in the nested set's map - falling back to `None` (not a
   substitute) on a miss, and reporting the miss the way the loader report
   already does for other gaps.
3. **`TextureSlots`'s public shape is unaffected by this** (it stays
   `Vec<Option<Arc<ModelTexture>>>`, one entry per node, in node order) - only
   *how a slot gets filled* changes, when `external` is `Some`. So the ripple
   into `race/assets.rs`, `livery.rs`, `zone_grade.rs` and `rcs/skin.rs` should
   be limited to `ps2_texture_set`'s own call sites needing to also pass the
   model's own node data (currently they only pass the nested-WAD blob) -
   check each one when implementing; `zone_grade.rs`'s own "index is the stage
   number" `TextureSlots` usage is unrelated to `Texture` nodes at all and
   should not be touched.
4. **Ships never showed a shortfall** (no duplicate `Texture` node names found
   there), so this change should be a no-op for every ship on the roster - add
   a regression test asserting exactly that (ship texture slots identical
   before/after) alongside the track-side fix.
5. **Regression test, run on all 32 circuits, not just the 5 near-miss ones**:
   for the 27 exact-match circuits, name-based resolution must produce
   *identical* bindings to the current positional scheme (a no-op there,
   since ordinal happens to equal position when there is no duplicate name);
   for the 5 near-miss circuits, name-based resolution must fill every slot,
   including the previously-white `12_Track` sky face and the 86 previously
   mis-bound (wrong-neighbour) surfaces on the same file. A test that only
   asserts "no slot is empty" would also pass on the current wrong bindings -
   assert against the *content* (e.g. sha256 of the decoded texture, or the
   entry's own `name_hash`) at a handful of known-mis-bound ordinals, not
   just presence.
6. Once the fix lands, sweep `12_Track`'s (and the other four near-miss
   circuits') main art mesh face-by-face the way sky/pads already were in the
   2026-08-27 sweep, to confirm the 86 previously mis-bound surfaces now draw
   correctly and nothing on the 27 exact-match circuits regressed.
7. `just` gate in full once `.rs` changes land (this is a rendering-crate
   change; `oag-render` is exempt from the determinism rules, but
   `check-size`, `check-deps`, `fmt-check`, `lint` and `test`/`test-data` all
   apply as normal).
