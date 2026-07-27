# Tools

| Tool | Status | Purpose |
| --- | --- | --- |
| [`oag-unpack`](oag-unpack.md) | working | Inspect and extract disc images |
| [`oag-view`](oag-view.md) | **working** | Display assets from a disc image; headless screenshots |
| `oag-convert` | M1 | Convert assets to standard formats |
| [`oag-trace`](oag-trace.md) | **working** | Compare a simulation run against a trace from the original |
| [`oag-game`](oag-game.md) | **working** | The engine itself: the front end, and the race its `Launch Game` starts |

All are run through `just` for convenience:

```sh
just unpack info data/images/pulse-psp-usa.chd
```

or directly:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```
