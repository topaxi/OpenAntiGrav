# Title behaviour as data: the last comparison to migrate

2026-10-06. [ADR-0058](../../docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md)
decides that per-title behaviour is `Title` data with a provenance tag and that a
generic crate never compares a title's identity. `just check-title-branching`
(`scripts/check-title-branching.py`) freezes the sites below at the counts in its
`BASELINE`; `oag-raceplay`'s 14 are migrated, and so are `oag-game`'s and `oag-source`'s (2026-10-06); one remains. Each migration lowers the row it touches in the
same change. Sites are `file:line` at main b115ec3ef - `python3
scripts/check-title-branching.py --list` prints the current ones.

## Open

1. ~~**Prove the shape on `oag-raceplay` effects**~~ - done, see the raceplay section below. Still open from it: a loader-report line for a title whose triggers are all `None` (2048, Omega) - no line is written today, as none was before.
2. ~~**`oag-raceplay`'s other flags**~~ - done.
3. ~~**`oag-game`, `oag-source`**~~ - done, see below. **`oag-ui`'s** HD English
   preselect (`frontend.rs:985`) is the last site; the language lane takes it.
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

### oag-game, oag-source (step 3) - done 2026-10-06 (`game-title-data`), 0 left

Where each went (all `Title` data with an `Origin`, behaviour byte-identical:
50 headless stills `cmp`-equal before and after on Pulse PSP EU, Pulse PS2,
Pure EU and USA, HD, 2048, Omega):

- `oag_title::Campaign` (`Title::campaign`, `crates/title/src/campaign.rs`): the
  campaign dialect (`campaign.rs` load and `draws_hd_campaign`, now taking the
  `Title`), the grids file `--campaign-cell` reads (`args.rs`), the circuit and
  loyalty unlock flags (`unlock.rs`, `Gate::read`, `gates_variants`) and HD's
  `Campaign Selection` string overlay (`session/campaign.rs`). Pure and 2048
  are `InheritedFrom("Wipeout Pulse")`, Omega `InheritedFrom("Wipeout HD")`.
- `oag_title::Pressings` (`Title::pressings`, `crates/title/src/pressing.rs`):
  Pure's per-serial movie region, TitleFrame and boot movies (`boot.rs`,
  `boot/movies.rs`); `Measured` off both executables.
- `RaceDefaults::fresh_variant` (`race/fresh.rs`): HD's fresh-profile model
  (`settings/race.rs`), `Origin::Chosen`.
- `GuestRoster::reships`: 2048 names HD as the title whose roster it reships;
  Race Remix's three fallbacks (`session/remix.rs`) read it. Pinned to
  `oag_hd::TITLE.name` by `crates/game/tests/title_behaviour_data.rs`.
- `oag-source`'s Pure hand-over is a table of deny-list openers
  (`PULSE_DENY_LIST_OPENERS`), not a name test; an error naming a title it does
  not list is still Pulse's.

**Still open**: `crates/ui/src/frontend.rs:985`, HD's English preselect - lane
`ptbr-a` owns language selection and takes it. `check-title-branching`'s
`BASELINE` is that one row.

## Next Steps

Start with item 1: read `crates/raceplay/src/absorb.rs` and `hit_sparks.rs`,
sketch `Trigger` and `EffectSpec` per the ADR's section 5, and migrate
`throws_weapon_spark` first (one site, one title, its test already exists).
