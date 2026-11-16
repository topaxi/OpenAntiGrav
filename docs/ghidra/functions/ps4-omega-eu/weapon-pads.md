# Omega's `WeaponPads_TestCraft`: mode 8 and `0x14` stamp nothing

2026-11-07 (`eliminator-pads`). Base `eboot.bin` (image base `0x01000000`, the patch's addresses
are in [`patch-v107-addresses.md`](patch-v107-addresses.md)). The 2048 counterpart is
[`../vita-2048-eu-v104/weapon-pads.md`](../vita-2048-eu-v104/weapon-pads.md); Omega's is the same function,
re-compiled for x86-64.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x01304c50` | `WeaponPads_TestCraft` | 85 |
| `0x01a10f30` | `g_weapon_pad_refresh_time` (data, one float per speed class) | 80 |
| `0x01a10f50` | `g_weapon_pad_elimination_refresh_time` (data, one float per speed class) | 80 |

Found from the sound cue string `ActivateWPad` (`0x01817662`), whose only two references are in
this function (the same cue 2048's `0x8121f860` plays). Confidence 85: the branch structure is
2048's. The two tables are 80: named from their use here, the `<WeaponPad>` parse that fills them
is `FUN_012d8e30` (the only reference to the `elimination_refresh_time` string at `0x01814664`) and
was not read through to the stores.

```c
// per pad, after FUN_01398b70 (the swept test) reported a hit and the slot checks passed
if (DAT_01f999e4 == 8 || DAT_01f999e4 == 0x14) {   // g_game_mode
    if (hit) goto grant;                           // no stamp
} else {
    ...                                            // not Eliminator: the 0x10..0x16 branch, then
    pad[0x140] = (DAT_01f999e4 == 8 ? elimination_table : refresh_table)[class];   // unreachable ternary
}
```

The same no-stamp arm is also taken when the current game-mode object's `+0x21c` is `1` or `4`
(`DAT_01e31910`, the family test of [`weapon-ai-fire.md`](../vita-2048-eu-v104/weapon-ai-fire.md)), which is how an
Omega campaign Eliminator event reaches it; which event type carries `1` or `4` was not read.

The elimination table is selected only inside the `else`, which a mode `8` never reaches: **Omega
does not stamp a pad in Eliminator (modes `8` and `0x14`), and `elimination_refresh_time` is dead
data**, as in 2048. Read statically, not run: shadPS4 0.20 has no memory readout (the native
`wipeout_pc` port does, but its drivers are scratch and not tracked), so a live Omega stamp is
**not checkable** here; the command that would settle it is the HD one,
`scripts/rpcs3-hd-weapon-pad-poll.py`, pointed at the port's `/proc/<pid>/mem` with the pad object's
`+0x140` timer in place of HD's `+0x160`.
