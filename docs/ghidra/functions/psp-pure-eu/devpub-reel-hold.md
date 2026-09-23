# `/pure/BOOT-psp-pure-eu.BIN` - the dev/pub reel's 2.0s hold is a real immediate, not a global read

Settles the two open questions
[`docs/architecture/pure-boot.md`](../../../architecture/pure-boot.md) left about the
dev/pub reel's frame holds: **what the pause *duration* actually is**, and whether the
function implementing it is reached from the boot sequence at all. Both are now read
directly rather than inferred - see [pure-status.md](../../../formats/pure-status.md)'s
"The dev/pub reel's hold duration is measured, not imported" section for the summary this
page backs.

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure EU, `UCES-00001`), image base `0x08804000`, and Pure USA (`UCUS-98612`), same base |
| **Decompiler constant offset** | Same as `title-screen.md`: every address this decompiler prints as a bare `_DAT_*` or a `lui`/`addiu` pair is the real address minus `0x08804000`. |

## The earlier reading conflated the clock with the duration

The handover thread's own framing - "the block loads a float from
`*(base+0x53d68) + 0x44`" - is correct about *an* address the block reads, but wrong about
*which value that read is*. `*(0x00053d68)` is a pointer to an engine clock struct (not an
offset off the executable's own image base at all - it is a real, separate address the
executable dereferences), and `+0x44` inside it is a running "current time" field. The
function samples that field **twice**: once to record when a hold begins
(`this->hold_started = *clock`), and again every subsequent frame to compute elapsed time
(`*clock - this->hold_started`). Neither read is the duration; the duration is what elapsed
time gets compared *against*.

## The comparison threshold is `2.0`, built as a raw bit pattern, identically on both pressings

`DevPubReel_UpdateFrameHolds` (`0x0894b1f0` EU / `0x0894b678` USA, byte-identical past
relocated call targets) is the per-frame update for the dev/pub reel's own state, gated on
`this->entered_count != 0` (set by a sibling init function, unread in this pass). It reads
the widget's frame counter, and on the three pause frames (`0x90`=144, `0xe7`=231,
`0x104`=260 - the same `li` immediates the handover thread already found) begins a hold and
samples the clock. Every frame while a hold is active:

```c
if ((fVar3 != 0.0) && (2.0 < *(float *)(_DAT_00053d68 + 0x44) - fVar3)) {
    FUN_088cad90(*(undefined4 *)(param_2 + 0xe0));   // resume playback
    *(undefined4 *)(param_2 + 0xe8) = 0;
    ...
}
```

`2.0` here decompiles as a bare literal because the disassembly builds it as a raw IEEE-754
bit pattern rather than loading it from `.rodata` or any global - the classic MIPS
float-immediate idiom, present at the identical relative offset on both pressings:

```
0894b358: lui a0,0x4000      # a0 = 0x40000000
0894b360: mtc1 a0,f14        # f14 = bit-cast to float = 2.0
0894b35c: sub.s f12,f13,f12  # f12 = now - hold_started
0894b364: c.le.s f12,f14     # branch stays held while elapsed <= 2.0
```

Read directly off memory on both binaries (`disassemble_function`, not just the
decompiler's own folding); the USA binary's `DevPubReel_UpdateFrameHolds`
(`0x0894b678`) carries the identical `lui a0,0x4000 / mtc1 a0,f14` pair at the same offset
from its own function start. **No JAP or KO Pure disc is in this project's corpus**, so
whether a third pressing bakes the same `2.0` is unread - the same limit
`movie-localised-suffix.md` already records for the region suffix.

This retires the "imported from Pulse" framing entirely: `oag_pulse::frontend::HOLD_SECONDS
= 2.0` was never wrong for Pure, it was simply unconfirmed. It is now confirmed independently,
in Pure's own executable, on both pressings.

## Live-confirmed: the breakpoint fires from a real boot, and the hold measures 1.9965s

Read-only static evidence would already justify a high score here, but this pass also ran
it, closing the "reached from the Developer Publisher screen-type handler" question the
handover thread left as "the only candidate, which is strong, but ... inference rather than
a traced call path":

- **PPSSPP v1.20.4, SDL build, Xvfb, own profile, `pure-psp-eu.chd`, true cold boot** (a
  fresh `HOME`, so no stale savedata - see the trap `pure-boot.md` already records). The
  boot landed on `Language Selection`, matching the documented sequence.
- A breakpoint on `DevPubReel_UpdateFrameHolds`'s entry (`0x0894b1f0`) fires even before the
  language is confirmed - it runs every frame the parent `Intro Screen` is alive (which
  includes `Language Selection`, a child screen), not only while `Developer Publisher
  Screen` is showing. This refines rather than refutes the handover thread's premise: the
  function itself is not screen-gated, only its *effect* is, because nothing sets the movie
  playing until the reel screen is entered.
- A breakpoint on the frame-144 hold's own pause call (`0x0894b2a4`, `jal
  DevPubReel_Pause_q` - unrenamed callee, confidence below the naming floor this pass) fired
  **1.70s of wall-clock time after `input.buttons.press(cross)` confirmed English**, which is
  consistent with the boot proceeding through `Developer Publisher Screen` and the reel
  reaching frame 144, corroborating `pure-boot.md`'s own screenshot-based table (which
  measured 4.5-5.0s from the same event using coarser 0.5s screenshot sampling - the gap is
  plausibly the picker's own settle time, not measured separately here).
- A breakpoint on the matching resume call (`0x0894b374`, `jal DevPubReel_Resume_q`)
  measured the actual held span off the PSP's own emulated cycle counter rather than wall
  clock: **443,220,048 ticks at 222,000,000 Hz = 1.9965s**, against the addend's own `2.0`.
  The 0.0035s shortfall is one frame's worth of scheduling slop at 60Hz (`1/60 = 0.0167s`),
  well inside that budget.

Both breakpoints were then cleared and the session torn down; nothing was left attached.

## Confidence: 92

Higher than `movie-localised-suffix.md`'s 85 for the same reason `title-screen.md` reaches
90 over it: a live breakpoint fired during a real cold boot, not just a static read. Higher
than 90 because this pass has *two* independent live measurements agreeing with the static
one (the call-path timing and the 1.9965s span), where `title-screen.md`'s 90 rests on one.
Short of higher because the sibling init function (the one gating
`entered_count`) and the two callees `DevPubReel_UpdateFrameHolds` invokes for pause/resume
were not renamed or independently characterised this pass - they are referenced above only
by address and a provisional `_q` role name that is **not** written to `names.tsv`, per the
below-70-confidence rule.

## Applied names

| Address | Name | Confidence | Binary |
| --- | --- | --- | --- |
| `0x0894b1f0` | `DevPubReel_UpdateFrameHolds` | 92 | Pure EU |
| `0x0894b678` | `DevPubReel_UpdateFrameHolds` | 92 | Pure USA |

## Next steps

- The sibling init function (`0x0894b6f4` EU / analogous in USA) that gates
  `entered_count` and resolves the two widget-path hashes (`0x271970`/`0x27197c`) is
  unrenamed - its role is plausible from the decompile (an `OnEnter`-shaped widget lookup)
  but not independently confirmed the way the hold logic now is.
- `FUN_088cad74`/`FUN_088cad90` (pause/resume) and `FUN_088cadac` (frame-counter read) are
  called by `DevPubReel_UpdateFrameHolds` on both pressings but are themselves unnamed;
  likely generic `Movie`-widget methods reused elsewhere, not reel-specific.
- No JAP or KO Pure disc exists in this project's corpus, so whether a third pressing bakes
  the same `2.0` immediate is unread, same limit as `movie-localised-suffix.md`.
