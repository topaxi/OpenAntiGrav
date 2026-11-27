# oag-testdata

Locates the user-supplied disc images from a test, and applies the
`OAG_REQUIRE_GAME_DATA` rule: skip when the data is missing, fail when it is
required.

Also where a test writes ([`scratch`](src/scratch.rs)): a guard-removed
directory or a stable cache under `target/tmp`, never the OS temp directory
(`just check-temp-use` enforces it).

Dev-dependency only. It holds no game knowledge and no image or archive names.
