# HD's glow draws, and the coordinate it samples on is the one thing not read

2026-08-31. Supersedes the "texture scroll is unwirable" row, whose blocker is
gone: HD's additive emissive layer and its clock both landed. The mechanism, the
census that cleared it and the reach that sized it are on
[rcsmaterial.md](../docs/formats/rcsmaterial.md), "A surface scrolls off an
engine `time`", and
[scenery-animation.md](../docs/rendering/scenery-animation.md). This row keeps
only what is not there.

**Where it stands.** `mesh::slots::ADD_SECOND` is read per material from the lit
variant's own microcode (`fragment::Program::accumulates`), the tint and the two
coordinate constants ride a per-material table written once at build, and
`mesh.wgsl` samples unit 1 at `(u, (v + a) * b + time)` and adds it. **119 slots
across the 16 circuits**, 62,904 of Modesto Heights' 1,151,776 vertices.
`crates/render/tests/hd_emissive_glow_ground_truth.rs` measures the glow's own
pixel contribution by rendering the same model with the bit cleared.

**A material that accumulates is not necessarily one that scrolls**, and the
first cut of this conflated them. Three populations: it authors its own `a`/`b`
pair and scrolls; it declares `time` but keeps those constants as the shader's
own **inline literals**, which live in the code rather than the material record
and which this reading has not recovered (`nr_billboardholographicscanlines`,
17 slots on one circuit); or it never reads the clock at all. Only the first
scrolls, and `Emissive::rate` is what carries that. Recovering the second
population's rate means following the patch chain to an *unpatched* constant -
`fragment::Instruction::constant` already holds it - and nobody has.

**The one approximation, and it is the interesting residual.** The program
addresses unit 1 from **`f[TC3]`**, and this renderer hands it `texcoord` - the
coordinate it reads out of the last four bytes of a vertex. Whether the UV set
feeding `TC3` in the *lit* variant's vertex program is that one is
**unestablished**. `skin::roles` already carries the same open question for the
second texture's own sampling, and `mesh.wgsl` states it at the call site. Only
`v` moves under the clock, so a mismatch shows as a glow tiled wrongly across a
surface rather than as a missing one - which is why it did not stop the layer
landing, and why nobody will notice it by accident either.

`scripts/ps3-microcode.py vp-file` on a lit variant answers it: read which
attribute the vertex program writes into `o[TC3]`, and compare against the
`0x7a3f521c` that `simpletextureandtexturealphauvoffsetscale` transforms. That
is the same attribute question `uvOffset` has.

**What the clock is has not been established either.** `scene.time` is bound
from `mesh_render::Scene::time` and the race path feeds it the same clock the
scenery animation rides. Whether the original's `time` is the race clock, a
free-running one, or something that pauses is unread - it matters for a
frame-accurate comparison against RPCS3 and for nothing else today.

## Open

- The scroll rate of the ~25 slots whose coordinate constants are shader inline
  literals rather than authored parameters is unrecovered; they draw still.
- The UV set feeding `f[TC3]` is not established, so the glow's coordinate is a
  stated approximation. A mismatch tiles a glow wrongly rather than dropping it.
- What the engine's `time` actually counts is unread.
- 50 accumulating slots get no layer because their second texture is the
  circuit's baked atlas. That refusal is deliberate - adding a light term and a
  sun mask paints a shadow map as a glow - but whether those materials bind
  their emissive somewhere else has not been looked at.
- `uvScale`/`uvOffset` are read and unwired, and are **not** worth doing:
  2,510 of 2,582 records author the identity and `uvScale` is `(1, 1)`
  everywhere on the disc. About fifty billboard surfaces carry a real sub-tile
  offset.
- 236 of the 300 parameter hashes have no preimage
  (`crates/render/examples/hd_param_names.rs` named 64). The three this layer
  uses - the tint and the two coordinate constants - are among the unnamed, and
  are cited by hash throughout.
- Exactly one material on the disc, `hd_waketrail`, is multiply-only rather
  than accumulating, and gets no layer. Nothing has looked at what it wants.

## Next Steps

1. Disassemble a lit variant's vertex program and settle which attribute
   reaches `o[TC3]`. That closes the one approximation in the shipped path and
   answers `uvOffset`'s question at the same time.
2. Only then consider a frame comparison against RPCS3, which is what would
   need the clock's meaning.
