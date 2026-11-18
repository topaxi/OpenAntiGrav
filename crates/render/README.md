# oag-render

The wgpu renderer's scene: shadows, visibility placement, the track-ribbon
builder and the cameras. The viewer and the game share it, so there is one mesh
pipeline and one camera convention.

Owns no window or event loop; each caller hands it a device, a queue and a target
format. Composes `oag-fx`, `oag-mesh` and `oag-gpu`. No gameplay crate may depend
on it.
