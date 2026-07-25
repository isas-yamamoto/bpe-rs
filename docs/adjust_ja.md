# 係数補正（adjust_output）ガイド

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [細かい係数を重いビットから](ac_stages_ja.md) | **学習ストーリー** · 第 10 / 11 章 · [入口](learn_ja.md) | 次: [手を動かして確かめる](verify_ja.md) → |

<!-- story-nav:end -->

## この章の結論

**ビットが足りないとき、未読み係数を不確定区間の中点へ寄せて画質の崩れを抑えます。**

用語: [用語集](glossary_ja.md)

## 絵で見る

```text
復号途中停止
  読み済み係数: そのまま
  未読み係数: 中点へ補正 (adjust)
```

## 詳細

レート制限で復号が途中停止したとき、**未確定の係数を不確定区間の中点へ寄せる** 復号後処理です。
実装: [`src/adjust.rs`](../src/adjust.rs) の `adjust_output`。

---

## 1. いつ動くか

- **復号側のみ**
- `rate_reached` が真で、`StopLocation` が有効（ブロック番号とプレーンが記録済）のとき

可逆圧縮（`-r 0`）では通常呼ばれません。

---

## 2. 入口処理

1. （浮動小数 DWT）`block_float <- block_int`
2. DC を `shifted_dc + decoding_dc_remainder` から復元
3. 停止情報から `beta_1` / `beta_2`（中点補正量）を計算
4. `stopped_stage` に応じて `stage1`〜`stage4` を呼ぶ（`dispatch_stage`）

---

## 3. ステージの意味

停止が発生したのは、エンコード側のステージと同じ番号です。

| `stopped_stage` | 停止場所 | 補正の考え方 |
|-----------------|----------|----------------|
| 1 | TypeP（親）走査中 | 子孫は未読み→ `beta_2` 依存 |
| 2 | TypeCi（子）走査中 | 孫は未読み、親は refine 済み得る |
| 3 | TypeHij（孫）走査中 | 走査順を再現し前/後で分岐 |
| 4 | refine 読み中 | 停止前はより細い補正、後は粗い補正 |

各ステージはブロックを「停止以前 / 停止ブロック / 以降」に分け、
停止ブロック内では `(x,y)` で走査済/未済を判定します。

`bump` が現在値の符号方向へ補正量を加算します。

---

## 4. ソース

| 関数 | 役割 |
|------|------|
| `adjust_output` | 入口 |
| `dispatch_stage` | stage 選択 |
| `stage1`〜`stage4` | 各停止階層 |
| `refine_amount` / `bump` | 補正量の決定・適用 |

## 関連

- [ac_stages_ja.md](ac_stages_ja.md) / [block_scan_ja.md](block_scan_ja.md)

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] `stopped_stage` 1〜4 が何を意味するかが分かる
- [ ] 未読み係数を中点へ寄せる理由を説明できる
- [ ] 復号側だけで動くことを知っている

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [細かい係数を重いビットから](ac_stages_ja.md) | **学習ストーリー** · 第 10 / 11 章 · [入口](learn_ja.md) | 次: [手を動かして確かめる](verify_ja.md) → |

<!-- story-nav:end -->
