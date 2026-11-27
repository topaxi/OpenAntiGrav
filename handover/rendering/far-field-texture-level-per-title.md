---
status: ready-to-wire
titles: [pure, pulse, hd, 2048, omega]
updated: 2026-11-27
summary: "Far-field texture level: 2048 bias ready to wire (90), Pure K to read"
---

# Far-field texture level, title by title

Lane `farfield-lod-titles`, 2026-11-27. The laws, evidence and confidences are in
[texture-level-per-title.md](../../docs/rendering/texture-level-per-title.md). Each item below wires
read values only, as `Title` data, with a test, a break-it, and the other titles' pairs byte-identical.

## Open

- **2048: `material_lod_bias` -1.5, ready to wire** (confidence 90, at the maintainer's gate of 90). In a
  Queens Mall race all 269 distinct disc textures bound over two frames carry the loader's state (bias -1.5,
  trilinear, no anisotropy), none differs (`docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`, "In a
  race"). Bound state is from one boot; a second boot agrees on resident descriptors only. Not measured live:
  Zone and Detonator track skins (static read: mip filter off), HD/Fury circuits, a second circuit.
  Wiring: one `material_lod_bias: Rule { on: Any, origin: Measured }` in `crates/2048/src/lib.rs`'s `Looks`,
  and a test like `oag-hd`'s `the_material_lod_bias_follows_its_rule`. Then re-render the three 2048 pairs.
  Check which model draws 2048's pads: `load/lod_bias.rs` leaves separate pad models at 0, and 2048's pad
  textures read -1.5. `detail_sample` takes no title bias, so 2048's effect programs would not get it. Omega must stay at bias 0 (its S# read 0.0).
- **Pure: the per-texture bias and mode-0 textures follow Pulse's wiring, with one caveat.** Same
  functions (`docs/ghidra/functions/psp-pure-usa/texture-level.md`), K stored at texture `+0x2a0`
  with the 0.2 rebuild. The Modesto bytes fit K about -9.05, not the drawn camera's -8.32. A Pure
  wiring on current-camera K would be 11 to 12 sixteenths coarse on 72 of 88 binds. Read where Pure's
  K comes from (`Camera_StoreTextureLodTerm`'s three callers) before wiring Pure.
- **Not read:**
  - which camera built the Modesto textures' stored K (-9.05, against the race camera's -8.32);
  - Omega's two +1.0-bias samplers' materials;
  - 2048's Zone and Detonator track skins and an HD/Fury circuit, live.

## Next Steps

1. Wire 2048's `material_lod_bias` (about an hour with its test and the three pairs).
2. Read: Pure's K source (about 30 minutes in Ghidra).
