# BPE 段階別ステップ解説（学生向け）

この文書は、各アルゴリズムを **何をするか→どの順番か→どの関数か** で追えるように書いています。
全体地図は [algorithm_ja.md](algorithm_ja.md)、検証方法は [verify_ja.md](verify_ja.md) を参照してください。

---

## 0. 読み方（共通）

各章は次の形式で書いてあります。

1. **目的** — この段階が解きたい問題
2. **入力 / 出力** — 何が入って何が出るか
3. **ステップ** — 実行順
4. **ソース** — 読むべきファイル・関数
5. **対になるデコード** — 復元側の対応

---

## 1. パイプライン入口（エンコード）

### 目的
画像ファイルから `.bpe` ビットストリームを作る。

### 入力 / 出力
- 入力: RAW 画像（例: 8bit 灰色）、幅・高さ、bpp、DWT 種類など
- 出力: `.bpe` ファイル

### ステップ
1. 画像サイズを確定し、8 の倍数になるようパディング量を計算する
2. 画像を読み、端行・端列を複製して埋める
3. `dwt_forward` でウェーブレット変換
4. `build_block_string` で 8x8 ブロック列に並べ替え
5. セグメントごとに `dc_encoding` → `ac_bpe_encoding`
6. バッファを flush してファイルを確定

### ソース
- [`src/encoder.rs`](../src/encoder.rs) — `encoder_engine`, `prepare_last_segment_header`, `build_block_string`
- [`src/main.rs`](../src/main.rs) — CLI 引数解析

### 対になるデコード
- [`src/decoder.rs`](../src/decoder.rs) — `decoder_engine`

---

## 2. ウェーブレット変換（DWT）

### 目的
画像を低周波（大まかな形）と高周波（細かい変化）に分解する。BPE はこの係数を圧縮する。

### 入力 / 出力
- 入力: パディング済み画像
- 出力: 変換後の係数画像（同サイズ）

### ステップ
1. `-t 1` なら **整数 9/7**、`-t 0` なら **浮動小数 9/7** を選ぶ
2. 行方向・列方向にリフティングを適用（多段階）
3. 後でブロック単位に並べ替えやすい形にする（グルーピング関連）

### ソース
- [`src/wavelet/mod.rs`](../src/wavelet/mod.rs) — `dwt_forward` / `dwt_reverse`
- [`src/wavelet/lifting97i.rs`](../src/wavelet/lifting97i.rs) — 整数
- [`src/wavelet/lifting97f.rs`](../src/wavelet/lifting97f.rs) — 浮動小数
- [`src/wavelet/coeff_group.rs`](../src/wavelet/coeff_group.rs) — 係数の組替え

### 対になるデコード
- `dwt_reverse` / `dwt_reverse_floating`（デコード後半）

---

## 3. DC 符号化

### 目的
各 8x8 ブロックの左上（LL）係数を、セグメント単位で効率よく送る。

### 入力 / 出力
- 入力: `block_string`（変換後ブロック列）
- 出力: ヘッダ（ビット深度など）＋ DC エントロピービット、各ブロックの AC 深度情報

### ステップ
1. **統計収集** — セグメント内の DC/AC 振幅の最大・最小を調べる（`collect_segment_dc_ac_stats`）
2. **ビット深度** — DC/AC が何ビット必要かを決める（`derive_bit_depth_*`）
3. **ヘッダ出力** — 後続のデコーダが知るべきパラメータを書く
4. **量子化** — `q` を決め、DC を右シフト（`apply_dc_quantization`）。低位は残差として後で AC プレーンと一緒に出すことがある
5. **DPCM** — 隣接 DC との差分を非負整数に写像（`dpcm_dc_mapper`）
6. **エントロピー** — ガッグル単位で Rice パラメータ `k` を選び、ビット列を出力（`dc_entropy_encoder` / `select_rice_k`）

### ソース
- [`src/dc/coding.rs`](../src/dc/coding.rs) — `dc_encoding` / `dc_decoding`
- [`src/dc/dpcm.rs`](../src/dc/dpcm.rs), [`src/dc/entropy.rs`](../src/dc/entropy.rs), [`src/dc/twos_comp.rs`](../src/dc/twos_comp.rs)
- [`src/rice.rs`](../src/rice.rs) — `select_rice_k`

### 対になるデコード
- 逆順: エントロピー解読 → DPCM demap → （追加プレーン） → 逆量子化

---

## 4. AC ビットプレーンループ

### 目的
DC 以外の係数を、**MSB から LSB へ** プレーンごとに送る。

### 入力 / 出力
- 入力: ブロック係数、AC 深度、量子化残差
- 出力: AC depth ビット＋各プレーンのシンボル＋ refine ビット

### ステップ
1. **AC depth 符号化** — ブロックごとの「AC が何ビット目まで必要か」を先に送る（`ac_depth_encoder`）
2. `bit_plane = bit_depth_ac` から 1 まで減らす
3. 各プレーンで `encode_one_bitplane`:
   1. （条件を満たすとき）DC 残差のそのビットを出力
   2. `block_scan_encode` — ブロック内シンボル生成
   3. `stages_en_coding` — ガッグル跨ぎで Rice 符号化＋ refine
4. レート制限（`-r`）で `segment_full` になったら中断

### ソース
- [`src/ac/bpe.rs`](../src/ac/bpe.rs) — `ac_bpe_encoding` / `encode_one_bitplane`
- [`src/ac/depth.rs`](../src/ac/depth.rs)

### 対になるデコード
- `ac_bpe_decoding` → `decode_one_bitplane` → `stages_de_coding`

---

## 5. ブロック走査（`block_scan_encode`）

### 目的
**1 ビットプレーン・1 ブロック** について、親→子→孫の階層で「今回初めて有意になったか」をシンボル化する。

### 入力 / 出力
- 入力: `block_int[8][8]`、現在プレーン、過去の命中履歴
- 出力: `symbols_block[]`（TypeP / TranB / ...）と refine 用ビット累積

### ステップ（1 ブロック内）
1. **TypeP** — 3 つの親係数（HL3/LH3/HH3 方向）
2. **TranB** — 子孫のどこかに有意係数があるか（無ければこのブロックは終了）
3. **TranD** — 方向ごとの子孫への遷移
4. **TypeCi** — 2x2 子係数
5. **TranGi / TranHi / TypeHij** — 孫係数側
6. すでに有意だった係数は、このプレーンでは refine ビットとして別途累積

### ソース
- [`src/block.rs`](../src/block.rs) — `scan_type_p`, `scan_tran_b`, ... `block_scan_encode`

### 対になるデコード
- ステージ解読（`stages/gaggles*.rs`）がシンボルから係数位置へ値を書き戻す

---

## 6. ステージ符号化（gaggles + refine）

### 目的
ブロック走査の結果を、**シンボル種類ごと・ガッグル（最大 16 ブロック）ごと** に Rice 符号化する。

### 入力 / 出力
- 入力: 各ブロックの `symbols_block` / refine 累積
- 出力: オプションビット＋ Rice コード＋符号ビット＋ refine ビット

### ステップ
1. ガッグル範囲を決める（`gaggle_ranges`）
2. 各ガッグルで `coding_options` — パターン写像後のコストから Rice オプションを選ぶ
3. **Pass 1** `stages_en_coding_gaggles1` — TypeP だけ
4. **Pass 2** `stages_en_coding_gaggles2` — TranB / TranD / TypeCi
5. **Pass 3** `stages_en_coding_gaggles3` — TranGi / TranHi / TypeHij
6. **refine** `ref_bits_en` — 累積したリファインメントビットを出力

デコードも同順で、レート打ち切り時は停止座標を記録する。

### ソース
- [`src/stages/orchestrate.rs`](../src/stages/orchestrate.rs)
- [`src/stages/gaggles1.rs`](../src/stages/gaggles1.rs) ... `gaggles3.rs`, [`refine.rs`](../src/stages/refine.rs)
- [`src/pattern/options.rs`](../src/pattern/options.rs), [`mapping.rs`](../src/pattern/mapping.rs)
- [`src/rice.rs`](../src/rice.rs) — `rice_coding` / `rice_decoding`

---

## 7. デコード後半（補正・逆変換）

### 目的
ビット制限で途中停止した場合の係数補正と、画像への戻し。

### ステップ
1. `adjust_output` — 停止位置以降の係数を中点補正する等
2. セグメント係数を全画像座標に再配置
3. 逆 DWT
4. （必要なら）転置を戻し、RAW 出力

### ソース
- [`src/adjust.rs`](../src/adjust.rs)
- [`src/decoder.rs`](../src/decoder.rs) — `reassemble_images`, `decoding_output_*`

---

## 8. 小さな例で追うときのポイント

| 見たいもの | おすすめの見方 |
|--------------|----------------|
| 最初の入口 | `encoder_engine` を上から通読 |
| DC だけ | `dc_encoding` の呼び出し列 |
| 1 プレーン | `encode_one_bitplane` → `block_scan_encode` → `stages_en_coding` |
| シンボル種類 | `block.rs` の `scan_*` と `stages/gaggles*.rs` を並べて読む |
| デコード対称性 | 同名の `*_decoding` / `stages_de_*` |

検証の具体手順は [verify_ja.md](verify_ja.md) へ。
