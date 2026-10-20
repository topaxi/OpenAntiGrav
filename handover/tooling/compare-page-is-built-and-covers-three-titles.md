# The compare page is built; HD and Pulse PSP give pairs, four titles are partial, publishing is locked

`just compare-page` builds a static page of original-vs-ours pairs (split slider,
onion skin, side by side, difference) from manifests under
`data/reference/compare-page/`, and `--serve` puts it on the LAN. Usage, the manifest
and per-title recipes with their status: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md).
Landed 2026-10-20 (`compare-page` lane).

## Open

- **`POLICY_ADR` in `scripts/compare-page.py` is unset.** `just publish-compare` refuses
  with "screenshot-policy ADR not found" until the screenshot-policy ADR exists and its
  `## Status` line starts with `Accepted`; set that one constant to its number then. It
  was never run past the lock, and `wrangler` is not installed here.
- **HD: the craft pose is not carried.** `rpcs3-drive.py capture` gives the camera, so
  our craft sits a little off (the original has moved a few frames on). `frame-shot`
  for HD wants a ship pose row too (the autopilot capture `scripts/rpcs3-hd-autopilot.py`
  reads one). A campaign-start pair also has the original in grid slot 8 and ours in
  slot 1: use a Time Trial state.
- **Pure (PSP): not reachable.** `psp-drive.py menu` is Pulse's walk; the camera hook
  `0x0883c13c` is Pulse USA's. Needs a hand walk to a grid, a save state
  (`psp-state.py save --keep`, `--serial` for Pure) and Pure's camera node located.
- **Pulse PS2: no pair.** A saved state restores and shots (private `OAG_PCSX2_DATA`),
  but `pcsx2-trace.py` needs a per-state craft pointer and the saved state's circuit is
  not recorded.
- **2048 (Vita3K): no race reached.** `vita3k-drive.py race`'s fixed-timing tap walk did
  not change screen and the frame was an intro cinematic.
- **Omega (shadPS4): menu only.** MAIN MENU is reached unattended in about 45 s; no
  race was walked. The pair is the original's menu against our own menu page.
- **Pulse PSP saved state did not restore on a fresh instance** (`psp-state.py load`
  placed the file and reported success, the game stayed on its title screen); the cold
  `menu` walk worked.

## Next Steps

1. Fill `POLICY_ADR` when the ADR number is known, then `python3 scripts/compare-page.py --check-publish-lock`.
2. Make a Time Trial state for each title the maintainer wants reviewed and capture
   one pair per circuit with the recipes in the doc.
3. HD ship pose: add the player body pose to `rpcs3-drive.py capture`'s JSON and let
   `render_with` carry it, so a pair is pose-matched as on Pulse PSP.
