# Is there a fifth handling class?

[`handling-stats.md`](../docs/formats/handling-stats.md) resolves `VECTOR`/`PHANTOM` ordering but explicitly does not answer whether a fifth class exists anywhere in Pulse. **The HD disc narrows it without closing it** (2026-08-17): HD's global `handlingstats.xml` authors the same five `<GlobalClass>` rungs with `VECTOR` first that both Pulse pressings do, and **no HD team file has a `<Class name="VECTOR">` either** - so on the one title whose ladder was supposed to begin at Vector, the per-team shape is Pulse's exactly. That makes "VECTOR is global-only in this lineage" the leading reading rather than "HD has a rung Pulse lacks". Still not chased to a runtime answer; `hds_global_handling_file_authors_vector_first_like_pulses` is the measurement.

**Three independent subsystems now agree, none of them a runtime trace (2026-09-02).** Chasing "check whether a fifth class is referenced or used at runtime" found that the question had already been half-answered and the two answers weren't cross-linked: [ai-stats.md](../docs/ghidra/functions/psp-pulse-usa/ai-stats.md#the-class-index-and-the-dead-vector-branch) (2026-08-17, confidence 88) already documents a second dead-`Vector` branch, in code unrelated to the handling XML - `AiStats_ParseFile` recognises a `VectorStats` tag but never assigns it an index, and no shipped `AIControlStats.xml`/`AIRaceStats_<class>.xml` authors one. That page didn't cite `handling-stats.md`'s finding or vice versa; both now cross-link. A third, found while chasing this thread: the front-end's loading-screen message keys run `MSC_LOAD_VENOM/FLASH/RAPIER/PHANTOM` - four, contiguous, in the block built for exactly this purpose, no `MSC_LOAD_VECTOR`. Confidence 85, same completeness reasoning as the WAD offset chain.

So every shipped-data table anyone has checked - two per-title handling XMLs (Pulse's two discs, HD's), the AI per-class stats file, and the front-end message keys - has exactly four real classes and nothing for a fifth. The name survives only as a string constant recognised by matching code and then discarded.

**A data point, not an answer (2026-09-05).** Every check above is about
Pulse and HD; **Wipeout Pure's per-team files were read for the first time and
they are not the same shape**. Every one of Pure's race teams authors a full
`<Class name="VECTOR">` block in `Data\Ships\<Team>\handlingstats.xml`, beside
the other four - seven of eight teams read directly off `pure-psp-eu.chd`,
with `oag_pure::race::handling_stats` recording the wider sweep at ten of
eleven ship directories. Pulse's per-team files, re-checked the same day the
same way, carry exactly four. So the distinction that matters is **global
versus per-team**: all three titles author five `<GlobalClass>` rungs with
`VECTOR` first, and only Pure backs one with per-team tuning.
`docs/formats/handling-stats.md` had said "no per-team file has a
`<Class name="VECTOR">`" unqualified, which was true of the titles then
measured and is now qualified to them.

**This does not move this thread's question.** Pure having a real fifth rung
is not evidence that Pulse has one - if anything it sharpens the contrast the
findings above already draw, since it shows what a title that *does* ship the
class looks like in the one file that decides what a class does. The four
Pulse-side checks (two handling XMLs, the AI per-class stats, the front-end
message keys) are untouched, and no live runtime check has been attempted on
any title. Recorded here so the next person does not re-derive it, and so that
"Pure has five" is not mistaken for an answer about Pulse.

## Open

- Whether a fifth handling class is ever *live* at runtime is still unanswered by any of this - every finding so far is "the shipped data doesn't contain one" and "the code that would recognise one discards the match", never a traced execution that reaches the discard path or a screen that shows a fifth option.
- No live emulator check has been attempted at all (PPSSPP watchpoint, or driving the front end to see if a class selector ever offers five options).

## Next Steps

- A live runtime check would need a genuine trigger, and none of the three dead branches found so far has one: nothing in the search turned up a debug menu, cheat code, or hidden entry point that would ever author a `VECTOR`/`Vector` element or select index 4. Before spending a PPSSPP session on this, first sweep for such a trigger - a debug/tweak menu string (the `Stats`/`AIStats` block sits next to `StatsArrowScrollUp`/`StatsArrowScrollDown`, which reads like a scrollable dev tuning screen and hasn't been chased) is the most promising lead, since it is the one place a fifth class could be *reached* rather than merely *named*.
- Failing a live trigger, the honest close on this thread is documentary: state in `handling-stats.md` that the question is answered as far as static analysis of shipped Pulse/HD data can answer it, and that closing it further needs either a leaked/prerelease build with a real fifth class or a maintainer decision that "no live trigger found after three independent checks" is sufficient.
