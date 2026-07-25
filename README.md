# BPE Rust (`bpe_rs`)

[![CI](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml)

`original/source` とビットストリーム互換の純 Rust Bit Plane Encoder。

## 互換性（検証済み）

同一入力で Rust / C の `.bpe` がバイト一致。クロスデコードも raw 一致。

```powershell
.\scripts\golden_test.ps1   # 要: original/source/bpe.exe
```

## アルゴリズムの読み方

初心者向けは **具体例（walkthrough）** から読んでください。数値で感覚をつかんでから、学習ストーリーの各章へ進みます。

- **[docs/walkthrough_ja.md](docs/walkthrough_ja.md)** — 具体例で追う（初心者必読・20〜30分）
- **[docs/learn_ja.md](docs/learn_ja.md)** — 学習ストーリー（章の一覧）
- **[docs/glossary_ja.md](docs/glossary_ja.md)** — 用語集
- **[docs/entropy_coding_ja.md](docs/entropy_coding_ja.md)** — なぜエントロピー符号化と呼ぶのか
- **[docs/code_reading_ja.md](docs/code_reading_ja.md)** — 実装を読み解くガイド（ソースの地図）

地図・詳解は次の通り（ストーリー順）:

0. [walkthrough_ja.md](docs/walkthrough_ja.md) — 具体例で追う（必読）
1. [algorithm_ja.md](docs/algorithm_ja.md) — 全体地図
2. [steps_ja.md](docs/steps_ja.md) — パイプラインの歩き方
3. [lifting97_ja.md](docs/lifting97_ja.md) — 画像を周波数へ（DWT）
4. [coeff_group_ja.md](docs/coeff_group_ja.md) — 8x8 の木に組む
5. [header_bitstream_ja.md](docs/header_bitstream_ja.md) — ビットの入れ物
6. [dc_coding_ja.md](docs/dc_coding_ja.md) — まず大まかな明るさ（DC）
7. [rice_ja.md](docs/rice_ja.md) — 共通の圧縮道具（Rice）
8. [block_scan_ja.md](docs/block_scan_ja.md) — ブロックを森状に走査
9. [ac_stages_ja.md](docs/ac_stages_ja.md) — 細かい係数を重いビットから
10. [adjust_ja.md](docs/adjust_ja.md) — 途中で止まったとき
11. [verify_ja.md](docs/verify_ja.md) — 手を動かして確かめる

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
