# Name the tracked allocator and the manager constructors off Game_Main

2026-08-26. First RE pass on `/vita-2048-eu-v104/eboot.elf` named two
functions - `Game_Main` (`0x81003dd2`) and `GameRoot_Construct` (`0x81000032`)
- see [game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md).
Both were reachable from the boot banner string alone; the next names are one
hop further out, already located but not yet verified or documented.

`Game_Main` calls the same three-argument allocator repeatedly, always as
`FUN_812e5590(size, "<Tag> LinkObj", 0x1ef, 0)` immediately before a
zero-fill and a construct-and-link call - a tracked/tagged heap allocator
(size, debug tag, source line, flags), not a bare `malloc`. Candidate:
`Memory_AllocTracked` or similar, pending a look at what `0x1ef` (497) is -
almost certainly a shared source line number baked in at a call-site macro,
worth checking whether it is the *same* line across every call site (would
confirm a macro-expanded tag rather than a real line number) before naming
around that assumption.

Five more objects are constructed the same way, each right after their own
`FUN_812e5590` allocation and a `FUN_81222014` link-into-GameRoot call (the
same link call `GameRoot_Construct` uses for its `BlockList` member). **Only
three of the five have a `Game_Main` log line naming them** - read the
decompilation in [game-boot.md](../docs/ghidra/functions/vita-2048-eu-v104/game-boot.md)'s
evidence for `Game_Main` again before trusting the "speech/sound/music
manager" framing there: it undercounted by conflating the unlabelled two with
their logged neighbours.

| Address | Alloc size | Preceded by (in `Game_Main`) |
| --- | --- | --- |
| `0x8122ebfc` | `0x3c` | nothing - straight after `GameRoot_Construct` returns |
| `0x812677ea` | `0x110` | `"Create speech manager..."` |
| `0x81262c5a` | `0x10e10` (~68 KiB, by far the largest) | `"Create sound manager..."` |
| `0x8106322a` | `0x570` | nothing - falls between sound's `"done\n"` and music's `"Create music manager..."` |
| `0x8125f796` | `0x398` | `"Create music manager..."` |

Reasonable guesses, **not yet verified**: `0x8122ebfc` (first, unlabelled,
small) as something like an input or options manager constructed before any
audio system; `0x8106322a` (unlabelled, between sound and music, `0x570`) as
a 3D-audio/positional-audio manager the other two depend on. Confidence 0 on
both - hypotheses only, not names, until read.

## Open

- None of the five addresses above have been decompiled yet - the table is
  built entirely from `Game_Main`'s own call shape (allocation size + log
  line proximity), not from reading the callees.
- `FUN_812e5590`'s own body hasn't been read either - naming it needs
  confirming what the four arguments actually are (size/tag/line/flags is
  read off the call sites, not the function).

## Next Steps

- Decompile `FUN_812e5590` first - every other name in this thread depends on
  correctly reading its parameters.
- Decompile the five constructor addresses above, in table order. Each
  should show the same vtable+tag-string shape `GameRoot_Construct` did,
  which is what would make naming them safe at similar confidence - the two
  unlabelled ones especially need their own internal evidence (an embedded
  filename string, a distinctive call target) since `Game_Main` gives them
  no log line to go on.
- Before naming, check [2048 vs HD/Fury lineage](vita-2048-vs-hd-fury-lineage.md)
  first if it has landed - a same-address or same-shape match in
  `ps3-hdfury-eu` would raise confidence past what this binary alone
  supports, per the confidence rubric's "corroborated in a second binary"
  language.
