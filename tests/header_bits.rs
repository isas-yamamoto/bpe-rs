use bpe_rs::bitstream;
use bpe_rs::header::{header_output, header_readin};
use bpe_rs::types::CodingPara;
use std::fs;
use std::path::PathBuf;

#[test]
fn header_roundtrip_default() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("header_only.bin");
    let mut enc = CodingPara::new();
    enc.header.part1.start_img_flag = true;
    enc.header.part1.eng_img_flg = false;
    enc.header.part1.part2_flag = true;
    enc.header.part1.part3_flag = true;
    enc.header.part1.part4_flag = true;
    enc.header.part4.image_width_20bits = 256;
    enc.header.part3.s_20bits = 256;
    enc.bits.open_write(path.to_str().unwrap()).unwrap();
    header_output(&mut enc).unwrap();
    if enc.bits.code_word_alignment_bits != 0 {
        let shift = enc.bits.code_word_length as i32 - enc.bits.code_word_alignment_bits as i32;
        bitstream::bits_output(&mut enc, 0, shift).unwrap();
    }
    drop(enc.bits.file.take());
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        bytes.len(),
        19,
        "expected 19-byte header, got {}",
        bytes.len()
    );
    let mut dec = CodingPara::new();
    dec.bits.open_read(path.to_str().unwrap()).unwrap();
    header_readin(&mut dec).unwrap();
    assert!(dec.header.part1.start_img_flag);
    assert!(!dec.header.part1.eng_img_flg);
    assert_eq!(dec.header.part4.image_width_20bits, 256);
    assert_eq!(dec.header.part3.s_20bits, 256);
}
