# The XML reader (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

This is the parser every data-driven subsystem in the game goes through:
[handling stats](handling-xml.md), the front end, and whatever else reads
[front-end XML](../../../formats/fexml.md). `Xml_AttributeNameIs` alone has 750
cross-references, so naming this family puts a readable name on roughly a
thousand call sites at once.

It is also, unexpectedly, the best evidence in the tree for
[fexml.md](../../../formats/fexml.md)'s short-name dictionary: the `<code>`
element and the 18-slot expansion table are not an inference from the data here,
they are read off the reader.

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70.

## What kind of parser it is

A **scanning parser over the raw text**, with no tree and no allocation per
node. There is no tokeniser and no document model: a cursor object holds a
`begin`/`end` pair into the loaded file buffer and re-scans the text on every
step. Iteration is therefore O(n) per call and O(n^2) over a document, which
does not matter for files this size and explains why the code looks the way it
does.

Three consequences a reimplementation should know:

- **It edits the file buffer.** `Xml_NextAttribute` writes a NUL over the
  closing quote of every attribute value it visits, so the value can be returned
  as a bare `char *` into the buffer. The loaded blob is not reusable as text
  afterwards.
- **Nothing is escaped.** No entity handling (`&amp;` and friends), no CDATA, no
  attribute-value normalisation. Values are byte ranges.
- **Comparisons are case-insensitive.** Both `Xml_ElementNameIs` and
  `Xml_AttributeNameIs` are `strcasecmp(...) == 0`, so `<Screen>` and `<screen>`
  are the same element, and `name=` and `Name=` are the same attribute.

## The document

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00202f10` | `Xml_OpenFile` | 92 |
| `0x00202d48` | `Xml_InitDocument` | 88 |
| `0x00202da8` | `Xml_CloseDocument` | 85 |
| `0x00202e88` | `Xml_InternString` | 85 |
| `0x00203058` | `Xml_OpenMemory` | 80 |

The document object, from `Xml_InitDocument`'s three `memset` calls and every
field's use:

| Offset | Field |
| --- | --- |
| `+0x04` | the loaded resource, or 0 |
| `+0x08` | buffer begin |
| `+0x0c` | buffer end |
| `+0x10` | the file's path, 256 bytes |
| `+0x110` | the file's *directory*, 256 bytes: the path with everything from the last `\` cut off |
| `+0x210` | **the short-name dictionary**: 18 `char *`, indexed by `name[0] - 'a'` |
| `+0x258` | head of a linked list of strings the document owns |

`Xml_OpenFile(doc, path)` records the path and its directory, loads the file
through `Resource_Load` (`0x001fd6b8`, the name-hashed resource registry - see
[wad-subsystem.md](wad-subsystem.md)), takes `begin`/`end` from the resource's
buffer pointer and length, and then reads the dictionary. On failure it prints
`ERROR : Can't open file "%s"` (`0x002c1170`) and **returns 1**; success returns
0, which is why every caller tests `== 1` for failure rather than truthiness.

`Xml_CloseDocument` releases the resource and walks the `+0x258` list freeing
every interned string, so `Xml_AttributeAsOwnedString`'s results live exactly as
long as the document.

### The resource registry underneath

Reading `Xml_OpenFile` settled two of the candidates
[the README](README.md) previously listed at confidence 60, so they are named
here rather than left as hypotheses:

| Address | Name | Conf |
| --- | --- | ---: |
| `0x001fd6b8` | `Resource_Load` | 80 |
| `0x001fd618` | `Resource_Find` | 75 |

`Resource_Load(name, flags, unused)` looks the name up with `Resource_Find`
first; on a miss it opens the path through the VFS, rejects a zero-length file
(`File Size 0`, `0x002c0510`) or a missing one
(`ERROR : Unable to find "%s"`, `0x002c0520`), allocates a 64-byte-aligned
buffer rounded up to a 32-byte multiple, reads the whole file into it, and
links a 40-byte node `{buffer, buffer, size, hash, name, flags, ..., refcount}`
into the registry rooted at `0x00284840`. `Resource_Find` walks that list
comparing `Resource_HashName` of the requested name against each node's `+0x0c`
and bumps the refcount on a hit.

`Xml_OpenFile` reads `+0x04` as the buffer and `+0x08` as the length, which is
what fixes those two node fields. Confidence **80** / **75**: the flow is
unambiguous but no consumer was traced beyond this one, and the flag word's
meanings are unread.

## The `<code>` dictionary, read off the reader

This is the part worth the whole page.
[fexml.md](../../../formats/fexml.md) describes the short-name scheme from the
data: a `<code>` element at the top of a file maps one- or two-letter codes to
real names, and the dictionary is **per file** because the same code means
different things in different files. All of that is directly visible here.

After loading, `Xml_OpenFile` scans the first element and, if it is named
`"code"` (the literal is at `0x002c1190`), walks its attributes:

```c
while (Xml_NextAttribute(&elem, &attr)) {
    name  = Xml_GetAttributeName(&attr);      /* e.g. "as" */
    value = Xml_GetAttributeValue(&attr);     /* e.g. "Values" */
    doc[(name[0] - 'a') * 4 + 0x210] = value;
}
```

So:

- **The key is the first character of the attribute name only.** `as="Values"`
  keys on `a`; the trailing `s` is not read. That confirms fexml.md's reading of
  `Xs="Name"` as "short name `X` maps to `Name`".
- **The table has exactly 18 slots** and the expansion sites guard with
  `c - 'a' < 0x12`, so codes run `a` through `r`. fexml.md independently counted
  "18 short codes" across `FEData.wad`. Two unrelated routes to the same 18.
- **The dictionary lives in the document**, so it is per file by construction. A
  shared table is not merely wrong, it is not expressible.
- The value pointer is stored **into the file buffer**, not copied, which is why
  the dictionary cannot outlive the document.

Expansion then happens at both levels, in `Xml_NextElement` and
`Xml_NextAttribute`, with the same three-line test: if the name just scanned is
exactly one character and that character is in `a`..`r` and the slot is
non-empty, `strcpy` the expansion over it. Everything downstream therefore sees
long names and never learns the file was shortened.

Two notes for anyone regenerating these files:

- **A two-character short name is not expanded.** The test is `name[1] == 0`.
  fexml.md's example dictionary uses two-character *attribute* names in `<code>`
  (`as=`, `bs=`) but the codes used in the body are one character; that
  asymmetry is required, not stylistic.
- **`Xml_OpenMemory` does not read `<code>`.** A shortened document parsed from
  memory rather than through the resource registry keeps its short names. Only
  `Xml_OpenFile` builds the dictionary.
- **A `<code>` key outside `a`..`r` writes past the table.** The store does no
  range check while every load does, and slot 18 is the `+0x258` interned-string
  list head. No shipped file does it; a hand-authored one could.

## Elements

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00203208` | `Xml_NextElement` | 92 |
| `0x00203538` | `Xml_ElementNameIs` | 92 |
| `0x00203568` | `Xml_FirstChildElement` | 90 |
| `0x00203098` | `Xml_GetRootElement` | 90 |
| `0x002031c0` | `Xml_InitElement` | 85 |
| `0x002031f8` | `Xml_SetElementRange` | 82 |
| `0x00203560` | `Xml_GetElementName` | 78 |
| `0x002031d0` | `Xml_DestroyElement` | 75 |

The element cursor is **160 bytes** on the caller's stack - which is what the
`undefined1 auStack_1e0 [160]` locals all over the front end and the handling
loader are:

| Offset | Field |
| --- | --- |
| `+0x00` | the element's name, NUL-terminated, expanded |
| `+0x80` | scan range begin |
| `+0x84` | scan range end |
| `+0x88` | just past the element's name: where its attributes start |
| `+0x8c` | the element's end: the `>` of a self-closing tag, or the matching `</name` |
| `+0x90` | cursor, as an offset from `+0x80` |
| `+0x94` | set if the element has attributes |
| `+0x98` | attribute scan pointer |
| `+0x9c` | the owning document |

`Xml_NextElement` is the engine. It scans forward for a `<` that is not `<?` or
`<!`, reads the name as `[A-Za-z0-9_]+`, records `+0x88`, and then scans to the
tag's `>` - skipping a `>` preceded by `-`, which is how it avoids stopping
inside `-->`. If the character before `>` is `/` the element is self-closing and
it is done. Otherwise it walks forward maintaining a depth counter (`<name` +1,
`/>` -1, `</` matched against the stored name) until the **matching** close tag,
and sets `+0x8c` there. So the cursor bounds a whole subtree, which is what
makes `Xml_FirstChildElement` a two-line function:

```c
child.begin = parent.attributes;   /* +0x88, i.e. just inside the open tag */
child.end   = parent.end;          /* +0x8c */
child.cursor = 0;
child.document = parent.document;
return Xml_NextElement(&child);
```

`Xml_GetRootElement(doc, elem)` is the same with the document's whole buffer as
the range. `Xml_DestroyElement(elem, flags)` is a C++ deleting destructor: it
frees only when bit 0 of `flags` is set, which is why every call site in the
game passes `2`.

Confidence **92** on `Xml_NextElement` from an unambiguous read of the scan and
its depth counter; **75** on `Xml_DestroyElement`, which is a two-line
compiler-generated destructor that could belong to something else in the class
hierarchy.

## Attributes

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00203610` | `Xml_NextAttribute` | 92 |
| `0x00203808` | `Xml_AttributeNameIs` | 92 |
| `0x00203868` | `Xml_AttributeAsFloat` | 92 |
| `0x002039a0` | `Xml_AttributeAsBool` | 92 |
| `0x002035a8` | `Xml_BeginAttributes` | 90 |
| `0x00203830` | `Xml_AttributeValueIs` | 90 |
| `0x00203a58` | `Xml_AttributeAsString` | 90 |
| `0x00203940` | `Xml_AttributeAsInt` | 88 |
| `0x00203a80` | `Xml_AttributeAsOwnedString` | 85 |
| `0x00203ab8` | `Xml_AttributeAsIntHex` | 85 |
| `0x00203858` | `Xml_GetAttributeValue` | 85 |
| `0x002030e0` | `Xml_ParseIntHexOrDecimal` | 82 |
| `0x00203970` | `Xml_AttributeAsFloatLibc` | 78 |
| `0x00203860` | `Xml_GetAttributeName` | 78 |

The attribute object is **1032 bytes**, which is the `auStack_440 [1040]` local
in every parser on [handling-xml.md](handling-xml.md):

| Offset | Field |
| --- | --- |
| `+0x000` | the attribute's name, up to 1024 bytes, NUL-terminated, expanded |
| `+0x400` | pointer to the value, in place in the file buffer |
| `+0x404` | the owning element |

The iteration protocol is two calls, and the state lives in the *element*:

```c
if (Xml_BeginAttributes(&elem)) {          /* rewind; false if the tag has none */
    while (Xml_NextAttribute(&elem, &attr)) {
        if (Xml_AttributeNameIs(&attr, "gain")) { ... }
    }
}
```

`Xml_BeginAttributes` points `+0x98` at `+0x88`, skips spaces, and returns false
if the first non-space is `>`. `Xml_NextAttribute` stops at `>` or `/`, skips
spaces and tabs, copies the name up to `=`, applies the dictionary, finds the
opening `"`, records the value pointer, scans to the closing `"` and **writes a
NUL over it**.

### `Xml_AttributeAsFloat` does not understand exponents

`0x00203868` is hand-rolled, and this is the one thing on this page that could
silently change a shipped number:

```c
value = 0; scale = 1; integer = true; negative = false;
for (c = *p; c; c = *++p) {
    if (isdigit(c)) {
        if (integer) value *= 10.0f; else scale *= 0.1f;
        value += (c - '0') * scale;
    }
    else if (c == '.') integer = false;
    else if (c == '-') negative = true;
}
return negative ? -value : value;
```

Every character that is not a digit, `.` or `-` is **ignored rather than
terminating the scan**. So `1e-5` parses as digits `1` and `5` with a `-` seen
somewhere, giving `-15`. `+3.0` parses as `3.0`. Trailing units or stray text
are silently absorbed. There is no overflow or precision handling.

**Everything on [handling-xml.md](handling-xml.md) goes through this
function**, so no handling parameter may use scientific notation, and any tool
that regenerates `HandlingStats.xml` must not emit it. Confidence **92**: read
straight out of an unambiguous decompilation.

There is a **second, correct float accessor**, `Xml_AttributeAsFloatLibc`
(`0x00203970`, 33 call sites), which goes through `atof` (`0x0025b9b0`, a
`strtod(s, NULL)` wrapper). Two accessors with different numeric grammars in one
parser is a trap worth knowing about before comparing two subsystems' values.

### The other accessors

- `Xml_AttributeAsBool` (`0x002039a0`) is true when the value is one of `"1"`,
  `"on"`, `"yes"`, `"true"` or **`"unlocked"`** - read out of `.rodata` at
  `0x002c11a8`..`0x002c11c8`, case-insensitively. Anything else, including
  `"0"`, is false. The fifth spelling is a nice piece of evidence that this is
  the parser the unlock/progression data goes through.
- `Xml_AttributeAsInt` (`0x00203940`) is `atoi`. `Xml_AttributeAsIntHex`
  (`0x00203ab8`) accepts `0x`-prefixed hex - it checks `value[1] == 'x'` and
  accumulates from the right through the hex-digit table at `0x002849c8` -
  and otherwise falls back to `atoi`.
- `Xml_AttributeAsString` (`0x00203a58`) is `strncpy(dst, value, n - 1)` into a
  caller buffer.
- `Xml_ParseIntHexOrDecimal` (`0x002030e0`) keeps its `Xml_` prefix on evidence
  rather than by association: all three of its callers are XML value parsers.
  Besides `Xml_AttributeAsIntHex`, `0x0017c630` reads an attribute as a string
  and then resolves it as either a literal number or an `FEGlobals>` /
  `FEConst>` indirection, and `0x0017c9f8` resolves the constant that lookup
  returns. So the front end can write `value="FEGlobals>Something"` where a
  number is expected - a schema feature [fexml.md](../../../formats/fexml.md)
  does not mention. Those two are **not renamed**: the indirection is clear but
  what the two tables at `0x00301988` hold was not read.
- `Xml_AttributeAsOwnedString` (`0x00203a80`) copies the value into an
  allocation owned by the document (`Xml_InternString`) and hands back a pointer
  that stays valid until `Xml_CloseDocument`. Callers store it as a plain
  `char *`.

## What this changes elsewhere

**[handling-xml.md](handling-xml.md) needed a correction.** That page described
`HandlingXml_ParseGlobal` as writing "per-class scalars" to five `.bss` arrays.
Three of them (`0x0033c340`, `0x0033c350`, `0x0027e828`) are indeed floats via
`Xml_AttributeAsFloat`, but `0x0033c330` and `0x0033c320` are written by
`Xml_AttributeAsOwnedString` and hold **`char *`, not numbers**. Corrected
there.

**[fexml.md](../../../formats/fexml.md)** gains a second, independent line of
evidence for the short-name scheme, and an answer to one of its open questions:
the PS2 release uses the same schema, because the same reader reads `<code>` the
same way. Noted there.

## Not determined

- **The PSP equivalent.** The generic reader is not documented on the PSP side,
  so every Cross-platform row below is "not located". What *is* documented is a
  **consumer**: `Movie_ParseAttributes` (`0x088ba284`, confidence 95, on
  [frontend-video.md](../psp-pulse/frontend-video.md)), which reads the `<Movie>`
  widget's nine attributes. That makes finding the PSP reader cheap - its
  callees are the attribute accessors - and the `"code"` literal plus a
  160-byte stack cursor are the other two anchors.

  Worth doing, for two reasons. It would say whether the PSP's
  `Handling_ParseStats` shares the exponent-blind float parser, so **the claim
  above about handling parameters applies to the PS2 build only** until then.
  And `frontend-video.md` records that `<Movie>`'s booleans are true for
  `"true"` or `"1"`, where `Xml_AttributeAsBool` here accepts five spellings -
  which is either a difference between the builds, a difference between a
  widget's own comparison and the shared accessor, or simply the two spellings
  that page happened to record. Nothing here distinguishes those.
- **Whether `Xml_AttributeAsFloatLibc`'s callee is really `strtod`.** `atof`
  (`0x0025b9b0`) is named from its `(string, NULL)` call shape and from its
  result being consumed as a float. The implementation behind it was not read,
  so it is scored at 75 rather than higher.
- **Element text content.** Nothing in this family reads the text *between*
  tags. Every value in this game's XML appears to be an attribute; that is a
  strong impression from the reader's shape, not a verified negative.
- **Nothing here was verified at runtime.**

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Xml_OpenFile` | `0x00202f10` | not located |
| `Xml_NextElement` | `0x00203208` | not located |
| `Xml_FirstChildElement` | `0x00203568` | not located |
| `Xml_ElementNameIs` | `0x00203538` | not located |
| `Xml_BeginAttributes` | `0x002035a8` | not located |
| `Xml_NextAttribute` | `0x00203610` | not located |
| `Xml_AttributeNameIs` | `0x00203808` | not located |
| `Xml_AttributeAsFloat` | `0x00203868` | not located |
| the reader's consumers | many | nearest documented is `Movie_ParseAttributes` `0x088ba284` |
| `Resource_Load` | `0x001fd6b8` | closest is `Resource_LoadFile` `0x08943330` |
| `strncpy` / `strcpy` / `memset` | `0x0010efc0` / `0x0010ed60` / `0x0010eb78` | not located |
| `strrchr` / `atoi` / `atof` | `0x0025c528` / `0x00110028` / `0x0025b9b0` | not located |

## History

- 2026-07-27: first pass. 92 on the scanner, the attribute iterator and the two
  name comparisons, from unambiguous decompilation corroborated by the `<code>`
  dictionary matching [fexml.md](../../../formats/fexml.md)'s independently
  counted 18 codes. The exponent-blind float parser and the
  `Xml_AttributeAsOwnedString` correction to
  [handling-xml.md](handling-xml.md) are the findings worth carrying forward.
