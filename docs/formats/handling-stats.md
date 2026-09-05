# Handling stats

**Status: decoded.** Implemented in
[`oag-formats::handling`](../../crates/formats/src/handling.rs). Ship handling is
**data, not code**. Every tunable lives in
`Data\Ships\<Team>\handlingstats.xml` inside `Data.wad`, one file per team,
with a block per speed class.

That is the best possible news for M4: the physics code defines the *shape* of
the model, and these files supply every constant.

## Reading them

```sh
oag-wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    'Data\Ships\Feisar\handlingstats.xml' --expand
```

Stored as [shortened XML](fexml.md), so `--expand` is needed.

Eight files were located by hashing candidate names, one per team **the disc
ships**: `AG_Systems`, `Assegai`, `EGX`, `Feisar`, `Goteki`, `Piranha`, `Qirex`,
`Triakis`. Each is about 2.7 KiB.

Four more exist, one per [downloadable pack](dlc-pack.md) - `Auricom`,
`Harimau`, `Icaras` and `Mantis`. They are in the same schema and parse with the
same reader, and they live in `PACKn_UI1.edat` rather than in the pack's main
archive, which is a trap worth knowing: mount only `PACKn.edat` and the ship
loads with no stats to race it under.

## No values are reproduced here

Per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md), this page
documents the **schema**: element names, attribute names, and which code
consumes each. The values are the game's design data and are read at runtime
from the user's own disc.

The distinction matters and is worth stating: a field name is a description of
the format, a tuning table is the content itself.

This holds for the code as well. `oag_formats::handling` and
`oag_physics::Handling` name every field and ship no value; the unit-test fixtures
count 1, 2, 3 ... in document order, which is about as obviously synthetic as data
gets. The ground-truth tests assert shapes and signs, never magnitudes.

## Schema

```xml
<Handling>
  <Stats team="...">
    <InternalCamera fov headtilt height length pitch/>
    <BackwardCamera fov headtilt height length pitch/>
    <BonnetCamera fov height length pitch/>
    <ExternalCameraFar   fov lookat_height lookat_length
                         pos_height pos_length spring_horiz spring_vert/>
    <ExternalCameraClose fov lookat_height lookat_length
                         pos_height pos_length spring_horiz spring_vert/>
    <AirbrakeGraphics amount down_speed up_speed/>
    <Misc height length shield easyshield width weight_distribution/>
    <FE speed thrust handling shield/>

    <Class name="VENOM|FLASH|RAPIER|PHANTOM">
      <Engine   accelcap amount falloff gain turbo/>
      <Brakes   amount falloff gain/>
      <Turning  amount falloff gain/>
      <Airbrake amount drag falloff gain turn slidegrip sideshift/>
      <Antigrav grip_air grip_ground landing_rebound
                rebound rebound_jump_time ride_height/>
      <Physical flight_gravity mass normal_gravity track_gravity/>
      <pitch    pitch_air pitch_ground pitch_damping antigrav_height_adjust/>
    </Class>
  </Stats>
</Handling>
```

`<FE>` holds the four bar values shown on the ship-select screen. They are
presentation only and need not agree with the physics.

`easyshield` alongside `shield` implies a difficulty-scaled shield pool.

## What is decoded

`oag_formats::handling` parses the whole schema above. Every element and every
attribute the table lists has a typed field:

| Element | Type | Confidence |
| --- | --- | --- |
| `Stats` | `handling::Stats`, one per team | 90 |
| `InternalCamera`, `BackwardCamera` | `Camera` | 85 |
| `BonnetCamera` | `BonnetCamera` | 85 |
| `ExternalCameraFar`, `ExternalCameraClose` | `ExternalCamera` | 90 |
| `AirbrakeGraphics` | `AirbrakeGraphics` | 55 |
| `Misc` | `Misc` | 88 |
| `FE` | `Fe` | 75 |
| `Class` | `Class`, four per team | 90 |
| `Engine`, `Brakes`, `Turning` | `Engine`, `Brakes`, `Turning` | 85 |
| `Airbrake` | `Airbrake` | 90 |
| `Antigrav` | `Antigrav` | 85 |
| `Physical` | `Physical` | 90 |
| `pitch` | `Pitch` | 80 |

The camera rows were 60 until the five blocks were traced through the loader and
read back out of a running race; what they mean, which of them a player can
select, and the sign convention of the two `ExternalCamera` blocks are all in
[camera.md](../ghidra/functions/psp-pulse-usa/camera.md). Note the finding there that
`<BackwardCamera headtilt>` is parsed by the file's own schema and then **not
stored**, and that `<AirbrakeGraphics amount>` is converted to radians at load,
which the Units section below does not yet cover.

The entry point is `handling::from_blob`, which takes an archive blob and does
whatever that blob needs: PSP's is [shortened](fexml.md) and gets expanded, PS2's
is plain text and does not. `handling::parse` takes text that is already expanded.
`oag_pulse::race::TEAMS` and `handling::entry_name` build the eight base-disc
archive names; `oag_pulse::race::DLC_TEAMS` names the four the downloadable
packs add.

Two decisions are worth stating, because both are load-bearing:

- **Nothing defaults.** A missing element, a missing attribute, or an attribute
  that is not a finite number is a typed `handling::Error`, never a zero. A `mass`
  that quietly arrived as `0.0` does not crash: it produces a ship that behaves
  oddly, which reads as a tuning problem and gets looked for in the force law
  rather than in the loader. Note that Rust's `f32` parser accepts `"nan"` and
  `"inf"`, so non-finite values are rejected explicitly.
- **The four `<Class>` blocks are a fixed array indexed by speed class**, not a
  map. "Exactly four, one each" is then a property of the type instead of
  something every caller has to check, and no iteration order that could differ
  between runs reaches the simulation.

The confidence scores above are all about *meaning*, not about parsing, and none
reaches 95 because none is runtime-verified. That the `Camera` fields are read
correctly is not in doubt; that `headtilt` is the roll-follow amount is a reading
of the name, which is why the camera rows sit at 60 under the
[rubric](../reverse-engineering/confidence-rubric.md). The physics rows are higher
because they were confirmed twice - see the cross-check below.

### The XML tree parser is shared

The parser reads its tree through `oag_formats::fexml::parse`, which is where the
hand-rolled walker that used to live in `oag-game`'s `screen.rs` now lives. Two
schemas, one format, one tag scanner. Same reasoning as the disc-to-blob
consolidation in [workspace layout](../architecture/workspace-layout.md).

That is not a tidiness argument, and this format supplied the proof while the move
was being made: the malformed PS2 declaration described below silently ate whole
documents, and the fix landed in one place instead of two. A second walker is a
second thing to notice it in, and the failure mode is a parser that returns
*nothing* rather than an error.

The shared walker also handles `<Values>`, which is an attribute carrier for its
parent throughout this format: `<Brakes><Values amount="..."/></Brakes>` and
`<Brakes amount="..."/>` mean the same thing. `handling` accepts both, and there
is a unit test for it. **No shipped `handlingstats.xml` was observed using the
carrier form** - in all 16 real files the attributes sit directly on their
elements - so this is tolerance for a documented convention of the format, not a
reading of the ship data.

### Never emit scientific notation when regenerating one

The float accessor the game reads every parameter in this file with is
hand-rolled and **cannot read exponent notation**. It skips any character that
is not a digit, `.` or `-` instead of stopping at it, so the exponent's digits
are absorbed as extra mantissa digits at whatever scale the scan has reached and
its sign is applied to the whole number. `1e-5` parses as `-15`; `1.5e3` parses
as `1.53`. Same defect, opposite-looking answers depending on where the `.` is.
There is no error and no clamp.

This is confirmed on **both** builds - `Xml_AttributeAsFloat` at `0x0895379c`
on the [PSP](../ghidra/functions/psp-pulse-usa/xml-reader.md) and `0x00203868` on
the [PS2](../ghidra/functions/ps2-pulse-eu/xml-reader.md) - and on the PSP every
`HandlingXml_Parse*` function calls it directly, so it is not something a
future build might have fixed. A correct `strtod`-based accessor exists in the
same parser, but nothing in this file goes through it.

No shipped `handlingstats.xml` uses exponent notation, so nothing is broken
today. It only matters for tooling: **a writer must format every value in plain
decimal**, which also means it must not let a language's default float
formatting fall back to `1e-5` for small magnitudes.

## Cross-check against the code

The names here **independently confirm** the parameter block recovered from the
binary. The Airbrake section was read from disassembly as
`{gain +0xd8, falloff +0xdc, amount +0xe0, turn +0xe4, drag +0xe8,
slidegrip +0xec, sideshift +0xf0}`, and the XML carries exactly those seven
names. Likewise `Physical` matches the recovered `normal_gravity +0xf8`,
`flight_gravity +0xfc`, `track_gravity +0x100`.

Two independent sources agreeing on a set of names is much stronger than either
alone, and it retires three Airbrake names that the static reading had left
unresolved.

See [physics](../physics/README.md) for how each is consumed.

### `<Misc>` sits on the stats base, and its offsets are PS2-confirmed

`<Misc>` is the one element outside every `<Class>` block whose destination was
unknown, and the PS2 build settles it. `HandlingXml_ParseMisc` (`0x0014db08` in
`SCES_547.48`) writes, with the attribute names read out of `.rodata` at
`0x002a5780`..`0x002a57c0` rather than guessed:

| Attribute | Offset from the stats base |
| --- | ---: |
| `width` | `0x78` |
| `length` | `0x7c` |
| `height` | `0x80` |
| `easyshield` | `0x84` |
| `mediumshield` | `0x88` |
| `hardshield` | `0x8c` |
| `shield` | `0x84`, `0x88` **and** `0x8c` at once - a default for all three |
| `weight_distribution` | `0x90` |

Note that the parser accepts two attributes **no shipped file authors**,
`mediumshield` and `hardshield`, and that plain `shield` is a bulk default filling
all three slots. That turns this page's "`easyshield` alongside `shield` implies a
difficulty-scaled shield pool" from an inference into a reading: there are three
difficulty slots, the files set the easy one explicitly and let `shield` cover the
other two.

Three things follow, and they are why this element is worth its own section.

**`weight_distribution` is the scalar the PSP's `Ship_UpdatePitch` reads.**
[psp-pulse-usa/engine.md](../ghidra/functions/psp-pulse-usa/engine.md) recorded
`stats_base + 0x90` as "a per-team scalar outside every class block; its element
is not determined" - it is this one, and it is the only stats-base field that page
named without an element. The PS2 `Ship_UpdatePitch` reads the same offset through
the stats pointer at `craft+0x8c`; see
[ps2-pulse-eu/craft-update.md](../ghidra/functions/ps2-pulse-eu/craft-update.md).

**The stats base is now accounted for with no gap.** `<Misc>` fills `0x78`
through `0x90`, which is exactly the range between `AirbrakeGraphics`
(`0x6c`..`0x74`) and the first class block at `0x94`. A contiguous run from `0x6c`
to `0x114` with nothing unclaimed is the same kind of argument the offset chain
makes for [the WAD directory](wad.md#the-offset-chain-pins-the-layout): it is not
proof, but a wrong offset here would have to be wrong in a way that still closes
the range.

Confidence **88**, up from the 80 this page gave `<Misc>` on the strength of its
attribute names alone. It is a direct read of the writing parser, corroborated by
`Ship_UpdatePitch` consuming `0x90` on both platforms, and capped below 95 because
nothing has been observed loading a file at runtime. Full reading and addresses:
[ps2-pulse-eu/handling-xml.md](../ghidra/functions/ps2-pulse-eu/handling-xml.md#what-the-ps2-answers-that-the-psp-page-left-open).

**One caveat, narrower than it was.** The PS2 page flags that the *PSP binary*
was not re-checked for a `<Misc>` parser, leaving open whether the element is
PS2-only. On the data side that is settled: all eight PSP team files carry
`<Misc>` with all six attributes, because `oag_formats::handling` requires the
element and every attribute with no defaults, and
`crates/formats/tests/handling_ground_truth.rs` parses all eight - re-run this
session, 8 teams and 32 parameter sets on each disc. What is still open is *which
PSP function* writes `0x78`..`0x90`; see the question added below.

## The self-check: completeness

Most formats here were settled by arithmetic - a correct parse accounts for every
byte, and a parse one structure out cannot. Text has no such check: a wrong
reading of XML does not overrun a buffer, it just misses something.

The checkable prediction for this format is **completeness** instead, and
`crates/formats/tests/handling_ground_truth.rs` asserts it:

- All eight team files resolve in the archive by name hash, **on both releases**.
  Eight, not "at least N": the interesting half of the claim is that none is
  missing, and a team spelled differently on one release would be a finding.
- Each is stored the way its platform stores it, asserted per platform: PSP
  shortened (`fexml::is_fexml`), PS2 plain text beginning `<?xml`.
- Each parses. Since the decoder requires every documented element and attribute,
  **a successful parse is the completeness assertion** - 174 attributes and 32
  parameter sets per file, none defaulted.
- The four `<Class>` blocks are exactly the four speed classes, each in its own
  slot, and no two blocks are the same parameter set. A mapping that read one
  block into four places would satisfy every count and fail this.
- Hull extents, both shield pools, `mass` and `ride_height` are positive, and
  `slidegrip` lies in 0 to 100. These are the properties the force law needs in
  order to be defined at all; none of them names a value.

**Confidence 90 for the schema, 88 for the completeness claim.** Both tests were
run and passed: 8 teams and 32 parameter sets on `pulse-psp-usa.chd`, and the same
8 and 32 on `pulse-ps2-eu.chd`. That is an exact invariant across 16 real files,
plus corroboration in a second binary, which the
[rubric](../reverse-engineering/confidence-rubric.md) caps at 94 for want of a
runtime trace: agreement with shipped data shows the layout is self-consistent,
not that the engine uses it the way this page says.

The camera rows in the table above stay at 60 regardless. A second release
agreeing on the name `headtilt` says nothing about what `headtilt` does.

**Validated against two discs**, both of them Pulse. *Wipeout Pure* was checked
as a third corpus and is **deliberately not counted**: it carries
`Data\Ships\<Team>\handlingstats.xml` at the same addresses, in plain
(unshortened) XML, with the same element names, attribute names and
`<Stats team>` / `<Class name>` nesting - **eleven** ship directories rather than
eight - but `oag_formats::handling::parse` still refuses ten of them. Pure's
whole element diff has since been enumerated in one pass, and every difference
runs the same way: Pure has a **fifth speed class** below the four Pulse ships,
and it lacks `<Class><pitch>`, `<Stats><FE>`, `<Airbrake sideshift>`,
`<Misc easyshield>` and `<Misc weight_distribution>`. Nothing else differs.

Every one is a Pulse-era *addition*, so the handling model grew between titles
rather than changing shape - and the parser refusing them is the "nothing
defaults" rule above working as designed on a file it was not designed for, not
a bug. `<pitch>` is the one still outstanding, because it feeds
`oag_physics::Pitch` and ADR-0009 item 2 gates second-title simulation work.
Confidence **94**, from an exact set comparison over 19 shipped files run by
`crates/pure/tests/handling_schema_ground_truth.rs`. Details, the eleventh file
that carries no `<Class>` at all, and what a two-title parser would cost are in
the [Pure probe](pure-status.md#handling-stats-the-schema-holds-the-parser-does-not).

The tests are `#[ignore]`d and never run in CI, because they need game content.
Run them with `just test-data`, and with `OAG_REQUIRE_GAME_DATA=1` so an absent
image fails instead of skipping green.

The decoder's own behaviour is covered by unit tests over synthetic fixtures with
invented values: a well-formed document, a missing attribute, a missing element, a
missing class, an unknown class name, a duplicated class, a non-numeric value, a
non-finite value, an attribute arriving on a `<Values>` carrier, a plain
unshortened document, and the full shortened-blob path. The missing-attribute test
removes each of the fixture's 174 attributes in turn and requires every removal to
be an error, so the "nothing defaults" rule is enforced rather than asserted in
prose.

## The PS2 release ships the same schema

This answers one of the questions that used to be open below. `WADS2.WAD` on
`pulse-ps2-eu.chd` carries all eight `Data\Ships\<Team>\handlingstats.xml` entries
under the same names, with **identical element and attribute sets**. Three
differences, all in storage rather than schema:

| | PSP | PS2 |
| --- | --- | --- |
| Storage | [shortened](fexml.md), per-file `<code>` dictionary | plain text, no dictionary |
| Pitch element | `<pitch>` | `<Pitch>` |
| Declaration | none | `<?xml ...?>`, after a leading space |

`handling::from_blob` dispatches on the leading bytes, so a caller does not have
to know which disc it is holding. Element lookup is case-insensitive, so `<Pitch>`
costs nothing - but it is worth knowing before grepping an expanded file for one
spelling.

The PS2 declaration is **malformed**: `<?xml version="1.0" encoding=utf-81"?>`,
with `encoding` unquoted and a stray digit. That leaves an odd number of `"` in
the tag, and a parser that tracks quotes across the closing `>` goes out of phase
for the whole rest of the file - every element after the declaration vanishes, with
no error anywhere. `fexml::tag_end` therefore ends a processing instruction at its
own `?>` and a comment at its own `-->`, the way XML says, rather than at the first
unquoted `>`. Two lines, eight otherwise unreadable files. A stricter XML reader
would reject all eight outright.

Whether the two releases ship the same *values* is still open, and deliberately
so: answering it would mean putting a comparison or a fingerprint of shipped design
data in the repository.

## A second file shares the parser: `Data\XML\HandlingStats.xml`

`Handling_ParseStats` (`0x0883a2f0`) has **two** callers and they open different
files:

| Caller | File | Carries |
| --- | --- | --- |
| `0x088c291c` | `%s\handlingstats.xml`, per team | `<Stats>` |
| `0x0894f6a8` | `Data\XML\HandlingStats.xml`, once | `<Global>` |

Both documents have a `<Handling>` root, and the parser walks its children
looking for `<Stats>` *and* for `<Global>`, handing the latter to
`Xml_ReadGlobalSettings` (`0x0883a970`). `<Global>` configures the engine rather
than a ship: Zone mode's speed law and shield recharge, the speed-pad and
weapon-pad tunables, per-class gravity, the start boost, and three camera pitch
modifiers.

**Which file carries which is measured rather than assumed.** All sixteen shipped
per-team files - eight teams on each of the PSP and PS2 discs - hold `<Stats>` and
nothing else, asserted by `which_top_level_elements_handlingstats_carries` in
`crates/formats/tests/handling_ground_truth.rs`. So `oag_formats::handling::parse`
does not look for `<Global>`; `global_from_blob` reads it out of the global file.

Two parts are decoded, because they are the two with consumers:

```text
<Global>
  <Special roll_cost roll_speed roll_turbotime speedpad_jump turbo_jump/>
  <Zone start increment recharge/>                       <- decoded
  <GlobalClass name="VECTOR|VENOM|FLASH|RAPIER|PHANTOM">
    <SpeedupPads amount time/>                           <- decoded
    <GravityMul airborne/>
    <WeaponPad refresh_time elimination_refresh_time/>
  </GlobalClass>                                          x5
  <ExternalCloseCamPitchMod/> <ExternalFarCamPitchMod/> <ReplayCamPitchMod/>
  <CameraSideOffset/> <StartBoost/>
</Global>
```

`<Zone>` is confidence **84**; `<SpeedupPads>` is **90**, because
`Xml_ReadGlobalSettings` was read to the store instruction: both attributes go
through `Xml_AttributeAsFloat` into the per-class tables at `0x08b36bc0`
(`amount`) and `0x08b36bd0` (`time`) **verbatim, with no load-time scale** -
unlike the five in `oag_gameplay::handling::SCALED_FIELDS`.
The rest is named in
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md) and can be added when
something needs it. Values are read at runtime from the player's own disc and are
not reproduced here, for the reason the next section up gives.

Two consequences worth having in one place:

- **`<GravityMul airborne>` fills `g_class_gravity_scale`** (`0x08ab0dcc`), which
  is `oag_physics::forces::Environment::class_gravity_scale`. Decoded, and the
  table's confidence goes from 78 to **90**: it now has a read writer and two read
  readers. See the subsection below, because the attribute's name is misleading.
- **`<Special speedpad_jump>` fills `0x08b36bec`**, and is decoded, as
  [`Special`](../../crates/formats/src/handling.rs). It is the magnitude of the
  speed-pad boost's tilt toward the hull's up axis while d-pad Up is held - a
  5.71-degree tilt and 0.5 % more force at the shipped `0.1`, **not** a jump. Read
  the same way `<SpeedupPads>` is, verbatim with no load-time scale. See
  `oag_physics::engine::speedup_pad` and
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md). The other four
  `<Special>` attributes - `roll_cost`, `roll_speed`, `roll_turbotime`,
  `turbo_jump` - still have no consumer and are still not decoded.

### `<GravityMul airborne>` scales the *grounded* gravity term

The attribute is named `airborne`. It is applied to the term that acts **on the
ground**, and the airborne term is multiplied by a literal `1.0`.

`0x08ab0dcc` has exactly three references - one write in
`Xml_ReadGlobalSettings` and two reads in `Ship_UpdateCraft` at
`0x08849b40`/`0x08849b48` - so there is no second table to confuse it with. The
gravity term builds four VFPU pairs and multiplies them:

```text
C600 = (class+0xf8, class+0xfc) = (normal_gravity, flight_gravity)
C610 = (g_class_gravity_scale[class], 1.0)      ; viim.s S611, 1
C620 = (mass, mass)
C630 = (craft+0x2b0, 1 - craft+0x2b0)           ; vocp.s S631, S630
worldForce.y += -(C600 * C610 * C620 * C630) summed over both lanes
```

Lane 0 carries `normal_gravity`, is multiplied by the grounded fraction, and is
the lane the scale lands on. Confidence **90**.

The two class-block offsets are not this page's guess either:
`HandlingXml_ParsePhysical` (`0x08838f50`) stores `flight_gravity` to `+0xfc`,
`mass` to `+0xf4`, `normal_gravity` to `+0xf8` and `track_gravity` to `+0x100`,
with a `0x80` stride per class that matches the `sll a0, a0, 0x7` at the gravity
site. That independently confirms the `Physical` row in
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md), which recorded the same
four offsets, and confirms `track_gravity` takes no part in gravity at all.

Whether "airborne" describes an intent the code does not implement, or a rename
nobody propagated, is **not** answered. What is established is which term the
number reaches.

**A capture agrees, and it was taken before any of this was known.**
[angular-velocity-column.md](../physics/angular-velocity-column.md) predicts the
craft's resting suspension compression from read constants and checks it against
a live `craft+0x308` read of `1.237` off the running original. That prediction
contains `normal_gravity` and `track_gravity` and **not** `flight_gravity`,
because it is a *resting* quantity:

| `classScale` used | predicted | error |
| --- | ---: | ---: |
| `1.0`, before this table was decoded | `1.2500` | `1.05 %` |
| the class's real scale | **`1.2390`** | **`0.16 %`** |

Two things follow. The scale is real and is not the identity, since applying it
improves an independent measurement by a factor of `6.6`. And it goes on the
**grounded** term, since putting it on the airborne one would leave a resting
compression untouched and the `1.05 %` gap with it. That is the same answer the
`vmul.p` chain gives, from evidence that knew nothing about it.

### There are five `<GlobalClass>` blocks and only four speed classes

Both discs author **`VECTOR` first**, then the four. No per-team file has a
`<Class name="VECTOR">`, and `SpeedClass` has no such variant.

**The original does not recognise it either.** `Xml_ReadGlobalSettings` matches
`name` against a four-entry table and, on no match, simply leaves
`g_handling_parse_class` holding whatever the previous match left there - it is a
global and nothing resets it per element. So `VECTOR`'s numbers are written into
some other class's slot and then overwritten by the four blocks that follow,
because `VECTOR` comes first. `oag_formats::handling::global_classes` skips
unrecognised names, which reproduces the outcome without reproducing the
accident. Confidence **88**.

**This holds only while `VECTOR` is authored first.** Authored last it would
corrupt `PHANTOM` in the original and not here.

**Two more independent subsystems carry the same shape, and neither is
authored either.** [ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md#the-class-index-and-the-dead-vector-branch)
found this page's own bug pattern a second time, in unrelated code: the
`AIStats` per-class loader recognises a `VectorStats` element name but never
assigns its match an index, so the class pointer would compute to a negative
offset if a shipped file ever authored one - and none does; `AIControlStats.xml`
and every `AIRaceStats_<class>.xml` carry only the four real classes.
Confidence 88, same page.

A third, checked while chasing this: the front-end's loading-screen message
keys run `MSC_LOAD_VENOM`, `MSC_LOAD_FLASH`, `MSC_LOAD_RAPIER`,
`MSC_LOAD_PHANTOM` (`0x08a827c0`-`0x08a827f0` on `pulse-psp-usa`) - four
entries, contiguous, immediately after a `"...Class Help"` key and before
`"Event Help"`, with no `MSC_LOAD_VECTOR` in the gap. Same completeness
argument as the WAD offset chain and the `<Misc>` stats-base range: not proof,
but a missing fifth key would have to sit somewhere else entirely rather than
in the run built for exactly this purpose. Confidence 85 - a string-table
completeness reading, not a traced consumer.

So three independently-recovered subsystems - ship handling globals, AI
per-class stats, and front-end message keys - all show the same four classes
and nothing shipped for a fifth. **Still not a runtime answer**: none of the
three has been confirmed by tracing a live consumer under PPSSPP, only by
exhausting the shipped data each one reads. Whether a fifth class exists
anywhere else in Pulse - Wipeout HD's class ladder does begin at Vector - is
**not** answered by this file and is not chased here. It is an open question on
[HANDOVER.md](../../HANDOVER.md), not a finding.

## Related files

Located alongside, same naming pattern, not yet decoded:

| Template | Notes |
| --- | --- |
| `Data\Ships\<Team>\Ship.vex` | The [model](vex.md) |
| `Data\Ships\<Team>\ship_FE.vex` | Front-end preview model |
| `Data\Ships\<Team>\<Team>shield.vex` | Shield effect |
| `Data\Ships\<Team>\ship_alt.dat` | **Decoded** - not a mesh but a livery: a `0x20` header (team name, `"ms"` tag) followed by four palette-plus-pixels blocks, three 128x128 and one 64x64, all 4 bits per pixel with a sixteen-entry `RGBA8` palette each. Exactly 26912 bytes. The blocks replace the hull model's `texture1.tga`..`texture4.tga` in place, so a skin is a texture swap on the same geometry. `PI_TeamModel`'s `PI_ModelSkin name="Alternative"` names it; see [`ship-skin.md`](../ghidra/functions/psp-pulse-usa/ship-skin.md) for the loader and the byte table, and [`dlc-pack.md`](dlc-pack.md#entry-0-is-a-manifest) for the declaring schema |
| `Data\Ships\<Team>\ship_eliminator.dat` | Byte-for-byte the same format as `ship_alt.dat` above, named by `PI_ModelSkin name="Eliminator"`. The two differ only in their pixels. Which of the two a hull gets is decided by a `== 0x12` comparison on a global rather than by an unlock - see [`ship-skin.md`](../ghidra/functions/psp-pulse-usa/ship-skin.md) |
| `Data\Ships\<Team>\Definition.xml` | Unknown |

## Open questions

- **Units.** Engine `amount` and Airbrake `amount` are on clearly different
  scales, by two orders of magnitude. The consuming code applies its own factors
  (the airbrake slide term multiplies by `0.01` and then `0.001`), so units only
  make sense read together with [physics](../physics/README.md). The two
  magnitudes themselves are shipped data and are not written down here, which is
  why this note describes the ratio rather than the numbers.

  **One attribute's unit is settled rather than open**, and it is the only one:
  `<AirbrakeGraphics amount>` is a **flap angle in degrees**, converted to
  radians by `HandlingXml_ParseAirbrakeGraphics` (`0x08839c68`) at load. That is
  a fact about the unit and not about any team's value, so it belongs here.
  Confidence 92; see
  [camera.md](../ghidra/functions/psp-pulse-usa/camera.md), which found it while
  mapping the camera block the parameter sits immediately after.
- **Camera semantics.** Every camera attribute parses, but the meanings in the
  table above are read off the names, not off the rendering code. Confidence 60.
  Settling them means finding the sites that consume the camera block.
- Whether the PS2 release ships the same *values*. The schema question is now
  settled - see above - but the values are not compared, and will not be here:
  recording the answer means recording a fingerprint of shipped design data. It
  would settle whether [ADR-0004](../architecture/adr/0004-asset-pipeline.md)'s
  "gameplay is identical across asset sets" holds, and needs a home outside the
  repository to do it in.
- **Which PSP function writes `0x78`..`0x90`**, the range the PS2's
  `HandlingXml_ParseMisc` fills. The data side is not in doubt - all eight PSP
  team files carry `<Misc>` with every attribute - and `Ship_UpdatePitch` reads
  `stats_base + 0x90` on the PSP, so something there must fill it. But no PSP
  `<Misc>` parser has been located, and none was searched for: the PSP binary was
  not reopened when the PS2 side was read. Deliberately carries **no confidence
  score**, because it is a hypothesis from the consumer rather than an observation
  of a parser. Finding the PSP writer of `0x78` settles it, and finding that there
  is none would be a genuine divergence between the builds.
- Whether tracks carry handling modifiers of their own.
