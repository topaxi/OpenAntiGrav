# oag-gpu

The small GPU vocabulary the renderer's crates share and none of them may own:
the offscreen target formats, the off-by-default perf probe and GPU timestamp
timing.

It sits below `oag-mesh` and `oag-post`, which is what keeps those two from
depending on each other. No workspace dependency.
