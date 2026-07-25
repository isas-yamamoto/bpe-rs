# BPE アルゴリズムガイド（学生向け）

この文書は、本リポジトリの **純 Rust 実装** がどのような順番で画像を圧縮・復元するかを、日本語で追えるようにしたものです。
細部の数式より「どの関数が何をするか」を優先しています。ソースを読むときの地図として使ってください。

対応する C 実装はリポジトリ直下の `original/source` です。Rust 側はビットストリーム互換を保つことを目的に移植されています。

---

## 全体のイメージ

BPE（Bit Plane Encoder）は、およそ次の流れです。

1. 画像を読み、8 の倍数になるよう端を埋める（パディング）
2. ウェーブレット変換（DWT）で周波数成分に分解する
3. 8×8 ブロックに並べ替える
4. セグメント単位で
   - **DC**（各ブロック左上の低周波係数）を符号化
   - **AC**（残り係数）を **ビットプレーン**（MSB→LSB）ごとに符号化
5. ビットストリームを書き出す

デコードはほぼ逆順です（ビットを読んで係数を復元 → 逆 DWT → 画像出力）。

---

## エンコードのマップ

GitHub / VS Code / Cursor など、Mermaid 対応のビューアで図が表示されます。

```mermaid
flowchart TD
  enc[encoder_engine] --> pad[padding]
  pad --> dwt[dwt_forward]
  dwt --> blocks[build_block_string]
  blocks --> segEnc[segment_loop]
  segEnc --> dcEnc[dc_encoding]
  segEnc --> acEnc[ac_bpe_encoding]
  acEnc --> plane[encode_one_bitplane]
  plane --> scan[block_scan_encode]
  plane --> stages[stages_en_coding]
  stages --> g1[gaggles1_TypeP]
  stages --> g2[gaggles2_TranB_D_Ci]
  stages --> g3[gaggles3_TranGi_Hi_Hij]
  stages --> ref[ref_bits_en]
```

（図中の英名はソース上の関数名です。意味は下表・テキスト版を参照。）

テキスト版（同じ内容）:

```
encoder_engine
  ├─ 画像サイズ確認・パディング
  ├─ dwt_forward（ウェーブレット）
  ├─ build_block_string（8×8 ブロック列）
  └─ セグメントごと
       ├─ dc_encoding
       └─ ac_bpe_encoding
            └─ 各ビットプレーン
                 ├─ （必要なら）DC 残差ビット
                 ├─ block_scan_encode
                 └─ stages_en_coding
                      ├─ gaggles1（TypeP）
                      ├─ gaggles2（TranB / TranD / TypeCi）
                      ├─ gaggles3（TranGi / TranHi / TypeHij）
                      └─ ref_bits_en（リファインメント）
```

入口関数: [`src/encoder.rs`](../src/encoder.rs) の `encoder_engine`

---

## デコードのマップ

```mermaid
flowchart TD
  dec[decoder_engine] --> hdr[header_readin]
  hdr --> segDec[segment_loop]
  segDec --> dcDec[dc_decoding]
  segDec --> acDec[ac_bpe_decoding]
  acDec --> stagesDec[stages_de_coding]
  segDec --> adj[adjust_output]
  segDec --> reassem[reassemble_coeffs]
  reassem --> idwt[dwt_reverse]
  idwt --> out[image_write]
```

テキスト版:

```
decoder_engine
  ├─ header_readin
  └─ セグメントごと
       ├─ dc_decoding
       ├─ ac_bpe_decoding
       │    └─ stages_de_coding（gaggles1→2→3→refine）
       ├─ adjust_output（打ち切り時の係数補正）
       └─ バッファ flush
  ├─ 全セグメントの係数を画像に再配置
  └─ 逆 DWT → 出力
```

入口関数: [`src/decoder.rs`](../src/decoder.rs) の `decoder_engine`

---

## 各段階の役割（短い説明）

| 段階 | 主な場所 | 何をしているか |
|------|----------|----------------|
| パディング | `encoder.rs` | 幅・高さを 8 の倍数に揃える |
| DWT | `wavelet/` | 9/7 リフティングで周波数分解（整数／浮動小数） |
| ブロック化 | `encoder.rs` (`build_block_string`) | 変換後係数を 8×8 の列に並べ替え |
| DC 符号化 | `dc/` | ビット深度決定、量子化、DPCM、Rice 系エントロピー |
| AC depth | `ac/depth.rs` | 各ブロックの AC 最大ビット深度を符号化 |
| ブロック走査 | `block.rs` | 1 プレーン内で親子孫の有意性シンボルを作る |
| ステージ符号化 | `stages/` | ガッグル単位でシンボルを Rice 符号化し、最後に refine |
| パターン | `pattern/` | シンボル値のテーブル写像と符号化オプション選択 |
| 調整 | `adjust.rs` | レート制限で途中停止したときの係数補正（デコード） |

---

## AC のビットプレーンとは

係数の絶対値を 2 進数で見たとき、**上位ビットから順に** 符号化します。

- あるプレーンで初めて「1」になった係数 → 有意性（significance）＋符号
- すでに有意だった係数 → リファインメントビット（そのプレーンの 0/1）

`ac_bpe_encoding` / `ac_bpe_decoding`（[`src/ac/bpe.rs`](../src/ac/bpe.rs)）が、このループの司令塔です。

---

## ステージ（gaggles）の意味

ブロック走査で作ったシンボルは、すぐに全部書くのではなく、**種類ごとにまとめて**（ガッグル＝最大 16 ブロック）符号化します。

| ファイル | シンボルの種類 |
|----------|----------------|
| `stages/gaggles1.rs` | TypeP（親係数） |
| `stages/gaggles2.rs` | TranB / TranD / TypeCi（子孫への遷移と子） |
| `stages/gaggles3.rs` | TranGi / TranHi / TypeHij（孫） |
| `stages/refine.rs` | リファインメントビット |
| `stages/orchestrate.rs` | 上記を 1→2→3→refine の順で実行 |

オーケストレーション: [`src/stages/orchestrate.rs`](../src/stages/orchestrate.rs)

---

## モジュールとソース対応

| 関心 | ディレクトリ / ファイル | 元の C（目安） |
|------|-------------------------|----------------|
| 全体エンジン | `encoder.rs`, `decoder.rs` | `bpe_encoder.c`, `bpe_decoder.c` |
| DC | `dc/` | `DC_EnDeCoding.c` |
| AC | `ac/` | `AC_BitPlaneCoding.c` |
| ブロック走査 | `block.rs` | `BPEBlockCoding.c` |
| ステージ | `stages/` | `StagesCodingGaggles.c` |
| パターン | `pattern/` | `PatternCoding.c` |
| Rice | `rice.rs` | `ricecoding.c` |
| ウェーブレット | `wavelet/` | `lifting_97*.c`, `CoeffGroup.c` |
| ヘッダ / ビット I/O | `header.rs`, `bitstream.rs` | `header.c`, `bitsIO.c` |

「1 ファイルに encode と対になる decode を置く」方針で整理されています（例: `dc/entropy.rs`, `stages/gaggles2.rs`）。

---

## 学習のおすすめ順

1. この文書のマップで全体像をつかむ
2. `encoder_engine` → `dc_encoding` → `ac_bpe_encoding` を上から読む
3. 1 ビットプレーンについて `block_scan_encode` と `stages_en_coding` を追う
4. デコード側で同じシンボルがどう係数に戻るかを見る
5. 必要なら `scripts/golden_test.ps1` で C 実装とのバイト一致を確認する

---

## 関連リンク

- 段階ごとのステップ解説: [steps_ja.md](steps_ja.md)
- このソースでの検証手順: [verify_ja.md](verify_ja.md)
- リポジトリ概要・ビルド手順: [README.md](../README.md)
- 互換テスト: `scripts/golden_test.ps1`

