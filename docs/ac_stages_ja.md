# AC ビットプレーン・ステージ符号化ガイド

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [ブロックを森状に走査](block_scan_ja.md) | **学習ストーリー** · 第 9 / 11 章 · [入口](learn_ja.md) | 次: [途中で止まったとき](adjust_ja.md) → |

<!-- story-nav:end -->

## この章の結論

**AC は整数振幅の重いビットから、シンボルをガッグル単位で Rice 出力します。**

用語: [用語集](glossary_ja.md) · [walkthrough 章 5](walkthrough_ja.md)

## 絵で見る

```text
AC depth
  -> plane = max .. 1
       走査でシンボル作成
       -> gaggles1 -> 2 -> 3 -> refine
```

## 詳細

この文書は、DC 以外の **整数係数** を、振幅の重いビットから軽いビットへ送り、ガッグル単位で Rice 符号化する流れを説明します。

> **注意**: 浮動小数 DWT（`-t 0`）でも、ここの対象は丸め後の整数振幅です。
> IEEE 754 のビット列を上位から送っているわけではありません（[algorithm_ja.md](algorithm_ja.md) 参照）。

DC（大まかな明るさ）の送り方は [dc_coding_ja.md](dc_coding_ja.md) を見てください。

---

## 0. AC とは何か（概念）

### DC との対比

| | DC | AC |
|--|----|----|
| 何を表すか | その場所の **平均的な明るさ** | **細かい変化**（エッジ・テクスチャ） |
| 個数 / ブロック | 1（`[0][0]`） | 63（親・子・孫） |
| 送り方 | 量子化 → DPCM → Rice（まとめて先に） | **ビットプレーン**（重いビットから） |
| 復号直後の効果 | ぼやけた画がすぐ見える | 輪郭・細部が段階的に立つ |

名前の由来（直流／交流の比喩）は [dc_coding_ja.md の概念節](dc_coding_ja.md) と同じです。

### なぜビットプレーンで送るか

AC 係数は多くが **ほぼ 0** で、非零でも振幅は小さいことが多いです。
一気に 63 個の生値を書くより、次の方が効きます。

1. **振幅の重いビットから** 送る → 画質への寄与が大きい情報が先に届く
2. **森状に走査**し、「この枝に有意なし」なら子孫を読まない（枝刈り）
3. すでに有意だった係数だけ、軽いビットで **refine**（精度上げ）

```text
|v| = 13 = 1101₂
  プレーン 4: 1  ← 初めて有意 → 符号(+/-)も送る
  プレーン 3: 1  ← refine
  プレーン 2: 0  ← refine
  プレーン 1: 1  ← refine
```

数値例は [walkthrough 章 5](walkthrough_ja.md)。走査シンボルは [block_scan_ja.md](block_scan_ja.md)。

### DC 残差とのつながり

DC 量子化で切り捨てた低位（`dc_remainder`）は、条件を満たすとき **AC の各プレーンと同じビット位** で一緒に送られます。
「DC を先に粗く送り、細部ビットは AC ループに乗せる」という一本のパイプラインです。

---

## 1. 全体流れ

```mermaid
flowchart TD
  ac[ac_bpe_encoding] --> depth[ac_depth_encoder]
  depth --> loop[bit_plane = depth .. 1]
  loop --> one[encode_one_bitplane]
  one --> dcr[DC remainder bits 条件付]
  one --> scan[block_scan_encode]
  one --> st[stages_en_coding]
  st --> g1[gaggles1 TypeP]
  st --> g2[gaggles2 TranB/D/Ci]
  st --> g3[gaggles3 TranGi/Hi/Hij]
  st --> ref[ref_bits_en]
```

`bit_depth_ac == 0` なら AC 符号化自体を省略します。

---

## 2. AC depth

プレーンループの前に、各ブロックの `bit_max_ac`（AC が何ビット目まで必要か）を送ります。

- depth=1: 1 bit/ブロック
- depth>1: DPCM 写像（`dpcm_ac_mapper`）→ ガッグル Rice（`select_rice_k`）

デコーダは `bit_max_ac < bit_plane` のブロックをスキップできます。

実装: [`src/ac/depth.rs`](../src/ac/depth.rs)

---

## 3. 1 プレーン: `encode_one_bitplane`

1. （条件満たすとき）DC 残差 `dc_remainder` の当該ビットを出力
2. `block_scan_encode` — シンボル / refine 累積
3. `stages_en_coding` — Rice + 符号 + refine 出力

`segment_full` になったら中断します。

---

## 4. ステージの 4 パス

`stages_en_coding`（[`orchestrate.rs`](../src/stages/orchestrate.rs)）は **ステージ跨ぎ** で全ガッグルを回します。

| 順 | 関数 | 内容 |
|------|------|------|
| A | `coding_options` + `stages_en_coding_gaggles1` | TypeP |
| B | `stages_en_coding_gaggles2` | TranB / TranD / TypeCi |
| C | `stages_en_coding_gaggles3` | TranGi / TranHi / TypeHij |
| D | `ref_bits_en` | 累積 refine ビット |

ガッグルサイズは `GAGGLE_SIZE = 16`。範囲分割は `gaggle_ranges`。

TypeP / TypeCi / TypeHij だけ符号ビットを伴い、遷移シンボルは Rice のみです。

---

## 5. デコード

| エンコード | デコード |
|------------|------------|
| `ac_bpe_encoding` | `ac_bpe_decoding` |
| `block_scan_encode` | （なし。ステージが係数へ直接書き戻す） |
| `stages_en_coding` | `stages_de_coding` |

レート制限では `rate_reached` 時に `stopped_stage` と停止座標（ブロック / x,y）を記録し、
後で `adjust_output`（[adjust_ja.md](adjust_ja.md)）が未確定係数を補正します。

---

## 6. ソース対応

| 役割 | 場所 |
|------|------|
| AC 入口 | `src/ac/bpe.rs` |
| AC depth | `src/ac/depth.rs` |
| オーケストラ | `src/stages/orchestrate.rs` |
| gaggles1..3 / refine | `src/stages/gaggles*.rs`, `refine.rs` |
| 共通 | `src/stages/common.rs` |

## 関連

- [block_scan_ja.md](block_scan_ja.md) / [rice_ja.md](rice_ja.md) / [dc_coding_ja.md](dc_coding_ja.md) / [adjust_ja.md](adjust_ja.md)

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] AC が「細部」であり、なぜ重いビットから送るかを説明できる
- [ ] AC depth が何のためかを説明できる
- [ ] gaggles1→2→3→refine の順を言える
- [ ] レート制限で停止情報が残ることを知っている

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [ブロックを森状に走査](block_scan_ja.md) | **学習ストーリー** · 第 9 / 11 章 · [入口](learn_ja.md) | 次: [途中で止まったとき](adjust_ja.md) → |

<!-- story-nav:end -->
