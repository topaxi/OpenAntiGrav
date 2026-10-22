# HD fidelity lanes queued behind the cold originals

2026-10-21, the render-fidelity drive's queue at wind-down, written down so it survives the session. Each
item has its evidence in a `docs/` page; this file only says what is left and in what order. Every
HD item is judged against the HD originals re-made cold at 1920x1080, Resolution Scale 100
(`docs/reverse-engineering/rpcs3-capture.md`), not the thirteen stored at scale 150, and none may raise
another HD pair's mean grey difference by more than about 0.1-0.2.

## Open

- **Moa Therma pair** (`hd/moa-therma`): 15.05 to 10.32 mean grey (2026-10-21, `hd-one-pair-moa`; the other twelve HD pairs 0.00).
  The glass cause is closed: `paraboloidReflectionTex` is the circuit's `sky.gtf` as a 512x256 dual hemisphere map and
  `dc_windows_a` reads it (`docs/rendering/hd-track-tone.md`, "The reflection map is the sky"). Left on this pair: the cast
  shadow under the craft (the original's is harder), the teal advert wall at the top right (ours 28 grey brighter in its cell,
  slot not found by bisection), texture sharpness, and the glass program's normal map.
- **The other families that declare the sky map** (`glass_reflect`, `ds_booth_glass`, `glasstest`, `mt_windows_a` on Anulpha Pass,
  the water sheets of Vineta K, the mag floors, the pipe family, the ice): the map and its binding exist
  (`mesh::sky_paraboloid`, group 2 binding 17); each family needs its program block read and a `rcs::window_glass`-style tag, one
  per commit with all 13 pairs re-judged. The tint is `ProbeTint::SCENERY` (`{1, 1, 1, 0}`, read on Moa's glass only); a second circuit's
  read would settle whether it is per circuit. The producer program itself is still undecoded.
- **HD's exposure adaptation law** (`docs/rendering/hd-track-tone.md`, "Decide the adaptation law"): the
  live readback matches the mean of the *encoded* per-pixel luma within 1.3 % on three circuits, ours
  encodes the linear mean (15-43 % high). The patch is six lines in `crates/post/shaders/hd_bloom.wesl`
  (`fs_reduce_first` returns the encoded value, `fs_adapt` drops its `pow`); held until judged on the
  cold originals, and the static read of `FUN_003b4690` says linear, so the console's rule is still open.
- **HD's `-1.5` material LOD bias** (`material_lod_bias: Rule::UNREAD`): **re-judged 2026-10-22 on the cold originals, still held off**
  (`docs/rendering/hd-track-tone.md`, "Re-judged on the cold originals"): 12 of 13 pairs' mean grey up, 8 by over 0.2, the Modesto
  tower box 3.2x the original's Laplacian variance already. It does bring back the glass grid's lines (the section's blurred metric
  0.0304 to 0.0410 against 0.0379). **Open: a bias on the glass sheen's grid tap alone** (a chosen scope), and whether RPCS3 applies
  the filter word's bias at all. Anisotropy: the original asks for none, the pair profile draws ours with none, the default is 16x.
- **Glass road mid and far brightness** (Tech de Ra, Talon's): the original's pixel implies the facing
  ramp read at `u` about 0.36-0.5 where ours reads 0.10-0.20. Next: dump draw 354's vertex constant
  `c[209]` (`eyePositionWorldSpace`, `0x3466fc0e`) at the cold pair's tick against our view eye, then
  the chunk's packed normal (same page, same section).
- **The chevron's emissive pulse** (`cf_chevron_pulse`, `docs/rendering/hd-track-tone.md` item 3): the
  pulse's colour and phase (`emask.a * colour * wave`) and its `+ emask.rgb` diffuse term are unwired.
- **`m_diffuse_normal_constantspecular`'s normal alpha**: the RGB is a normal map
  (`docs/formats/rcsmaterial.md`); what its alpha carries is unread.

## Later, not queued

- **Pure's trail queue position**: Pulse PSP queues the trail at `0x4d000000` before list B
  (`docs/rendering/draw-order.md`); whether Pure does the same is unread.
- **The HD race pair's lean**: the PSP race pairs are re-made with the drawn lean (`--pose-lean`); the HD
  race pair (`hd-race/talons-pack`) is not.

## Next Steps

1. Adopt the cold HD originals, run the provenance scan, `just compare-rerender-ours --only /hd/`.
2. Spawn the Moa Therma lane against them.
3. Then, in order: the exposure law, the glass eye dump, the chevron pulse.
