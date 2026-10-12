# `/pulse/BOOT-psp-pulse-eu.BIN` - whose handling an opponent flies

**2026-10-12.** The question and the live measurement are on the USA page:
[grid.md, "Each craft flies its own team's handling"](../psp-pulse-usa/grid.md#each-craft-flies-its-own-teams-handling-measured-2026-10-12).
This page carries the EU addresses of the two functions that decide it and
the evidence that the EU code is the same code.

**Answer for EU: each AI craft flies its own team's handling.** Confidence
**88**: the deciding functions diff identical to the USA ones that were
measured live, but no EU runtime trace was taken.

## The two functions

| EU address | USA address | Name | Diff against USA (`diff_functions`) |
| --- | --- | --- | --- |
| `0x08840b24` | `0x08840c74` | `Craft_Construct` | 795 instructions each; 778 equal, 17 differ, every one an address immediate (`lui`/`addiu` pairs into data, `-18424` against `-20392` and the like). Same prologue and epilogue |
| `0x088892e4` | `0x08889488` | `Team_FindByName` | 43 instructions each, **43 equal** |

Read off the EU decompile of `0x08840b24`:

```c
team = Team_FindByName(name /* 5th arg */, 0);    // 0x088892e4
craft->team /* +0x370 */ = team;
if (!team) { name = "Feisar"; team = Team_FindByName("Feisar", 1); ... }
Ship_LoadHandlingStats(team);                     // 0x088c23fc
craft->engine /* +0x94 */ = new ShipEngine;       // Ship_InitCraft 0x088491e0
sprintf(key, "%s HANDLING STATS", name);          // string at 0x08a7b1f0
engine->stats /* +0x6c */ = Registry_Lookup(..., key);
Ship_LoadModel(craft, name);
```

`Team_FindByName` (`0x088892e4`) walks the team list and returns the first
whose `+0x74` name matches the argument by `strcasecmp`, or null.

**What would contradict it:** an EU-only branch in either function. The diff
shows none; the only differences are relocated addresses.

## Names landed

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x08840b24` | `Craft_Construct` | 85 |
| `0x088892e4` | `Team_FindByName` | 85 |

`Craft_Construct` is 85 here against 65 for the USA row (camera.md): the USA
row predates the full read, and this lane read the whole body and watched the
three fields it writes (`+0x370`, `+0x94`, `+0x6c`) on 16 live crafts. The USA
row is another lane's and is left as it is.
