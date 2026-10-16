# 2048 pays no loyalty: what the executable and the data carry

2026-10-16, lane `loyalty-earning`. Ghidra
`/2048/eboot-vita-2048-eu-v104.elf` and `data.psarc` read in place. No rename comes of
this page; it is a negative finding, confidence **75** (absence of strings and widgets in
the executable and every plugin XML, but the award path itself was not decompiled).

- The executable carries one `Loyalty` string (`0x8142ba80`), the attribute name in the
  `<Unlock>` row parser `FUN_81065ea2` (stored at record `+0x48`). Nothing else in the
  executable names `ER_LOY`, `ER_TOT_LOY`, `loyalty1.1` or `ER_ELIM`/`ER_PZONE`.
- The 87 `data/plugins/**/*.xml` entries hold no `loyalty` attribute on any row; the
  string tables (`languages/*/entries.xml`) still carry Pulse-era `ER_LOY`,
  `ER_TOT_LOY` and `MAN_2_SL` text, shared across the studio's titles and drawn by
  nothing: `EndRace_Definition.xml` and `LegacyEndRace_Definition.xml` have no loyalty
  widget, and the page lists `PASS_BONUS_XP`, `FE_EVENT_XP` (XP) instead.
- Conclusion: **2048 earns experience, not loyalty.** The maintainer chose
  (2026-10-16) that 2048, whose assets Omega re-ships, runs Omega's measured law (HD's,
  `ps4-omega-eu/endrace-loyalty.md`) into the shared per-team total, labelled inherited
  (`Title::loyalty`, `Origin::InheritedFrom("Wipeout Omega Collection")`). That is a chosen
  carry-over, not a recovered one.

The XP system that replaces it is read in `xp-award.md` (not loyalty: falsified, 85).

Cross-check against Omega: checked, differs (Omega's executable carries HD's law,
`docs/ghidra/functions/ps4-omega-eu/endrace-loyalty.md`).
