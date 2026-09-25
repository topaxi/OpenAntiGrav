# Front-end XML

**Status: understood.** Implemented in
[`oag-tables::fexml`](../../crates/tables/src/fexml.rs).

Screens, widgets, 3D model previews, menu transitions and
[handling stats](handling-stats.md) are all XML. The front end is data-driven,
so much of the "menu code" is really menu *data*.

## Name shortening

Element and attribute names are replaced by one- or two-letter codes, with a
`<code>` element at the top of each file giving the mapping. An attribute
`Xs="Name"` maps the short name `X` to `Name`.

```xml
<code as="Values" bs="Screen" cs="Mode3D" ds="name" es="Model" ls="Src"></code>
<b d="MPLobby Info">
  <c><e d="Ship"><a l="Data\Ships\AG_Systems\ship_FE.vex"/></e></c>
</b>
```

expands to

```xml
<Screen name="MPLobby Info">
  <Mode3D><Model name="Ship"><Values Src="Data\Ships\AG_Systems\ship_FE.vex"/></Model></Mode3D>
</Screen>
```

**The dictionary is per file.** Across the 64 blobs in `FEData.wad`, every one
of the 18 short codes carries more than one meaning: `a` is `Values` in one file
and `Class` in another, `b` is `Screen` or `Difficulty`. A shared table would
silently corrupt most files, so the expander uses each file's own.

This is a size optimisation, not encryption.

**Confirmed in the reader.** Everything above was inferred from the data; the
PS2 executable's XML reader does exactly it. `Xml_OpenFile` scans the first
element, and if it is named `code` it stores each attribute's value into an
**18-slot** table on the document object, keyed by the **first character** of the
attribute name - so `as="Values"` keys on `a` and the trailing `s` is not read.
`Xml_NextElement` and `Xml_NextAttribute` then expand any name that is exactly
one character in `a`..`r`. The table lives on the document, so per-file scope is
not a convention but the only thing expressible. Details, including three traps
for anyone writing these files, are on
[the PS2 XML reader page](../ghidra/functions/ps2-pulse-eu/xml-reader.md).

That the 18 slots in the reader match the 18 codes counted across `FEData.wad`
by two unrelated routes is the strongest evidence on this page.

## Reading it

```sh
oag-wad cat <archive> 'Data\Ships\Feisar\handlingstats.xml' --expand
```

Identified by the leading `<code`. Files that begin `<?xml` are plain and need
no expansion.

**Validated against three discs**: `pulse-psp-usa`, `pulse-ps2-eu` and
`pure-psp-usa` - and Pure is the one that says something. Of its 291 XML
entries, **not one begins `<code`**: the name shortening does not exist on Pure
at all, so it is a Pulse-era size optimisation rather than a lineage-wide
convention. `--expand` is correctly a no-op on every Pure file, which is the
cheapest possible confirmation that the identification rule above (leading
`<code`, else plain) is the right one rather than a heuristic that happened to
work. See the [Pure probe](pure-status.md).

## Why it matters beyond the front end

**This is the richest source of real asset names in the game.** Most names are
built at runtime from templates like `%s\%strack%s.vex`, so they never appear in
the executable and cannot be recovered by scanning strings. The XML contains
them written out.

`fexml::asset_paths` collects them, and feeding those to
[`wad::hash_name`](wad.md#the-name-hash) resolves archive entries that binary
strings alone cannot. That is how all eight `handlingstats.xml` files were
located.

## Known elements

Recovered from expanded files, not exhaustive:

| Element | Role |
| --- | --- |
| `Screen` | A named screen or sub-screen |
| `Values` | Attribute carrier for its parent |
| `Mode3D`, `Viewport` | 3D preview embedded in a menu |
| `Model` | A `.vex` model, with `Src`, position and rotation |
| `Image` | A texture |
| `Movie` | Video playback; see [frontend-video](../ghidra/functions/psp-pulse-usa/frontend-video.md) |
| `Redirect`, `goto` | State transitions |
| `Entry`, `Class`, `Difficulty` | Menu and mode data |
| `LoadXML` | Includes another XML file |
| `TagInput` | Pulse and Pure's own text-entry widget; see below |

## `TagInput`: the row of character cells, and it is not a keyboard

Neither PSP title hands text entry to a key grid or the firmware OSK - all
four `sceUtilityOsk` NIDs are absent from both Pulse executables (confidence
92), and Pure links the firmware dialog yet still carries this element (a
different, uppercase-only alphabet), so linking it settles nothing about
which one it draws with. Both author a `<TagInput>` instead: a fixed row of
`length` character cells the player scrolls one glyph at a time.

```xml
<TagInput name="Name" focus="true" Transition="0.1">
<Values x="FEGlobals->TitleXOffset" y="85" length="10" color="FEGlobals->TextColor" scale="2.0"></Values>
</TagInput>
```

Measured off `Data.wad` entry hash `0xb94fe6f9` on `pulse-psp-usa.chd` (index
1083) and `pulse-psp-eu.chd` (index 1082 - **the index differs across
pressings, the hash does not**, which is why this entry has no resolved name
and is reached by hash):

| Attribute | On | Type | Notes |
| --- | --- | --- | --- |
| `name` | `TagInput` | string | Which field - `Name`, `Tag`, or the online `GameNameTag`/`GamePasswordTag`/`UsernameTag`/`PasswordTag` |
| `focus` | `TagInput` | `true`/`false`/`force` | `force` on the two online fields that must be filled before the screen proceeds |
| `Transition`/`transition` | `TagInput` | float | Case varies by instance; unread here |
| `x`, `y` | `Values` | number, `FEGlobals->` | Text position, not the cell background's |
| `length` | `Values` | integer | Cell count: `10` (Name), `3` (Tag/pilot tag), `14` (every online field) |
| `color` | `Values` | ARGB, `FEGlobals->` | Glyph colour |
| `scale` | `Values` | float | Absent on `Create Profile Setup`'s `Tag`, meaning `1.0` |
| `Encrypt` | `Values` | bool | `GamePasswordTag`/`PasswordTag` only - masks the glyphs |
| `AllowBlank` | `Values` | bool | `UsernameTag` (`false`), `PasswordTag` (`true`) |

**The cell row, the `FE_CONFIRM` item and its two gradient bars are drawn as
ordinary sibling widgets, not part of the `TagInput` schema itself**: a
`<TagInput>`'s cell backgrounds are colour-only `<Image name="BGgradient">`
tiles under a sibling `<Item OffsetX="46" OffsetY="88">` (each `x="0"`,
`x="30"`, `x="60"`... - 30 units apart, `width="28" height="25"`,
`color="0x2fffffff"`), and the confirm prompt is a `<Text name="confirm"
idstring="FE_CONFIRM">` bracketed by two `<Image>` ARGB gradients. Nothing
here couples a `TagInput`'s `length` to that pitch; a reader that wants the
cell rects reads the `Item`'s own `BGgradient` children, and mapping glyph
*i* to cell *i* by index is this project's own choice, not authored.

**Reachable via `oag_ui::screen::Screens` only from a *named* enclosing
`Screen`.** The `Name`/first `Tag` instances (profile name/tag) sit directly
under an **anonymous** `<Screen type="FE_Default">`, which
`Screens::collect` walks through without registering - see that function's
own doc for why this project does not change that rule for one widget.
`Create Profile Setup`'s own `Tag` (`length="3"`) is the one instance that
*is* under a named `Screen`, and is what `crates/ui/tests/tag_input_ground_truth.rs`
checks against the real disc. A reader after the anonymous instances (Pulse's
own profile-name/tag screens) walks `oag_tables::fexml::parse`'s tree
directly - see `oag_ui::tag_entry`.

**The alphabet is not in this file.** It is 70 bytes in `BOOT.BIN`, not the
WAD - see `oag_pulse::tag_input::ALPHABET`.

## `FEGlobals->` and `FEConst->`: named values where a number is expected

An attribute that expects a number may instead name a value held in a run-time
registry. Two spellings, and **they are the same namespace and the same
registry** - they differ only in when the name is resolved:

```xml
<Values Color="0xff8000"/>              <!-- a literal -->
<Values Color="FEConst->HiliteColor"/>  <!-- resolved once, at load -->
<Values Color="FEGlobals->HiliteColor"/><!-- re-resolved on every use -->
```

- **`FEConst->Name`** is a *snapshot*. The registry is read while the file is
  being parsed and the resulting number is baked into the widget.
- **`FEGlobals->Name`** is a *live binding*. The parser stores the name's hash
  in the field and sets a bit in the widget's flags word; every read re-resolves
  it, so changing the global changes what the widget shows.

Rules a writer or a reimplementation needs:

- **The sigil is an arrow, `->`.** Matching is case-insensitive, like the rest
  of the reader, so `feglobals->x` also works.
- **A value is only examined when it starts with `FE`** (either case).
  Everything else goes straight to the number parser, so ordinary values are
  unaffected and a leading `FE` is the only thing to avoid.
- **A value that starts with `FE` but matches neither sigil is silently
  dropped** - the destination field is left at whatever it already held. There
  is no error and no fallback to parsing it as a number.
- **Registry values are stored as text** and re-parsed on each read, so the same
  global can be read as an integer by one attribute and as a float by another.
- **The float form of this reader parses correctly**, via `strtod` - unlike the
  default float accessor noted below, it does understand `1e-5`. Which of the
  two an attribute uses depends on the widget.
- **Names are prefixed in code, not in XML.** `FEGlobals->TeamModel` is the
  entry the C++ side calls `FE_TeamModel`; the reader prepends `FE_` before
  hashing.

Widgets can also *publish* their own value as a global, which is what makes the
live form useful: one widget's state drives another widget's attribute.

Mechanism, addresses and the registry layout are on
[ps2-pulse-eu/fe-globals.md](../ghidra/functions/ps2-pulse-eu/fe-globals.md); the PSP
build has the same three-way test and the same two literals. **Which attributes
accept the indirection is not enumerated** - only `Color` on a text widget's
`<Values>` is confirmed, out of 26 call sites.

## Open questions

- **Which attributes accept `FEGlobals->` / `FEConst->`**, and the full list of
  registered global names. Only `FE_TeamModel` and `FE_ModelSkin` are confirmed
  entries.
- The full element and widget set. Each widget registers a name and a vtable in
  the binary, so the complete list is recoverable from the registration sites.
- ~~Whether the PS2 release uses the same schema.~~ It does, at least for the
  shortening: the PS2 reader parses `<code>` exactly as described above. Whether
  the *element* vocabulary matches is still open.
- How layout and anchoring are expressed.

## Recovering a start tag missing its own `>` (2026-09-25)

**Fixed.** `tag_end` (`crates/tables/src/fexml.rs`) used to have no recovery
for a `<Tag ...attr="value"</Tag>` shape - an opening tag missing the `>`
that should close it before the matching close tag's own text appears. Its
quote-tracking scan found no unquoted `>` inside the malformed attribute
list, so it kept scanning into the following `</Tag>` and treated *that* `>`
as the opening tag's own terminator - leaving the node open on the parse
stack and reparenting everything the file authors afterwards underneath it
instead of beside it. First found on Wipeout HD/Fury's own
`EndRace_Definition.xml`, which authors `<Values Src="..." ... RotY="-0.5"</Values>`
three times (`MedalModelGold`/`Silver`/`Bronze`, `EndRace Results`' own
trophy widgets) - see
[hd-endrace-screens.md](hd-endrace-screens.md#a-malformed-tag-upstream-swallowed-navigationcontroller-on-this-screen-alone---fixed-2026-09-25)
for what it silently swallowed as a result before the fix.

**The rule: an unquoted `<` met while still scanning a tag proves the
previous tag was never closed.** `tag_end` now returns as soon as it meets
one, treating the character just before it as the (missing) `>` - so
scanning resumes exactly on that `<`, which is then read as the next tag
(here, the real `</Values>`) rather than consumed as the malformed tag's own
content. This format escapes nothing and authors no raw `<` inside a value
anywhere this project has found, so a `<` met before the previous tag's own
`>` cannot be genuine content; treating it as the missing terminator only
ever recovers structure a naive scan was dropping, never invents structure
that was not authored - see the census below for why that is not merely
asserted.

**This is a chosen recovery for this project's own reader, not a
reproduction of a measured original behaviour - say so plainly.** The
original's own `Xml_NextElement` ([ps2-pulse-eu/xml-reader.md](../ghidra/functions/ps2-pulse-eu/xml-reader.md))
"scans forward for a `<` that is not `<?` or `<!`, reads the name, and then
scans to the tag's `>` - skipping a `>` preceded by `-`, which is how it
avoids stopping inside `-->`". Nothing in that read describes what happens
if a bare `<` turns up before that scan finds its `>`, and no RPCS3 capture
of `EndRace Results` specifically (or of `grid_04`'s own headrush cell,
still 2048/Omega's own campaign schema) exists to settle it by observation
either - `hd-endrace-screens.md`'s own record is explicit that "a live
RPCS3 end-of-race walk would settle it" and has not happened. So it is
equally possible that the shipped PS3/PS4/Vita game hits the identical bug
on these exact widgets and simply never showed a working `Confirm` on
`EndRace Results` or a headrush cell on `grid_04` either - a real,
if obscure, shipped defect, not something this project's reader introduced
by *not* recovering. What is not in question is the file's own authorial
intent: `grid_04.xml`'s raw bytes carry ten well-formed
`<PI_Cell name="...">...</PI_Cell>` pairs (matched open/close counts,
confirmed by direct grep on the extracted file), and `EndRace_Definition.xml`
carries a complete, well-formed `<NavigationController>` right after the
broken `<Values>` - both are things *a human authored* and a scanning
parser lost only because of a missing `>`, not because they are absent from
the disc. Recovering them is this project's own forgiving-parser
philosophy (`fexml::parse`'s own doc: "the goal is to read whatever the
shipped files contain, not to validate them") applied consistently, not
a claim about what a PS3 or Vita actually drew.

**Not a no-op, and that is itself the finding.** A scratch census
(`cargo run -p oag-game --example fexml_recovery_noop_census`, not
committed as a test - it needs every disc image and every decrypted
`data/extracted/` tree this project has) parses every fexml-candidate entry
across all eight sources this project reads (Pulse PSP EU/USA, Pulse PS2 EU,
Pure PSP EU/USA, HD, Omega, 2048/Vita) with the old pipeline (`tag_end_old`,
and - for a shortened blob - `expand_old`/`dictionary_old`, all copied
verbatim rather than reusing the crate's own now-fixed `expand`, since that
would silently run the fix on both sides of the comparison) against the
fixed one, and diffs the trees. Of **2,966 entries checked, 23 differ** -
the same authoring bug (a start tag missing its own `>`) shipped in more
places than the one screen this recovery was written for:

| File | Where | What the old tree lost |
| --- | --- | --- |
| `EndRace_Definition.xml` | HD, all 5 served copies; Omega, 1 | `NavigationController` and everything after it on `EndRace Results` (see hd-endrace-screens.md) |
| `stats_definition.xml` | HD (4 copies), Omega (1) | `MiniText`, `ScrollBar`, `Item`, `Redirect`, and two entire sibling `Screen`s |
| `grid_04.xml` | HD (3 copies), Omega, 2048/Vita | Ten real `PI_Cell` campaign-grid cells, reparented under the malformed `Values` instead of `PI_Grid`'s own children |
| `HUD_objectives.xml` | 2048/Vita | An `Image` widget's own `Values` attribute carrier |
| `SP.xml` | Omega | 2048's campaign schema (read through [`mjolnir`](../../crates/tables/src/mjolnir.rs), which builds on this same tree) |
| Pulse's "stats holder" screen | PSP EU, PSP USA, PS2 EU (`fe.wad`, 2 entries) - four entries total, all three platforms | `LeftLayer`, misnaming the first child `Text` instead |

One further instance, `Data/environments2048/tower/track.pvsxml` (Omega),
is not fexml this project actually reads at all -
[`docs/formats/README.md`](README.md)'s own census marks `.pvsxml` "not
opened or compared against" anything, so whatever the recovery does there
has no live consequence.

In every instance above (dumped and read individually, not just diffed),
the old tree lost real, named elements as descendants of the
wrongly-left-open tag; the new tree recovers them as the tree's own
authored siblings, never the other way round - the recovery only ever
*adds back* structure the old parser was dropping. **`grid_04.xml` is the
one with a live downstream reader**: `oag_tables::race_campaign` walks
`PI_Grid`'s own children looking for `PI_Cell`, so the old parser's
misparse silently under-populated a campaign grid on three titles wherever
this exact shape shipped. `crates/hd/tests/campaign_grids_ground_truth.rs`
had pinned the old, wrong zero-cell count for this exact grid as its own
ground truth, with a comment naming this precise scenario ("a future fix to
the XML reader that started tolerating the break would have to notice this
assertion") - updated in the same change as this recovery, since a red gate
here blocks every lane's own merge; see that file's own history for the
corrected counts.

Regression coverage: `crates/tables/src/fexml/tests.rs`'s
`recovers_a_start_tag_missing_its_closing_bracket`/
`expand_recovers_the_same_missing_bracket` (the exact `RotY="-0.5"` shape,
both through `parse` directly and through `expand`) and
`a_doubled_opening_bracket_does_not_panic` (a `<<` right at a tag's own
start has no content before the second `<` to recover into - guarded by
`tag_end`'s own `index > 1`, not `> 0`, for exactly this reason), plus
`crates/game/tests/hd_endrace_ground_truth.rs`'s
`hd_endrace_results_draws_its_confirm_prompt_despite_the_malformed_tag`
against the real disc.

## Parser behaviour worth knowing

From the reader itself, documented on both builds
([PSP](../ghidra/functions/psp-pulse-usa/xml-reader.md),
[PS2](../ghidra/functions/ps2-pulse-eu/xml-reader.md)) and structurally identical
between them. It applies to anything that writes these files as well as anything
that reads them:

- **Element and attribute names are matched case-insensitively.**
- **Nothing is escaped.** No entities, no CDATA; values are raw byte ranges, and
  the parser NUL-terminates each one in place inside the loaded buffer.
- **Values live in attributes.** Nothing in the reader looks at text between
  tags.
- **The default float accessor does not understand exponent notation.** `1e-5`
  parses as `-15`, silently: any character that is not a digit, `.` or `-` is
  skipped rather than ending the number. **Confirmed on both builds** -
  `0x0895379c` on the PSP, `0x00203868` on the PS2 - so this is not a
  quirk of one port. A second, `strtod`-based accessor
  (`Xml_AttributeAsFloatLibc`) exists on both, and on the PSP it is what the
  front-end widget layer actually uses; the blind one is what the handling and
  camera data go through. Never emit scientific notation into either.
- **A `<code>` key outside `a`..`r` corrupts the document.** The dictionary
  store does no range check while the two expansion sites do; the slot just past
  `r` is the document's interned-string list head. Same on both builds. No
  shipped file does it; a generated one could.
