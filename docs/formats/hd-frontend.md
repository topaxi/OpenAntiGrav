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

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Where is the skin? | `/data/plugins/frontend/gui/skin.xml`, in **six** of the seven archives | 94 |
| Do the six copies agree on layout? | **Yes, on every layout global, exactly** | 92 |
| Which copy does the runtime load? | **Still unknown.** An ordered array of all seven is at `0x00860c80`; what consumes it is unread | 88 / - |
| Coordinate space | **1920x1080**, not Pulse's 480x272 | 90 |
| Is the main menu a row list? | **No.** It is a `<HorizMenu>` - horizontal | 90 |
| Is there a `menu` font role? | **No.** HD's language plugins declare no such slot | 90 |
| Is there a looping movie backdrop? | **No.** The FE background is a real-time `.vex` scene | 88 |
| Are any `FEGlobals` referenced but undeclared? | **None, in any of the six** | 90 |
| Declared boot chain | Nine screens, quoted below | 80 |
| Runtime boot order | **Unverified.** Needs an emulator | - |
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

## The declared boot chain

Read redirect for redirect out of `DATA00`/`DATA05`/`DATA06`'s `skin.xml`, which
agree with each other:

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

**Confidence 80 for the chain as *declared*. Confidence 0 for it as the runtime
order, because that was not measured.** [ADR-0023] exists precisely because a
front-end XML's declared entry point is not the runtime's: Pulse's XML declares
the language picker first and its runtime opens on `LogoFMV` instead. HD may
well do the same. **Anyone filling in `BootProfile` from this table is filling
in a declaration, and the doc comment must say so.**

Two specific ambiguities inside the declaration, both unresolved:

- **Step 4 is genuinely two-valued.** `EpilepsyWarning` carries *both* a
  `<Dialog>` whose single option goes to `FirstPlay` and a
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
  is an **inference, confidence 55**, not a reading. `FirstPlay` itself is
  conditional in name ("new to WipEout?") and its dialog sends both YES and NO
  to `Save Warning`, so a boot that skips it lands in the same place.

- **`Language Selection` may not run at all.** The PS3 takes its system
  language from the XMB. The screen exists, is fully authored, and has a
  `<DisplayLanguages>` widget - but whether a PS3 boot ever shows it, or whether
  the engine reads the XMB setting and jumps straight to `PreFMVConnect`, is
  exactly the kind of thing only an emulator or the EBOOT can settle.

### Two chain families

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
the Fury-only circuits. **That makes `DATA00` look like the newest layer, and
looking like it is not evidence.** Nothing here establishes load order.

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
| `text` | yes | `0xFFFFFFFF` | 92 | `TextColor`, all six. |
| `title` | yes | `0xFF646464` | 90 | `TitleColor`, all six - but see the caveat. |
| `selected` | measured field | `None` | - | No capture. Needs an emulator. |
| `transition_secs` | yes | `0.5` | 70 | Dominant `<LeftLayer transition=>`; see the warning. |

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
or has a compiled-in table is unknown, and is the same open question Pure's
`FALLBACK_GLOBALS` records for colours. So `None` here means "HD's rows are
drawn in the default face" *as a finding*, and the doc comment on the HD crate
should say which of the two `None` meanings it is - Pure's precedent.

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
engine drawing it.

The only `.bik` files on the disc that are not per-track previews or UI icons
are the two Studio Liverpool logos. There is nothing a `menu_backdrop` could
point at even if the type wanted one.

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

**HD is Pulse's front end scaled up, not a new one.** The element vocabulary
(`Screen`, `Variable global=`, `Redirect`/`Default goto=`, `Menu`, `LeftLayer`,
`FEGlobals->` references, `LoadXML`) is the same, the shortened-dialect
difference aside. Which is the same headline
[hd-status](hd-status.md) reached for the asset layer: not a new engine.

## What the executable says

Read out of `/ps3-hdfury-eu/EBOOT.elf` in the project's Ghidra database on
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
declared chain opens on `Language Selection` and the only alternative the
executable offers is `Launch Game`, which is the straight-into-a-race path a
demo or trial boot would take, not a different front-end order. That is Pure's
situation rather than Pulse's - but it is *not* the cold-boot confirmation Pure
has, and this page still does not assert a runtime order.

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
live alternative reading. The consuming loop was not identified: the only xref
Ghidra offers into `0x00860c80` is a read at `0x0032e5d8`, inside a function that
decompiles as a GameData-installer debug dump, and the TOC-relative loads that
would reach the array leave no instruction operand to search for.

**So the archive-layering question stays exactly where it was** before the
executable was opened - and until the consumer is found, the *order* in the
array carries no priority meaning either. A first-wins array and a last-wins
array are byte-identical.

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

## What could not be determined

Named explicitly, with what each would take.

**Still needs the PS3 executable, after the sweep above:**

1. **What consumes the seven-archive array at `0x00860c80`, and with what
   polarity.** Until the consumer is found it is not even established that the
   array is a mount list, and a first-wins and a last-wins list look identical.
   This is the *single* blocker on "which `skin.xml` is live", and therefore on
   which boot-chain family and which logo `.bik` are HD's. It does not affect
   `MenuSkin`; all six copies agree there.
2. **The call site of `0x000186f0`**, to lift the boot entry point from
   inference (70) to a reading.
3. **Whether an unresolved `font=` falls back to `Default` or to a compiled-in
   table.** `font="menu"` and `font="InGame"` name slots no plugin declares.
   Cannot change `menu_font`, which is `None` either way.
4. **The easing curve on a page change**, the same gap Pulse has.

**Needs an emulator (RPCS3 or equivalent) - none was available here:**

5. **The runtime boot order.** Everything in the chain table is *declared*.
   `0x000186f0` now gives a 70-confidence reason to think the runtime's default
   first screen is the picker, which is a partial answer to the *first* step
   only; the remaining eight are unmeasured, and Pulse's declared and actual
   orders differ, so this is a real risk and not a formality.
6. **Whether `Language Selection` is ever shown** on a machine that takes its
   language from the XMB. `0x000186f0` returns its name by default, which is
   evidence the engine *intends* to open there, not evidence a player sees it.
7. **Step 4's real path** - `EpilepsyWarning` -> `FirstPlay` -> `Save Warning`
   as the dialog declares, or `EpilepsyWarning` -> `Save Warning` as the
   redirect declares.
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

**Not attempted here:**

13. **`ingame_definition.xml`** (84-122 KiB per copy, 60+ screens) - the in-race
    HUD and pause tree. Out of scope for a front end.
14. **`entries.xml`** - 16 languages x ~125 KiB of string tables. The
    `idstring` values quoted above (`FE_MM`, `FE_RC`, `BOOT_HEALTH_WARN_1`)
    were not resolved to text.
15. **`.gtf` front-end textures.** 7,333 on the disc; none decoded for this.

## Implemented where

Nowhere yet. This page is the input to an `oag-hd` title package, not a record
of one. When that crate lands, [ADR-0022] governs its shape and [ADR-0023] its
boot table, and this page becomes the citation its doc comments point at - the
way [pure-boot](../architecture/pure-boot.md) backs `oag_pure::frontend`.

[ADR-0022]: ../architecture/adr/0022-title-packages.md
[ADR-0023]: ../architecture/adr/0023-boot-sequence-as-title-data.md
