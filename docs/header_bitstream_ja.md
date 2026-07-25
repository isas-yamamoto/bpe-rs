# ヘッダ・ビットストリーム構造ガイド

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [8x8 の木に組む](coeff_group_ja.md) | **学習ストーリー** · 第 5 / 11 章 · [入口](learn_ja.md) | 次: [まず大まかな明るさ（DC）](dc_coding_ja.md) → |

<!-- story-nav:end -->

## この章の結論

**セグメントはヘッダ（何を送るかの説明書）とビット列で構成されます。**

用語: [用語集](glossary_ja.md)

## 絵で見る

```text
[.bpe セグメント]
  Part1 (必須)  深度・フラグ
  Part2? レート上限
  Part3? ブロック数 s
  Part4? DWT種類・幅
  --------
  DC ビット ...
  AC ビット ...
```

## 詳細

セグメント先頭のヘッダと、その後に続くビット出力の基本規則です。

---

## 1. ヘッダ Part1〜4

`header_output` / `header_readin`（[`src/header.rs`](../src/header.rs)）。

### Part1（常に存在）

| フィールド | bit | 意味 |
|------------|-----|------|
| `start_img_flag` | 1 | 画像先頭セグメント |
| `eng_img_flg` | 1 | 最後セグメント |
| `segment_count_8bits` | 8 | セグメント番号 |
| `bit_depth_dc_5bits` | 5 | DC ビット深度 |
| `bit_depth_ac_5bits` | 5 | AC ビット深度 |
| `part2/3/4_flag` | 1x3 | 後続 Part の有無 |
| `pad_rows_3bits` | 3 | （最後セグメント時）行パディング |

### Part2（レート制御）

| フィールド | 意味 |
|------------|------|
| `seg_byte_limit_27bits` | セグメントバイト上限 |
| `dc_stop` / `bit_plane_stop_5bits` / `stage_stop_2bits` | 早期停止条件 |
| `use_fill` | 上限まで 0 で埋めるか |

### Part3

| フィールド | 意味 |
|------------|------|
| `s_20bits` | セグメント内ブロック数 |
| `opt_dc_select` / `opt_ac_select` | Rice `k` の全探索 vs ヒューリスティック |

### Part4（画像パラメータ）

| フィールド | 意味 |
|------------|------|
| `dwt_type` | 0=浮動小数 9/7、1=整数 9/7 |
| `pixel_bit_depth_4bits` | 画素深度 |
| `image_width_20bits` | 幅 |
| `codeword_length_2bits` | 8/16/24/32 bit コードワード |
| `custom_wt_*` | バンドごとの重み（オプション） |

`header_update` は次セグメント用にフラグをリセットします。

---

## 2. ビットストリーム I/O

[`src/bitstream.rs`](../src/bitstream.rs)

| 関数 | 役割 |
|------|------|
| `bits_output` | MSB から 1 bit ずつ書き込み |
| `bits_read` | 同様に読み出し |
| `segment_buffer_flush_encoder` | コードワード境界へパディング、必要なら fill |

### セグメント上限

- 書き: `seg_bit_counter + length >= seg_byte_limit*8` で `segment_full = true`
- 読み: `seg_bit_counter >= decoding_allowed_bits_size_in_segment` で `rate_reached = true`

コードワード長 16/32 はリトルエンディアン書き出し（C/Windows 互換）。

## 検証

```powershell
cargo test --test header_bits
cargo test bitstream::
```

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] Part1〜4 が何を伝えるかを説明できる
- [ ] `bits_output` / `bits_read` の役割が分かる
- [ ] `segment_full` と `rate_reached` の意味を言える

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [8x8 の木に組む](coeff_group_ja.md) | **学習ストーリー** · 第 5 / 11 章 · [入口](learn_ja.md) | 次: [まず大まかな明るさ（DC）](dc_coding_ja.md) → |

<!-- story-nav:end -->
