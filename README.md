# BPE Rust (`bpe_rs`)

[![CI](https://github.com/yamamo-to/bpe-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/yamamo-to/bpe-rs/actions/workflows/ci.yml)

`original/source` とビットストリーム互換の純 Rust Bit Plane Encoder。

## 互換性（検証済み）

同一入力で Rust / C の `.bpe` がバイト一致。クロスデコードも raw 一致。

```powershell
.\scripts\golden_test.ps1   # 要: original/source/bpe.exe
```

## アルゴリズムの読み方

学生向けの流れ・用語の解説と図（Mermaid）は次を参照してください。

- **[docs/algorithm_ja.md](docs/algorithm_ja.md)** — 全体マップ
- **[docs/steps_ja.md](docs/steps_ja.md)** — 段階ごとのステップ解説
- **[docs/verify_ja.md](docs/verify_ja.md)** — CLI / `cargo test` / ゴールデンでの検証

要約（エンコード）:

1. `encoder_engine` — パディング → DWT → ブロック並べ替え → セグメントループ
2. `dc_encoding` — 統計 → ビット深度 → ヘッダ → 量子化 → DPCM → エントロピー
3. `ac_bpe_encoding` — AC depth → ビットプレーンループ
4. 各プレーン: `block_scan_encode` → `stages_en_coding`（gaggles1/2/3 → refine）

デコードは鏡像: `dc_decoding` → `ac_bpe_decoding` → `stages_de_coding` → `adjust_output` → 逆 DWT。

## 構成

```
rust/
  src/
    main.rs                 # CLI（-e/-d/-o/-r/-w/-h/-b/-f/-t/-s/-g）
    types.rs, error.rs
    bitstream.rs, header.rs, image_io.rs
    rice.rs                 # Rice 本体 + gaggle の k 選択
    encoder.rs, decoder.rs  # パイプライン入口
    block.rs, adjust.rs
    dc/                     # twos_comp | dpcm | entropy | coding
    ac/                     # depth | bpe
    pattern/                # mapping | options
    stages/                 # gaggles1..3 | refine | orchestrate | common
    wavelet/                # integer/float 9/7 lifting
```

エンコード／デコードはすべて純 Rust。C FFI や `c_bridge` は含まない。

## CI / CD

GitHub Actions で次を自動実行する。

| ワークフロー | 契機 | 内容 |
|--------------|------|------|
| [`ci.yml`](.github/workflows/ci.yml) | push / PR (main) | `cargo fmt --check`、`cargo clippy`、Linux/Windows/macOS で `cargo test` とラウンドトリップ検証 |
| [`release.yml`](.github/workflows/release.yml) | `v*` タグ | 3 OS 分の `bpe` バイナリをビルドし GitHub Release に公開 |

C 参照実装はこのリポジトリに含まれないため、バイト一致の `scripts/golden_test.ps1` はローカル専用。
CI では `scripts/ci_roundtrip.py` が encode -> decode を通し、サイズと画素誤差を検査する。

## ビルド / 実行

```powershell
cd rust
$env:CARGO_TARGET_DIR = "$PWD\target"
cargo build --release
.\target\release\bpe.exe -e in.raw -o out.bpe -r 1.0 -w 256 -h 256
.\target\release\bpe.exe -d out.bpe -o decoded.raw
```
