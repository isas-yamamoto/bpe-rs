# BPE Rust (`bpe_rs`)

[![CI](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml)

A pure Rust Bit Plane Encoder whose bitstream is compatible with the C
reference implementation.

## Related repositories

| Repository | Contents |
|---|---|
| [bpe-training](https://github.com/isas-yamamoto/bpe-training) | Learning documentation for the algorithm, in Japanese (walkthrough, guided chapters, glossary) |
| [bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison) | Byte-level compatibility verification against the C reference (golden tests) |

## Attribution

This implementation follows the [CCSDS 122.0](https://public.ccsds.org/)
Recommended Standard for Image Data Compression. Bitstream compatibility
was verified against the C reference implementation by Hongqiang Wang at
the University of Nebraska-Lincoln (historically distributed from
http://hyperspectral.unl.edu/).

The UNL source code is not included in this repository. See
[`NOTICE`](NOTICE) for details. The license of this repository itself is
still under review.

## Compatibility (verified)

For identical inputs, the Rust and C encoders produce byte-identical
`.bpe` files, and cross decoding yields identical raw output. See
[bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison)
for the procedure (local only, since it needs the C reference binary).

## Algorithm overview

Encoding, in short:

1. `encoder_engine` — padding, DWT, block reordering, segment loop
2. `dc_encoding` — statistics, bit depth, header, quantization, DPCM, entropy coding
3. `ac_bpe_encoding` — AC depth, bit-plane loop
4. per plane: `block_scan_encode` then `stages_en_coding` (gaggles1/2/3, refine)

Decoding mirrors it: `dc_decoding`, `ac_bpe_decoding`, `stages_de_coding`,
`adjust_output`, inverse DWT.

A beginner-friendly explanation (in Japanese) lives in
[bpe-training](https://github.com/isas-yamamoto/bpe-training).

## Layout

```
src/
  main.rs                 # CLI (-e/-d/-o/-r/-w/-h/-b/-f/-t/-s/-g)
  types.rs, error.rs
  bitstream/, header.rs   # bitstream: common | encode | decode
  image_io/               # common | size | read | write
  rice/                   # encode | decode | select_k
  encoder.rs, decoder.rs  # pipeline entry points
  block/, adjust/         # block: common|orchestrate|type_*/tran_*
  dc/                     # twos_comp | dpcm | entropy | coding
  ac/                     # depth | bpe
  pattern/                # mapping | options
  stages/                 # gaggles1..3 | refine | orchestrate | common
  wavelet/                # integer/float 9/7 lifting
```

Encoding and decoding are pure Rust; there is no C FFI or `c_bridge`.

## Branch model

- `main` — stable. Tagged releases (`v*`) are cut here.
- `develop` — day-to-day development. Merged into `main` once stable, then tagged.

## CI / CD

GitHub Actions runs the following.

| Workflow | Trigger | Contents |
|----------|---------|----------|
| [`ci.yml`](.github/workflows/ci.yml) | push / PR (main, develop) | `cargo fmt --check`, `cargo clippy`, then `cargo test` and a round-trip check on Linux/Windows/macOS |
| [`release.yml`](.github/workflows/release.yml) | `v*` tags | builds the `bpe` binary for the three OSes and publishes a GitHub Release |

Byte-level comparison against the C reference happens in
[bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison).
CI in this repository runs `scripts/ci_roundtrip.py`, which encodes and
decodes a test image and checks output size and pixel error.

## Build / run

```bash
cargo build --release
./target/release/bpe -e in.raw -o out.bpe -r 1.0 -w 256 -h 256
./target/release/bpe -d out.bpe -o decoded.raw
```

On Windows, use `.\target\release\bpe.exe`.
