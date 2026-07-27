# Front-end globals: `FEGlobals->` and `FEConst->`

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

[xml-reader.md](xml-reader.md) noticed that `0x0017c630` reads an attribute as a
string and then resolves it as either a literal number or an indirection, and
left it there: "what the two tables at `0x00301988` hold was not read". This
page reads it.

It is a **named-variable mechanism for [front-end XML](../../../formats/fexml.md)**:
a widget attribute that expects a number may instead name a value held in a
run-time registry, and the schema has two spellings that differ only in *when*
the name is resolved.

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70.

## The one thing to take away

`FEConst->` and `FEGlobals->` are **the same namespace and the same registry**.
Both derive the key identically and both look in `g_fe_globals`
(`0x00301988`). They differ only in when the lookup happens:

| Spelling | Resolved | Stored in the field |
| --- | --- | --- |
| `FEConst->Name` | at **parse** time, once | the resolved number |
| `FEGlobals->Name` | at **every use** | the *hash*, plus a bit in the widget's flags word |
| anything else | - | the literal, parsed as a number |

So `FEConst->` is a snapshot and `FEGlobals->` is a live binding. That is the
whole distinction; there is no second table and no separate constant pool.

**The reimplementation trap** follows directly: on the `FEGlobals->` path the
field holds a 32-bit *hash*, in the same slot that otherwise holds an `int` or a
`float`. A consumer that forgets to test the flag bit does not read a sensible
default - it reads a hash reinterpreted as a number. The bit is not an
optimisation hint, it is a tag.

## The two attribute readers

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0017c630` | `Xml_AttributeAsIntOrGlobalRef` | 90 |
| `0x0017c710` | `Xml_AttributeAsFloatOrGlobalRef` | 90 |

Both have the identical shape, differing only in how the literal branch parses
and which accessor the `FEConst->` branch calls:

```c
void Xml_AttributeAsIntOrGlobalRef(u32 *dest, XmlAttribute *attr,
                                   u32 *flags, u32 bit)
{
    char buf[256];
    Xml_AttributeAsString(attr, buf, 0xff);

    if (strncasecmp(buf, "FE", 2) != 0) {
        *dest = Xml_ParseIntHexOrDecimal(buf);           /* literal */
    } else if (strncasecmp(buf, "FEGlobals->", 11) == 0) {
        *dest  = FeGlobals_HashName(buf + 11);           /* deferred */
        *flags |= 1u << (bit & 31);
    } else if (strncasecmp(buf, "FEConst->", 9) == 0) {
        *dest = FeGlobals_GetIntByHash(FeGlobals_HashName(buf + 9));
    }
    /* note: no else - an unrecognised "FE..." value leaves *dest untouched */
}
```

Points that matter for the schema:

- **The sigil is `->`, an arrow.** The literals are `"FEGlobals->"`
  (`0x002aa338`, 11 bytes) and `"FEConst->"` (`0x002aa348`, 9 bytes). Not
  `FEGlobals>`.
- **Matching is case-insensitive.** `0x0025c460` is `strncasecmp` - it folds
  both sides through `tolower` - so `feglobals->x` works, consistent with the
  reader's case-insensitive element and attribute names.
- **The `"FE"` prefix is the fast path.** A value not starting with `FE` (in
  either case) goes straight to the literal parser. A value that *does* start
  with `FE` but matches neither sigil - `FEATURE`, say - falls through every
  branch and **leaves the destination unwritten**, silently keeping whatever was
  there before.
- **The float reader's literal branch calls `atof`** (`0x00110008`, a
  `strtod(s, NULL)` wrapper), not the exponent-blind `Xml_AttributeAsFloat`.
  That makes a third numeric grammar in this parser - see
  [xml-reader.md](xml-reader.md) for the other two. An attribute read through
  this path *does* understand `1e-5`.

`Xml_AttributeAsFloatOrGlobalRef` stores the hash in the same `float` field on
the deferred path, so the tag bit is doing double duty as a type tag.

## Resolution at use time

`0x00197250`, a text-layout helper, is the clean example:

```c
float v = widget->f_0xd0;
if (widget->flags_0x80 & 2)
    v = FeGlobals_GetFloatByHash(*(u32 *)&widget->f_0xd0);
```

One flags word per widget, one bit per attribute, chosen by the `bit` argument
the parser was called with. `0x00187348` (a text widget's `<Values>` parser)
passes bit `0` for its `Color` attribute with the flags word at `+0x80`;
`0x00197250` tests bit `1` of the same word for the field at `+0xd0`. So the
bit index is per attribute and assigned by the parser, not derived from
anything.

## The registry

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0017c810` | `FeGlobals_HashName` | 90 |
| `0x0017c9f8` | `FeGlobals_GetIntByHash` | 90 |
| `0x0017c980` | `FeGlobals_GetFloatByHash` | 90 |
| `0x0017c908` | `FeGlobals_RegisterByName` | 85 |
| `0x0017c870` | `FeGlobals_GetIntByName` | 82 |
| `0x001fc770` | `FeGlobals_FindAndAcquire` | 82 |
| `0x001fc4f8` | `FeGlobals_Insert` | 80 |
| `0x001fc6b0` | `FeGlobals_Remove` | 78 |
| `0x001fc868` | `FeGlobals_Release` | 78 |
| `0x00301988` | `g_fe_globals` (data) | 85 |

`FeGlobals_HashName(name)` is two lines and fixes the whole naming convention:

```c
char buf[128];
sprintf(buf, "FE_%s", name);        /* format string at 0x002aa360 */
return Hash_Crc32String(buf);
```

**So the XML name and the C++ name differ by the prefix.** XML writes
`FEGlobals->TeamModel`; the C++ side hashes the literal string `"FE_TeamModel"`
(`0x0029dc78`) and looks it up in the same registry from `0x00104f28`. They are
the same entry. `"FE_ModelSkin"` (`0x0029dc88`) is the other name with registry
evidence.

Be careful with the other ~120 `FE_*` strings in the binary
(`FE_ACCEPT`, `FE_ON`, `FE_OFF`, `FE_CONFIRM_BUTTON` and so on). They have the
same prefix but no observed registry use and look like localised-text keys.
Only `FE_TeamModel` and `FE_ModelSkin` are demonstrated globals.

### The table

`g_fe_globals` is a fixed-capacity open array of 256 entries, laid out as three
parallel arrays plus a count:

| Offset | Field |
| --- | --- |
| `+0x000` | `void *value[256]` - a pointer to the slot holding the value |
| `+0x400` | `u32 key[256]` - the hash |
| `+0x800` | `s32 count[256]` - a per-entry use counter |
| `+0xc00` | `s32 used` |

Lookup is a **linear scan** over `key[]`, which is fine at this size and is why
there is no bucket structure. `FeGlobals_Insert` refuses at 256 entries
(returning 2) and refuses a duplicate key (returning 1).

`FeGlobals_FindAndAcquire` increments an entry's counter and returns its
`value` pointer; `FeGlobals_Release` decrements it. Both accessors call the pair
back to back, so the net effect on the counter is zero and the counter's purpose
is not visible from here.

**The stored pointer points at a `char *`, not at a number.** Both accessors
dereference one level and then parse text:

- `FeGlobals_GetIntByHash` → `Xml_ParseIntHexOrDecimal(*(char **)entry)`
- `FeGlobals_GetFloatByHash` → `atof(*(char **)entry)` narrowed to `float`

Confirmed on the disassembly (`lw a0, 0x0(s0)` at `0x0017ca40`), not just the
decompiler. So **front-end globals are strings**, re-parsed on every read, and a
global can be read as either an integer or a float from the same stored text.

`FeGlobals_Insert` and `FeGlobals_FindAndAcquire` both take a namespace
argument: when it is non-`NULL` the key becomes
`hash(name) ^ Hash_Crc32String(namespace)`. **Every observed call passes 0**, so
the scoping feature is present but unused on the paths read here.

### Publishing

Two ways in:

- `FeGlobals_RegisterByName(name, slot)` (`0x0017c908`) - hash the name, remove
  any existing entry with that key, insert `slot`.

`FeGlobals_GetIntByName` (`0x0017c870`) is the by-name convenience wrapper over
`FeGlobals_HashName` + `FeGlobals_GetIntByHash`. It takes a **second argument
that is dead**: the disassembly forwards `a1` straight through, and
`FeGlobals_GetIntByHash` zeroes `a1` before using it. Reads like a defaulted C++
parameter that survived into the ABI; the decompiler shows it at one arity and
the call sites at another, so trust the disassembly.
- Directly, by a widget: `0x001aba10` copies its own
  `+0x98` into `+0x88` and registers `&self->_0x88` under the hash stored at
  `+0x5c`. Read off the disassembly, because the decompiler drops the key
  argument at that site. So **a widget can publish its value as a global**,
  which is what makes `FEGlobals->` worth having: one widget's value drives
  another widget's attribute, live.

`Handling_LoadForTeam` (`0x0019f070`) also inserts into this same table, which
is independent evidence that it is the general front-end variable registry
rather than something local to one screen.

## Library routines this pins down

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0025c460` | `strncasecmp` | 90 |
| `0x001fed38` | `Hash_Crc32String` | 82 |

`Hash_Crc32String(s)` is `strlen` followed by `0x001fee90(s, len)`, the
byte-wise reflected CRC-32 over the lazily built table at `0x003025e0` -
register initialised to `0` rather than `~0` and the result complemented at the
end. [README.md](README.md) lists that whole family
(`0x001fed88` / `0x001fede8` / `0x001fee90` / `0x001fef20`) at confidence 55
with "what uses it was not traced". **This is what uses it**: the front-end
globals registry. `0x001fed38` itself was not in that list; it is a fifth
member, the convenience wrapper over `0x001fee90`.

The exact CRC variant is **read off the code, not verified against a known
constant** - see Not determined.

## Not determined

- **The full set of attributes that accept the indirection.** Exactly one is
  verified: `Color`, on a text widget's `<Values>` element, from `0x00187348`.
  The enumeration is mechanical from the call sites and was not done:
  `Xml_AttributeAsIntOrGlobalRef` has 14 (`0x00187424`, four in `0x00191958`,
  two in `0x00194588`, `0x00196e38`, `0x001987a4`, two in `0x001a7010`,
  `0x001a4cb4`, `0x001a92e4`, `0x001a9d9c`) and
  `Xml_AttributeAsFloatOrGlobalRef` has 12 (two in `0x00194588`, three in
  `0x00196c98`, two in `0x00198608`, three in `0x001a7010`, two in
  `0x001a4b88`). **Do not generalise from `Color` to "colour attributes only".**
- **The complete list of registered global names.** Only `FE_TeamModel` and
  `FE_ModelSkin` were traced to a registry call. The rest of the ~15 insert
  sites (`0x00125750`, `0x0014ec90`, `0x00150400`, `0x00150d20`, `0x00160580`,
  `0x00160b40`, `0x0016b2f8`, `0x0017eb70`, `0x001bebc0`, `Handling_LoadForTeam`
  and others) were not read.
- **What the per-entry counter at `+0x800` is for.** Every read path increments
  and immediately decrements it. `FeGlobals_Remove` returns a distinct code when
  it is non-zero, so it looks like a liveness or leak check, but nothing was
  found acting on that.
- **The CRC variant.** The algorithm is read off `0x001fee90` unambiguously, but
  no hash in this family has been checked against a value computed
  independently. `FeGlobals_Insert` special-cases the key `0x7ada3731` with a
  `printf("Adding our baby\n")` (`0x002c01b0`), which would have been a free
  ground-truth pair, but that hash does not match any `FE_*` string in the
  binary under four plausible CRC-32 variants - the name presumably lives only
  in the XML data.
- **Whether an unrecognised `FE...` value is a real authoring hazard.** The
  fall-through leaves the field unwritten; no shipped file was checked for one.
- **Nothing here was verified at runtime.**

## Cross-platform

The PSP has the same mechanism, spelled the same way. The literals are at
`0x08a7d2a8` (`"FEGlobals->"`) and `0x08a7d2b4` (`"FEConst->"`) in `BOOT.BIN`,
read directly rather than assumed, and the `"FE"` fast-path prefix is at
`0x08a7d2c0`.

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Xml_AttributeAsIntOrGlobalRef` | `0x0017c630` | `0x08888bd8` (not renamed) |
| `FeGlobals_HashName` | `0x0017c810` | `0x08888db4` (not renamed) |
| `FeGlobals_GetIntByHash` | `0x0017c9f8` | `0x08889048` (not renamed) |
| `Xml_ParseIntHexOrDecimal` | `0x002030e0` | `0x08953650` |

The PSP counterparts are structurally identical - same three-way prefix test,
same `buf + 11` / `buf + 9` skips - but their registry and float variant were
not traced, so they are **not renamed**. See
[psp-pulse/xml-reader.md](../psp-pulse/xml-reader.md).

## History

- 2026-07-27: first pass, from the loose end
  [xml-reader.md](xml-reader.md) left. The finding worth carrying forward is
  that `FEConst->` and `FEGlobals->` are one namespace differing only in resolve
  time, that the stored values are **strings** re-parsed per read, and that the
  deferred path stores a hash in the value slot with a per-widget flag bit as
  the tag.
