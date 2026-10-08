# Wipeout HD / Fury: the front end, as its own disc states it

**This page is a reading, not a measurement.** It records what
*Wipeout HD / Fury*'s front-end XML authors, quoted, so that an `oag-hd` title
package can be filled in from evidence rather than from Pulse's numbers. It is
the HD counterpart of [menus-original](../ui/menus-original.md) and
[pure-boot](../architecture/pure-boot.md), and it sits beside
[hd-status](hd-status.md), which is the format-layer probe this continues.

**Nothing here has been verified under an emulator.** Every claim below rests on
static reading: the disc's own XML, and - for the section
[what the executable says](#what-the-executable-says) alone - a read-only sweep
of `EBOOT.elf`. Per the [rubric](../reverse-engineering/confidence-rubric.md)
that caps an authored-value claim at 94 and puts any *runtime* claim - what the
boot actually walks, what colour a selected row is, how long a page change takes
on screen - at zero, because none was made. The executable sweep renamed
nothing; it cites addresses, and anyone promoting one to a name owes a
`docs/ghidra/functions/ps3-hdfury-eu/` page and a `names.tsv` row in the same
change.

The disc is `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`, layer-1 decrypted per
[ps3-disc](ps3-disc.md); everything is read through the
[`.psarc` reader](psarc.md).

## This front end is wired, and its chain has now been watched

**Added 2026-08-17, upgraded 2026-09-05.** When this page was written,
everything on it was recovered data that nothing consumed:
`oag_hd::TITLE.front_end` was `None`, because `oag_title::BootProfile::chain`
was defined as a *measurement* and the chain below was a *declaration*, so there
was no honest way to ship it.

[ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md)
changed the type rather than the evidence: a chain carries a `Provenance`, HD's
was `Declared`, and every boot of it printed a line saying so before anything was
drawn. **Nothing on this page became more certain that day** - and the ADR named
what would end the state: "an emulator capture now *upgrades* a title rather
than unlocking it."

**That capture happened on 2026-09-05 and `oag_hd::frontend::BOOT` is
`Provenance::Measured`.** Three cold boots on RPCS3 walked all eight steps in the
declared order; the same boots settled that `DATA00`'s is the live `skin.xml`,
and turned the other five copies from "unexercised" into "unreachable". What is
*still* open, deliberately and separately, is whether the language picker is ever
**shown** as opposed to entered - see ["what the capture did not
settle"](#what-the-capture-did-not-settle).

What the wiring bought is that HD's own screens draw in its own 1920x1080 grid,
its sixteen language plugins and 1,602 strings resolve, and `Studio Logo` plays
the [Studio Liverpool reel](bik.md) out of the disc's own `.bik`. What this
build **cannot** drive - the connection check, the three dialogs, the EULA and
the save warning - is stepped over and named on the report, so a shortened
sequence says it was shortened. See
`crates/game/tests/hd_boot_ground_truth.rs`, which pins all of it against the
disc.

Two things that surfaced only once something walked this data: HD ships **no
Latin `.fnt` this build can read**, so its whole front end draws in the built-in
5x7 glyphs (its language plugins name `helv.fnt`, which is present and fails
`oag_texture::fnt`'s magic check - unread, and probably the byte-order story
again); and its `<Image>` widgets are `.gtf`, so they draw nothing and say so -
`0 of 3 front-end image(s) decoded into a 1x1 sheet`.

**That second one is now a wiring gap rather than a format gap**, and the
distinction arrived with the merge: [`gtf.md`](gtf.md) reads the PS3's texture
container, and HD's HUD samples ten `.gtf` textures through it. What still does
not is `boot::load_sprites`, which offers a front-end image to the PSP `.mip`
path alone and gets `zero-sized texture 1281x0` for its trouble. Routing it
through `oag_texture::gtf` is a small change nobody has made, and it is what
would put `saveIcons`, `line` and `Title_Arrow_HD` on screen.

### The sixteen language plugins, and the three that lie about themselves

`Data\Plugins\Languages\<name>`, sixteen of them, each holding the
`Definition.xml` and `entries.xml` pair a numbered PSP plugin holds - which is
what let one mechanism serve both once the plugin list became
[`oag_title::FrontEnd::language_plugins`]. All sixteen resolve as entries.
**Fifteen parse into a language**; German's table alone is 1,602 strings.

Two findings, and the first is the disc's own bug rather than a reading error:

- **Japanese, Korean and TraditionalChinese all report their native name as
  `Svenska`.** Not a mix-up in this build: `japanese/definition.xml` contains
  `<Entry ID="Japanese" String="Svenska">` literally, and the other two carry
  the same copy-paste from Swedish. A boot report reading `Japanese (Svenska)`
  is quoting the disc. Pinned in
  `crates/game/tests/hd_boot_ground_truth.rs` so that a future change which
  *fixes* these is noticed rather than assumed correct.
- **Portuguese is the one plugin that does not parse**, and Russian's native
  name comes through as `P??????`. Both look like an encoding fault rather than
  a schema one: every file declares `encoding="utf-8"` and
  `portuguese/definition.xml` writes `Portugu<0xea>s`, which is Latin-1. Not
  chased - it costs one language in a picker this build's own menus do not
  depend on - and recorded here so the next reader starts from the symptom
  rather than from the plugin list.

[`oag_title::FrontEnd::language_plugins`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/title/src/lib.rs

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Where is the skin? | `/data/plugins/frontend/gui/skin.xml`, in **six** of the seven archives | 94 |
| Do the six copies agree on layout? | **Yes, on every layout global, exactly** | 92 |
| Which copy does the runtime load? | **`DATA00`'s.** `data00.psarc` loads last, and the runtime stats a `.bik` only that copy names | 92 |
| Coordinate space | **1920x1080**, not Pulse's 480x272 | 90 |
| Is the main menu a row list? | **No.** It is a `<HorizMenu>` - horizontal | 90 |
| Is there a `menu` font role? | **No.** HD's language plugins declare no such slot | 90 |
| Is there a looping movie backdrop? | **No.** The FE background is a real-time `.vex` scene | 88 |
| Are any `FEGlobals` referenced but undeclared? | **None, in any of the six** | 90 |
| Declared boot chain | Eight screens then `Main Menu`, quoted below | 80 |
| Runtime boot order | **The same order.** Three cold boots on RPCS3, 2026-09-05 | 85 |
| Is the language picker ever *shown*? | **No evidence of it presenting.** `LanguageAutoRedirect` fired within 2-296 ms on all four RPCS3 boots tried, regardless of system language matching a shipped plugin or a save already existing | 85 (mechanism), 75 (never presents) |
| Does the executable agree on the first screen? | **Yes.** `0x000186f0` returns `"Language Selection"` by default | 70 |

## Reading it yourself

```sh
python3 scripts/psarc.py list data/images/hdfury-ps3-eu-dec.iso > /tmp/hd.txt
python3 scripts/psarc.py cat \
    'data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA06.PSARC' \
    /data/plugins/frontend/gui/skin.xml
```

`fd` and `rg` respect `.gitignore` and return nothing under `data/`; use
`/bin/ls`, `find` or `grep`, or pass `--no-ignore`.

## Six skins, one layout

`skin.xml` appears in `DATA00`, `DATA02`, `DATA03`, `DATA04`, `DATA05` and
`DATA06` - every archive except the sound-only `DATA01`. All six differ by
MD5, by global count (62 or 64) and by screen list. **They do not differ on a
single layout number.** Read out of all six independently:

| Global | Value, all six copies |
| --- | --- |
| `MenuXOffset` | `800` |
| `MenuYOffset` | `300` |
| `MenuScale` | `1.0` |
| `TitleXOffset` | `194` |
| `TitleYOffset` | `62` |
| `TitleScale` | `1.0` |
| `TextColor` | `0xFFFFFFFF` |
| `TitleColor` | `0xFF646464` |
| `MSWarningScale` | `1.0` |
| `FMVFrameCount` | `2847` |

**Confidence 92.** Six independent copies of an authored file agreeing to the
digit is the "exact agreement across many real files" case the rubric puts at
the top of the 85-94 band. It falls short of 95 because nothing shows the engine
*consumes* them the way `MenuSkin` assumes - that needs the executable.

The practical consequence is the useful one: **the `MenuSkin` numbers do not
depend on resolving which archive the runtime loads.** The boot chain does.

The two extra globals in `DATA00` and `DATA06` are `HD_BG` and `HD_transBG`,
declared in a nested `<Screen name="HD_Colours">` block that the other four
copies do not carry.

**"They do not differ on a single layout number" is exactly as narrow as it
sounds: they differ on the *colours*.** That `HD_Colours` block is the FE style,
and `DATA00`'s and `DATA06`'s copies of it disagree on five of their six values -
see [the palette section](#the-hd_-palette-is-the-fe-style-and-the-archives-disagree),
which is why nothing in this project transcribes an `HD_*` colour.

## The coordinate space is 1920x1080

This matters more than any single number. Pasting `menu_x: 800.0` into a
480-wide layout puts the menu off-screen. Evidence, all from `DATA06`'s
`skin.xml`:

```xml
<Movie name="LogoMovie" transition="0">
    <Values X="0" Y="0" Width="1920" height="1080" src="Data/FE/Images/StudioLiverpool.bik" ...></Values>
</Movie>
```

```xml
<BackgroundAnim name="bgAnim" DisableTransition="0">
    <Values OriginX="960" OriginY="540" nearZ="1.0" ...></Values>
</BackgroundAnim>
```

```xml
<Text name="AmericanLegalLine" transition="0" delay="0.7">
  <Values string="WipEout® HD ©2008 Sony Computer Entertainment Europe. ..."
          x="985" y="972" scale="0.7" color="0xff646464" Align="centre" ...></Values>
</Text>
```

A centred backdrop origin at (960, 540), a movie at 1920x1080, and a footer
line at y=972 all agree on a 1920x1080 authoring space. `MenuXOffset` 800 puts
the menu column just right of centre; Pulse's 50 in a 480-wide space put it hard
left. **Confidence 90** - three independent sites agree and no site contradicts,
but no capture confirms the space is not letterboxed or safe-zone-inset before
presentation. `obeySafeZone="false"` and `ignoreSafeZoneFullscreen="true"`
appear as attributes, so a safe-zone transform exists and is per-widget.

### It is drawn in that space now, and what that cost

**2026-08-18.** Until this landed, `oag-game` laid every menu out in the PSP's
480x272 whatever disc was mounted, and the paragraph above was a warning rather
than a description. What it warned about is exactly what happened: `menu_x` 800
in a 480-wide grid put the whole label column 320 pixels past the right-hand
edge, so the RACE page drew a column of *values* with no labels beside them -
which reads as a string-table failure, not a coordinate one, and cost a pass to
recognise.

Two things changed, and the second is the one worth carrying:

- [`oag_title::MenuSkin`](../../crates/title/src/menu.rs) now states the grid its
  own numbers were **read in**, and `oag_game::menu::Skin` draws in the
  **source's**. `oag_hd::frontend::MENU_SKIN` says `(1920.0, 1080.0)`.
- **Those are different questions and one field could not hold both.** Wipeout
  Pulse ships a PSP pressing and a PS2 one; its table was read off the PSP
  `Skin.xml` and the PS2's own file places widgets in 640x448. The first attempt
  scaled only *this build's* figures into the source's grid and left the title's
  alone, which is right exactly while the two agree - and on the PS2 disc it left
  the value column at 640/480 of where it belonged while the label column stayed
  put. Both sides convert now, each from its own grid.

Drawing in the source's grid rather than converting into the PSP's is the choice
that keeps the picture: HD's faces are rasterised for a 1080-line screen, so
squeezing them into a 272-line one draws them at quarter scale from an atlas
built for four times that; and the grid is what the whole front end is
letterboxed to, so borrowing the PSP's would show a 16:9 menu at 480/272 =
1.765. Pinned by `crates/game/src/menu/tests/drawing.rs` against the shipped
table, and by `the_menu_skins_own_grid_is_the_one_this_disc_authors_in` against
the disc.

**Still unconfirmed, and unchanged by any of it**: whether the space is
letterboxed or safe-zone-inset before presentation. The `obeySafeZone` attributes
above are the reason to expect it is.

## The three front-end images are `.gtf`, and all three draw

`Data\FE\Images\saveIcons.gtf` (256x128), `line.gtf` (8x8) and
`Title_Arrow_HD.gtf` (32x32) are every image the front-end root names. All three
are ordinary [`.gtf`](gtf.md) and decode with no new work - `oag_texture::gtf`
had handled them since long before the front end asked for one.

**What was missing was a branch, and the error named the wrong format.**
`oag_hud::sprite::Image::decode` tried the PSP `.mip` parser and then the PS2
one, so a `.gtf` came back as *"zero-sized texture 1281x0"* - a complaint about a
format the file is not - all three failed, and the sheet collapsed to a single
transparent texel. `line.gtf` is the 8x8 tile stretched to the 1600-pixel rules
above and below every screen, so a sheet with *some* width would not have been
enough to notice; the sizes are asserted in
`every_front_end_image_this_disc_names_decodes`.

## A circuit's name is in a different archive from the circuit list

**Confidence 97, raised from 92 on 2026-08-30: the choice of `DATA06` is now
measured against the real front end, not only inferred from archive
geometry.** `scripts/rpcs3-drive.py browse` (added for this) walks Racebox's
`Track Creation` carousel with no button but `right`, screenshotting every
step. Across all 24 presses the carousel exposes (12 environments, seen twice -
**qualified 2026-09-29: frames 13-24 repeat frames 1-12 with the cursor on the
same top hex row and no reverse glyph, so `right` most likely wraps at twelve
rather than walking a forward-then-reverse half; see `docs/ui/campaign-screens.md`'s
"Wipeout HD/Fury: `Track Creation`"**), the drawn name **never once carries a `REVERSE` suffix
or any other direction text** - forward and reverse read identically, exactly
as `DATA06`'s table does and the served `DATA02` table does not. The specific
pairing this page's geometry argument turns on is directly confirmed: both of
`Talons_Junction`'s two carousel entries draw `TALON'S JUNCTION`, never
`SEBENCO CLIMB REVERSE`. The direction itself is conveyed by a small arrow
glyph in the `CIRCUIT DIRECTION` cluster beside the name, not by text - the
original does not solve the one-column problem the same way
`oag_raceplay::catalogue::label`'s `FE_REVERSE` suffix does, it avoids having the
problem by keeping both columns' worth of information in one carousel slot.
Zone's four circuits were not reached: Racebox's `Track Creation` only ever
showed the 24 base entries and wrapped, so whether `PRO TOZO` et al. appear
anywhere in the real front end is still unmeasured. Screenshots stayed under
`data/` and were not committed, per this project's leakage policy; the names
transcribed above are the reproducible part.

The front end ahead of Main Menu also settled two things
[the declared boot chain](#the-boot-chain-declared-and-then-watched) had flagged as
unmeasured, on the same boots: `Language Selection` does run on a PS3 boot
(`TTY.log`'s `Switching Screen` sequence reads `"" -> Top -> Language
Selection -> PreFMVConnect -> Studio Logo -> ...`), and `EpilepsyWarning`'s
`<Dialog>` is the live path, not its `<Redirect>` twin - a boot that only
waits, pressing nothing, sits at `EpilepsyWarning` for as long as it is given.

The front-end plugin keys circuits `NN_Track` and every
string table keys their names `NN_TRACK`, so nothing on the RACE page resolved
and every circuit drew as its own id. This page used to conclude that the names
were unreachable, on the grounds that folding the case resolves sixteen of the
twenty-eight and puts a **confidently wrong** name on eight. That much was
right. The conclusion was not, and the measurement it was missing is **which
copy**: HD ships `entries.xml` five times, and only one of the five names every
circuit `DATA00` declares.

| Archive | `NN_TRACK` keys | `17_TRACK` reads |
| --- | ---: | --- |
| `DATA00` | *no `entries.xml` at all* | - |
| `DATA02` | 24 | `SEBENCO CLIMB REVERSE` |
| `DATA03` | 24 | `SEBENCO CLIMB REVERSE` |
| `DATA04` | 24 | `SEBENCO CLIMB REVERSE` |
| `DATA05` | 24 | `SEBENCO CLIMB REVERSE` |
| `DATA06` | **28** | `TALON'S JUNCTION` |

`oag_assets::Archives` serves `DATA02`'s, which is the copy with the base
game's numbering; `DATA00` supplies the circuit list, whose `17_Track` loads
`Data\Environments\Talons_Junction`. The four keys the others lack are exactly
the four Zone circuits - `25_Track`..`28_Track`, `Zone_1`..`Zone_4`, which
`DATA06` names `PRO TOZO`, `MALLAVOL`, `CORRIDON 12` and `SYNCOPIA`. So the
copy is chosen on **coverage**: the one table that names the whole list. Read
off **all sixteen languages in all five archives** - 24 keys everywhere but
`DATA06`, 28 there, with no exception - so it is a property of the archive
rather than of one language file.

**What corroborates the choice is the geometry.** Twelve of HD's environments
are declared twice, once each way round. On `DATA06`'s copy every such pair
reads one name - `17_Track` and `18_Track` both load `Talons_Junction` and both
read `TALON'S JUNCTION`. On the served copy they read `SEBENCO CLIMB REVERSE`
and `SOL 2 REVERSE`: two names for one piece of track, which is the mismatch
stating itself. That agreement is checked and asserted, and it is deliberately
**not** the selector - it is false by design on Pulse, where `16_Track` and
`32_Track` share an environment and are named separately.

The same table keeps a parallel `_OLD` set carrying the old numbering
(`09_TRACK_OLD` = `TALON'S JUNCTION`, `10_TRACK_OLD` = `TECH DE RA`,
`11_TRACK_OLD` = `MODESTO HEIGHTS`, `12_TRACK_OLD` = `THE AMPHISEUM`), which is
the renumbering saying so in the data.

**Only the circuits move.** The served table stays served for everything else:
`DATA02`'s and `DATA06`'s copies differ on 42 shared keys in english and 72 in
german, none of them a circuit, and which copy the runtime loads is the same
open question [the six `skin.xml` copies](#six-skins-one-layout) pose. Swapping
the whole table would be answering it; choosing a copy for the one column whose
pairing is checkable is not.

Pinned by `a_circuit_is_named_off_the_copy_that_agrees_with_the_circuit_list`,
which asserts the pairing, the coverage and the one-environment-one-name
corroboration, and still asserts that the served copy holds the old numbering -
so a future change that swaps the whole table fails here rather than passing
quietly.

### The direction is drawn beside the name, not inside it

`DATA06` names a circuit and its reverse **identically**: `01_Track` and
`09_Track` are Vineta K forward and backward and both read `VINETA K`. The
older copies carried a suffix and the `_OLD` keys still do (`13_TRACK_OLD` is
`VINETA K REVERSE`), so the renumbering dropped it deliberately - and the disc
ships `FE_REVERSE` (`REVERSE`, `RÜCKWÄRTS`, in every copy and every language)
to draw the direction with.

This build's RACE page is one column, where the original's is not, so twelve
pairs of identical rows would be a menu nobody can use. `oag_raceplay::catalogue::label`
appends `FE_REVERSE`'s own string to a reversed circuit **only where it would
otherwise read the same as the circuit it shares an environment with** - two of
the disc's strings joined, rather than a word this build made up, and nothing
at all on either PSP title, whose tables already name the two directions
separately.

## Zone is a `Mode` list entry, not a separate screen - and the walk that would settle whether it widens is blocked on hardware access

**Confidence 94 (authored value, per this page's own cap) - 2026-09-03.**
`data/plugins/frontend/gui/racebox_definition.xml`'s `Single Player` screen
(reached from `Main Menu`'s `FE_RACEBOX` tab) carries one `<List name="Mode">`
with five entries - `Arcade`, `Time Trial`, `Speed Lap`, `Tournament`, `Zone` -
and one `<Redirect>`: `Tournament` alone branches, to `Tournament C`; every
other choice, `Zone` included, falls through the same `Default goto="Track
Creation"`. There is no `ZoneTrackSelection` screen type and no Zone-specific
branch in this redirect - whatever circuits Zone mode offers, it offers them
through the identical `Track Creation` carousel every other single-player mode
uses, the one [measured against the real front end above](#a-circuits-name-is-in-a-different-archive-from-the-circuit-list).
That carousel's own `<List name="Track">` block authors a `Padlock` and three
`ReverseIcon*` images per row and nothing else - no Zone-only pendant or badge
widget is declared in the layout at all.

**What this does and does not settle.** It rules out the shape where Zone gets
its own dedicated, narrower screen - a real alternative that had to stay open
when `oag_title::ZoneCircuit::Separate`'s `also_race_circuits: true` was
implemented from a play recollection, not a decompile or a capture. It
does **not** settle what the shared `Track` list's *contents* are once `Mode`
is `Zone` - that is built by code this project has not located (the only named
front-end function anywhere in this database is `FrontendRoot_Construct`,
`game-boot.md`; nothing about list population or per-row mode filtering has
been read), and the one existing capture of this exact carousel
(`#a-circuits-name-is-in-a-different-archive-from-the-circuit-list`, above)
walked it **without ever selecting Zone mode**, so its "only 24 base entries,
wrapped" result describes Arcade's list, not Zone's, and settles nothing about
whether Zone widens to all 28 or narrows to its own four.

**The walk that would settle it directly is written and currently blocked, not
run.** `scripts/rpcs3-drive.py browse --nav "Main Menu=right" --nav "Single
Player=down,down,down,down,cross" --screen "Track Creation" --button right
--steps 28` would select `Zone` in the `Mode` list (four `down`s off its
`default="Arcade"`) before confirming into the same carousel the existing
capture already knows how to read, and either result is decisive: four rows
naming `PRO TOZO`/`MALLAVOL`/`CORRIDON 12`/`SYNCOPIA` and nothing else settles
`Separate`-only; 28 rows including all twelve ordinary environments
corroborates `also_race_circuits: true` outright. It has not been run this
session: `just rpcs3-preflight` reports `/dev/uinput` is mode `0600` group
`root` with no udev rule granting it to a group, and `sudo -n true` confirms
no passwordless sudo is available, so the one-time fix
(`echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee
/etc/udev/rules.d/99-uinput.rules && sudo udevadm control --reload-rules &&
sudo udevadm trigger /dev/uinput`) needs the machine's own user, not a
session running unprivileged. This is a one-time, per-machine fix - once
applied, the walk above is the whole remaining step.

**2026-09-03, later the same day: a second, independent play recollection
corroborates the wide reading.** Asked directly what the Zone-mode picker
showed on real hardware, the user's answer was "the full track list, with a
few zone-only ones mixed in" - the same shape `also_race_circuits: true`
already implements (all 28 `PI_Track` entries, the four zone-exclusive ones
first) and the opposite of a narrow, zone-only carousel. This is a second,
independently-asked play observation, not the capture above and not a
decompile - it does not raise the claim past where a corroborated hypothesis
sits, but it is real evidence in the same class as the pendant recollection
that started this fork, this time answering the specific question this
section's capture command was written to settle.

All sixteen plugins parse. Fifteen did until 2026-08-18, and the sixteenth was
lost to one byte: `Portuguese` writes its own name `Portugu\xeas`, which is
Latin-1 and not valid UTF-8, so the file failed to decode and the plugin was
skipped with no report line naming it. `Spanish` on the same disc is genuinely
UTF-8 (`Español`), so the release is **mixed-encoding** rather than Latin-1;
`boot::xml::expand` tries UTF-8 first and falls back to Latin-1, which is total
and so cannot lose a file again.

**Four of the native names are wrong on the disc itself and are left alone.**
`japanese/definition.xml` literally contains `<Entry ID="Japanese"
String="Svenska">`, and Korean, Swedish and TraditionalChinese carry the same
copy-paste; `Russian` declares `P??????`. A boot report reading
`Japanese (Svenska)` looks exactly like this build mixing two plugins up, which
is why it is pinned rather than merely noted - see
`every_declared_language_plugin_resolves`. Correcting them here would be
inventing.

## The dead PSP screen

Four copies (`DATA02`, `DATA03`, `DATA04`, `DATA05`) carry a `LogoFMV` screen:

```xml
<Screen name="LogoFMV">
    <Image transition="0">
        <Values width="480" height="272" color="0xFF000000"></Values>
    </Image>
    <Movie transition="0" name="BackdropMovie">
        <Values src="Data\Movies\Intro" sound="true" autostart="true" repeat="false" autoredirect="true"></Values>
    </Movie>
    <Redirect name="AutoRedirect" StartEnabled="false">
        <Values backward="none" forward="none"></Values>
        <Default goto="Show Logo"></Default>
    </Redirect>
    ...
```

**This is inherited Pulse boilerplate and is not HD's boot.** Four independent
facts, each checkable:

1. The `<Image>` is `480x272` - the PSP screen - in a file authored at
   1920x1080 everywhere else.
2. `Data\Movies\Intro` does not exist. `grep -ci "/data/movies"` over the
   11,664-entry listing returns **0**; there is no `Data\Movies` directory on
   this disc at all.
3. `Show Logo` is never defined. It is the target of that redirect and appears
   nowhere as a `<Screen name=...>` in any of the six skins or the ~30 other
   frontend files.
4. **Nothing reaches `LogoFMV`.** No `goto="LogoFMV"` exists anywhere in any
   copy, including `DATA05`, which carries both `LogoFMV` *and* the newer
   `PreFMVConnect` and routes the picker to the latter.

The two newest-looking copies (`DATA00`, `DATA06`) drop the screen entirely.
**Confidence 90.** `DATA02` also carries `memorystickbootscreens_psp.xml` and
`memorystickscreens_psp.xml`, and every copy still declares `MSWarningScale`,
`MSWarningColour1` and `MSWarningColour2` - Memory Stick globals on a machine
with no Memory Stick. The lineage is visible in the leftovers, and this is the
same "the original is a specification, not a source" trap: a screen can be
present, well-formed and completely dead.

A related tell, from `/data/plugins/languages/english/definition.xml`:

```xml
<Entry Language="English" ID="Wipeout Pulse" String="Wipeout Pulse"></Entry>
```

HD's own language plugin still identifies the game as Wipeout Pulse.

## The boot chain, declared and then watched

**Measured 2026-09-05, confidence 85.** Read redirect for redirect out of
`DATA00`/`DATA05`/`DATA06`'s `skin.xml`, which agree with each other - and since
2026-09-05, watched twice on RPCS3 going exactly this way. The `Leaves by`
column is still the XML's; the *order* is no longer only the XML's. Reproduce
with `just rpcs3-bootchain`; both captures are under
`data/reference/hd-boot-chain/`.

| # | Screen | `type=` | Leaves by | To |
| --- | --- | --- | --- | --- |
| 1 | `Language Selection` | `Language Selection` | `LanguageAutoRedirect`, `Default goto` | `PreFMVConnect` |
| 2 | `PreFMVConnect` | - | `<UnityConBasic>`, `MoveTo success=/failure=` | `Studio Logo` |
| 3 | `Studio Logo` | `FMV` | `AutoRedirect` + five skip buttons | `EpilepsyWarning` |
| 4 | `EpilepsyWarning` | `EpilepsyWarning` | `<Dialog>` `Option1Destination` | `FirstPlay` |
| 5 | `FirstPlay` | `FirstPlay` | `<Dialog>` both options | `Save Warning` |
| 6 | `Save Warning` | `HDDBoot` | `Main Menu Redirect`, `Default goto` | `EULA` |
| 7 | `EULA` | - | `Redirect`, `Default goto` | `Update Announcement` |
| 8 | `Update Announcement` | - | `<Dialog>` `Option1Destination`, `<SVOData redirectTo=>` | `Main Menu` |
| 9 | `Main Menu` | `Main Menu` | (in `mainmenu_definition.xml`) | - |

The quoted evidence for the two that carry the boot's only movie:

```xml
<Screen name="PreFMVConnect">
    <ScreenClear>
        <Values Colour="0x00000000"></Values>
    </ScreenClear>
    <UnityConBasic name="PreFMVConnection" >
            <MoveTo failure="Studio Logo" success="Studio Logo" failTime="1.5"></MoveTo>
            <Connection checkNP="true" allowFailure="true"></Connection>
    </UnityConBasic>
</Screen>

<Screen name="Studio Logo" type="FMV">
    <ScreenClear>
        <Values Colour="0x00000000"></Values>
    </ScreenClear>
    <Movie name="LogoMovie" transition="0">
        <Values X="0" Y="0" Width="1920" height="1080" src="Data/FE/Images/StudioLiverpool.bik"
                ignoreSafeZoneFullscreen="true" repeat="false" preload="true"></Values>
    </Movie>
    <UnityConBasic name="StudioLogoConnection" >
            <Connection checkNP="true"></Connection>
    </UnityConBasic>
    <Redirect name="AutoRedirect" StartEnabled="false">
        <Values backward="none"></Values>
        <Default goto="EpilepsyWarning"></Default>
    </Redirect>
    <!--so we can skip the movie with these buttons-->
    <Redirect name="StartRedirect">
        <Values backward="none" forward="start"></Values>
        <Default goto="EpilepsyWarning"></Default>
    </Redirect>
    ...
```

`PreFMVConnect` reaches `Studio Logo` on both branches, so it is a screen the
chain passes through unconditionally within 1.5 s, not a fork.

### The capture that made this an order rather than a reading

This paragraph used to read "**confidence 80 for the chain as *declared*.
Confidence 0 for it as the runtime order, because that was not measured**", and
the worry behind it was concrete: [ADR-0023] exists because Pulse's XML declares
the language picker first and its runtime opens on `LogoFMV` instead, so HD
might have done the same. **It does not.** `oag_hd::frontend::BOOT` carries
`Provenance::Measured`.

Three cold boots, RPCS3 `v0.0.42-19777`, `BCES-00664`, the layer-1 decrypted
image, savedata moved aside (see below), no GDB connection at any point. Screen
names read off `TTY.log`'s own `Switching Screen` lines, `cross` sent only when
a screen had sat still for eight seconds so no auto-redirect was hurried:

| # | Screen | cold-01 | cold-02 | left by |
| --- | --- | ---: | ---: | --- |
| 1 | `Language Selection` | 17.16 s | 15.77 s | auto |
| 2 | `PreFMVConnect` | 17.16 s | 15.77 s | auto |
| 3 | `Studio Logo` | 17.16 s | 15.77 s | `cross` |
| 4 | `EpilepsyWarning` | 28.30 s | 30.36 s | `cross` |
| 5 | `FirstPlay` | 39.19 s | 41.70 s | `cross` |
| 6 | `Save Warning` | 50.07 s | 52.70 s | auto |
| 7 | `EULA` | 53.85 s | 55.92 s | auto |
| 8 | `Update Announcement` | 53.85 s | 55.92 s | auto |
| - | `Main Menu` | 53.85 s | 55.92 s | - |

Identical ordering both runs, nothing extra between any two steps, and it is the
table above exactly.

**Confidence 85, and the arithmetic is worth spelling out** because it is a
judgement inside a band rather than a cap. This is a *runtime trace*, which the
[confidence rubric](../reverse-engineering/confidence-rubric.md) ceilings at 94
"until a second binary agrees" - and the project already treats an emulator
trace as a runtime trace, PPSSPP breakpoint traces being where several 88s on
the PSP side come from. So the rubric permits up to 94 here and nothing caps
this at 85. It is scored at the **bottom** of the 85-94 band for two reasons
this page states rather than defers: RPCS3 is not a PS3 and its front-end timing
and connection checks are its own, and there is no second pressing or second
platform to corroborate against the way Pure's cold boot has. A capture on real
hardware, or on HD's PSN release, is what would move it up.

**Step 4's live path is the dialog, now on three observations.**
`EpilepsyWarning` plus `cross` goes to `FirstPlay` both times, which is the
`<Dialog>`'s `Option1Destination` and *not* `EpilepsyWarningRedirect`'s
`Default goto="Save Warning"`.

#### `FirstPlay` is save-state conditional, and it is the trap in this measurement

**With `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/BCES00664-AUTO-`
present, HD skips `FirstPlay` entirely** and goes `EpilepsyWarning` straight to
`Save Warning`. `TTY.log` says why in the same breath - `Data BCES00664-AUTO-
has been found`, `get->isNewData: 0`. A run on a used profile therefore watches
**seven** steps and looks exactly like a complete boot.

That is worth stating as loudly as the result, because upgrading `Provenance` on
such a run would have put a step no capture ever showed inside a chain labelled
watched - [ADR-0025]'s own error mode, one level down. Move the save aside
before capturing, and put it back after; `just rpcs3-bootchain`'s recipe comment
says so too.

`data/reference/hd-boot-chain/cold-01/02-firstplay.png` is the screen: *"ARE YOU
NEW TO WIPEOUT? ... Choosing YES below will enable the Pilot Assist option."*

#### What the capture did **not** settle, and what a later one did

**How this build draws that picker anyway (2026-09-30).** Nothing here was watched,
so it is inference from the XML: the `<Menu>` authors no row pitch, and the rows
step by one line of the loaded `Default` face (`helv`, 33 units) rather than by
Pulse's 13-unit table, which printed them over one another. Its confirm prompt
(`font="menu"`, a role HD declares no slot for) draws through the `Buttons`
face, the only one holding `FE_CONFIRM_BUTTON`'s codepoint. Both are **chosen,
not measured**; `oag_title::BootProfile::picker_from_loaded_faces` is the switch
and `picker_pitch_ground_truth` the test.

**No frame of `Language Selection` was ever caught in the original three
cold boots.** cold-02's 1.5-second film runs `Sony Computer Entertainment
presents` (11.77 s) -> black (14.97 s) -> the Studio Liverpool reel already
playing (20.01 s). `TTY.log` names the picker being entered on every boot, so
the *state* runs - but whether it ever **presents a frame to a player** was
open, and the picker's own `LanguageAutoRedirect` was a sufficient
explanation for it not doing so on a console whose XMB has already answered
*if* the redirect keyed off the system language. That specific mechanism is
now falsified - see below - but the broader "does a frame ever draw" question
carries a lower score than the mechanism does, for the reason given there.

**2026-09-23: four RPCS3 boots, crossing system language against savedata,
all leave the screen in 2-296 ms - the redirect does not key off either.**
The handover thread this closes (`is-hds-language-picker-ever-shown-to-a-player.md`)
asked for exactly this: set the PS3 system language to one HD ships no plugin
for, and see whether `Language Selection` stops there instead of leaving in
the same frame it does under a shipped language. It was tried on **two**
axes at once, because a first pass conflated them - the first attempt reused
the RPCS3 profile this project already boots with, whose
`dev_hdd0/home/00000001/savedata/BCES00664-AUTO-` skips `FirstPlay` (see the
capture above), and the picker's own `<Menu name="Language" ... save="true">`
means a previously-recorded choice is at least as plausible an explanation
for an instant exit as an unconditional redirect is. Both axes needed
splitting apart to tell them apart:

| Run | System language | Savedata | `Language Selection` entered | Left for `PreFMVConnect` | Dwell |
| --- | --- | --- | ---: | ---: | ---: |
| 1 | English (US) (`american` plugin ships) | present | - | - | same TTY.log burst |
| 2 | Polish (no plugin ships) | present | - | - | same TTY.log burst |
| 3 | English (US) | **none** (fresh profile) | 36.305750 s | 36.602016 s | **296 ms** |
| 4 | Polish | **none** (fresh profile) | 6.138388 s | 6.140439 s | **2 ms** |

Runs 3 and 4 are the falsifying pair: a genuinely fresh
`dev_hdd0/home/00000001` (no `savedata` directory at all, not one moved aside
and restored - built by giving RPCS3 its own isolated profile, `dev_flash`
firmware and `dev_hdd0/game` symlinked read-only from the real one so nothing
needed reinstalling), timestamped off `RPCS3.log`'s own `sys_tty_write` lines
rather than this driving script's poll loop. English, which matches a shipped
plugin, and Polish, which matches none, both leave within a few hundred
milliseconds of entering - **the same behaviour under both settings**, which
is exactly what the thread named as what would falsify "the redirect keys off
the system language". It does not. Runs 1 and 2, on the shared profile with a
save present, agree with runs 3 and 4 rather than explaining them: the "a
saved choice already answers it" hypothesis predicts runs 3-4 would differ
from 1-2, and they do not.

**A second, independent line in the same logs closes the string-table half of
the question too.** Before `Language Selection` is even entered, both runs
load exactly the same plugin - `RPCS3.log`: `` "Created Plugin from
\"Data\Plugins\languages\English\"" `` at 0:00:34.136 (English config) and
0:00:03.638 (Polish config), word for word, in both. Nothing in either run
ever created a second language plugin afterward, through `Studio Logo` and
into `EpilepsyWarning`. So English is not a fallback picked *because* Polish
has no plugin - it is what loads regardless, before the system language could
matter at all.

**Confidence 85** for the mechanism - `LanguageAutoRedirect` fires with no
measured dependency on system language or savedata - the same band and the
same reasoning [runtime boot order](#the-capture-that-made-this-an-order-rather-than-a-reading)
already carries: RPCS3 is not a PS3, and there is no second pressing or
platform to corroborate against, but this is now *four* independent boots
crossing two variables rather than three repeats of one. **Confidence 75**,
separately, for "so no frame is ever presented to a player" - 296 ms is still
upward of a dozen frames at 60 Hz, long enough that a frame *could* have
drawn even though nothing here could act on it, so this is an inference from
the dwell time rather than a direct capture. [ADR-0025] rejected a confidence
number in `Provenance` itself and put the rubric score here, on the page,
which is where both numbers above live rather than in the type.

**This build had the corresponding gap, now fixed.** `oag_ui::frontend::Frontend`
already skipped the picker when `settings.toml` named a language RPCS3's
save-analogue would have (`Frontend::preselect_language`), but a genuinely
first run - no settings language yet - fell through to the interactive
Up/Down/Cross path, which is exactly the screen the measurement above says
the original never presents on *any* run, first or not.
`Frontend::skip_never_shown_picker` closes that: on HD alone, with no
settings language yet, it defaults straight to `"English"` - the plugin both
matched and unmatched runs actually loaded - rather than waiting on a player.
Wired into both boot paths (`crates/game/src/main/session/load.rs`,
`crates/game/src/main/headless.rs`), tested in
`crates/ui/src/frontend/tests/intro.rs`.

#### Two `TTY.log` screen names that are not boot steps

- **`Top`** fires once per plugin created - `languages/English`, `frontend`,
  `billboards`, `grids`. It is the `<Screen name="Top">` root of each plugin's
  own `definition.xml`, above `skin.xml` entirely.
- **`Blank`** is `<Screen name="Blank" type="CommunityScreen">` in
  `DATA06`'s `Data\XML\Community\Community.xml`, loaded right after the picker.
  Another plugin's root.

Neither is a `<Screen>` in any of the six `skin.xml` copies - all six checked.
A reader diffing `TTY.log` against the table above will meet both and should not
read either as a missing chain step.

Two specific ambiguities that were once inside the declaration, both since
resolved on RPCS3 - kept here because the *XML* is still two-valued and a reader
coming to `skin.xml` fresh will meet the fork before they meet the measurement:

- **Step 4 is genuinely two-valued in the XML, and measured 2026-08-30: the
  dialog is the live path.** `EpilepsyWarning` carries *both* a `<Dialog>`
  whose single option goes to `FirstPlay` and a
  `<Redirect ... StartEnabled="false">` whose `Default goto` is `Save Warning`:

  ```xml
  <Dialog name="EpilepsyWarningDialog" StartEnabled="false">
      <Values TitleID="BOOT_HEALTH_WARN_1" Icon="Warning" BoxWidth="875" BoxHeight="550"
              TextID="BOOT_HEALTH_WARN_2" NumOptions="1" Option1="FE_CONTINUE"
              Option1Destination="FirstPlay"></Values>
  </Dialog>

  <Redirect name="EpilepsyWarningRedirect" StartEnabled="false">
      <Values backward="none"></Values>
      <Default goto="Save Warning" previous="false"></Default>
  </Redirect>
  ```

  The table above takes the dialog's path, on the reasoning that the dialog is
  what the player presses and `FirstPlay` would otherwise be unreachable. That
  reasoning is now **confidence 80** rather than an inference at 55: a boot on
  RPCS3 that only waits at `EpilepsyWarning`, pressing nothing, sits there for
  as long as it is given - both `Redirect`s in the family table below are
  `StartEnabled="false"` and neither fires unprompted. Pressing `cross` moves
  it to `FirstPlay` every time (`scripts/rpcs3-drive.py`'s
  `wait_for_screen_pressing`), matching the dialog's own
  `Option1Destination`. **Confidence 85 since 2026-09-05**, the cold boots
  above having taken the same branch twice more; not raised past that because
  this is RPCS3, not the EBOOT or real hardware.

- **`Language Selection` does run, measured the same way.** The PS3 takes its
  system language from the XMB, and the screen's own `<DisplayLanguages>`
  widget suggested the engine might read the XMB setting and skip straight to
  `PreFMVConnect` instead of showing it. It does not skip: `TTY.log`'s
  `Switching Screen` sequence on a fresh boot reads `"" -> Top -> Language
  Selection -> PreFMVConnect -> Studio Logo -> EpilepsyWarning -> ...`,
  confidence 85, three boots.

  **The screen being entered is not the screen being shown.** Both are now
  measured: entry, on these three boots at confidence 85; presenting, at the
  same confidence for the mechanism and 75 for the inference that no frame
  draws - see [what the capture did not settle, and what a later one
  did](#what-the-capture-did-not-settle-and-what-a-later-one-did).

### Two chain families - and only one of them is reachable

The six copies split, which is worth recording because it is how the dead screen
was identified. `DATA04` sits between the two families rather than in either:

| | `DATA00`, `DATA05`, `DATA06` | `DATA02`, `DATA03` | `DATA04` |
| --- | --- | --- | --- |
| picker's `Default goto` | `PreFMVConnect` | `Studio Logo` | `Studio Logo` |
| `PreFMVConnect` screen | present | absent | absent |
| `LogoFMV` screen | absent in `DATA00`/`DATA06`, orphaned in `DATA05` | present, orphaned | present, orphaned |
| `Save Warning` redirects | one, to `EULA` | two: `Main Menu`, `EULA` | three: `Main Menu`, `Update Announcement`, `EULA` |
| `EULA` leads to | `Update Announcement` | `Main Menu` | `Main Menu` |
| `Update Announcement` screen | present | absent | present |
| logo movie | `StudioLiverpool_fury.bik` (`DATA00`), `StudioLiverpool.bik` (others) | `StudioLiverpool.bik` | `StudioLiverpool.bik` |

`Save Warning` carrying two or three named `Redirect`s in the older copies, all
`StartEnabled="false"`, is a second reason the chain table above is a
declaration rather than an order: something outside the XML picks between them.

`DATA00` is the only copy naming `Data/FE/Images/StudioLiverpool_fury.bik`, and
that file exists only in `DATA00` (21,151,804 bytes); `studioliverpool.bik`
exists only in `DATA02` (15,139,496 bytes). `DATA00` is also the archive holding
the Fury-only circuits. That made `DATA00` *look* like the newest layer, and
this paragraph used to end "looking like it is not evidence. Nothing here
establishes load order."

#### `DATA00`'s copy is the live one. Confidence 92, and load order is now known

Two independent lines, both off the 2026-09-05 boots and both free - the game's
own `printf` and RPCS3's syscall log, no debugger connection needed.

**1. The game announces its own archive order.** `TTY.log`, verbatim, on every
boot:

```
PSARC file found data00.psarc          <- discovery, numeric
PSARC file found data01.psarc
... through data06 ...
PSARC file Loading data01.psarc        <- loading, and it is NOT numeric
PSARC file Loading data02.psarc
PSARC file Loading data03.psarc
PSARC file Loading data04.psarc
PSARC file Loading data05.psarc
PSARC file Loading data06.psarc
PSARC file Loading data00.psarc        <- last
```

`data00` is found first and **loaded last**. Identical across three separate
boots. That is the load order this page said nothing established, and it is
consistent with `DATA00` being the Fury layer laid over the base game.

**2. The runtime asks for a filename only `DATA00`'s copy names.** During
`Studio Logo`, `RPCS3.log` records the FIOS loose-file probe that precedes every
archive read:

```
sys_fs_stat(path=".../USRDIR/data/fe/images/studioliverpool_fury.bik") -> CELL_ENOENT
```

Extract every `.xml` from all six archives - 771 files - and grep: the string
`StudioLiverpool_fury` appears in **exactly one file on the whole disc**,
`DATA00`'s `skin.xml`. Every other copy names `StudioLiverpool.bik`. So the
`Skin.xml` the ScreenManager loaded (`ScreenManager Load
"Data\Plugins\frontend\GUI\Skin.xml"`) is `DATA00`'s.

**This retires the framing, not just the question.** `DATA02`/`DATA03` and
`DATA04` are not alternative boot paths a run might take instead - they are
superseded copies of one file that the front end never loads. The chains in the
table above are **unreachable**, not merely unexercised, and there is nothing
about them left for a capture to exercise.

It does not follow that every `DATA00` entry wins every lookup - that is a claim
about the archive layer's own resolution rule, and only `Skin.xml` was traced.
What is established is the load order and this one file.

## The declared menu tree

The top level is `mainmenu_definition.xml`'s `<Redirect>`, read verbatim:

```xml
<Redirect>
    <Values backward="none"></Values>
    <Entry item="Mode" equals="FE_RC" goto="Grid Selection"></Entry>
    <Entry item="Mode" equals="FE_RACEBOX" goto="Single Player"></Entry><? SA - Formerly went to Racebox ?>
    <Entry item="Mode" equals="FE_ONLINE" goto="NetLoginPage"></Entry>
    <Entry item="Mode" equals="FE_RECORDS" goto="ScoreSync"></Entry>
    <Entry item="Mode" equals="FE_OPT_PLUS" goto="Additional"></Entry>
    <Default goto="To Be Done"></Default>
</Redirect>
```

Five entries in the `<HorizMenu>` order given earlier, and the disc records its
own edit: **`FE_RACEBOX` goes to `Single Player`, not to `Racebox`.**
`racebox_definition.xml` declares `Single Player` as its first screen and
`Racebox` is not a screen name anywhere, so the comment describes a change that
was actually made. A reimplementation reading the file name rather than the
redirect would wire the wrong target.

Screen counts by definition file, `DATA06`, for scale rather than as a list to
implement (`docs/architecture/menus.md` owns the menu *tree*, which stays ours):

| file | screens | file | screens |
| --- | ---: | --- | ---: |
| `ingame_definition.xml` | 58 | `manual_definition_*.xml` | 6 each |
| `network_definition.xml` | 50 | `team_selection_definition.xml` | 5 |
| `skin.xml` | 38 | `cellmode_definition.xml` | 4 |
| `online_definition.xml` | 36 | `track_selection_definition.xml` | 3 |
| `additional_definition.xml` | 23 | `showunlocks_definition.xml` | 3 |
| `stats_definition.xml` | 9 | `recordgrid_definition.xml` | 2 |
| `demo_definition.xml` | 8 | `controls_definition_ps3.xml` | 2 |
| `racebox_definition.xml` | 7 | `credits_definition.xml` | 1 |
| `endrace_definition.xml` | 7 | `gallery_definition.xml` | 1 |
| `mainmenu_definition.xml` | 6 | `debug_screens.xml` | 1 |

`skin.xml`'s 38 includes duplicates: `Main Menu`, `SoundTest`, the five
`Manual Part` screens, `Controls` and `Race Records` each appear twice, inside
two sibling `<Screen name="default">` blocks. Which of the two wins is another
question for the executable.

## Filling in `MenuSkin`

```rust
pub const MENU_SKIN: &oag_title::MenuSkin = &oag_title::MenuSkin {
    // Authored, `FEGlobals->MenuXOffset` = 800, identical in all six copies of
    // `skin.xml`. **This is a 1920x1080 coordinate**, not Pulse's 480x272 one.
    menu_x: 800.0,
    // Authored, `FEGlobals->MenuScale`.
    menu_scale: 1.0,
    // Authored, `FEGlobals->TitleXOffset`. Referenced by `mainmenu_definition`'s
    // title text as `x="FEGlobals->TitleXOffset"`, so it is live, not vestigial.
    title_x: 194.0,
    // Authored, `FEGlobals->TitleYOffset`.
    title_y: 62.0,
    // Authored, `FEGlobals->TitleScale`.
    title_scale: 1.0,
    // **Not authored.** HD's main menu is a `<HorizMenu>`, so it has no first
    // row to put a y on; the 17 `<Menu>` widgets across the whole GUI tree do
    // not converge on one value. See "first_row_y is absent" below.
    first_row_y: None,
    // **Measured field, and nothing has been measured.** No capture exists.
    row_extra_leading: None,
    // **Not authored, and this one is a measurement rather than a gap.** HD's
    // language plugins declare no `Menu` font slot at all.
    menu_font: None,
    // Authored, `FEGlobals->TextColor` = 0xFFFFFFFF.
    text: Some(0xFFFF_FFFF),
    // Authored, `FEGlobals->TitleColor` = 0xFF646464.
    title: Some(0xFF64_6464),
    // **Measured field, and nothing has been measured.** No capture exists.
    selected: None,
    // Authored on HD's own `<LeftLayer transition="0.5">`, the dominant value
    // across its GUI files. Read off HD, not borrowed - but see the warning.
    transition_secs: 0.5,
    // Authored, `MainMenu_Definition.xml`'s own `<HorizMenu name="Mode">`, and
    // identical in the five archives carrying that file. See "the main menu is
    // horizontal, and it is drawn that way now" below.
    strip: Some(oag_title::MenuStrip {
        x: 160.0,
        y: 125.0,
        color: 0xFF70_5070,
        // Confirmed live, 2026-09-01 - see "The FE Style switch is live"
        // below.
        selected_fill: Some("HD_Blue"),
    }),
};
```

### Per-field evidence

| Field | Authored? | Value | Confidence | Note |
| --- | --- | --- | ---: | --- |
| `menu_x` | yes | `800.0` | 92 | `MenuXOffset`, all six copies. 1920-wide space. |
| `menu_scale` | yes | `1.0` | 92 | `MenuScale`, all six. |
| `title_x` | yes | `194.0` | 92 | `TitleXOffset`, all six, and referenced by `mainmenu_definition.xml`. |
| `title_y` | yes | `62.0` | 92 | `TitleYOffset`, all six, same reference. |
| `title_scale` | yes | `1.0` | 92 | `TitleScale`, all six. |
| `first_row_y` | **no** | `None` | 85 | No `<Menu>` on the main menu; no dominant y. |
| `row_extra_leading` | measured field | `None` | - | No capture. Needs an emulator. |
| `menu_font` | **no** | `None` | 90 | No `Menu` slot in any language plugin. |
| `text` | yes | `0xFFFFFFFF` | 92 | `TextColor`, all six - one of the globals the FE style does *not* move. |
| `title` | yes | `0xFF646464` | 90 | `TitleColor`, all six - but see the caveat. |
| `selected` | measured field | `None`, unchanged | 88 | Capture (2026-09-01): text does not brighten on selection at all on the strip idiom, so `strip::draw` no longer reads this field for a strip's text - see below. Still `None` in the title package; `rows::draw` (both PSP titles) still reads it and is untouched. |
| `transition_secs` | yes | `0.5` | 70 | Dominant `<LeftLayer transition=>`; see the warning. |
| `strip` | yes | `(160, 125, 0xff705070, Some("HD_Blue"))` | 92 / 90 | The main menu's own `<HorizMenu>`, identical in all five copies of `MainMenu_Definition.xml` (92); `selected_fill`'s name is the same capture as `selected` above (90). |

### `first_row_y` is absent, and why

`MenuSkin::first_row_y` is documented as authored *per screen* - Pulse's
`MainMenu_Definition.xml` menu widget says `y="32"`, and Pure took the dominant
value across 33 `<Menu>` widgets (19 of them `y="45"`). Neither route works on
HD, for a reason that is itself the finding:

**HD's main menu is horizontal.** `mainmenu_definition.xml` contains no `<Menu>`
widget at all:

```xml
<HorizMenu name="Mode" focus="true" transition="0.4" delay="0.2">
  <Values align="left" x="160" y="125" color="0xff705070"></Values>
  <Entry IDString="FE_RC"></Entry>
  <Entry IDString="FE_RACEBOX"></Entry>
  <Entry IDString="FE_ONLINE"></Entry>
  <Entry IDString="FE_OPT_PLUS"></Entry>
  <Entry IDString="FE_RECORDS"></Entry>
</HorizMenu>
```

A five-entry horizontal strip at (160, 125) has no "top of the first row" in the
sense the field means. The same `<HorizMenu ... x="160" y="125">` shape recurs
in `additional_definition.xml` for the options and controls pages, so this is
HD's menu idiom rather than one screen's exception.

The vertical `<Menu>` widget still exists, but sparsely and without agreement.
Across `DATA06`'s 29 frontend files, 17 `<Menu>` widgets:

| `y=` | count | where |
| --- | ---: | --- |
| (none) | 3 | `UserList` x3, `CreateOption` |
| `32` | 4 | `online_definition.xml` login/friend/block/pending |
| `FEGlobals->MenuYOffset` | 3 | `additional_definition.xml` `Option` |
| `82` | 3 | `online_definition.xml` `UserList` |
| `184`, `700`, `160`, `45`, `130`, `80`, `120` | 1 each | scattered |

The best-supported literal is 4 of 17 (24%), against Pure's 19 of 33 (58%). No
dominant value exists.

**There is an alternative reading, and it is deliberately not taken.** HD
authors `FEGlobals->MenuYOffset = 300`, a global Pulse does not have at all, and
three `<Menu>` widgets consume it as their `y`. Mapping `MenuYOffset` onto
`first_row_y` would be an *interpretation* - the field is documented as
per-screen, `MenuYOffset` is not the main menu's, and no screen this build would
draw uses it. Recording it here as an authored fact and leaving the field `None`
keeps the two separable. If a later pass wants it, the number is `300.0` and the
change is one line; inventing it now would make an interpretation look like a
reading.

### The main menu is horizontal, and it is drawn that way now

**Added 2026-08-19.** The paragraph above has said "HD's main menu is a
`<HorizMenu>`" since this page was written, and until now that was a reason a
field was empty rather than something anything drew: `oag-game` laid every menu
out as a column, so HD's root page came out as a stack of rows at
`MenuXOffset` 800 - a layout its own disc does not author anywhere.

The widget, out of `DATA06` and identical in the other four copies:

```xml
<HorizMenu name="Mode" focus="true" transition="0.4" delay="0.2">
  <Values align="left" x="160" y="125" color="0xff705070"></Values>
  <Entry IDString="FE_RC"></Entry>
  <Entry IDString="FE_RACEBOX"></Entry>
  <Entry IDString="FE_ONLINE"></Entry>
  <Entry IDString="FE_OPT_PLUS"></Entry>
  <Entry IDString="FE_RECORDS"></Entry>
</HorizMenu>
```

**Confidence 92**, the same band and for the same reason as the `FEGlobals`
above: five archives carry `MainMenu_Definition.xml` - `DATA00`, `DATA02`,
`DATA03`, `DATA05`, `DATA06`, the other two carrying none - and all five write
those three numbers identically. `crates/game/tests/hd_menu_ground_truth.rs`
asserts that against every copy rather than against one, so the day the archives
diverge is a failure rather than a coin toss.

#### It is an axis, and here is the measurement that licensed it

[ADR-0022] wants a second corpus before a constant becomes a field, and the two
PSP titles supply it by authoring **no such widget at all**. Measured rather than
assumed - every blob of the named archives was extracted and searched, and the
`Menu` column is the control:

| disc | archives read | files | holding `Menu` | holding `HorizMenu` |
| --- | --- | ---: | ---: | ---: |
| `pulse-psp-usa.chd` | `Data.wad`, `FE.wad`, `FEData.wad` | 1,411 | 30 | **0** |
| `pulse-psp-eu.chd` | `Data.wad` | 1,138 | 27 | **0** |
| `pulse-ps2-eu.chd` | `WADSP.WAD`, `WADS2.WAD` | 7,393 | 76 | **0** |
| `pure-psp-eu.chd` | `Data.wad`, `FE.wad`, `FEData.wad` | 1,241 | 46 | **0** |
| `hdfury-ps3-eu-dec.iso` | `DATA06`'s front-end tree | 29 | 18 | **8** |

A string search finds a shortened element because the PSP dialect writes the
full name into each file's own `<code>` dictionary - see `oag_tables::fexml`.
The PS2 pressing's other two archives, `PRERACE.WAD` and `PS2MUSIC.WAD`, are
swept too (2026-09-01) and also come back **0** for both columns - but not
because the vocabulary is absent from them. Both are `oag_formats::ps2_music`-
shaped raw-PCM containers, not `fexml`, so neither can hold an XML widget by
format at all. `WADSP.WAD`/`WADS2.WAD` are the whole of that disc's front end.

Within HD it is an idiom rather than one screen's exception: eight files carry
ten `<HorizMenu>` widgets against seventeen vertical `<Menu>`, all ten at
`x="160"` and `align="left"`, six at `y="125"` and four - the manual pages - at
`y="140"`. Eight of the ten use `0xff705070`; the two in
`network_definition.xml`/`online_definition.xml` use `0xffffffff`.

#### Which pages this build draws as a strip, and why that is ours

The menu **tree** is still this project's own - see
[menus](../architecture/menus.md), which this changes nothing about. What follows
the disc is where the entries go.

`oag_game::menu::strip::suits` draws a page as a strip when every entry is
navigation - a submenu, an action, or the way back - because a strip has nowhere
to put the value column a row list anchors on the right. **That rule is ours.**
It is *shaped* by HD, whose `<HorizMenu>` screens (`Main Menu`, `Additional`,
`Extras`, `Controls Menu`) are all pages that only choose where to go next while
`Settings` and its neighbours are `<Item>` lists with values - a consistent split
and not a declaration.

**On the shipped `assets/ui/menu.toml` it fires on the root page alone**, and
that is worth stating plainly rather than implying the rule tracks HD's idiom
generally: `options` is this build's counterpart to `Additional` and would be a
strip on HD's own reading, but a `choice` hangs off it here, so it keeps its
column. Of the seven pages, `main` is the only one with no `choice`, `toggle` or
`binding`.

Two more things are ours and marked where they are made: the **gap** between two
entries (`STRIP_GAP` in `crates/game/src/menu/skin.rs` - the widget states no
`gap` and there is no second anchor to derive one from), and the **selected**
colour, `MenuSkin::selected` being a measured field with no HD capture behind it.
Left and right step a strip, because that is the axis it is drawn on; up and down
keep working, which is an affordance rather than a reading.

**Every entry is drawn, and there is no carousel.** Nothing in the widget says
the selection is centred or that entries off the end are hidden, so all of them
run from the anchor. That is the only reading the data supports; a capture would
settle whether the original scrolls.

### The frame the menus sit in, and it is read rather than transcribed

**Added 2026-08-19, with the strip.** A horizontal menu on a black field is
still not what this disc draws. HD authors a *frame* - the rules above and below
the page, the mark in the top left, and what the whole thing is cleared to - and
authors all of it on **one screen**, `FE Screen`, the parent every menu screen
is nested inside. Read out of `DATA00`'s `skin.xml`:

```xml
<Screen name="FE Screen">
    <ScreenClear><Values Colour="FEGlobals->HD_BG"></Values></ScreenClear>
    ...
    <Image transition="0.3" delay="0.1">
        <Values transitiontype="hscale" x="160" y="110" width="1600" height="8"
                color="FEGlobals->HD_Grey" src="Data\FE\Images\line.gtf"></Values>
    </Image>
    <Image transition="0">
        <Values x="160" y="73" width="32" height="32"
                color="FEGlobals->HD_Grey" src="Data\FE\Images\Title_Arrow_HD.gtf"></Values>
    </Image>
    <Image transition="0.3" delay="0.3">
        <Values transitiontype="hscale" x="160" y="975" width="1600" height="8"
                color="FEGlobals->HD_Grey" src="Data\FE\Images\line.gtf"></Values>
    </Image>
```

`line.gtf` is an 8x8 tile stretched to 1600 wide, which is why the widget's own
size has to win over the texture's: drawn at its own size it is eight pixels of
what should be most of the screen.

**The title package names the screen and nothing else** -
`oag_title::FrontEnd::menu_frame` is `Some("FE Screen")` - and
`oag_game::menu::frame` reads the widgets off it at boot. That is deliberate and
it is `CLAUDE.md`'s rule: a table of `(160, 110, 1600, 8)` in a Rust file would
be correct to the digit and still wrong, because it would not survive the disc
being re-read and would say nothing about the other twenty-odd frames on this
disc. Reported at boot as `menu frame FE Screen: clears to #000000, 3 mark(s),
ink #969696`, so what a menu is sitting in is visible without a screenshot.

**Its `<Text>` widgets are deliberately not drawn.** They are the trial build's
(`FE_TRIAL_MODE`, `FE_PURCHASE_NOW`) or belong to a `<NavigationController>`,
which decides per screen which button prompts to show and which this build does
not have; the three images and the clear carry no such owner. Drawing everything
on the screen would put "purchase now" across a retail menu.

### The `HD_*` palette is the FE style, and the archives disagree

**This is the finding of the pass, and it arrived by contradiction.** Reading
`DATA06` says HD's front end is white: `HD_BG` `0xffffffff`, `HD_Grey`
`0xff646464`, `HD_Blue` `0xff8ac0ca`. A menu drawn on that would need grey rows,
because `FEGlobals->TextColor` is white and white on white is nothing. So the
first attempt at this change put `MenuSkin::text` at grey - and then the disc-
backed test failed, because the archive the boot actually serves is `DATA00`,
which declares **the same six globals with different values**:

| global | `DATA00` | `DATA06` |
| --- | --- | --- |
| `HD_BG` | `0xff000000` (black) | `0xffffffff` (white) |
| `HD_transBG` | `0xe0000000` | `0xe0ffffff` |
| `HD_Grey` | `0xff969696` | `0xff646464` |
| `HD_LightGrey` | `0x7f646464` | `0xffdedede` |
| `HD_Blue` | `0xffac0717` (red) | `0xff8ac0ca` (teal) |
| `HD_White` | `0xffffffff` | `0xffffffff` |

Black with red against white with teal: **those are Wipeout Fury and Wipeout HD**,
and `additional_definition.xml` names the switch between them -

```xml
<List name="FE Style" focus="true" global="FE Style" save="true" default="HD" delay="0.5">
  <Values idstring="OPT_FE_STYLE" ...></Values>
  <Entry string="HD"></Entry>
  <Entry string="FURY"></Entry>
</List>
```

`DATA00` is the archive holding the Fury circuits and the `_fury` logo reel, so
the two palettes travel with the two layers of content. **Confidence 88** that
the `HD_*` block is the FE style: two archives, six globals, every one differing
in the direction the two games' art directions differ, and a named option with
exactly those two entries. What is *not* read is the code that chooses, so
whether the option rebinds the globals or the engine loads a different archive's
`skin.xml` is open - and it is the same "which copy does the runtime serve?"
question this page has carried since it was written.

Three consequences, all of them live:

1. **`FEGlobals->TextColor` and `TitleColor` are not part of it.** Both archives
   declare them identically (`0xFFFFFFFF`, `0xFF646464`), which is why
   `MenuSkin::text` can hold a value at all and why the "six copies agree" table
   above is still true of everything it lists.
2. **`MenuSkin::selected` stays `None` although the disc states a highlight.**
   Twenty-five `highlightColor` attributes, every one `FEGlobals->HD_Blue`,
   always beside a `color` of `HD_Grey` - so the disc does answer the question
   this measured field asks. It answers it with a *global whose value is the
   style*, and a constant in `oag-hd` would hard-code one game's look into a
   table that cannot say which.
3. **This build shows the Fury palette**, because `DATA00` serves the front-end
   root: a black field, `0xff969696` rules, and the menu title in the same grey.
   That is coherent - it is one archive's own answer, not a mixture - and it is
   a *choice of archive* rather than a reading. Serving `DATA06`'s root instead
   would give the white style, and would change the boot chain and the logo reel
   with it.

The menu title is drawn in the frame's own ink rather than in `TitleColor` for
the same reason: HD's `Main Menu` authors its title `color="FEGlobals->HD_Grey"`
and the frame's three widgets carry that same global, so "the title is the
colour of the rules" is read off the disc, and taking it from the marks makes it
resolve to whichever archive was served instead of being pinned to one.

### The FE Style switch is live, works, and is not an archive swap - confirmed by capture

**2026-09-01, `scripts/rpcs3-drive.py`, RPCS3 booted the real disc.** The
previous section's open question - "what is not read is the code that chooses"
- is now answered for the practical case: **"Menu Style" (the on-disc label
for `OPT_FE_STYLE`) is a real, working, in-game setting**, reached from `Main
Menu` -> `OPTIONS` -> `GAME`, the last row of a nine-row list. Selecting it
repaints the whole `Game Options` screen from the Fury palette (black field,
`0xff969696` grey, `0xffac0717` red) to the HD one (white field, `0xff646464`
grey, `0xff8ac0ca` teal) in the same frame the value changes - no reload, no
loading screen. `global="FE Style" save="true"` is not decorative either: a
completely fresh boot, no navigation at all, came back with `Main Menu` itself
in the HD palette, so the choice is a persisted profile value the front end
reads on every screen it draws, not a per-session toggle. **Confidence 90** -
a runtime capture showing the exact behaviour claimed, on the one binary this
project has; the two-archive reading from the previous section corroborates it
independently. Screenshots (gitignored, not committed - `data/reference/` on
the machine that captured them):
`hd-main-menu-screenshot/00.png` and `hd-menu-highlight-shift/00-campaign.png`
are the same screen, Fury and HD side by side.

**What this also settles: `additional_definition.xml` is not served from
`DATA00`, and not from `DATA02` either.** `DATA00` carries no copy of the file
at all (`mainmenu_definition.xml` and `Skin.xml` are the only two front-end
XMLs it ships), and `DATA02`'s copy declares five rows under `Settings` -
`Camera`, `Safe Area Setting`, `Multiplayer Tags`, `Ghost Ship Visible`,
`Pilot Assist` - with no `HUD Style` and no `FE Style` at all. The live
`Game Options` screen has nine, in this exact order: `Camera`, `Screen Size`,
`Multiplayer Tags`, `Ghost Ships Visible`, `Pilot Assist - Player 1`,
`Pilot Assist - Player 2`, `HUD Style`, `HUD Effects`, `Menu Style` - which is
`DATA06`'s copy, field for field (`CameraP1`, `Safe Area Setting`,
`Multiplayer Tags`, `GhostOption`, `Pilot Assist`, `Pilot Assist 2`,
`HUD Style`, `HUD Effects`, `FE Style`, in that order; `CameraP2` sits between
the two camera rows with `StartEnabled="false"` and does not show). So **the
served front end is not one archive** - `DATA00` overrides the root screens
(`mainmenu_definition.xml`, `Skin.xml`) and `DATA06` supplies files `DATA00`
does not carry at all, `additional_definition.xml` among them. Which rule
picks `DATA06` over `DATA02` for this one file, and whether it is the same
rule that picks `DATA00` for the root, is still open - what changed is that
"the archive" is now known to be a *per-file* question, not a single answer
that resolves everything downstream of it.

**`MenuSkin::selected` is resolved, and it is not what the field was for -
and this is now implemented, not just documented.** The capture shows
unselected and selected entries in the *identical* white `TextColor` -
"CAMPAIGN" (selected) and "RACEBOX" (not) read as the same white, at the same
weight, in both the Fury and the HD shot. What changes is the entry's own
background: a tab, filled `HD_Grey` when unselected and `HD_Blue` when
selected (red on Fury, teal on HD - exactly the archive's own value, matching
the 25 `highlightColor` attributes already read as `HD_Blue`), plus a short
underline mark under the selected label alone, in the label's own white
rather than either fill colour. **So brightening the text was the wrong
mechanism**: `strip::draw` no longer reads `Skin::selected()` for a strip's
text at all - every entry draws in `Skin::normal()` - and the highlight moved
entirely onto a new background fill. `Skin::selected()` itself is untouched:
`rows::draw` (both PSP titles) still reads it, and nothing here says whether
its own "brighten toward white" reading is right for a column, only that it
is wrong for a strip. **Confidence 88** for the finding; the fill and
underline colours (`HD_Grey`/`HD_Blue`, read live off the served archive's
own globals - `oag_title::MenuStrip::selected_fill`, resolved the same way
`crate::loading`'s own palette is) are **confirmed exact reads**, matched to
the capture's own pixels to the byte. The tab's *shape* is not: nothing on
disc states one at all, and what shipped is a rectangle sized from
`measure(label)` plus a left/top pad, at **confidence 55** - the real tab's
top-right corner is chamfered, `17`x`7` pixels in a `1278`-wide capture
against a `188`-`232`-pixel-wide tab, left edge vertical throughout, and
reproducing that cut as a true diagonal needs a primitive this build's
`Draw`/`Quad` pipeline does not have. **Not a shear** - a shear moves both
right corners into a parallelogram, which is the wrong shape - but an offset
on the one top-right corner alone (a whole-quad `rotation` only exists today,
see `crates/game/src/ui.wgsl`), and that primitive's blast radius (every
`Quad` in the game shares its vertex layout) was judged not worth paying for
one small corner. **2026-09-02: shipped anyway, as a one-step band rather
than a plain rectangle** - `strip::draw` now draws the tab as two
`Draw::Fill`s, the full tab below the chamfer's height and a second band
above it short by the chamfer's width, using the same raw measurement. A
render comparison against the capture found the honest result: at a normal
crop the step and the real bevel read the same, but a close zoom shows the
step is a right angle where the capture is a diagonal - a resolution limit
from drawing the true cut with an existing primitive, not an invented shape.
`crates/game/src/menu/strip.rs`, `crates/game/src/menu/frame.rs`,
`crates/game/src/menu/skin.rs`.

### The tab's corner, pixel-measured: a 45-degree cut *and* a flat landing

**Confidence 88**, measured 2026-09-02 on two independent captures of the same
screen - one at HD's own `1280x720` output and one at `3840x2160`, three times
the linear resolution, through RPCS3's framebuffer grab rather than a
root-window one (see
[rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md#a-root-window-grab-is-not-the-framebuffer-and-only-one-of-them-can-measure-geometry)).
Both agree, and both say the same thing: **the previous `17`x`7` reading
measured the whole cut region and called all of it the chamfer.** It is not.

Taking the selected `CAMPAIGN` tab's right edge row by row, in the `3840`-wide
framebuffer:

| Rows | Right edge | What that is |
| --- | --- | --- |
| `y=26`..`y=44` (19) | `x=55` -> `x=72` (18) | a **45-degree** diagonal: 18 across in 19 down |
| `y=45` onward | `x=107`, unchanging | the tab's own right edge |

So between the diagonal's foot (`x=72`) and the right edge (`x=107`) there are
**35 more pixels of flat top edge**. The corner is a pentagon: high top edge,
a 45-degree cut down, a *flat landing* at the lower level, then the vertical
right edge - `---\___|`, not `---\|`.

Converted to the `1278`-wide capture's own units (divide by three), and then to
this build's (`* 0.3756`):

| | raw px at 720p | 480-grid units | shipped today |
| --- | ---: | ---: | --- |
| diagonal width | `6.0` | `2.25` | - |
| flat landing | `11.7` | `4.4` | - |
| whole cut region | `17.7` | `6.6` | `TAB_CHAMFER_WIDTH = 6.4` |
| cut height | `6.3` | `2.4` | `TAB_CHAMFER_HEIGHT = 2.6` |

The `1280x720` capture measured independently gives `6` columns of diagonal
(`x=103`..`109` over `y=5`..`11`) and a `12`-pixel landing - the same numbers
within a pixel, from a capture taken on a different boot at a third of the
resolution.

**This retires the "step or diagonal" question by answering both halves.** The
one-step band that shipped had the cut's *extent* right (`6.4` against a
measured `6.6`) and its *slope* wrong; the `Draw::ChamferedFill` primitive built
and reverted on 2026-09-01 had the slope right and ran the diagonal the whole
`6.4` to the corner, with no landing - which is exactly the "the original looks
like a step, ours is just a cut angle" the direct comparison reported. Neither
was wrong about what it saw. The shape needs both: a `2.25`-unit 45-degree cut
followed by a `4.4`-unit flat run.

**Drawn as of 2026-09-02, and checked at HD's own resolution rather than by
eye.** `Draw::ChamferedFill` is back, applied to a *band* the height of the cut
and narrowed by the landing, over an ordinary `Draw::Fill` - so the landing is
the gap between the band's right edge and the tab's, and nothing draws it.
Rendering `--menu-page main --screenshot --size 1920x1080` puts this build in
HD's own grid, where the corner is directly comparable with the capture:

| | diagonal across | diagonal down | landing |
| --- | ---: | ---: | ---: |
| the real menu | `9` | `9.5` | `17.5` |
| this build | `9` | `9` | `18` |

Within half a pixel on all three. `crates/game/src/menu/strip.rs`,
`crates/game/src/menu/skin.rs`, `crates/game/src/ui.wgsl`.

**It is rasterised geometry, not a texture mask.** The diagonal's per-row steps
at `3840` are `+2,0,+2,0,+1,+1,+1,+1,+1,+2,0,+2,0` - irregular, averaging one,
which is what a line rasteriser does at a non-integer position. A magnified
16x16 mask (`corner2.gtf`, the `<Bracket>` asset the previous pass found) would
show uniform runs at the magnification factor instead. So the `<Bracket>`
corner asset is **not** what draws this, and the open question of whether the
two share a design language is now moot for this widget: it does not sample a
corner texture at all.

**2026-09-02: what `0xff705070` is for, from a wider sweep rather than the one instance already read.** Every `<HorizMenu>`/`<VertMenu>` widget's own top-level `<Values>` across `mainmenu_definition.xml`, `additional_definition.xml` (two `<HorizMenu>`, one `<VertMenu>`) and all four `manual_definition*.xml` copies carries this exact literal - eight instances, matching the table row above, all the *widget's own* declared colour and none of them reached through `FEGlobals->`, unlike every colour already established as live (`HD_Grey`, `HD_Blue`, `TitleColor`...). The already-captured case (`strip`, this section, above) shows this exact mechanism - a menu-family widget's own `color=` - going unread, text drawing in `TextColor` regardless. Structurally the same widget field on the other seven instances, unconfirmed individually - **confidence 70**, one capture generalised across a consistent pattern rather than seven more captures.

**`online_definition.xml` (`DATA02`/`DATA05`/`DATA06`, fourteen instances per copy) is a different picture, and argues the literal is not simply dead everywhere it appears.** Past the `OnlineMenu` `<HorizMenu>` itself (which *does* use `0xffffffff`, the two-of-ten exception the table above already counts), the same `0xff705070` recurs on five ordinary `<Menu>` widgets (`LoginMenu`, `UserList` x2, `FriendOptions`, `BlockOptions`, `PendingOptions`) and six standalone `<Text>` widgets carrying community/friends status strings (`friendRequests`, `FriendInfo`, `SkinInfo`, `blockedInfo`, `statusInfo`, `pendingInfo`) - every one an info/status label, not a navigation entry. A `<Text>` widget is a materially different draw than a menu entry in this build's own equivalent code (`Frontend::draw_screen_at` reads a parsed `Text`'s own `color` directly, unlike `strip::draw`/`rows::draw` which read a skin-level colour instead) - so nothing here says a `<Text>` widget's own colour goes unread the way a menu entry's does, and the online/community screens are not implemented in this project to check against a capture either way. **Confidence 55** for "this is likely a real, deliberately-chosen secondary/muted text colour on `<Text>` widgets specifically, and likely inert on the `<Menu>`-family widgets beside it the same way it is on the captured strip" - plausible from the pattern and from this build's own analogous code split, not verified against the executable or a capture of the online screen, which does not exist in this project's evidence.

**No carousel, confirmed rather than assumed.** Stepping the highlight to
`RECORDS` - the last of the five main-menu entries - leaves it sitting at its
own natural position at the right end of the strip; nothing re-centres it and
nothing scrolls. That is the reading `strip::draw` already implements, now
with a capture behind it instead of only the widget's own silence on the
question. **Confidence 90.**

**The menu backdrop's mechanism gets one more anchor, and one caution.**
`renderer.md`'s shader-registry census (a fixed cost of reading the whole
`EBOOT.elf`, not new work for this) names nine `FEBackgroundAnim*` programs,
two of them `FEBackgroundAnimFuryWave` and `FEBackgroundAnimFuryBlend` -
sharing the `<BackgroundAnim>` widget's own name and, in two of the nine,
naming Fury specifically. The Fury-style capture shows a dense, animated
red-and-gold particle/ember field with no discernible mesh silhouette in it
anywhere; two captures a boot apart differ in exact shape, so it is animated
rather than a static image, consistent with a real-time render. **The HD-style
capture shows nothing at all** - a flat white field, no particle motion,
nothing where the Fury capture is busy. Read together with the shader names,
the more precise hypothesis is that `FrontEndScene_HD_ATG.vex`/`.rcsmodel`
supply geometry a **style-specific shader** draws (a `*Wave`/`*Blend` pair for
Fury, something else or nothing for HD) rather than one mesh rendered the same
way regardless of style - which the previous confidence-88 reading, written
before either style had been seen running, could not have distinguished from
"the scene simply is not authored for HD." **Still open**: which of the seven
non-Fury `FEBackgroundAnim*` names (if any) HD's style resolves to, and
whether HD genuinely draws nothing or draws something this capture's settle
time was too short to catch mid-transition (`Skin.xml`'s
`<ScreenSetting name="Main Menu" ... blur="0">` at least says HD's own author
also thought the Main Menu screen ought to be sharp, for what that is worth).
Neither this build nor `oag-render` draws any of it today - open
implementation work, not yet done.

**2026-09-02: the geometry decodes, and its own material argues against the
"style-specific shader draws it" hypothesis above.** `.rcsmodel`/`.vex`
decoding landed elsewhere since that paragraph was written - the chunk-header
and per-chunk-space findings on [rcsmodel.md](rcsmodel.md) - which unblocked
reading the file directly:

```sh
cargo run -q -p oag-view --bin oag-view -- \
  "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC" \
  --mesh 'Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex' \
  --screenshot /tmp/fescene.png
```

parses and draws clean: 56 of 56 mesh nodes, 9,184 triangles, 18,315 authored
vertex normals, radius 333.10 - a ring-shaped lattice of small repeating
segments, unlit and untextured (the shipped lit-race pass does not resolve a
shader for it - see below). It has exactly **one** material, named by
`PS3_GAME/USRDIR/DATA02.PSARC`'s own manifest:
`/data/materials/frontendscene/basic_vertexemissive.rcsmaterial`. That is an
ordinary per-surface `.rcsmaterial` - the same container every track and ship
material ships - and **not** one of the 121 engine-owned `_vp`/`_fp` names
`renderer.md`'s shader-registry census reads out of `EBOOT.elf`, `FEBackgroundAnim*`
among them. It carries exactly one variant: class `RigidBody`, feature set
`HalfBrightAmbientSunSpot0SVC0` - the plain sun-plus-ambient lit-race
permutation [`oag_rcs::rcsmaterial::Features::chunk_word`]'s own doctest
names for "no lightmap coordinate, no vertex colour set" (`LIT_RACE_PASS`,
`decl: None`), despite the file's own name promising a vertex-driven emissive
term. Read together, this favours a different picture than "a style-specific
`FEBackgroundAnim*` shader draws this same mesh": the mesh's own material asks
to be lit like an ordinary in-race `RigidBody` - sun colour, sun direction,
ambient - which a front-end screen has no `.envsettings` to supply at all, so
even a build that shipped this exact variant would have no authored light to
feed it. The `FEBackgroundAnimFuryWave`/`FEBackgroundAnimFuryBlend` pair
sharing the `<BackgroundAnim>` widget's name is more likely a **separate**
full-screen effect layered over or instead of this geometry than the shader
this geometry's own material asks for. **Confidence 60** - the variant key is
read off the file, not guessed, but which pass actually draws this scene
in-game (and whether the key this project computes is the one the executable
would) is not traced from the executable side. Neither this build nor
`oag-render` draws any of it - still open implementation work, now with a
narrower and more specific blocker than "the decoder doesn't exist yet".

### The strip, measured against a disc-calibrated capture

**2026-09-05.** The strip's geometry was re-measured off a 3840x2160 RPCS3
framebuffer grab of the real main menu, at a ruler solved from the picture
rather than assumed from its resolution. That corrected one structural error and
one constant, and settled three questions that had been open for want of a
capture.

#### The ruler

`data/reference/hd-menu-tab-corner/mainmenu-3840x2160.png` (gitignored) shows
the **HD** style - white page, rules `#646464`, selected tab `#8ac0ca`. The
scale is solved from two *disc-authored* marks in the same frame, the `FE Screen`
frame's own two `line.gtf` rules at `x=160 y=110 w=1600 h=8` and `y=975`:

**`capture = authored * 1.92 + (76.8, 43.2)`**

- The rules' ink runs capture `x=384..3455` with no antialiasing on either edge:
  width `3072.0` = `1600 * 1.92` exactly, giving `ox = 76.8` exactly.
- Coverage-weighted top edges are capture `260.17` and `1920.96`, `1660.79`
  apart against an authored `865`: `1.9200`.
- `76.8 = 3840 * 0.02` and `43.2 = 2160 * 0.02` - a centred 4% underscan, the
  two axes agreeing without being fitted to one another.
- Cross-checked on `Title_Arrow_HD.gtf`, which shares no property with `line.gtf`
  but the screen: authored `x 161..182, y 73..105` predicts capture
  `385.9..426.2` and `183.4..244.8`, and it measures `385..426` and `183..245`.

**The trap it nearly became.** `line.gtf` is an 8x8 tile whose opaque rows are
`3..6` of 8, so the *visible* rule is 4 units tall at authored `y=113..116`
inclusive, not 8 at `110..117`. Equating the ink's top edge with the
authored `y=110` yields an offset 3 units out and makes every later number
wrong by the same amount. This build already renders that correctly - its rules
draw at `y=113..116` and `y=978..981` - so the inset is a property of the asset,
not a gap.

#### What it measured

All in authored `1920x1080` units.

| quantity | real | this build, before | after |
| --- | --- | --- | --- |
| tab's left edge | `160.5` | `150.8` | `160.0` |
| tab's top edge | `126.0` | `117.5` | `125.0` |
| tab height | `62.0` | `56.8` | `62.0` |
| tab left pad | `9.4 .. 9.9` | `9.2` | unchanged |
| label cap top, below the tab top | `8.27` | `17.0` | unchanged - see below |
| gap between tab fills | `11.46, 10.94, 11.46, 11.46` | `12.0` | unchanged |
| underline width / height | `20.8` / `7.82` | `19.6` / `7.5` | unchanged |
| underline top | `167.1` | - | `168.0` |

**The structural error: the anchor is the tab's corner, not the label's pen.**
`<HorizMenu>` authors `x="160" y="125"`, and the real menu puts the *tab's*
top-left corner there to within a unit on both axes. This build read it as the
label's pen and derived the tab by padding outward and upward, putting every tab
one left-pad too far left and one top-pad too high. The pads' magnitudes were
right; only which edge they hang off was wrong. Fixed 2026-09-05; confidence 85
(three independent anchors in one capture, both axes agreeing).

**`TAB_HEIGHT` re-derived** from `14.3` to `15.6` in this build's 480x272 grid -
`62.0` authored, from coverage-weighted tab edges at capture `285.13` and
`404.17`. Measured on a selected *and* an unselected tab and equal to four
digits, which mattered because the selected tab is 24% *wider* than its
neighbours, so height had to be shown not to vary with selection. Confidence 75,
against the surrounding group's 55: the ruler is disc-authored marks in the same
frame rather than an assumed downscale.

#### Tab width is an engine default, and the disc says so by declining to author it

The four unselected tabs are `296.88` wide **each, to two decimals**, while their
labels are not the same length; the selected one is `366.67`; the gaps are
`~11.3`; and the strip's outer edges are the frame rules' own `160` and `1760`.
The total is conserved - `366.67 + 4*296.88 + 4*11.3 = 1599.8` - with the
selected tab taking its extra width from the other four rather than from the
strip.

**None of that is authored.** Every `<HorizMenu>` `<Values>` line on the disc:

```
 21  <Values align="left" x="160" y="140" color="0xff705070">
  9  <Values align="left" x="160" y="125" scale="1" color="0xff705070">
  6  <Values GSDisableEntriesBitField="0x0D" align="left" gap="15" x="160" y="125" color="0xffffffff">
  5  <Values align="left" x="160" y="125" color="0xff705070">            <? the main menu ?>
  2  <Values align="left" gap="15" x="160" y="125" color="0xffffffff">
  1  <Values align="left" gap="15" x="160" y="125" ItemWidth="375" color="0xffffffff">
```

The widget accepts `gap` and `ItemWidth`, and `online_definition.xml` uses both
when it wants fixed-width tabs at a fixed pitch. **The main menu sets neither,
and its five `<Entry>` elements carry `IDString` and nothing else.** So `296.88`
and `11.46` are runtime engine defaults, and the screen that *does* hard-code a
tab width is the evidence for that rather than a counter-example.

Hence this build still sizes a tab to its label, and hard-coding `296.88` would
be inventing a constant the disc deliberately declines to state - `CLAUDE.md`'s
hand-transcribed-table rule applied to layout. Recorded as measured-and-unauthored.

#### Three questions this closes

- **The highlight does not pulse.** No `blink`, `flash`, `cycle`, `oscill`,
  `sine`, `frequency`, `amplitude`, `period` or `phase` exists anywhere in the
  front-end attribute census; `<Key>` carries only `Time`, `X` and `Y`, so a
  colour cannot be keyframed at all; `Loop` is `false` on every `<Animation>` in
  `DATA00` and `DATA06`. `pulse="true"` is a real feature used on four
  boot/placeholder strings and **never on a menu `<Entry>`**. The pixels agree:
  the strip band is unchanged between two captures of the same screen at
  different times. Drawing the highlight flat is correct for HD. Confidence 88.
- **No easing curve is authored.** 122 `<Key>` elements carry `Time`, `X`, `Y`
  and nothing else; the shape of a motion is authored by adding a key, not by
  naming a curve. Interpolation is hard-coded in the executable, so recovering a
  real curve is executable-side work and the data route is closed. Confidence 90.
- **There is no menu background video.** Six `<Movie>` elements exist across all
  seven archives, naming the `Studio Logo` reel, the track-selection `FlyByMovie`
  (whose `src` is a placeholder - the per-track preview is chosen at runtime),
  and a PSP-era intro backdrop absent from both shipped skins. **No menu screen
  references a movie as a background and the main menu references none at all**;
  its backdrop is `<BackgroundAnim>`, a real-time scene. Confidence 88.

#### Still open on the strip

- **The label sits `8.7` units too low inside its tab.** The real cap top is
  `8.27` below the tab's top; this build's is `17.0`. The residual is this
  build's own font atlas - the text pen anchors the glyph *cell*, which carries
  `9.46` units of ascender headroom above the cap at this scale - not the menu
  geometry. Correcting it in `TAB_TOP_PAD` alone would need `-1.19` authored
  units, a negative "pad" whose real content is "the authored inset minus this
  build's atlas headroom", which rots silently the moment the atlas changes. The
  honest fix is per-role `.fnt` loading with metrics read back, which
  `crate::frontend::font_line_height`'s own doc already names; after it,
  `TAB_TOP_PAD` becomes the measured `8.27` authored with no atlas term in it.
- The title authors `random="true"` - a character-scramble reveal - and this
  build draws it plain.

### 2026-09-14: the tab is a `Block`, and the executable says every number the capture measured

**The strip section above is now a measurement of code that has been read.**
`Block_Item.cpp`, `HorizMenu_Item.cpp` and `List_Item.cpp` in `EBOOT.elf` are
what draw the box behind an entry, and
[menu-blocks.md](../ghidra/functions/ps3-hdfury-eu/menu-blocks.md) is the
evidence page - every address, constant and UV. What it settles, against the
numbers above:

| Measured above | Executable | Where |
| --- | --- | --- |
| unselected tab `296.88` wide | `ItemWidth` default **`298.0`**, less the one-unit border inset each side | `HorizMenu_Construct` |
| selected tab `366.67` wide | `ItemWidth + 70.0` | `HorizMenu_LayoutBlocks` |
| gap `11.3..11.46` | pitch is width **`+ 10.0`**, the inset landing in the gap | `HorizMenu_LayoutBlocks` |
| tab height `62.0`, top at `126.0` for `y=125` | block height **`64.0`**, its three-texel border starting one texel in | `HorizMenu_AddEntryBlock`, `file2.gtf` |
| label inset `9.4..9.9` | **`10.0`** | `HorizMenu_LayoutBlocks` |
| diagonal `9`, landing `17.5` at 1080p | a 10-unit 45-degree cut and a band **17** short of the fill, whose one-unit overhang makes the landing 18 | `Block_DrawTopBand`, `0x008ac594` |
| underline top `167.1` for `y=125` | image at `y + 35`, its bar at the sheet's row 7 | `HorizMenu_LayoutBlocks`, `cursor.gtf` |

`368 + 4 * 298 + 4 * 10 = 1600`, the frame rules' own span, which the
measured `1599.8` was. **"Tab width is an engine default, and the disc says so
by declining to author it" stands, and the default is now recovered rather
than measured** - so `oag_hd::frontend::MENU_BLOCKS` carries `298.0` as the
executable's number, cited to its address, where the measured `296.88` was
deliberately never written down.

**What the capture could not have shown: the border and the fill are
separate, and the fill is drawn twice.** The block is a nine-patch border cut
from `Data\FE\Images\file2.gtf` (64x64, white, its alpha a three-texel
outline with one chamfered corner and one stepped one) over a flat fill whose
vertices sample a *swatch texel* of the same texture at alpha `110/255`. The
fill is drawn once at that alpha on both styles, then again at that alpha on
the Fury style or at an opaque texel on the HD style - so a Fury tab's inside
is `1 - 0.569^2 = 0.676` of its colour over the black page and an HD tab's is
solid. That is the `102/150 = 0.68` a Fury capture measures inside a `#969696`
border, and the `#646464`-to-the-byte, border-invisible tab the HD capture
measures - one mechanism, not two rules. The Fury style also chamfers the
block's top-left corner (10 units, visible in
`hd-menu-style-toggle/03-main-menu-after.png` rows 49-55); the HD style
leaves it square, which is why the 3840-wide capture's left edge is vertical
throughout.

**"It is rasterised geometry, not a texture mask" was half right.** The
fill's diagonal is a triangle `Block_DrawTopBand` rasterises; the border's
diagonal is `file2.gtf`'s own texels, drawn as 40x18 corner pieces at one
texel per unit. The paragraph above tested the texture-mask hypothesis
against `corner2.gtf`, the `<Bracket>` asset, which does not draw this widget.

**The underline's `6.6`-unit residual is closed.** A `.gtf`'s rows run
bottom-up and `oag_hud::sprite::Sheet` flips them (measured on the loading
screen's craft, see `sprite.rs`); the bar is at the file's row 1, so the
sheet's row 7, so `y + 35 + 7 = 167` for the capture's `167.1`. The
executable's `y + 35` needed no measured correction once the flip was counted.

**The underline blinks, on the code's reading - 8 ticks on, 9 off - and
slides in from the strip's right when a page arrives.** Five captures all
show it lit, a 2 % event at that duty cycle, so this is recorded at
confidence 70 with the tension stated rather than resolved; `just
rpcs3-record` would settle it. The *tab* still does not pulse: nothing in
the layout touches its colour or width on a timer.

**Drawn as of 2026-09-14**: `crates/ui/src/menu/block.rs` is `Block_Render`
reimplemented, `strip.rs` draws the executable's widths and eases the
selected one at a sixth of the remaining distance a tick (`Menu::tick_focus`),
and `rows.rs` draws HD's settings rows as `List_Item.cpp` does - a 520-wide
label block, a value block that grows from 280 to 340 with focus, and the
step arrows off `HD_options_arrow.gtf`. The measured `TAB_*` group in
`skin.rs` is now the fallback for a frame whose nine-patch did not decode.
`crates/game/tests/hd_menu_ground_truth.rs`'s
`the_menu_blocks_art_is_the_discs_and_the_swatch_is_the_texel` pins the
texture, the swatch and the sheet-row of the bar against the disc.

### `menu_font` is `None`, and that is a measurement

Pulse's rows say `font="menu"`, which its language plugins resolve to a face
two-thirds taller than the body one. Pure declares no such role. **HD is a third
case: the attribute is used and the slot does not exist.**

Every language plugin on the disc - 16 languages in `DATA02` and 16 in `DATA03`,
the only two archives carrying them - declares the same font slots:

```xml
<Font><Values name="Default"  Language="English" Src="Data\FE\Fonts\helv.fnt"></Values></Font>
<Font><Values name="Title"    Language="English" Src="Data\FE\Fonts\helvb.fnt"></Values></Font>
<Font><Values name="Buttons"  Language="English" Src="Data\FE\Fonts\PS_BUTTONS.fnt"></Values></Font>
<Font><Values name="HUD"      Language="English" Src="Data\FE\Fonts\PulseHud.fnt" borderExtendPixels="5"></Values></Font>
<Font><Values name="HUDSmall" Language="English" Src="Data\FE\Fonts\small.fnt" borderExtendPixels="3"></Values></Font>
```

`DATA03`'s copies add `wo3HUD` and `2097HUD` (the retro HUD skins). **All 32
`definition.xml` files were read**, not sampled, giving slot-name counts of
`Default` 32, `Title` 32, `Buttons` 32, `HUD` 32, `HUDSmall` 32, `wo3HUD` 16,
`2097HUD` 16 - and **no slot whose name matches `menu` case-insensitively, in
any of the 32**.

Four `<Menu>` widgets in `online_definition.xml` nevertheless say `font="menu"`,
and two `ingame_definition.xml` widgets say `font="InGame"`. Both name slots
that no plugin on this disc declares. Whether the engine falls back to `Default`
or has a compiled-in table is unknown. So `None` here means "HD's rows are
drawn in the default face" *as a finding*, and the doc comment on the HD crate
should say which of the two `None` meanings it is. Pure used to be the
precedent for a compiled-in colour table; it turned out its colours were
authored in a *style* skin the front-end root does not include (see
[race-setup.md](race-setup.md)), so before recording a name as undeclared,
check whether the title activates a second `PI_Skin`. HD declares none.

### The `transition_secs` warning, read this before pasting

The number is `0.5`, and **it is the same number Pulse carries**. That
coincidence is exactly the failure mode this project has been bitten by, so
here is the derivation in full so a reviewer can check it independently.

Pulse's 0.5 came off `<LeftLayer transition="0.5">` - the only layer element in
its 17 GUI files - and was then *confirmed by capture*: the outgoing page is
gone by frame 13-14 at 30 Hz, which is 0.43-0.47 s
(see [menus-original](../ui/menus-original.md#the-transition)).

HD has `<LeftLayer>` too, unlike Pure, which has none. `DATA06`'s counts:

| `<LeftLayer ...>` | count |
| --- | ---: |
| `transition="0.5"` | 17 |
| no `transition` attribute | 14 |
| `transition="0.7"` | 4 |
| `transition="0.0"` | 2 |

The other five copies give the same ordering (`0.5` 1-16, `0.7` 4, `0.0` 0-3).
`LeftLayer` appears in **4 of `DATA06`'s 29 frontend files** -
`additional_definition.xml`, `ingame_definition.xml`, `network_definition.xml`,
`online_definition.xml` - against Pulse's 17. Counting what the 17
`transition="0.5"` ones wrap: 7 `<Menu>`, 3 `<HorizMenu>`, 3 `<Item>`,
2 `<Text>`, 1 `<Image>`, 1 `<List>`. So it is menu-page content, which is the
case the field is about, but it is not exclusively a row list.

**Confidence 70, not 92.** It is authored, it is HD's own file, and it is the
same element Pulse's number came off - but the seconds interpretation was
established by a Pulse capture, no HD capture exists, and 0.5 is a plurality
(17 of 37 `LeftLayer` elements) rather than the single value Pulse has. Anyone
who wants this above 70 needs a capture.

**`MenuSkin::transition_secs` is not `Option<f32>`**, so there is no way to
express "unread" here. If the reviewer would rather HD said nothing than said
0.5, the type has to change. Flagged rather than decided.

### The `TitleColor` caveat

**It diverged (2026-10-08).** In the skin the front end loads, `HD_Grey` resolves
to `0xFF969696` (the draw lists read `150,150,150`), not `TitleColor`'s
`0xFF646464`, and RPCS3's `Main Menu`, `Grid Selection` and `Cell Selection` titles
all draw `150,150,150`. `oag_hd::frontend::MENU_SKIN.title` now carries
`0xFF969696`; `Language Selection`'s own title, which authors `TitleColor` itself, is
the screen that still reads 100. Omega's skin shares the front end and keeps
`0xFF646464`: checked, applies, not wired (no PS4 capture).

`TitleColor` is authored `0xFF646464` in all six copies, and `Language
Selection`'s own title text consumes it:

```xml
<Text name="LanguageText" StartEnabled="false">
    <Values idstring="Language Selection" font="Title" x="84"
            y="FEGlobals->TitleYOffset" scale="FEGlobals->TitleScale"
            color="FEGlobals->TitleColor"></Values>
</Text>
```

But `mainmenu_definition.xml`'s title does **not**:

```xml
<Text transition="2" delay="0.4">
  <Values idstring="FE_MM" font="Title" random="true" x="FEGlobals->TitleXOffset"
          y="FEGlobals->TitleYOffset" color="FEGlobals->HD_Grey"></Values>
</Text>
```

`HD_Grey` is declared `0xFF646464` - the same value - so the two agree
numerically and nothing here is wrong. It is recorded because the *mechanism*
differs: HD's menu screens reach for an `HD_*` palette (`HD_Blue`, `HD_Grey`,
`HD_LightGrey`, `HD_White`, `HD_BG`, `HD_transBG`) that Pulse has no counterpart
for, and if that palette ever diverges from `TitleColor`, `MenuSkin::title` will
be reading the wrong one. Note also that the picker's title hard-codes `x="84"`
rather than using `TitleXOffset`, so `title_x = 194` is the *menu* screens'
number, not the picker's.

### The title's own font role, wired 2026-09-13

Both widgets quoted above also say `font="Title"`, and until this date
nothing downstream read that - every menu text this build drew, chrome title
included, used the `Default` role (`helv.fnt`, line height 33; see
[fnt.md](fnt.md)'s font table). `oag_title::MenuSkin::title_font` is now
`Some("Title")` on `oag_hd::frontend::MENU_SKIN`, resolved through the same
`<Font><Values name=...>` plugin slots `menu_font` already used, and loaded
by `boot::fonts::load_title_font` (mirrors `load_menu_font`) into
`Shell::title_font`. The renderer binds it as a second glyph texture
alongside the body atlas - `oag_ui::frontend::Draw::FacedText`, resolved to
`MODE_FACE_ATLAS` in `ui.wgsl` - so the chrome title draws in `helvb.fnt`
(line height 44) in the same frame the rest of the page draws in `helv.fnt`.
Confirmed at `--menu-page main --screenshot --size 1920x1080`: "OPENANTIGRAV"
renders visibly bolder and taller than the strip tabs below it, where the two
previously matched.

**Pulse authors the identical role and was left unflipped.**
`MainMenu_Definition.xml`'s own title widget - `Data.wad`,
`Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, read 2026-09-13 - says
`<b d="FE_MM" p="title" ...>` (`p` is `font` in that file's own tag
dictionary), which resolves case-insensitively to the same `Title` slot,
landing on `Pulse_14.fnt` (17px) against `Default`'s 13px `pulse_text.fnt`;
`TournamentLoad`'s own title says `font="Title"` outright. Left `None` on
`oag_pulse::frontend::MENU_SKIN` anyway: flipping it would move output
[menus-original.md](../ui/menus-original.md)'s Layout table already verified
against a capture at confidence 95, and no capture has checked whether the
taller face is what that capture actually shows - see
`oag_title::MenuSkin::title_font`'s own doc.

**Pure's `Main Menu` screen was located and checked, 2026-09-25, and wired.**
`Data.wad`'s `Data\Plugins\PI001\GUI\Skin.xml` (both `pure-psp-eu.chd` and
`pure-psp-usa.chd`) authors `idstring="Main Menu" font="Title"
x="FEGlobals->TitleXOffset" y="FEGlobals->TitleYOffset"
scale="FEGlobals->TitleScale" color="FEGlobals->TitleColor"` on the same root
screen `Race Campaign`/`Racebox`/`Remix`/`Records`/`Options`/`Quit` draw
under. Unlike Pulse, this carried none of the same risk: Pure's `Title` role
resolves to `FX300ANG.fnt`, the identical file `Default` already does (see
`oag_ui::language::roles`'s own font table), so `oag_pure::frontend::MENU_SKIN::title_font`
is now `Some("Title")` and a `--menu-page main --screenshot` render is
pixel-identical before and after - the flip is correctness (the title now
draws through its own authored role) rather than a visible change, so it
needed no fresh capture to justify.

## Filling in `BootProfile`

```rust
pub const BOOT_PROFILE: &oag_title::BootProfile = &oag_title::BootProfile {
    // **The chain HD's own `skin.xml` DECLARES, not one that was measured.**
    // No emulator has been run against this disc. `oag_title::boot`'s own docs
    // record that a declared entry point is not the runtime's - Pulse's two
    // differ - so this is a starting hypothesis, confidence 80, and it must not
    // be described as HD's boot order until someone watches it.
    chain: &[
        oag_title::BootStep::screen(states::LANGUAGE_SELECTION),
        oag_title::BootStep::screen(states::PRE_FMV_CONNECT),
        oag_title::BootStep::playing(states::STUDIO_LOGO, names::STUDIO_LOGO_MOVIE),
        oag_title::BootStep::screen(states::EPILEPSY_WARNING),
        oag_title::BootStep::screen(states::FIRST_PLAY),
        oag_title::BootStep::screen(states::SAVE_WARNING),
        oag_title::BootStep::screen(states::EULA),
        oag_title::BootStep::screen(states::UPDATE_ANNOUNCEMENT),
    ],
    // `Studio Logo` is the one screen on the declared chain that plays a movie,
    // and the movie is named for the Studio Liverpool logo. On the boot path,
    // so `--reel` shows a screen the disc shows rather than being refused.
    // **Confidence 60, and this is the one field filled by analogy.** The
    // screen and the movie path are read straight off the XML; the claim that
    // this is HD's counterpart to Pure's dev/pub reel is *not* read. Pure's was
    // established by decoding the movie and matching its cards against captured
    // frames. No Bink decoder was run here, so nothing on this page knows what
    // is inside `StudioLiverpool.bik` beyond its name.
    reel: Some(oag_title::BootStep::playing(
        states::STUDIO_LOGO,
        names::STUDIO_LOGO_MOVIE,
    )),
    // **A measurement, not a gap.** HD ships no looping movie backdrop at all;
    // its menu background is a real-time `.vex` scene. See below.
    menu_backdrop: None,
    // **A measurement, not a gap.** The picker declares no fill, and its parent
    // is `skin.xml`'s unnamed root `<Screen>`, which declares none either.
    picker_backdrop_parent: None,
    // **A measurement, not a gap.** Every `FEGlobals->` key referenced anywhere
    // in the frontend tree is declared by that archive's own `skin.xml`, in all
    // six copies. Nothing is missing to fall back for.
    fallback_globals: &[],
};
```

With the screen and movie literals:

```rust
pub mod states {
    pub const LANGUAGE_SELECTION: &str = "Language Selection";
    pub const PRE_FMV_CONNECT: &str = "PreFMVConnect";
    pub const STUDIO_LOGO: &str = "Studio Logo";
    pub const EPILEPSY_WARNING: &str = "EpilepsyWarning";
    pub const FIRST_PLAY: &str = "FirstPlay";
    pub const SAVE_WARNING: &str = "Save Warning";
    pub const EULA: &str = "EULA";
    pub const UPDATE_ANNOUNCEMENT: &str = "Update Announcement";
    pub const MAIN_MENU: &str = "Main Menu";
    pub const TOP_FE_SCREEN: &str = "Top FE Screen";
}

pub mod names {
    /// `DATA06`'s spelling. `DATA00` names `StudioLiverpool_fury.bik` instead,
    /// and that is the only file of the two present in `DATA00`.
    pub const STUDIO_LOGO_MOVIE: &str = "Data/FE/Images/StudioLiverpool.bik";
    pub const STUDIO_LOGO_MOVIE_FURY: &str = "Data/FE/Images/StudioLiverpool_fury.bik";
    pub const FRONT_END_SCENE: &str = r"Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex";
}
```

**Forward slashes in the movie path are the disc's**, not a transcription
slip - `src="Data/FE/Images/StudioLiverpool.bik"` - while every other asset
reference in the same file uses backslashes. Recorded because a normaliser that
"fixes" it would be changing the data.

### `menu_backdrop: None` is a finding, not a blank

Pulse ships `Data\Movies\Backdrop.PMF` and loops it behind the menus. HD does
not, and does not ship anything equivalent. Its `Top FE Screen` instead carries:

```xml
<BackgroundAnim name="bgAnim" DisableTransition="0">
    <Values OriginX="960" OriginY="540" nearZ="1.0" obeySafeZone="false"
            Src="Data\FE\FrontEndScene\FrontEndScene_HD_ATG.vex" UseModelCamera="true"
            x="4.0" y="1.0" z="-11.0" RotX="0.4" RotY="-0.5"></Values>
    <ScreenSetting name="default" blur="3" use_bands="false" min_band_height="0.25" ...>
        <Main edge_level="0.3" fill_level="0.1" edge_width="1.5"></Main>
        <Band1 edge_level="0.2" fill_level="0.1" edge_width="3.0"></Band1>
        <Band2 edge_level="0.2" fill_level="0.1" edge_width="3.0"></Band2>
    </ScreenSetting>
    <ScreenSetting name="Main Menu" use_bands="false" blur="0"> ... </ScreenSetting>
    <ScreenSetting name="SoundTest" use_bands="false" blur="6"> ... </ScreenSetting>
    ...
```

`FrontEndScene_HD_ATG.vex` (118,432 bytes) and its `.rcsmodel` (403,060 bytes)
both exist in `DATA02`. So HD's menu backdrop is **a real-time `.vex` scene with
a camera pose and per-screen blur**, not a video - and `.vex` is the format
[hd-status](hd-status.md) already reads byte-swapped, with the caveat that its
mesh batches have moved to `.rcsmodel`. **Confidence 88**: the widget, the
attributes and both files are all present and consistent; nothing shows the
engine drawing it. A live capture now confirms *something* real-time and
animated draws there on the Fury style and nothing does on the HD style - see
"The FE Style switch is live, works, and is not an archive swap" below, which
also refines what that something more likely is.

The only `.bik` files on the disc that are not per-track previews or UI icons
are the two Studio Liverpool logos. There is nothing a `menu_backdrop` could
point at even if the type wanted one.

**2026-09-14: the "something real-time" on the Fury style is read.** The same
`Top FE Screen` carries a second widget, `<BackgroundAnimFury name="bgAnimFury"
startenabled="false">`, with thirteen `<ScreenSetting name=".." tint="0xAARRGGBB"
equaliser="0"/>` rows - `Main Menu`, `Additional`, `Controls Menu` and `Extras`
white, the rest `0xFF202020`. The Fury style enables it, and it draws one of
nineteen `Data/FE/Fury/*.points2` hull point clouds along the camera paths in
`Data/fe/fury.envsettings`, as sprites through `RadioHead` GPU modes with a
feedback trail, tinted by that row. Read in full on
[menu-backdrop.md](../ghidra/functions/ps3-hdfury-eu/menu-backdrop.md);
`oag_ui::backdrop` and `oag_game::render::backdrop` draw it, and
`oag_hd::frontend::names::FURY_CLOUDS`/`FURY_SETTINGS` name the files. The HD
style's `<BackgroundAnim>` above stays undrawn on HD; it is read in
[menu-backdrop-scene.md](../ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md) and drawn for Omega.

### `picker_backdrop_parent: None`, with a caveat

Pure's picker is a child of `Intro Screen` and inherits its white fill. HD's
picker is a direct child of `skin.xml`'s **unnamed root `<Screen>`**, and:

- `Language Selection` declares no `<ScreenClear>` of its own.
- The root `<Screen>` declares no `<ScreenClear>` either. Everything before the
  first named child screen is the `<Variable>` block, the nested
  `<Screen name="HD_Colours">` variable block (`DATA00`/`DATA06` only), and one
  `<Image name="networkbusy">`.

So `None` is correct for the field as defined - there is no parent screen whose
fill the picker inherits. **But that also means what the picker draws on is not
authored anywhere.** The neighbouring boot screens all clear to
`FEGlobals->HD_BG` (`0xffffffff`, white) explicitly, so white is the plausible
guess; it is *not* stated, and this page does not assert it. An emulator settles
it in one frame.

## Where HD and Pulse differ

The comparison is the interesting result, so it gets its own table. Pulse's
column is from [menus-original](../ui/menus-original.md); Pure's is in
[pure-status](pure-status.md).

| | Pulse | HD / Fury |
| --- | --- | --- |
| skin path | `Data\Plugins\PI001\GUI\Skin.xml` | `/data/plugins/frontend/gui/skin.xml` |
| copies on disc | 1 | **6**, one per archive but `DATA01` |
| dialect | `<code>`-shortened | plain `<?xml`, `<?...?>` used as comments |
| coordinate space | 480x272 | **1920x1080** |
| `MenuXOffset` / `MenuScale` | 50 / 1.0 | **800** / 1.0 |
| `MenuYOffset` | **not declared** | **300** |
| `TitleXOffset` / `TitleYOffset` / `TitleScale` | 50 / 0 / 1.0 | **194 / 62** / 1.0 |
| `TextColor` / `TitleColor` | `0xFF33A6B9` / `0xFF000000` | **`0xFFFFFFFF` / `0xFF646464`** |
| globals declared | dozens | 62 or 64 |
| `HD_*` palette | none | `HD_Blue`/`Grey`/`LightGrey`/`White`/`BG`/`transBG` |
| main menu widget | `<Menu>`, vertical, `y="32"` | **`<HorizMenu>`, horizontal, `x="160" y="125"`** |
| `Menu` font slot | present, resolves to a taller face | **absent from every language plugin** |
| font slots per language | 8 | **5** (7 in `DATA03`) |
| layer element | `LeftLayer`, 17 files | `LeftLayer`, 4 of `DATA06`'s 29 |
| `transition` values | 0, 0.2, 0.5, 0.7 | 0, 0.1, 0.2, 0.25, 0.3, 0.4, 0.5, 0.55, 0.7, 1, 2 |
| boot movie | `Data\Movies\IntroMovieP1_EU.PMF` | `Data/FE/Images/StudioLiverpool.bik` (Bink) |
| menu backdrop | `Data\Movies\Backdrop.PMF`, looping | **a real-time `.vex` scene** |
| boot chain length | 3 screens (**measured**) | **8-9 screens (declared only)** |
| network screens on boot | none | `PreFMVConnect`, `EULA`, `Update Announcement` |
| undeclared `FEGlobals` refs | none | none |

Two of these carry weight beyond the numbers:

**The front end grew a network leg.** Three of HD's nine declared boot screens
exist only because the game is online: `PreFMVConnect` runs a `<UnityConBasic>`
connection check before the logo, and `EULA` and `Update Announcement` both
carry `<SVOData>` elements. Pulse's boot has no equivalent at all. A
reimplementation that has no network will step over all three, which is exactly
what `BootProfile::next_after`'s `usable` predicate is for.

**HD's `<LoadXML>` includes carry a stale path beside a working one.** 21 of
`Skin.xml`'s 23 `<LoadXML>` elements author both `Src` and `SrcRel` on the same
`<Values>`, e.g.

```xml
<LoadXML><Values Src="Data\Plugins\PI001\GUI\MainMenu_Definition.xml" SrcRel="MainMenu_Definition.xml"></Values></LoadXML>
```

`Src` still spells the numbered-plugin path from the shared Pulse-lineage
authoring (`PI001`), which is not a real directory on this disc; `SrcRel` is a
bare filename, joined against the directory `Skin.xml` itself lives in, and
lands exactly on the real file - `/data/plugins/frontend/gui/mainmenu_definition.xml`,
11,035 bytes, the same path `oag_hd::frontend::names::MAIN_MENU` reads
directly. Nothing needs `Src` to resolve, because `SrcRel` already does; the
remaining 2 of 23 (`Team_Selection_Definition.xml`, `Track_Selection_Definition.xml`)
carry `SrcRel` alone, no `Src` at all. Confidence **92** - read directly off
`Data\Plugins\Frontend\Gui\Skin.xml` in `DATA00.PSARC`, cross-checked against
the independently-recovered `MainMenu_Definition.xml` path and byte count.
Which attribute the original engine prefers when both are present is
unconfirmed; a reader only needs to try `SrcRel` first, never `Src` first,
which is the one reading with both authored *and* resolving in the same file.

**HD is Pulse's front end scaled up, not a new one.** The element vocabulary
(`Screen`, `Variable global=`, `Redirect`/`Default goto=`, `Menu`, `LeftLayer`,
`FEGlobals->` references, `LoadXML`) is the same, the shortened-dialect
difference aside. Which is the same headline
[hd-status](hd-status.md) reached for the asset layer: not a new engine.

## What the executable says

Read out of `/hdfury/EBOOT-ps3-hdfury-eu.elf` in the project's Ghidra database on
2026-08-17, **by reading only**. Nothing was renamed, so there is no
`names.tsv` row and no `docs/ghidra/functions/` page for any of this; addresses
are cited instead. Anyone promoting one of these to a name owes the page and the
row in the same change, per [ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

**The database's `r2` fix is applied**, checked before anything below was
trusted: `0x00392478` decompiles with its TOC pointers resolving to the
allocator globals and its callees coming out as the functions
[`memory.md`](../ghidra/functions/ps3-hdfury-eu/memory.md) records, and `_opd_`
thunks are present throughout. That check matters because the trap is live in
this very sweep - see the warning at the end of this section.

### The boot entry point is chosen by a function, and its default is the picker

At `0x000186f0`, in its entirety:

```c
undefined * FUN_000186f0(void)
{
  if (*(char *)(DAT_008a57a0 + 1) != '\0') {
    return PTR_s_Launch_Game_008a57a4;      // "Launch Game"
  }
  return PTR_s_Language_Selection_008a57a8; // "Language Selection"
}
```

Both are real HD screen names: `Language Selection` is `skin.xml`'s picker and
`Launch Game` is `ingame_definition.xml`'s first screen. They sit adjacent in
the application string pool at `0x007792f0` and `0x00779300`, immediately before
`QuitGameMutex` and `ShutdownMutex` - and `DAT_008a57a0` is the object that owns
both of those mutexes (`0x00018728` installs them at `+8` and `+0x20`). So the
global is the game-root object and this is one of its accessors; the function is
reached only through a vtable slot at `0x008709d0`, alongside five siblings at
`0x000186d8`-`0x00018870` that read the same object.

**Confidence 88 that the function returns one of those two screen names on that
flag** - it is four instructions, unambiguous, and below `0x32d5e0` where the
TOC needs no correction. **Confidence 70 that this is *the* boot entry point.**
The call site is virtual and was not resolved, so "a game-root accessor that
yields a start-screen name" is read and "the boot begins here" is inference.

What it does support, at 70: **HD's runtime default agrees with its XML.** The
chain opens on `Language Selection` and the only alternative the executable
offers is `Launch Game`, which is the straight-into-a-race path a demo or trial
boot would take, not a different front-end order.

**This paragraph used to end "it is *not* the cold-boot confirmation Pure has,
and this page still does not assert a runtime order."** It has that confirmation
now - three cold boots on RPCS3, 2026-09-05, every one opening on `Language Selection`
- so the static read and the watched boot concur. **Neither number moves.** 88
and 70 score what can be read out of four instructions and an unresolved virtual
call site, and watching a boot does not make the call site resolved; it is a
second, independent line of evidence, not a promotion of the first.

The trial reading of that branch is corroborated all over the binary and the
data: `UpgradeToFullVersionCheckout_Screen.cpp` is one of the 47 classes;
`mainmenu_definition.xml` declares `PurchaseGame`, `ProductInfoConnectingScreen`
and `ProductInfoErrorScreen` and carries an `<Item name="TrialItems">` block;
`demo_definition.xml` declares `Demo Launch Real` and `Demo InGame`. A flag that
skips the whole front end and lands in a race is exactly what a timed demo
needs.

### An ordered array of the seven archives, purpose unread

The seven archive names are string literals at `0x00777f88`-`0x00777fe8`, 16
bytes apart, and the **only** place they are referenced is a NULL-terminated
array of seven pointers at `0x00860c80`:

```
0x00860c80: 00777f88 00777f98 00777fa8 00777fb8   -> data00 data01 data02 data03
0x00860c90: 00777fc8 00777fd8 00777fe8 00000000   -> data04 data05 data06 NULL
```

**Confidence 88, and it is scored for exactly one claim**: an ordered,
NULL-terminated array of pointers to all seven names, in `DATA00`..`DATA06`
order, is the sole reference site for those strings. That is data rather than
code, so the TOC trap cannot touch it.

**What the array is *for* is unread, and is deliberately not scored.** Calling
it a mount list would be the invention this project has already been bitten by
twice. A validation list, a prefetch manifest and an existence check all have
this shape, and `"Prefetched %d files\n"` sits a few hundred bytes away as a
live alternative reading. The consuming loop was not identified, and the one
lead Ghidra's own xref list offered turned out not to be it: `0x0032e5d8` was
re-decompiled 2026-08-30, after the PS3 TOC fix, and `.opd.FUN_0032e430` is a
`CellSaveDataStatGet` result dumper (`dirName`, `isNewData`, `hddFreeSizeKB`,
`PARAM_SFO_TITLE`, and so on) with no relation to the archive array at all.
`0x0032e5d8` is `lwz r3,-0x7f60(r2)`, loading the format string for that
function's last `printf` (`PARAM_SFO_LIST_PARAM`), which the corrected
decompile now names directly. Why Ghidra's xref index pointed here - it is
tagged `[PARAM]`, and that tag was not explained either - is itself
unresolved. The TOC-relative loads that would reach the array still leave no
instruction operand to search for, so this is a dead end confirmed rather
than a new one opened.

**So the archive-layering question stayed exactly where it was** before the
executable was opened - and until the consumer is found, the *order* in the
array carries no priority meaning either. A first-wins array and a last-wins
array are byte-identical.

**The runtime answered it anyway, 2026-09-05, without the consumer being
found.** HD prints `PSARC file found data00..data06` and then `PSARC file
Loading` in the order `data01, data02, ... data06, data00`, so the loading order
is **not** this array's order and `DATA00` goes last. That is a stronger result
than the array could have given: whatever this array is for, it is not the thing
that decides load order, and the discovery-order-versus-load-order split is
visible in the game's own log. See ["`DATA00`'s copy is the live
one"](#data00s-copy-is-the-live-one-confidence-92-and-load-order-is-now-known).

One weaker corroboration did fall out. A second site names **only
`data00.psarc` and `data06.psarc`** - `0x007a4bf8` and `0x007a4c20` - among a
run of file-system literals:

```
data00.psarc / "PSARC file Loading %s\n" / data06.psarc /
"/dev_bdvd/PS3_GAME/USRDIR" / "Prefetched %d files\n" /
"Creating file systems" / "/dev_hdd0"
```

Those two, and no other five, singled out among file-system strings is the same
pairing the `skin.xml` families showed - `DATA00` and `DATA06` being the two
copies that drop `LogoFMV` and carry the `HD_Colours` block. **Confidence 55**:
two independent sources agreeing that *some* distinction separates those two
archives from the other five is real, but the code around these literals was not
read either, string adjacency is not a data structure, and nothing here says
what the distinction is.

### The skin path is built, not hardcoded

`0x0078f698` holds `"%s\Skin.xml"` and `0x00786a40` holds
`"Data\Plugins\frontend"`, which is exactly what `definition.xml` authors:

```xml
<PI_Skin name="UI">
    <Values location="Data\Plugins\frontend\GUI" activate="true"></Values>
</PI_Skin>
```

So the engine composes the skin path from the plugin's own `location`, and the
composed path is **identical for all six archives**. The executable therefore
cannot disambiguate them either; only the mount polarity can. Worth noting that
the format string is `Skin.xml` with a capital S and backslashes while the
archives store `/data/plugins/frontend/gui/skin.xml` in lower case with forward
slashes, so the lookup normalises case and separators somewhere.

`FrontendRoot.cpp` (`0x00786950`) and `FrontendGlobals.cpp` (`0x00785b58`) are
the module names, if someone picks this up.

### 47 screen classes, and the `type=` correspondence

The binary is stripped of symbols but keeps 462 `.cpp` filename strings (see
[`memory.md`](../ghidra/functions/ps3-hdfury-eu/memory.md)), of which **47 are
`*_Screen.cpp`**. Each sits beside its own type-name string - `EpilepsyWarning`
at `0x00794e60` beside `EpilepsyWarning_Screen.cpp` at `0x00794e18`, `FEMain` at
`0x00794f00` beside `FEMain_Screen.cpp` at `0x00794ec0`, `HDDBoot` at
`0x00795d60` and `Language Selection` at `0x00795db0` beside
`LanguageSelection_Screen.cpp` at `0x00795d68`.

HD's `skin.xml` uses **eight** distinct `type=` values, and **seven of the eight
have a class**: `Language Selection`, `EpilepsyWarning`, `FirstPlay`, `HDDBoot`,
`FMV`, `FEMain`, `Main Menu`.

**The eighth is a counterexample and the rule does not survive it.**
`<Screen type="RunTeaser" name="Game Share">` names a type with no
`RunTeaser_Screen.cpp` and no `Teaser_Screen.cpp` among the 47. So
**confidence 80 that `type=` selects a compiled screen class for the boot
screens, and it is not a universal rule** - `RunTeaser` is either handled
generically, handled by a class whose file is named something else, or dead the
way `LogoFMV` is. Not read either way.

A second oddity on the same screen, worth one line: `Game Share`'s only
`Redirect` is `Default goto="extras"`, lower case, while the screen it must mean
is `additional_definition.xml`'s `Extras`. Together with `%s\Skin.xml` resolving
to a lower-case `skin.xml` in the archive, that is a second hint that the
resolver folds case - **confidence 50**, two coincidences and no code read.

**Do not read the converse.** `PreFMVConnect`, `EULA` and `Update Announcement`
declare no `type=` and have no class, and that is *expected* under the same
rule, not evidence they are dead - `Default_Screen.cpp` and `FEDefault_Screen.cpp`
are exactly the generic drivers a typeless screen would use. The `LogoFMV`
finding above rests on different evidence entirely: a missing asset, an
undefined target and no inbound `goto`.

Two of the 47 are worth naming: **`DeveloperPublisher_Screen.cpp`
(`0x00793c00`) and `MemoryStickWarning_Screen.cpp` (`0x00796110`) are compiled
into a PS3 binary.** Pure's boot screens, still linked in on a machine with no
Memory Stick. The PSP legacy is in the code as well as in the data.

### Executable corroboration for the dead screen

`LogoFMV`, `Show Logo`, `StudioLiverpool` and `Backdrop` are **not strings in
the executable at all**. The first two being absent is a third independent
reason to treat `LogoFMV` as dead, on top of the four in
[the dead PSP screen](#the-dead-psp-screen). The second two being absent says
something different and useful: the logo movie is named only by the XML, so
**which `.bik` plays is decided by which `skin.xml` is live**, not by the code -
which folds that question back into the unresolved mount polarity.

### The TOC trap, caught in the act

`memory.md`'s first trap showed itself during this sweep and is worth the
warning. `0x003383b0` - **above** `0x32d5e0` - decompiles as a screen-name
lookup that returns `"Language Selection"` for two input values and otherwise
calls a formatter whose format string Ghidra resolves to
`"<Bronze Target %d> <Bronze>"`. A medals string inside a screen-name function
is not a surprising find, it is a wrong one: that is the wrong-TOC symptom
exactly as documented, in a function twenty minutes' reading away from a real
result. Nothing above `0x32d5e0` was used on this page without the check
described at the top of this section.

## The end screens are located, not read

**Added 2026-09-18**, off the back of a log line rather than a reading pass:
a finished HD race warned `Data\Plugins\PI001\GUI\EndRace_Definition.xml`
missing on every frame it sat on the results table. The entry name is
**Pulse's**, and HD authors its own end screens under its own named-plugin
convention - the same divergence [`FrontEnd::root`] already carries for
`Skin.xml`.

They are `/data/plugins/frontend/gui/endrace_definition.xml`, in five of the
seven archives, no two copies alike:

| Archive | Size | Screens it declares |
| --- | --- | --- |
| `DATA02` | 30,110 | Results, **Rewards**, Menu |
| `DATA03` | 30,623 | Results, **Rewards**, Menu |
| `DATA04` | 31,053 | Results, **Rewards**, Menu |
| `DATA05` | 42,406 | Results, **Rewards**, Menu, **Podium** |
| `DATA06` | 41,190 | Results, Menu, **Podium** - no Rewards |

All five differ by MD5; every copy also carries `Kill Game Transition`,
`InGame Restart Transition`, `EndRaceSaveGhost` and `EndRaceDeleteGhost`.
`DATA00` and the sound-only `DATA01` carry none. **Confidence 94** for the
inventory - it is a file list and five reads of authored XML - and **nothing
here says which copy the original loads**, which is the same unresolved
question [six skins, one layout](#six-skins-one-layout) records for `skin.xml`.
`oag_assets::Archives` would serve **`DATA02`'s**: `holder_of` walks
`ArchiveCandidates`' `data` then `fe` then `extra`, which for HD is `DATA00`
(no copy) and then `DATA02` (`crates/hd/src/lib.rs`'s `DATA_CANDIDATES`/
`FE_CANDIDATES`). That is a reading of this build's own mount order, not of
the original's.

### Why the path is not simply swapped in

Three of the screen *names* match Pulse's (`EndRace Results`/`EndRace
Rewards`/`EndRace Menu`), so `oag_game::endrace::load` would find all three in
`DATA02`'s copy and then draw almost nothing: the widgets underneath are a
different vocabulary, and on `EndRace Results` a different screen altogether.

| | Pulse (`EndRace_Definition.xml`) | HD (`endrace_definition.xml`) |
| --- | --- | --- |
| Results' table | `lap{n}.{c}` - the player's own per-lap splits | `Grid{row}.{col}`, 8 rows x 10 columns plus `Grid{n}.h`, `Gridp.{n}`, `Gridi.{n}`, `GridHead{n}` - the **finishing order of the whole field** |
| Results' extras | `tablebg{n}`, `tablehighlight`, `perfectlap{n}`, `boostimg` | `GridHighlight`, `GridSideBarL`/`R`, `GridTopBar`/`BottomBar`, `GridStrikeThrough`, `MedalBlock` with `MedalModelGold`/`Silver`/`Bronze`, `RecordNotifyBlock`, `Target{n}` |
| Headline | `Line1` | `Line1` - one of the two names that transfer at all, `loyaltybar` being the other |
| Rewards | medal glyph, loyalty text, `loyaltybar` as an `<Image>` | `MedalImg`, `LoyaltyImg`, `BigPos`, `RewardLine1`/`2`, `RewardLoyaltyPoints`/`Active`, and `loyaltybar` as a **`<Slider>`** (`minSlide`/`maxSlide` 0-100000) |
| Menu | one list populated per mode by `EndRaceMenu_PopulateOptions` | one `<Block>` per option, shown by mode: `next_race`, `race_again`, `return_to_grid`, `return_to_menu`, `quit_tournament`, `return_to_lobby`, `view_again`, `view_MP_again`, plus an `Endrace Difficulty` `<List>` |

So pointing the constant at HD trades a named missing entry - an honest
absence - for three screens that read and draw blank, which is the
["plausible-looking stand-in"](../../CLAUDE.md) failure one step removed.
**Confidence 90** on the vocabulary table: it is quoted from `DATA02`'s and
`DATA06`'s copies directly, and the caveat is that four of the five copies were
only inventoried, not read widget by widget.

What the reading pass would need, and what makes it cheap now: HD's Blocks
carry `idstring`s this build's own [`MenuOption`] set already mirrors
(`ER_RACE_AGAIN`, `ER_RETURN_GRID`, `ER_RETURN_MENU`, `ER_VIEW_AGAIN`,
`ER_NEXT_RACE`), and `just rpcs3-race` can drive a real HD race to its own end
screens for the frames to compare against - see
[rpcs3-debugger](../reverse-engineering/rpcs3-debugger.md).

### What this did change

`crates/game/src/main/session/endrace.rs` no longer retries the load once a
race has failed it: `RaceStage::endrace_unavailable` records the failure, so
the warning is once per race and the disc image is reopened once per race
rather than once per frame. The message says "this title's EndRace screens are
not read by this build" rather than "this source has no EndRace screens",
which this section is the evidence for.

### 2026-09-21: `EndRace Results`/`EndRace Menu` are read and drawn

The reading pass this section's own "Next Steps" called for happened -
[`hd-endrace-screens.md`](hd-endrace-screens.md) is the widget-by-widget
read (`DATA02`'s copy, plus every diff against `DATA03`-`DATA06`), and
`oag_title::FrontEnd::endrace_entry` is now the per-title axis this section's
own point 3 named as "worth nothing until a second title's screens actually
draw" - HD's own value goes through the same `holder_of` precedence this
page documents for `skin.xml`. `EndRace Rewards` (present on `DATA02`-`05`)
and `EndRace Podium` (`DATA05`/`DATA06` only) are inventoried on that page
but not drawn this pass - see its own scope notes. `docs/ui/endrace-screens.md`
carries the picture half (what draws and why) and this project's live
verification against a real disc-driven HD race.

[`FrontEnd::root`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/title/src/lib.rs
[`MenuOption`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/ui-screens/src/endrace.rs

## What could not be determined

Named explicitly, with what each would take.

**Still needs the PS3 executable, after the sweep above:**

1. **What consumes the seven-archive array at `0x00860c80`, and with what
   polarity.** Until the consumer is found it is not even established that the
   array is a mount list, and a first-wins and a last-wins list look identical.
   **It is no longer the blocker on "which `skin.xml` is live"** - it was called
   the single one here, and the runtime answered the question around it on
   2026-09-05: `DATA00`'s copy, on the load order HD prints and on a filename
   only that copy names. What is left is the array's *own* purpose, which is now
   a smaller question than the one it was standing in front of. It does not
   affect `MenuSkin`; all six copies agree there.
2. **The call site of `0x000186f0`**, to lift the boot entry point from
   inference (70) to a reading.
3. **Whether an unresolved `font=` falls back to `Default` or to a compiled-in
   table.** `font="menu"` and `font="InGame"` name slots no plugin declares.
   Cannot change `menu_font`, which is `None` either way.
4. **The easing curve on a page change**, the same gap Pulse has.

**Needs the title *running*, which it now can be: `just rpcs3-race` walks HD's
front end from a cold boot into a race with no window on anyone's desktop, and
HD names every screen it enters on `TTY.log`
([rpcs3-debugger.md](../reverse-engineering/rpcs3-debugger.md)). Every item
below is therefore a reading waiting to be taken rather than a blocked one -
none has been taken yet:**

5. ~~**The runtime boot order.**~~ **Done, 2026-09-05**, three cold boots, all
   eight steps in the declared order - `oag_hd::frontend::BOOT` is
   `Provenance::Measured`. The risk this item named was real and did not
   materialise: HD's runtime agrees with its XML where Pulse's does not.
6. **Whether `Language Selection` is ever *shown*** on a machine that takes its
   language from the XMB. **Still open, and now sharper.** The capture proves the
   screen is *entered* on every boot, so `0x000186f0` returning its name is
   corroborated - but no frame of it was ever caught between the SCE licence
   screen and the Studio Liverpool reel, so whether a player sees it is
   unanswered. `LanguageAutoRedirect` is a sufficient explanation for a boot that
   passes straight through, and this is the one question a capture at a finer
   sampling rate than 1.5 s could still settle.
7. ~~**Step 4's real path.**~~ **Done**: the dialog's,
   `EpilepsyWarning` -> `FirstPlay` -> `Save Warning`, on three boots - and
   `FirstPlay` only appears at all when the savedata is absent, which is its own
   finding.
8. **`MenuSkin::selected`.** No XML on this disc states a selected-row colour,
   exactly as on Pulse and Pure. It is a pixel measurement or nothing.
9. **`MenuSkin::row_extra_leading`.** The `gap` attribute is documented as *not*
   being row pitch on Pulse; HD's `gap` values are 10, 15, 25 and 80, which
   spread too wide to be a leading anyway.
10. **Whether `transition` is seconds on PS3.** Assumed from Pulse's capture,
    where 0.5 measured as 0.43-0.47 s at 30 Hz. HD presents at a different rate
    and the assumption is untested.
11. **What the language picker draws on.** Neither it nor its parent declares a
    fill.
12. **Whether the 1920x1080 space is presented 1:1** or safe-zone-inset.
    Measured 2026-10-08 on the maintainer's RPCS3 save: **not 1:1** - scale
    0.904, offset about (+150, -8), see `docs/ui/campaign-screens.md`'s "The
    layout scale". Open: whether that is `Safe Area Setting` at a non-default
    value (it is a user option) or the default.

**Not attempted here:**

13. **`ingame_definition.xml`** (84-122 KiB per copy, 60+ screens) - the in-race
    HUD and pause tree. Out of scope for a front end.
14. **`entries.xml`** - 16 languages x ~125 KiB of string tables. The
    `idstring` values quoted above (`FE_MM`, `FE_RC`, `BOOT_HEALTH_WARN_1`)
    were not resolved to text.
15. **`.gtf` front-end textures.** 7,333 on the disc; none decoded for this.

## Implemented where

`crates/hd/src/frontend.rs`, which this page is the citation for - the way
[pure-boot](../architecture/pure-boot.md) backs `oag_pure::frontend`.
[ADR-0022] governs the crate's shape and [ADR-0023] its boot table, whose
provenance is [ADR-0025]'s.

**This paragraph used to read "nowhere yet"**, and it was written before the
crate existed. What consumes the page today:

| This page's section | Where it landed |
| --- | --- |
| `MenuSkin`, every field | `oag_hd::frontend::MENU_SKIN`, pinned by `crates/game/tests/menu_skin.rs` |
| the `<HorizMenu>` | `oag_title::MenuStrip` and `oag_game::menu::strip`, pinned by `crates/game/tests/hd_menu_ground_truth.rs` |
| `FE Screen`'s frame | `oag_title::FrontEnd::menu_frame` names the screen; `oag_game::menu::frame` reads it, same test |
| `BootProfile`, declared | `oag_hd::frontend::BOOT`, pinned by `crates/game/tests/hd_boot_ground_truth.rs` |
| the 1920x1080 grid | `oag_display::space::Space::HD` |
| the sixteen language plugins | `oag_hd::frontend::LANGUAGE_PLUGINS` |

[ADR-0025]: ../architecture/adr/0025-a-boot-chain-carries-its-provenance.md

[ADR-0022]: ../architecture/adr/0022-title-packages.md
[ADR-0023]: ../architecture/adr/0023-boot-sequence-as-title-data.md
