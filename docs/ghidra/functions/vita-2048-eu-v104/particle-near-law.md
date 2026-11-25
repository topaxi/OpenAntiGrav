# 2048's particle quad builders and the near-camera law

Patch 1.04, `eboot.elf`, ARM Thumb-2, read statically in Ghidra on 2026-11-25. The question
came from HD's law ([particle-render.md](../ps3-hdfury-eu/particle-render.md)): does 2048's CPU
build cull, fade and cap its particles by view depth?

**Yes, the same law with the same constants.** Two builders carry it, both called from
`FUN_8128e580` (`0x8128eb62` calls one, `0x8128f2cc` and `0x8128f546` the other). Which render
modes each serves was not read.

| Address | Name | What differs |
| --- | --- | --- |
| `0x8128ceca` | `Particle_PushScaledQuadBatch` | The particle's size (`particle + 0x68`) is multiplied by the instance's scale (`instance + 0x64`) before the cap |
| `0x8128e100` | `Particle_PushQuadBatch` | The particle's size is taken as stored |

Per live particle, `z` the view-space z of its centre (negative in front of the eye), both read:

1. **Cull.** The quad is written only when `z <= -1.0`: a particle nearer than 1 unit draws
   nothing.
2. **Fade.** When `z > -4.0`, `fade = (z + 1.0) * -0.33333334`, else `1.0`. The particle's alpha
   byte (`particle + 0x6f`) is multiplied by it.
3. **Cap.** With `cap = *(float *)(emitter + 0xc80)`, `emitter` at `instance + 0x28`: when
   `size / z < -cap`, `size = -(z * cap)`. The half width is the emitter's `+0x4c8` times `size`.

The constants are `vmov.f32` immediates (`-1.0`, `-4.0`) and a literal `-0.33333334`, so the
earlier scan for `1/3` as a data literal found nothing.

Confidence **88**: the three rules and their order are the same instruction shapes as HD's SPU
job and Omega's CPU twin, read here in two functions. Not seen live.

## The data

Every billboard emitter (render modes 1 to 4) of 2048's `.pob` files authors a positive cap in
`+0xc80`: `0.2`, `0.3`, `0.5`, `0.6`, `0.8`, `1.0` and a few others from `0.02` to `4.0`
(`crates/fx/tests/psys_2048_ground_truth.rs`, every parsed effect of the five archives).

## Names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x8128ceca` | function | `Particle_PushScaledQuadBatch` | 75 |
| `0x8128e100` | function | `Particle_PushQuadBatch` | 75 |

75 for both: each is read to its last submit call and builds four-vertex particle quads from
the pool with the law above; the names follow Omega's `Particle_PushQuadBatch`
([heat-haze.md](../ps4-omega-eu/heat-haze.md)), and the pairing of the two with 2048's render
modes is unread.

## What it means here

`oag_2048::TITLE.effects.near_law` carries the law as **measured**
(`oag_fx::psys`'s `near` module runs it). Cross-title: **ported** to Omega, which ships the same
law in its own executable.
