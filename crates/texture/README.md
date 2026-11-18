# oag-texture

Every pixel format the originals ship, decoded to RGBA8888, plus a PNG writer.
Four consoles share one crate on purpose: the PS3 and Vita formats share block
maths, and the PSP and PS2 share palette handling.

Its only normal workspace dependencies are `oag-formats` and `oag-bytes`.
