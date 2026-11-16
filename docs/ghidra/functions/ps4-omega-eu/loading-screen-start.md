# The loading screen's starter, and which kinds anything starts

Function in the base `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, v1.00), image base `0x01000000`; the
1.07 patch's addresses are given beside it and live on this page only
([patch-v107-addresses.md](patch-v107-addresses.md)). The name is applied from [names.tsv](names.tsv). Lane
`vita-race-load-re`, 2026-11-16, as the Omega side of 2048's race load
([vita-2048-eu-v104/race-load.md](../vita-2048-eu-v104/race-load.md)). The format page is
[omega-loading.md](../../../formats/omega-loading.md).

## `LoadingScreen_Start` - `0x01708fe0`

**Confidence: 88.** `void (int kind)`. It saves the kind (`mov r14d, edi`), allocates `0xb18` bytes
(`0x017a5a40`), runs the constructor on them with the kind (`0x01704280(object, kind)`), stores the object in a global
(`0x020e2d80`), and sets the meter's rate pair to `0.115` (`movabs rax, 0x3deb851f3deb851f` into `+0x12c`) and its
ceiling to `94.62` (`0x42bd3d70` into `+0x128`): the same two constants 2048's `LoadingScreen_Start` (`0x812845ce`)
stores into its `+0x90`, `+0x94` and `+0x8c`. `omega-loading.md` called this address "the constructor"; the
constructor proper is its callee `0x01704280`.

## The census of its callers

Every direct `call` and `jmp` to it in the text segment, found by scanning the file for a relative branch landing on
it (the bytes of the executable, not Ghidra's reference table), with the kind read from the `edi` load before each:

| base site | kind | in | 1.07 site | kind |
| --- | ---: | --- | --- | ---: |
| `0x0124fe49` | 8 (`mov edi, 8`) | `ModeManager_ConstructByMode` | `0x0125cb79` | 10 |
| `0x01252b86` | **7** (`mov edi, 7` at `0x01252b5f`) | `0x012527c0` | `0x0125f71f` | 9 |
| `0x01277757` | 8 | `0x01277470` | `0x01288b8c` | 10 |
| `0x0163e722` | 7 | `Game_Main` | `0x0167e085` | 9 |
| `0x0163ea00` | 9 (`mov edi, 9`) | `Game_Main` | none | |
| `0x0163eaa2` | 0 (`xor edi, edi`) | `Game_Main` | `0x0167e2d7` | 0 |

In the 1.07 build the starter and the constructor are one function, `0x01750280` (found by the same `movabs`, at
`0x01751519`), its kinds are numbered two higher (10 is the race load), and Ghidra's reference table lists the same
five calls.

**The value at `0x01252b86` is 7**, the kind `Game_Main` also starts. It lines up with 2048's kind 6 (the picture-less
screen `Game_Main` and the front-end transitions start there): Omega's kinds are 2048's plus one from 4 up (2048's 4,
6, 7, 8 are Omega's 5, 7, 8, 9), and Omega's `Game_Main` passes 7, 9 and 0 where 2048's passes 6, 8 and 0.

**State 5 has no caller.** No site in either build passes 5 (base) or its 1.07 counterpart. A sweep of the whole
base text (2.36 million instructions) finds one `mov dword ptr [reg + 4], 5`, at `0x01011efe`, in an unoptimised
three-field initialiser far from the class (`0x0170....`), so nothing re-kinds a running screen to 5 either. So the percentage-box screen that is 2048's
race load is **dormant on Omega**, the way HD's feature screen (2048's kind 7) is dormant on 2048. Confidence 82: a
call through a pointer with a computed kind would not show, and the starter has no such reference.
