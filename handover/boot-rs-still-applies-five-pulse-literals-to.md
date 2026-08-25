# `boot.rs` still applies five Pulse literals to every source

Scoped out of the ADR-0023 pass deliberately, and listed so the next one does not have to re-derive them: the front-end root (`Data\Plugins\PI001\GUI\Skin.xml`), the font set, the language-plugin ids, `Data\Plugins\PI001\Definition.xml`, and `load_teams`' fallback to `oag_formats::handling::TEAMS` (Pulse's eight, on a source with nine). The first four are *identical* on both discs - `oag_pure::names::GAME_PLUGIN_DEFINITION` already restates one verbatim and is unused - so routing them through the boot profile would add indirection with no behavioural difference today. The `TEAMS` fallback is a real divergence but unreachable, Pure never reaching a race. Real ADR-0022 debt; move them when a third title or a Pure race forces it.

## Open

- `boot.rs` still hardcodes five Pulse literals (front-end root, font set, language-plugin ids, `Definition.xml`, `TEAMS` fallback) rather than routing them through the boot profile.
- The `TEAMS` fallback is a real divergence (Pulse's eight teams vs. a nine-team source) but unreachable, since Pure never reaches a race.

## Next Steps

- Move the five literals into the boot profile when a third title or a Pure race forces the issue.
