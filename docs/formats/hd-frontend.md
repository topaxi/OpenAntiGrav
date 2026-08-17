# Wipeout HD / Fury: the front end, as its own disc states it

**This page is a reading, not a measurement.** It records what
*Wipeout HD / Fury*'s front-end XML authors, quoted, so that an `oag-hd` title
package can be filled in from evidence rather than from Pulse's numbers. It is
the HD counterpart of [menus-original](../ui/menus-original.md) and
[pure-boot](../architecture/pure-boot.md), and it sits beside
[hd-status](hd-status.md), which is the format-layer probe this continues.

**Nothing here has been verified under an emulator, and no PS3 executable has
been read for any of it.** Every claim below rests on static reading of the
disc's own XML. Per the [rubric](../reverse-engineering/confidence-rubric.md)
that caps an authored-value claim at 94 and puts any *runtime* claim - what the
boot actually walks, what colour a selected row is, how long a page change takes
on screen - at zero, because none was made.

The disc is `hdfury-ps3-eu-dec.iso`, serial `BCES-00664`, layer-1 decrypted per
[ps3-disc](ps3-disc.md); everything is read through the
[`.psarc` reader](psarc.md).

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Where is the skin? | `/data/plugins/frontend/gui/skin.xml`, in **six** of the seven archives | 94 |
| Do the six copies agree on layout? | **Yes, on every layout global, exactly** | 92 |
| Which copy does the runtime load? | **Unknown.** Needs the EBOOT | - |
| Coordinate space | **1920x1080**, not Pulse's 480x272 | 90 |
| Is the main menu a row list? | **No.** It is a `<HorizMenu>` - horizontal | 90 |
| Is there a `menu` font role? | **No.** HD's language plugins declare no such slot | 90 |
| Is there a looping movie backdrop? | **No.** The FE background is a real-time `.vex` scene | 88 |
| Are any `FEGlobals` referenced but undeclared? | **None, in any of the six** | 90 |
| Declared boot chain | Nine screens, quoted below | 80 |
| Runtime boot order | **Unverified.** Needs an emulator | - |

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

`DATA03`'s copies add `wo3HUD` and `2097HUD` (the retro HUD skins). Sampling
five languages across both archives gives slot-name counts of `Default` 10,
`Title` 10, `Buttons` 10, `HUD` 10, `HUDSmall` 10, `wo3HUD` 5, `2097HUD` 5 -
and **no `Menu` slot, in either spelling, anywhere**.

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
    // and the movie is the Studio Liverpool logo - HD's counterpart to Pure's
    // `Developer Publisher Screen`. Same shape as Pure's: on the boot path, so
    // `--reel` shows the screen the disc shows rather than being refused.
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

## What could not be determined

Named explicitly, with what each would take.

**Needs the PS3 executable (`scripts/import-ps3-eboot.sh`, ghidra-mcp):**

1. **Which `skin.xml` the runtime loads.** Six copies, no manifest, no declared
   priority. This does not affect `MenuSkin` (all six agree) but it decides
   which of the two boot-chain families is live and which logo `.bik` plays.
2. **How `.psarc` archives layer.** Whether a later archive shadows an earlier
   one, and in what order they mount.
3. **Whether an unresolved `font=` falls back to `Default` or to a compiled-in
   table.** `font="menu"` and `font="InGame"` name slots no plugin declares.
4. **The easing curve on a page change**, the same gap Pulse has.

**Needs an emulator (RPCS3 or equivalent) - none was available here:**

5. **The runtime boot order.** Everything in the chain table is *declared*.
   Pulse's declared and actual first screens differ, so this is a real risk and
   not a formality.
6. **Whether `Language Selection` is ever shown** on a machine that takes its
   language from the XMB.
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
