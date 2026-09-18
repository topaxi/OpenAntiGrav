# HD's end screens are located and not read

2026-09-18, off a log line rather than a reading pass. Full write-up in
[hd-frontend.md](../../docs/formats/hd-frontend.md)'s own "the end screens are
located, not read".

**The one to know**: Wipeout HD ships its own EndRace screens and this build
was looking for Pulse's. `oag_game::endrace::SCREEN_ENTRY` is
`Data\Plugins\PI001\GUI\EndRace_Definition.xml`, a numbered PSP plugin; HD
authors `/data/plugins/frontend/gui/endrace_definition.xml`, in five of its
seven archives (`DATA02`-`DATA06`, no two copies alike by MD5), the same
named-plugin divergence `oag_title::FrontEnd::root` already carries for
`Skin.xml`.

Three of HD's screen names match Pulse's, so **swapping the constant is the
wrong fix**: `EndRace Results`/`Rewards`/`Menu` would all be found in
`DATA02`'s copy and draw nearly nothing, because the widgets under them are a
different vocabulary - and Results is a different screen entirely. Pulse's
Results is the player's own per-lap table (`lap{n}.{c}`); HD's is the
finishing order of the whole field (`Grid{row}.{col}`, eight rows by ten
columns, with `MedalBlock`/`MedalModelGold|Silver|Bronze` and a
`RecordNotifyBlock` beside it). HD's Menu is one `<Block>` per option shown by
mode (`race_again`, `return_to_grid`, `return_to_menu`, `next_race`,
`view_again`, `quit_tournament`, `return_to_lobby`, `view_MP_again`) rather
than a populated list, and its `loyaltybar` is a `<Slider>` where Pulse's is
an `<Image>`. `DATA05` adds an `EndRace Podium` screen; `DATA06` carries
Podium and **no Rewards at all**.

What did change in code: the failed load is no longer retried per frame.
`RaceStage::endrace_unavailable` records the first failure, so the disc image
is reopened once per race rather than sixty times a second, and the warning is
one line per race. Its text now says the screens are not *read by this build*
rather than not present on the source, which was factually wrong for HD.

## Open

- **Nothing reads HD's `endrace_definition.xml`.** The inventory above is the
  whole of what was taken: the five copies' sizes, MD5s and screen lists, and
  `DATA02`'s and `DATA06`'s widget names. Four of the five were not read
  widget by widget, and no `<Values>` number from any of them is recorded
  anywhere.
- **Which copy the runtime loads is unresolved**, the same open question
  [hd-frontend.md](../../docs/formats/hd-frontend.md) records for `skin.xml`'s
  six copies. `oag_assets::Archives` would serve `DATA02`'s - `holder_of`
  walks `data` (`DATA00`, which carries no copy) then `fe` (`DATA02`) - which
  is this build's own mount order, not a measurement of the original's.
- **`EndRace Podium` has no counterpart in this build at all.** Pulse has no
  such screen, so it is not a variant of something already modelled - and
  whether it is the campaign's own reward screen under another name, or the
  tournament's, is unread.
- **The `Grid{n}.{c}` table needs a field-wide standings model.**
  `oag_ui::endrace::Results` carries the player's laps and total, which is
  what Pulse's screen wants; HD's wants every craft's position, name and time.
  `oag_race` already computes standings for the built-in scoreboard - whether
  that is the right feed, or whether HD's ten columns want something else, is
  the first thing to settle.
- **2048 and Pure reach the same failure** and neither was looked at. The
  once-per-race line is all that is known about either.

## Next Steps

1. **Read `DATA02`'s copy widget by widget** into a
   `docs/formats/hd-endrace-screens.md` of its own, the way
   [endrace-screens.md](../../docs/ui/endrace-screens.md) reads Pulse's -
   every `<Values>` number quoted, each claim scored.
2. **Capture the real screens.** `just rpcs3-race` drives a cold HD boot into
   a race with no window on anyone's desktop, and HD names every screen it
   enters on `TTY.log`
   ([rpcs3-debugger.md](../../docs/reverse-engineering/rpcs3-debugger.md)) -
   so which copy is live, and whether Podium is ever entered, are readings
   waiting to be taken rather than blocked ones.
3. **Then decide the seam**, and not before: `oag_game::endrace::SCREEN_ENTRY`
   becoming a title axis (`oag_title::FrontEnd`, beside `race_box`) is the
   obvious shape, and it is worth nothing until a second title's screens
   actually draw. A constant that names a file this build cannot read would
   trade one honest absence for a blank screen.
