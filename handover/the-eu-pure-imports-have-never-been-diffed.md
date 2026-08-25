# The EU/Pure imports have never been diffed

Both Pulse builds and both Pure builds are imported and folder-organised per binary. **The payoff has not been cashed in**: run `diff_functions`/`compare_programs_documentation` between `/psp-pulse-usa/BOOT.BIN` and `/psp-pulse-eu/BOOT.BIN` to test the networking-code hypothesis. Same opportunity for the two Pure builds, with no hypothesis yet to test. When cleaning up a botched import, do the delete/rename in the Ghidra GUI - the bridge does not reliably release a program.

## Open

- The EU/Pure imports have never been diffed against their USA/Pulse counterparts, so the networking-code hypothesis is untested
- No hypothesis yet exists for what a diff between the two Pure builds would show

## Next Steps

- Run `diff_functions`/`compare_programs_documentation` between `/psp-pulse-usa/BOOT.BIN` and `/psp-pulse-eu/BOOT.BIN` to test the networking-code hypothesis
- Run the same diff between the two Pure builds (no hypothesis yet to test)
