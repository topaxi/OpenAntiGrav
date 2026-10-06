# Title behaviour as data: the 37 comparisons still to migrate

2026-10-06. [ADR-0058](../../docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md)
decides that per-title behaviour is `Title` data with a provenance tag and that a
generic crate never compares a title's identity. `just check-title-branching`
(`scripts/check-title-branching.py`) freezes the sites below at the counts in its
`BASELINE`; nothing was migrated. Each migration lowers the row it touches in the
same change. Sites are `file:line` at main b115ec3ef - `python3
scripts/check-title-branching.py --list` prints the current ones.

## Open

1. **Prove the shape on `oag-raceplay` effects**: `hit_sparks.rs` (HD Cannon
   spark, Pulse hit sparks), `absorb.rs` (three bursts), `wreck_fx.rs`, then
   `load.rs` / `load/roster.rs` flags. Add `Trigger` / `EffectSpec` /
   `Provenance` to `oag-title`, fill them in `oag-pulse`, `oag-pure`, `oag-hd`,
   give 2048 and Omega `None` with a loader-report line, and check 2048 against
   Omega as CLAUDE.md asks.
2. **`oag-raceplay`'s other flags** (`pulse_ps2.rs`, `pulse_psp.rs`,
   `pulse_laid_pose`): a title-and-platform predicate wants to be a title field.
3. **`oag-game`, `oag-ui`, `oag-source`**: the front-end quirks. Least uniform;
   shape them from what step 1 proved. Race Remix's HD/2048 rules in `remix.rs`
   are ADR-0035's fallback and become a `Title` field naming the title that
   reships the roster.
4. **Reconcile with merges.** Lanes running on 2026-10-06 (2048-sky-fog,
   hd-anim-textures) may have added sites; the lead lowers or corrects the
   baseline at merge.

## The sites, by crate

### oag-raceplay (migration step 1 and 2) - 14

- `crates/raceplay/src/absorb.rs:113`: `if title.name == oag_pulse::TITLE.name {`
- `crates/raceplay/src/absorb.rs:115`: `} else if title.name == oag_pure::TITLE.name {`
- `crates/raceplay/src/absorb.rs:117`: `} else if title.name == oag_hd::TITLE.name {`
- `crates/raceplay/src/hit_sparks.rs:86`: `if title.name != oag_pulse::TITLE.name {`
- `crates/raceplay/src/hit_sparks.rs:130`: `title.name == oag_hd::TITLE.name`
- `crates/raceplay/src/load/pulse_ps2.rs:9`: `title.name == oag_pulse::TITLE.name && archives.layout.platform == oag_assets::Platform::Ps2`
- `crates/raceplay/src/load/pulse_psp.rs:16`: `title.name == oag_pulse::TITLE.name && archives.layout.platform == oag_assets::Platform::Psp`
- `crates/raceplay/src/load/roster.rs:211`: `hull_overlay: oag_fx::hull_overlay::DRAWN && craft_title.name == oag_pulse::TITLE.name,`
- `crates/raceplay/src/load/roster.rs:214`: `&& craft_title.name == oag_pulse::TITLE.name,`
- `crates/raceplay/src/load/roster.rs:216`: `&& craft_title.name == oag_pulse::TITLE.name`
- `crates/raceplay/src/load/roster.rs:218`: `absorb_shell: craft_title.name == oag_hd::TITLE.name,`
- `crates/raceplay/src/load.rs:874`: `shield_palette: if craft_title.name == oag_hd::TITLE.name {`
- `crates/raceplay/src/load.rs:887`: `pulse_laid_pose: craft_title.name == oag_pulse::TITLE.name,`
- `crates/raceplay/src/wreck_fx.rs:75`: `if title.name != oag_pulse::TITLE.name {`

### oag-game (step 3) - 21

- `crates/game/src/boot/movies.rs:53`: `if title.name == "Wipeout Pure" {`
- `crates/game/src/boot/movies.rs:69`: `if title.name != "Wipeout Pure" {`
- `crates/game/src/boot.rs:478`: `if title.name == "Wipeout Pure" {`
- `crates/game/src/campaign.rs:185`: `title_name == oag_hd::TITLE.name || title_name == oag_omega::TITLE.name`
- `crates/game/src/campaign.rs:238`: `if title.name == oag_hd::TITLE.name {`
- `crates/game/src/campaign.rs:241`: `if title.name == oag_omega::TITLE.name {`
- ~~`crates/game/src/capture/endrace_page.rs:209`: `if title.name == oag_hd::TITLE.name {`~~ migrated 2026-10-06 by the `2048-endrace` lane: `oag_title::FrontEnd::endrace_style`, read through `oag_game::endrace::dialect`.
- ~~`crates/game/src/endrace.rs:166`: `if title.name == oag_hd::TITLE.name {`~~ migrated 2026-10-06 by the `2048-endrace` lane: `oag_title::FrontEnd::endrace_style`, read through `oag_game::endrace::dialect`.
- `crates/game/src/main/args.rs:372`: `t if t == oag_pulse::TITLE.name => oag_pulse::campaign::DEFINITION_ENTRY,`
- `crates/game/src/main/args.rs:373`: `t if t == oag_hd::TITLE.name => oag_hd::campaign::DEFINITION_ENTRY,`
- `crates/game/src/main/session/campaign.rs:73`: `if shell.title.name == oag_hd::TITLE.name`
- ~~`crates/game/src/main/session/endrace.rs:172`: `let is_hd = title_ref.name == oag_hd::TITLE.name;`~~ migrated 2026-10-06 by the `2048-endrace` lane: `oag_title::FrontEnd::endrace_style`, read through `oag_game::endrace::dialect`.
- `crates/game/src/main/session/remix.rs:77`: `let has_hd = titles.iter().any(|c| c.title() == oag_hd::TITLE.name);`
- `crates/game/src/main/session/remix.rs:78`: `let has_2048 = titles.iter().any(|c| c.title() == oag_2048::TITLE.name);`
- `crates/game/src/main/session/remix.rs:108`: `(craft_title == oag_hd::TITLE.name)`
- `crates/game/src/main/session/remix.rs:109`: `.then(|| titles.iter().find(|c| c.title() == oag_2048::TITLE.name))`
- `crates/game/src/main/session/remix.rs:292`: `if requested == oag_2048::TITLE.name {`
- `crates/game/src/main/session/remix.rs:296`: `} else if requested == oag_hd::TITLE.name && candidate.title() != oag_hd::TITLE.name {`
- `crates/game/src/settings/race.rs:157`: `if title.name == oag_hd::TITLE.name && self.variant.is_empty() && !self.variant_chosen {`
- `crates/game/src/unlock.rs:133`: `if title != oag_pulse::TITLE.name {`
- `crates/game/src/unlock.rs:189`: `title == oag_pulse::TITLE.name`

### oag-source (step 3) - 1

- `crates/source/src/title.rs:106`: `Err(Error::WrongTitle { title, .. }) if title == "Wipeout Pure" => {`

### oag-ui (step 3) - 1

- `crates/ui/src/frontend.rs:985`: `title.name == oag_hd::TITLE.name && self.preselect_language("English")`

## Next Steps

Start with item 1: read `crates/raceplay/src/absorb.rs` and `hit_sparks.rs`,
sketch `Trigger` and `EffectSpec` per the ADR's section 5, and migrate
`throws_weapon_spark` first (one site, one title, its test already exists).
