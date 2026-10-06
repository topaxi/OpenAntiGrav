# Generic crates still name title packages by path

2026-10-06. `check-title-reach` (`scripts/check-title-reach.py`) counts
`oag_(pulse|pure|hd|omega|2048)::` in non-comment, non-test code of every crate that is
not a title package. `oag-hud` is at zero and keeps its title crates as dev-dependencies
only. The rest is the backlog, per crate (`python3 scripts/check-title-reach.py --by-crate`):

| Crate | References |
| --- | --- |
| `oag-game` | 123 |
| `oag-raceplay` | 36 |
| `oag-ui` | 14 |
| `oag-livery` | 8 |
| `oag-ui-screens` | 7 |
| `oag-render` | 1 |
| `oag-source` | 1 |

The brief's earlier figures (game 353, hud 61, ...) counted comments, doc links and test
modules; those need no Cargo edge and are not counted.

## Open

- The biggest files: `crates/game/src/campaign.rs` (33), `crates/game/src/boot/campaign2048.rs`
  (27), `crates/raceplay/src/load/campaign.rs` (19). Campaign grids and `SCREEN_ENTRY`
  are per-title data a `Title` field can carry, the way `FrontEnd` does.
- `oag-hud` still has the title crates as dev-dependencies for its tests. They stay until
  a test fixture can be built off `oag_title::Title` data alone.
- The ratchet cannot see test code on purpose (about 60 references in `oag-hud`'s tests
  alone), so a Cargo edge can only be forbidden for normal dependencies: `check-deps`
  needs a dev-dependency exemption, or the test fixtures move to `oag_title::Title` data.
- Omega's `--race` shows no HUD, a known gap (`docs/formats/omega-status.md`: the HUD
  textures are not in the archives); seen again in this lane's before/after captures,
  so the Omega leg of a HUD comparison proves nothing about the HUD.

## Next Steps

1. Take one crate per lane, in the order above after `oag-livery`/`oag-ui` (small, mostly
   `Title` fields that exist): move each reference to a `Title` field or an argument, lower
   the BASELINE rows in the same commit, drop the Cargo edge at zero.
2. When a crate has no title dependency left, add it to a rule in
   `check-dependency-rules.py` so the edge cannot return.
3. Last: `oag-game` itself, where what remains is the composition root and moves into
   `oag-source`'s registry.
- **Decided 2026-10-06 (maintainer, on the lead's recommendation):** when `check-deps` grows the end-state rule, it forbids only normal `[dependencies]` on title packages from generic crates. `[dev-dependencies]` stay allowed: ground-truth tests need each disc's own constants, and fixtures would not exercise the real data.
