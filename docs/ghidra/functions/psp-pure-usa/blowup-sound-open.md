# `Blowup`'s Pure trigger is not found - what was tried, so it isn't retried

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** blocked. This page is a negative result, recorded per this
project's own rule that a search dead end is worth writing down so the next
pass does not repeat it.

Answers, incompletely, `oag_game::audio::sfx::Cue::Blowup` for Pure - the
last of the nine `Cue` variants still unconfirmed there after
[`rocket-and-collision-fx.md`](rocket-and-collision-fx.md),
[`shield-sound.md`](shield-sound.md), [`dry-play-cues.md`](dry-play-cues.md),
[`exhaust-sound.md`](exhaust-sound.md) and [`lockon-sound.md`](lockon-sound.md)
covered the other eight.

## What is confirmed

**The `~BLOWUP` string exists in Pure's executable, at `0x08a7a648`.**
`search_strings` does not index it (the same gap `TURBO`'s bare text showed
in `dry-play-cues.md` - not every real string is a Ghidra-defined data item),
so it was found by `search_byte_patterns` for the raw ASCII bytes
(`7E424C4F57555000`), landing on one hit. Read directly as memory: `~BLOWUP\0`,
followed immediately by `~AUTOPILOT\0\0autopilot_eng\0\0\0` - the same two
"deliberately unwired" partner strings `sfx.rs`'s `Disengaging` doc comment
already names, confirming this is the right neighbourhood of the string
table (shared with `Shield`/`ShieldActive`'s own strings nearby at
`0x08a7a418`/`0x08a7a428`, `shield-sound.md`).

**The cue is present on Pure's disc**: `just wad sounds
"data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"` lists `~BLOWUP`
(2 waveforms, 1 looping).

**Pulse's own trigger, already fully cited in `sfx.rs`'s prose, was
re-verified at the instruction level** - the jump table address, the case 4
address, `FUN_0883e9b0`, volume `0x400` and the `craft+0x368` gate all check
out exactly as `sfx.rs` states. `Ship_SetState`'s case 4 is reached through
an indirect function-pointer jump table at `0x08a7bc18` (raw offset
`0x277c18 + 0x08804000`), confirmed by reading the table's own 5th entry
(`0x08a7bc28`, index `4*4`) - `0x0004030c`, which `+ 0x08804000 = 0x0884430c`,
matching `sfx.rs`'s citation exactly. That address has no Ghidra function
boundary (only ever reached indirectly), so it was read via
`disassemble_bytes` (`dry_run: true`, no database mutation) rather than
`decompile_function`. **The one genuinely new detail**: the `~BLOWUP` string
itself is an *indirect* load (`lui a0,0x27` / `lw a2,0x76c8(a0)`, a
pointer-table slot at `0x08a7b6c8` holding `0x002776c0`, which
`+ 0x08804000 = 0x08a7b6c0` reads `~BLOWUP\0`) - not something `sfx.rs`'s
prose citation records, and the double-indirection
pattern the corrected `.COLLISIONS`/`ABSORB` reads used, not the direct
`lui`/`addiu` build most of Pure's own cues turned out to use.

## What was tried on Pure, and came up empty

1. **Direct `lui`/`addiu` build of the string's own offset (`0x276648`)**:
   `search_instructions` with `mnemonic: "addiu"`, `operand_pattern: "0x6648"`
   - zero matches. Also tried `ori` in place of `addiu` (some codegens use it
   for a low-half constant that needs no sign extension) - zero matches.
2. **An unqualified `operand_pattern: "0x6648"` sweep** (mnemonic empty, to
   catch `lw`-based indirect loads too) - six matches, all `lw`s indexing a
   *different* base (`lui s1,0x9`/`lui a0,0x9`, i.e. offset `0x00096648`, an
   unrelated large table, not `0x00276648`) - a coincidental low-16-bit
   collision, confirmed a false trail by disassembling the containing
   function (`FUN_08899b8c`) and finding no relation to sound.
3. **A pointer-table slot holding the string's own address as a raw
   little-endian value** (`search_byte_patterns` for `48662700`, i.e.
   `0x00276648` byte-reversed) - no matches, so if Pure does reach this
   string indirectly, the pointer's own resting place was not found this way
   either (unlike Pulse's own indirect load, confirmed above).
4. **Every direct caller of the dry-play chain's gate helper**
   (`func_0x001209e0`, real `0x089249e0`) - the same nine-site search that
   found `SpeedupPad`/`Disengaging`/`ShieldActive` (`dry-play-cues.md`). None
   of the nine builds `~BLOWUP`.
5. **Every direct caller of the chain's middle hop**
   (`func_0x0002c8f0`, real `0x088308f0`) - the same shape `LockOn` reaches
   directly, bypassing the gate helper (`lockon-sound.md`). 44 call sites
   found; none builds `~BLOWUP`, and a representative sample
   (`FUN_08925d7c`, a countdown-crossing-zero dry-play call at a *different*
   string offset, `0x2764b0`) turned up an unrelated cue on inspection. This
   does **not** rule out `Blowup` living outside these 44 sites: `Disengaging`'s
   own confirmed caller (`FUN_0884c794`, `dry-play-cues.md`) reaches the chain
   through the **gate helper** (method 4 above), not this middle hop, and
   Pulse's own `Blowup` trigger goes through its gate helper too (see the
   re-verification below) - so a middle-hop-only sweep was never guaranteed to
   see it in the first place. Methods 4 and 5 together cover the two known
   entry points into the chain; neither found it.
6. **Fuzzy-matching Pulse's `Ship_SetState` (`0x08844100`) against Pure**
   (`find_similar_functions_fuzzy`, threshold 0.6) - 4,614 matches, all
   weakly and closely tied (0.73-0.76), the exact "false positive shape"
   `corroboration.md` already documents for this method on this binary pair.
   The single most plausible-looking candidate by address proximity
   (`FUN_0884c480`, near `Disengaging`'s own caller) was decompiled directly
   and is an unrelated craft-slot initialiser, not a state dispatcher.

## Why this is a stop, not a guess

Every remaining avenue from here needs either a genuinely different search
strategy (a broad scan for Pure's own indirect-jump-table dispatch pattern,
which none of the other eight cues needed) or a Ghidra project session spent
specifically walking `Ship_SetState`'s Pure counterpart from scratch, neither
of which is a small next step. Per this project's reverse-engineering
methodology, a search dead end is written down rather than pushed through on
a guess - nothing here is renamed, and `oag_game::audio::sfx::Cue::Blowup`
stays on the module-level confidence-50 bet for Pure, same as it already was
for every cue before this thread started.

**One mechanism this session never checked: a register-indirect call
(`jalr`).** Methods 4 and 5 both used `search_instructions` for direct `jal`
targets against the chain's two known entry points. That finds every static
caller, but not a call reached through a register - and the one confirmed
fact about *how* both binaries reach a `Blowup`-shaped state handler is that
`Ship_SetState`'s own case 4 is itself only reachable through an indirect
jump table (see the Pulse re-verification below), never a direct `jal`. If
Pure's caller into the dry-play chain for `Blowup` is likewise reached only
through an indirect table - and calls the chain's gate or middle hop via a
register rather than a literal `jal` immediate - neither of this session's
two caller sweeps would ever have seen it. A future pass should search for
`jalr` instructions whose target register was just loaded from the gate
helper's or middle hop's address, not just literal `jal 0x...` targets.

## What is not verified

- **Pure's trigger for `Blowup` at all** - the whole point of this page.
- **Whether Pure's `Ship_SetState` counterpart uses an indirect jump table
  the same way Pulse's does**, or dispatches its nine-plus states some other
  way - not established either way.

## History

- **2026-09-04.** Written after exhausting the search methods above,
  answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s last open
  Pure cue with a documented negative result rather than a forced guess.
