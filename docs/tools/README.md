# Tools

| Tool | Status | Purpose |
| --- | --- | --- |
| [`oag-unpack`](oag-unpack.md) | working | Inspect and extract disc images |
| `oag-view` | M1 | Preview decoded assets |
| `oag-convert` | M1 | Convert assets to standard formats |
| `oag-trace` | M3 | Compare a simulation run against a trace from the original |
| `oag-game` | M4 | The engine itself |

All are run through `just` for convenience:

```sh
just unpack info data/images/pulse-psp-usa.chd
```

or directly:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```
