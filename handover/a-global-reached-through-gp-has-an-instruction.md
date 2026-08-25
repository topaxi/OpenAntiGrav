# A global reached through `$gp` has an instruction displacement unrelated to its address

2026-08-17, found while looking for what writes `DAT_08b32428`. Program-wide there is **no** instruction whose operand text contains `0x2428`, yet the decompiler shows reads and writes of that global all over the weapon code. The PSP ELF uses `$gp` for small data, so the access is `lw v0, <small gp offset>($gp)` and the address never appears as an immediate anywhere. **Consequence, and it is the third trap of this shape in two days**: a displacement sweep on a global's low half can return zero hits on a global that is read constantly, and that zero looks exactly like "nothing touches it". It does *not* undermine the `WeaponAIstats` sweeps - that record is heap, reached through a `this` pointer, not `$gp` - but any future "nothing reads this global" claim has to rule `$gp` out first. The way to do it: read the global in a live emulator with a watchpoint, or find its `$gp` offset from a known access in the decompiler rather than from its address.

## Open

- What writes `DAT_08b32428` is still unfound; a displacement sweep on its low half returns zero hits because the access goes through `$gp`.
- Any future "nothing reads this global" claim has to rule out `$gp`-relative addressing first.

## Next Steps

- Read `DAT_08b32428` in a live emulator with a watchpoint, or find its `$gp` offset from a known access in the decompiler rather than from its address.
