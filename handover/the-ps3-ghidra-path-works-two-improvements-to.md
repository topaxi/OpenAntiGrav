# The PS3 Ghidra path works; two improvements to it are still open

2026-08-17. Importing `EBOOT.elf` needs no processor module - stock Ghidra decodes the Cell PPU as `PowerPC:BE:64:A2ALT-32addr` - but it does need a decrypt step, a hand-edited compiler spec and a script pack. Full procedure and its traps in [toolchain.md](../docs/reverse-engineering/toolchain.md#ps3). **Three improvements, ranked; the third is done.** (1) **Fork Ps3GhidraScripts to ship a language** rather than patching Ghidra: `ghidra-allegrex` and `ghidra-emotionengine-reloaded` both carry their own `data/languages/` with a `.cspec`, `.ldefs` and an `.opinion`, which is proof an extension can do this. A vendored cspec **retires `scripts/patch-ghidra-ppc-cspec.sh` entirely** - that script exists only because the stock spec is wrong and root-owned - and an `.opinion` gets the language auto-selected on import instead of chosen by hand. Cost, measured: sleigh `@include` resolves relative to the including file, so an extension cannot reach `/opt/ghidra/Ghidra/Processors/PowerPC/`, and the fork must vendor **21 `.sinc` files, 19,842 lines**, re-synced on every Ghidra upgrade. Half a day for the cspec-only version, which needs no vendoring at all if it reuses the stock `.sla`. (2) **Teach sleigh `lvlx`** - one instruction, not the eight the README implies: `EBOOT.elf` has **851 `lvlx`** and zero `lvrx`/`stvlx`/`stvrx` or `l`-variants. This is what forces the vendoring, so do it only once those 851 sites are in the way. Worth asking clienthax first; upstream is active and a cspec-shipping extension helps every user of that repo. (3) **A headless import script** - *done, 2026-08-17*: `scripts/import-ps3-eboot.sh` drives `analyzeHeadless` with the loader, the processor and the pre/post scripts in one command, decrypts the SELF first if the ELF is missing, and refuses to run without the cspec fix or while Ghidra holds the project lock. `scripts/apply-ghidra-names.py`'s `BINARY_PROGRAMS` also has its `ps3-hdfury-eu` row now.

## Open

- No forked Ps3GhidraScripts language exists yet - the project still relies on patching stock Ghidra (`scripts/patch-ghidra-ppc-cspec.sh`).
- Sleigh does not decode `lvlx` (851 sites in `EBOOT.elf`), which is what forces vendoring the 21 `.sinc` files if a language fork is done today.

## Next Steps

- Build the cspec-only fork first (about half a day, no vendoring needed if it reuses the stock `.sla`); ask clienthax before vendoring the `.sinc` files.
- Teach sleigh `lvlx` once the 851 sites are actually in the way.
