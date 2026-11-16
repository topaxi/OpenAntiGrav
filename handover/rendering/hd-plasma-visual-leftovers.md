---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-16
summary: "HD Plasma landed; bolt core, charge glow, release burst, blast haze, sphere colour, late flight and lights differ"
---

# HD Plasma: what still differs from the original

HD/Fury's Plasma landed 2026-11-15/16 (lanes `hd-plasma`, `hd-plasma-2`, `hd-plasma-3`): the blast
draws with its own three programs, the wind-up (`WO_PLASMA_CHARGING`) and release
(`WO_PLASMA_LAUNCH`) effects play from the hull's `ship_muzzle` node, and the bolt leaves at its
release speed with no 0.75 hold. Evidence, pairs and the ranked list of differences:
`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, section "2026-11-15", subsections "Open" and
"Lineage".

## Open

In the order a player notices them (causes and addresses on plasma.md, "Open"):

1. The flying bolt: the original is a white-hot core in a violet halo; ours a small ring trailing
   violet sparkles. Whether the head's two templates draw under the ball is undetermined; its
   emitters probably ride the effect frame `+0xb0`.
2. The charge glow at 0.9 s is faint on ours (`GLOW` template's scale curve as played, unread).
3. The release burst is wider and more violet with no white core; `FLASH`/`START_FLASH` are muted
   because they white out the frame (same symptom as Pure's absorb burst, cause unread).
4. The blast after 0.5 s: the original keeps a lit haze to 1.2 s; the two explosion lights at
   `*0x008aa260 + 0x30` and `+ 0xa0` are unwired.
5. The sphere's colour is paler: `iblScalePower` and the probe are the scenery glass's value; two
   RSX dumps never caught the sphere's draw.
6. Flight past 1 s bends with a banked road where the original holds its heading
   (`Plasma_Update` `0x0011fc88`, HD's floor arms unread).
7. The Plasma's lights (wind-up two, release one, blast two) are unwired.
8. Lineage: Omega and 2048 are "checked, applies, not wired" for the triggers; their blast
   programs are recooked and not read.

## Next Steps

1. Items 1 to 3 together: capture the head and the release on RPCS3 with the frame-held pose
   (`--pose-hold-ticks`, `scripts/hd-weapon-pair.py`) and read how the templates are played.
2. Item 7 then item 4: wire the five light descriptors through the scene light path once their
   fields are read.
3. Item 6: read HD's floor arms in `Plasma_Update` before changing the follower.
4. Omega/2048: answer both triggers in their effect tables once HD's are final.
