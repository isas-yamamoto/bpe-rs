//! End-to-end pipeline tests driving the library API the same way the CLI does.

use bpe_rs::decoder::decoder_engine;
use bpe_rs::encoder::encoder_engine;
use bpe_rs::types::{CodingPara, FLOAT_WAVELET, INTEGER_WAVELET};
use std::fs;
use std::path::{Path, PathBuf};

const SIZE: u32 = 64;

fn testdata_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Smooth gradient with a bright square, so both DC and AC bands carry data.
fn write_test_image(path: &Path) -> Vec<u8> {
    let mut data = Vec::with_capacity((SIZE * SIZE) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let base = ((x * 3 + y * 5) % 200) as u8;
            let value = if (16..32).contains(&x) && (16..32).contains(&y) {
                base.saturating_add(55)
            } else {
                base
            };
            data.push(value);
        }
    }
    fs::write(path, &data).unwrap();
    data
}

fn encode(
    name: &str,
    blocks_per_segment: u32,
    bits_per_pixel: f32,
    dwt_type: u8,
) -> (Vec<u8>, Vec<u8>) {
    let dir = testdata_dir();
    let raw = dir.join(format!("e2e_{}.raw", name));
    let bpe = dir.join(format!("e2e_{}.bpe", name));
    let original = write_test_image(&raw);

    let mut coding = CodingPara::new();
    coding.input_file = raw.to_str().unwrap().to_string();
    coding.coding_output_file = bpe.to_str().unwrap().to_string();
    coding.bits_per_pixel = bits_per_pixel;
    coding.image_rows = SIZE;
    coding.image_width = SIZE;
    coding.header.part4.dwt_type = dwt_type;
    coding.header.part3.s_20bits = blocks_per_segment;
    if bits_per_pixel != 0.0 {
        coding.header.part2.seg_byte_limit_27bits =
            (bits_per_pixel * blocks_per_segment as f32 * 64.0 / 8.0) as u32;
    }
    encoder_engine(&mut coding).unwrap();

    (original, fs::read(&bpe).unwrap())
}

fn decode(name: &str) -> Vec<u8> {
    let dir = testdata_dir();
    let bpe = dir.join(format!("e2e_{}.bpe", name));
    let out = dir.join(format!("e2e_{}_decoded.raw", name));

    let mut coding = CodingPara::new();
    coding.input_file = bpe.to_str().unwrap().to_string();
    coding.coding_output_file = out.to_str().unwrap().to_string();
    decoder_engine(&mut coding).unwrap();

    fs::read(&out).unwrap()
}

fn mean_absolute_error(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    let total: u32 = a
        .iter()
        .zip(b.iter())
        .map(|(&p, &q)| (p as i32 - q as i32).unsigned_abs())
        .sum();
    total as f64 / a.len() as f64
}

#[test]
fn integer_wavelet_without_rate_limit_is_lossless() {
    let (original, stream) = encode("int_lossless", 64, 0.0, INTEGER_WAVELET);
    assert!(!stream.is_empty());
    let decoded = decode("int_lossless");
    assert_eq!(
        decoded, original,
        "unbounded integer coding must be lossless"
    );
}

#[test]
fn several_segments_round_trip() {
    // 64x64 is 64 blocks, so 16 blocks per segment means four segments.
    let (original, stream) = encode("int_segments", 16, 0.0, INTEGER_WAVELET);
    assert!(!stream.is_empty());
    let decoded = decode("int_segments");
    assert_eq!(
        decoded, original,
        "segment boundaries must not corrupt data"
    );
}

#[test]
fn rate_limited_stream_honours_the_byte_budget() {
    let bits_per_pixel = 2.0;
    let blocks_per_segment = 64u32;
    let (original, stream) = encode(
        "int_rate",
        blocks_per_segment,
        bits_per_pixel,
        INTEGER_WAVELET,
    );

    let budget = (bits_per_pixel * blocks_per_segment as f32 * 64.0 / 8.0) as usize;
    assert!(
        stream.len() <= budget,
        "stream of {} bytes exceeds the {} byte budget",
        stream.len(),
        budget
    );

    let decoded = decode("int_rate");
    assert_eq!(decoded.len(), original.len());
    let error = mean_absolute_error(&original, &decoded);
    assert!(error < 4.0, "mean absolute error too large: {}", error);
}

#[test]
fn float_wavelet_round_trip_stays_close() {
    let (original, stream) = encode("float_rate", 64, 2.0, FLOAT_WAVELET);
    assert!(!stream.is_empty());
    let decoded = decode("float_rate");
    assert_eq!(decoded.len(), original.len());
    let error = mean_absolute_error(&original, &decoded);
    assert!(error < 4.0, "mean absolute error too large: {}", error);
}

#[test]
fn lower_rate_produces_a_smaller_stream() {
    let (_, small) = encode("rate_1bpp", 64, 1.0, INTEGER_WAVELET);
    let (_, large) = encode("rate_4bpp", 64, 4.0, INTEGER_WAVELET);
    assert!(
        small.len() < large.len(),
        "1 bpp ({} bytes) should be smaller than 4 bpp ({} bytes)",
        small.len(),
        large.len()
    );
}
