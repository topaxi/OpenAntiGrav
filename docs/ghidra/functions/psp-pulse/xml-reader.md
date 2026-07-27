# The XML reader (PSP)

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Wipeout Pulse, PSP, UCUS-98712), image
base `0x08804000`.

This is the PSP half of the reader documented on
[ps2-pulse/xml-reader.md](../ps2-pulse/xml-reader.md), located from
`Movie_ParseAttributes`'s callees. It is the parser every data-driven subsystem
goes through: [handling stats](engine.md), the [camera blocks](camera.md), the
front end and everything else that reads
[front-end XML](../../../formats/fexml.md).

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70.

## The headline: the PSP has the same exponent-blind float parser

The PS2 page recorded that its default float accessor is hand-rolled and cannot
read scientific notation, and flagged it as PS2-only evidence because the PSP
reader had never been located. **It is not PS2-only.** `Xml_AttributeAsFloat` at
`0x0895379c` is the same routine:

```c
value = 0; scale = 1; integer = true; negative = false;
for (c = *p; c; c = *++p) {
    if (isdigit(c)) {
        if (integer) value = value * 10.0f + (c - '0') * scale;
        else       { scale *= 0.1f; value += (c - '0') * scale; }
    }
    else if (c == '.') integer = false;
    else if (c == '-') negative = true;
}
return negative ? -value : value;
```

Every character that is not a digit, `.` or `-` is **ignored rather than
terminating the scan**, exactly as on the PS2. An exponent's digits are
therefore absorbed as extra mantissa digits at whatever scale the scan has
reached, and its sign is applied to the whole number: `1e-5` parses as `-15`,
while `1.5e3` parses as `1.53`. `+3.0` parses as `3.0`. Trailing units or stray
text are silently absorbed. No overflow or precision handling.

The consumers are traced, which is what makes this actionable rather than
academic. Direct callers of `0x0895379c` include **every** handling parser -
`HandlingXml_ParsePhysical`, `ParseAntigrav`, `ParseEngine`, `ParseBrakes`,
`ParsePitch`, `ParseTurning`, `ParseAirbrake`, `ParseAirbrakeGraphics` on
[engine.md](engine.md) - and **all five** camera parsers
(`HandlingXml_ParseInternalCamera` and siblings) on [camera.md](camera.md).

So the rule stated on [handling-stats.md](../../../formats/handling-stats.md)
and [fexml.md](../../../formats/fexml.md) now rests on both builds: no handling
or camera parameter may use scientific notation, and any tool that regenerates
those files must not emit it. Confidence **95** - unambiguous decompilation, a
second binary agreeing instruction-for-instruction in behaviour, and the
consumers traced rather than assumed.

### The second, correct accessor exists here too

`Xml_AttributeAsFloatLibc` (`0x089538a4`) is a real parser: it calls
`atof` (`0x08972730`, a `strtod(s, NULL)` wrapper) and narrows the `double` to
`float` before storing. It has well over a hundred call sites; the ~120 sampled
were all in the front-end widget layer (`0x0888...`-`0x088d...`) and none was a
handling or camera loader. The cross-reference list was capped, so read that as
"none seen" rather than a verified absence - what *is* verified, from a complete
listing, is the other direction: every handling and camera parser calls the
blind accessor.

That is the same split as the PS2: two accessors with different numeric
grammars in one parser, gameplay data on the blind one and the front end on the
correct one. Do not assume a value read by one is read the same way by the
other.

This function is only visible in the disassembly - Ghidra had not created a
function at `0x089538a4`, and the same is true of `Xml_AttributeValueIs` at
`0x08953764`. Both sit in what looks like a gap between defined functions.

## What kind of parser it is

Identical in shape to the PS2's: a **scanning parser over the raw text**, no
tree, no tokeniser, no allocation per node. A cursor holds a `begin`/`end` pair
into the loaded file buffer and re-scans on every step.

The same three consequences apply, each read off the PSP code rather than
carried over:

- **It edits the file buffer.** `Xml_NextAttribute` writes a NUL over the
  closing quote of every attribute value it visits.
- **Nothing is escaped.** No entities, no CDATA, no value normalisation.
- **Comparisons are case-insensitive.** `Xml_ElementNameIs`,
  `Xml_AttributeNameIs` and `Xml_AttributeValueIs` are all
  `strcasecmp(...) == 0`.

## The document

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08953ddc` | `Xml_OpenFile` | 92 |
| `0x08953cdc` | `Xml_InitDocument` | 88 |
| `0x08954014` | `Xml_InternString` | 85 |
| `0x08953fc0` | `Xml_CloseDocument` | 78 |

`Xml_InitDocument`'s three `memset` calls reproduce the PS2 document layout
exactly, which is the strongest single piece of evidence that both builds
compile the same source:

| Offset | Field |
| --- | --- |
| `+0x04` | the loaded resource, or 0 |
| `+0x08` | buffer begin |
| `+0x0c` | buffer end |
| `+0x10` | the file's path, 256 bytes |
| `+0x110` | the file's *directory*, 256 bytes: the path with everything from the last `\` cut off |
| `+0x210` | **the short-name dictionary**: 18 `char *` |
| `+0x258` | head of a linked list of strings the document owns |

`Xml_OpenFile(doc, path)` records the path, cuts the directory copy at the last
`\` with `strrchr`, loads the file through `Resource_LoadFile` (`0x08943330`,
see [vex.md](../../../formats/vex.md)) with flags `1`, takes `begin` from the
resource's `+0x04` and `end` from `begin + [+0x08]`, and then reads the
dictionary. Success returns 0, failure returns 1 - so callers test `== 1`, same
as the PS2. Unlike the PS2 it prints nothing on failure; there is no
`ERROR : Can't open file` string in this binary.

`Xml_InternString` allocates `strlen(s) + 5`, links the node onto `doc+0x258`
and returns the byte after the link word - the identical `+0x258` list head the
PS2 has.

`Xml_CloseDocument` (`0x08953fc0`) releases the resource through its vtable and
nulls `doc+0x04`. It does **not** walk the `+0x258` list, where the PS2's
counterpart does; and it has exactly one caller. Scored **78** for that reason:
the body is unambiguous, but this may be one step of a teardown whose other half
lives in a destructor that was not located.

## The `<code>` dictionary, read off the PSP reader too

After loading, `Xml_OpenFile` takes the root element and, if it is named
`"code"` (the literal is at `0x08a8a580`), walks its attributes:

```c
while (Xml_NextAttribute(&elem, &attr)) {
    value = Xml_GetAttributeValue(&attr);
    name  = Xml_GetAttributeName(&attr);
    doc[name[0] * 4 + 0x8c] = value;      /* 0x8c + 'a'*4 == 0x210 */
}
```

Every property [fexml.md](../../../formats/fexml.md) records from the data, and
that the PS2 reader confirmed, is confirmed a third time here:

- **The key is the first character of the attribute name only**, so `as="Values"`
  keys on `a`.
- **The table has 18 slots.** Both expansion sites guard with
  `0x60 < c && c < 0x73`, i.e. `a` through `r`.
- **The dictionary lives in the document**, so it is per file by construction.
- The value pointer is stored **into the file buffer**, not copied.
- **A `<code>` key outside `a`..`r` writes past the table.** The store does no
  range check while both loads do, and the slot one past `r` (`c == 's'`) is
  `+0x258`, the interned-string list head. Same trap, same slot, on both builds.
- **A two-character short name is not expanded**: the test is `name[1] == 0`.

Expansion happens in `Xml_NextElement` and `Xml_NextAttribute` with the same
three-line test as the PS2, so everything downstream sees long names.

## Elements

| Address | Name | Conf |
| --- | --- | ---: |
| `0x089540a4` | `Xml_NextElement` | 92 |
| `0x08953a88` | `Xml_ElementNameIs` | 92 |
| `0x08953a50` | `Xml_FirstChildElement` | 90 |
| `0x08953f78` | `Xml_GetRootElement` | 90 |
| `0x08953a14` | `Xml_InitElement` | 85 |
| `0x08953cd0` | `Xml_SetElementRange` | 82 |
| `0x08953a24` | `Xml_DestroyElement` | 75 |

The element cursor is **160 bytes** on the caller's stack - the
`undefined1 auStack_4c0 [160]` locals throughout the front end and the handling
loader - with the PS2's field layout unchanged:

| Offset | Field |
| --- | --- |
| `+0x00` | the element's name, NUL-terminated, expanded |
| `+0x80` | scan range begin |
| `+0x84` | scan range end |
| `+0x88` | just past the element's name: where its attributes start |
| `+0x8c` | the element's end |
| `+0x90` | cursor, as an offset from `+0x80` |
| `+0x94` | set if the element has attributes |
| `+0x98` | attribute scan pointer |
| `+0x9c` | the owning document |

`Xml_NextElement` scans for a `<` that is not `<?` or `<!`, reads the name as
`[A-Za-z0-9_]+`, records `+0x88`, scans to the tag's `>` while skipping a `>`
preceded by `-` (so it does not stop inside `-->`), and either stops at a
self-closing `/>` or walks a depth counter to the matching close tag. Because
the cursor bounds a whole subtree, `Xml_FirstChildElement` is four assignments
and a call.

`Xml_DestroyElement(elem, flags)` frees only when bit 0 of `flags` is set, which
is why every call site passes `2` - a C++ deleting destructor, and scored **75**
for the same reason as on the PS2: the shape could belong to a sibling class.

## Attributes

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0895379c` | `Xml_AttributeAsFloat` | 95 |
| `0x08953b0c` | `Xml_NextAttribute` | 92 |
| `0x0895372c` | `Xml_AttributeNameIs` | 92 |
| `0x08953930` | `Xml_AttributeAsBool` | 92 |
| `0x089538dc` | `Xml_AttributeAsString` | 90 |
| `0x08953ac0` | `Xml_BeginAttributes` | 90 |
| `0x08953764` | `Xml_AttributeValueIs` | 88 |
| `0x08953878` | `Xml_AttributeAsInt` | 88 |
| `0x089538a4` | `Xml_AttributeAsFloatLibc` | 85 |
| `0x08953904` | `Xml_AttributeAsIntHex` | 85 |
| `0x089539d8` | `Xml_AttributeAsOwnedString` | 85 |
| `0x08953868` | `Xml_GetAttributeValue` | 85 |
| `0x08953650` | `Xml_ParseIntHexOrDecimal` | 82 |
| `0x08953870` | `Xml_GetAttributeName` | 78 |

The attribute object is **1032 bytes** - the `auStack_420 [1032]` local in every
parser on [engine.md](engine.md) and [camera.md](camera.md):

| Offset | Field |
| --- | --- |
| `+0x000` | the attribute's name, up to 1024 bytes, NUL-terminated, expanded |
| `+0x400` | pointer to the value, in place in the file buffer |
| `+0x404` | the owning element |

Iteration is two calls with the state in the *element*:

```c
if (Xml_BeginAttributes(&elem)) {
    while (Xml_NextAttribute(&elem, &attr)) {
        if (Xml_AttributeNameIs(&attr, "gain")) { ... }
    }
}
```

`Xml_BeginAttributes` points `+0x98` at `+0x88`, skips spaces and returns false
if the first non-space is `>`. `Xml_NextAttribute` stops at `>` or `/`, skips
spaces and tabs, copies the name up to `=` (capped at 1024), applies the
dictionary, finds the opening `"`, records the value pointer, scans to the
closing `"` and writes a NUL over it.

`Xml_GetAttributeName` (`0x08953870`) is literally `return param_1` - the name
is at offset 0 - so it is nameable only from its role in `Xml_OpenFile`'s
`<code>` loop. Hence **78**.

### The other accessors

- `Xml_AttributeAsBool` (`0x08953930`) is true when the value is one of `"1"`,
  `"on"`, `"yes"`, `"unlocked"` or `"true"`, compared case-insensitively -
  read out of `.rodata` at `0x08a8a558`..`0x08a8a56c`. **The same five
  spellings, including `"unlocked"`, as the PS2.** Anything else, including
  `"0"`, is false.
- `Xml_AttributeAsInt` (`0x08953878`) is `atoi`, and `Xml_AttributeAsIntHex`
  (`0x08953904`) goes through `Xml_ParseIntHexOrDecimal` (`0x08953650`), which
  accepts a `0x` prefix - it tests `value[1] == 'x'` and accumulates from the
  right through the hex-digit table at `0x08ac22e8` - and otherwise falls back
  to `atoi`.
- `Xml_AttributeAsString` (`0x089538dc`) is `strncpy(dst, value, n - 1)`.
- `Xml_AttributeAsOwnedString` (`0x089539d8`) interns the value into the
  document, so the result lives until the document is closed.
- Unlike the PS2's, all of these except `Xml_AttributeAsFloat` write through an
  out-parameter rather than returning a value. Only a codegen difference; the
  grammars are the same.

`Xml_ParseIntHexOrDecimal` keeps its `Xml_` prefix on evidence: its three
callers are all XML value parsers. Besides `Xml_AttributeAsIntHex`, `0x08888bd8`
reads an attribute as a string and dispatches on a `"FE"` prefix
(`0x08a7d2c0`) - `FEGlobals->` (`0x08a7d2a8`) or `FEConst->` (`0x08a7d2b4`)
resolves an indirection, anything else parses as a literal number - and
`0x08889048` resolves the constant. Both literals were read out of this binary,
not carried over from the PS2. So the PSP front end has **the same
`value="FEGlobals->Something"` indirection**, now documented on
[ps2-pulse/fe-globals.md](../ps2-pulse/fe-globals.md) and in
[fexml.md](../../../formats/fexml.md). The PSP functions are still **not
renamed**: their registry and float variant were not traced here.

## Library routines this family pins down

Named here because the reader is what fixes them, and because
[imports.md](imports.md) covers only the NID-resolved import stubs, not
statically linked libc:

| Address | Name | Conf |
| --- | --- | ---: |
| `0x089732d8` | `strcasecmp` | 92 |
| `0x08973770` | `strrchr` | 90 |
| `0x0897274c` | `atoi` | 90 |
| `0x089792a0` | `strtol` | 85 |
| `0x08972730` | `atof` | 88 |
| `0x08974930` | `strtod` | 82 |

`strcasecmp` folds case through the `_ctype_` table at `0x08a90d20` already
named on [wad-subsystem.md](wad-subsystem.md) (bit 0 = upper case, add `0x20`).
`atoi` is `strtol(s, 0, 10)` and `atof` is `strtod(s, 0)`, both read directly;
the `strtol`/`strtod` bodies themselves were not read, hence the lower scores on
those two.

## What this settles on frontend-video.md

Three things fall out of reading `Movie_ParseAttributes` (`0x088ba284`) against
the reader, and [frontend-video.md](frontend-video.md) has been corrected:

- **`<Movie>` does require a `Values` element.** The parser's first act is
  `Xml_ElementNameIs(elem, "Values")` and it returns 0 if that fails. That was
  an open question there.
- **The widget does not use `Xml_AttributeAsBool`.** It reads each flag with
  `Xml_AttributeAsString` into a 64-byte stack buffer and runs its own
  `strcasecmp`. So the two-spelling booleans there and the five-spelling shared
  accessor here are not a difference between the builds - it is a widget rolling
  its own comparison. That resolves the third open item on the PS2 page.
- **`sound` is narrower than its siblings**, accepting only `"true"`;
  `autoredirect`, `autostart`, `repeat` and `stoppowersave` accept `"true"` or
  `"1"`.
- The `localised` rule was recorded backwards. It is
  `localised == "true"` → `_US.PMF`, absent or anything else → `.PMF`.

## Not determined

- **`Xml_OpenMemory`.** The PS2 has one (`0x00203058`) that loads a document
  from a buffer without reading `<code>`, and it sits immediately *before*
  `Xml_ParseIntHexOrDecimal` at the head of the PS2 block. The mirror position
  on the PSP is occupied: the unrelated `FUN_08953324` ends at `0x0895364f`,
  exactly where `Xml_ParseIntHexOrDecimal` begins. So if the PSP has one it is
  not adjacent to its siblings. The rest of the block
  (`0x08953650`..`0x08954437`) was walked for gaps, but only six functions'
  extents were measured exactly - the rest is inference from instruction counts,
  and that is precisely the reasoning that hid `Xml_AttributeValueIs` and
  `Xml_AttributeAsFloatLibc` until the gaps were disassembled.
- **Where the `+0x258` interned-string list is freed**, given
  `Xml_CloseDocument` does not do it and has one caller.
- **Element text content.** Nothing in this family reads text between tags, on
  either build.
- **Nothing here was verified at runtime.**

## Cross-platform

Every row the PS2 page had to leave as "not located" is filled in.

| Function | PSP (`BOOT.BIN`) | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| `Xml_OpenFile` | `0x08953ddc` | `0x00202f10` |
| `Xml_InitDocument` | `0x08953cdc` | `0x00202d48` |
| `Xml_CloseDocument` | `0x08953fc0` | `0x00202da8` |
| `Xml_InternString` | `0x08954014` | `0x00202e88` |
| `Xml_OpenMemory` | not located | `0x00203058` |
| `Xml_GetRootElement` | `0x08953f78` | `0x00203098` |
| `Xml_ParseIntHexOrDecimal` | `0x08953650` | `0x002030e0` |
| `Xml_InitElement` | `0x08953a14` | `0x002031c0` |
| `Xml_DestroyElement` | `0x08953a24` | `0x002031d0` |
| `Xml_SetElementRange` | `0x08953cd0` | `0x002031f8` |
| `Xml_NextElement` | `0x089540a4` | `0x00203208` |
| `Xml_ElementNameIs` | `0x08953a88` | `0x00203538` |
| `Xml_FirstChildElement` | `0x08953a50` | `0x00203568` |
| `Xml_BeginAttributes` | `0x08953ac0` | `0x002035a8` |
| `Xml_NextAttribute` | `0x08953b0c` | `0x00203610` |
| `Xml_AttributeNameIs` | `0x0895372c` | `0x00203808` |
| `Xml_AttributeValueIs` | `0x08953764` | `0x00203830` |
| `Xml_GetAttributeValue` | `0x08953868` | `0x00203858` |
| `Xml_GetAttributeName` | `0x08953870` | `0x00203860` |
| `Xml_AttributeAsFloat` | `0x0895379c` | `0x00203868` |
| `Xml_AttributeAsInt` | `0x08953878` | `0x00203940` |
| `Xml_AttributeAsFloatLibc` | `0x089538a4` | `0x00203970` |
| `Xml_AttributeAsBool` | `0x08953930` | `0x002039a0` |
| `Xml_AttributeAsString` | `0x089538dc` | `0x00203a58` |
| `Xml_AttributeAsOwnedString` | `0x089539d8` | `0x00203a80` |
| `Xml_AttributeAsIntHex` | `0x08953904` | `0x00203ab8` |
| `strcasecmp` / `strrchr` | `0x089732d8` / `0x08973770` | - / `0x0025c528` |
| `atoi` / `atof` | `0x0897274c` / `0x08972730` | `0x00110028` / `0x0025b9b0` |

The link order differs - the PS2 emits documents, elements, attributes; the PSP
emits attributes, elements, documents - but every function has a counterpart and
the two document and element layouts are byte-for-byte the same. That is a
twenty-plus-function structural match, and it joins the handling parameter
block, the camera block and the WAD hash on the list of things the two builds
share unchanged.

## History

- 2026-07-27: first pass, anchored from `Movie_ParseAttributes`'s callees. The
  finding worth carrying forward is that the exponent-blind float parser is
  **not** PS2-only: it is the accessor both handling and camera data go through
  on the PSP as well. `Xml_AttributeAsFloatLibc` and `Xml_AttributeValueIs` were
  initially believed absent because Ghidra had defined no function at their
  addresses; they were found by disassembling the apparent gaps.
