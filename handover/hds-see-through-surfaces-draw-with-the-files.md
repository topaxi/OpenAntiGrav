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

**Chased one level further and it lands in weapon code, confirming the
above.** The six callers are three classes, not six (each pairs up by
identical decompiled bodies, same as `SortRoot.cpp` itself - detail on
`renderer.md`). One of the three is owned, as a plain member, by
`DetonatorBomb.cpp` - now named
([`DetonatorBomb_Construct`/`DetonatorBomb_ConstructComplete`](../docs/ghidra/functions/ps3-hdfury-eu/detonator-bomb.md),
confidence 80, found by the same `__FILE__`-at-offset-`0x30` string
attribution `RaceManager_Construct` uses). `SortRoot`'s only located use is
now two hops deep in a bomb weapon's own constructor, tied to
`RaceManager_GetInstance()` - about as far from "render draw-order root" as
gameplay code gets. That table entry should be treated as refuted pending a
render-side counter-example, not merely unconfirmed.

**2026-08-26 finds the render layer's own dispatch loop, and it narrows the
question further without closing it.** `RenderManager_FlushDrawQueue_q`
(`0x002d6300`) - paired with `RenderManager_PrepareEye_q` (`0x002d6a78`),
called once per eye from `Game_PresentLoop_q` - walks an array at
`RenderManager+0x630` strictly index `0` to the count at `+0x44b0`, calling a
virtual method on each queued object. **No comparison against any per-entry
value happens anywhere in that loop** - it plays the array back in whatever
order it was populated, nothing more. See
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md#the-per-eye-draw-dispatch-and-what-it-says-about-sort-order).
**What this does not show**: where `+0x630` gets populated, or whether it
holds every transparent draw. No literal `0x630` store exists anywhere in the
render layer's address range, so the enqueue site either sits outside it or
uses a computed offset - a different search than this one.

## Open

- Transparent draws are not depth-sorted in the dispatch/flush step (strict
  insertion order, confirmed 2026-08-26); the `SortRoot.cpp` lead is spent
  and the flush loop itself has no sort key, so the only remaining place a
  sort could hide is the **enqueue** side - whatever writes into
  `RenderManager+0x630` before `_FlushDrawQueue_q` reads it back
- `Transparency::Mode2` is not routed to the cutout pass; `material.rs`
  already refutes alpha-test for it, so what it should route to instead is
  unread
- Whether Pulse's own transparent draws should be depth-sorted is unmeasured
  (the question above was scoped to HD only)

## Next Steps

- Find what populates `RenderManager+0x630` (the `{object, extra}` pairs
  `_FlushDrawQueue_q` walks) and what decides the order entries land in it.
  **Three approaches already tried the same session and came up empty**,
  recorded so the next session doesn't re-spend the time: (1) a literal
  `stw ..., 0x630(rX)` search across the whole image, zero hits in the render
  layer's address range; (2) direct data-xrefs to the singleton global
  (`PTR_DAT_008b3d00`, the pointer `RenderManager_CreateInstance` stores
  itself into) - only seven hits, all flag setters and destructor-shaped
  cleanup in `RenderManager.cpp`'s own small function cluster
  (`0x002d4b90`-`0x002d4d78`), none touching `+0x630`; (3) a global `stwx`
  (indexed store) sweep restricted to the render layer's address range - many
  hits, none inspected yet resolve to a `{ptr, extra}` pair write matching
  the queue's shape, and `Model`'s own constructor (`0x002c04b0`) does no
  vtable setup that a reader can see, so verifying which class implements
  vtable slot `0x1c` is still blocked on finding that constructor's *other*
  half or its base class first. A cleaner angle for next time: work forward
  from a specific already-decoded material/model draw call (if one gets
  found for other reasons) rather than backward from the empty queue
- If that search turns up a real sort key on the enqueue side, read it before
  implementing anything - do not add a distance-based sort speculatively. If
  it turns up nothing (submission order only, e.g. scene-graph traversal
  order), that positively confirms "no sort" rather than just narrowing to it
- Separately, and not blocking the above: `renderer.md`'s `SortRoot` note now
  names two of its three unnamed derived classes' shared shape (a
  pooled-allocator-plus-hash-property-lookup pattern) without naming the
  classes themselves - low-priority, since it has no bearing on render draw
  order, but worth closing if someone is already in that address range
