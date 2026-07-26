//! Fixed-table Rice encode for AC stage symbols.

use crate::bitstream::bits_write;
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
        1 => bits_write(coding, input_val, 1),
        2 => {
            if option[0] == 1 {
                bits_write(coding, input_val, 2)
            } else if option[0] == 0 {
                if input_val <= 2 {
                    bits_write(coding, 0, input_val as i32)?;
                    bits_write(coding, 1, 1)
                } else {
                    bits_write(coding, 0, 3)
                }
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        3 => {
            if option[1] == 0 {
                if input_val <= 2 {
                    bits_write(coding, 0, input_val as i32)?;
                    bits_write(coding, 1, 1)
                } else if input_val <= 5 {
                    bits_write(coding, 0, 3)?;
                    bits_write(coding, input_val - 3, 2)
                } else if input_val <= 7 {
                    bits_write(coding, 0, 3)?;
                    bits_write(coding, input_val, 3)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 1 {
                if input_val <= 1 {
                    bits_write(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_write(coding, input_val, 3)
                } else if input_val <= 7 {
                    bits_write(coding, 0, 2)?;
                    match input_val {
                        4 => bits_write(coding, 2, 2),
                        5 => bits_write(coding, 3, 2),
                        6 => bits_write(coding, 0, 2),
                        7 => bits_write(coding, 1, 2),
                        _ => unreachable!(),
                    }
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 3 {
                bits_write(coding, input_val, 3)
            } else {
                Ok(())
            }
        }
        4 => {
            if option[2] == 0 {
                if input_val <= 3 {
                    bits_write(coding, 0, input_val as i32)?;
                    bits_write(coding, 1, 1)
                } else if input_val <= 7 {
                    bits_write(coding, 0, 5)?;
                    bits_write(coding, input_val - 4, 2)
                } else if input_val <= 15 {
                    bits_write(coding, 0, 4)?;
                    bits_write(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 1 {
                if input_val <= 1 {
                    bits_write(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_write(coding, input_val, 3)
                } else if input_val <= 5 {
                    bits_write(coding, 0, 2)?;
                    bits_write(coding, input_val - 2, 2)
                } else if input_val <= 11 {
                    bits_write(coding, 0, 3)?;
                    bits_write(coding, input_val - 6, 3)
                } else if input_val <= 15 {
                    bits_write(coding, 0, 3)?;
                    bits_write(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 2 {
                if input_val <= 3 {
                    bits_write(coding, input_val + 4, 3)
                } else if input_val <= 7 {
                    bits_write(coding, input_val, 4)
                } else if input_val <= 11 {
                    bits_write(coding, 0, 2)?;
                    bits_write(coding, input_val - 4, 3)
                } else if input_val <= 15 {
                    bits_write(coding, input_val - 12, 5)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 3 {
                bits_write(coding, input_val, 4)
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        _ => Err(BpeError::RiceCodingError),
    }
}

#[cfg(test)]
mod tests {
    use crate::bitstream::segment_buffer_flush_encoder;
    use crate::rice::{rice_coding, rice_decoding};
    use crate::types::CodingPara;
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
}
