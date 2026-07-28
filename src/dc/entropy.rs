//! DC entropy (Rice-like) encoding/decoding - original/source/DC_EnDeCoding.c

use crate::bitstream::{bits_read, bits_write};
use crate::error::{BpeError, BpeResult};
use crate::rice::{select_rice_k, UNCODED_FLAG};
use crate::types::{BitPlaneBits, CodingPara, GAGGLE_SIZE, INTEGER_WAVELET};

fn dc_encoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    start_index: usize,
    gaggles: usize,
    max_k: i32,
    id_length: i32,
) -> BpeResult<()> {
    let mapped: Vec<u32> = (0..(start_index + gaggles))
        .map(|i| block_info[i].mapped_dc)
        .collect();
    let min_k = select_rice_k(
        &mapped,
        start_index,
        gaggles,
        coding.n,
        max_k,
        coding.header.part3.opt_dc_select,
    );

    bits_write(coding, min_k as u32, id_length)?;

    for i in start_index..(start_index + gaggles) {
        if (min_k == UNCODED_FLAG) || (i == 0) {
            bits_write(coding, block_info[i].mapped_dc, coding.n as i32)?;
        } else {
            bits_write(coding, 1, ((block_info[i].mapped_dc >> min_k) + 1) as i32)?;
        }
    }
    if min_k != UNCODED_FLAG {
        for i in start_index.max(1)..(start_index + gaggles) {
            bits_write(coding, block_info[i].mapped_dc, min_k)?;
        }
    }
    Ok(())
}

pub fn dc_entropy_encoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let (max_k, id_length) = if coding.n == 2 {
        (0, 1)
    } else if coding.n <= 4 {
        (2, 2)
    } else if coding.n <= 8 {
        (6, 3)
    } else {
        (8, 4)
    };

    let s = coding.header.part3.s_20bits as usize;
    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        dc_encoder(
            coding,
            block_info,
            gaggle_start_index,
            gaggles,
            max_k,
            id_length,
        )?;
        gaggle_start_index += gaggles;
    }

    if coding.header.part1.bit_depth_ac_5bits < coding.quantization_factor_q {
        let numaddbitplanes: i32 = if coding.header.part4.dwt_type == INTEGER_WAVELET {
            coding.quantization_factor_q as i32
                - (coding.header.part1.bit_depth_ac_5bits as i32)
                    .max(coding.header.part4.custom_wt_ll3 as i32)
        } else {
            coding.quantization_factor_q as i32 - coding.header.part1.bit_depth_ac_5bits as i32
        };

        for i in 0..numaddbitplanes {
            for k in 0..s {
                bits_write(
                    coding,
                    (block_info[k].dc_remainder >> (coding.quantization_factor_q as i32 - i - 1))
                        as u32,
                    1,
                )?;
            }
        }
    }
    Ok(())
}

fn dc_gaggle_decoding(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    start_index: usize,
    gaggles: usize,
    id_length: i16,
) -> BpeResult<()> {
    let temp_word = bits_read(coding, id_length)?;
    let min_k = temp_word as u8;

    let uncoded = (id_length == 1 && min_k == 1)
        || (id_length == 2 && min_k == 3)
        || (id_length == 3 && min_k == 7)
        || (id_length == 4 && min_k == 15);

    for i in start_index..(start_index + gaggles) {
        if uncoded || (i == 0) {
            let w = bits_read(coding, coding.n as i16)?;
            block_info[i].mapped_dc = w;
        } else {
            let mut counter: u32 = 0;
            let mut word = bits_read(coding, 1)?;
            while (word == 0) && !coding.rate_reached {
                counter += 1;
                word = bits_read(coding, 1)?;
            }
            if coding.rate_reached {
                break;
            }
            block_info[i].mapped_dc = counter;
            block_info[i].mapped_dc <<= min_k;
        }
    }
    if !uncoded && !coding.rate_reached {
        for i in start_index.max(1)..(start_index + gaggles) {
            let w = bits_read(coding, min_k as i16)?;
            block_info[i].mapped_dc += w;
            if coding.rate_reached {
                break;
            }
        }
    }
    Ok(())
}

pub fn dc_entropy_decoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let id_length: i16 = if coding.n == 2 {
        1
    } else if coding.n <= 4 {
        2
    } else if coding.n <= 8 {
        3
    } else if coding.n <= 10 {
        4
    } else {
        return Err(BpeError::DataError);
    };

    let s = coding.header.part3.s_20bits as usize;
    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        dc_gaggle_decoding(coding, block_info, gaggle_start_index, gaggles, id_length)?;
        gaggle_start_index += gaggles;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::dpcm::dpcm_dc_mapper;
    use crate::bitstream::segment_buffer_flush_encoder;
    use std::fs;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn make_blocks(shifted_dc: &[u32], dc_remainder: &[u16]) -> Vec<BitPlaneBits> {
        shifted_dc
            .iter()
            .zip(dc_remainder)
            .map(|(&sdc, &rem)| BitPlaneBits {
                shifted_dc: sdc,
                dc_remainder: rem,
                ..Default::default()
            })
            .collect()
    }

    /// Cross-checks against verify/vectors/dc_entropy_vectors.txt, generated
    /// by verify/c_unit_tests/gen_dc_entropy_vectors.c, which calls the real
    /// C `DCEntropyEncoder` directly -- covering every QuantizationFactorQ_
    /// prime branch, every N bracket, and both with/without section-4.3.3
    /// extra bitplanes, rather than hoping a full pipeline sweep's images
    /// happen to land on a given combination (COMPATIBILITY_REPORT.md
    /// documents crafting whole synthetic images to reach these branches
    /// once each at the pipeline level; this is the direct-call equivalent
    /// of gen_ac_depth_vectors.c's test for the AC side).
    ///
    /// Cross-decodes only the combos with no extra bitplanes: with extra
    /// bitplanes present, the encoder's trailing bytes belong to section
    /// 4.3.3's raw output, not to dc_entropy_decoder's own contract (that
    /// decode lives elsewhere in the real pipeline), so feeding the full
    /// byte stream to dc_entropy_decoder would test something it was never
    /// meant to consume.
    ///
    /// Ignored by default (like the other shared-vector tests) because it
    /// needs verify/run_unit_vectors.py to have generated the vectors file
    /// first; that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn dc_entropy_shared_vectors_match_c_reference() {
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../verify/vectors/dc_entropy_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for (line_no, line) in text.lines().enumerate() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [bit_depth_dc, bit_depth_ac, q, n, s_20bits, shifted_dc_csv, dc_remainder_csv, bytes_hex] =
                fields[..]
            else {
                panic!("malformed vector line: {line}");
            };
            let bit_depth_dc: u8 = bit_depth_dc.parse().unwrap();
            let bit_depth_ac: u8 = bit_depth_ac.parse().unwrap();
            let q: u8 = q.parse().unwrap();
            let n: u8 = n.parse().unwrap();
            let s_20bits: usize = s_20bits.parse().unwrap();
            let shifted_dc: Vec<u32> = shifted_dc_csv.split(',').map(|v| v.parse().unwrap()).collect();
            let dc_remainder: Vec<u16> =
                dc_remainder_csv.split(',').map(|v| v.parse().unwrap()).collect();
            assert_eq!(shifted_dc.len(), s_20bits);
            assert_eq!(dc_remainder.len(), s_20bits);

            let mut coding = CodingPara::new();
            coding.header.part1.bit_depth_dc_5bits = bit_depth_dc;
            coding.header.part1.bit_depth_ac_5bits = bit_depth_ac;
            coding.header.part3.s_20bits = s_20bits as u32;
            coding.header.part3.opt_dc_select = true;
            coding.header.part4.dwt_type = INTEGER_WAVELET;
            coding.n = n;
            coding.quantization_factor_q = q;

            let mut blocks = make_blocks(&shifted_dc, &dc_remainder);
            dpcm_dc_mapper(&mut blocks, s_20bits, n as i16);

            let enc_path = temp_path(&format!("shared_dc_entropy_enc_{line_no}.bin"));
            coding.bits.open_write(enc_path.to_str().unwrap()).unwrap();
            dc_entropy_encoder(&mut coding, &mut blocks).unwrap();
            segment_buffer_flush_encoder(&mut coding).unwrap();
            drop(coding.bits.file.take());

            let got_bytes = fs::read(&enc_path).unwrap();
            let got_hex: String = got_bytes.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(
                got_hex, bytes_hex,
                "bit_depth_dc={bit_depth_dc} bit_depth_ac={bit_depth_ac} q={q} n={n} \
                 s_20bits={s_20bits}: rust encoded {got_hex} but C reference produced {bytes_hex}"
            );

            // Matches dc_entropy_encoder's own INTEGER_WAVELET formula, floored
            // by custom_wt_ll3 (defaults to 3 in CodingPara::new(), matching
            // HeaderInilization's hardcoded LL3-subband weight -- not left at 0).
            let numaddbitplanes =
                q as i32 - (bit_depth_ac as i32).max(coding.header.part4.custom_wt_ll3 as i32);
            if numaddbitplanes <= 0 {
                let dec_path = temp_path(&format!("shared_dc_entropy_dec_{line_no}.bin"));
                let c_bytes: Vec<u8> = (0..bytes_hex.len() / 2)
                    .map(|i| u8::from_str_radix(&bytes_hex[i * 2..i * 2 + 2], 16).unwrap())
                    .collect();
                fs::write(&dec_path, &c_bytes).unwrap();

                let mut dec_coding = CodingPara::new();
                dec_coding.n = n;
                dec_coding.header.part3.s_20bits = s_20bits as u32;
                dec_coding.bits.open_read(dec_path.to_str().unwrap()).unwrap();
                let mut dec_blocks = make_blocks(&vec![0; s_20bits], &vec![0; s_20bits]);
                dc_entropy_decoder(&mut dec_coding, &mut dec_blocks).unwrap();
                let got_mapped: Vec<u32> = dec_blocks.iter().map(|b| b.mapped_dc).collect();
                let expected_mapped: Vec<u32> = blocks.iter().map(|b| b.mapped_dc).collect();
                assert_eq!(
                    got_mapped, expected_mapped,
                    "bit_depth_dc={bit_depth_dc} bit_depth_ac={bit_depth_ac} q={q} n={n}: \
                     decoded mapped_dc {got_mapped:?} but expected {expected_mapped:?}"
                );
            }

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }
}
