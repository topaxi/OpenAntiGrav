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

Eight files were located by hashing candidate names, one per playable team:
`AG_Systems`, `Assegai`, `EGX`, `Feisar`, `Goteki`, `Piranha`, `Qirex`,
`Triakis`. Each is about 2.7 KiB.

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
[camera.md](../ghidra/functions/psp-pulse/camera.md). Note the finding there that
`<BackwardCamera headtilt>` is parsed by the file's own schema and then **not
stored**, and that `<AirbrakeGraphics amount>` is converted to radians at load,
which the Units section below does not yet cover.

The entry point is `handling::from_blob`, which takes an archive blob and does
whatever that blob needs: PSP's is [shortened](fexml.md) and gets expanded, PS2's
is plain text and does not. `handling::parse` takes text that is already expanded.
`handling::TEAMS` and `handling::entry_name` build the eight archive names.

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
on the [PSP](../ghidra/functions/psp-pulse/xml-reader.md) and `0x00203868` on
the [PS2](../ghidra/functions/ps2-pulse/xml-reader.md) - and on the PSP every
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
[psp-pulse/engine.md](../ghidra/functions/psp-pulse/engine.md) recorded
`stats_base + 0x90` as "a per-team scalar outside every class block; its element
is not determined" - it is this one, and it is the only stats-base field that page
named without an element. The PS2 `Ship_UpdatePitch` reads the same offset through
the stats pointer at `craft+0x8c`; see
[ps2-pulse/craft-update.md](../ghidra/functions/ps2-pulse/craft-update.md).

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
[ps2-pulse/handling-xml.md](../ghidra/functions/ps2-pulse/handling-xml.md#what-the-ps2-answers-that-the-psp-page-left-open).

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
`<Stats team>` / `<Class name>` nesting - nine teams rather than eight - but
`oag_formats::handling::parse` refuses every one of them. Pure has a **fifth
speed class** below the four Pulse ships, which the fixed-length array indexed
by `SpeedClass` cannot hold, and it lacks three attributes this parser requires:
`<Airbrake sideshift>`, `<Misc easyshield>` and `<Misc weight_distribution>`.

All three are Pulse-era *additions*, so the handling model grew between titles
rather than changing shape - and the parser refusing them is the "nothing
defaults" rule above working as designed on a file it was not designed for, not
a bug. Confidence **92**, from a direct schema diff. Details and what a
two-title parser would cost are in the [Pure probe](pure-status.md#handling-stats-the-schema-holds-the-parser-does-not).

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

## Related files

Located alongside, same naming pattern, not yet decoded:

| Template | Notes |
| --- | --- |
| `Data\Ships\<Team>\Ship.vex` | The [model](vex.md) |
| `Data\Ships\<Team>\ship_FE.vex` | Front-end preview model |
| `Data\Ships\<Team>\<Team>shield.vex` | Shield effect |
| `Data\Ships\<Team>\ship_eliminator.dat` | Unknown |
| `Data\Ships\<Team>\Definition.xml` | Unknown |

## Open questions

- **Units.** Engine `amount` and Airbrake `amount` are on clearly different
  scales, by two orders of magnitude. The consuming code applies its own factors
  (the airbrake slide term multiplies by `0.01` and then `0.001`), so units only
  make sense read together with [physics](../physics/README.md). The two
  magnitudes themselves are shipped data and are not written down here, which is
  why this note describes the ratio rather than the numbers.
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
