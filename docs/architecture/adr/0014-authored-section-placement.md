# ADR-0014: Place draw calls by their authored section group, and pad with bits, not masks

## Status

Accepted. Supersedes two parts of
[ADR-0011](0011-authored-pvs-before-frustum-culling.md): its decision item 6
(geometric multi-section placement) and the mask-union padding in decision
item 2. Everything else in ADR-0011 - PVS before frustum, the authored set,
spline-based adjacency, unknown-means-everything - stands. Decision item 2's
plain union is itself refined by
[ADR-0015](0015-ordered-visible-set-union.md), which orders the sources and
keeps authored alternatives from rejoining across them.

## Context

ADR-0011 was written when nothing had been recovered about **which section a
mesh belongs to**. It therefore invented an association - a draw call belongs
to every section whose authored box its bounding sphere touches - and
validated it by the only bar available at the time: thirty captures
byte-identical to culling nothing. That bar was the problem. An association
that never changes the picture can never *hide* anything the artists hid.

The Moa Therma magstrip investigation (see
[`magstrip_ground_truth.rs`](../../../crates/render/tests/magstrip_ground_truth.rs))
found what they hid. The circuit ships a coarse **far-LOD copy** of its track
surface - one quad across where the detailed strip has five rows, texture
mapped `0..1` where the strip maps `2/128..125/128` - coincident with the
real geometry. Drawn together, the two z-fight and the strip's painted lines
chop into sideways-stepping segments; the original never shows this. Three
facts, all measured from the disc:

1. **The association is authored, structural, and on the disc.** A track is
   authored as sibling groups: one transform per group, whose children are a
   `section` node and the group's geometry. All 64 of Moa Therma's sections
   have this shape, and 97-100% of every PSP track file's draw calls are
   governed by one
   ([`pvs_placement_ground_truth.rs`](../../../crates/render/tests/pvs_placement_ground_truth.rs)).
2. **The far-LOD copy's group is governed by a section (62) that only four
   distant vantage sections list in their masks** - while its *geometry* sits
   inside the racing sections' boxes. Geometric placement therefore put it in
   the racing sections and could never hide it; no spatial rule can.
3. **The masks encode a LOD swap.** No shipped mask names both section 62 and
   a detailed section the copy coincides with: from the start valley you see
   the copy and not the far detail, from the loop you see the detail and not
   the copy. Mutual exclusion is authored per viewpoint - which also breaks
   the mask-union padding, below.

## Decision

**1. A draw call belongs to the single section governing its scene node.**

`oag_formats::pvs::governing_sections` derives the per-node section by the
sibling-group rule (a `section` node governs its parent's whole subtree);
each `DrawCall` carries its source node and gets that section's one bit.
A draw call with no node, no governing section, or a governing section the
track does not declare gets all-ones and always draws - unknown still means
everything.

**2. Padding contributes the neighbours' bits, not their masks.**

The visible set is still built for a camera the original does not cull to:
`visible_from(craft) | visible_from(camera) | near(craft) | near(camera)`,
where `near` is the spline-adjacency neighbourhood *as bits*. The two real
viewpoints contribute everything they see; a not-yet-entered neighbour
contributes only its own geometry, so crossing a boundary never reveals a
blank section. Unioning the neighbours' **masks** - ADR-0011's rule - pulls
in what is visible *from* places the camera is not, and across a LOD-swap
boundary that draws the copy over its original: measured at the Moa Therma
loop, where the craft's section (26) correctly excludes the copy but its
two-hop neighbours (2, 11, 51) are exactly the vantage sections that see it.

## Consequences

- **The artifact this fixes**: the far-LOD copy is hidden while racing, and
  Moa Therma's magstrip draws its lines smooth - verified by A/B capture at
  `--pose 88.2,97.1,-462.3` against the manual skip-the-overlay experiment.
  Vertica's trackside barrier panels stop double-drawing the same way.
- **Culling improves**: ~20-29% of draw calls survive the first tier on the
  live camera-trailing average, against ~50-78% under geometric placement,
  with 97-100% of draws governed.
- **The centre-placement regression stays fixed for the right reason.**
  ADR-0011 rejected one-section-by-centre because a ridge spanning a dozen
  sections vanished; the flaw was the *guessed* bit, not the single bit. The
  authored bit is the one the masks were authored against.
- **A newly-entered section's vista arrives one frame late** in the worst
  case (when neither craft nor camera is in it yet). The original has no
  such transient - it culls by the craft's section alone with no camera set
  to build - so this is still an engine decision, now a strictly smaller one.
- Geometry outside every group (0-2.6% of draws per track) is never culled,
  as before.

## Alternatives considered

### Keep geometric placement and special-case LOD sections

Requires knowing which sections are "LOD alternates", which nothing on the
disc names. The authored association already encodes it, for free, along
with everything else the artists decided.

### Keep mask-union padding

Safer-looking, and wrong in the one place the masks disagree on purpose. The
padding exists to cover a one-frame boundary crossing; the neighbour's own
geometry is what that needs, not the neighbour's whole vista.
