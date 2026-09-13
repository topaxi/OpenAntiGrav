# HD ships a per-chunk PVS, and drawing every chunk was the whole "meshes that should not be there"

2026-08-25, **landed and largely closed**. Format: [hd-pvs.md](../../docs/formats/hd-pvs.md). Engine side, twelve names: [visibility.md](../../docs/ghidra/functions/ps3-hdfury-eu/visibility.md). `oag_rcs::hd_pvs` + `oag_render::pvs::ChunkSet`, five ground truths, `--pvs true|false` to A/B a capture. `track.pvs` gives each cell one bit per `.rcsmodel` chunk; a cell sets about 33% and we drew 100%. **Four candidate causes of the "walls vanish as I approach" report were killed**, worth not re-running: the index mapping (a per-chunk test for the signature a local permutation must leave, a chunk drawn only from far away - 26 of 983 on Talon's, 4 of 1,125 on Anulpha); our chunk list; our padding (we union several cells where the original unions **none**); and **the original gating less geometry, falsified**. **The answer is that the behaviour is authored**: 71%/78% of chunks are drawn by all twenty of their nearest cells, and Anulpha's chunk 335 by 285 cells at a median 957 units and by none of the twenty beside it. **The disc hides large scenery up close on purpose.** **Two knobs are ours**: `CHUNK_PAD` (24) unions the camera's neighbourhood; `CHUNK_TRUST_RADIUS` (64) drops the tier for a craft off the partition. **Still open**: whether our *frame* looks right where the original's does - only a matched-camera RPCS3 comparison settles it, and that needs the deferred camera RE. **Not a PVS problem**: Talon's 56 unaddressed prop nodes, whose hashes exist only in *amphiseum* and *tech_de_ra*.

## Open

- Talon's 56 unaddressed prop nodes (hashes only in *amphiseum* and *tech_de_ra*) remain unaddressed, though confirmed not a PVS problem. **2026-08-26**: also confirmed not a runtime cross-file lookup - the shipped executable fails to resolve the identical 56 hashes at load time and never opens a second circuit's `.rcsmodel`; see [rcsmodel.md](../../docs/formats/rcsmodel.md) item 6.

**2026-09-13: the other open item is resolved.** Whether our drawn *frame*
matches the original's, camera-for-camera, is settled yes for geometry: the
camera pick landed (`docs/reverse-engineering/rpcs3-capture.md`, "The pick
is fixed, and a rendered overlay confirms it") and its 50/50 overlay across
three Talon's Junction poses shows the tunnel's pipes, guard rails, lane
markings and support pylons landing on top of each other with no mirroring
or inversion. This session's own per-region measurement
(`scripts/hd-frame-compare.py`, `docs/ghidra/functions/ps3-hdfury-eu/
renderer.md`'s "The matched-camera comparison") corroborates the same thing
from a different angle at all three poses: the drawn geometry is where the
original's is, and the mismatch that comparison does find (the frame reads
darker) is a lighting/material question, not a PVS or drawn-frame-placement
one. Nothing here calls the culling or the node-pass draw list back into
question.

## Next Steps

None named here. The remaining Open item (the 56 unaddressed prop nodes) has
no next step of its own beyond what its two dated notes already record.
