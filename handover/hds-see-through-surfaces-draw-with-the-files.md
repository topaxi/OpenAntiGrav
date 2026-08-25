# HD's see-through surfaces: the blend-equation fix landed, and the texture-coordinate/second-texture residue this file tracked is now resolved elsewhere

2026-08-25. This file's original two open items (2026-08-18) are both stale -
substantial work landed after it and was never folded back in here. Rewritten
to point at where each actually settled, rather than restate them.

**(a) Landed, unchanged**: `Material::blend` carries both blend factors
instead of keying on destination alone - 367 of 2,574 see-through materials
were drawn with the wrong equation. Pulse authors no such pair and is
untouched by construction.
[rcsmodel.md](../docs/formats/rcsmodel.md#the-factor-values-and-which-are-mapped).

**(b) Resolved, not the way this file expected**: the texture-coordinate
residue was diagnosed here as two coordinate *types* sniffed from content
(`Mesh::texcoord_format`, never wired to a draw path). That reading is
retired. What the sniff was separating was not a second coordinate type but a
**different attribute** entirely - a four-byte `tangent` or `colorSet1` in
the last four bytes of a vertex whose real coordinate sits elsewhere. The
file's own vertex-attribute declaration (`+0x58`, gated on the chunk's
`+0x06` layout byte) names the type directly, so there is nothing left to
sniff. See the retirement note at
[`crates/formats/src/rcsmodel.rs:316-335`](../crates/formats/src/rcsmodel.rs)
and `VertexDecl::diffuse_texcoord`/`vertex_decl` in the same file. The trap
that produced the original texel-density reading, and why it looked
convincing, is written up in `HANDOVER.md`'s "But one of them was, and the
shape of it is worth carrying" (Talon's Junction).

**(c) Superseded by three more weeks of work, tracked in its own place now**:
which of the second texture slot's four uses applies per material stopped
being this file's story on 2026-08-18 itself, when the `Crc32_HashString`
wordlist angle opened up. It has since gone through lightmap identification,
variant-key reading, the glass family's second slot, and a lighting-family
split - all on [rcsmaterial.md](../docs/formats/rcsmaterial.md), whose own
`## Open` section (87 of 125 sampler hashes still unnamed, and the microcode
operations themselves) is the current frontier. Do not requote it here;
follow that page.

**(d) Also resolved**: whether ADR-0020's `Rgba8Unorm` decision holds for RSX
the way it does for the PSP's GE. It does not, fully - narrowed by
[ADR-0026](../docs/architecture/adr/0026-hd-authored-lighting-is-linear.md),
measured against an rpcs3 reference frame of Talon's Junction: HD's authored
lighting branch shades in linear light and encodes back to the shared gamma
target; storage and every other title's arithmetic are untouched.

**(e) Still genuinely open, and still deliberately unfixed**: transparent
draws are not depth-sorted anywhere - confirmed again 2026-08-25,
`Model::sort_by_layer`
([`crates/render/src/mesh/order.rs:44`](../crates/render/src/mesh/order.rs))
sorts `transparent_draws` by `draw.layer` (authored list order) only, no
distance term. This is deliberately not "just add a back-to-front sort":
nothing recovered says the original does either, and adding one would be
inventing render behaviour the disc doesn't evidence - exactly what
[ADR-0026](../docs/architecture/adr/0026-hd-authored-lighting-is-linear.md)
required an rpcs3 reference frame before touching. `Transparency::Mode2` also
still is not routed to the cutout pass - confirmed 2026-08-25,
[`crates/formats/src/rcsmodel/material.rs:345-368`](../crates/formats/src/rcsmodel/material.rs)
still treats `Mode2` and `Blended` identically as "see-through", with no
separate cutout path - and Pulse's own depth-sort (or lack of one) has never
been re-measured against this question either.

**2026-08-25's Ghidra sweep found a caller, and it narrows the question
rather than closing it.** `SortRoot.cpp`'s four constructors
(`0x002dbe08`/`0x002dbe50`/`0x002dbe98`/`0x002dbf40`) were recorded as
callerless on all four in the 2026-08-18 renderer sweep. One of them,
`0x002dbe50`, has six real callers - see
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#what-was-deliberately-not-read)
for the addresses and the base/complete split this separates. **All six sit
outside the render layer's own address range, and one cites a generic
`LinkObj.h` pooled-allocator call** - evidence that `SortRoot` is a general
sorted intrusive-list container used well beyond rendering, not confirmation
that it *is* the render draw-order mechanism the "draw-order root" table
entry guessed. No render call site instantiating a bare `SortRoot` was
found either, on either constructor pair. The renderer's actual draw-order
logic - if `SortRoot` is even part of it - is still unlocated.

## Open

- Transparent draws are not depth-sorted anywhere (list order only); the
  `SortRoot.cpp` lead has been chased and narrowed (see 2026-08-25 note
  above), not resolved - the render layer's own draw-order call site is
  still unfound
- `Transparency::Mode2` is not routed to the cutout pass; `material.rs`
  already refutes alpha-test for it, so what it should route to instead is
  unread
- Whether Pulse's own transparent draws should be depth-sorted is unmeasured
  (the question above was scoped to HD only)
- Whether `SortRoot` has anything to do with render draw order at all is now
  in question - its one located call site is non-render

## Next Steps

- Read the six `0x002dbe50` callers' own callers (`0x00137690`, `0x00151ad8`,
  `0x00154cf0`, `0x00137e88`, `0x001525f8`, `0x00155088` - none named, none
  in `names.tsv`) to identify what subsystem at `0x00130000`-`0x00156000`
  they belong to; that tells us what `SortRoot` is actually for before
  spending more time assuming it is render-specific
- Separately, search for the renderer's *own* draw-order mechanism by RSX
  method constants or state-sort call sites, per renderer.md's existing "the
  draw path" open item - do not assume `SortRoot` is it until a render call
  site is found
- If either turns up a real sort key, read it before implementing anything -
  do not add a distance-based sort speculatively
