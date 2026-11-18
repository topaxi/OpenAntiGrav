# oag-schema

The `[toml-schema]` header every TOML file of this project carries, and the one
place that reads it: kind and major version come from the header's `location`, and
an unknown one is refused by name.

No workspace dependency. See
[ADR-0064](../../docs/architecture/adr/0064-project-toml-files-declare-a-toml-schema.md).
