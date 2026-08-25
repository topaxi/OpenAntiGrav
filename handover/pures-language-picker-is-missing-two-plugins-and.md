# Pure's language picker is missing two plugins, and naive discovery makes it worse

2026-08-12. `boot::load_languages` scans `oag_pulse::LANGUAGE_PLUGINS` = `PI008`..`PI012`, which is wrong twice over: it misses Pure's `PI003` and `PI005`, and it names `PI012`, which the **EU Pulse pressing does not carry**. Measured across both discs by hashing `Data\Plugins\PI0NN\Definition.xml` for `NN` in 1..32: Pulse EU fills `PI001`, `PI004`, `PI008`-`PI011`; Pure fills `PI001`, `PI003`, `PI004`, `PI005`, `PI008`-`PI012`. **Probing the id space is the obvious fix and it is not safe as written**: `Language::from_definition` keys on the first `Language=` attribute anywhere in the tree, and on Pure `PI003`, `PI005` and `PI012` would *all* come back English - `PI005` is the Japanese font plugin and carries an English `<Entry>` before its `<Font Language="Japanese">` blocks. So discovery needs `find_language_attribute` re-measured first (probably: prefer the `<Font>` block's own `Language`), then a dedupe rule, or the picker gains duplicate rows and loses Japanese. Not attempted; the milestone did not need it.

## Open

- `boot::load_languages` misses Pure's `PI003` and `PI005`, and names `PI012` which the EU Pulse pressing does not carry
- Naive id-space probing is unsafe: `PI003`, `PI005` and `PI012` would all resolve as English under `find_language_attribute`'s current first-match rule, and `PI005` (Japanese font plugin) would be misclassified

## Next Steps

- Re-measure `find_language_attribute` (likely: prefer the `<Font>` block's own `Language`), then add a dedupe rule, before probing the plugin id space
