# oag-mesh

The mesh pipeline: a `.vex` or `RCSMODEL` model decoded into one portable vertex
and index buffer, the pipeline that draws it offscreen or into a surface, the
orbit camera and the headless capture.

Owns no window. Never depends on `oag-render`, `oag-post` or a title package, and
the effects and the renderer are built above it.
