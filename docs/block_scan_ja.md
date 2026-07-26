# ブロック走査・シンボル生成ガイド

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [共通の圧縮道具（Rice）](rice_ja.md) | **学習ストーリー** · 第 8 / 11 章 · [入口](learn_ja.md) | 次: [細かい係数を重いビットから](ac_stages_ja.md) → |

<!-- story-nav:end -->

## この章の結論

**1 プレーンで森を走査し、有意な場所だけシンボル化し、無い枝は切ります。**

用語: [用語集](glossary_ja.md)

## 絵で見る

```text
TypeP(親) -> TranB(子孫ある?)
              TranB=0 なら終了
           -> TypeCi(子) -> TypeHij(孫)
すでに有意 -> refine へ回す
```

## 詳細

この文書は、AC ビットプレーンごとに **8x8 ブロック内の係数を森状に走査し、シンボルと refine ビットを作る** 処理を説明します。
実装: [`src/block/`](../src/block/) の `block_scan_encode`（`orchestrate.rs` + 各 `type_*` / `tran_*`）。

後続の Rice 符号化は [rice_ja.md](rice_ja.md)、ステージ出力は [ac_stages_ja.md](ac_stages_ja.md) を参照してください。

---

## 1. 何をするか

各ブロックの AC 係数について、現在ビットプレーンで

1. **初めて有意になったか**（整数振幅の最上位ビットが現在プレーン）→ シンボル（有意ビット + 符号）
2. **すでに有意だったか**（過去プレーンで発見済）→ refine ビットだけ累積
3. **子孫領域に有意係数があるか**（遷移シンボル）→ 無ければ枝刈り

これはゼロツリー系（CCSDS 122 系）の「親→子→孫」走査です。

---

## 2. 8x8 内の座標

`block_int[x][y]`（第 1 添字が x）。DC は `(0,0)` で本走査の対象外。

| band | 方向 | 親 (scale=1) | 子 2x2 原点 (2) | 孫 4x4 原点 (4) |
|------|------|----------------|-------------------|-------------------|
| 0 | HL | (0,1) | (0,2) | (0,4) |
| 1 | LH | (1,0) | (2,0) | (4,0) |
| 2 | HH | (1,1) | (2,2) | (4,4) |

孫の 2x2 グループ `group=0..3` は `grand_child_origin(band, group)` で求めます。
相対オフセット: 0→(0,0), 1→(0,2), 2→(2,0), 3→(2,2)。

```text
+----+----+--------+
|DC  |P0  |  C0    |   P = TypeP (親)
|    |P1/P2| (2x2)  |   C = TypeCi (子)
+----+----+--------+
| C1 (2x2)| C2     |
|         | (2x2)  |
+---------+--------+
|   H0..H3 孫 4x4 x 3 bands ...
+------------------+
```

---

## 3. シンボル種類

| 定数 | 値 | 意味 |
|------|------|------|
| `ENUM_TYPE_P` | 1 | 親 3 係数の有意性 + 符号 |
| `ENUM_TRAN_B` | 2 | 子孫のどこかに有意があるか（1 bit） |
| `ENUM_TRAN_D` | 3 | バンドごとの子孫集合 Di への遷移 |
| `ENUM_TYPE_CI` | 4 | 子 2x2 個々の有意性 + 符号 |
| `ENUM_TRAN_GI` | 5 | 孫 4x4 全体 Gi への遷移 |
| `ENUM_TRAN_HI` | 6 | 孫内 2x2 グループ Hij への遷移 |
| `ENUM_TYPE_HIJ` | 7 | 孫 4 個の個別有意性 + 符号 |

---

## 4. `block_scan_encode` の順番

セグメント内の各ブロックで:

1. `bit_max_ac < bit_plane` なら **ブロック全体スキップ**
2. `scan_type_p` — TypeP
3. `scan_tran_b` — TranB。0 なら **残り全省略**
4. `scan_tran_d` — TranD
5. `scan_type_ci` — TypeCi
6. `scan_tran_gi` — TranGi
7. `scan_tran_hi` — TranHi
8. `scan_type_hij` — TypeHij

履歴ビット（`str_plane_hit_history`）が立っている係数はシンボルではなく refine へ回されます。

### 有意判定

- 一般: `(|v| & (1 << (bit_plane-1))) != 0`
- TypeP のみ特殊: `2^(b-1) <= |v| < 2^b`（このプレーンが振幅の最上位）

---

## 5. refine の累積

| 階層 | 関数 |
|------|------|
| 親 | `append_parent_refine` |
| 子 | `append_children_refine` |
| 孫 | `append_grand_children_refine(band)` |

実際のビット出力はステージ最後の `ref_bits_en` が行います。

---

## 6. ソース対応

| 役割 | 関数 / ファイル |
|------|------|
| 入口 | `block_scan_encode` — `orchestrate.rs` |
| 共有 | `ScanCtx`, `band_origin`, … — `common.rs` |
| 各ステージ | `type_p` / `tran_b` / `tran_d` / `type_ci` / `tran_gi` / `tran_hi` / `type_hij` |
| C 参照 | `original/source/BPEBlockCoding.c` |

---

## 7. 検証

単体のユニットテストは少ないので、パイプラインとゴールデンで見ます。

```powershell
cargo test --test pipeline_roundtrip
.\scripts\golden_test.ps1
```

## 関連

- [ac_stages_ja.md](ac_stages_ja.md) / [rice_ja.md](rice_ja.md) / [coeff_group_ja.md](coeff_group_ja.md)

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] TypeP / TranB / TypeCi / TypeHij の役割を説明できる
- [ ] TranB=0 で枝刈りする理由が分かる
- [ ] シンボルと refine の違いを言える

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [共通の圧縮道具（Rice）](rice_ja.md) | **学習ストーリー** · 第 8 / 11 章 · [入口](learn_ja.md) | 次: [細かい係数を重いビットから](ac_stages_ja.md) → |

<!-- story-nav:end -->
