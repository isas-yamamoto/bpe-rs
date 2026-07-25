#!/usr/bin/env python3
"""CI 用のラウンドトリップ検証。

C 参照実装 (original/source) はこのリポジトリに含まれないため、
CI ではバイト一致のゴールデンテスト (scripts/golden_test.ps1) を実行できない。
代わりに Rust 単体で encode -> decode を通し、出力サイズと画素誤差を確認する。
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WIDTH = 256
HEIGHT = 256
PIXEL_BITS = 8
BLOCKS_PER_SEGMENT = 256

# (ケース名, レート -r, DWT 種別 -t, 許容する平均誤差, 許容する最大誤差)
CASES = [
    ("int_r0", "0", "1", 0.0, 0),
    ("int_r1", "1.0", "1", 2.0, 32),
    ("float_r1", "1.0", "0", 2.0, 32),
]


def release_binary():
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR") or ROOT / "target")
    for name in ("bpe.exe", "bpe"):
        candidate = target_dir / "release" / name
        if candidate.is_file():
            return candidate
    raise SystemExit("release binary not found under {}".format(target_dir / "release"))


def make_test_image(path):
    data = bytes(((x * 3 + y * 5) // 2) & 0xFF for y in range(HEIGHT) for x in range(WIDTH))
    path.write_bytes(data)
    return data


def run(binary, args):
    result = subprocess.run([str(binary)] + args, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit("command failed ({}): {}".format(result.returncode, " ".join(args)))


def error_stats(original, decoded):
    total = 0
    worst = 0
    for a, b in zip(original, decoded):
        diff = abs(a - b)
        total += diff
        if diff > worst:
            worst = diff
    return total / len(original), worst


def main():
    binary = release_binary()
    workdir = ROOT / "testdata"
    workdir.mkdir(exist_ok=True)

    raw_path = workdir / "ci_input.raw"
    original = make_test_image(raw_path)

    failures = []
    for name, rate, wavelet, mean_limit, max_limit in CASES:
        bpe_path = workdir / "ci_{}.bpe".format(name)
        decoded_path = workdir / "ci_{}_decoded.raw".format(name)

        run(
            binary,
            [
                "-e", str(raw_path),
                "-o", str(bpe_path),
                "-r", rate,
                "-w", str(WIDTH),
                "-h", str(HEIGHT),
                "-b", str(PIXEL_BITS),
                "-t", wavelet,
                "-s", str(BLOCKS_PER_SEGMENT),
            ],
        )
        run(binary, ["-d", str(bpe_path), "-o", str(decoded_path)])

        compressed = bpe_path.stat().st_size
        decoded = decoded_path.read_bytes()

        if compressed == 0:
            failures.append("{}: compressed stream is empty".format(name))
            continue
        if len(decoded) != len(original):
            failures.append(
                "{}: decoded size {} != {}".format(name, len(decoded), len(original))
            )
            continue

        mean_error, max_error = error_stats(original, decoded)
        status = "PASS"
        if mean_error > mean_limit or max_error > max_limit:
            status = "FAIL"
            failures.append(
                "{}: mean {:.3f} (limit {}), max {} (limit {})".format(
                    name, mean_error, mean_limit, max_error, max_limit
                )
            )
        print(
            "{}: {} bpe={}B mean_error={:.3f} max_error={}".format(
                status, name, compressed, mean_error, max_error
            )
        )

    if failures:
        print("")
        print("FAILURES:")
        for failure in failures:
            print("  - " + failure)
        return 1

    print("")
    print("ALL PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
