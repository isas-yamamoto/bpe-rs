# 実装を読み解くガイド（コードリーディング）

学習導線: [learn_ja.md](learn_ja.md) · 前提: [steps_ja.md](steps_ja.md) · 用語: [用語集](glossary_ja.md)

## この文書の結論

**入口は `main.rs` → `encoder_engine` / `decoder_engine` の 2 本だけ。
「モジュール＝パイプラインの段」の対応で降りていけば、全ソースを迷わず読めます。**

アルゴリズム文書（何をするか）と本ガイド（どこに書いてあるか）を往復しながら読んでください。

## 絵で見る

```text
src/
  main.rs          CLI 引数 → CodingPara を組み立てる（入口）
  lib.rs           モジュール一覧（この地図の目次）
  types.rs         共有データ構造（CodingPara / BitPlaneBits / Header）
  error.rs         BpeError / BpeResult
  ---- パイプラインの段 ----
  image_io.rs      画像の読み書き（raw ↔ Vec<Vec<i32>>）
  wavelet/         DWT（lifting97i/f）＋係数並べ替え（coeff_group）
  encoder.rs       エンコード全体の指揮者
  decoder.rs       デコード全体の指揮者
  header.rs        セグメントヘッダ Part1〜4 の出力／読込
  bitstream.rs     1 ビット単位の入出力（bits_output / bits_read）
  dc/              DC 系: 統計 → 量子化 → DPCM → エントロピー
  ac/              AC 系: depth 符号化＋ビットプレーンループ
  block.rs         1 プレーン分のブロック走査（シンボル生成）
  pattern/         シンボルのパターン写像とオプション選択
  stages/          gaggles1/2/3 ＋ refine の 4 パス出力
  rice.rs          Rice 符号の本体と k 選択
  adjust.rs        途中停止時の係数補正（デコードのみ）
```

## 詳細

### 1. 読み始める場所

エンコードもデコードも、呼び出しの起点は 2 つだけです。

| 入口 | 場所 | 何をするか |
|------|------|-----------|
| `main()` | `src/main.rs` | CLI 引数を解析し `CodingPara` を組み立てる |
| `encoder_engine` | `src/encoder.rs` | エンコードの全段を順番に呼ぶ |
| `decoder_engine` | `src/decoder.rs` | デコードの全段を順番に呼ぶ |

まず `encoder.rs`（約 150 行）を **上から素読み** してください。
パイプラインの順番がコメント付きで 1 関数にまとまっており、これが読解の背骨になります。

### 2. エンコードの呼び出しツリー

```text
encoder_engine                       encoder.rs
├─ image_size / image_read           image_io.rs   サイズ決定と読込
├─ （パディング: 端の行・列を複製）   encoder.rs 内
├─ dwt_forward                       wavelet/mod.rs
│   ├─ lifting_m97_2d（整数）        wavelet/lifting97i.rs
│   ├─ lifting_f97_2d（浮動）        wavelet/lifting97f.rs
│   └─ coeff_regroup(_f97)           wavelet/coeff_group.rs
├─ build_block_string                encoder.rs   8x8 ブロック列へ並べ替え
└─ セグメントループ（block_counter < total_blocks）
    ├─ dc_encoding                   dc/coding.rs
    │   ├─ 統計・ビット深度・header_output   header.rs
    │   ├─ apply_dc_quantization（量子化 q）
    │   ├─ dpcm_dc_mapper            dc/dpcm.rs
    │   └─ dc_entropy_encoder        dc/entropy.rs（select_rice_k は rice.rs）
    ├─ ac_bpe_encoding               ac/bpe.rs
    │   ├─ ac_depth_encoder          ac/depth.rs
    │   └─ プレーンループ（重い → 軽い）
    │       ├─ block_scan_encode     block.rs     シンボル生成
    │       └─ stages_en_coding      stages/orchestrate.rs
    │           ├─ gaggles1/2/3      stages/gaggles*.rs（Rice 出力）
    │           └─ refine            stages/refine.rs
    └─ segment_buffer_flush_encoder  bitstream.rs
```

### 3. デコードの呼び出しツリー

デコードはエンコードの鏡像です。1 段だけ追加があります（`adjust_output`）。

```text
decoder_engine                       decoder.rs
├─ header_readin                     header.rs
└─ セグメントループ（eng_img_flg まで）
    ├─ dc_decoding                   dc/coding.rs
    ├─ ac_bpe_decoding               ac/bpe.rs → stages_de_coding
    ├─ adjust_output                 adjust.rs   ★デコード専用。途中停止の補正
    └─ segment_buffer_flush_decoder  bitstream.rs
（全セグメント後）
├─ reassemble_images                 decoder.rs  ブロック列 → 係数画像
├─ coeff_degroup(_floating)          wavelet/coeff_group.rs
├─ dwt_reverse(_floating)            wavelet/mod.rs
└─ image_write(_float)               image_io.rs
```

対応関係のコツ: **「encode と対になる decode は同じファイルにある」** 方針です。
例: `dc/entropy.rs` に `dc_entropy_encoder` と `dc_entropy_decoder`、
`stages/gaggles2.rs` に encode/decode の両方向。
片方を理解したら、同じファイル内の逆方向を読むと理解が固まります。

### 4. 主要データ構造（types.rs）

コードを読むとき、この 3 つを常に頭に置いてください。

#### `CodingPara` — 全段が共有する「状態かばん」

ほぼ全関数が `&mut CodingPara` を受け取ります。中身は大きく 4 種類:

| 分類 | フィールド例 | 意味 |
|------|-------------|------|
| 入出力 | `bits`, `input_file`, `coding_output_file` | ビットストリームとファイル |
| 画像情報 | `image_rows`, `image_width`, `pad_cols_3bits` | サイズとパディング |
| ヘッダ | `header`（Part1〜4） | セグメントごとの設定値 |
| 進行状態 | `block_counter`, `bit_plane`, `segment_full`, `rate_reached` | 「今どこまで処理したか」 |

`segment_full` / `rate_reached` は **レート制限の停止フラグ** で、
多くの関数が途中 return の条件に使います。読んでいて分岐が増えたらこの 2 つを疑ってください。

#### `BitPlaneBits` — ブロック 1 個分の作業領域

セグメント内の各 8x8 ブロックに 1 つずつ作られ（`block_info: Vec<BitPlaneBits>`）、
DC・AC の全段がここへ書き込みます。

| フィールド | 使う段 |
|-----------|--------|
| `shifted_dc`, `mapped_dc`, `dc_remainder` | DC（量子化 → DPCM → エントロピー） |
| `bit_max_ac`, `mapped_ac` | AC depth |
| `str_plane_hit_history`, `symbols_block` | ブロック走査（block.rs） |
| `refine_bits` | refine ステージ |
| `block_int`, `block_float` | 係数そのもの（8x8） |

#### `BlockString` — ブロック列のメモリレイアウト

`Vec<[i32; 8]>` で、**ブロック b は行 `b*8 .. b*8+8` を占有** します。
C 実装のポインタ配置をそのまま写した形なので、インデックス計算が頻出します。
`block_index * BLOCK_SIZE` が出てきたら「そのブロックの先頭行」と読み替えてください。

### 5. ビット入出力の約束（bitstream.rs）

すべての出力は最終的に `bits_output(coding, value, length)`、
読み込みは `bits_read(coding, length)` に集まります。

- 内部にワードバッファを持ち、セグメント境界で `segment_buffer_flush_*` が吐き出す
- **レート制限の検知もここ**: 出力上限に達すると `segment_full` を立てる

「どこでビットが実際に書かれるのか」と迷ったら、この 2 関数に
ブレークポイント（または `dbg!`）を置くのが最短です。

### 6. おすすめの読解順序

1. **`encoder.rs`** — 背骨。全段の呼び出し順（コメント付き）
2. **`types.rs`** — `CodingPara` と `BitPlaneBits` の項目をざっと眺める
3. **`bitstream.rs`** — `bits_output` / `bits_read` の約束を知る
4. **`dc/`** — 段数が少なく、DPCM → Rice の流れが短くまとまっている
5. **`rice.rs`** — DC で使った `select_rice_k` の中身
6. **`block.rs`** — 一番の難所。[block_scan_ja.md](block_scan_ja.md) を横に置いて読む
7. **`stages/`** — gaggles1 → 2 → 3 → refine の順にファイルが分かれている
8. **`decoder.rs` + `adjust.rs`** — 鏡像と、途中停止の補正
9. **`wavelet/`** — 独立性が高いので後回しで OK

### 7. 動かして確かめながら読む

読解と検証を交互にやると理解が速いです（詳細は [verify_ja.md](verify_ja.md)）。

```powershell
cargo test                       # 全ユニット＋結合テスト
cargo test --test header_bits    # ヘッダのビット配置だけ
cargo test rice                  # rice.rs のテストだけ
```

- 各モジュール末尾の `#[cfg(test)] mod tests` が **その関数の使用例** になっています。
  読んでいる関数のテストを先に見ると、入出力の具体像がつかめます。
- パイプライン全体は `tests/pipeline_roundtrip.rs`、
  C 実装とのバイト一致は `scripts/golden_test.ps1` で確認できます。

### 8. C 実装との対応

各ファイル冒頭のコメントに対応する C ファイルが書いてあります（例: `encoder.rs` → `bpe_encoder.c`）。
対応表は [algorithm_ja.md](algorithm_ja.md) の「ソース対応表」を参照してください。
C と見比べるときは、ポインタ演算がインデックス計算に置き換わっている点だけ注意すれば
ほぼ 1:1 で追えます。

---

## この章のあとで分かること

- [ ] `encoder_engine` の 6 ステップを言える
- [ ] `CodingPara` と `BitPlaneBits` の役割の違いを説明できる
- [ ] 「この処理はどのファイルにあるか」を地図から引ける
- [ ] encode を読んだら同じファイルの decode で答え合わせできると知っている

## 関連

- [steps_ja.md](steps_ja.md) — 各段の「何をするか」（本ガイドは「どこにあるか」）
- [block_scan_ja.md](block_scan_ja.md) / [ac_stages_ja.md](ac_stages_ja.md) — 難所の詳細
- [verify_ja.md](verify_ja.md) — テストで確かめる
- [learn_ja.md](learn_ja.md) — 学習ストーリー入口
