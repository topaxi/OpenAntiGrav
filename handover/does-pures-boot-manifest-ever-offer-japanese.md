# Does Pure's boot manifest ever offer Japanese (`PI005`) in its picker?

2026-08-27. Pure's `language_plugins` list is now `PI000` (English), `PI008`
French, `PI009` German, `PI010` Spanish, `PI011` Italian - read directly off
both pressings and asserted in
`pure_boot_ground_truth.rs::pures_picker_offers_its_own_five_languages_english_included`.
See `docs/formats/pure-status.md#the-language-plugin-id-space-is-pures-own-not-pulses`.

`PI005` is a complete, structurally picker-worthy Japanese plugin present on
both pressings - eight `<Font Language="Japanese">` blocks and its own inline
strings, exactly the shape `PI000` has. It is left out of the picker list on
the strength of Pulse's own precedent, not a measurement of Pure's: Pulse's
boot-time plugin manifest, read directly out of the executable
(`docs/architecture/frontend-boot.md#the-eu-disc-pulse-psp-eudchd`,
`FUN_0888b980` USA / `FUN_0888b7dc` EU), loads `PI005` but never lists it
among the ids the picker offers, on either region. No equivalent read has
been done on Pure's own executable - the analogy is reasonable (identical
plugin shape, identical role in the disc's own comment as an unlocalised
Japanese-support plugin) but unconfirmed.

## Open

- Whether Pure's own boot-time plugin manifest ever lists `PI005` among the
  picker's ids is unread - no Ghidra pass has walked Pure's equivalent of
  Pulse's `FUN_0888b980`/`FUN_0888b7dc` plugin-table walk.

## Next Steps

- Find Pure's `BOOT.BIN` function that walks its plugin table (the Pulse
  precedent used `strings`/`diff_functions` to locate the EU counterpart by
  positional correspondence through an already-matched call chain - the same
  approach should work here) and read its plugin list directly, USA and EU
  both.
- If `PI005` is in the manifest at all, confirm whether it is loaded as a
  language choice or as something else (a hidden support plugin loaded
  unconditionally, the way Pulse's own manifest loads it without offering it).
