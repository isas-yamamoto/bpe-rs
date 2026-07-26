//! Orchestration: entry point and stage dispatch for AdjustOutPut.

use crate::dc::deconv_twos_comp;
use crate::error::BpeResult;
use crate::types::{BitPlaneBits, CodingPara, StopLocation, BLOCK_SIZE, INTEGER_WAVELET};

use crate::adjust::stage1::stage1;
use crate::adjust::stage2::stage2;
use crate::adjust::stage3::stage3;
use crate::adjust::stage4::stage4;

fn dispatch_stage(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    match stop.stopped_stage {
        1 => stage1(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        2 => stage2(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        3 => stage3(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        _ => stage4(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
    }
}

pub fn adjust_output(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    let total_blocks = coding.header.part3.s_20bits as usize;

    if coding.header.part4.dwt_type != INTEGER_WAVELET {
        for block in block_info.iter_mut().take(total_blocks) {
            for m in 0..BLOCK_SIZE {
                for n in 0..BLOCK_SIZE {
                    block.block_float[m][n] = block.block_int[m][n] as f32;
                }
            }
        }
    }

    let bit_depth_dc = coding.header.part1.bit_depth_dc_5bits as i16;
    for block in block_info.iter_mut().take(total_blocks) {
        let combined =
            (block.shifted_dc as i32).wrapping_add(block.decoding_dc_remainder as i32) as u32;
        block.block_int[0][0] = deconv_twos_comp(combined, bit_depth_dc)?;
        block.block_float[0][0] = block.block_int[0][0] as f32;
    }

    if coding.rate_reached
        && coding.decoding_stop_locations.block_no_stop_decoding != -1
        && coding.decoding_stop_locations.bit_plane_stop_decoding != -1
    {
        let stop = coding.decoding_stop_locations.clone();

        let b_dc: i32 =
            if (stop.bit_plane_stop_decoding as i32) <= coding.quantization_factor_q as i32 {
                stop.bit_plane_stop_decoding as i32
            } else {
                coding.quantization_factor_q as i32
            };

        if coding.header.part4.dwt_type == INTEGER_WAVELET {
            if b_dc >= 1 {
                let add = 1i32 << (b_dc - 1);
                for block in block_info.iter_mut().take(total_blocks) {
                    block.block_int[0][0] += add;
                }
            }

            let (beta_1, beta_2): (f32, f32) = if stop.bit_plane_stop_decoding >= 1 {
                let bp = stop.bit_plane_stop_decoding as i32;
                (((1i32 << (bp - 1)) - 1) as f32, ((1i32 << bp) - 1) as f32)
            } else {
                (0.0, 0.0)
            };
            let bit_plane_check: u32 = 1u32 << (stop.bit_plane_stop_decoding as u32);

            dispatch_stage(
                block_info,
                total_blocks,
                &stop,
                beta_1,
                beta_2,
                bit_plane_check,
            );
        } else {
            let bit_plane_check: u32 = 1u32 << (stop.bit_plane_stop_decoding as u32);

            if b_dc >= 1 {
                let temp = (1i32 << (b_dc - 1)) as f32 - 0.5;
                for block in block_info.iter_mut().take(total_blocks) {
                    block.block_float[0][0] += temp;
                }
            }

            let (beta_1, beta_2): (f32, f32) = if stop.bit_plane_stop_decoding >= 1 {
                let bp = stop.bit_plane_stop_decoding as i32;
                (
                    ((1i32 << (bp - 1)) as f32) - 0.5,
                    ((1i32 << bp) as f32) - 0.5,
                )
            } else {
                (
                    0.0,
                    if stop.bit_plane_stop_decoding == 0 {
                        0.5
                    } else {
                        0.0
                    },
                )
            };

            dispatch_stage(
                block_info,
                total_blocks,
                &stop,
                beta_1,
                beta_2,
                bit_plane_check,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HeaderPart1, HeaderPart3, HeaderPart4, StopLocation, BLOCK_SIZE};
    use std::fs;
    use std::path::PathBuf;

    const TOTAL_BLOCKS: usize = 3;
    const BLOCK_NO: i32 = 1;

    // Must match verify/c_unit_tests/gen_adjust_output_vectors.c's int_val/float_val exactly.
    fn int_val(block: i64, m: i64, n: i64, variant: i32) -> i32 {
        let mut v = ((block * 7 + m * 3 + n * 5) % 11 - 5) as i32;
        if variant & 1 != 0 {
            v = -v;
        }
        if variant & 2 != 0 {
            v += ((block * 3 + m * 11 + n * 7) % 7 - 3) as i32;
        }
        v
    }
    fn float_val(block: i64, m: i64, n: i64, variant: i32) -> f32 {
        let mut v = ((block * 5 + m * 7 + n * 2) % 9 - 4) as i32;
        if variant & 1 != 0 {
            v = -v;
        }
        if variant & 2 != 0 {
            v += ((block * 2 + m * 5 + n * 13) % 7 - 3) as i32;
        }
        v as f32
    }

    /// Cross-checks against verify/vectors/adjust_output_vectors.txt, generated
    /// by verify/c_unit_tests/gen_adjust_output_vectors.c, which calls the real
    /// C `AdjustOutPut` directly (not through a full encode/decode roundtrip)
    /// across every (DWTType, stoppedstage, b_DC-branch, X/Y_LocationStopDecoding)
    /// combination -- the full 8x8 X/Y sweep exists specifically to reach the
    /// deep per-stage decision trees that a black-box rate/content sweep can
    /// only hit by chance (see COMPATIBILITY_REPORT.md for why AdjustOutPut
    /// needed this rather than more full-pipeline test cases).
    ///
    /// Ignored by default (like golden_roundtrip.rs) because it needs
    /// verify/run_unit_vectors.py to have generated the vectors file first;
    /// that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../verify/vectors/adjust_output_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, int_csv, float_csv] =
                fields[..]
            else {
                panic!("malformed vector line: {line}");
            };
            let dwt_type: u8 = dwt_type.parse().unwrap();
            let stoppedstage: u8 = stoppedstage.parse().unwrap();
            let b_dc_case: u8 = b_dc_case.parse().unwrap();
            let x_loc: i8 = x_loc.parse().unwrap();
            let y_loc: i8 = y_loc.parse().unwrap();
            let variant: i32 = variant.parse().unwrap();
            let expected_int: Vec<i32> = int_csv.split(',').map(|v| v.parse().unwrap()).collect();
            let expected_float: Vec<f32> =
                float_csv.split(',').map(|v| v.parse().unwrap()).collect();
            assert_eq!(expected_int.len(), TOTAL_BLOCKS * BLOCK_SIZE * BLOCK_SIZE);
            assert_eq!(expected_float.len(), TOTAL_BLOCKS * BLOCK_SIZE * BLOCK_SIZE);

            let mut coding = CodingPara {
                header: crate::types::Header {
                    part1: HeaderPart1 {
                        bit_depth_dc_5bits: 8,
                        ..Default::default()
                    },
                    part3: HeaderPart3 {
                        s_20bits: TOTAL_BLOCKS as u32,
                        ..Default::default()
                    },
                    part4: HeaderPart4 {
                        dwt_type,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                rate_reached: true,
                // b_dc_case 2 makes BitPlaneStopDecoding > QuantizationFactorQ (5 > 2), the
                // only way to reach AdjustOutPut's `b_DC = QuantizationFactorQ` else-branch;
                // must match gen_adjust_output_vectors.c's run_case exactly.
                quantization_factor_q: match b_dc_case {
                    0 => 0,
                    1 => 5,
                    _ => 2,
                },
                decoding_stop_locations: StopLocation {
                    bit_plane_stop_decoding: match b_dc_case {
                        0 => 0,
                        1 => 3,
                        _ => 5,
                    },
                    block_no_stop_decoding: BLOCK_NO,
                    stopped_stage: stoppedstage,
                    x_location_stop_decoding: x_loc,
                    y_location_stop_decoding: y_loc,
                    ..Default::default()
                },
                ..CodingPara::new()
            };

            let mut blocks: Vec<BitPlaneBits> = (0..TOTAL_BLOCKS)
                .map(|b| {
                    let mut block = BitPlaneBits {
                        shifted_dc: (100 + b) as u32,
                        decoding_dc_remainder: 0.0,
                        ..Default::default()
                    };
                    for m in 0..BLOCK_SIZE {
                        for n in 0..BLOCK_SIZE {
                            block.block_int[m][n] = int_val(b as i64, m as i64, n as i64, variant);
                            block.block_float[m][n] =
                                float_val(b as i64, m as i64, n as i64, variant);
                        }
                    }
                    block
                })
                .collect();

            adjust_output(&mut coding, &mut blocks).unwrap();

            let got_int: Vec<i32> = blocks
                .iter()
                .flat_map(|b| b.block_int.iter().flat_map(|row| row.iter().copied()))
                .collect();
            let got_float: Vec<f32> = blocks
                .iter()
                .flat_map(|b| b.block_float.iter().flat_map(|row| row.iter().copied()))
                .collect();

            assert_eq!(
                got_int, expected_int,
                "dwt_type={} stoppedstage={} b_dc={} x={} y={} variant={}: int mismatch: rust={:?} c={:?}",
                dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, got_int, expected_int
            );
            assert_eq!(
                got_float, expected_float,
                "dwt_type={} stoppedstage={} b_dc={} x={} y={} variant={}: float mismatch: rust={:?} c={:?}",
                dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, got_float, expected_float
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }
}
