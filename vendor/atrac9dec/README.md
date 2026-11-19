# atrac9dec

Pure-Rust decoder for Sony **ATRAC9** (Adaptive TRansform Acoustic Coding 9).

Port of [LibAtrac9](https://github.com/Thealexbarney/LibAtrac9) (MIT) by Alex Barney.

## Status

**Sample-identical to LibAtrac9** on the reference stereo 48 kHz stream  
(`sample.at9`: 466 superframes, 954 368 samples, 0 differences).

| Component | Status |
|-----------|--------|
| Bitstream / Huffman / scale factors | ✅ Bit-position matched to C |
| IMDCT + window + overlap | ✅ Literal C tables/formulas |
| Full-file PCM vs LibAtrac9 | ✅ 100% identical |
| BEX | Payload skipped but bitstream aligned (OK for this stream) |
| LFE / multi-channel | Partial |

### Production readiness

| Component | Status |
|-----------|--------|
| Core ATRAC9 decoder | 🟢 Yes |
| Normal streams covered by current tests | 🟢 Yes |
| Error handling | 🟢 Yes |
| Reference equivalence | 🟢 Strongly validated |
| DMC 16–64ch | 🟡 Not yet validated (need working C reference) |
| BEX | ⚪ Out of scope |
| Fuzz/robustness testing | 🟢 `cargo-fuzz` harness + `fuzz_robustness` (20k+ cases, no panic/oom) |
| Entire ATRAC9 implementation | ⚪ Out of scope |

## Installation

```toml
[dependencies]
atrac9dec = "0.1"
```

## Usage

```rust
use atrac9dec::Atrac9Decoder;

let config: [u8; 4] = /* from RIFF fmt chunk */;
let mut decoder = Atrac9Decoder::new(&config)?;
let info = decoder.codec_info();
let mut pcm = vec![0i16; info.channels * info.frame_samples * info.frames_in_superframe];
let bytes_used = decoder.decode(&superframe, &mut pcm)?;
```

## CLI

```bash
cargo run --release --example decode_at9 -- input.at9 output.wav
```

## Tests

```bash
cargo test --release
# pcm_identity: first superframe vs LibAtrac9 golden PCM
# fuzz_robustness: 20k arbitrary/mutated inputs — no panic/oom/hang
```

## Fuzz / robustness

Targeted fuzzing of the decoder entry points (`Atrac9Decoder::new` + `decode`):

* **Property**: any byte input → `Ok` or `Err`, never panic/hang/OOM
* **harness**: `fuzz/fuzz_targets/fuzz_target_1.rs` (libFuzzer + ASAN)
* **stable fallback**: `tests/fuzz_robustness.rs` (runs on `cargo test`)
* **corpus**: `fuzz/corpus/fuzz_target_1/seed_*` + `fuzz/mutate_corpus.py` for deeper mutations of golden `.at9` samples

```bash
cargo install cargo-fuzz && rustup toolchain install nightly
cargo +nightly fuzz run fuzz_target_1 -- -runs=50000          # 30 s smoke
cargo +nightly fuzz run fuzz_target_1 -- -max_total_time=3600 # 1 h
cargo test --test fuzz_robustness --release                   # stable CI gate

# generate corpus from a known-good sample
python fuzz/mutate_corpus.py path/to/golden.at9 --out fuzz/corpus/fuzz_target_1
```

See `fuzz/README.md` for sanitizers, Windows notes, and the full checklist.

## License
MIT