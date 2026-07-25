# BPE 検証ガイド（このソースで確かめる）

この文書は、**実装が正しいこと**、および **C 参照実装と互換であること** を、このリポジトリのツールで確認する手順です。

---

## 1. 何を検証するか

| 検証項目 | 意味 | 主な手段 |
|----------|------|----------|
| ユニットテスト | 部分関数が壊れていない | `cargo test` |
| ラウンドトリップ | Rust 単体で encode->decode できる | CLI 手動 |
| **ゴールデン（バイト一致）** | Rust の `.bpe` が C と **完全同一** | `scripts/golden_test.ps1` |
| クロスデコード | C 圧縮を Rust 復号、逆も一致 | 同スクリプト |

学生実験で最も重要なのは **ゴールデンテスト** です。「動く」だけでなく「C と同じビット列」を目指します。

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

例:
- `wavelet::lifting97i` の小規模 roundtrip
- `tests/header_bits.rs` のヘッダ書き込み->読み戻し

`tests/golden_roundtrip.rs` はデフォルトで `#[ignore]` です（C 二進が必要なため）。

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
| `-t` | `1`=整数 9/7、`0`=浮動小数 9/7 |
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
