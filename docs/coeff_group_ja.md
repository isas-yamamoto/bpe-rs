# 係数並べ替え（coeff_group）ガイド

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [画像を周波数へ](lifting97_ja.md) | **学習ストーリー** · 第 4 / 11 章 · [入口](learn_ja.md) | 次: [ビットの入れ物](header_bitstream_ja.md) → |

<!-- story-nav:end -->

## この章の結論

**サブバンドに散らばった係数を、同一場所の 8x8（DC+親・子・孫）にまとめ直します。**

用語: [用語集](glossary_ja.md)

## 絵で見る

```text
サブバンド配置          8x8 ブロック
+--+--+--+           +----+----+----+
|LL|HL|..|  regroup  | DC | P  | C  |
+--+--+--+  ------>  +----+----+----+
|LH|HH|..|           | C  | H ....  |
+--+--+--+           +--------------+
```

## 詳細

3 レベル DWT 後の **サブバンド配置** を、BPE が走査する **8x8 ブロック配置** へ組み替える処理です。
実装: [`src/wavelet/coeff_group.rs`](../src/wavelet/coeff_group.rs)。

---

## 1. なぜ必要か

DWT 直後は画像座標上に LL/HL/LH/HH が分かれています。
BPE は「同一空間位置の全周波数係数」を 1 ブロックにまとめたいので、`coeff_regroup` で並べ替えます。

| 関数 | 方向 |
|------|------|
| `coeff_regroup` / `coeff_regroup_f97` | サブバンド → ブロック |
| `coeff_degroup` / `coeff_degroup_floating` | ブロック → サブバンド |

---

## 2. 8x8 ブロック内の帰属

| バンド | ブロック内座標 | 個数 |
|--------|----------------|------|
| LL3 | `[0][0]` | 1 |
| HL3 / LH3 / HH3 | `[0][1]` / `[1][0]` / `[1][1]` | 各 1 |
| HL2 / LH2 / HH2 | 2x2 塊（例: `[0..2][2..4]` 等） | 各 4 |
| HL1 / LH1 / HH1 | 4x4 塊（例: `[0..4][4..8]` 等） | 各 16 |

合計 1+3+12+48 = 64。これが [block_scan_ja.md](block_scan_ja.md) の親・子・孫階層と一致します。

> **コラム**: この 8x8 が欠けなく作れるためにも、入力画像は 8 の倍数である必要があります。理由のまとめは [lifting97_ja.md](lifting97_ja.md) のコラムを参照してください。

---

## 3. パイプライン上の位置

- エンコード: `dwt_forward` →（スケーリング）→ `coeff_regroup` → `build_block_string`
- デコード: ブロック復元 →（必要なら degroup）→ 逆 DWT

DWT 自体の説明は [lifting97_ja.md](lifting97_ja.md)。

## 関連

- [lifting97_ja.md](lifting97_ja.md) / [block_scan_ja.md](block_scan_ja.md)

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] LL3/HL3/… がブロック内のどこに入るかを指せる
- [ ] regroup と degroup の方向が分かる
- [ ] 後のゼロツリー走査がなぜ可能かを説明できる

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [画像を周波数へ](lifting97_ja.md) | **学習ストーリー** · 第 4 / 11 章 · [入口](learn_ja.md) | 次: [ビットの入れ物](header_bitstream_ja.md) → |

<!-- story-nav:end -->
