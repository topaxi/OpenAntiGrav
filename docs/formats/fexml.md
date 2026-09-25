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
