//! Rice coding — original/source/ricecoding.c
//!
//! `option` corresponds to the C `UCHAR8 *Option` / `splitOption` array indexed
//! as `option[0]` (2-bit codewords), `option[1]` (3-bit codewords) and
//! `option[2]` (4-bit codewords).

use crate::bitstream::{bits_output, bits_read};
use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

pub fn rice_coding(
    coding: &mut CodingPara,
    input_val: u32,
    bit_length: u8,
    option: &[u8; 3],
) -> BpeResult<()> {
    match bit_length {
        0 => Ok(()),
        1 => bits_output(coding, input_val, 1),
        2 => {
            if option[0] == 1 {
                bits_output(coding, input_val, 2)
            } else if option[0] == 0 {
                if input_val <= 2 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else {
                    bits_output(coding, 0, 3)
                }
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        3 => {
            if option[1] == 0 {
                if input_val <= 2 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else if input_val <= 5 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val - 3, 2)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val, 3)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 1 {
                if input_val <= 1 {
                    bits_output(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_output(coding, input_val, 3)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 2)?;
                    match input_val {
                        4 => bits_output(coding, 2, 2),
                        5 => bits_output(coding, 3, 2),
                        6 => bits_output(coding, 0, 2),
                        7 => bits_output(coding, 1, 2),
                        _ => unreachable!(),
                    }
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 3 {
                bits_output(coding, input_val, 3)
            } else {
                Ok(())
            }
        }
        4 => {
            if option[2] == 0 {
                if input_val <= 3 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 5)?;
                    bits_output(coding, input_val - 4, 2)
                } else if input_val <= 15 {
                    bits_output(coding, 0, 4)?;
                    bits_output(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 1 {
                if input_val <= 1 {
                    bits_output(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_output(coding, input_val, 3)
                } else if input_val <= 5 {
                    bits_output(coding, 0, 2)?;
                    bits_output(coding, input_val - 2, 2)
                } else if input_val <= 11 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val - 6, 3)
                } else if input_val <= 15 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 2 {
                if input_val <= 3 {
                    bits_output(coding, input_val + 4, 3)
                } else if input_val <= 7 {
                    bits_output(coding, input_val, 4)
                } else if input_val <= 11 {
                    bits_output(coding, 0, 2)?;
                    bits_output(coding, input_val - 4, 3)
                } else if input_val <= 15 {
                    bits_output(coding, input_val - 12, 5)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 3 {
                bits_output(coding, input_val, 4)
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        _ => Err(BpeError::RiceCodingError),
    }
}

/// Faithfully mirrors the C control flow: after every `BitsRead` call, if
/// `coding.rate_reached` became true the decoded value is forced to 0 and
/// decoding stops early (matching the many `*decoded = 0; return;` sites).
pub fn rice_decoding(
    coding: &mut CodingPara,
    bit_length: i16,
    split_option: &[u8; 3],
) -> BpeResult<u32> {
    match bit_length {
        0 => Ok(0),
        1 => {
            let word = bits_read(coding, 1)?;
            Ok(word)
        }
        2 => match split_option[0] {
            1 => {
                let word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            0 => {
                let mut i: u32 = 0;
                while i < 3 {
                    let word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                Ok(i)
            }
            _ => Err(BpeError::RiceCodingError),
        },
        3 => match split_option[1] {
            3 => {
                let word = bits_read(coding, 3)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            1 => {
                let mut word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if (word & 0x2) > 0 {
                    if (word & 0x1) == 0 {
                        Ok(0)
                    } else {
                        Ok(1)
                    }
                } else if (word & 0x1) > 0 {
                    word = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word > 0 {
                        Ok(3)
                    } else {
                        Ok(2)
                    }
                } else {
                    word = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match word {
                        0x2 => Ok(4),
                        0x3 => Ok(5),
                        0x0 => Ok(6),
                        0x1 => Ok(7),
                        _ => unreachable!(),
                    }
                }
            }
            0 => {
                let mut word_readin: u32 = 0;
                let mut i: u32 = 0;
                while i < 3 {
                    word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                if word_readin == 1 {
                    Ok(i)
                } else {
                    let word = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word != 3 {
                        Ok(word + 3)
                    } else {
                        let word2 = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if word2 == 0 {
                            Ok(6)
                        } else {
                            Ok(7)
                        }
                    }
                }
            }
            _ => Err(BpeError::RiceCodingError),
        },
        4 => match split_option[2] {
            3 => {
                let word = bits_read(coding, 4)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            2 => {
                let word = bits_read(coding, 3)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if (word & 0x4) > 0 {
                    match word & 0x3 {
                        0x0 => Ok(0),
                        0x1 => Ok(1),
                        0x2 => Ok(2),
                        0x3 => Ok(3),
                        _ => unreachable!(),
                    }
                } else if (word & 0x2) > 0 {
                    if (word & 0x1) == 0 {
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w == 0 {
                            Ok(4)
                        } else {
                            Ok(5)
                        }
                    } else {
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w == 0 {
                            Ok(6)
                        } else {
                            Ok(7)
                        }
                    }
                } else if (word & 0x1) == 1 {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match w {
                        0x0 => Ok(8),
                        0x1 => Ok(9),
                        0x2 => Ok(10),
                        0x3 => Ok(11),
                        _ => unreachable!(),
                    }
                } else {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match w {
                        0x0 => Ok(12),
                        0x1 => Ok(13),
                        0x2 => Ok(14),
                        0x3 => Ok(15),
                        _ => unreachable!(),
                    }
                }
            }
            1 => {
                let word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if word >= 2 {
                    if word == 2 {
                        Ok(0)
                    } else {
                        Ok(1)
                    }
                } else if (word & 1) == 1 {
                    let w = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if w == 0 {
                        Ok(2)
                    } else {
                        Ok(3)
                    }
                } else {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if (w & 0x2) > 0 {
                        if (w & 0x1) == 0 {
                            Ok(4)
                        } else {
                            Ok(5)
                        }
                    } else if (w & 0x1) == 0 {
                        let w2 = bits_read(coding, 2)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        match w2 {
                            0x0 => Ok(6),
                            0x1 => Ok(7),
                            0x2 => Ok(8),
                            0x3 => Ok(9),
                            _ => unreachable!(),
                        }
                    } else {
                        let w2 = bits_read(coding, 2)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w2 == 0 {
                            Ok(10)
                        } else if w2 == 1 {
                            Ok(11)
                        } else if w2 == 2 {
                            let w3 = bits_read(coding, 1)?;
                            if coding.rate_reached {
                                return Ok(0);
                            }
                            if w3 == 0 {
                                Ok(12)
                            } else {
                                Ok(13)
                            }
                        } else {
                            let w3 = bits_read(coding, 1)?;
                            if coding.rate_reached {
                                return Ok(0);
                            }
                            if w3 == 0 {
                                Ok(14)
                            } else {
                                Ok(15)
                            }
                        }
                    }
                }
            }
            0 => {
                let mut i: u32 = 0;
                while i < 4 {
                    let word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                if i != 4 {
                    Ok(i)
                } else {
                    let word = bits_read(coding, 3)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if (word & 0x4) == 0 {
                        match word {
                            0x0 => Ok(4),
                            0x1 => Ok(5),
                            0x2 => Ok(6),
                            0x3 => Ok(7),
                            _ => unreachable!(),
                        }
                    } else {
                        let mut decoded = word;
                        decoded <<= 1;
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        decoded += w;
                        Ok(decoded)
                    }
                }
            }
            _ => Err(BpeError::RiceCodingError),
        },
        _ => Err(BpeError::RiceCodingError),
    }
}

/// Sentinel k value meaning "emit uncoded words for this gaggle".
pub(crate) const UNCODED_FLAG: i32 = 0xFF;

// Heuristic thresholds matching the original C DC/AC gaggle k selection.
const HEUR_UNCODED_DELTA_MUL: i64 = 64;
const HEUR_UNCODED_J_MUL: i64 = 23;
const HEUR_K0_J_MUL: i64 = 207;
const HEUR_K0_DELTA_MUL: i64 = 128;
const HEUR_LARGE_K_SHIFT: i64 = 5;
const HEUR_LARGE_K_DELTA_MUL: i64 = 128;
const HEUR_LARGE_K_J_MUL: i64 = 49;
const HEUR_SCAN_SHIFT_BASE: i64 = 7;
const HEUR_SCAN_DELTA_MUL: i64 = 128;
const HEUR_SCAN_J_MUL: i64 = 49;

/// Choose Rice parameter `k` for one gaggle of mapped magnitudes.
///
/// `mapped[i]` is the mapped DC or AC value for block `start_index + i` style
/// absolute indices via the slice covering `[start_index, start_index+gaggles)`.
/// `opt_select` mirrors `header.part3.opt_dc_select` (exhaustive search vs heuristic).
pub(crate) fn select_rice_k(
    mapped: &[u32],
    start_index: usize,
    gaggles: usize,
    n: u8,
    max_k: i32,
    opt_select: bool,
) -> i32 {
    if opt_select {
        let mut min_k = UNCODED_FLAG;
        let mut min_bits: u32 = 0xFFFF;
        for k in 0..=max_k {
            let mut total_bits: u32 = if start_index == 0 { n as u32 } else { 0 };
            for i in start_index.max(1)..(start_index + gaggles) {
                total_bits += ((mapped[i] >> k) + 1) + k as u32;
            }
            if (total_bits < min_bits) && (total_bits < n as u32 * gaggles as u32) {
                min_bits = total_bits;
                min_k = k;
            }
        }
        min_k
    } else {
        let mut delta: i64 = 0;
        let mut j = gaggles as i64;
        if start_index == 0 {
            j = gaggles as i64 - 1;
        }
        for i in start_index..(start_index + gaggles) {
            delta += mapped[i] as i64;
        }
        if HEUR_UNCODED_DELTA_MUL * delta >= HEUR_UNCODED_J_MUL * j * (1i64 << n) {
            UNCODED_FLAG
        } else if HEUR_K0_J_MUL * j > HEUR_K0_DELTA_MUL * delta {
            0
        } else if j * (1i64 << (n as i64 + HEUR_LARGE_K_SHIFT))
            <= HEUR_LARGE_K_DELTA_MUL * delta + HEUR_LARGE_K_J_MUL * j
        {
            n as i32 - 2
        } else {
            let mut min_k = 0;
            while j * (1i64 << (min_k as i64 + HEUR_SCAN_SHIFT_BASE))
                <= HEUR_SCAN_DELTA_MUL * delta + HEUR_SCAN_J_MUL * j
            {
                min_k += 1;
            }
            min_k - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitstream::segment_buffer_flush_encoder;
    use crate::types::GAGGLE_SIZE;
    use std::fs;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// Encode every value of `values` in sequence, then decode the same sequence
    /// back and require an exact match. This pins the Rice codeword tables.
    fn assert_roundtrip(name: &str, bit_length: u8, option: [u8; 3], values: &[u32]) {
        let path = temp_path(name);
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        for &value in values {
            rice_coding(&mut enc, value, bit_length, &option).unwrap();
        }
        segment_buffer_flush_encoder(&mut enc).unwrap();
        drop(enc.bits.file.take());

        let mut dec = CodingPara::new();
        dec.bits.open_read(path.to_str().unwrap()).unwrap();
        for &value in values {
            let decoded = rice_decoding(&mut dec, bit_length as i16, &option).unwrap();
            assert_eq!(
                decoded, value,
                "bit_length={} option={:?} value={}",
                bit_length, option, value
            );
        }
    }

    #[test]
    fn roundtrip_length1() {
        assert_roundtrip("rice_len1.bin", 1, [0, 0, 0], &[0, 1, 1, 0, 1]);
    }

    #[test]
    fn roundtrip_length2_all_options() {
        let values: Vec<u32> = (0..=3).collect();
        assert_roundtrip("rice_len2_opt0.bin", 2, [0, 0, 0], &values);
        assert_roundtrip("rice_len2_opt1.bin", 2, [1, 0, 0], &values);
    }

    #[test]
    fn roundtrip_length3_all_options() {
        let values: Vec<u32> = (0..=7).collect();
        assert_roundtrip("rice_len3_opt0.bin", 3, [0, 0, 0], &values);
        assert_roundtrip("rice_len3_opt1.bin", 3, [0, 1, 0], &values);
        assert_roundtrip("rice_len3_opt3.bin", 3, [0, 3, 0], &values);
    }

    #[test]
    fn roundtrip_length4_all_options() {
        let values: Vec<u32> = (0..=15).collect();
        assert_roundtrip("rice_len4_opt0.bin", 4, [0, 0, 0], &values);
        assert_roundtrip("rice_len4_opt1.bin", 4, [0, 0, 1], &values);
        assert_roundtrip("rice_len4_opt2.bin", 4, [0, 0, 2], &values);
        assert_roundtrip("rice_len4_opt3.bin", 4, [0, 0, 3], &values);
    }

    #[test]
    fn length_zero_emits_nothing() {
        let path = temp_path("rice_len0.bin");
        let mut enc = CodingPara::new();
        enc.bits.open_write(path.to_str().unwrap()).unwrap();
        rice_coding(&mut enc, 5, 0, &[0, 0, 0]).unwrap();
        drop(enc.bits.file.take());
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);

        let mut dec = CodingPara::new();
        dec.bits.open_read(path.to_str().unwrap()).unwrap();
        assert_eq!(rice_decoding(&mut dec, 0, &[0, 0, 0]).unwrap(), 0);
    }

    #[test]
    fn unsupported_bit_length_is_rejected() {
        let path = temp_path("rice_bad_len.bin");
        let mut coding = CodingPara::new();
        coding.bits.open_write(path.to_str().unwrap()).unwrap();
        assert!(rice_coding(&mut coding, 0, 5, &[0, 0, 0]).is_err());
        assert!(rice_decoding(&mut coding, 5, &[0, 0, 0]).is_err());
    }

    #[test]
    fn unsupported_option_is_rejected() {
        let path = temp_path("rice_bad_opt.bin");
        let mut coding = CodingPara::new();
        coding.bits.open_write(path.to_str().unwrap()).unwrap();
        assert!(rice_coding(&mut coding, 0, 2, &[2, 0, 0]).is_err());
        assert!(rice_coding(&mut coding, 0, 4, &[0, 0, 4]).is_err());
    }

    #[test]
    fn out_of_range_value_is_rejected() {
        let path = temp_path("rice_out_of_range.bin");
        let mut coding = CodingPara::new();
        coding.bits.open_write(path.to_str().unwrap()).unwrap();
        assert!(rice_coding(&mut coding, 8, 3, &[0, 0, 0]).is_err());
        assert!(rice_coding(&mut coding, 16, 4, &[0, 0, 0]).is_err());
    }

    #[test]
    fn exhaustive_search_picks_zero_for_small_values() {
        let mapped = vec![0u32; GAGGLE_SIZE];
        let k = select_rice_k(&mapped, 0, GAGGLE_SIZE, 8, 6, true);
        assert_eq!(k, 0);
    }

    #[test]
    fn exhaustive_search_falls_back_to_uncoded() {
        // Every value needs the full word width, so no k can beat uncoded output.
        let mapped = vec![0xFFu32; GAGGLE_SIZE];
        let k = select_rice_k(&mapped, 0, GAGGLE_SIZE, 8, 6, true);
        assert_eq!(k, UNCODED_FLAG);
    }

    #[test]
    fn exhaustive_search_stays_within_max_k() {
        let mapped: Vec<u32> = (0..GAGGLE_SIZE as u32).map(|i| i * 3).collect();
        let k = select_rice_k(&mapped, 0, GAGGLE_SIZE, 8, 6, true);
        assert!(
            k == UNCODED_FLAG || (0..=6).contains(&k),
            "unexpected k {}",
            k
        );
    }

    #[test]
    fn heuristic_picks_zero_for_small_values() {
        let mapped = vec![0u32; GAGGLE_SIZE];
        let k = select_rice_k(&mapped, 0, GAGGLE_SIZE, 8, 6, false);
        assert_eq!(k, 0);
    }

    #[test]
    fn heuristic_falls_back_to_uncoded() {
        let mapped = vec![0xFFu32; GAGGLE_SIZE];
        let k = select_rice_k(&mapped, 0, GAGGLE_SIZE, 8, 6, false);
        assert_eq!(k, UNCODED_FLAG);
    }

    /// Cross-checks against verify/vectors/rice_vectors.txt, generated by
    /// verify/c_unit_tests/gen_rice_vectors.c from the real C reference
    /// (linked against its actual ricecoding.o/bitsIO.o). Each line is
    /// `bit_length opt0 opt1 opt2 num_values bytes_hex`: encoding values
    /// 0..num_values with this module's rice_coding must produce the exact
    /// same bytes the C RiceCoding produced for the same sequence.
    ///
    /// Ignored by default (like golden_roundtrip.rs) because it needs
    /// verify/run_unit_vectors.py to have generated the vectors file first;
    /// that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verify/vectors/rice_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [bit_length, opt0, opt1, opt2, num_values, bytes_hex] = fields[..] else {
                panic!("malformed vector line: {line}");
            };
            let bit_length: u8 = bit_length.parse().unwrap();
            let option = [
                opt0.parse().unwrap(),
                opt1.parse().unwrap(),
                opt2.parse().unwrap(),
            ];
            let num_values: u32 = num_values.parse().unwrap();

            let path = temp_path(&format!(
                "shared_rice_{}_{}_{}_{}.bin",
                bit_length, option[0], option[1], option[2]
            ));
            let mut enc = CodingPara::new();
            enc.bits.open_write(path.to_str().unwrap()).unwrap();
            for value in 0..num_values {
                rice_coding(&mut enc, value, bit_length, &option).unwrap();
            }
            segment_buffer_flush_encoder(&mut enc).unwrap();
            drop(enc.bits.file.take());

            let got_bytes = fs::read(&path).unwrap();
            let got_hex: String = got_bytes.iter().map(|b| format!("{:02x}", b)).collect();
            assert_eq!(
                got_hex, bytes_hex,
                "bit_length={} option={:?}: rust produced {} but C reference produced {}",
                bit_length, option, got_hex, bytes_hex
            );

            // Cross-decode: write the C-produced bytes (not Rust's own encode
            // output) to disk and confirm rice_decoding recovers 0..num_values
            // from them. This checks Rust's *decoder* against real C-encoder
            // bytes, which the encode-byte comparison above doesn't exercise.
            let decode_path = temp_path(&format!(
                "shared_rice_decode_{}_{}_{}_{}.bin",
                bit_length, option[0], option[1], option[2]
            ));
            let c_bytes: Vec<u8> = (0..bytes_hex.len() / 2)
                .map(|i| u8::from_str_radix(&bytes_hex[i * 2..i * 2 + 2], 16).unwrap())
                .collect();
            fs::write(&decode_path, &c_bytes).unwrap();

            let mut dec = CodingPara::new();
            dec.bits.open_read(decode_path.to_str().unwrap()).unwrap();
            for value in 0..num_values {
                let decoded = rice_decoding(&mut dec, bit_length as i16, &option).unwrap();
                assert_eq!(
                    decoded, value,
                    "bit_length={} option={:?}: rust decode of C-produced bytes gave {} at value index {}",
                    bit_length, option, decoded, value
                );
            }

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }

    #[test]
    fn heuristic_k_grows_with_magnitude() {
        let small: Vec<u32> = vec![4; GAGGLE_SIZE];
        let large: Vec<u32> = vec![40; GAGGLE_SIZE];
        let k_small = select_rice_k(&small, 0, GAGGLE_SIZE, 8, 6, false);
        let k_large = select_rice_k(&large, 0, GAGGLE_SIZE, 8, 6, false);
        assert!(
            k_large >= k_small,
            "k should not shrink as magnitudes grow: {} -> {}",
            k_small,
            k_large
        );
    }
}
