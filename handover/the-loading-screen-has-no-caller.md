# The loading screen has no caller

The rippling band is procedural and the `loading` plugin holds no video; the decode is pinned against the real blob (which is **swizzled**, bit 0 of `+0x07`, and **fully opaque**, so both plausible wrong readings are excluded by construction). **Nothing is runtime-verified against the original, and the game does not draw a loading screen yet.**

## Open

- Nothing about the decode is runtime-verified against the original.
- The game does not draw a loading screen yet.

## Next Steps

- No next step named in the original record - read the prose above and decide one.
