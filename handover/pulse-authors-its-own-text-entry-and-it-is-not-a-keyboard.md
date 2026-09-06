# Pulse authors its own text entry, and it is not a keyboard

Found 2026-09-06, while checking whether the disc had a keyboard to play
before this project drew one of its own for the pilot editor's RENAME. It
does not - but it has something better, and nobody had looked.

## Three results, and the middle one was a surprise

1. **No keyboard layout and no key glyphs anywhere on Pulse.** A
   case-insensitive sweep of all four WADs for `keyboard`, `keypad`,
   `alphabet`, `textentry`, `entername`, `nameentry`, `charset` and
   `qwerty` returns nothing but binary noise inside compressed payloads.
2. **`sceUtilityOsk` is refuted, not confirmed** - the standing assumption
   was that a PSP game hands text entry to the firmware dialog. All four
   OSK NIDs are **absent** from both shipped Pulse executables, while
   `sceUtilityMsgDialog*`, `sceUtilitySavedataInitStart` and
   `sceUtilityNetconfInitStart` are all present, as are the must-hit
   controls the method was validated on first. **Confidence 92.**
   Wipeout **Pure is the opposite**: it links the firmware OSK *and* has
   `TagInput`, with a different 43-character uppercase-only alphabet.
3. **Pulse authors the whole text-entry screen in disc data.** Not a key
   grid: a `<TagInput>` widget, a fixed row of `length` character cells
   the player scrolls one glyph at a time, with a `FE_CONFIRM` item
   bracketed by two gradient bars immediately right of the last cell.
   **Confidence 90.**

## Where it is

- **`Data.wad` entry #1083**, hash `b94fe6f9`, 40,083 bytes - holds
  `NameChange` and `TagChange`. **Its name is unresolved**: 16 prefix
  variants were tried and none hashes to it, so it is reachable by index
  only and there is no `names.tsv` row to write.
- 15 `<TagInput>` instances across `Data.wad`: `Name` at `length="10"`,
  `Tag` at `3`, and the online `GameNameTag`/`GamePasswordTag`/
  `UsernameTag`/`PasswordTag` at `14`, carrying `Encrypt="true"` and
  `AllowBlank`.
- Geometry is authored per screen and **must be parsed, not transcribed**:
  cells `28x25` at pitch `30` from `OffsetX="46" OffsetY="88"`, fill
  `0x2fffffff`, text at `y="85"` scale `2.0` in `FEGlobals->TextColor`.
  Those numbers are the parser's test target, not a `const` - see
  CLAUDE.md's "Never invent what the assets already author", which this
  file exists to serve rather than to work around.
- **The alphabet is in the executable**, not the WAD:
  `PSP_GAME/SYSDIR/BOOT.BIN` file offset **2,808,976**, vaddr
  **`0x08AB1C10`** at the standard load base, 70 bytes:
  `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz 0123456789!-+@:?*`.
  Five `lui 0x002b` / `lo16 0xdc10` pairs reference it, all inside one
  `0x0c6xxx`-`0x0c7xxx` code region that also holds the `TagInput`
  element-name reference. **Confidence 85.** The glyph-map rival reading
  is ruled out by Pure's copy having no lowercase at all while its front
  end demonstrably draws lowercase.
- String ids `PRO_ENTER_NAME` and `PRO_ENTER_TAG`, in nine languages.

Full evidence, with every command re-derivable, is in the investigation
scratch file - not committed, because it is 556 MB of extracted disc
content away from being reproducible and this summary is the durable part.

## Why the pilot editor did not use any of it

`crate::prompt::Keyboard` (landed 2026-09-06) is a **grid**, and that is a
deliberate divergence with the evidence above behind it rather than an
unchecked one. Two reasons, both structural:

- **The authored alphabet cannot spell a pilot name.** 70 characters
  including uppercase, a space and `!-+@:?*`; `pilots::check_name` allows
  lowercase, digits, `-` and `_` and nothing else, because the name
  becomes a filename. Using the widget would mean re-authoring a
  38-character subset - invention either way, and invention that looks
  recovered, which is the worse of the two.
- **The geometry is authored for a PSP profile screen.** Adopting
  `OffsetX=46` for a page no PSP has would be less faithful than deriving
  from the live `menu::Skin`, which is what lets an HD skin get an
  HD-sized panel.

A pilot name is this project's own data on this project's own screen. A
profile name is the disc's, on the disc's.

## Open

- **Nothing renders the `TagInput` yet.** `crates/formats`' front-end XML
  path would need the element; `docs/formats/fexml.md`'s element table has
  no `TagInput` row, and the schema recovered here is `length`, `Encrypt`,
  `AllowBlank` plus the usual `x`/`y`/scale/colour/`focus`/`transition`.
- **Input mapping is unsettled, at 65**: up/down presumed to cycle the
  glyph under the cursor and left/right to move between cells, which is
  the only reading the cell strip and a `sll`/`sra` byte sign-extend (a
  cycling selector's wrap) support - but nothing has been captured.
- **Not checked at all**: PS2 Pulse, both EU discs, the HD/Fury PSARC
  interiors (a raw grep cannot see inside them), and 2048 (its PKGs are
  encrypted).
- Whether Pure *uses* its `TagInput` or its firmware OSK, having both.

## Next Steps

1. Settle the input mapping with a PPSSPP capture of
   `Profile > Your Details > Enter your name` - one recording answers the
   only thing between this and a faithful implementation.
2. Add `TagInput` to `docs/formats/fexml.md`'s element table with the
   schema above, and a parser for it in `crates/formats`, testing against
   `Data.wad` entry #1083's own numbers rather than against a fixture.
3. Recover entry #1083's **name**; until then any code reaching it does so
   by index, which is the kind of thing that silently breaks on the EU
   pressing. `just mine-names` is the tool.
4. Write the alphabet's consumer up as a Ghidra evidence page before any
   `names.tsv` row - the address is solid at 85 but the *function* was
   never traced, and this project's rule is that the page and the row land
   together.
