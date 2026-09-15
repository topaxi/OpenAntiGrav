# Game boot: `Game_Main`, and four managers the compiler inlined into it

2026-09-15. Functions in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. The natural
next RE pass once this binary itself became an RE target (see
[`README.md`](README.md)): find its own `GameRoot`/`Game_Main`-equivalent boot
chain, the same starting point
[`vita-2048-eu-v104/game-boot.md`](../vita-2048-eu-v104/game-boot.md) and
[`ps3-hdfury-eu/game-boot.md`](../ps3-hdfury-eu/game-boot.md) already used for
their own first RE pass.

**The names here are applied**, from [names.tsv](names.tsv).

## `Game_Main` - `0x0163b920`

**Confidence: 88**

Found the same way as on the other two binaries: `search_strings("Welcome to
WipEout")` returns exactly one match, the literal
`"3...2...1...Welcome to WipEout"` at `0x0183d3a0`, and `get_xrefs_to` on it
names exactly one caller - this function, at `0x0163b942`, a `puts` call one
instruction after the string is loaded.

Evidence, in the order it appears in the disassembly (decompilation of the
whole function times out past 240s - see "Not read" below for what that
means for this score):

- Opens with a run of `sceSysmoduleLoadModule`/`sceCommonDialogInitialize`/
  `sceErrorDialogInitialize`/`sceAppContentInitialize` calls before the banner
  even prints - PS4 SDK module bring-up that has no counterpart on Vita/PS3,
  where the equivalent init happens before `Game_Main` is ever called (from
  the CRT trampoline on PS3, from the SDK's module-start thunk on Vita).
  `get_function_callees` on this function also returns `sceUserServiceInitialize`,
  `sceSaveDataTerminate`, `scePadInit`, `sceAudioOutInit`,
  `sceRemoteplayInitialize`, `sceShareUtilityInitializeEx2` and
  `sceSystemServiceHideSplashScreen` among ~140 callees - the shape of a
  platform `main()` that owns every subsystem's init/term pair, not a leaf
  gameplay routine.
- Sets the window/session title to `"WIPEOUT_HD"` at `0x0163db51` - the same
  literal Vita and PS3 both set from their own `Game_Main`, on a PS4 disc
  whose own title is Omega Collection, not HD - the same signal
  `vita-2048-eu-v104/game-boot.md`'s own confirmation used, now on a third
  binary.
- Constructs `GameRoot`, `SystemRoot`, a `SpeechManager`-shaped object and
  `MusicManager` **inline, in this function's own body** rather than by
  calling out to a separate constructor - see "Four constructors, no longer
  four functions" below. This is new relative to Vita/PS3, where all four are
  distinct functions found through the same `.cpp`-tag technique.
- Between those, calls out to two survivor manager constructors that were
  *not* inlined - [`SoundManager_Construct`](#soundmanager_construct---0x017359c0)
  and [`FrontendRoot_Construct`](#frontendroot_construct---0x013c28f0) - each
  still bracketed by its own `"Create sound manager..."`/`"Create speech
  manager..."`/`"Create music manager..."` log line
  (`0x0182d8d7`/`0x0182d8ef`/`0x0182d90d`), read back from the GameRoot global
  (`0x01f997b8`) each time, the same allocate-then-register-then-construct
  shape Vita's own writeup names.
- Construction order measured here is **sound, speech, frontend, music** -
  Vita's own page records **speech, sound, frontend, music**. Sound and
  speech are swapped between the two builds; frontend and music keep the same
  relative position (frontend between speech and music, music last) on both.
  Recorded as measured, not reconciled - a source-order difference this small
  is as likely a compiler/build artifact as a real code change, and nothing
  here distinguishes the two.

Not raised to 90 (the band `Game_Main` sits at on both other binaries): the
per-frame loop and teardown block after manager construction - roughly a
third of this function's ~22 KB body - was not read at all, the same gap
that caps `ps3-hdfury-eu`'s own `Game_Main` at 88 rather than higher.

## Four constructors, no longer four functions

On Vita and PS3, `GameRoot_Construct`, `SystemRoot_Construct`,
`SpeechManager_Construct` and `MusicManager_Construct` are each their own
function, found by looking up their `.cpp` debug tag independently of
`Game_Main`. On this binary, all four are inlined directly into `Game_Main`'s
own body instead - there is no separate address to name for any of them, so
none gets a `names.tsv` row. This is the same *kind* of finding
[`ps4-omega-eu/README.md`](README.md#the-lineage-question-extended-a-third-time-shared-tree-confirmed-at-the-code-level)
already flagged for `Collision/SimpleMesh.cpp` (heavier inlining on this
binary than its Vita/PS3 counterparts, read but left unnamed there because
the shape didn't transfer cleanly) - here the shape transfers fine, it is
just folded into the caller rather than kept as a callee. Evidence that these
four are the same constructors, not four different objects that happen to
initialize similarly:

| Role | Alloc size (PS4) | Tag written | Same slot/order as Vita/PS3 |
| --- | ---: | --- | --- |
| `GameRoot` | `0x90` | `"C:\WOPS4\Wipeout\Code\Game\GameRoot.cpp"` at `0x0163ecc2` | First object built, stored to the same kind of single engine-wide global (`0x01f997b8`) every later manager reads back |
| `SystemRoot` | `0x4080` | `"C:\WOPS4\Wipeout\Code\System\System\SystemRoot.cpp"` at `0x0163ef04` | Built immediately after `GameRoot`, before any logged manager - same slot Vita/PS3 both use |
| `SpeechManager`-shaped | `0x1d8` | **Not a clean tag** - the field at the usual tag offset (`+0x58`) instead points at a block reading `"file\0Feisar\0Up\0Down\0Left\0Right\0Triangle\0Cross\0Square\0Circle\0L1\0..."` (control-mapping/ship-name strings, verified with `inspect_memory_content`) | Built directly after the `"Create speech manager..."` log, the same log-bracket evidence Vita uses. **Not independent corroboration**: [`weapons.md`](weapons.md#not-yet-verified-leachbeammanagercpp-and-minemanagercpp-share-one-composite-function) later found this exact same block (`&DAT_0183a20e`) reused as a placeholder tag on a completely unrelated object (the `LeachBeamManager`/`MineManager` composite at `0x013781e0`), so its presence here is evidence of a generic reused placeholder on this binary, not evidence this object is specifically a `SpeechManager` - that identification rests on the log-bracket position alone. |
| `MusicManager` | `0x3e0` | `"C:\WOPS4\Wipeout\Code\System\Sound\MusicManager.cpp"` at `0x016400fd` | Built last, directly after the `"Create music manager..."` log - same position as Vita/PS3 |

The `SystemRoot` size is the one measurement that does not line up cleanly:
Vita/PS3 allocate a small outer object (`0x3c`/`0x40` bytes) and separately
link a large nested member (`0x6250` bytes on Vita) onto it, while this build
allocates `0x4080` bytes in one shot - plausibly the two folded into a single
allocation by the same inlining that removed the separate constructor call,
not chased further this pass.

## `SoundManager_Construct` - `0x017359c0`

**Confidence: 90**

Decompiles cleanly (unlike `Game_Main`, small enough not to time out).
Tagged `param_1[0xb] = "Unknown"` on entry, then overwritten with
`"C:\WOPS4\Wipeout\Code\System\Sound\SoundManager.cpp"` before return - the
same placehold-then-real-tag idiom this project's `Memory_Alloc`/`Heap_Alloc`
writeup already documents for Vita's generic-tag default. Sets a vtable
(`*param_1 = &PTR_FUN_0192c460`) and a second function pointer
(`param_1[1] = FUN_01641150`, a function sitting immediately after
`Game_Main`'s own body - the destructor/callback slot the same shape as every
other constructor on this page). Initializes two fixed-size free-list pools
(`0x9000` bytes at `0x90` stride, `0x2b800` bytes divided at `0xf0` stride) -
the same "two fixed-size free-list pools" shape Vita's own writeup names -
and loads bank names by literal string: `"frontend"`, `"Speech_NGP"`,
`"generaltrack"`, `"speech_fe"`, `"Weapons_NGP"`, `"crowd"`,
`"Speech_NGP_Grid"`, plus a `"Ship_NGP"`-gated special case. Vita's own
`SoundManager_Construct` loads `frontend.bnk`, `speech_results.bnk`,
`generaltrack.bnk`, `speech_fe_NGP.bnk` - three of four names match exactly
by root (`frontend`, `generaltrack`, `speech_fe`), the fourth
(`speech_results` vs `Speech_NGP`) is the same role under a different name.
Called from `Game_Main` directly after the `"Create sound manager..."` log
(`0x0163fb87`), the same log-bracket evidence class `Game_Main`'s own
identification rests on.

## `FrontendRoot_Construct` - `0x013c28f0`

**Confidence: 90**

Also decompiles cleanly. Tagged `"Unknown"` then
`"C:\WOPS4\Wipeout\Code\Frontend\General\FrontendRoot.cpp"` before return,
sets a vtable (`&PTR_FUN_01917f10`), and loads
`"data/fe/fe.envsettings"` - the identical path (case aside)
`vita-2048-eu-v104/game-boot.md` names for its own `FrontendRoot_Construct`.
Also builds `sprintf(..., "Data/Plugins/languages\%s", ...)` and references
`"Data/Plugins/tracks"` - the same plugin-loader shape (a language plugin and
a `Data/Plugins/...` list) Vita's own writeup describes, differing only in
which list (`neartitleentries` there, `tracks` here). Not found in this
binary's call graph from `Game_Main` directly by tag search; located instead
via `get_xrefs_to` on the `FrontendRoot.cpp` tag string, which names exactly
this function as its only writer, then confirmed as `Game_Main`'s own callee
(`CALL 0x013c28f0` at `0x0163ffa5`, between the speech and music log lines -
the same relative position Vita/PS3 both use).

## Not read

- `Game_Main`'s per-frame loop and teardown (see its own confidence note
  above) - roughly the last third of the function's body, past the
  `MusicManager` construction this page does read.
- The `SystemRoot`-equivalent object's size mismatch against Vita/PS3 (see
  the table above) - plausible explanation given, not verified.
- Whether the swapped sound/speech construction order (see `Game_Main`,
  above) is a genuine engine change or purely a compiler/scheduling artifact -
  recorded as measured, not resolved.
