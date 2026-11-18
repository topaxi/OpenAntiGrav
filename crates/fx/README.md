# oag-fx

The renderer's visual effects: the `.pob` particle player, the exhaust, mist,
clouds, beams, flashes and weapon quads. Each keeps its state machine free of wgpu
so it tests on a CPU.

Render-only: no gameplay crate may depend on it. Sits above `oag-gpu` and
`oag-mesh` and below `oag-render`, with no normal dependency on a title package.
Particle format: [pob](../../docs/formats/pob.md).
