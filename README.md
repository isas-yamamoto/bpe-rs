# BPE Rust (`bpe_rs`)

[![CI](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/isas-yamamoto/bpe-rs/actions/workflows/ci.yml)

C 参照実装とビットストリーム互換の純 Rust Bit Plane Encoder。

## 関連リポジトリ

| リポジトリ | 内容 |
|---|---|
| [bpe-training](https://github.com/isas-yamamoto/bpe-training) | アルゴリズム学習ドキュメント（walkthrough / 学習ストーリー / 用語集ほか） |
| [bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison) | C 参照実装とのバイト一致検証（ゴールデンテスト） |

## 互換性（検証済み）

同一入力で Rust / C の `.bpe` がバイト一致。クロスデコードも raw 一致。
検証手順は [bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison) を参照
（C 参照実装のバイナリが必要なためローカル専用）。

## アルゴリズム概要

要約（エンコード）:

1. `encoder_engine` — パディング → DWT → ブロック並べ替え → セグメントループ
2. `dc_encoding` — 統計 → ビット深度 → ヘッダ → 量子化 → DPCM → エントロピー
3. `ac_bpe_encoding` — AC depth → ビットプレーンループ
4. 各プレーン: `block_scan_encode` → `stages_en_coding`（gaggles1/2/3 → refine）

デコードは鏡像: `dc_decoding` → `ac_bpe_decoding` → `stages_de_coding` → `adjust_output` → 逆 DWT。

初学者向けの詳しい解説は [bpe-training](https://github.com/isas-yamamoto/bpe-training) にあります。

## 構成

```
src/
  main.rs                 # CLI（-e/-d/-o/-r/-w/-h/-b/-f/-t/-s/-g）
  types.rs, error.rs
  bitstream/, header.rs   # bitstream: common | encode | decode
  image_io/               # common | size | read | write
  rice/                   # encode | decode | select_k
  encoder.rs, decoder.rs  # パイプライン入口
  block/, adjust/         # block: common|orchestrate|type_*/tran_*
  dc/                     # twos_comp | dpcm | entropy | coding
  ac/                     # depth | bpe
  pattern/                # mapping | options
  stages/                 # gaggles1..3 | refine | orchestrate | common
  wavelet/                # integer/float 9/7 lifting
```

エンコード／デコードはすべて純 Rust。C FFI や `c_bridge` は含まない。

## ブランチ運用

- `main` — 安定版。タグ付きリリース（`v*`）はここから作成する。
- `develop` — 日々の開発ブランチ。安定したら `main` にマージしてタグを打つ。
- `archive/pre-public` — 履歴整理前の開発記録（参照用）。

## CI / CD

GitHub Actions で次を自動実行する。

| ワークフロー | 契機 | 内容 |
|--------------|------|------|
| [`ci.yml`](.github/workflows/ci.yml) | push / PR (main, develop) | `cargo fmt --check`、`cargo clippy`、Linux/Windows/macOS で `cargo test` とラウンドトリップ検証 |
| [`release.yml`](.github/workflows/release.yml) | `v*` タグ | 3 OS 分の `bpe` バイナリをビルドし GitHub Release に公開 |

C 参照実装とのバイト一致検証は [bpe-c-comparison](https://github.com/isas-yamamoto/bpe-c-comparison) 側で行う。
このリポジトリの CI では `scripts/ci_roundtrip.py` が encode -> decode を通し、サイズと画素誤差を検査する。

## ビルド / 実行

```bash
cargo build --release
./target/release/bpe -e in.raw -o out.bpe -r 1.0 -w 256 -h 256
./target/release/bpe -d out.bpe -o decoded.raw
```

Windows では `.\target\release\bpe.exe` を使う。
