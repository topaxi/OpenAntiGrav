# Pickups

What a `Weapon Pad` hands out, what a craft does with it, and - separately -
the free Turbo a solo event is given once a lap.

Implemented in [`oag_gameplay::pickup`](../../crates/gameplay/src/pickup.rs)
(the draw and the inventory),
[`oag_gameplay::projectile`](../../crates/gameplay/src/projectile.rs) (rockets in
the air and what they hit), `oag_game::race` (the trigger, the grant and
spending it), `oag_physics::engine` (what a fired Turbo does) and
`oag_physics::damage` (what a fired Shield refuses and what a blast costs). The pad
geometry is [pads.md](../formats/pads.md); the table is
[weapon-stats.md](../formats/weapon-stats.md); the runtime side of the pad is
[the Ghidra page](../ghidra/functions/psp-pulse-usa/pads.md).

## Read this first: what is recovered and what is ours

This subsystem is unusually mixed, so the split comes before anything else.

| Piece | Status | Confidence |
| --- | --- | --- |
| The pad trigger: swept test, distance cache, stamping `<WeaponPad refresh_time>` | **recovered** | 90 |
| `<Pickupodds>`: weights per class, per `ai`/`human`/`front`/`back` | **recovered** | 92 |
| `<WeaponPad refresh_time elimination_refresh_time>` | **recovered** | 90 |
| `SQUARE` fires, `CIRCLE` absorbs | **recovered** | 90 |
| `T += Engine.turbo`, uncapped, and where it lands in `Ship_UpdateEngine` | **recovered** | 84 |
| That flag bit `0x200` is the Turbo pickup's half of that gate | inferred from the pair with the barrel roll's `0x400` | 75 |
| `Ship_SetShield`'s clamp, which absorb pays through | **recovered** | 84 |
| Weapons off hides the pads and empties the trigger list | **recovered** | 88 |
| The free Turbo once a lap in a time trial and a speed lap | **recovered**, from two shipped records | 85 |
| **That a crossing grants anything at all** | **recovered 2026-08-17** - `WeaponPickup_Grant` (`0x08861d20`), called on the pad-crossing flag | 85 |
| **The weighted draw**: `rand() % total` then a cumulative walk over `<Pickupodds>` | **recovered 2026-08-17** | 92 |
| **The inventory's shape** - one slot holding a weapon id, `-1` for empty (`craft+0x1bc`) | **recovered 2026-08-17** | 88 |
| **Granting only into an empty slot** | **ours** | - |
| **Lap 1's free Turbo arrives at the countdown's release edge, not the start line** | **recovered 2026-09-07**, from a maintainer play-test on `pulse-psp-eu` | 85 |
| **The moment within a lap for laps 2..N** (crossing the finish line, or the start line a tick later) | unrecovered | - |
| `<Rocket>`: `damage`, `blastforce`, `blastradius`, `launchSpeed`, a speed per class | **recovered** | 92 |
| `Ship_Damage`'s `source == 2` being a weapon hit | **recovered** | 75 |
| `entity+0x1b8` is the fire-request word, one bit per weapon | **recovered** | 80 |
| **A Rocket fires three at once, at `-spread`, `0`, `+spread`** | **recovered** | 88 |
| `spread` is that fan's half-angle, in radians | **recovered** | 82 |
| **That a fired Shield refuses damage, and refuses it outright** | **ours** | - |
| **That a rocket flies straight at a constant speed** | **ours** | - |
| **A sphere for a hull, and full damage inside `blastradius` with no falloff** | **ours**, and the falloff half is now measured to be wrong | [missile.md](../ghidra/functions/psp-pulse-usa/missile.md) |
| **The launch offset, the flight speed being class + `launchSpeed`, the lifetime cap** | **ours** | - |
| `<Missile>`: `damage`, `blastforce`, `blastradius`, `launchSpeed`, a speed per class, **and its two lock distances** | **recovered** | 90 |
| **A Missile press puts exactly one in the air**, where a Rocket puts three | **recovered** | 90 |
| The lock: longitudinal window, `0.9` cone, along-track screen, nearest-by-distance | **recovered** | 88 |
| The guidance: a chord-clamped move-towards at `dt * 4.0`, applied a tick late | **recovered** | 95 |
| A missile's speed ramps from its launcher's own speed to the class speed over one second | **recovered** | 90 |
| A missile glances off walls up to five times where a rocket detonates on the first | **recovered** | 92 |
| `<Mine>`: `damage`, `blastforce`, `blastradius`, `absorb`, **`timetodie` and `trigger_radius`** | **recovered** | 92 |
| **A Mine press lays a cluster, one every `0.1 s`**, and the craft holds the pickup until the last is out | **recovered** | 90 |
| The cluster leaves from `craft+0xa0`, the **rear** anchor - the Bomb's too, and nothing else's | **recovered** | 90 |
| A mine's countdown is the authored `timetodie`, taken straight into the entity by `Mine_Init` | **recovered** | 92 |
| **How many mines a press lays** | **ours** | - |
| **That a mine does not move once laid**, and that `trigger_radius` is what sets it off | **ours** | - |
| **That a mine cannot be tripped by the craft that laid it**, at any range | **ours** | - |
| `<Bomb>`: the Mine's six, every one of them larger on both shipped tables | **recovered** | 92 |
| **A Bomb press lays exactly one**, where a Mine press lays a cluster | **recovered** | 88 |
| **That a bomb is static**, like a mine - its spawn direction is recovered and any speed is not | **ours** | - |
| **That a bomb's fuse is its own `timetodie`**, by analogy with `Mine_Init` | **ours** | - |
| **The blast's impulse falls off linearly** over `blastradius`; the damage does not | **recovered** | 82 |
| `<Plasma>`: the Rocket's block less `spread`, at measured offsets `+0x9c`..`+0xc4` | **recovered** | 92 |
| **A Plasma press puts exactly one in the air**, where a Rocket puts three | **recovered** | 88 |
| **A plasma bolt flies the Rocket's own floor-following path** - `Plasma_Update` is `Rocket_Update` | **recovered** | 85 |
| The bolt rides `WO_PLASMA_HEAD` and fires the `PLASMA` cue, both read out of `.rodata` | **recovered** | 90 |
| **A plasma bolt winds up for 1.0 s on the nose before it flies** - `Plasma_Init`'s `+0x4c`/`+0x50`, spent by `Plasmas_Update` | **recovered** | 90 |
| **A bolt is reaped at 10.0 s** by the pool walker, a hardcoded ceiling and not an authored `timetodie` | **recovered** | 90 |
| The bolt's detonation is `WO_PLASMA_FLASH`, off `Plasma_SpawnDetonation` in the pool teardown | **recovered** | 88 |
| **What `<Plasma charge_time>` does** - authored on three tables, and a calibrated sweep of both PSP executables says **nothing reads it** | **recovered (negative)** | 90 |
| `<Shuriken>`: ten of thirteen, at measured offsets `+0x144`..`+0x174` | **recovered** | 92 |
| **A Shuriken press throws exactly one blade**, at `±0.349066` rad - `20` degrees - on a coin | **recovered** | 88 |
| A blade carries the **throwing craft's own speed** on top of the class speed | **recovered** | 88 |
| A blade **reflects perfectly off walls** with no damping and no bounce budget | **recovered** | 88 |
| **What ends a blade** - the `fuse` is read, whether running out detonates or reaps is not | **ours** | - |
| **When `rhicochetdamage`/`rhicochetForce` are spent** - the only second pair any weapon authors | **unread** | - |
| **Not firing at all when nothing locks** | **ours** | - |

The three "ours" rows in the middle are not a gap anyone can close by looking
harder in the obvious place. `WeaponPads_TestCraft` (`0x0888727c`) stamps the
pad's refresh timer on a hit and **that is the whole of what it does** - no
pickup-grant call site has been found anywhere, and the craft-side flag word
that would hold the result (`*(entity+0x4c) + 0x1b8`, and `craft+0x1c0`) has
one bit identified out of fourteen. So the machinery *around* the grant is
recovered in detail and the grant itself is this project's reading of what a
weapon pad is for.

**Those three rows used to say "ours", and the reason was that no grant existed
to read.** `WeaponPads_TestCraft` (`0x0888727c`) stamps the pad's refresh timer
and that is the whole of what it does, so three separate passes concluded there
was no grant anywhere in the executable. There is: `Weapons_DispatchFire` calls
`WeaponPickup_Grant` (`0x08861d20`) on the pad-crossing flag at `craft+0x1c4`,
right after a `WEAPONPICKUP` cue, and it does a `rand() % total` cumulative walk
over `<Pickupodds>`. **Measured rather than inferred**, two independent ways: the
sub-parser `WeaponStats_ParsePickupOdds` (`0x0880e93c`) writes its weights to
exactly the offsets the grant walks, and `WeaponStats_Parse`'s tail sums those
same offsets into the total the grant divides by. Full layout on
[missile.md](../ghidra/functions/psp-pulse-usa/missile.md#by-catch-the-pickup-grant-and-it-is-pickupodds).

**Ported 2026-08-17.** `oag_gameplay::pickup::draw` now does what the original
does: the `ai` column flat for an opponent, and for the player a **blend** of
`human` with `front`/`back` by race position -

```text
weight = human + back * t + front * (1 - t),   t = (place - 1) / ship_count
```

`t` is `0` for the leader, so the leader gets `front` added and the tail gets
`back`. With the shipped Venom table - Shield `front="2" back="0"`, Turbo
`back="2" front="0"` - **a player in front draws more Shields and a player at the
back more Turbos. The pickup draw rubber-bands, and it rubber-bands the player
rather than the field.** The divisor is the whole field rather than `field - 1`,
which is the original's own arithmetic and means last place never quite reaches
the `back` column.

The **no-repeat rule** is ported with it: the grant compares each draw against
the craft's previous one and re-rolls rather than handing it over, which is what
the second copy of the held weapon id (`craft+0x1c0`) is for. `pickup::Held` grew
a `last` field, and that field is hashed - what a craft was last given decides
what it can be given next, so it is simulation state.

**Three things about the port are ours, and each is labelled where it lives.**
The *sequence* still cannot match the original, for the reason the next paragraph
gives. The retry behind the no-repeat rule is **bounded** where the original's
loop is not: `IMPLEMENTED` holds eleven weapons, so a table weighting only one of
them would spin for ever, and after `pickup::REDRAW_ATTEMPTS` a repeat is
accepted - about one grant in twenty thousand on the shipped odds. And an
*unplaced* craft spends the `human` column alone, because the original always has
a place to blend against.

**The PRNG bounds what can ever be checked.** The original's generator is an
open question on the [roadmap](../overview/roadmap.md), so the *sequence* of
draws cannot match the original even with a byte-exact algorithm. Only the
*distribution* is testable, and only against the authored weights. Every draw
goes through `World::rng` so that a replay of one of our own races is
reproducible - which is the guarantee that survives.

## Which modes hand out what

Three different things happen, and the disc distinguishes them in two
independent files.

| Mode | Weapon pads | Free Turbo |
| --- | --- | --- |
| Time trial | hidden, trigger list empty | **once per lap** |
| Speed lap | hidden, trigger list empty | **once per lap** |
| Zone | hidden, trigger list empty | no |
| Single race | armed | no |

The first column is `Race_ReadSetupOptions` (`0x08896b84`) and
`World_CollectNodeLists` (`0x088879d4`): a weapons-off race does not ignore a
crossing, it clears each pad node's visibility bits and zeroes the trigger
list's own count. `oag_race::Mode::weapons_enabled` is the port, and
`oag_game::race::Race::start` drops the volumes rather than testing the mode per
tick, which is the same layer the original decides it at.

### The free Turbo, and how the second record was found

`MSC_EVENT_TT` and `MSC_EVENT_SL` both read *"You will be given a free turbo
pickup once per lap"*. That is a shipped string and it was known; what was
missing was any second source, and a mode whose pads are hidden getting a
pickup from nowhere is exactly the kind of claim a manual makes loosely.

**The HUD layouts settle it.** Each of the four authors a different pickup set:

| Layout | Pickup widgets |
| --- | --- |
| `Arcade_HUD.xml` | the backdrop and all thirteen icons |
| `Elimination_HUD.xml` | the same |
| `TimeTrial_HUD.xml` | the backdrop and **`TurboIcon` alone** |
| `Zone_HUD.xml` | none |

A layout carrying exactly one weapon icon, for a mode with no armed pads, is
otherwise inexplicable - and it is the same weapon the event text names. A
string table and a HUD layout agreeing is two independent records of one rule,
the same shape of evidence that settled weapons-off energy recovery in
`oag_gameplay::damage_rules`. Confidence **85**.

This was not found by looking for it. `every_weapon_has_an_icon_widget_named_after_it`
was written asserting that the weapons-off layouts author *no* pickup widgets,
and failed. **The assumption it was written to confirm came from a grep whose
key was wrong** - these files are [shortened XML](../formats/fexml.md) with a
per-file dictionary, so `name` is `b=` in `Arcade_HUD.xml` and `c=` in
`TimeTrial_HUD.xml`, and grepping the first file's key against the second
returns nothing with exit code 0. Expand before grepping, or use the parser.

**Lap 1 gets one too - reported 2026-08-19 as silently missing, since
`grant_free_turbo` originally hung only off `oag_race::Outcome::lap_completed`
and lap 1 never crosses that edge on its way in.** The fix landed at the time
called `Race::start` directly, at tick 0, on the reasoning that lap 1 is a lap
too and should get its turbo "at its own start instead of its own end". **That
reasoning was wrong, caught 2026-09-07 by a maintainer-requested side-by-side
against the original**: a green `TurboIcon` hexagon sat over the start gantry
in every Time Trial screenshot with no counterpart in the original, and the
maintainer confirmed directly on `pulse-psp-eu` (our own render target) that
the free Turbo appears only *after* the countdown releases the craft - the
original never holds anything in the pickup slot through the countdown at
all. Confidence 85 (a single, direct play-test on the render-target disc, not
yet corroborated by a second binary or a decoded record). Fixed by moving the
grant from `Race::start` to `Race::tick`, firing once at the tick
`oag_race::state::RaceState::thrust_gated` first reads `false` (`self.world.tick
== oag_race::state::COUNTDOWN_TICKS`) - the same instant the HUD's countdown
board itself releases. Both calls (this one and `lap_completed`'s) share
every other gate: mode, an empty slot, a loaded weapon table. **What is still
not recovered is the moment within a lap for laps 2..N**: "once per lap"
constrains the count, not whether the original grants it crossing the finish
line or at the start line a tick later (the same edge, either description) -
nothing read pins which.

## The icon is found by name, not by an id

[hud.md](../ui/hud.md) recorded the pickup widgets as "14 `*Icon` widgets" whose
"icon ids are numeric", with no id-to-weapon mapping known - which made drawing
the right icon look like it needed something unrecovered.

It does not. `Arcade_HUD.xml` authors **thirteen**, and names each after its
weapon's own `type` string: `TurboIcon`, `ShieldIcon`, `RocketIcon`,
`LeachBeamIcon` and `RepulserIcon` misspellings included. That is exactly
`oag_tables::weapons::Weapon::ALL`, so the lookup is
`format!("{}Icon", weapon.as_type())` and nothing else -
`oag_game::hud::pickup_icon_name`. Pinned against the shipped file for all
thirteen by `every_weapon_has_an_icon_widget_named_after_it`.

The numeric ids are still real - `0x0883b3b8` forces one, and it is `6` - they
are simply not needed to draw the right icon.

### Drawing both widgets as authored gives an opaque white hexagon

Found by looking at it, after the first version of this shipped. **It is not a
bug in the reader; it is what the shipped data says**, and both halves are
measured:

- `PickupBackground` samples a **filled hexagon whose alpha is 255** on 2,424 of
  its 2,492 opaque pixels; `TurboIcon` samples a glyph in the same white with
  antialiased edges. Both are pure white masks, so all the colour is meant to
  come from the tint.
- Both are authored `Color="FEConst->HudColour1"`, and that constant is
  `0xFFFFFFFF` - **opaque white**.

So an opaque white hexagon is drawn and an opaque white glyph is drawn on top of
it, and the second is invisible against the first. The original does not look
like that, so **it must set at least one of the two colours at runtime**, and
that is unrecovered. `0x0883b3b8` is the known place the runtime reaches into
these widgets - it forces the icon id - and is where to look.

**What this build did until 2026-09-04 was a substitution, not a recovery.**
The backdrop was drawn in `HudBGColour`, the only background colour the layout
defines (`0x40000000`, a quarter-alpha black), and the icon kept its authored
white. Every value still came off the player's own disc; what was ours was the
choice of which constant. The precedent is the front end's title colour,
substituted the same way while the widget behind it is unbuilt.

Two ground-truth assertions keep the fallback honest, both against the shipped
file: that the backdrop and the icon really are authored in one colour - if
that ever stopped being true the substitution would no longer be needed - and
that the layout really defines `HudBGColour`, without which the substitution
would silently not happen and the icon would go back to being invisible. Both
still hold and both still run; the fallback is what `Bomb`/`Mine` draw today,
per the next section.

### The backdrop is colour-coded per weapon, and a reference frame settled it

**2026-09-04.** The reference frame this page asked for above has now been
taken - a PSP under Xvfb (`docs/reverse-engineering/ppsspp-debugger.md`)
driven onto Talon's Junction's own weapon pads and, once, sat on its Time
Trial start line, screenshotted while holding what it picked up. Four frames:
`ShieldIcon` and `AutopilotIcon` came up on a green hexagon, `MissileIcon` on
a magenta one, and `TurboIcon` - Time Trial's own free-pickup grant, the one
weapon a placement cannot be aimed at - on a third, brighter-reading green shot
against open sky rather than a tunnel interior.

**That is a category, not thirteen independent colours, and the grouping for
the other nine weapons comes from a second, independent source.**
`Data\Plugins\loading\Definition.xml` points every weapon's own *loading
screen* tip at `Data\Defaults\Loading\Pulse\<Name>.mip`, and each of those
authors the same hexagon-and-glyph picture the HUD does. Sampling all thirteen
files turns up exactly three flat fills: `Bomb`/`Mine` at `(0, 0, 255)`, eight
more (`Rocket`, `Missile`, `Quake`, `Cannon`, `Plasma`, `LeachBeam`,
`Repulser`, `Shuriken`) at `(250, 1, 189)`, and `AutoPilot`/`Shield`/`Turbo` at
`(0, 255, 0)` - the same three-way split the in-race frames show for the four
weapons they cover.

**The `Turbo` frame reads differently from the other two green ones, and why
is not established.** Shot against bright sky rather than a dark tunnel, its
hexagon carries a real blue channel where `Shield`'s and `Autopilot`'s read
near zero. Two explanations were checked and ruled out: the same frame's
`TimeIcon`, a widget authored `HudColour2` with no substitution, samples
within antialiasing noise of that constant's own declared value, so this is
not the whole HUD reading brighter that frame; and `TimeTrial_HUD.xml` (this
frame's own layout) authors `PickupBackground`/`TurboIcon` identically to
`Arcade_HUD.xml`'s, checked directly. Two explanations remain open and neither
is confirmed: the backdrop blends with the scene behind it, which would read
brighter over sky than over a tunnel exactly this way; or `Turbo`'s own
runtime colour genuinely differs from `Shield`'s and `Autopilot`'s despite the
loading screen filing all three under one authored fill, since that fill
being independently authored art settles the *category* rather than that
every member is byte-identical at runtime. **The loading screen's own fill
running consistently brighter than every in-race frame** (Missile's red and
blue, Shield's green, each roughly half) is consistent with either
explanation too, not a tiebreaker.

`oag_pulse::hud::PICKUP_COLOURS` draws each of the eleven weapons the loading
screen groups opaque, in the colour `Shield` and `Autopilot` agree on rather
than an average that would let the `Turbo` anomaly quietly move the number -
not a claim that the original draws it opaque, but the simplest thing this
build can draw without picking between two unconfirmed explanations. `Bomb`
and `Mine` have no frame at all, so they still draw the fallback above.
Confidence **70** for the green category (two agreeing frames), **60** for
pink (one frame, `Missile`, standing for all eight members its loading-screen
category carries). See the constant's own doc comment for the numbers and the
full account, including an earlier version of this table that extrapolated a
`Bomb`/`Mine` colour from the loading screen's brightness ratio and was
retracted the same day once that ratio turned out not to settle even the
`Turbo` question it was trying to explain.

**Open**: a weapon pad placement that comes up `Bomb` or `Mine`; a second
frame of any pink weapon besides `Missile` to check against it; and, the one
that would resolve the `Turbo` question either way, a capture harness that can
read the pixel actually occluded by the hexagon rather than estimate it from
a screen-space neighbour.

The forced id `6` was briefly read as evidence that the `1.2` pickup is the
Turbo. **It is not**: `6` lands on `Shield` or `Autopilot` depending on where
the class-name pool starts counting, and the turbo turned out to be a different
term entirely - see below.

### Pure does the same thing with a different widget kind, not a different answer

**2026-09-04.** Pure's weapon icons are not `<Image>` sprites at all -
`Arcade_HUD.xml` and `TimeTrial_HUD.xml` (`Data.wad`, both byte-identical on
this point) draw each one as a `<Mode3D><Model>`, and ten of them carry a
`colour="0xAARRGGBB"` attribute directly, e.g. `<Model name="TURBO_icon">
<Values Src="Data\HUD\Weapon_turbo.vex" colour="0xff40ff40" .../></Model>`.
Confidence **95**: read straight off the disc, no inference, no reference
frame needed the way Pulse's fill required one - a colour written in the XML
answers the question Pulse's authored-white backdrop leaves for a runtime
substitution table to answer instead.

The same four-colour split Pulse's own reference frames found shows up again,
independently: green (`0xff40ff40`) for `Turbo`/`Shield`/`Autopilot`, one blue
(`0xff40acff`) for `Rocket`/`Missile`/`Disruptor`, a second, purer blue
(`0xff0000ff`) for `Quake`/`Plasma`, and orange (`0xffffc040`) for
`Mine`/`Bomb` - four categories, not ten independent colours, the same shape
as Pulse's pink/green/fallback grouping even though neither the colours nor
the exact membership match weapon-for-weapon.

**`Disruptor` is a weapon Pure's roster has that Pulse's does not.** Pure's
own executable strings carry `WO_DISRUPTOR_EXPLO` alongside every other
weapon's explosion effect name, so this is not a second spelling of `Cannon` -
`Cannon` has no icon in either layout, and Pure's ten omit `LeachBeam`,
`Repulser` and `Shuriken` from Pulse's thirteen as well. `Weapon`
(`crates/tables/src/weapons.rs`) is scoped to Pulse's own weapon table, so a
per-title roster difference like this is expected rather than a gap to
reconcile by guessing a mapping - see `crates/pure/src/hud.rs`'s module doc
for the full accounting.

**This corrects a claim made and merged the same day**: `oag_pure::hud::ART`
briefly stated Pure "has no per-weapon icon widget of any kind to colour",
concluded from grepping `<Image name=` alone and never checking for `<Model
name=` inside a `<Mode3D>` block. The retraction and the corrected reading are
both in `crates/pure/src/hud.rs` now.

### Pure's icons draw, and the size comes from the mesh rather than a guess

**2026-09-04, same day.** `oag_title::HudArt::pickup_icon_models` names the
`<WEAPON>_icon` widget for nine of the nine weapons `oag_gameplay::
pickup::IMPLEMENTED` can hand out on Pure, plus `Quake` for when it joins that
pool; `pickup_icon_backdrop_model` names `weapon_icon_grid`, the frame they
sit in. `oag_game::hud::draw::pickup_model_draws` reads the held weapon's
model, tints it with its own authored `colour` (Pure's model widgets carry
one, so there is nothing to substitute the way Pulse's backdrop needs), and
draws it centred on the widget's authored position.

**The one thing that could not be measured from the XML alone was each
mesh's own size**, and it is not a hand-measured constant the way the sight
brackets' `SIGHT_SIZE` is: `oag_game::race::hud::vex_model_art` reads it off
the model's own vertex positions - the widest span on `x` and `y` across
every vertex - once, when the model decodes. The ten weapon-icon meshes are
not one uniform size the way the sight brackets are: read off
`pure-psp-usa.chd`, they run from `Weapon_turbo.vex`'s `31.4x6.5` units to
`Weapon_quake.vex`'s `56.6x11.7`, so a single shared constant would have been
wrong for at least eight of them.

**`Shuriken` is the gap this makes visible rather than papers over.** It is
in `IMPLEMENTED` and has no icon on Pure's disc - the title predates the
weapon - so a Pure race that hands one out draws the backdrop grid and no
icon inside it, which is what `crates/game/tests/pickup_icon_ground_truth.rs`
and `oag_game::hud::pickup_model_tests` both pin rather than leave to chance.

Verified two ways: a ground-truth test against `pure-psp-usa.chd` confirms
all nine implemented icons plus the grid decode with a real quad extent, and
`cargo run -p oag-game -- <pure image> --race --mode single_race --give
turbo --screenshot out.png` shows a green `TURBO_icon` in its own backdrop -
the first time this build has drawn any part of Pure's pickup on screen.

## Only what has an effect is handed out

`oag_gameplay::pickup::IMPLEMENTED` is the pool a pad draws from, and it holds
**Turbo, Shield, Rocket, Missile, Autopilot, Mine, Bomb, Plasma, Shuriken, the
Cannon and, later the same day, the Quake**. **Being on this list is necessary
but not sufficient** - see
"Shuriken and Repulser are gated by mode, not by the pool" below for the two
weapons the *pool* would hand out and the authored *odds* never do.

- ~~**Missile** needs the lock distances its own `<Stats>` authors, and a target
  worth locking - which needs the AI.~~ **Built 2026-08-17**, and what unblocked
  it was not the AI but a reading: the lock is recovered whole from
  `Ship_AcquireLock` (`0x08844784`) and the guidance from `Missile_Update`
  (`0x0885a918`). See
  [missile.md](../ghidra/functions/psp-pulse-usa/missile.md).
- ~~**Bomb** needs the Mine first.~~ **Built 2026-08-26**, the same day, and it
  is the cheapest weapon this project has added: one bigger charge out of the
  same rear anchor, sharing every line of `oag_gameplay::projectile::mine`
  except a count.
- ~~**Mine** needs somewhere to sit and something to trip it.~~ **Built
  2026-08-26**, and the thing that unblocked it was somebody else's mistake:
  the fire handler had been attributed to the Cannon and the Bomb's to the Mine,
  because `Weapon_RequestFire`'s jump table holds those two entries out of
  address order. See [mine.md](../ghidra/functions/psp-pulse-usa/mine.md).
- ~~**Plasma** needs a fire path nothing has read.~~ **Built 2026-09-02**, and
  it is the cheapest weapon since the Bomb for the mirror-image reason: the
  Bomb reused the Mine's whole module and the Plasma reuses the *Rocket's*
  whole flight model. `Plasma_Update` (`0x0885c6cc`) is `Rocket_Update`'s floor
  follower instruction for instruction, and `Weapon_FirePlasma` (`0x0886a868`)
  spawns exactly one where the Rocket spawns three. See
  [plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md).
- ~~**Shuriken** needs a ricochet and a fuse.~~ **Built 2026-09-02**, and the
  estimate that said otherwise is the finding. Judged from its constructor and
  its bounce alone it looked like a session's work; `Shuriken_Update`
  (`0x08877bdc`) then turned out to be the *same floor follower* the Rocket and
  the Plasma share, differing only in that a wall bounces a blade instead of
  ending it. One blade, thrown at a coin-flipped **±20 degrees**, carrying the
  throwing craft's own speed, reflecting perfectly off walls until its authored
  `fuse` runs out. See
  [shuriken.md](../ghidra/functions/psp-pulse-usa/shuriken.md).
- ~~**Cannon** needs a fire path nothing has read.~~ **Built 2026-09-07**, the
  same day its evidence page was written, and it is the odd one out among
  every weapon built here: it does not fire through
  `Weapon_RequestFire`'s bit system at all. `Cannon_UpdateReload`
  (`0x0883f424`) advances a per-craft countdown on every frame the **fire
  button is held**, and it is *that* countdown - reloaded from the Cannon's
  own authored `rate` - that periodically arms the bit `Weapon_FireCannon`
  (`0x088577ac`) reads. So holding fire gives auto-repeat at the authored rate
  and tapping gives a few frames of countdown per tap, twin barrels
  alternating by the low bit of its own remaining `rounds`, until the magazine
  runs out - `oag_gameplay::pickup::Held::advance_cannon_reload` is the port,
  called from `Race::advance_cannons` for the player alone, on the ticks the
  button is down.

  **Corrected 2026-09-07, after a play report.** The first build of this
  weapon had it self-firing with no press, from reading the countdown's own
  gate - `*(*(entity+0x94)+0x78) + 0x16` - as a *track weapon-pad* flag. It is
  the craft's control record, the block the binary registers under the string
  `player_input`, and `+0x16` is the held state of `OPT_CTRL_FIRE`. Nothing in
  the image writes that byte except the human pad's own update, which is why
  the port fires this weapon for slot 0 only. Each round carries the firing craft's
  own current speed rather than a class figure - the same "carry the
  shooter's speed" shape the Shuriken's throw already has - plus a base this
  engine had to invent: the Cannon's `<Stats>` authors no speed at all, and
  the per-class base `Cannon_Init` adds to it (`func_0x00060af4`,
  `0x08864af4`) was not decompiled this pass. See `oag_gameplay::
  projectile::cannon::BASE_SPEED_KMH`'s own doc comment, marked chosen and
  not measured. Damage is direct-hit only rather than a blast, and that is
  the schema's own shape: the Cannon is the only projectile weapon whose
  block authors neither `blastforce` nor `blastradius`, so a round that hits
  a wall costs nobody anything and one that hits a craft costs that craft
  alone, through `oag_gameplay::projectile::cannon::direct_hit`. See
  [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
- ~~**Quake** needs its per-frame travel and its hit latch, neither
  located.~~ **Built 2026-09-07**, the same day the latch was found inside a
  function this page already had part of the reading of - the "wave has
  reached me" test (`entity+0x860 & 0x40`) sits a dozen lines above the
  damage branch `cannon-quake-leachbeam.md` had already quoted. **This engine
  draws no track deformation** - and note the reason changed on 2026-09-08:
  the original *does* deform the road (`Quake_UpdateSpan`, `0x0891cab8`,
  rewrites the road mesh's own packed vertex positions under a `vcos_q`
  profile), but the deformation is decorative and the damage path needs none
  of it. The earlier "the Quake is not a track deformation" reading was drawn
  from the three functions that pass it by; see that page's own 2026-09-08
  correction. A single travelling instance
  (`oag_gameplay::projectile::quake::Wave`) advances at a fixed, unauthored
  `270.0` units a second along the course, tracked as a plain distance-along
  rather than the original's segment-plus-parametric-`t` pair, and its hit on
  a craft reuses the same generic pending-damage channel the Missile and
  Mine/Bomb already share - gated on the Quake's own authored `radius`, the
  one attribute with no other consumer anywhere in the fire chain. The
  visual is the disc's own `WO_QUAKE`, re-positioned and re-scaled every tick
  to the midpoint and width of the track's own two edges nearest the wave -
  orientation is not established by anything read, so it draws
  axis-aligned, chosen rather than measured. **The wave lives 5.0 seconds from
  launch and is then dropped** (`oag_gameplay::projectile::quake::LIFETIME_SECONDS`,
  recovered 2026-09-08 at confidence 85), which is also what re-opens the
  once-at-a-time guard so a second Quake can be fired in the same race. Before
  that landed the wave circled the ring forever and a player got exactly one
  Quake a race. See
  [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
- ~~**The LeachBeam is read but not built** (2026-09-07)~~ **Built
  2026-09-08**, and what unblocked it was three reads the 2026-09-07 pass had
  left open, one of which was blocked on a wrong address: the page's two
  drain-rate functions were rebased `0x4000` low, exactly as its own Cannon
  candidate had been. Corrected, they are `LeachBeam_DrainRate`
  (`0x08872edc`) and `LeachBeam_RepairRate` (`0x08872f18`), each returning an
  authored per-tick rate times a **one-shot** `energy_multiplier` held on its
  own instance byte - so the first draining tick moves fifty times what every
  tick after it does. The two accumulators' consumers, which that pass called
  "the single largest remaining gap", are `Ship_ApplyPendingWeaponDamage`
  (`0x0883f13c`) for the victim - the ordinary `Ship_Damage` path a fired
  Shield swallows - and `Ship_ApplyPendingWeaponRepair` (`0x0883f228`) for the
  shooter, which is `Ship_AddShield` straight into its own pool. The lifetime
  is the authored `active_time`, from `LeachBeam_Advance` (`0x08873fa0`)
  returning `age < active_time`. Target selection is the Missile's lock
  outright, and `FUN_0883f540` confirms that directly by calling
  `Ship_AcquireLock` for held weapon ids `1` and `10` and no others.
  `oag_gameplay::projectile::leach_beam` holds the single link a race ever
  has - a whole-race pool cursor, the strictest gate any weapon here has -
  and `oag_tables::weapons::LeachBeamStats` now decodes all nine attributes.
  **One half is deliberately not wired**: `slowShipFactor` lands on
  `craft+0x31c`, the one-shot thrust scale `oag_physics::engine` documents and
  does not implement, so a craft under a beam is not yet throttled. See
  [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
  The **Repulser** stays the one true "field the
  craft *is in* rather than a projectile" - its handler copies four of its
  own `<Stats>` onto the firing craft before it spawns anything - and is
  still deferred as Eliminator-only, per `HANDOVER.md`.

- **Autopilot** is the AI's own controller taking over: `Ai_Construct`
  (`0x088536bc`) names the local player's input source the literal
  `"autopilot input"`. It is AI work wearing a pickup's clothes, not pickup
  work.

**This is a departure and a deliberate one.** The authored table weights
thirteen weapons and the draw sees eleven of them, so what a player gets is the
authored distribution *conditioned on* the implemented set. It narrows to
nothing as weapons land - adding a variant to `IMPLEMENTED` is the whole change
- and it beats handing out a mine that cannot be dropped.

### Shuriken and Repulser are gated by mode, not by the pool

**Recovered, confidence 84.** `Data\XML\WeaponStats_Race.xml`'s four
`<Pickupodds class="...">` blocks - `Venom`, `Flash`, `Rapier` and `Phantom`,
checked all four - author `ai="0" back="0" front="0" human="0"` for both
`Shuriken` and `Repulser`. `Data\XML\WeaponStats_Elimination.xml` authors the
same two weapons nonzero instead - `Shuriken` at `ai="10" human="10"`,
`Repulser` at `ai="4" human="8"` - and it is a mode split rather than a
per-class one for a stronger reason than "checked all four and they match":
**diffing the four `<Pickupodds>` blocks against each other, block by block,
inside either file, shows the *entire* block is byte-identical across
`Venom`/`Flash`/`Rapier`/`Phantom` bar the `class` attribute itself** - not
only these two weapons, all thirteen. Neither shipped PSP table varies its
pickup odds by speed class at all today; the `ai`/`human`/`front`/`back` split
is the only one doing any work, and it does the same work under every class
name. (The Shuriken/Repulser gate also runs in reverse, which is worth having
in one place: `Autopilot`, `Shield` and `Turbo` are the ones zeroed in
`WeaponStats_Elimination.xml` instead, so the split is not one-directional
toward Eliminator.)

Measured directly off both files on `pulse-psp-usa.chd`; `pulse-psp-eu.chd`
carries byte-identical copies of both files end to end, which is this
reading's corroboration - not yet checked against the PS2 disc or Wipeout
HD's own copy. Held at the rubric's data-read ceiling rather than the
schema's own 92 (`<Pickupodds>` in [weapon-stats.md](../formats/weapon-stats.md)):
two regions agreeing is one title's authoring pass confirmed twice, not an
arithmetic invariant across many independent files, and this is a values
read rather than a traced call site.

**The code does not need a fix for this - it already reads correctly.**
`pickup::draw_once` skips any weapon whose weight is `<= 0.0`, and
`oag_tables::weapons::{RACE_ENTRY, ELIMINATION_ENTRY}` already name both
files. What needs saying is that `IMPLEMENTED`'s list is misleading read
alone: `Shuriken` is on it, and `SingleRace` - the only mode with weapon pads
armed at all, per the table above - loads `RACE_ENTRY`, where `Shuriken`'s
weight is zero in every class. So a `SingleRace` inventory slot can hold a
Shuriken (it can be thrown once granted by a test or a script setting it
directly - see `crates/game/tests/shuriken_ground_truth.rs`) but a weapon pad
in this build can never *hand out* one: not because the pool excludes it, but
because the odds do. **`Shuriken`'s presence in `IMPLEMENTED` was correct-but-unreachable until
2026-09-08, when `Mode::Eliminator` landed and closed the gap this section
used to describe as open.** `oag_game::race::load_weapons` now takes the mode
and opens `title.weapons.elimination` for Eliminator specifically, falling
back to `race` (with a report line) for a title that ships no second table -
see [race-modes.md](race-modes.md#eliminator). A `SingleRace` weapon pad
still cannot hand out a Shuriken - `RACE_ENTRY`'s odds are still zero there,
unchanged - but an Eliminator one now can, through the table this section
already measured. `Repulser` is still unreachable, for its own, independent
reason: it is absent from `IMPLEMENTED` itself (see the list above), because
no craft-state field a weapon can attach to yet exists for the "field the
craft is in" mechanic its handler needs. Landing `Repulser` is a
`crates/gameplay` question now, not a mode-existence one.

`crates/game/tests/shuriken_ground_truth.rs`'s own ground-truth test moved
onto `Mode::Eliminator` the same day, for the reason above - it is the mode
the weapon is actually reachable in, and its own single remaining red
(a thrown blade detonating on a nearby grid-mate rather than flying free, see
that file's own doc comment) turned out to be a fixture confound - a straggler
in the blade's path, unrelated to the mode - fixed by disabling the rest of
the grid before the throw, not by anything about Eliminator's own mechanics.

## What a fired Turbo does

**`T += Engine.turbo`, uncapped.** From `Ship_UpdateEngine` (`0x0884c5c8`), read
out of the decompiler on 2026-08-11:

```c
if ((flags & 0x200) || (flags & 0x400)) {
    if (craft+0x2a4 == 1) {
        T += Engine.turbo;                        // after the cap, before the * 2.0
        if (controls.buttons & 1)  lift = g_boost_lift * T;
    }
}
```

The add sits **after** the `0.5 * speed + accelcap` clamp and before the fixed
doubling, so it is not clipped by the acceleration cap - which is what makes a
turbo a turbo rather than a nudge. `ShipState::turbo_timer` is the gate and
`oag_physics::engine::engine` is the port.

**Two bits turn one term on, and that is the evidence for reading `0x200` as the
pickup.** `0x400` is held while the barrel roll's timer runs
([input-bindings](../ghidra/functions/psp-pulse-usa/input-bindings.md),
confidence 80), and `<Special>` authors `roll_turbotime` beside it - so a roll
grants a *turbo*, and the term has two sources. `0x200` is the other one.
**Its writer has not been found**, so this is an inference from the pair rather
than a traced path: confidence **75**.

Not reproduced, both stated where they live: the `craft+0x2a4 == 1` gate, whose
enum nothing decodes, and the **boost lift** along body up under the thrust
button, whose `g_boost_lift` global was never read.

### The `1.2` multiplier is a different pickup, and wiring it here was wrong

Worth keeping because it is an easy mistake and it shipped for a few hours.
`craft+0x2a0 = 1.2` under flag `0x0004` is real and recovered
(`oag_physics::engine::ENGINE_PICKUP_SPEEDUP`), and it is **not** the turbo:

- It multiplies a thrust that has *already been clamped*. On Venom the cap is
  `0.5 * speed + accelcap` with `accelcap` in the tens, so `1.2` buys a few
  units of force; `<Engine turbo>` is authored an order of magnitude above the
  same class's `accelcap`. Measured end to end on the disc's own tunables, a
  fired turbo leaves the craft at **222 units/s against a control's 52** after
  one second - the multiplier version was imperceptible, which is exactly how
  it was reported.
- The branch that arms it lives in the **HUD update** (`0x0883b3b8`), not the
  craft update, and it also forces HUD icon id `6` and drives a fill from the
  pickup's own `+0x148` timer. So `0x800` is some timed effect the HUD draws a
  bar for; **which pickup it is remains unidentified**, since id `6` lands on
  `Shield` or `Autopilot` depending on where the class-name pool starts
  counting, and neither is obviously a speed effect.

**The lesson for the tests**, written down because the unit tests all passed:
asserting the *ratio* between a boosted and an unboosted tick cannot tell a
turbo from a nudge, because a ratio is a ratio. `a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race`
now compares speed after a second against a control on the shipped numbers,
which is a magnitude and would have caught it.

**The timer itself is ours.** The original holds a bit; this holds seconds,
because neither `craft+0x1c0`'s writer nor the pickup word behind it has been
read. Its duration is the file's own `<Weapon type="Turbo"><Stats time>` and its
magnitude the class's own `<Engine turbo>`, so both numbers are real. The field
is hashed by the determinism gate, which is why it lives on `ShipState` rather
than beside the inventory.

## What a fired Shield does

**It refuses damage outright for `<Weapon type="Shield"><Stats time>` seconds**,
through `oag_physics::ShipState::shield_pickup_timer` and an early return in
`damage::apply_contact`.

**Only the duration is recovered.** `WeaponStats_ParseShield` (`0x0880ca2c`)
reads an `absorb` and a `time` into `craft`-relative `+0x8c`/`+0x90`, and
**no reader of those offsets has been found** - nor any branch in `Ship_Damage`
(`0x088439ac`) that a shield would have to take. So *that* it refuses damage is
this project's reading of what a shield is for, in the same sense that a weapon
pad granting anything at all is. Refusing rather than *reducing* is part of that
reading: the original may well scale, nothing says either way, and refusing is
the choice that cannot half-work.

Two consequences worth having in one place:

- **It refuses on the same edge a destroyed craft does**, before the
  subtraction, so it also suppresses `Shield::depleted` and
  `crossed_critical`. A shielded craft cannot be destroyed and does not cry
  energy-critical. That means a mode ending on `depleted` - Zone's unbuilt end
  condition - could be held open by a pickup. **Not reachable today**, and worth
  saying so rather than implying a live hazard: Zone races with weapons off, so
  its pads are unarmed and it is excluded from the free Turbo, and a craft in
  Zone can never be holding a Shield. It is a constraint on whoever builds that
  end condition, not a bug in this one.
- **It does not refuse momentum.** A rocket's `blastforce` still shoves a
  shielded craft off the racing line. Extending the shield to stop that would
  be a second invented rule stacked on the first.

The timer runs one tick longer than the arithmetic, exactly as the Turbo's does
and for the same reason - it is read before it is decremented. Pinned by
`a_fired_shield_refuses_damage_for_its_authored_duration` rather than tolerated.

## What a fired Rocket does

**One projectile, straight ahead, at the class's own authored speed.** Built in
[`oag_gameplay::projectile`](../../crates/gameplay/src/projectile.rs); the
damage lands through `oag_physics::damage::apply_weapon`, which is
`Ship_Damage`'s recovered body - the state gate, the weapons-off halving, the
clamp and the destroyed transition - sharing one function with the contact path
so the two cannot drift.

**Recovered:** the numbers. `<Weapon type="Rocket"><Stats>` authors `damage`,
`blastforce`, `blastradius`, `launchSpeed` and a *separate flight speed per
speed class* (`venomspeed`/`flashspeed`/`rapierspeed`/`phantomspeed`), measured
at 800/900/1000/1100 on the shipped race table and a rung lower on the
Eliminator one. Also recovered, at confidence 75: `Ship_Damage`'s `source`
argument, whose value 2 is a weapon hit.

**Ours, and unavoidably so** - no firing call site, no projectile class and no
flight update has been found anywhere in the executable:

- straight-line flight at constant speed, with no gravity and no steering;
- the launch point (the hull's own forward extent, so a shot starts outside the
  craft that fired it) and the flight speed being the class's plus `launchSpeed`
  rather than one or the other, and the craft's own velocity *not* inherited;
- a sphere of half the largest `<Misc>` dimension for a hull, which matches the
  box nose-to-tail and is *wider* than it on the other two axes - so a near miss
  can register as a hit, which favours the shooter and is the generous reading
  rather than the conservative one;
- **full damage everywhere inside `blastradius`, with no falloff**, and the
  firing craft not excluded from its own blast - the falloff half of which is
  now known to be wrong and is not yet fixed: `FUN_08868ea4` adds
  `direction * (1 - distance/blastradius) * blastforce`, a **linear** falloff on
  the impulse, and reaches no damage at all (see
  [missile.md](../ghidra/functions/psp-pulse-usa/missile.md#the-blast-and-what-does-not-reach-it));
- `blastforce` applied as an impulse rather than a force held over a duration;
- a ten-second lifetime cap, so a rocket that leaves the world through a gap in
  the collision soup cannot hold its slot for the race.

### Three at once, fanned by `spread` - and this one is recovered

`Weapon_FireRocket` (`0x0886e104`) spawns one rocket through the craft's own
matrix, one through it rotated by `+spread`, and one by `-spread`: **three
literal calls to one spawn helper in a single invocation, with no timer between
them.** They leave together and fly a fan. Confidence **88**; the whole path is
on [weapon-fire.md](../ghidra/functions/psp-pulse-usa/weapon-fire.md).

`spread` is the fan's **half-angle in radians**, at confidence 82 - the original
multiplies it by the VFPU constant `2/pi` before `vcos_s`/`vsin_s`, which is
exactly Allegrex's radians-to-quarter-turns conversion and only makes sense one
way round. Its absence from the Missile reads correctly once this is known: a
homing weapon has no use for a launch fan.

**What is still ours here is the axis.** The original builds its rotation
through four `vpfxs`-prefixed lanes and reading the axis back off them was not
attempted; this engine rotates about the craft's up axis, which is what a
lateral spread of forward-firing rockets wants.

**This corrected an earlier reading, and the correction is worth keeping.** The
first pass through the executable found a *different* multi-shot handler -
`0x088675cc`, one projectile per `0.1 s` until a round counter on the craft runs
out - and wrote it up as the Rocket. That would have produced a staggered
stream. It is almost certainly the Cannon, whose `<Stats>` is the only one
authoring `rounds` and `rate`. What caught it was somebody who had played the
game saying the three fly in parallel.

### What a rocket can hit, and what it still cannot do

`oag_race::Mode::has_opponents` is `true` for a single race now that the AI
drives, so a volley can reach seven moving craft as well as track geometry and
the firing craft's own hull at close range.

**What is missing is aiming.** Nothing picks a target: the player connects by
pointing the craft, and an opponent never fires at anybody at all - see
[ai.md](ai.md). The fan is what stands in for aim, which is a reasonable reading
of what a three-rocket spread is *for*, and is worth remembering before anyone
concludes the rockets are inaccurate.

`a_rocket_fired_on_a_real_track_flies_and_detonates` checks the flight half off a
real disc: 221 units over 16 ticks into Talon's Junction's own collision soup,
with the volley's lateral components measured symmetric on the disc's own
`spread`.

### What is not built

**The slowdown mechanic is recovered but not built.** Its law came out of the
PSP executable on 2026-09-06 - `Ship_AddSlowdown` (`0x08848690`) adds a hit's
`slowdown_time` to a timer at `craft+0x2e0` and clamps the running total to
`<Global slowdown_limit>`, which is therefore a **ceiling on seconds of slowdown
outstanding**, not a speed floor; while that timer runs the victim gets no
engine thrust, a zeroed throttle and no lateral grip, and its hover target
height is lowered. `oag_tables::weapons` decodes `slowdown_time` on all six
decoded blocks as of the same day. What is *not* built is the physics half - a
craft hit by a mine, rocket or missile does not slow down yet. See
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md). `Ship_Damage`'s `weapon_kind`
sub-bucket - nine cases - is unmapped, and the absorb-spark effect its
`source == 2` branch triggers is not reproduced. The visual is a placeholder: an
additive billboard per rocket and a fading one where it goes off, in the engine
flare's own texture, because `Ship Muzzle` (`0x3e2`) and `cannon_flash`
(`0x3eb`) are the original's own weapon effects and neither is built.

## Absorbing

`CIRCLE`, and it pays the weapon's own `<Stats absorb>` into the energy pool
through `oag_physics::damage::add`, which clamps at the skill-indexed `<Misc>`
maximum. **The clamp is recovered**: `Ship_SetShield` (`0x0883e6f4`) takes
`min(amount, max)` and floors nothing, so a full pool gains nothing and the
`ShieldBar` cannot overfill.

Absorb works for every weapon, implemented or not, because `absorb` is decoded
for all thirteen - so a pickup this engine cannot fire is still worth
collecting. That is not a design choice; it is what happens when the file
authors one attribute for everything.

## The refresh timer is a debounce, not a respawn

`<WeaponPad refresh_time>` is `0.55` seconds for every speed class on both
shipped PSP discs, and `elimination_refresh_time` is `0.05`. A craft at racing
speed clears a pad about 9.6 units long in well under half a second, so what the
timer buys is that one crossing is one pickup - not that a collected pad goes
away for a while. Eliminator's being an order of magnitude shorter fits a mode
that hands weapons out far more freely.

Stamped on **any** hit, including one that grants nothing, because that is what
the original does: the stamp is unconditional and the grant is not.

## What is not built

- **Two of the thirteen weapons**, now that the Cannon and the Quake have
  joined the built set - see
  [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
  The LeachBeam's beam is a resolved link to whatever the Missile's own
  lock-on already picked, drained continuously while connected; the two rate
  functions and the health/shield consumer are still unread. The Repulser
  remains the one weapon needing a genuine craft-state field, and is
  deferred as Eliminator-only regardless.
- **The Cannon's per-class base speed, `func_0x00060af4` (`0x08864af4`).**
  `Cannon_Init` adds it to the firing craft's own current speed and the
  Cannon's `<Stats>` authors no speed at all for it to be read off the disc
  instead, so this build invents a value - see `oag_gameplay::
  projectile::cannon::BASE_SPEED_KMH`'s own doc comment, chosen and given no
  confidence score. This is the one place `cannon-quake-leachbeam.md` itself
  is not sufficient to build the Cannon from without a further Ghidra
  session, and its own "Not chased" line says so.
- **What a Cannon round's collision does beyond direct damage.** No
  `Cannon_HitCraft`-style function was located, so this build gives it the
  same floor-following flight and wall/craft detonation every other unread
  projectile weapon here gets as a placeholder, and applies
  `damage_per_bullet` to whatever it struck directly with no splash - the
  schema's own shape, since the Cannon authors neither `blastforce` nor
  `blastradius`.
- **The Plasma's `charge_time` is dead data, and the wind-up it looks like it
  describes is real anyway.** Both closed 2026-09-09, and they are separate
  facts. The weapon *does* wind up before it fires - one second, held on the
  firing craft's nose, from `Plasma_Init`'s `+0x4c`/`+0x50` and the pool
  walker `Plasmas_Update` (`0x0886b490`) that spends them - so the from-play
  report was right and the earlier read had followed the wrong four functions.
  But the duration is a hardcoded `1.0f` in `.rodata` (`0x08a7c098`), not the
  `charge_time="3"` the file authors: a sweep of all 69 functions that reach
  the weapon-stats table finds **zero** accesses at `charge_time`'s `+0x9c`
  against fourteen at the `venomspeed` control offset, and **Pure hard-codes
  the same second in the same two stores**. The attribute earns no field in
  `oag_tables::weapons::PlasmaStats` for the usual reason, and this build now
  holds the shot for `oag_gameplay::projectile::plasma::CHARGE_SECONDS`. See
  [plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md#the-charge-is-real-and-it-is-not-charge_time).
- **The Bomb's `damageradius`.** Authored, the only second radius any weapon
  has, and no consumer found - so it is decoded nowhere and spent nowhere. See
  `oag_tables::weapons::BombStats`.
- **A mine or a bomb draws nothing.** `Pulse_Mine.vex` and `Pulse_Bomb.vex` are
  both named and located and no renderer reads either, so a laid charge is
  invisible.
- **The pad's ready-to-collect colour cycle.** `WeaponPad_UpdateRefreshTimer`
  (`0x0892c034`) packs a grey into `pad+0x6c` while cooling down and cross-fades
  a small colour table once it is collectable. Observed, not implemented, so a
  spent pad looks the same as a fresh one.
- **`SubWeapon`.** The layouts author it, which suggests a craft can hold two.
  Nothing read says so, and the inventory here is one slot.
- **The `front`/`back` odds columns**, which need race positions, which need
  opponents that move.
## The determinism gate, and the hole that used to be here

**The simulation is deterministic.** Nothing here reads a wall clock, OS entropy
or a `HashMap` iteration order; every draw goes through the seeded `World::rng`,
so the same seed and the same inputs give the same race. That part of
[the rules](../architecture/determinism.md) always held.

**What did not hold, until 2026-08-11, is that the gate could see it.** The
committed hash covered `ShipState` and the tick alone
(`oag_physics::probe::hash_state`), and three pieces of this feature's state sat
outside it: the per-pad refresh timers on `Race`, the inventory on
`oag_gameplay::Ship`, and the generator's own position, which nothing had ever
hashed.

That was sharper than "an uncovered field". A pad's refresh timer decides whether
`pickup::draw` is called, and `draw` consumes `World::rng` - so **a refresh timer
one tick out shifted the whole generator stream from there on, changing every
later draw in the race, and the gate reported green.**

### What covers it now

Two halves, and neither needs a disc, so both run on all three CI platforms:

| Half | Covered by | Guarded by |
| --- | --- | --- |
| The whole `World` - ships, inventory, projectiles, lap state, RNG position | `oag_gameplay::hash::hash_world` | committed constants in `crates/gameplay/tests/determinism.rs` |
| The pad timers and distance caches, which live on `Race` | `oag_game::race::Race::state_hash` | `a_pad_refresh_timer_one_tick_out_moves_the_race_hash` and its neighbours, in `race.rs`'s own `#[cfg(test)]` block |

The split is not tidiness: `Race` normally needs a disc, so its constants could
only live in an `#[ignore]`d test. What makes the pad half coverable is that
`race_with_weapon_pads` builds a `Race` from synthetic pads with no disc at all.
**Nothing here is left to `just test-data`.**

**The fields stayed where they were**, which is what this page previously said
the fix should be: `oag-physics` depends on nothing but `oag-core` and cannot
name a `Weapon`, and pad timers belong to the track rather than to a craft. The
hash widened around them instead.

Every struct `hash_world` touches is destructured **exhaustively, with no `..`
rest pattern**, the same mechanism `probe::hash_state` uses - so a new field on
`World`, `Ship`, `Held`, `Projectile` or `RaceState` stops that file compiling
rather than being silently left out.

### Still open

A **snapshot-based replay** still cannot restore a single race, because the pad
state is not in the world snapshot even though it is now in the hash. Moving it
would be the replay's change to make, with its own reason, rather than a side
effect of this one.

## See also

- [race modes](race-modes.md) - what a single race is, and what it does without
- [weapon stats](../formats/weapon-stats.md) - the table and its schema
- [pads](../formats/pads.md) - the geometry and the trigger volumes
- [the pad runtime](../ghidra/functions/psp-pulse-usa/pads.md) - the trigger
- [shield](../ghidra/functions/psp-pulse-usa/shield.md) - the pool absorb pays
- [the HUD](../ui/hud.md) - the widgets
