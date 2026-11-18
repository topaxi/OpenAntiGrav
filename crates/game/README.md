# oag-game

The composition root: the windowed application, the command line, the front end
boot and the headless runs for every supported title. A thin binary over a library
so the boot sequence tests without a GPU.

Nothing may depend on it (dependency rule 2). Run it with
[oag-game](../../docs/tools/oag-game.md); the crate map is
[workspace layout](../../docs/architecture/workspace-layout.md).
