# Game boot: `Game_Main` and the `GameRoot` manager chain, cross-verified against 2048

2026-09-02. Answers the open question
[`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed)'s
lineage finding left standing: whether this binary's own boot sequence
constructs the same higher-level managers `vita-2048-eu-v104`'s `Game_Main`
does, in the same order. It does, for four of five, with the class names
matching literally rather than just by role - see
[`vita-2048-eu-v104/game-boot.md`](../vita-2048-eu-v104/game-boot.md), which
this page's evidence now corroborates.

Read [memory.md](memory.md) first for the per-function TOC defect. Every
function named on this page is confirmed `exact` (module A) via
`scripts/ps3-toc.py toc <addr>` before being trusted - the one address that
is not (see below) is left
unnamed for exactly that reason.

## `Game_Main` - `0x000196e8`

**Confidence: 88**

The process's actual `main(argc, argv, envp)`. Found by walking up from
`GameRoot_Construct`'s one code caller rather than by searching for an entry
point directly:

- `get_xrefs_to(0x00011c40)` gives exactly one code reference:
  `0x0001985c` inside this function, an unconditional call. Everything else
  referencing either `GameRoot_Construct` address is a `[DATA]` xref (vtable
  or RTTI-adjacent table entries, not call sites).
- This function's own only caller, `0x00010360`, is unmistakably CRT startup:
  `sys_initialize_tls`, an `argc`/`argv` copy loop, `sys_time_get_system_time`,
  three `sys_prx_register_library` calls, `atexit`/`at_Exit` registration,
  then `_opd_FUN_000196e8((int)param_1,param_2,param_3)` followed immediately
  by what decompiles as `exit(return_value)`. That is the SDK's
  `_start`-to-`main` trampoline, and this function is what it calls with an
  `(argc, argv, envp)`-shaped signature.
- Body, in call order: `cellSysutilRegisterCallback`, then
  [`RenderManager_CreateInstance`](renderer.md) (already named), then the
  manager chain below, then a large per-frame-shaped block ending in
  [`Game_PresentLoop_q`](engine-trail.md) (already named, confidence 65) and
  a final `_opd_FUN_00018de0()`.

Scored 88 rather than higher: the CRT-caller and manager-chain evidence is
unambiguous, but the large per-frame block between manager construction and
`Game_PresentLoop_q` was not read line by line, only skimmed for its call
shape.

## `GameRoot_Construct` - `0x00011c40`

**Confidence: 90**

The engine's root singleton, allocated and constructed as the very first
thing `Game_Main` does:

```c
iVar7 = FUN_006762b8(0x100, uVar6, 0x21e, 0);  // allocate 0x100 bytes
FUN_00676218(iVar7, 0, 0x100);                 // zero it
*(iVar7 + 0xc) = 0;
_opd_FUN_00011c40(iVar7);                      // construct
*piVar2 = iVar7;                               // store as the global GameRoot instance
```

`0x00011c40` itself tags the object with `PTR_s_GameRoot_cpp_008a562c` -
the literal string `"GameRoot.cpp"` - sets a vtable pointer
(`PTR_PTR_008a5628`) and, distinctively, allocates and links a **nested
`0x2044`-byte member** tagged `"...Code/System/System_LinkObj"`. Compare
`vita-2048-eu-v104`'s own `GameRoot_Construct`
(confidence raised 84→90 by this page): same `"GameRoot.cpp"` tag verbatim,
same first-constructed-singleton role, and a nested member within four bytes
of the same size (`0x2040` there, `0x2044` here) - independently built
codebases do not converge on that by coincidence.

A second address, `0x00011aa0`, decompiles nearly identically (same tag,
same vtable-and-member shape) and is `0x00011c40`'s sibling in the usual
base-object/complete-object constructor pair this project has seen before
(see [mode-manager.md](mode-manager.md#the-names)). It is **not named**:
`get_xrefs_to` finds no code caller for it at all, only a `[DATA]` reference,
so unlike `ModeManager`'s pair there is no second call site to confirm which
role it plays.

## The four managers

`Game_Main` links four more objects onto the just-constructed `GameRoot`
after it, each the same shape: allocate a fixed size tagged with the same
allocator call, `_opd_FUN_003238f8(gameRoot, obj)` to register it as
`GameRoot`'s member, then call the class's own constructor, in this order:

| Address | Name | Size | `.cpp` tag | Confidence |
| --- | --- | ---: | --- | ---: |
| `0x00310138` | `SpeechManager_Construct` | `0xf4` | `SpeechManager.cpp` | 90 |
| `0x003013f0` | `SoundManager_Construct` | `0x9d0` | `SoundManager.cpp` | 90 |
| `0x001641f0` | `FrontendRoot_Construct` | `0x620` | `FrontendRoot.cpp` | 90 |
| `0x002fb528` | `MusicManager_Construct` | `0x378` | `MusicManager.cpp` | 90 |

Every tag was read straight from `scripts/ps3-toc.py map`'s `.cpp` →
function-address table (the `__FILE__`-attribution trick this directory
already relies on), not guessed from role. **This is the same construction
order `vita-2048-eu-v104`'s `Game_Main` uses for its own
`SpeechManager_Construct`/`SoundManager_Construct`/`FrontendRoot_Construct`/
`MusicManager_Construct`** - identical class names, not just a same-role
match, four for four. All four 2048-side confidences are raised or annotated
by this finding; see
[`vita-2048-eu-v104/game-boot.md`](../vita-2048-eu-v104/game-boot.md).

## Not named: the `SystemRoot`-slot object

Between `GameRoot_Construct` and `SpeechManager_Construct`, `Game_Main`
allocates one more object - `0x40` bytes, registered onto `GameRoot` the same
`_opd_FUN_003238f8` way every manager is, in exactly the slot
`vita-2048-eu-v104`'s `SystemRoot_Construct` (`0x3c` bytes, same position)
occupies. That size and position match is suggestive, but its constructor,
`0x006767e8`, resolves to an **`inherited`** TOC per `scripts/ps3-toc.py toc`
- the risky half of this binary's per-function TOC defect - and has no entry
in the `.cpp` attribution map at all, unlike every manager above. Both facts
together mean it is very plausibly a generic allocator/utility routine rather
than a tagged class constructor, or its true TOC is simply wrong the way this
whole defect predicts. **Not decompiled for a name on that basis** - doing so
safely needs the TOC fixed first (`AssignPs3R2FromOpd.java`, via the full
`scripts/import-ps3-eboot.sh` re-import this directory's `HANDOVER.md` entry
already tracks separately). Left as a structural candidate only.

## Not read

The large per-frame block `Game_Main` runs between manager construction and
`Game_PresentLoop_q` - the pad/render/present shape 2048's own `Game_Main`
names explicitly (`FUN_8100014a`/`FUN_81000240`/`FUN_8129417c` there) has no
PS3-side counterpart identified yet. Teardown on loop exit, which 2048's
`Game_Main` also documents, was not chased here either.
