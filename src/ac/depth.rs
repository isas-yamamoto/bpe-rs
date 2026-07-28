//! AC bit-depth gaggle encoding/decoding - original/source/AC_BitPlaneCoding.c

use crate::bitstream::{bits_read, bits_write};
use crate::error::{BpeError, BpeResult};
use crate::rice::{select_rice_k, UNCODED_FLAG};
use crate::types::{BitPlaneBits, CodingPara, GAGGLE_SIZE};

fn ac_gaggle_encoding(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    start_index: usize,
    gaggles: usize,
    max_k: i32,
    id_length: i32,
) -> BpeResult<()> {
    let mapped: Vec<u32> = (0..(start_index + gaggles))
        .map(|i| block_info[i].mapped_ac as u32)
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
            bits_write(coding, block_info[i].mapped_ac as u32, coding.n as i32)?;
        } else {
            bits_write(coding, 1, ((block_info[i].mapped_ac as i32) >> min_k) + 1)?;
        }
    }
    if min_k != UNCODED_FLAG {
        for i in start_index.max(1)..(start_index + gaggles) {
            bits_write(coding, block_info[i].mapped_ac as u32, min_k)?;
        }
    }
    Ok(())
}

pub fn dpcm_ac_mapper(block_info: &mut [BitPlaneBits], size: usize, n: i16) {
    let x_min: i32 = 0;
    let x_max: i32 = (1i32 << n) - 1;

    let mut diff_ac = vec![0i32; size];
    diff_ac[0] = block_info[0].bit_max_ac as i32;
    block_info[0].mapped_ac = block_info[0].bit_max_ac;
    for i in 1..size {
        diff_ac[i] = block_info[i].bit_max_ac as i32 - block_info[i - 1].bit_max_ac as i32;
    }

    for i in 1..size {
        let prev = block_info[i - 1].bit_max_ac as i32;
        let theta = (prev - x_min).min(x_max - prev);
        if diff_ac[i] >= 0 && diff_ac[i] <= theta {
            block_info[i].mapped_ac = (2 * diff_ac[i]) as u16;
        } else if diff_ac[i] < 0 && diff_ac[i] >= -theta {
            block_info[i].mapped_ac = (-2 * diff_ac[i] - 1) as u16;
        } else {
            block_info[i].mapped_ac = (theta + diff_ac[i].abs()) as u16;
        }
    }
}

pub fn ac_depth_encoder(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    coding.n = 0;
    while (coding.header.part1.bit_depth_ac_5bits >> coding.n) > 0 {
        coding.n += 1;
    }

    let s = coding.header.part3.s_20bits as usize;
    dpcm_ac_mapper(block_info, s, coding.n as i16);

    let (max_k, id_length): (i32, i32) = if coding.n == 2 {
        (0, 1)
    } else if coding.n <= 4 {
        (2, 2)
    } else if coding.n <= 5 {
        (6, 3)
    } else {
        return Err(BpeError::DataError);
    };

    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        ac_gaggle_encoding(
            coding,
            block_info,
            gaggle_start_index,
            gaggles,
            max_k,
            id_length,
        )?;
        if coding.segment_full {
            return Ok(());
        }
        gaggle_start_index += gaggles;
    }
    Ok(())
}

fn ac_gaggle_decoding(
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

    'gaggle_loop: for i in start_index..(start_index + gaggles) {
        if uncoded || (i == 0) {
            let w = bits_read(coding, coding.n as i16)?;
            block_info[i].mapped_ac = w as u16;
            if coding.rate_reached {
                return Ok(());
            }
        } else {
            let mut counter: u16 = 0;
            let mut word = bits_read(coding, 1)?;
            while (word == 0) && !coding.rate_reached {
                counter += 1;
                word = bits_read(coding, 1)?;
            }
            if coding.rate_reached {
                break 'gaggle_loop;
            }
            block_info[i].mapped_ac = counter;
            block_info[i].mapped_ac <<= min_k;
        }
    }
    if coding.rate_reached {
        return Ok(());
    }
    if !uncoded && !coding.rate_reached {
        for i in start_index.max(1)..(start_index + gaggles) {
            let w = bits_read(coding, min_k as i16)?;
            block_info[i].mapped_ac += w as u16;
            if coding.rate_reached {
                break;
            }
        }
    }
    Ok(())
}

pub fn dpcm_ac_demapper(block_info: &mut [BitPlaneBits], size: usize, n: i16) {
    let x_min: i32 = 0;
    let x_max: i32 = (1i32 << n) - 1;

    block_info[0].bit_max_ac = (block_info[0].mapped_ac as u8) as u16;

    for i in 1..size {
        let prev = block_info[i - 1].bit_max_ac as i32;
        let theta = (prev - x_min).min(x_max - prev);
        let mapped = block_info[i].mapped_ac as i32;

        let mut diff: i32;
        if mapped % 2 == 0 {
            diff = mapped / 2;
            if diff >= 0 && diff <= theta {
                block_info[i].bit_max_ac = (diff + prev) as u16;
                continue;
            }
        } else {
            diff = -((mapped + 1) / 2);
            if diff <= 0 && diff >= -theta {
                block_info[i].bit_max_ac = (diff + prev) as u16;
                continue;
            }
        }

        diff = mapped - theta;
        block_info[i].bit_max_ac = (diff + prev) as u16;

        if (block_info[i].bit_max_ac as i32) < x_min || (block_info[i].bit_max_ac as i32) > x_max {
            diff = -diff;
            block_info[i].bit_max_ac = (diff + prev) as u16;
        }
    }
}

pub fn ac_depth_decoder(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    coding.n = 0;
    while (coding.header.part1.bit_depth_ac_5bits >> coding.n) > 0 {
        coding.n += 1;
    }

    let id_length: i16 = if coding.n == 2 {
        1
    } else if coding.n <= 4 {
        2
    } else if coding.n <= 5 {
        3
    } else {
        return Err(BpeError::DataError);
    };

    let s = coding.header.part3.s_20bits as usize;
    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        ac_gaggle_decoding(coding, block_info, gaggle_start_index, gaggles, id_length)?;
        gaggle_start_index += gaggles;
    }

    dpcm_ac_demapper(block_info, s, coding.n as i16);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitstream::segment_buffer_flush_encoder;
    use std::fs;
    use std::path::PathBuf;

    fn make_blocks(raw: &[u16]) -> Vec<BitPlaneBits> {
        raw.iter()
            .map(|&v| BitPlaneBits {
                bit_max_ac: v,
                ..Default::default()
            })
            .collect()
    }

    /// Cross-checks against verify/vectors/ac_dpcm_vectors.txt, generated by
    /// verify/c_unit_tests/gen_ac_dpcm_vectors.c from the real C reference
    /// (linked against its actual AC_BitPlaneCoding.o). Each line is
    /// `N size raw_csv mapped_csv decoded_csv`.
    ///
    /// Unlike the DC-side DPCM_DCMapper/DeMapper (dc/dpcm.rs), the AC-side
    /// BitMaxAC/MappedAC fields are `WORD16` (unsigned short) rather than
    /// `DWORD32` (unsigned long): C's integer promotion converts WORD16 to
    /// *signed* int for the theta subtraction, unlike DWORD32 which stays
    /// unsigned, so the theta wraparound bug found on the DC side is not
    /// expected here -- this test exists to confirm that empirically rather
    /// than leave it as an unverified assumption.
    ///
    /// Ignored by default (like golden_roundtrip.rs) because it needs
    /// verify/run_unit_vectors.py to have generated the vectors file first;
    /// that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verify/vectors/ac_dpcm_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let parse_csv =
            |s: &str| -> Vec<u16> { s.split(',').map(|v| v.parse().unwrap()).collect() };

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [n, size, raw_csv, mapped_csv, decoded_csv] = fields[..] else {
                panic!("malformed vector line: {line}");
            };
            let n: i16 = n.parse().unwrap();
            let size: usize = size.parse().unwrap();
            let raw = parse_csv(raw_csv);
            let expected_mapped = parse_csv(mapped_csv);
            let expected_decoded = parse_csv(decoded_csv);
            assert_eq!(raw.len(), size);

            let mut encoded = make_blocks(&raw);
            dpcm_ac_mapper(&mut encoded, size, n);
            let got_mapped: Vec<u16> = encoded.iter().map(|b| b.mapped_ac).collect();
            assert_eq!(
                got_mapped, expected_mapped,
                "N={} mapper mismatch: rust={:?} c={:?}",
                n, got_mapped, expected_mapped
            );

            let mut decoded = make_blocks(&vec![0; size]);
            for i in 0..size {
                decoded[i].mapped_ac = encoded[i].mapped_ac;
            }
            dpcm_ac_demapper(&mut decoded, size, n);
            let got_decoded: Vec<u16> = decoded.iter().map(|b| b.bit_max_ac).collect();
            assert_eq!(
                got_decoded, expected_decoded,
                "N={} demapper mismatch: rust={:?} c={:?}",
                n, got_decoded, expected_decoded
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    /// Cross-checks against verify/vectors/ac_depth_vectors.txt, generated by
    /// verify/c_unit_tests/gen_ac_depth_vectors.c, which calls the real C
    /// `ACDepthEncoder` directly -- covering every N (2-5) it supports, 1-3
    /// gaggle segment sizes, and several BitMaxAC distributions -- rather
    /// than hoping a full pipeline sweep's images happen to produce a given
    /// distribution. Unlike the mapper-only test above, this exercises the
    /// private per-gaggle Rice-split selection (`ac_gaggle_encoding`) and
    /// real bitstream output, the part a full pipeline sweep drives least
    /// reliably.
    ///
    /// Ignored by default (like the other shared-vector tests) because it
    /// needs verify/run_unit_vectors.py to have generated the vectors file
    /// first; that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn ac_depth_shared_vectors_match_c_reference() {
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../verify/vectors/ac_depth_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for (line_no, line) in text.lines().enumerate() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [bit_depth_ac_5bits, s_20bits, bitmaxac_csv, bytes_hex] = fields[..] else {
                panic!("malformed vector line: {line}");
            };
            let bit_depth_ac_5bits: u8 = bit_depth_ac_5bits.parse().unwrap();
            let s_20bits: usize = s_20bits.parse().unwrap();
            let raw: Vec<u16> = bitmaxac_csv.split(',').map(|v| v.parse().unwrap()).collect();
            assert_eq!(raw.len(), s_20bits);

            let mut coding = CodingPara::new();
            coding.header.part1.bit_depth_ac_5bits = bit_depth_ac_5bits;
            coding.header.part3.s_20bits = s_20bits as u32;
            coding.header.part3.opt_dc_select = true;

            // Encode side: must produce byte-identical output to the C reference.
            let enc_path = temp_path(&format!("shared_ac_depth_enc_{line_no}.bin"));
            coding.bits.open_write(enc_path.to_str().unwrap()).unwrap();
            let mut blocks = make_blocks(&raw);
            ac_depth_encoder(&mut coding, &mut blocks).unwrap();
            segment_buffer_flush_encoder(&mut coding).unwrap();
            drop(coding.bits.file.take());

            let got_bytes = fs::read(&enc_path).unwrap();
            let got_hex: String = got_bytes.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(
                got_hex, bytes_hex,
                "bit_depth_ac_5bits={bit_depth_ac_5bits} s_20bits={s_20bits}: \
                 rust encoded {got_hex} but C reference produced {bytes_hex}"
            );

            // Cross-decode: feed the C-produced bytes (not Rust's own encode
            // output) into ac_depth_decoder and confirm the original BitMaxAC
            // sequence is recovered, exercising Rust's decoder against real
            // C-encoder bytes.
            let dec_path = temp_path(&format!("shared_ac_depth_dec_{line_no}.bin"));
            let c_bytes: Vec<u8> = (0..bytes_hex.len() / 2)
                .map(|i| u8::from_str_radix(&bytes_hex[i * 2..i * 2 + 2], 16).unwrap())
                .collect();
            fs::write(&dec_path, &c_bytes).unwrap();

            let mut dec_coding = CodingPara::new();
            dec_coding.header.part1.bit_depth_ac_5bits = bit_depth_ac_5bits;
            dec_coding.header.part3.s_20bits = s_20bits as u32;
            dec_coding.bits.open_read(dec_path.to_str().unwrap()).unwrap();
            let mut dec_blocks = make_blocks(&vec![0; s_20bits]);
            ac_depth_decoder(&mut dec_coding, &mut dec_blocks).unwrap();
            let got_decoded: Vec<u16> = dec_blocks.iter().map(|b| b.bit_max_ac).collect();
            assert_eq!(
                got_decoded, raw,
                "bit_depth_ac_5bits={bit_depth_ac_5bits} s_20bits={s_20bits}: \
                 decoded {got_decoded:?} but expected original {raw:?}"
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }
}
