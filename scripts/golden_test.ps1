$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$repo = Split-Path $root -Parent
$testdir = Join-Path $root "testdata"
$baseline = Join-Path $testdir "baseline"
New-Item -ItemType Directory -Force -Path $testdir | Out-Null
New-Item -ItemType Directory -Force -Path $baseline | Out-Null

$raw = Join-Path $testdir "test256.raw"
if (-not (Test-Path $raw)) {
  python -c "from pathlib import Path; import sys; data=bytes((i+j)&0xFF for i in range(256) for j in range(256)); Path(sys.argv[1]).write_bytes(data)" $raw
  Write-Host "Generated test256.raw"
}

$env:CARGO_TARGET_DIR = Join-Path $root "target"
Push-Location $root
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
Pop-Location

$rust = Join-Path $root "target\release\bpe.exe"
$c = Join-Path $repo "original\source\bpe.exe"
if (-not (Test-Path $c)) { throw "Build original/source/bpe.exe first" }
if (-not (Test-Path $rust)) { throw "Rust release binary missing" }

function Assert-BytesEqual([string]$label, [byte[]]$a, [byte[]]$b) {
  if ($a.Length -ne $b.Length) {
    throw "${label}: length mismatch $($a.Length) vs $($b.Length)"
  }
  for ($i = 0; $i -lt $a.Length; $i++) {
    if ($a[$i] -ne $b[$i]) {
      throw "${label}: mismatch at offset $i (got $($a[$i]) expected $($b[$i]))"
    }
  }
}

$cases = @(
  @{ Name = "int_r0";   Args = @("-r", "0",   "-w", "256", "-h", "256", "-s", "256", "-t", "1", "-b", "8") },
  @{ Name = "int_r1";   Args = @("-r", "1.0", "-w", "256", "-h", "256", "-s", "256", "-t", "1", "-b", "8") },
  @{ Name = "float_r1"; Args = @("-r", "1.0", "-w", "256", "-h", "256", "-s", "256", "-t", "0", "-b", "8") }
)

foreach ($case in $cases) {
  $name = $case.Name
  $caseArgs = $case.Args
  $cBpe = Join-Path $testdir ($name + "_c.bpe")
  $rBpe = Join-Path $testdir ($name + "_rust.bpe")
  $cRaw = Join-Path $testdir ($name + "_c_from_c.raw")
  $rFromC = Join-Path $testdir ($name + "_rust_from_c.raw")
  $cFromR = Join-Path $testdir ($name + "_c_from_rust.raw")
  $baseBpe = Join-Path $baseline ($name + "_c.bpe")

  & $c -e $raw -o $cBpe @caseArgs
  if ($LASTEXITCODE -ne 0) { throw "${name}: C encode failed" }
  & $rust -e $raw -o $rBpe @caseArgs
  if ($LASTEXITCODE -ne 0) { throw "${name}: Rust encode failed" }

  $cBytes = [IO.File]::ReadAllBytes($cBpe)
  $rBytes = [IO.File]::ReadAllBytes($rBpe)
  Assert-BytesEqual "$name encode .bpe" $rBytes $cBytes
  [IO.File]::WriteAllBytes($baseBpe, $cBytes)
  Write-Host "PASS: $name encode identical ($($cBytes.Length) bytes)"

  & $c -d $cBpe -o $cRaw
  if ($LASTEXITCODE -ne 0) { throw "${name}: C decode of C failed" }
  & $rust -d $cBpe -o $rFromC
  if ($LASTEXITCODE -ne 0) { throw "${name}: Rust decode of C failed" }
  & $c -d $rBpe -o $cFromR
  if ($LASTEXITCODE -ne 0) { throw "${name}: C decode of Rust failed" }

  $cDec = [IO.File]::ReadAllBytes($cRaw)
  Assert-BytesEqual "$name rust-decode-of-C vs C-decode-of-C" ([IO.File]::ReadAllBytes($rFromC)) $cDec
  Assert-BytesEqual "$name C-decode-of-Rust vs C-decode-of-C" ([IO.File]::ReadAllBytes($cFromR)) $cDec
  Write-Host "PASS: $name cross decode"
}

Write-Host "ALL PASS"
