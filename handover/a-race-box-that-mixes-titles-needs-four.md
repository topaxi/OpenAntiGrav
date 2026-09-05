# A race box that mixes titles needs four rows off the title, not one flow

2026-09-05. The architecture half of the race-box investigation. The per-title
evidence is in [`docs/formats/race-setup.md`](../docs/formats/race-setup.md);
this thread is the delta against what this build already has.

Milestone: **M7 - Shell and polish**, which already carries "Menus and the
front end" as `[~]`.

## The frame this has to fit

`docs/architecture/menus.md` settled the boundary and nothing measured here
moves it:

> **The tree is ours. The presentation is the disc's.**

So this is explicitly *not* a proposal to reimplement Pulse's three-page flow.
Four of the five titles do agree on a three-page shape, and that agreement is
interesting evidence - it is not a mandate, because the argument in `menus.md`
was always about which rows a PC game with a monitor and a keyboard should
offer, and reading the disc does not change that.

What *is* the disc's, and is therefore in scope, is the **values** on those
rows - and they turn out to vary by title in ways this build currently
hardcodes.

## Open

### 1. SPEED CLASS is hardcoded and is wrong for Pure

Both `assets/ui/menu.toml` pages carry a literal
`values = ["venom", "flash", "rapier", "phantom"]`. **Pure authors five
classes** - it adds `Vector` - as a live `<Menu name="Class">` entry on its own
`Class Selection` screen. Confidence 94.

Cross-title mixing is a stated goal for this project, so a literal list here is
the same mistake `oag_title::MenuStrip` was created to fix: a constant that is
either wrong for one title or silently reused as if measured. This wants a
title axis. ADR-0022's bar for adding one - a *second* measured corpus - is
already cleared, because Pure and Pulse were each read off their own disc and
they disagree.

**Note this does not say Pulse has a fifth class.** See the Pure thread.

**DONE, 2026-09-05.** `oag_title::SpeedClasses` (`crates/title/src/speed.rs`)
is the axis, filled per title from each one's own **per-team**
`handlingstats.xml` - which is the file that decides what a class does, and is
not the global one: all three measured titles author five `<GlobalClass>`
rungs with `VECTOR` first, and only Pure backs one with per-team tuning.
Pulse 4, HD 4, Pure 5, 2048 `None` (unread, not empty). Both `menu.toml` rows
are now `values_from`.

**And the fifth rung is authored, measured, and still not offered.** Pure's
`VECTOR` has real tuning in every one of its race teams' files - so the union
genuinely carries five names when a Pure image is present - but nothing
downstream can *name* it: `oag_physics::SpeedClass` has four variants, whose
discriminants index fixed four-wide arrays for per-class weapon speeds and the
global speed-pad/gravity/weapon-pad tables, and whose `ALL` means "the rungs
every title has". `SpeedClasses::is_selectable` filters it out rather than let
a menu offer a class that would silently race on another class's tuning.
Making it selectable is a real change to two determinism-bound crates and is
its own piece of work - **not** the same question as
`handover/is-there-a-fifth-handling-class.md`, which is about *Pulse*.

### 2. AI DIFFICULTY differs in cardinality and in existence

| | levels | key |
| --- | --- | --- |
| ours | 4 (`novice`/`skilled`/`elite`/`ace`) | `ai.difficulty` |
| Pulse PSP, Pulse PS2, HD | 3 (`Easy`/`Medium`/`Hard`) | `global="SkillLevel"` |
| Pure | **none authored anywhere** | - |

The four *names* being ours is deliberate and documented
(`crates/ai/src/difficulty.rs`, per ADR-0006) and is not in question. The
*count* is a property of the release, the way SPEED CLASS's four are. Three
titles agreeing on three levels, with a fourth authoring none, is the shape of
a title axis rather than a constant.

Also worth carrying into any implementation: on all three titles that have it,
the widget is named `Difficulty` and the persisted key is `SkillLevel`. Code
keying off the widget name will not find the saved value.

### 3. Rows the disc has and we do not

- **WEAPONS on/off.** Authored on the single-player page by Pulse PSP, Pulse
  PS2 and HD. Pure has it only on its multiplayer create screens. We have no
  such row at all.
- **ELIMINATIONS, and it is a false friend.** Pulse and PS2 author
  5/10/15/20/25 - a *kill count*. HD authors an `aSlider` min 200 max 600 step
  20 - a *score* target - and it is disabled (see the `a`-prefix note in the
  HD/2048 thread). Same widget name, different quantity. **Do not unify them.**
- **NumberOfPlayers / SplitScreen** on HD, and the whole split-screen leg on
  PS2.

### 4. Laps: nobody offers it, and that is the finding

No title authors a lap-count row on its custom-race page. Four independent
measurements say so. Yet `laps="%d"` sits in Pulse's serialised race record and
HD's commit-and-launch handler reads a lap value. So laps is a real race
parameter set from the mode and the circuit, never chosen by the player.

A LAPS row would therefore be **ours**, and would have to be marked as ours the
way `menus.md` marks MONITOR.

### 5. Unlock gating: two axes, not one

Our TRACK row lists everything raceable. Pulse offers three circuits until a
grid is cleared. And craft variants gate on a *different* axis again - per-team
loyalty, not campaign progress. A single "unlocked" flag will not model both.

This is the row where "the contents come off the disc" and "the player has a
profile" meet, and there is no profile in this build yet. It may be right to
decide the race box simply ignores unlocks and offers everything - but that
should be a stated decision, not an omission.

**DONE, 2026-09-05 - the maintainer settled it and it is written down.** The
race box offers everything and gates on nothing, for now, because there is no
profile to gate against. It is a `## Unlocks` section in
`docs/architecture/menus.md`, framed as a deliberate divergence with the
reason, so nobody later "fixes" it back into a lock; the originals' two gate
axes are recorded there beside it, because the contrast is the argument.

### 6. The flow has to be re-entrant, not a wizard

On Pulse the same `Track Creation` and `Team Selection` screens serve **three**
callers - custom race, tournament, and the grid editor - discriminated only by
a `<Redirect>` keyed on another screen's current value
(`<Entry Item="Racebox->RBMode" equals="RB_EDIT_GRID" goto="Cell Setup">`).
HD does the same thing for its online leg, and for split screen it visits
`Team Selection` twice, once per player.

A fixed three-page wizard would not survive contact with any of that. Whatever
shape our race box takes, "who called me and where do I return" needs to be a
parameter.

### 7. What the `remix` page already gets right

`assets/ui/menu.toml`'s `remix` page already does title-scoped mixing - TRACK
TITLE / TRACK / CRAFT TITLE / TEAM / VARIANT, with the lower rows resupplied
when the title above changes. **That is the precedent to extend, not to
replace.** The items above are mostly about the rows it shares verbatim with
the ordinary `race` page, which are the hardcoded ones.

## Next Steps

1. Decide the one open question of principle first, because everything else
   follows from it: **does our race box gate on unlocks at all?** There is no
   profile in this build, so the honest default is "no, offer everything", and
   writing that down in `docs/architecture/menus.md` as a deliberate divergence
   costs ten minutes and unblocks the rest.
2. Move SPEED CLASS off its literal list onto a title axis, following
   `oag_title::MenuStrip`'s shape (`crates/title/src/menu.rs`) - a field on the
   title package, filled from each title's own measured data, with the
   authored/measured distinction its doc comment already makes. Pure's fifth
   class is the second corpus that licenses the axis.
3. Same for AI difficulty's *count*, keeping our four names.
4. Add the WEAPONS row - three titles author it, it is one `toggle` entry, and
   `oag_race` would need to carry it.
5. Leave previews to
   `handover/the-race-setup-previews-are-meshes-and-nothing.md`; they are
   independent of all of the above.
