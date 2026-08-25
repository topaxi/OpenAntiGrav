# Pure's HUD shows raw `IG_HUD_*` keys, because its English plugin names no string table

2026-08-12. A Pure race draws `HUD_CURRENT` / `HUD_BEST` / `HUD_LAP` where Pulse draws `actuel` / `meilleur` / `tour`. `load_strings` picks English, and Pure's `PI012` - the plugin `find_language_attribute` calls English - carries no `Dynamic Entry File Source` entry, so there is no `entries.xml` to read and `StringTable::get_or_id` falls back to the key. The report says `English names no string table` plainly, so this is visible rather than silent. Whether Pure's English strings live in `PI003` (also `Language="English"`, also scanned by nothing today) or somewhere else is unread.

## Open

- Where Pure's English HUD strings actually live (`PI003` or elsewhere) is unread

## Next Steps

- No next step named in the original record - read the prose above and decide one.
