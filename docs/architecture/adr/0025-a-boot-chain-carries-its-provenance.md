# ADR-0025: A boot chain carries its provenance

## Status
Accepted. Narrows [ADR-0023](0023-boot-sequence-as-title-data.md) rather than
superseding it, the way
[ADR-0024](0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md) narrows
ADR-0019.

## Context

[ADR-0023](0023-boot-sequence-as-title-data.md) made the boot sequence a table
per title, and defined `oag_title::BootProfile::chain` as a **measurement**. The
reason is specific and it still holds: **a front-end XML's declared entry point
is not the runtime's.** Both PSP titles' `Skin.xml` declare the language picker
first; Pure's runtime agrees and **Pulse's does not** - a cold boot opens
straight into `LogoFMV` playing its intro. A build that had read the declaration
and called it the order would have shipped Pulse's boot wrong, and had nothing
in the type to tell it so.

Wipeout HD then arrived in a state that definition could not express:

- Its **layout is fully recovered** - `skin.xml` in six archives agreeing on
  every layout global to the digit, confidence 92
  ([hd-frontend](../../formats/hd-frontend.md)).
- Its **order is only declared**. The nine screens and their redirects were read
  screen by screen out of that same XML, and **no capture of a PS3 running this
  title exists in this project**. The executable settles the entry point alone,
  at confidence 70.

`oag_title::FrontEnd` holds the two together, so HD had exactly two states
available and both were wrong:

- **`front_end: None`** - what it had. Refused a front end whose layout is
  better evidenced than some of what is already shipped, and left an entire
  title unable to reach its own menus, its own intro reel or its own music. The
  soundtrack work landed in that state and could never be *heard*.
- **`front_end: Some(..)` with the declaration in `chain`** - a hypothesis in
  the one field whose contract is that it never holds one. Every screenshot of
  that boot would then be filable as evidence of what a PS3 does.

## Decision

**A boot chain says where its order came from.** `BootProfile` gains

```rust
pub enum Provenance { Measured, Declared }
```

as a required field. `Measured` means a cold boot of the original was watched;
`Declared` means the title's own data states this order and nothing has watched
it run. Both PSP titles are `Measured`; Wipeout HD is `Declared`.

**Anything that shows such a boot to a person says so.** `oag-game`'s
`boot::load_shell` prints a line near the top of every boot report of a
`Declared` chain, before any of it is drawn.

The field is neither defaulted nor an `Option`. A profile is a compile-time
constant in a title package, and making the author write one of two words is
what stops a fourth title's declaration arriving as a measurement by omission.

## Consequences

- **Wipeout HD boots.** It walks its declared chain, draws its own screens in
  its own 1920x1080 grid, and plays the Studio Liverpool reel out of a `.bik`
  ([bik](../../formats/bik.md)). Its refusal in `load_shell` is gone; the
  refusal for a title with **no recovered front end at all** stays.
- **The distinction is now visible where it was previously enforced.** This is
  the honest trade and it is a real loss: a reader who ignores the report line
  can screenshot a declared boot and believe it. The old `None` made that
  impossible by making everything impossible. What replaces the guarantee is a
  label plus `oag-hd`'s own test, which fails if anyone changes HD to
  `Measured` without a capture behind it.
- **An emulator capture now *upgrades* a title rather than unlocking it.** The
  work HD still needs - watch a PS3 boot, settle whether the picker runs at all
  on a machine that takes its language from the XMB, settle which of the six
  `skin.xml` copies is live - is unchanged in kind and smaller in consequence.
- **ADR-0023 is narrowed, not overturned.** Its reasoning about why a
  declaration cannot be trusted as an order is exactly why this field exists
  rather than why the field is unnecessary. What changes is that "the chain is a
  measurement" becomes "the chain says whether it is one".
- **Two more axes moved into the title package alongside it**, both because HD
  disagreed with the PSP titles rather than because the field looked tidy:
  `FrontEnd::root` (HD names its front-end plugin where the PSP titles number
  it) and `FrontEnd::language_plugins` (sixteen named plugins against five
  numbered ones). Each had been a constant in `oag-pulse` that every title
  reached for.
- **It does not grade anything finer than two values.** "The executable returns
  this screen name at confidence 70" and "somebody read the redirects" are both
  `Declared`, and the difference lives in prose on the title package and the
  docs page. A confidence number in the type was rejected below.

## Alternatives considered

- **Leave `front_end: None` and wait for a capture.** The status quo, and it was
  a defensible position for as long as the only cost was HD's menus. It stopped
  being defensible when the cost included a decoded intro reel and a working
  soundtrack that no one could reach.
- **Ship the declaration in `chain` with a doc comment.** Cheapest, and the one
  this ADR exists to refuse: a caveat in a doc comment is not carried by the
  value, so nothing downstream can print it, test it, or refuse to trust it.
- **A confidence score rather than two values.** Tempting, the repository
  already scoring every RE claim 0-100. Rejected: the consumer is a boolean -
  either a run says "this order was watched" or it says the other thing - and a
  number invites an arithmetic nobody defined. The rubric score belongs on the
  docs page, where it is, and where it can carry its evidence.
- **A separate `DeclaredBootProfile` type.** Would make the difference
  unignorable by construction. Rejected as duplicating every field and every
  method of `BootProfile` to express one bit, and it pushes the branch into
  every consumer rather than into the one that reports.

## See also

- [ADR-0023](0023-boot-sequence-as-title-data.md) - the decision this narrows
- [ADR-0022](0022-title-packages.md) - the title axis `FrontEnd` sits on
- [hd-frontend](../../formats/hd-frontend.md) - what HD's XML declares, and what a capture would settle
- [bik](../../formats/bik.md) - the reel the chain's one movie step plays
