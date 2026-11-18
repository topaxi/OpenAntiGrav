# oag-livery

A race's per-slot ship assets: hulls, skins, shield shells, boost plumes and
wrecks, built from a title's own archive entries.

Draws, so it sits with the render-side crates, above `oag-mesh` and `oag-fx` and
without `oag-render`. No gameplay crate may depend on it.
