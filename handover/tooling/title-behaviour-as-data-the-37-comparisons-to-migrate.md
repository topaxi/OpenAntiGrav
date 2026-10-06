# Title behaviour as data: the 23 comparisons still to migrate

2026-10-06. [ADR-0058](../../docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md)
decides that per-title behaviour is `Title` data with a provenance tag and that a
generic crate never compares a title's identity. `just check-title-branching`
(`scripts/check-title-branching.py`) freezes the sites below at the counts in its
`BASELINE`; `oag-raceplay`'s 14 are migrated (2026-10-06), 23 remain. Each migration lowers the row it touches in the
same change. Sites are `file:line` at main b115ec3ef - `python3
scripts/check-title-branching.py --list` prints the current ones.

## Open

1. ~~**Prove the shape on `oag-raceplay` effects**~~ - done, see the raceplay section below. Still open from it: a loader-report line for a title whose triggers are all `None` (2048, Omega) - no line is written today, as none was before.
2. ~~**`oag-raceplay`'s other flags**~~ - done.
3. **`oag-game`, `oag-ui`, `oag-source`**: the front-end quirks. Least uniform;
   shape them from what step 1 proved. Race Remix's HD/2048 rules in `remix.rs`
   are ADR-0035's fallback and become a `Title` field naming the title that
   reships the roster.
4. **Reconcile with merges.** Lanes running on 2026-10-06 (2048-sky-fog,
   hd-anim-textures) may have added sites; the lead lowers or corrects the
   baseline at merge.

## The sites, by crate

### oag-raceplay (migration steps 1 and 2) - done, 0 left

All 14 sites are gone (2026-10-06, `raceplay-title-data`) and `oag-raceplay` has no
row in the script's `BASELINE`. Where each went:

- `absorb.rs`, `hit_sparks.rs`, `wreck_fx.rs`: `Title::effect_on(Trigger)` over
  `oag_title::Effects`, filled in `oag-pulse`, `oag-pure`, `oag-hd` (`effects.rs`
  in each); 2048 and Omega carry `Effects::NONE`.
- `load/roster.rs`, `load.rs` (`shield_palette`, `pulse_laid_pose`),
  `load/pulse_psp.rs`, `load/pulse_ps2.rs`: `Title::looks` (`oag_title::Looks`),
  per-platform `Rule`s and a `ShieldPalettes` selector keyed on the craft's archive
  platform.

**Where the shape differs from ADR-0058's sketch** (ADRs are immutable; the same
note is in `crates/title/src/effects.rs`'s module doc):

- `Origin`, not `Provenance`: `oag_title::Provenance` is already the boot chain's
  `Measured`/`Declared` tag, re-exported at the root and matched in `oag-game`.
- `Effects` is a struct of `Option<EffectSpec>` per trigger, not a `&[(Trigger,
  EffectSpec)]`: a const slice cannot be struct-updated, which ADR item 4 wants.
- `EffectSpec::effects` is a list of `Data\Psys` names (a wreck throws three).
- `Burst` moved from `oag_raceplay::AbsorbBurst` into `oag-title`; its three
  constants live in `oag-pulse`, `oag-pure` and `oag-hd`.
- Pure, 2048 and Omega's shield tint is Pulse's by fall through today. It is kept
  and labelled `InheritedFrom("Wipeout Pulse")`; restated in each title package
  because a title package does not depend on another outside dev-dependencies.

**What is still `Chosen`**: every `Rule::UNREAD` (a look a title does not draw
because nothing of it is read). The effect `None`s carry no tag at all, being
absences.

### oag-game (step 3) - 21

- `crates/game/src/boot/movies.rs:53`: `if title.name == "Wipeout Pure" {`
- `crates/game/src/boot/movies.rs:69`: `if title.name != "Wipeout Pure" {`
- `crates/game/src/boot.rs:478`: `if title.name == "Wipeout Pure" {`
- `crates/game/src/campaign.rs:185`: `title_name == oag_hd::TITLE.name || title_name == oag_omega::TITLE.name`
- `crates/game/src/campaign.rs:238`: `if title.name == oag_hd::TITLE.name {`
- `crates/game/src/campaign.rs:241`: `if title.name == oag_omega::TITLE.name {`
- `crates/game/src/capture/endrace_page.rs:209`: `if title.name == oag_hd::TITLE.name {`
- `crates/game/src/endrace.rs:166`: `if title.name == oag_hd::TITLE.name {`
- `crates/game/src/main/args.rs:372`: `t if t == oag_pulse::TITLE.name => oag_pulse::campaign::DEFINITION_ENTRY,`
- `crates/game/src/main/args.rs:373`: `t if t == oag_hd::TITLE.name => oag_hd::campaign::DEFINITION_ENTRY,`
- `crates/game/src/main/session/campaign.rs:73`: `if shell.title.name == oag_hd::TITLE.name`
- `crates/game/src/main/session/endrace.rs:172`: `let is_hd = title_ref.name == oag_hd::TITLE.name;`
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
