# ADR-0058: Per-title behaviour is `Title` data with provenance, never a title comparison

## Status

Accepted. Narrows [ADR-0022](0022-title-packages.md) item 4 (and, with it,
item 3's "struct of tables" restraint) for the axes named below; extends
[ADR-0025](0025-a-boot-chain-carries-its-provenance.md)'s provenance idea from
the boot chain to every per-title behaviour. ADR-0022, 0023, 0034 and 0035
stand otherwise as written. This ADR records the decision and its guard only;
**no code is migrated by it.**

## Context

What a title *ships* has been data since ADR-0022/0023: `oag_title::Title` is a
struct of static tables a composition root selects once. What a title *does
differently* mostly was not. On 2026-10-06 there were **37 non-test sites in 20
files** of a generic crate asking which title it was holding (counts below):

```rust
// crates/raceplay/src/hit_sparks.rs
pub(super) fn throws_weapon_spark(title: &oag_title::Title) -> bool {
    title.name == oag_hd::TITLE.name
}
// crates/raceplay/src/absorb.rs
if title.name == oag_pulse::TITLE.name { Some(PULSE_ABSORB_BURST) }
else if title.name == oag_pure::TITLE.name { Some(PURE_ABSORB_BURST) }
else if title.name == oag_hd::TITLE.name { Some(HD_ABSORB_BURST) }
else { None }
```

Three things are wrong with that, none of them any single commit's fault:

1. **The title package that knows the answer is not where the answer lives.**
   `oag-raceplay` names `oag-hd`, `oag-pulse` and `oag-pure` to ask them a
   yes/no question their own crate could have answered. Every new title or
   behaviour widens a chain of `else if` in a crate that should not know titles
   exist. One compares the literal string `"Wipeout Pure"` (`boot.rs`).
2. **"Unmeasured titles inherit Pulse's rule" is implicit.** Pulse's law is the
   default path and a title opts out by branch, so nothing says whether a
   title's behaviour was measured, inherited from Pulse on purpose, or just
   never looked at. `hit_sparks.rs:86` (`!= Pulse`) and `:130` (`== HD`) are the
   two opposite shapes of the same unrecorded decision.
3. **The same facts are scattered.** HD's Cannon spark, its absorb burst, its
   shield palette and its `absorb_shell` flag are one title's effect behaviour
   in four files.

ADR-0022 item 4 was right to refuse an engine-side type for axes with a single
measured corpus: "Presentation vocabulary ... stays as plain constants ... because
`pure-status.md` measured none of it", with ADR-0009 item 3's n=1 objection
behind it. That reasoning has a condition, and for **effect triggers and the
per-title laws wired beside them** it no longer holds: Pulse and HD (and, in
part, Pure) are each measured, and the code above is the evidence - three
titles' worth of data, hand-sorted by `if`. The restraint is lifted for exactly
those axes and no others.

## Decision

1. **Per-title behaviour is `Title` data.** An effect trigger, a law, a render
   flag or a front-end quirk that differs by title is a field of `oag_title::Title`
   (or of a table it already owns, such as `RaceDefaults`), filled in by the
   title package. **A generic crate never compares a title's identity** -
   `title.name == oag_X::TITLE.name`, a literal title name, or a match on either.
   It asks the title what it does.
2. **Shared code asks "what does this title do on trigger X?", and the answer is
   an `Option`.** `None` means draw or play nothing: the title either does not do
   it or it is unread. That is CLAUDE.md's "never invent what the assets author"
   made a type, not a convention. A title that has not been looked at gets
   `None`, which is visible in the table, and not Pulse's value by the fall
   through of an `else`.
3. **Every entry carries a provenance tag**, the same idea ADR-0025 gave the boot
   chain, as a required field with no default:

   ```rust
   pub enum Provenance {
       /// Read off this title's own executable or data, or watched running.
       Measured,
       /// Same law as that title, checked and found to apply, not re-measured.
       InheritedFrom(&'static str),
       /// Not measured; picked by the project. Carries no confidence score.
       Chosen,
   }
   ```

   Required, not defaulted, for ADR-0025's reason: it is what stops a guess
   arriving as a measurement by omission. Anything that shows a `Chosen` entry to
   a person says so, as `Declared` boot chains already do.
4. **Title packages build their tables from Pulse's with struct-update syntax**
   (`..oag_pulse::X`) and override only what differs. Inheritance is then a
   visible, greppable `InheritedFrom("Wipeout Pulse")`, in the package that
   chooses it, not an `else` in a consumer. It stays a struct of constants
   selected at boot: ADR-0022 item 3 ("no `trait Game`, no `enum Title`
   dispatch, no registry") and ADR-0034 item 2 stand unchanged.
5. **Shape of it**, from today's `hit_sparks` and `absorb` code. A sketch to fix
   the vocabulary, not a signature; the migration lane owns the real one:

   ```rust
   // oag-title
   pub enum Trigger { CraftHitByWeapon, ShieldAbsorb, Wreck, WallScrape /* ... */ }
   pub struct EffectSpec {
       pub effect: &'static str,          // Data\Psys\<name>.POB, the disc's own
       pub burst: Option<Burst>,          // today's AbsorbBurst, minus the title test
       pub provenance: Provenance,
   }
   impl Title {
       pub fn effect_on(&self, t: Trigger) -> Option<&'static EffectSpec>;
   }
   // oag-hd:   effects: &[(Trigger::CraftHitByWeapon, EffectSpec { effect: "WO_SHIP_SPARK_DAMAGE_WEAPON", .. Measured })]
   // oag-pure: effects: &[(Trigger::ShieldAbsorb, EffectSpec { burst: PURE_BURST, .. Measured })],
   //           ..oag_pulse::EFFECTS   // every other trigger: InheritedFrom("Wipeout Pulse")
   // consumer: `if let Some(spec) = title.effect_on(Trigger::ShieldAbsorb) { ... }`
   ```

   `oag-title` is the type's home because `oag-raceplay` already reads it; the
   effect *names* stay strings the generic `psys::Library` resolves, so
   `oag-fx` still reaches no title package (dependency rule 3).
6. **A ratchet guards the line**: `just check-title-branching`
   (`scripts/check-title-branching.py`, in the `just` gate). A per-file
   `BASELINE` of today's comparisons; a file may drop, never rise, and a file
   not in it may have none. A file that drops prints a hint to lower its row.
   Title packages (`oag-pulse`, `oag-pure`, `oag-hd`, `oag-omega`, `oag-2048`,
   `oag-title`), `tests/`, `examples/`, `tests.rs` and `#[cfg(test)]` modules are
   exempt: a test naming a title is a fixture, not a branch.
7. **Not counted, deliberately**: a lookup that matches a survey entry against a
   runtime-chosen name (`candidate.title() == requested`, `sound`'s
   `theirs.name != title.name`). That selects data and branches on no title. The
   Race Remix code in `remix.rs` that names HD and 2048 *literally* is counted:
   it is a behaviour rule (ADR-0035's fallback) and wants to become data.

### Migration order (not done here)

1. **`oag-raceplay` effects**: `hit_sparks`, `absorb`, `wreck_fx`, then the
   `shield_palette` / `absorb_shell` / `hull_overlay` flags in `load.rs` and
   `load/roster.rs`. This is where the `EffectSpec` shape is proven, on the two
   titles (Pulse, HD) whose triggers are both read.
2. **`oag-raceplay`'s other flags**: `pulse_psp.rs` / `pulse_ps2.rs` (a
   platform-and-title predicate), `pulse_laid_pose`.
3. **`oag-game`**: `campaign.rs`, `endrace.rs`, `unlock.rs`, `args.rs`,
   `boot*.rs`, `settings/race.rs`, `main/session/*`; then `oag-ui` and
   `oag-source`. These are front-end quirks and are the least uniform; they get
   the shape the first step proved, not a guess made ahead of it.

Each step lowers `BASELINE` in the same change. The list of sites is
`python3 scripts/check-title-branching.py --list`.

### What was counted (2026-10-06, main at b115ec3ef)

37 sites, 20 files. By crate: `oag-game` 21, `oag-raceplay` 14, `oag-source` 1,
`oag-ui` 1, `oag-ui-screens` 0. (The 2026-10-06 estimate of "about 42" counted
test and lookup lines; `oag-ui-screens` has none left.) Per file, as in the
script's `BASELINE`: `game/src/main/session/remix.rs` 6;
`raceplay/src/load/roster.rs` 4; `game/src/campaign.rs`, `raceplay/src/absorb.rs`
3 each; `game/src/boot/movies.rs`, `game/src/main/args.rs`,
`game/src/unlock.rs`, `raceplay/src/hit_sparks.rs`, `raceplay/src/load.rs` 2
each; the other eleven files 1 each. Reproduce with
`python3 scripts/check-title-branching.py --list`, or by hand:

```sh
rg -n -e '(==|!=)\s*oag_\w+::TITLE\.name|oag_\w+::TITLE\.name\s*(==|!=)' \
   -e '(==|!=)\s*"Wipeout[^"]*"|"Wipeout[^"]*"\s*(==|!=)' \
   -e '\btitle(_ref)?\.name\s*(==|!=)|\btitle_name\s*(==|!=)' \
   -e '\bmatch\s+[\w.()]*title[\w.()]*name\b' --glob '*.rs' \
   --glob '!crates/{pulse,pure,hd,omega,2048,title}/**' --glob '!**/tests/**' \
   --glob '!**/examples/**' --glob '!**/benches/**' --glob '!**/tests.rs' crates \
   | rg -v '^[^:]+:[0-9]+:\s*//'
```

The `rg` and the script agree at 37. Lanes landing between this and the merge may
add sites; the baseline is reconciled at merge, and a legitimate new site is a
reason to migrate it, not to raise a row.

## Alternatives considered

**Leave it and rely on review.** Rejected: 37 sites accrued across review, each
small. It is the same failure `check-size` was written for.

**A `trait Title` with per-title methods.** Rejected, again, on ADR-0022 item 3
and ADR-0034 item 2: a vtable that behaviour dispatches through is the thing
those forbid, and a table is inspectable, serialisable and diffable where a
method body is not.

**`enum Title` and `match title { .. }` in consumers.** Rejected: it is the same
branching with exhaustiveness, and every new title edits every consumer.

**One provenance for the whole `Title`.** Rejected: HD's boot order was
`Declared` while its layout was `Measured` (ADR-0025). Provenance is per entry.

**Make Pulse's value the default for every trigger.** Rejected: that is today's
implicit rule. A title without an entry should do nothing and say so; inheriting
is an explicit `..oag_pulse::X`, labelled.

**Migrate now.** Out of scope for this lane: the shape wants to be proven on
raceplay's effects before the front-end quirks, which are less uniform, adopt it.

## Consequences

- A new per-title behaviour is a table entry in the title's package with a
  provenance, reviewed there; the consumer does not change.
- `Option` makes an unread title draw nothing, which makes absences visible in
  data and in the loader report instead of looking like Pulse.
- `oag-title` grows an engine-side effect vocabulary, which ADR-0022 item 4 had
  withheld. The risk is the one ADR-0009 item 3 named: a type shaped on too few
  titles. Mitigation: only Pulse and HD (and Pure's absorb) back it, `Trigger`
  is added to per trigger as each is read, and a trigger measured on one title
  alone stays a title-local constant until a second corpus exists.
- Until the migration lands, the ratchet freezes 37 known violations, and a
  contributor who needs a 38th must migrate one first. That friction is the point;
  it will also trip a lane that rebases over a landed comparison, and the lead
  reconciles the row at merge.
- The check is textual. It does not see a comparison split across lines, a
  `Title` compared by pointer, or a boolean smuggled through a helper in a
  title package. It stops the cheap regression, not a determined one.
- Struct-update from Pulse couples every title's table to Pulse's field set:
  adding a field means choosing, for each title, a provenance for it. That is
  the intended cost.
