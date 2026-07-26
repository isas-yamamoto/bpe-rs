//! Round-trip tests spanning both encode and decode.

use crate::bitstream::{bits_read, bits_write, segment_buffer_flush_encoder};
use crate::types::CodingPara;
use std::fs;
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// Write `(value, length)` pairs, flush, then read the same widths back.
fn assert_roundtrip(name: &str, items: &[(u32, i32)]) {
    let path = temp_path(name);
    let mut enc = CodingPara::new();
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    for &(value, length) in items {
        bits_write(&mut enc, value, length).unwrap();
    }
    segment_buffer_flush_encoder(&mut enc).unwrap();
    drop(enc.bits.file.take());

    let mut dec = CodingPara::new();
    dec.bits.open_read(path.to_str().unwrap()).unwrap();
    for &(value, length) in items {
        let got = bits_read(&mut dec, length as i16).unwrap();
        assert_eq!(got, value, "value={} length={}", value, length);
    }
}

#[test]
fn single_bits_roundtrip() {
    let items: Vec<(u32, i32)> = vec![(1, 1), (0, 1), (1, 1), (1, 1), (0, 1)];
    assert_roundtrip("bits_single.bin", &items);
}

#[test]
fn mixed_widths_roundtrip() {
    let items: Vec<(u32, i32)> = vec![(0x5, 3), (0xA, 4), (0x1FF, 9), (0x0, 5), (0xFFFF, 16)];
    assert_roundtrip("bits_mixed.bin", &items);
}

#[test]
fn full_word_roundtrip() {
    let items: Vec<(u32, i32)> = vec![(0xDEADBEEF, 32), (0x0, 8)];
    assert_roundtrip("bits_word.bin", &items);
}

#[test]
fn zero_length_writes_nothing() {
    let path = temp_path("bits_zero.bin");
    let mut enc = CodingPara::new();
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    bits_write(&mut enc, 0xFF, 0).unwrap();
    drop(enc.bits.file.take());
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    assert_eq!(enc.bits.total_bit_counter, 0);
}

#[test]
fn flush_pads_to_whole_codeword() {
    let path = temp_path("bits_flush.bin");
    let mut enc = CodingPara::new();
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    bits_write(&mut enc, 1, 3).unwrap();
    segment_buffer_flush_encoder(&mut enc).unwrap();
    drop(enc.bits.file.take());
    // 3 bits become one 8-bit codeword: 001 followed by five zero bits.
    assert_eq!(fs::read(&path).unwrap(), vec![0b0010_0000]);
}

#[test]
fn segment_byte_limit_marks_segment_full() {
    let path = temp_path("bits_limit.bin");
    let mut enc = CodingPara::new();
    enc.header.part2.seg_byte_limit_27bits = 1; // 8 bits allowed
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    bits_write(&mut enc, 0xF, 4).unwrap();
    assert!(!enc.segment_full);
    bits_write(&mut enc, 0xFF, 8).unwrap();
    assert!(
        enc.segment_full,
        "writing past the limit must stop the segment"
    );
    let counter_after_full = enc.bits.total_bit_counter;
    bits_write(&mut enc, 0xFF, 8).unwrap();
    assert_eq!(
        enc.bits.total_bit_counter, counter_after_full,
        "no bits may be emitted once the segment is full"
    );
}

#[test]
fn reads_stop_once_rate_is_reached() {
    let path = temp_path("bits_rate.bin");
    let mut enc = CodingPara::new();
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    for _ in 0..4 {
        bits_write(&mut enc, 0xFF, 8).unwrap();
    }
    segment_buffer_flush_encoder(&mut enc).unwrap();
    drop(enc.bits.file.take());

    let mut dec = CodingPara::new();
    dec.decoding_allowed_bits_size_in_segment = 8;
    dec.bits.open_read(path.to_str().unwrap()).unwrap();
    let _ = bits_read(&mut dec, 8).unwrap();
    assert!(
        dec.rate_reached,
        "reaching the allowed size must set rate_reached"
    );
    assert_eq!(bits_read(&mut dec, 8).unwrap(), 0);
}
