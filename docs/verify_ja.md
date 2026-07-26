# BPE 検証ガイド（このソースで確かめる）

<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [途中で止まったとき](adjust_ja.md) | **学習ストーリー** · 第 11 / 11 章 · [入口](learn_ja.md) |  |

<!-- story-nav:end -->

## この章の結論

**理解はテストで確かめる。単体→ラウンドトリップ→ゴールデンの順が安全です。**

用語: [用語集](glossary_ja.md)

## 絵で見る

```text
cargo test  (単体)
    |
ラウンドトリップ  (Rust 単体)
    |
golden_test.ps1  (C とバイト一致)
```

## 詳細

この文書は、**実装が正しいこと**、および **C 参照実装と互換であること** を、このリポジトリのツールで確認する手順です。

---

## 1. 何を検証するか

| 検証項目 | 意味 | 主な手段 |
|----------|------|----------|
| ユニットテスト | 部分関数が壊れていない | `cargo test` |
| ラウンドトリップ | Rust 単体で encode->decode できる | CLI 手動 |
| **ゴールデン（バイト一致）** | Rust の `.bpe` が C と **完全同一** | `scripts/golden_test.ps1` |
| クロスデコード | C 圧縮を Rust 復号、逆も一致 | 同スクリプト |

互換性の確認で最も重要なのは **ゴールデンテスト** です。「動く」だけでなく「C と同じビット列」を目指します。

---

## 2. 用意するもの

- Rust toolchain（`cargo`）
- Windows なら PowerShell
- （ゴールデン用）`original/source/bpe.exe` がビルド済み
  - C 参照実装はリポジトリ直下の `original/source`（`bpe-rs` 単体クローンだと親ディレクトリを確認）
- Python 3（テスト用 RAW 生成に使用）

---

## 3. 手順 A: 最小のユニットテスト

```powershell
cd rust
$env:CARGO_TARGET_DIR = "$PWD\target"
cargo test
```

中身は大きく 2 種類です。

**モジュール内の単体テスト**（`src/**` の `#[cfg(test)] mod tests`）

| 場所 | 見ていること |
|------|--------------|
| `rice/` | Rice 符号表を `bit_length` 1..4 × 全オプション × 全値で往復させ、`select_rice_k` の k 選択 |
| `bitstream/` | ビット入出力の往復、パディング、セグメント上限と `rate_reached` の挙動 |
| `dc/twos_comp.rs` | 2 の補数変換の往復と不正幅の拒絶 |
| `dc/dpcm.rs` | DPCM 写像→逆写像で `shifted_dc` が戻ること |
| `dc/coding.rs` | DC/AC ビット深度の導出と量子化係数の分岐 |
| `pattern/mapping.rs` | パターン写像の全値全射性（TranD / TranHi の未使用値を除く） |
| `wavelet/lifting97i.rs` | 整数 9/7 が 1D/2D で完全可逆、平坦信号で高周波が 0 |
| `wavelet/lifting97f.rs` | 浮動小数 9/7 が許容誤差内で復元 |

**統合テスト**（`tests/`）

| ファイル | 見ていること |
|----------|--------------|
| `pipeline_roundtrip.rs` | ライブラリ API で encode -> decode。レート無制限なら可逆、複数セグメントでも可逆、レート制限でバイト上限を守ること |
| `header_bits.rs` | ヘッダの書き込み -> 読み戻し |
| `golden_roundtrip.rs` | C 二進とのバイト一致。C 実装が必要なのでデフォルト `#[ignore]` |

`#[ignore]` を含めて実行したいときは `cargo test -- --include-ignored` を使います。

---

## 4. 手順 B: Rust 単体でラウンドトリップ

「圧縮できて、復号できる」ことを最小限確認します。

```powershell
cd rust
$env:CARGO_TARGET_DIR = "$PWD\target"
cargo build --release

# 256x256 の 8bit RAW を作成（golden と同様）
New-Item -ItemType Directory -Force testdata | Out-Null
python -c "from pathlib import Path; Path('testdata/test256.raw').write_bytes(bytes((i+j)&0xFF for i in range(256) for j in range(256)))"

.\target\release\bpe.exe -e testdata\test256.raw -o testdata\out.bpe -r 1.0 -w 256 -h 256 -s 256 -t 1 -b 8
.\target\release\bpe.exe -d testdata\out.bpe -o testdata\decoded.raw
```

確認の目安:
- `-r 0` （可逆圧縮寄り）では `decoded.raw` が入力と一致しやすい
- `-r 1.0` などの損失圧縮では画像は近いがバイト一致は期待しない

### CLI パラメータ（よく使うもの）

| オプション | 意味 |
|------------|------|
| `-e` / `-d` | エンコード / デコード |
| `-o` | 出力ファイル |
| `-r` | ビット/画素（レート）。0 で可逆寄り |
| `-w` / `-h` | 幅 / 高さ |
| `-b` | 画素ビット深度（デフォルト 8） |
| `-t` | DWT の実装。`1`=整数精度の 9/7（可逆）、`0`=浮動小数精度の 9/7 |
| `-s` | 1 セグメントあたりのブロック数 |

---

## 5. 手順 C: ゴールデン互換テスト（推奨）

### 何を比べるか

スクリプト [`scripts/golden_test.ps1`](../scripts/golden_test.ps1) は次を自動実行します。

1. 同一 RAW（256x256）を C / Rust 両方でエンコード
2. `.bpe` を **バイト単位で比較**（長さも内容も一致すべき）
3. クロスデコード
   - C の `.bpe` を Rust で解凍
   - Rust の `.bpe` を C で解凍
   - 出力 RAW が C->C の復号結果と一致

### 実行

```powershell
# 事前: original/source/bpe.exe をビルドしておく
cd rust
.\scripts\golden_test.ps1
```

成功時は `ALL PASS` と、例えば次が出ます。

```
PASS: int_r0 encode identical (... bytes)
PASS: int_r0 cross decode
PASS: int_r1 encode identical (...)
...
ALL PASS
```

### ケースの意味

| 名前 | 主な引数 | 見ているもの |
|------|------------|----------------|
| `int_r0` | `-r 0 -t 1` | 整数 DWT・可逆寄り |
| `int_r1` | `-r 1.0 -t 1` | 整数 DWT・損失圧縮 |
| `float_r1` | `-r 1.0 -t 0` | 浮動小数 DWT・損失圧縮 |

生成物:
- `testdata/*_c.bpe`, `*_rust.bpe`
- `testdata/baseline/*_c.bpe`（C 出力の保存）

---

## 5.5 手順 D: CI と同じスモークテスト

GitHub Actions では C 参照実装を置けないので、バイト一致の代わりに
Rust 単体の encode -> decode を通して画素誤差を見ている。手元でも同じことができる。

```powershell
cd rust
$env:CARGO_TARGET_DIR = "$PWD\target"
cargo build --release
python scripts/ci_roundtrip.py
```

出力例:

```
PASS: int_r0 bpe=18300B mean_error=0.000 max_error=0
PASS: int_r1 bpe=8192B mean_error=0.381 max_error=5
PASS: float_r1 bpe=8192B mean_error=0.134 max_error=4

ALL PASS
```

見方:

- `int_r0`（整数 DWT・レート無制限）は **誤差 0**、すなわち可逆になる
- 損失ケースは平均・最大誤差が閾値内かを見る（大きく崩れたときだけ失敗する）
- ビット一致の保証にはならないので、改造後は必ず手順 C も実行する

---

## 6. 失敗したときの見方

| 症状 | 疑う場所の例 |
|------|----------------|
| encode の長さ不一致 | DC/AC ループ、レート制限、ヘッダ |
| encode の途中バイト不一致 | オフセット付近の段階（ヘッダ直後=DC、その後=AC depth / プレーン） |
| クロスデコードだけ失敗 | デコーダ側（`stages_de_*`, `adjust`） |
| Rust 単体では動くが C と合わない | 「動作的に近いがビット不一致」—移植バグの典型 |

アルゴリズム段階とソースの対応は [steps_ja.md](steps_ja.md) を参照してください。

---

## 7. 学習・改造時のおすすめ順

1. `cargo test` が通る
2. 手順 B で自分の入力を 1 回 encode/decode
3. `golden_test.ps1` が `ALL PASS`
4. コードを変えたら **必ず** 再度 3 を実行（ビット一致が崩れていないか）

---

## 関連リンク

- 全体地図: [algorithm_ja.md](algorithm_ja.md)
- 段階ステップ: [steps_ja.md](steps_ja.md)
- README: [../README.md](../README.md)

<!-- story-checkpoint:start -->

## この章のあとで分かること

- [ ] `cargo test` で何が保証されるかを説明できる
- [ ] ゴールデンが ALL PASS になる意味が分かる
- [ ] 失敗時にどこを見るかの目安がある

満足したら、下の「次へ」へ進んでください。

<!-- story-checkpoint:end -->
<!-- story-nav:start -->

| | | |
|---|:---:|---|
| ← 前: [途中で止まったとき](adjust_ja.md) | **学習ストーリー** · 第 11 / 11 章 · [入口](learn_ja.md) |  |

<!-- story-nav:end -->
